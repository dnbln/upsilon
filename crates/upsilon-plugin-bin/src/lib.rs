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

use std::ffi::{OsString};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Stdio;

use log::{error, info, trace};
use serde::de::Error;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use upsilon_plugin_core::rocket::{async_trait, Orbit, Rocket};
use upsilon_plugin_core::PluginLoadApi;

pub struct PluginBinConfig {
    pub pass_port_option: bool,
}

pub struct PluginBin {
    pub name: &'static str,
    pub bin: PathBuf,

    pub plugin_bin_config: PluginBinConfig,
}

struct PluginBinOptions {
    options: Vec<OsString>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct BinConfigEnvName {
    pub env_name: &'static str,
}

impl BinConfigEnvName {
    pub const fn new(env_name: &'static str) -> Self {
        Self { env_name }
    }

    pub fn load_from_env<C: for<'de> serde::Deserialize<'de> + 'static>(
        &self,
    ) -> Result<C, serde_json::Error> {
        let config = std::env::var(self.env_name).map_err(serde_json::Error::custom)?;
        serde_json::from_str(&config)
    }
}

pub struct BinConfig<C: serde::Serialize + Send> {
    pub env_name: BinConfigEnvName,
    pub config: C,
}

impl<C> BinConfig<C>
where
    C: serde::Serialize + Send,
{
    pub fn new(env_name: BinConfigEnvName, config: C) -> Self {
        Self { env_name, config }
    }

    fn to_env(&self) -> Result<(&'static str, String), serde_json::Error> {
        let config = serde_json::to_string(&self.config)?;
        Ok((self.env_name.env_name, config))
    }
}

#[async_trait]
pub trait PluginLoadApiBinExt {
    async fn run_bin_on_liftoff<C, ConfigPatch>(
        &mut self,
        bin: PluginBin,
        config: BinConfig<C>,
        config_patch: ConfigPatch,
    ) where
        C: serde::Serialize + Send + 'static,
        ConfigPatch: for<'a> FnOnce(
                &'a Rocket<Orbit>,
                &'a mut C,
            ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
            + Send
            + 'static;

    async fn run_bin_on_liftoff_no_config(&mut self, bin: PluginBin);
}

#[async_trait]
impl<'registry> PluginLoadApiBinExt for dyn PluginLoadApi<'registry> {
    async fn run_bin_on_liftoff<C, ConfigPatch>(
        &mut self,
        bin: PluginBin,
        config: BinConfig<C>,
        config_patch: ConfigPatch,
    ) where
        C: serde::Serialize + Send + 'static,
        ConfigPatch: for<'a> FnOnce(
                &'a Rocket<Orbit>,
                &'a mut C,
            ) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>
            + Send
            + 'static,
    {
        self.register_liftoff_hook(format!("Run {}", bin.name), move |rocket| {
            let fut = async move {
                let mut config = config;

                config_patch(rocket, &mut config.config).await;

                let config = match config.to_env() {
                    Ok(c) => Some(c),
                    Err(e) => {
                        error!("Failed to serialize config for {}: {e}", bin.name);
                        None
                    }
                };

                let mut opts = PluginBinOptions { options: vec![] };

                if bin.plugin_bin_config.pass_port_option {
                    opts.options.push("--port".into());
                    opts.options.push(rocket.config().port.to_string().into());
                }

                tokio::spawn(async move {
                    let r = run_bin_task(&bin, opts, config).await;

                    if let Err(e) = r {
                        error!("Failed to run {}: {e}", bin.name);
                    }
                });
            };

            Box::pin(fut)
        })
        .await;
    }

    async fn run_bin_on_liftoff_no_config(&mut self, bin: PluginBin) {
        self.run_bin_on_liftoff(
            bin,
            BinConfig {
                env_name: BinConfigEnvName::new("__upsilon_plugin_bin_noconfig"),
                config: (),
            },
            |_, _| Box::pin(async {}),
        )
        .await;
    }
}

async fn run_bin_task(
    bin: &PluginBin,
    opts: PluginBinOptions,
    config: Option<(&'static str, String)>,
) -> Result<(), std::io::Error> {
    trace!("Running {}", bin.name);
    let mut cmd = Command::new(&bin.bin);

    for opt in opts.options {
        cmd.arg(opt);
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    if let Some((config_name, config)) = config {
        cmd.env(config_name, config);
    }

    let mut child = cmd.spawn()?;

    trace!("Waiting for {}", bin.name);

    let exit_status = child.wait().await?;

    info!("{} exited with status: {exit_status}", bin.name);

    let stdout_pipe = child.stdout.as_mut().expect("failed to get stdout pipe");
    let stderr_pipe = child.stderr.as_mut().expect("failed to get stderr pipe");

    use std::io::Write;

    let mut stderr = std::io::stderr();
    let guard = "=".repeat(30);

    {
        let mut stdout_str = String::new();

        stdout_pipe.read_to_string(&mut stdout_str).await?;

        if !stdout_str.is_empty() {
            write!(
                &mut stderr,
                "{} stdout:\n{guard}\n{stdout_str}{guard}\n",
                bin.name
            )?;
        }
    }

    {
        let mut stderr_str = String::new();
        stderr_pipe.read_to_string(&mut stderr_str).await?;

        if !stderr_str.is_empty() {
            write!(
                &mut stderr,
                "{} stderr:\n{guard}\n{stderr_str}{guard}\n",
                bin.name
            )?;
        }
    }

    if !exit_status.success() {
        error!(
            "{} exited with non-zero status code: {exit_status}",
            bin.name,
        );
    } else {
        info!("{} finished successfully", bin.name);
    }

    Ok(())
}
