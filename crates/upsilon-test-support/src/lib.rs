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
pub extern crate upsilon_test_support_macros;

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

use git2::Repository;
pub use upsilon_test_support_macros::upsilon_test;

use crate::client::Client;

mod client;

pub struct TestCx {
    client: Client,
    root: String,
    child: tokio::process::Child,
    config: TestCxConfig,
}

impl TestCx {
    pub type Config = TestCxConfig;

    pub async fn with_client<'a, F, Fut, R>(&'a self, f: F) -> R
    where
        F: FnOnce(&'a Client) -> Fut,
        Fut: Future<Output = R> + 'a,
    {
        f(&self.client).await
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

        if let Some(cfg) = &config.config {
            #[cfg(debug_assertions)]
            const CONFIG_FILE: &str = "upsilon.dev.yaml";
            #[cfg(not(debug_assertions))]
            const CONFIG_FILE: &str = "upsilon.yaml";

            let config_path = workdir.join(CONFIG_FILE);

            std::fs::write(&config_path, cfg).expect("Failed to write config file");
        }

        let path = {
            let mut path = std::env::current_exe().unwrap();
            path.pop(); // target/debug/deps
            path.pop(); // target/debug
            path.push("upsilon-web"); // target/debug/upsilon-web
            path.set_extension(std::env::consts::EXE_EXTENSION);
            path
        };

        let port_file_path = workdir.join(".port");

        if port_file_path.exists() {
            tokio::fs::remove_file(&port_file_path)
                .await
                .expect("Failed to remove port file");
        }

        let mut child = tokio::process::Command::new(path)
            .env("UPSILON_PORT", config.port.to_string())
            .env("UPSILON_PORT_FILE", &port_file_path)
            .kill_on_drop(true)
            .current_dir(&workdir)
            .spawn()
            .expect("Failed to spawn web server");

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
            port_file_path: port_file_path.clone(),
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

        let port = tokio::fs::read_to_string(port_file_path)
            .await
            .expect("Failed to read port file");

        let port: u16 = port.trim().parse().expect("Failed to parse port");

        let root = format!("http://localhost:{}", port);

        Self {
            client: Client::new(&root),
            root,
            child,
            config,
        }
    }

    pub async fn tempdir(&self, name: &str) -> std::io::Result<PathBuf> {
        let mut p = self.config.workdir();
        p.push("tmp");
        p.push(name);

        tokio::fs::create_dir_all(&p).await?;

        Ok(p)
    }

    pub async fn clone(&self, name: &str, remote_path: &str) -> TestResult<(PathBuf, Repository)> {
        let path = self.tempdir(name).await?;
        let repo = Repository::clone(&format!("{}/{remote_path}", self.root), &path)?;

        Ok((path, repo))
    }

    pub async fn finish(mut self) {
        if self
            .child
            .try_wait()
            .expect("Failed to check if the web server is running")
            .is_none()
        {
            self.child
                .kill()
                .await
                .expect("Failed to kill the webserver");
        }

        let workdir = self.config.workdir();

        tokio::fs::remove_dir_all(&workdir)
            .await
            .expect("Failed to delete workdir");
    }
}

pub struct CxConfigVars {
    pub workdir: PathBuf,
    pub test_name: &'static str,
    pub source_file_path_hash: u64,
}

pub struct TestCxConfig {
    port: u16,
    config: Option<String>,
    tempdir: PathBuf,
    source_file_path_hash: u64,
    test_name: &'static str,
}

impl TestCxConfig {
    pub fn new(vars: &CxConfigVars) -> Self {
        Self {
            port: 0,
            config: None,
            tempdir: vars.workdir.clone(),
            source_file_path_hash: vars.source_file_path_hash,
            test_name: vars.test_name,
        }
    }

    pub fn with_port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    pub fn with_config(&mut self, config: impl Into<String>) -> &mut Self {
        self.config = Some(config.into());
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

    pub use crate::helpers::*;
    pub use crate::{
        assert_json_eq, upsilon_test, Anything, IdHolder, TestCx, TestCxConfig, TestResult
    };
}
