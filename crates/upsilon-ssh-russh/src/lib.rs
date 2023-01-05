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

use std::future;
use std::net::SocketAddr;
use std::process::Stdio;
use std::sync::Arc;
use std::time::Duration;

use log::info;
use russh::server::{Auth, Handle, Handler, Msg, Server, Session};
use russh::{Channel, ChannelId, CryptoVec, MethodSet};
use russh_keys::key::PublicKey;
use russh_keys::PublicKeyBase64;
use serde::{Deserialize, Deserializer};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use upsilon_ssh::async_trait::async_trait;
use upsilon_ssh::{
    impl_wrapper, CommonSSHError, SSHKey, SSHServer, SSHServerConfig, SSHServerInitializer, SSHServerWrapper
};

#[derive(thiserror::Error, Debug)]
pub enum RusshServerError {
    #[error("russh error: {0}")]
    Russh(#[from] russh::Error),
    #[error("russh-keys error: {0}")]
    RusshKeys(#[from] russh_keys::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
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
    Ok(humantime::parse_duration(s)
        .map_err(|e| serde::de::Error::custom(e))?
        .into())
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

impl SSHServerConfig for RusshServerConfig {}

pub struct RusshServerInitializer {
    config: RusshServerConfig,
}

#[async_trait]
impl SSHServerInitializer for RusshServerInitializer {
    type Config = RusshServerConfig;
    type Error = RusshServerError;
    type Server = RusshServer;

    fn new(config: Self::Config) -> Self {
        Self { config }
    }

    async fn init(self) -> Result<Self::Server, Self::Error> {
        let config = self.config;

        let (sender, receiver) = tokio::sync::oneshot::channel();

        let join_handle = {
            let config = config.clone();

            let mut russh_config = russh::server::Config::default();
            russh_config.auth_rejection_time_initial = config.auth_rejection_time_initial;
            russh_config.auth_rejection_time = config.auth_rejection_time;
            russh_config
                .keys
                .push(russh_keys::key::KeyPair::generate_ed25519().unwrap());

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
                keys: Mutex::new(RusshKeys::new()),
            }),
        };

        let _ = sender.send(server.clone());

        Ok(server)
    }
}

struct RusshKeys {
    keys: Vec<PublicKey>,
}

impl RusshKeys {
    fn new() -> Self {
        Self { keys: vec![] }
    }

    async fn add_key(&mut self, key: SSHKey) -> Result<(), RusshServerError> {
        self.keys
            .push(russh_keys::key::parse_public_key(key.as_bytes(), None)?);

        Ok(())
    }

    fn contains(&self, key: &PublicKey) -> bool {
        self.keys.contains(key)
    }
}

struct RusshServerInternals {
    config: RusshServerConfig,
    join_handle: Arc<JoinHandle<()>>,
    keys: Mutex<RusshKeys>,
}

#[derive(Clone)]
pub struct RusshServer {
    internals: Arc<RusshServerInternals>,
}

#[async_trait]
impl SSHServer for RusshServer {
    type Config = RusshServerConfig;
    type Error = RusshServerError;
    type Initializer = RusshServerInitializer;

    async fn stop(&self) -> Result<(), Self::Error> {
        self.internals.join_handle.abort();

        Ok(())
    }

    async fn add_key(&self, key: SSHKey) -> Result<(), Self::Error> {
        self.internals.keys.lock().await.add_key(key).await?;

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
}

impl RusshServerHandler {
    fn new(server: &mut RusshServer, peer_addr: Option<SocketAddr>) -> Self {
        Self {
            internals: Arc::clone(&server.internals),
            peer_addr,
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

        let result = self.internals.keys.lock().await.contains(public_key);

        // testing only. TODO: remove later
        let result = true;

        match result {
            true => Ok((self, Auth::Accept)),
            false => self.auth_reject_pubkey(),
        }
    }

    async fn auth_succeeded(self, session: Session) -> Result<(Self, Session), Self::Error> {
        Ok((self, session))
    }

    async fn channel_close(
        self,
        channel: ChannelId,
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        Ok((self, session))
    }

    async fn channel_eof(
        self,
        channel: ChannelId,
        mut session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        session.close(channel);

        Ok((self, session))
    }

    async fn channel_open_session(
        self,
        channel: Channel<Msg>,
        session: Session,
    ) -> Result<(Self, bool, Session), Self::Error> {
        Ok((self, true, session))
    }

    async fn exec_request(
        self,
        channel: ChannelId,
        data: &[u8],
        session: Session,
    ) -> Result<(Self, Session), Self::Error> {
        let mut shell = tokio::process::Command::new("bash")
            .arg("~git/git-sh")
            .arg(std::str::from_utf8(data).expect("invalid utf8"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let session_handle = session.handle();
        let mut shell_stdout = shell.stdout.take().unwrap();
        let mut shell_stderr = shell.stderr.take().unwrap();

        tokio::spawn(async move {
            async fn forward<R: AsyncRead + Send + Unpin>(
                session_handle: &Handle,
                chan_id: ChannelId,
                r: &mut R,
            ) -> Result<(), RusshServerError> {
                const BUF_SIZE: usize = 1024 * 32;

                let mut buf = [0u8; BUF_SIZE];

                loop {
                    let read = r.read(&mut buf).await?;

                    if read == 0 {
                        break;
                    }

                    if let Err(_) = session_handle
                        .data(chan_id, CryptoVec::from_slice(&buf))
                        .await
                    {
                        break;
                    }
                }

                Ok(())
            }

            let result = shell.wait().await;

            let status = result?;

            forward(&session_handle, channel, &mut shell_stdout).await?;
            forward(&session_handle, channel, &mut shell_stderr).await?; // TODO: figure out how to send stderr

            let _ = session_handle
                .exit_status_request(channel, status.code().expect("Terminated by signal") as u32) // TODO: handle signals
                .await;

            let _ = session_handle.eof(channel).await;

            Ok::<_, RusshServerError>(())
        });

        Ok((self, session))
    }
}
