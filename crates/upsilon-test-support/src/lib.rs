/*
 *        Copyright (c) 2022 Dinu Blanovschi
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
pub extern crate serde_json;
pub extern crate upsilon_test_support_macros;

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

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
    child: tokio::process::Child,
    config: TestCxConfig,

    required_online: bool,

    tokens: HashMap<Username, Token>,
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

    pub async fn init(config: TestCxConfig) -> Self {
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

        let mut cmd = tokio::process::Command::new(path);

        upsilon_gracefully_shutdown::setup_for_graceful_shutdown(&mut cmd);

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
            .kill_on_drop(true)
            .current_dir(&workdir);

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
            child: &'a mut tokio::process::Child,
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

        Self {
            client: Client::new(&root),
            root,
            git_protocol_root,
            child,
            config,
            required_online: false,
            tokens: HashMap::new(),
        }
    }

    pub async fn require_online(&mut self) {
        if !self.required_online {
            if self.config.works_offline {
                panic!(
                    r#"Test requires online mode, but was annotated with #[offline(run)].
Help: Annotate it with `#[offline(ignore)]` instead."#);
            }

            self.required_online = true;
        }
    }

    pub async fn tempdir(&self, name: &str) -> std::io::Result<PathBuf> {
        let mut p = self.config.workdir();
        p.push("tmp");
        p.push(name);

        tokio::fs::create_dir_all(&p).await?;

        Ok(p)
    }

    pub async fn finish(mut self) {
        let exited_normally = if self
            .child
            .try_wait()
            .expect("Failed to check if the web server is running")
            .is_none()
        {
            println!("Gracefully shutting down the web server");

            let _ = self.client.post_empty("/api/shutdown").await;

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
            .try_wait()
            .expect("Child should have exited by now")
            .expect("Child should have exited by now");

        if exited_normally && !status.success() {
            panic!("Subprocess failed: {status:?}");
        }

        let workdir = self.config.workdir();

        tokio::fs::remove_dir_all(&workdir)
            .await
            .expect("Failed to delete workdir");

        if !self.config.works_offline && !self.required_online {
            panic!("Test works offline, please annotate it with `#[offline]`");
        }
    }
}

pub struct CxConfigVars {
    pub workdir: PathBuf,
    pub test_name: &'static str,
    pub source_file_path_hash: u64,
    pub works_offline: bool,
}

pub struct TestCxConfig {
    port: u16,
    git_daemon_port: u16,
    config: String,
    tempdir: PathBuf,
    source_file_path_hash: u64,
    test_name: &'static str,
    works_offline: bool,
}

impl TestCxConfig {
    pub fn new(vars: &CxConfigVars) -> Self {
        let mut test_cx_config = Self {
            port: 0,
            git_daemon_port: 0,
            config: "".to_string(),
            tempdir: vars.workdir.clone(),
            source_file_path_hash: vars.source_file_path_hash,
            test_name: vars.test_name,
            works_offline: vars.works_offline,
        };

        helpers::upsilon_basic_config(&mut test_cx_config);

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
        assert_json_eq, upsilon_test, Anything, Client, IdHolder, TestCx, TestCxConfig, TestResult
    };
}

fn env_var(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        panic!("{name} not set; did you use `cargo xtask test` to run the tests?")
    })
}
