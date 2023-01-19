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

#![feature(inherent_associated_types)]

pub extern crate anyhow;
pub extern crate git2;
pub extern crate log;
pub extern crate serde_json;
pub extern crate upsilon_test_support_macros;

use std::collections::HashMap;
use std::fmt::Display;
use std::future::Future;
use std::io::Write;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context, Poll};
use std::time::Duration;

use anyhow::bail;
use log::{error, info};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::Child;
pub use upsilon_test_support_macros::upsilon_test;

pub use crate::client::Client;

mod client;

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct Username(String);

impl From<String> for Username {
    fn from(value: String) -> Self {
        Self(value)
    }
}

pub struct Token(String);

pub struct TestCx {
    client: Client,
    root: String,
    git_protocol_root: String,
    ssh_protocol_root: String,
    child: Child,
    config: TestCxConfig,
    kfile: PathBuf,

    required_online: bool,

    tokens: HashMap<Username, Token>,

    cleaned_up: bool,
}

impl TestCx {
    pub type Config = TestCxConfig;

    pub async fn with_client<F, Fut, R>(&self, f: F) -> R
    where
        F: FnOnce(Client) -> Fut,
        Fut: Future<Output = R>,
    {
        f(self.client.clone()).await
    }

    pub async fn with_client_as_user<F, Fut, R>(&self, user: impl Into<String>, f: F) -> R
    where
        F: FnOnce(Client) -> Fut,
        Fut: Future<Output = R>,
    {
        match self.tokens.get(&Username(user.into())) {
            None => f(self.client.clone()).await,
            Some(token) => {
                let client = self.client.with_token(&token.0);

                async move { f(client).await }.await
            }
        }
    }

    async fn dump_stream(
        name: impl Display,
        stream: Option<&mut (impl AsyncRead + Unpin)>,
        mut output: impl AsyncWrite + Unpin,
    ) -> std::io::Result<()> {
        let Some(stream) = stream else {return Ok(())};

        let mut buffer = Vec::with_capacity(32 * 1024);
        stream.read_to_end(&mut buffer).await?;

        let sep = "\n".repeat(6);

        output
            .write_all(
                format!(
                    "{sep}{start}{sep}",
                    start = format_args!("===== {name} [START] =====")
                )
                .as_bytes(),
            )
            .await?;

        output.write(&buffer).await?;

        output
            .write_all(
                format!(
                    "{sep}{end}{sep}",
                    end = format_args!("===== {name} [END] =====")
                )
                .as_bytes(),
            )
            .await?;

        Ok(())
    }

    async fn dump_streams(child: &mut Child) -> std::io::Result<()> {
        Self::dump_streams_for("server", child).await
    }

    async fn dump_streams_for(name: impl Display, child: &mut Child) -> std::io::Result<()> {
        Self::dump_stream(
            format_args!("{name} stdout"),
            child.stdout.as_mut(),
            tokio::io::stdout(),
        )
        .await?;
        Self::dump_stream(
            format_args!("{name} stderr"),
            child.stderr.as_mut(),
            tokio::io::stderr(),
        )
        .await?;

        Ok(())
    }

