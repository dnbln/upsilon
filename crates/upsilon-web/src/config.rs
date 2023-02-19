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

use std::path::PathBuf;

use serde::{Deserialize, Deserializer};
use upsilon_core::config::{GqlDebugConfig, UsersConfig};
use upsilon_ssh_russh::{CompleteRusshServerConfig, RusshServerConfig};
use upsilon_vcs::UpsilonVcsConfig;

use crate::data::DataBackendConfig;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub vcs: UpsilonVcsConfig,
    #[serde(default, rename = "git-ssh")]
    pub git_ssh: Option<GitSshProtocol>,
    #[serde(rename = "data-backend")]
    pub data_backend: DataBackendConfig,

    pub users: UsersConfig,

    pub frontend: FrontendConfig,

    #[serde(rename = "vcs-errors", default)]
    pub vcs_errors: VcsErrorsConfig,

    pub debug: DebugConfig,
}

#[derive(Debug)]
pub enum FrontendConfig {
    Enabled { frontend_root: PathBuf },
    Disabled,
}

impl<'de> Deserialize<'de> for FrontendConfig {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(serde::Deserialize)]
        pub struct FrontendConfigTemp {
            enabled: bool,
            #[serde(default)]
            root: Option<PathBuf>,
        }

        let config = FrontendConfigTemp::deserialize(deserializer)?;

        match config {
            FrontendConfigTemp {
                enabled: true,
                root: Some(frontend_root),
            } => Ok(FrontendConfig::Enabled { frontend_root }),
            FrontendConfigTemp {
                enabled: true,
                root: None,
            } => Err(serde::de::Error::missing_field("root")),
            FrontendConfigTemp {
                enabled: false,
                root: _,
            } => Ok(FrontendConfig::Disabled),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum GitSshProtocol {
    #[serde(rename = "russh")]
    Russh(RusshServerConfigTemp),
}

#[derive(Deserialize, Debug)]
#[serde(from = "RusshServerConfig")]
pub enum RusshServerConfigTemp {
    Incomplete(RusshServerConfig),
    Temp,
    Complete(CompleteRusshServerConfig),
}

impl From<RusshServerConfig> for RusshServerConfigTemp {
    fn from(value: RusshServerConfig) -> Self {
        RusshServerConfigTemp::Incomplete(value)
    }
}

impl RusshServerConfigTemp {
    pub fn complete(&mut self, vcs_config: UpsilonVcsConfig) {
        let old = std::mem::replace(self, RusshServerConfigTemp::Temp);
        if let RusshServerConfigTemp::Incomplete(config) = old {
            *self = RusshServerConfigTemp::Complete(config.complete(vcs_config));
        }
    }

    pub fn port(&self) -> u16 {
        match self {
            RusshServerConfigTemp::Complete(config) => config.port,
            RusshServerConfigTemp::Temp => panic!("RusshServerConfigTemp in Temp state!"),
            RusshServerConfigTemp::Incomplete(config) => config.port,
        }
    }

    pub(crate) fn expect_complete(&self) -> &CompleteRusshServerConfig {
        match self {
            RusshServerConfigTemp::Complete(config) => config,
            RusshServerConfigTemp::Temp => panic!("RusshServerConfigTemp in Temp state!"),
            RusshServerConfigTemp::Incomplete(_) => {
                panic!("RusshServerConfigTemp was not completed!")
            }
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct DebugConfig {
    #[serde(default = "false_f")]
    pub debug_data: bool,

    #[serde(default)]
    pub graphql: GqlDebugConfig,

    #[serde(default = "false_f")]
    pub shutdown_endpoint: bool,
}

fn false_f() -> bool {
    false
}

#[derive(Debug)]
pub struct VcsErrorsConfig {
    pub leak_hidden_repos: bool,
    pub verbose: bool,
}

impl VcsErrorsConfig {
    fn debug_default() -> Self {
        Self {
            leak_hidden_repos: true,
            verbose: true,
        }
    }

    fn release_default() -> Self {
        Self {
            leak_hidden_repos: false,
            verbose: false,
        }
    }

    pub(crate) fn if_verbose<T, F>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
        T: for<'a> From<&'a str>,
    {
        if self.verbose {
            f()
        } else {
            T::from("")
        }
    }
}

impl Default for VcsErrorsConfig {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        {
            Self::debug_default()
        }
        #[cfg(not(debug_assertions))]
        {
            Self::release_default()
        }
    }
}

impl<'de> Deserialize<'de> for VcsErrorsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct VcsErrorsConfigPatch {
            leak_hidden_repos: Option<bool>,
            verbose: Option<bool>,
        }

        let patch = VcsErrorsConfigPatch::deserialize(deserializer)?;
        let mut patched_value = Self::default();

        if let Some(leak_hidden_repos) = patch.leak_hidden_repos {
            patched_value.leak_hidden_repos = leak_hidden_repos;
        }

        if let Some(verbose) = patch.verbose {
            patched_value.verbose = verbose;
        }

        Ok(patched_value)
    }
}
