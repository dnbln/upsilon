/*
 *        Copyright (c) 2023 Dinu Blanovschi
 *
 *    Licensed under the Apache License, Version 2.0 (the "License");
 *    you may not use this file except in compliance with the License.
 *    You may obtain a copy of the License at
 *
 *        https://www.apache.org/licenses/LICENSE-2.0
 *
 *    Unless required by applicable law or agreed to in writing, software
 *    distributed under the License is distributed on an "AS IS" BASIS,
 *    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *    See the License for the specific language governing permissions and
 *    limitations under the License.
 */

use std::collections::HashMap;
use std::net::SocketAddr;
use std::process::{ExitStatus, Stdio};
use std::sync::Arc;
use std::time::Duration;

use log::{error, info};
use russh::server::{Auth, Handle, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId, CryptoVec, MethodSet};
use russh_keys::key::PublicKey;
use serde::{Deserialize, Deserializer};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::ChildStdin;
use tokio::task::JoinHandle;
use upsilon_models::users::{UserId, UserSshKey};
use upsilon_ssh::async_trait::async_trait;
use upsilon_ssh::{
    impl_wrapper, CommonSSHError, SSHServer, SSHServerConfig, SSHServerInitializer, SSHServerWrapper
};
use upsilon_vcs::UpsilonVcsConfig;
use upsilon_vcs_permissions::{check_user_has_permissions, GitService};

#[derive(thiserror::Error, Debug)]
pub enum RusshServerError {
    #[error("russh error: {0}")]
    Russh(#[from] russh::Error),
    #[error("russh-keys error: {0}")]
    RusshKeys(#[from] russh_keys::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl From<RusshServerError> for CommonSSHError {
    fn from(value: RusshServerError) -> Self {
        match value {
            value => Self::Other(Box::new(value)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RusshServerConfig {
    port: u16,
    auth_rejection_time_initial: Option<Duration>,
    auth_rejection_time: Duration,
}

#[derive(Clone, Debug)]
pub struct CompleteRusshServerConfig {
    port: u16,
    auth_rejection_time_initial: Option<Duration>,
    auth_rejection_time: Duration,
    vcs_config: UpsilonVcsConfig,
}

impl RusshServerConfig {
    pub fn complete(self, vcs_config: UpsilonVcsConfig) -> CompleteRusshServerConfig {
        CompleteRusshServerConfig {
            port: self.port,
            auth_rejection_time_initial: self.auth_rejection_time_initial,
            auth_rejection_time: self.auth_rejection_time,
            vcs_config,
        }
    }
}

impl Default for RusshServerConfig {
    fn default() -> Self {
        Self {
            port: 22,
            auth_rejection_time_initial: Some(Duration::from_secs(1)),
            auth_rejection_time: Duration::from_secs(10),
        }
    }
}

impl<'de> Deserialize<'de> for RusshServerConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RusshServerConfigPatch {
            #[serde(default)]
            port: Option<u16>,
            #[serde(default, deserialize_with = "deserialize_duration_opt")]
            auth_rejection_time_initial: Option<Duration>,
            #[serde(default, deserialize_with = "deserialize_duration_opt")]
            auth_rejection_time: Option<Duration>,
        }

        let patch = RusshServerConfigPatch::deserialize(deserializer)?;

        let mut config = RusshServerConfig::default();

        if let Some(port) = patch.port {
            config.port = port;
        }

        if let Some(auth_rejection_time_initial) = patch.auth_rejection_time_initial {
            config.auth_rejection_time_initial = Some(auth_rejection_time_initial);
        }

        if let Some(auth_rejection_time) = patch.auth_rejection_time {
            config.auth_rejection_time = auth_rejection_time;
        }

        Ok(config)
    }
}

fn parse_duration<'de, D>(s: &str) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    humantime::parse_duration(s).map_err(serde::de::Error::custom)
}

fn deserialize_duration<'de, D>(d: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <&str>::deserialize(d)?;

    parse_duration::<D>(s)
}

fn deserialize_duration_opt<'de, D>(d: D) -> Result<Option<Duration>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = <&str>::deserialize(d)?;

    match s {
        "none" => Ok(None),
        s => parse_duration::<D>(s).map(Some),
    }
}

impl SSHServerConfig for CompleteRusshServerConfig {}

pub struct RusshServerInitializer {
    config: CompleteRusshServerConfig,
}

#[async_trait]
impl SSHServerInitializer for RusshServerInitializer {
    type Config = CompleteRusshServerConfig;
    type Error = RusshServerError;
    type Server = RusshServer;

