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
use std::future::Future;
use std::pin::Pin;
use std::process::Stdio;

use log::{error, info, trace};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use upsilon_core::config::Cfg;
use upsilon_plugin_core::{
    Plugin, PluginConfig, PluginError, PluginLoad, PluginLoadApi, PluginMetadata
};

#[cfg_attr(feature = "dynamic-plugins", no_mangle)]
pub const __UPSILON_METADATA: PluginMetadata =
    PluginMetadata::new("upsilon-debug-data-driver", "0.0.1");

#[cfg_attr(feature = "dynamic-plugins", no_mangle)]
pub const __UPSILON_PLUGIN: PluginLoad = load_debug_data_driver;

fn load_debug_data_driver(config: PluginConfig) -> Result<Box<dyn Plugin>, PluginError> {
    let config = config.deserialize::<DebugDataDriverConfig>()?;
    Ok(Box::new(DebugDataDriverPlugin { config }))
}

#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq,
)]
#[serde(transparent)]
pub struct DebugDataDriverUsername(pub String);

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DebugDataDriverUserPrivateFields {
    pub password: String,
    pub email: String,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
#[serde(transparent)]
pub struct DebugDataDriverUserMap {
    pub users: HashMap<DebugDataDriverUsername, DebugDataDriverUserPrivateFields>,
}

#[derive(
    serde::Serialize, serde::Deserialize, Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq,
)]
#[serde(transparent)]
pub struct DebugDataDriverRepoName(pub String);

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DebugDataDriverRepoConfig {
    #[serde(default = "false_f")]
    pub exists: bool,
    #[serde(default)]
    pub setup_mirror_from: Option<String>,
}

fn false_f() -> bool {
    false
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct DebugDataDriverConfig {
    #[serde(default)]
    pub users: DebugDataDriverUserMap,
    #[serde(default)]
    pub repos: HashMap<DebugDataDriverRepoName, DebugDataDriverRepoConfig>,
}

impl DebugDataDriverConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let config = std::env::var(CONFIG_ENV_VAR)?;
        let config = serde_json::from_str(&config)?;

        Ok(config)
    }
}

pub struct DebugDataDriverPlugin {
    config: DebugDataDriverConfig,
}

impl Plugin for DebugDataDriverPlugin {
    fn init<'a, 'b, 'registry, 'fut>(
        &'a mut self,
        load: &'b mut dyn PluginLoadApi<'registry>,
    ) -> Pin<Box<dyn Future<Output = Result<(), PluginError>> + Send + 'fut>>
    where
        'a: 'fut,
        'registry: 'b,
        'b: 'fut,
    {
        let fut = async move {
            let config = self.config.clone();

            load.register_liftoff_hook("Debug data driver", move |rocket| {
                Box::pin(async move {
                    let vcs_cfg = rocket
                        .state::<Cfg<upsilon_vcs::UpsilonVcsConfig>>()
                        .expect("Missing vcs config");

                    let mut config = config;

                    for (repo_name, repo) in &mut config.repos {
                        if upsilon_vcs::exists_global(vcs_cfg, &repo_name.0) {
                            repo.exists = true;
                        }
                    }

                    let port = rocket.config().port;

                    tokio::spawn(async move {
                        let result = debug_data_driver_task(port, &config).await;

                        if let Err(e) = result {
                            error!("Failed to run debug data driver: {e}");
                        }
                    });
                })
            })
            .await;

            Ok::<_, PluginError>(())
        };

        Box::pin(fut)
    }
}

const CONFIG_ENV_VAR: &str = "DDD_CONFIG";

async fn debug_data_driver_task(
    port: u16,
    config: &DebugDataDriverConfig,
) -> Result<(), std::io::Error> {
    let debug_data_driver = upsilon_core::alt_exe("upsilon-debug-data-driver");

    let mut cmd = Command::new(debug_data_driver);

    cmd.arg("--port")
        .arg(port.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("RUST_LOG", "INFO");

    cmd.env(CONFIG_ENV_VAR, serde_json::to_string(config).unwrap());

    let mut child = cmd.spawn()?;

    trace!("Waiting for debug data driver");

    let exit_status = child.wait().await?;

    info!("Debug data driver exited with status: {}", exit_status);

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
                "Debug Data Driver stdout:\n{guard}\n{stdout_str}{guard}\n",
            )?;
        }
    }

    {
        let mut stderr_str = String::new();
        stderr_pipe.read_to_string(&mut stderr_str).await?;

        if !stderr_str.is_empty() {
            write!(
                &mut stderr,
                "Debug Data Driver stderr:\n{guard}\n{stderr_str}{guard}\n",
            )?;
        }
    }

    if !exit_status.success() {
        error!(
            "Debug data driver exited with non-zero status code: {}",
            exit_status
        );
    } else {
        info!("Debug data driver finished successfully");
    }

    Ok(())
}
