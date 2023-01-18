/*
 *        Copyright (c) 2022-2023 Dinu Blanovschi
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

use std::ffi::{OsStr, OsString};
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use anyhow::{bail, format_err};
use git2::{BranchType, Cred, FetchOptions, RemoteCallbacks, Repository};
use log::info;

use crate::{env_var, gql_vars, IdHolder, TestCx, TestCxConfig, TestResult, Token, Username};

pub async fn register_dummy_user(cx: &mut TestCx) {
    cx.create_user("test", "test", "test")
        .await
        .expect("Failed to create user");
}

pub fn upsilon_basic_config(cfg: &mut TestCxConfig) {
    cfg.with_config(
        r#"
vcs:
  path: ./vcs/repos
  jailed: true
  git-protocol:
    enable: false
  http-protocol:
    enable: true
    push-auth-required: true

vcs-errors:
  leak-hidden-repos: true
  verbose: true

web:
  api:
    origin: "https://api.upsilon.dnbln.dev"
  web-interface:
    origin: "https://upsilon.dnbln.dev"
  docs:
    origin: "https://docs.upsilon.dnbln.dev"

debug:
  debug-data: false

data-backend:
  type: in-memory
  save: false

  cache:
    max-users: 1

users:
  register:
    enabled: true
  auth:
    password:
      type: argon2
    "#,
    );
}

pub fn upsilon_basic_config_with_git_daemon(cfg: &mut TestCxConfig) {
    cfg.with_config(
        r#"
vcs:
  path: ./vcs/repos
  jailed: true
  git-protocol:
    enable: true
    git-daemon:
      start: true
      services:
        receive-pack:
          enable: false
          override: allow
        upload-archive:
          enable: false
          override: allow
        upload-pack:
          enable: false
          override: allow
  http-protocol:
    enable: true
    push-auth-required: true

vcs-errors:
  leak-hidden-repos: true
  verbose: true

web:
  api:
    origin: "https://api.upsilon.dnbln.dev"
  web-interface:
    origin: "https://upsilon.dnbln.dev"
  docs:
    origin: "https://docs.upsilon.dnbln.dev"

debug:
  debug-data: false

data-backend:
  type: in-memory
  save: false

  cache:
    max-users: 1

users:
  register:
    enabled: true
  auth:
    password:
      type: argon2
    "#,
    )
    .with_git_protocol()
    .with_git_daemon_port(portpicker::pick_unused_port().expect("Cannot find an unused port"));
}

pub fn upsilon_basic_config_with_ssh(cfg: &mut TestCxConfig) {
    cfg.with_config(
        r#"
vcs:
  path: ./vcs/repos
  jailed: true
  git-protocol:
    enable: false
  http-protocol:
    enable: true
    push-auth-required: true

git-ssh:
  type: russh

vcs-errors:
  leak-hidden-repos: true
  verbose: true

web:
  api:
    origin: "https://api.upsilon.dnbln.dev"
  web-interface:
    origin: "https://upsilon.dnbln.dev"
  docs:
    origin: "https://docs.upsilon.dnbln.dev"

debug:
  debug-data: false

data-backend:
  type: in-memory
  save: false

  cache:
    max-users: 1

users:
  register:
    enabled: true
  auth:
    password:
      type: argon2
    "#,
    )
    .with_ssh_protocol()
    .with_git_ssh_port(portpicker::pick_unused_port().expect("Cannot find an unused port"));
}

pub async fn make_global_mirror_from_github(cx: &mut TestCx) -> TestResult<String> {
    cx.require_online().await?;

    #[derive(serde::Deserialize)]
    struct GlobalMirror {
        #[serde(rename = "_debug__globalMirror")]
        global_mirror: IdHolder,
    }

    let result = cx
        .with_client(|cl| async move {
            cl.gql_query::<GlobalMirror>(
                r#"
mutation {
  _debug__globalMirror(
    name: "upsilon",
    url: "https://github.com/dnbln/upsilon"
  ) {
    id
  }
}
"#,
            )
            .await
        })
        .await?;

    Ok(result.global_mirror.id)
}

pub async fn make_global_mirror_from_local(cx: &mut TestCx) -> TestResult<String> {
    let upsilon_repo = upsilon_cloned_repo_path();
    if !upsilon_repo.exists() {
        panic!("Upsilon repository not found. Use `cargo xtask test` to run the tests.");
    }

    #[derive(serde::Deserialize)]
    struct CopyRepoFromLocalPath {
        #[serde(rename = "_debug__cpGlrFromLocal")]
        copy: IdHolder,
    }

    let id = cx
        .with_client(|cl| async move {
            cl.gql_query_with_variables::<CopyRepoFromLocalPath>(
                r#"
mutation($localPath: String!) {
  _debug__cpGlrFromLocal(name: "upsilon", localPath: $localPath) {
    id
  }
}
"#,
                gql_vars! {"localPath": upsilon_repo},
            )
            .await
        })
        .await?
        .copy
        .id;

    Ok(id)
}

pub async fn make_global_mirror_from_host_repo(cx: &mut TestCx) -> TestResult<String> {
    if cfg!(ci) {
        #[cfg(not(offline))]
        return make_global_mirror_from_local(cx).await;

        #[cfg(offline)]
        panic!("Cannot run this test in CI while also using --offline");
    }

    let upsilon_repo = upsilon_host_repo_git();
    if !upsilon_repo.exists() {
        panic!(
            "\
Upsilon repository .git folder not found. Use `cargo xtask test` to run the tests,
and make sure to do so in a valid git directory."
        );
    }

    #[derive(serde::Deserialize)]
    struct CopyRepoFromLocalPath {
        #[serde(rename = "_debug__cpGlrFromLocal")]
        copy: IdHolder,
    }

    let id = cx
        .with_client(|cl| async move {
            cl.gql_query_with_variables::<CopyRepoFromLocalPath>(
                r#"
mutation($localPath: String!) {
    _debug__cpGlrFromLocal(name: "upsilon", localPath: $localPath) {
        id
    }
}
"#,
                gql_vars! {"localPath": upsilon_repo},
            )
            .await
        })
        .await?
        .copy
        .id;

    Ok(id)
}

pub fn upsilon_cloned_repo_path() -> PathBuf {
    let setup_env = env_var("UPSILON_SETUP_TESTENV");

    let setup_env = PathBuf::from(setup_env);

    setup_env.join("repo/upsilon")
}

fn upsilon_host_repo_git() -> PathBuf {
    let host_repo_path = env_var("UPSILON_HOST_REPO_GIT");

    PathBuf::from(host_repo_path)
}

const SSH_KEY: &str = r#"
-----BEGIN RSA PRIVATE KEY-----
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
-----END RSA PRIVATE KEY-----
"#;

impl TestCx {
    async fn _clone_repo(&self, path: PathBuf, target_url: String) -> TestResult<Repository> {
        {
            let path = path.clone();
            tokio::task::spawn_blocking(move || {
                info!("Cloning {} into {}", target_url, path.display());

                let mut rcb = RemoteCallbacks::new();
                rcb.credentials(|_url, _username_from_url, cred_ty| {
                    Cred::ssh_key_from_memory("git", None, &SSH_KEY, None)
                });

                let mut fo = FetchOptions::new();
                fo.remote_callbacks(rcb);

                git2::build::RepoBuilder::new()
                    .fetch_options(fo)
                    .clone(&target_url, &path)?;

                Ok::<_, git2::Error>(())
            })
            .await??;
        }

        let repo = Repository::open(&path)?;

        Ok(repo)
    }

    pub fn http_repo_url(&self, path: &str) -> String {
        format!("{}/{path}", &self.root)
    }

    pub fn git_repo_url(&self, path: &str) -> TestResult<String> {
        if !self.config.has_git_protocol {
            bail!("Git protocol is not enabled");
        }

        Ok(format!("{}/{path}", self.git_protocol_root))
    }

    pub fn ssh_repo_url(&self, path: &str) -> TestResult<String> {
        if !self.config.has_ssh_protocol {
            bail!("Ssh protocol is not enabled");
        }

        Ok(format!("{}/{path}", self.ssh_protocol_root))
    }

    fn build_target_url<F>(&self, remote_path: F) -> TestResult<String>
    where
        F: FnOnce(GitRemoteRefBuilder) -> GitRemoteRefBuilder,
    {
        let builder = GitRemoteRefBuilder::new();
        let builder = remote_path(builder);
        let remote_ref = builder.build()?;

        let target_url = match remote_ref {
            GitRemoteRef {
                protocol: GitAccessProtocol::Git,
                path,
            } => self.git_repo_url(&path)?,
            GitRemoteRef {
                protocol: GitAccessProtocol::Http,
                path,
            } => self.http_repo_url(&path),
            GitRemoteRef {
                protocol: GitAccessProtocol::Ssh,
                path,
            } => self.ssh_repo_url(&path)?,
        };

        Ok(target_url)
    }

    pub async fn clone<F>(&self, name: &str, remote_path: F) -> TestResult<(PathBuf, Repository)>
    where
        F: FnOnce(GitRemoteRefBuilder) -> GitRemoteRefBuilder,
    {
        let path = self.tempdir(name).await?;

        let target_url = self.build_target_url(remote_path)?;

        let repo = self._clone_repo(path.clone(), target_url).await?;

        Ok((path, repo))
    }

    pub async fn clone_git_binary<F>(
        &self,
        name: &str,
        remote_path: F,
        timeout: Duration,
    ) -> TestResult<(PathBuf, Repository)>
    where
        F: FnOnce(GitRemoteRefBuilder) -> GitRemoteRefBuilder,
    {
        let path = self.tempdir(name).await?;

        let target_url = self.build_target_url(remote_path)?;

        let exit_status = self
            .run_command(
                "git",
                |c| c.arg("clone").arg(target_url).arg(&path),
                timeout,
            )
            .await?;

        if !exit_status.success() {
            bail!("git clone failed with exit status: {:?}", exit_status);
        }

        let repo = Repository::open(&path)?;

        Ok((path, repo))
    }

    pub async fn lookup(&self, path: &str) -> TestResult<String> {
        #[derive(serde::Deserialize)]
        struct LookupResult {
            #[serde(rename = "lookupRepo")]
            lookup_repo: IdHolder,
        }

        Ok(self
            .with_client(|cl| async move {
                cl.gql_query_with_variables::<LookupResult>(
                    r#"query($path: String!) {lookupRepo(path: $path) { id }}"#,
                    gql_vars! {"path": path},
                )
                .await
            })
            .await?
            .lookup_repo
            .id)
    }

    pub async fn create_user(
        &mut self,
        username: &str,
        password: &str,
        email: &str,
    ) -> TestResult<CreateUserResult> {
        #[derive(serde::Deserialize)]
        struct CreateUserToken {
            #[serde(rename = "_debug__createTestUser")]
            result: CreateUserResult,
        }

        let result = self
            .with_client(|cl| async move {
                cl.gql_query_with_variables::<CreateUserToken>(
                    r#"
mutation ($username: Username!, $password: PlainPassword!, $email: Email!) {
  _debug__createTestUser(username: $username, password: $password, email: $email)
}
"#,
                    gql_vars! {
                        "username": username,
                        "password": password,
                        "email": email,
                    },
                )
                .await
            })
            .await?
            .result;

        self.tokens
            .insert(Username(username.to_string()), Token(result.token.clone()));

        Ok(result)
    }

    pub async fn run_command<F>(
        &self,
        program: impl AsRef<OsStr>,
        command_args: F,
        timeout: Duration,
    ) -> TestResult<ExitStatus>
    where
        for<'a> F: FnOnce(&'a mut Cmd<'a>) -> &'a mut Cmd<'a>,
    {
        let mut cmd = Cmd::new(program.as_ref());
        let cmd = command_args(&mut cmd);

        info!("Running: {cmd}");

        let mut tokio_cmd = tokio::process::Command::new(cmd.program);
        tokio_cmd.args(cmd.args.iter());

        static INDEX: AtomicUsize = AtomicUsize::new(0);

        let idx = INDEX.fetch_add(1, Ordering::SeqCst);

        let murderer_file = self
            .tempdir(&format!("shutdown-{idx}"))
            .await?
            .join("shutdown");

        upsilon_gracefully_shutdown::setup_for_graceful_shutdown(&mut tokio_cmd, &murderer_file);

        tokio_cmd.stdout(Stdio::piped());
        tokio_cmd.stderr(Stdio::piped());

        let mut child = tokio_cmd.spawn()?;

        enum Done {
            Done(ExitStatus),
            Timeout,
        }

        let done = tokio::select! {
            status = child.wait() => {Done::Done(status?)}
            _ = tokio::time::sleep(timeout) => {Done::Timeout}
        };

        match done {
            Done::Done(status) => {
                info!("Child exited with status {status}");

                Self::dump_streams_for(format_args!("{cmd}"), &mut child).await?;

                Ok(status)
            }
            Done::Timeout => {
                info!("Timeout, gracefully shutting it down ({cmd})");

                upsilon_gracefully_shutdown::gracefully_shutdown(
                    &mut child,
                    Duration::from_secs(1),
                )
                .await;

                tokio::time::sleep(Duration::from_secs(1)).await;

                Self::dump_streams_for(format_args!("{cmd}"), &mut child).await?;

                Ok(child.try_wait()?.expect("Child should have exited"))
            }
        }
    }

    pub async fn clone_repo_twice(
        &self,
        name1: &str,
        name2: &str,
        remote_path: impl Fn(GitRemoteRefBuilder) -> GitRemoteRefBuilder,
    ) -> TestResult<(Repository, Repository)> {
        let ((_, repo1), (_, repo2)) = tokio::try_join!(
            self.clone(name1, &remote_path),
            self.clone(name2, &remote_path)
        )?;

        Ok((repo1, repo2))
    }
}

#[derive(serde::Deserialize)]
#[serde(transparent)]
pub struct CreateUserResult {
    pub token: String,
}

pub fn branch_commit<'repo>(
    repo: &'repo Repository,
    branch_name: &str,
) -> TestResult<git2::Commit<'repo>> {
    let br = repo.find_branch(branch_name, BranchType::Local)?;
    let commit = br.get().peel_to_commit()?;

    Ok(commit)
}

pub fn assert_same_trunk(repo_a: &Repository, repo_b: &Repository) -> TestResult<()> {
    let commit_a = branch_commit(repo_a, "trunk")?;
    let commit_b = branch_commit(repo_b, "trunk")?;

    if commit_a.id() != commit_b.id() {
        bail!(
            "trunk commit mismatch: {commit_a} != {commit_b}",
            commit_a = commit_a.id(),
            commit_b = commit_b.id(),
        );
    }

    Ok(())
}

pub struct Cmd<'a> {
    program: &'a OsStr,
    args: Vec<OsString>,
}

impl<'a> Display for Cmd<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.program)?;

        for arg in &self.args {
            write!(f, " {:?}", arg)?;
        }

        Ok(())
    }
}

impl<'a> Cmd<'a> {
    fn new(program: &'a OsStr) -> Self {
        Self {
            program,
            args: Vec::new(),
        }
    }

    pub fn arg(&mut self, arg: impl Into<OsString>) -> &mut Self {
        self.args.push(arg.into());
        self
    }
}

pub struct GitRemoteRef {
    protocol: GitAccessProtocol,
    path: String,
}

pub struct GitRemoteRefBuilder {
    protocol: GitAccessProtocol,
    path: Option<String>,
}

impl GitRemoteRefBuilder {
    pub fn new() -> Self {
        Self {
            protocol: GitAccessProtocol::Http,
            path: None,
        }
    }

    pub fn set_protocol(&mut self, protocol: GitAccessProtocol) -> &mut Self {
        self.protocol = protocol;
        self
    }

    pub fn set_path(&mut self, path: impl Into<String>) -> &mut Self {
        self.path = Some(path.into());
        self
    }

    pub fn protocol(mut self, protocol: GitAccessProtocol) -> Self {
        self.set_protocol(protocol);
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.set_path(path);
        self
    }

    pub fn build(self) -> TestResult<GitRemoteRef> {
        let protocol = self.protocol;
        let path = self.path.ok_or_else(|| format_err!("Path not set"))?;

        Ok(GitRemoteRef { protocol, path })
    }
}

pub enum GitAccessProtocol {
    Http,
    Git,
    Ssh,
}

pub fn upsilon_global(rb: GitRemoteRefBuilder) -> GitRemoteRefBuilder {
    rb.protocol(GitAccessProtocol::Http).path("upsilon")
}

pub fn upsilon_global_git_protocol(rb: GitRemoteRefBuilder) -> GitRemoteRefBuilder {
    rb.protocol(GitAccessProtocol::Git).path("upsilon")
}

pub fn upsilon_global_ssh(rb: GitRemoteRefBuilder) -> GitRemoteRefBuilder {
    rb.protocol(GitAccessProtocol::Ssh).path("upsilon")
}