    fn new(config: Self::Config) -> Self {
        Self { config }
    }

    async fn init(
        self,
        dcmh: upsilon_data::DataClientMasterHolder,
    ) -> Result<Self::Server, Self::Error> {
        let config = self.config;

        let (sender, receiver) = tokio::sync::oneshot::channel();

        let join_handle = {
            let config = config.clone();

            let russh_config = russh::server::Config {
                auth_rejection_time_initial: config.auth_rejection_time_initial,
                auth_rejection_time: config.auth_rejection_time,
                keys: vec![russh_keys::key::KeyPair::generate_ed25519().unwrap()],
                ..Default::default()
            };

            tokio::spawn(async move {
                let server = receiver.await.unwrap();
                let result = russh::server::run(
                    Arc::new(russh_config),
                    SocketAddr::from(([0, 0, 0, 0], config.port)),
                    server,
                )
                .await;

                if let Err(e) = result {
                    log::error!("russh server error: {e}");
                }
            })
        };

        let server = RusshServer {
            internals: Arc::new(RusshServerInternals {
                config,
                join_handle: Arc::new(join_handle),
                dcmh,
            }),
        };

        let _ = sender.send(server.clone());

        Ok(server)
    }
}

struct RusshServerInternals {
    config: CompleteRusshServerConfig,
    join_handle: Arc<JoinHandle<()>>,
    dcmh: upsilon_data::DataClientMasterHolder,
}

#[derive(Clone)]
pub struct RusshServer {
    internals: Arc<RusshServerInternals>,
}

#[async_trait]
impl SSHServer for RusshServer {
    type Config = CompleteRusshServerConfig;
    type Error = RusshServerError;
    type Initializer = RusshServerInitializer;

    async fn stop(&self) -> Result<(), Self::Error> {
        self.internals.join_handle.abort();

        Ok(())
    }

    fn into_wrapper(self) -> Box<dyn SSHServerWrapper + Send + Sync> {
        Box::new(RusshServerWrapper::new(self))
    }
}

impl_wrapper! {RusshServer, RusshServerWrapper}

impl Server for RusshServer {
    type Handler = RusshServerHandler;

    fn new_client(&mut self, peer_addr: Option<SocketAddr>) -> Self::Handler {
        RusshServerHandler::new(self, peer_addr)
    }
}

pub struct RusshServerHandler {
    internals: Arc<RusshServerInternals>,
    peer_addr: Option<SocketAddr>,
    user: Option<UserId>,
    stdin: HashMap<ChannelId, ChildStdin>,
}

impl RusshServerHandler {
    fn new(server: &mut RusshServer, peer_addr: Option<SocketAddr>) -> Self {
        Self {
            internals: Arc::clone(&server.internals),
            peer_addr,
            user: None,
            stdin: HashMap::new(),
        }
    }

    fn auth_reject_pubkey(self) -> Result<(Self, Auth), RusshServerError> {
        Ok((
            self,
            Auth::Reject {
                proceed_with_methods: Some(MethodSet::PUBLICKEY),
            },
        ))
    }

    async fn send_stdin(
        &mut self,
        channel_id: ChannelId,
        data: &[u8],
    ) -> Result<(), RusshServerError> {
        if let Some(stdin) = self.stdin.get_mut(&channel_id) {
            stdin.write_all(data).await?;
        }

        Ok(())
    }
}

macro_rules! reject_not_git_user {
    ($self:expr, $user:expr) => {
        if $user != "git" {
            return Ok((
                $self,
                Auth::Reject {
                    proceed_with_methods: None,
                },
            ));
        }
    };
}

#[async_trait]
impl Handler for RusshServerHandler {
    type Error = RusshServerError;