    pub async fn init(config: TestCxConfig) -> Self {
        pretty_env_logger::init_custom_env("UPSILON_TESTSUITE_LOG");

        let workdir = config.workdir();

        if workdir.exists() {
            tokio::fs::remove_dir_all(&workdir)
                .await
                .expect("Failed to clean workdir");
        }

        tokio::fs::create_dir_all(&workdir)
            .await
            .expect("Failed to create workdir");

        const CONFIG_FILE: &str = "upsilon.yaml";

        let config_path = workdir.join(CONFIG_FILE);

        tokio::fs::write(&config_path, &config.config)
            .await
            .expect("Failed to write config file");

        let path = {
            let path = env_var("UPSILON_WEB_BIN");
            PathBuf::from(path)
        };

        let portfile_path = workdir.join(".port");

        if portfile_path.exists() {
            tokio::fs::remove_file(&portfile_path)
                .await
                .expect("Failed to remove port file");
        }

        const K_FILE_NAME: &str = "gracefully-shutdown-k";

        let k_file = workdir.join(K_FILE_NAME);

        let mut cmd = tokio::process::Command::new(path);

        upsilon_gracefully_shutdown::setup_for_graceful_shutdown(&mut cmd, &k_file);

        cmd.env("UPSILON_PORT", config.port.to_string())
            .env(
                "UPSILON_VCS_GIT-PROTOCOL_GIT-DAEMON_PORT",
                config.git_daemon_port.to_string(),
            )
            .env("UPSILON_PORTFILE", &portfile_path)
            .env("UPSILON_CONFIG", &config_path)
            .env("UPSILON_DEBUG_GRAPHQL_ENABLED", "true")
            .env("UPSILON_DEBUG_SHUTDOWN-ENDPOINT", "true")
            .env("UPSILON_WORKERS", "3")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .current_dir(&workdir);

        if config.has_ssh_protocol {
            cmd.env("UPSILON_GIT-SSH_PORT", config.git_ssh_port.to_string());
        }

        let mut child = cmd.spawn().expect("Failed to spawn web server");

        struct WaitForPortFileFuture {
            port_file_path: PathBuf,
        }

        impl Future for WaitForPortFileFuture {
            type Output = ();

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                if self.get_mut().port_file_path.exists() {
                    Poll::Ready(())
                } else {
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
            }
        }

        let wait_for_port_file_fut = WaitForPortFileFuture {
            port_file_path: portfile_path.clone(),
        };

        struct WaitForWebServerExitFuture<'a> {
            child: &'a mut Child,
        }

        impl<'a> Future for WaitForWebServerExitFuture<'a> {
            type Output = std::io::Result<std::process::ExitStatus>;

            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
                let result = self.get_mut().child.try_wait();

                match result {
                    Ok(Some(status)) => Poll::Ready(Ok(status)),
                    Ok(None) => {
                        cx.waker().wake_by_ref();
                        Poll::Pending
                    }
                    Err(e) => Poll::Ready(Err(e)),
                }
            }
        }

        let wait_for_web_server_exit_fut = WaitForWebServerExitFuture { child: &mut child };

        enum Done {
            PortFile,
            WebServerExit(std::io::Result<std::process::ExitStatus>),
            Timeout,
        }

        const PORT_FILE_TIMEOUT: Duration = Duration::from_secs(10);

        let done = tokio::select! {
            _ = wait_for_port_file_fut => {Done::PortFile}
            status = wait_for_web_server_exit_fut => {Done::WebServerExit(status)}
            _ = tokio::time::sleep(PORT_FILE_TIMEOUT) => {Done::Timeout}
        };

        match done {
            Done::PortFile => {}
            Done::WebServerExit(exit_status) => {
                let exit_status = exit_status.expect("Failed to get web server exit status");

                Self::dump_streams(&mut child)
                    .await
                    .expect("Failed to dump streams");

                panic!("Web server exited with status {exit_status}");
            }
            Done::Timeout => {
                panic!(
                    "Failed to start web server in {} seconds",
                    PORT_FILE_TIMEOUT.as_secs()
                );
            }
        }

        let port = tokio::fs::read_to_string(portfile_path)
            .await
            .expect("Failed to read port file");

        let port: u16 = port.trim().parse().expect("Failed to parse port");

        let root = format!("http://localhost:{port}");

        let git_port = config.git_daemon_port;
        let git_protocol_root = format!("git://localhost:{git_port}");

        let git_ssh_port = config.git_ssh_port;
        let ssh_protocol_root = format!("ssh://git@localhost:{git_ssh_port}");

        Self {
            client: Client::new(&root),
            root,
            git_protocol_root,
            ssh_protocol_root,
            child,
            config,
            kfile: k_file,
            required_online: false,
            tokens: HashMap::new(),
            cleaned_up: false,
        }
    }

    pub async fn require_online(&mut self) -> TestResult {
        if !self.required_online {
            if self.config.works_offline {
                bail!(
                    r#"Test requires online mode, but was annotated with #[offline(run)].
Help: Annotate it with `#[offline(ignore)]` instead."#
                );
            }

            self.required_online = true;
        }

        Ok(())
    }

    pub async fn tempdir(&self, name: &str) -> std::io::Result<PathBuf> {
        let mut p = self.config.workdir();
        p.push("tmp");
        p.push(name);

        if p.exists() {
            tokio::fs::remove_dir_all(&p).await?;
        }

        tokio::fs::create_dir_all(&p).await?;

        Ok(p)
    }

    pub async fn finish(&mut self) -> TestResult<()> {
        self.cleaned_up = true;

        tokio::fs::write(&self.kfile, "").await?;

        tokio::time::sleep(Duration::from_secs(1)).await;

        let exited_normally = if self.child.try_wait()?.is_none() {
            info!("Gracefully shutting down the web server");

            upsilon_gracefully_shutdown::gracefully_shutdown(
                &mut self.child,
                Duration::from_secs(10),
            )
            .await;

            false
        } else {
            true
        };

        tokio::time::sleep(Duration::from_secs(1)).await;

        let status = self
            .child
            .try_wait()?
            .expect("Child should have exited by now");

        Self::dump_streams(&mut self.child).await?;

        if exited_normally && !status.success() {
            bail!("Subprocess failed: {status:?}");
        }

        let workdir = self.config.workdir();

        tokio::fs::remove_dir_all(&workdir)
            .await
            .expect("Failed to delete workdir");

        if !self.config.works_offline && !self.required_online {
            bail!(
                "\
Test seems to work offline, please annotate it with `#[offline]`, or \
call `cx.require_online()` if it really does require online mode"
            );
        }

        Ok(())
    }
}

