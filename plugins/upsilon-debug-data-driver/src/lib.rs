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

use upsilon_core::config::Cfg;
use upsilon_plugin_bin::{
    BinConfig, BinConfigEnvName, PluginBin, PluginBinConfig, PluginLoadApiBinExt
};
use upsilon_plugin_core::{
    Plugin, PluginConfig, PluginError, PluginLoad, PluginLoadApi, PluginMetadata
};
use upsilon_vcs::UpsilonVcsConfig;

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
        Ok(CONFIG_ENV_VAR.load_from_env()?)
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

            load.run_bin_on_liftoff(
                PluginBin {
                    name: "Debug data driver",
                    bin: upsilon_core::alt_exe("upsilon-debug-data-driver"),
                    plugin_bin_config: PluginBinConfig {
                        pass_port_option: true,
                    },
                },
                BinConfig {
                    env_name: CONFIG_ENV_VAR,
                    config,
                },
                |rocket, config| {
                    Box::pin(async move {
                        let vcs_cfg = rocket
                            .state::<Cfg<UpsilonVcsConfig>>()
                            .expect("VCS config not found");

                        for (repo_name, repo) in &mut config.repos {
                            repo.exists = upsilon_vcs::exists_global(vcs_cfg, &repo_name.0);
                        }
                    })
                },
            )
            .await;

            Ok::<_, PluginError>(())
        };

        Box::pin(fut)
    }
}

const CONFIG_ENV_VAR: BinConfigEnvName = BinConfigEnvName::new("DDD_CONFIG");