    async fn auth_none(self, user: &str) -> Result<(Self, Auth), Self::Error> {
        reject_not_git_user!(self, user);

        self.auth_reject_pubkey()
    }

    async fn auth_password(self, user: &str, _password: &str) -> Result<(Self, Auth), Self::Error> {
        reject_not_git_user!(self, user);

        self.auth_reject_pubkey()
    }

    async fn auth_publickey(
        self,
        user: &str,
        public_key: &PublicKey,
    ) -> Result<(Self, Auth), Self::Error> {
        reject_not_git_user!(self, user);

        let result = self
            .internals
            .dcmh
            .query_master()
            .query_user_ssh_key(UserSshKey::new(public_key.clone()))
            .await
            .map_err(|e| {
                error!("Failed to query user ssh key: {}", e);

                RusshServerError::Other(Box::new(e))
            })?;

        match result {
            Some(user) => Ok((
                Self {
                    user: Some(user),
                    ..self
                },
                Auth::Accept,
            )),
            None => self.auth_reject_pubkey(),
        }
    }

    async fn channel_eof(
        mut self,
        channel: ChannelId,
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let stdin = self.stdin.remove(&channel);
        if let Some(mut stdin) = stdin {
            stdin.shutdown().await?;
        }

        Ok((self, session))
    }

    async fn channel_open_session(
        self,
        _channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        Ok((self, true, session))
    }

    async fn data(
        mut self,
        channel: ChannelId,
        data: &[u8],
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        self.send_stdin(channel, data).await?;

        Ok((self, session))
    }

    async fn exec_request(
        mut self,
        channel: ChannelId,
        data: &[u8],
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let git_shell_cmd = std::str::from_utf8(data).expect("invalid utf8");

        let (service, path) = parse_git_shell_cmd(git_shell_cmd);

        let path = path.strip_prefix('/').unwrap_or(path);

        let repo_id = upsilon_vcs::read_repo_id(&self.internals.config.vcs_config, path)
            .await
            .map_err(|e| {
                error!("Failed to read repo id: {}", e);
                session.extended_data(channel, 1, CryptoVec::from_slice(b"Internal server error"));
                session.channel_failure(channel);
                RusshServerError::Other(Box::new(e))
            })?;

        let repo_id = repo_id
            .parse::<upsilon_models::repo::RepoId>()
            .map_err(|e| {
                error!("Failed to parse repo id: {}", e);
                session.extended_data(channel, 1, CryptoVec::from_slice(b"Internal server error"));
                session.channel_failure(channel);
                RusshServerError::Other(Box::new(e))
            })?;

        let repo = self
            .internals
            .dcmh
            .query_master()
            .query_repo(repo_id)
            .await
            .map_err(|e| {
                error!("Failed to query repo: {}", e);
                session.extended_data(channel, 1, CryptoVec::from_slice(b"Internal server error"));
                session.channel_failure(channel);
                RusshServerError::Other(Box::new(e))
            })?;

        let user_id = self.user.expect("user not set");

        check_user_has_permissions(
            &repo,
            service,
            &self.internals.dcmh.query_master(),
            Some(user_id),
        )
        .await
        .map_err(|e| {
            error!("Failed to check repo perms: {}", e);
            session.extended_data(channel, 1, CryptoVec::from_slice(b"Internal server error"));
            session.channel_failure(channel);
            RusshServerError::Other(Box::new(e))
        })?;

        let reconstructed_shell_cmd = reconstruct_shell_cmd(service, path);

        let mut cmd = {
            let mut cmd = tokio::process::Command::new("git");
            cmd.arg("shell").arg("-c").arg(reconstructed_shell_cmd);
            cmd.current_dir(self.internals.config.vcs_config.get_path());

            cmd
        };

        let mut shell = match cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(shell) => {
                session.channel_success(channel);
                shell
            }
            Err(e) => {
                session.channel_failure(channel);
                return Err(RusshServerError::from(e));
            }
        };

