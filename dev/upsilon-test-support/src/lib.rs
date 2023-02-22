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

#![allow(incomplete_features)]
#![feature(inherent_associated_types)]

pub extern crate anyhow;
pub extern crate futures;
pub extern crate git2;
pub extern crate log;
pub extern crate serde_json;
pub extern crate upsilon_test_support_macros;

use std::any::Any;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;
use std::task::{Context as AsyncCx, Poll};
use std::time::Duration;

use anyhow::{bail, format_err, Context};
use log::info;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::Child;
pub use upsilon_test_support_macros::upsilon_test;

pub use crate::client::Client;

mod client;

#[derive(Ord, PartialOrd, Eq, PartialEq, Hash, Clone)]
pub struct Username(String);

impl From<String> for Username {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl<'a> From<&'a str> for Username {
    fn from(value: &'a str) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone)]
pub struct Token(String);

struct TestRtPanicInfo {
    payload: Box<dyn Any + Send>,
}

impl From<Box<dyn Any + Send>> for TestRtPanicInfo {
    fn from(payload: Box<dyn Any + Send>) -> Self {
        Self { payload }
    }
}

struct TestRt {
    panic: Option<TestRtPanicInfo>,
}

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
    rt: TestRt,
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

    pub async fn with_client_as_user<F, Fut, R>(&self, user: impl Into<Username>, f: F) -> R
    where
        F: FnOnce(Client) -> Fut,
        Fut: Future<Output = R>,
    {
        match self.tokens.get(&user.into()) {
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

        if buffer.is_empty() {
            return Ok(());
        }

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

        output.write_all(&buffer).await?;

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

    pub async fn init(config: TestCxConfig) -> TestResult<Self> {
        pretty_env_logger::init_custom_env("UPSILON_TESTSUITE_LOG");

        #[cfg(difftests)]
        let difftests_env = {
            let pkg_name = config.pkg_name;
            let crate_name = config.crate_name;
            let bin_name = config.bin_name;
            let bin_path = config.bin_path.clone();
            let test_name = config.test_name;
            let mut difftests_tempdir = config.tempdir.join("upsilon-difftests");
            difftests_tempdir.push(pkg_name);
            difftests_tempdir.push(crate_name);
            if let Some(bin_name) = bin_name {
                difftests_tempdir.push(bin_name);
            }
            difftests_tempdir.push(test_name);
            let env = cargo_difftests_testclient::init(
                cargo_difftests_testclient::TestDesc {
                    pkg_name: pkg_name.to_string(),
                    crate_name: crate_name.to_string(),
                    bin_name: bin_name.map(ToString::to_string),
                    bin_path,
                    test_name: test_name.to_string(),
                    other_fields: std::collections::HashMap::new(),
                },
                &difftests_tempdir,
            )?;

            env
        };

        let workdir = config.workdir();

        if workdir.exists() {
            tokio::fs::remove_dir_all(&workdir)
                .await
                .context("Failed to remove workdir")?;
        }

        tokio::fs::create_dir_all(&workdir)
            .await
            .context("Failed to create workdir")?;

        const CONFIG_FILE: &str = "upsilon.yaml";

        let config_path = workdir.join(CONFIG_FILE);

        tokio::fs::write(&config_path, &config.config)
            .await
            .context("Failed to write config file")?;

        let path = &config.upsilon_web_bin;

        let portfile_path = workdir.join(".port");

        if portfile_path.exists() {
            tokio::fs::remove_file(&portfile_path)
                .await
                .context("Failed to remove port file")?;
        }

        const K_FILE_NAME: &str = "gracefully-shutdown-k";

        let kfile = workdir.join(K_FILE_NAME);

        let mut cmd = tokio::process::Command::new(path);

        upsilon_gracefully_shutdown::setup_for_graceful_shutdown(
            &mut cmd,
            &config.gracefully_shutdown_host_bin,
            &kfile,
            Duration::from_secs(60),
        );

        cmd.env("UPSILON_PORT", config.port.to_string())
            .env(
                "UPSILON_VCS_GIT-PROTOCOL_PORT",
                config.git_daemon_port.to_string(),
            )
            .env("UPSILON_PLUGINS_PORTFILE-WRITER_PORTFILE", &portfile_path)
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

        #[cfg(difftests)]
        cmd.envs(difftests_env.env_for_children());

        let mut child = cmd.spawn().context("Failed to spawn web server")?;

        struct WaitForPortFileFuture {
            port_file_path: PathBuf,
        }

        impl Future for WaitForPortFileFuture {
            type Output = ();

            fn poll(self: Pin<&mut Self>, cx: &mut AsyncCx<'_>) -> Poll<Self::Output> {
                let this = self.get_mut();
                let file = &this.port_file_path;

                if file.exists() {
                    let Ok(metadata) = file.metadata() else {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    };

                    if metadata.len() == 0 {
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }

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

            fn poll(self: Pin<&mut Self>, cx: &mut AsyncCx<'_>) -> Poll<Self::Output> {
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
                let exit_status = exit_status.context("Failed to get web server exit status")?;

                Self::dump_streams(&mut child)
                    .await
                    .context("Failed to dump streams")?;

                bail!("Web server exited with status {exit_status}");
            }
            Done::Timeout => {
                bail!(
                    "Failed to start web server in {} seconds",
                    PORT_FILE_TIMEOUT.as_secs()
                );
            }
        }

        let port = tokio::fs::read_to_string(portfile_path)
            .await
            .context("Failed to read port file")?;

        let port: u16 = port.trim().parse().expect("Failed to parse port");

        let root = format!("http://localhost:{port}");

        let git_port = config.git_daemon_port;
        let git_protocol_root = format!("git://localhost:{git_port}");

        let git_ssh_port = config.git_ssh_port;
        let ssh_protocol_root = format!("ssh://git@localhost:{git_ssh_port}");

        let cx = Self {
            client: Client::new(&root),
            root,
            git_protocol_root,
            ssh_protocol_root,
            child,
            config,
            kfile,
            required_online: false,
            tokens: HashMap::new(),
            cleaned_up: false,
            rt: TestRt { panic: None },
        };

        Ok(cx)
    }

    pub fn set_panic_info(&mut self, pi: Box<dyn Any + Send>) {
        self.rt.panic = Some(TestRtPanicInfo::from(pi));
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

    pub async fn finish(&mut self, result: TestResult) -> TestResult<()> {
        let result = self.finish_impl(result).await;

        if let Some(panic_info) = self.rt.panic.take() {
            std::panic::resume_unwind(panic_info.payload);
        } else {
            result
        }
    }

    async fn finish_impl(&mut self, result: TestResult) -> TestResult<()> {
        if self.cleaned_up {
            return Ok(());
        }

        self.cleaned_up = true;

        tokio::fs::write(&self.kfile, "").await?;

        while tokio::fs::read(&self.kfile).await?.is_empty() && self.child.try_wait()?.is_none() {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }

        let _ = self.client.post_empty("/api/shutdown").await;

        let child_wait_fut = self.child.wait();

        let r = tokio::time::timeout(Duration::from_secs(20), child_wait_fut).await;

        let (exited_normally, status) = match r {
            Err(_) => {
                info!("Gracefully shutting down the web server");

                upsilon_gracefully_shutdown::gracefully_shutdown(
                    &mut self.child,
                    Duration::from_secs(60),
                )
                .await;

                (
                    false,
                    self.child
                        .try_wait()?
                        .ok_or_else(|| format_err!("Child should have exited by now"))?,
                )
            }
            Ok(status) => (true, status?),
        };

        Self::dump_streams(&mut self.child).await?;

        if exited_normally && !status.success() {
            bail!("Subprocess failed: {status:?}");
        }

        let workdir = self.config.workdir();

        tokio::fs::remove_dir_all(&workdir).await?;

        if !self.config.works_offline && !self.required_online {
            bail!(
                "\
Test seems to work offline, please annotate it with `#[offline]`, or \
call `cx.require_online()` if it really does require online mode"
            );
        }

        let _ = result?;

        Ok(())
    }
}

impl Drop for TestCx {
    fn drop(&mut self) {
        if !self.cleaned_up && !::std::thread::panicking() {
            panic!("TestCx was not cleaned up");
        }
    }
}

pub struct CxConfigVars {
    pub workdir: PathBuf,
    pub upsilon_web_bin: PathBuf,
    pub gracefully_shutdown_host_bin: PathBuf,
    pub crate_name: &'static str,
    pub pkg_name: &'static str,
    pub bin_name: Option<&'static str>,
    pub bin_path: PathBuf,
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
    upsilon_web_bin: PathBuf,
    gracefully_shutdown_host_bin: PathBuf,
    source_file_path_hash: u64,
    crate_name: &'static str,
    pkg_name: &'static str,
    bin_name: Option<&'static str>,
    bin_path: PathBuf,
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
            upsilon_web_bin: vars.upsilon_web_bin.clone(),
            gracefully_shutdown_host_bin: vars.gracefully_shutdown_host_bin.clone(),
            source_file_path_hash: vars.source_file_path_hash,
            crate_name: vars.crate_name,
            pkg_name: vars.pkg_name,
            bin_name: vars.bin_name,
            bin_path: vars.bin_path.clone(),
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

pub use upsilon_json_diff::assert_json_eq;

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

fn env_var_path(name: &str) -> PathBuf {
    PathBuf::from(env_var(name))
}

pub fn pre_init_test() {
    #[cfg(difftests)]
    {
        cargo_difftests_testclient::pre_init_test();
    }
}