impl Drop for TestCx {
    fn drop(&mut self) {
        if !self.cleaned_up {
            panic!("TestCx was not cleaned up");
        }
    }
}

pub struct CxConfigVars {
    pub workdir: PathBuf,
    pub test_name: &'static str,
    pub source_file_path_hash: u64,
    pub works_offline: bool,
    pub config_init: fn(&mut TestCxConfig),
}

pub struct TestCxConfig {
    port: u16,
    git_daemon_port: u16,
    git_ssh_port: u16,
    config: String,
    tempdir: PathBuf,
    source_file_path_hash: u64,
    test_name: &'static str,
    works_offline: bool,
    has_git_protocol: bool,
    has_ssh_protocol: bool,
}

impl TestCxConfig {
    pub fn new(vars: &CxConfigVars) -> Self {
        let mut test_cx_config = Self {
            port: 0,
            git_daemon_port: 0,
            git_ssh_port: 0,
            config: "".to_string(),
            tempdir: vars.workdir.clone(),
            source_file_path_hash: vars.source_file_path_hash,
            test_name: vars.test_name,
            works_offline: vars.works_offline,
            has_git_protocol: false,
            has_ssh_protocol: false,
        };

        (vars.config_init)(&mut test_cx_config);

        test_cx_config
    }

    pub fn with_port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    pub fn with_git_daemon_port(&mut self, port: u16) -> &mut Self {
        self.git_daemon_port = port;
        self
    }

    pub fn with_git_protocol(&mut self) -> &mut Self {
        self.has_git_protocol = true;
        self
    }

    pub fn with_git_ssh_port(&mut self, port: u16) -> &mut Self {
        self.git_ssh_port = port;
        self
    }

    pub fn with_ssh_protocol(&mut self) -> &mut Self {
        self.has_ssh_protocol = true;
        self
    }

    pub fn with_config(&mut self, config: impl Into<String>) -> &mut Self {
        self.config = config.into();
        self
    }

    fn workdir(&self) -> PathBuf {
        let mut p = self.tempdir.clone();
        p.push(self.source_file_path_hash.to_string());
        p.push(self.test_name);
        p
    }
}

pub type TestError = anyhow::Error;
pub type TestResult<T = ()> = Result<T, TestError>;
pub use client::Anything;

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IdHolder {
    pub id: String,
}

pub mod helpers;

pub mod json_diff;

pub mod prelude {
    pub use anyhow::bail;
    pub use serde_json::json;

    pub use crate::helpers::*;
    pub use crate::{
        assert_json_eq, gql_vars, upsilon_test, Anything, Client, IdHolder, TestCx, TestCxConfig, TestResult
    };
}

fn env_var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        panic!("{name} not set; did you use `cargo xtask test` to run the tests?")
    })
}