        let session_handle = session.handle();
        let stdin = shell.stdin.take().unwrap();
        self.stdin.insert(channel, stdin);
        let mut shell_stdout = shell.stdout.take().unwrap();
        let mut shell_stderr = shell.stderr.take().unwrap();

        let fut = async move {
            async fn forward<'a, R, Fut, Fwd>(
                session_handle: &'a Handle,
                chan_id: ChannelId,
                r: &mut R,
                mut fwd: Fwd,
            ) -> Result<(), RusshServerError>
            where
                R: AsyncRead + Send + Unpin,
                Fut: std::future::Future<Output = Result<(), CryptoVec>> + 'a,
                Fwd: FnMut(&'a Handle, ChannelId, CryptoVec) -> Fut,
            {
                const BUF_SIZE: usize = 1024 * 32;

                let mut buf = [0u8; BUF_SIZE];

                loop {
                    let read = r.read(&mut buf).await?;

                    if read == 0 {
                        break;
                    }

                    if fwd(session_handle, chan_id, CryptoVec::from_slice(&buf[..read]))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }

                Ok(())
            }

            use futures::future::FutureExt;

            let stdout_fut = forward(
                &session_handle,
                channel,
                &mut shell_stdout,
                |handle, chan, data| async move { handle.data(chan, data).await },
            )
            .fuse();

            tokio::pin!(stdout_fut);

            let stderr_fut = forward(
                &session_handle,
                channel,
                &mut shell_stderr,
                |handle, chan, data| async move {
                    // SSH_EXTENDED_DATA_STDERR = 1
                    handle.extended_data(chan, 1, data).await
                },
            )
            .fuse();

            tokio::pin!(stderr_fut);

            loop {
                enum Pipe {
                    Stdout(Result<(), RusshServerError>),
                    Stderr(Result<(), RusshServerError>),
                    Exit(std::io::Result<ExitStatus>),
                }

                let result = tokio::select! {
                    result = shell.wait() => Pipe::Exit(result),
                    result = &mut stdout_fut => Pipe::Stdout(result),
                    result = &mut stderr_fut => Pipe::Stderr(result),
                };

                match result {
                    Pipe::Stdout(result) => {
                        let _ = result?;
                    }
                    Pipe::Stderr(result) => {
                        let _ = result?;
                    }
                    Pipe::Exit(result) => {
                        let status = result?;

                        stdout_fut.await?;
                        stderr_fut.await?;

                        let status_code = status.code().unwrap_or(128) as u32; // TODO: handle signals properly

                        let _ = session_handle
                            .exit_status_request(channel, status_code)
                            .await;

                        let _ = session_handle.eof(channel).await;
                        // let _ = session_handle.close(channel).await;

                        break;
                    }
                }
            }

            Ok::<(), RusshServerError>(())
        };

        tokio::spawn(fut);

        Ok((self, session))
    }
}

fn strip_apostrophes(s: &str) -> &str {
    if s.starts_with('\'') && s.ends_with('\'') && s.len() >= 2 && !s[1..s.len() - 1].contains('\'')
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

fn parse_git_shell_cmd(git_shell_cmd: &str) -> (GitService, &str) {
    if let Some(rec_pack_path) = git_shell_cmd.strip_prefix("git-receive-pack ") {
        (GitService::ReceivePack, strip_apostrophes(rec_pack_path))
    } else if let Some(upl_ref_path) = git_shell_cmd.strip_prefix("git-upload-pack ") {
        (GitService::UploadPack, strip_apostrophes(upl_ref_path))
    } else if let Some(upl_arc_path) = git_shell_cmd.strip_prefix("git-upload-archive ") {
        (GitService::UploadArchive, strip_apostrophes(upl_arc_path))
    } else {
        panic!("invalid git shell command: {git_shell_cmd:?}");
    }
}

fn reconstruct_shell_cmd(service: GitService, path: &str) -> String {
    let service_str = match service {
        GitService::ReceivePack => "receive-pack",
        GitService::UploadPack => "upload-pack",
        GitService::UploadArchive => "upload-archive",
    };

    format!("git-{service_str} '{path}'")
}
