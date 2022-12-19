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

use serde::{Deserialize, Deserializer};
use upsilon_core::config::{GqlDebugConfig, UsersConfig};
use upsilon_vcs::UpsilonVcsConfig;

use crate::data::DataBackendConfig;

#[derive(Deserialize, Debug)]
pub struct Config {
    pub vcs: UpsilonVcsConfig,
    #[serde(rename = "data-backend")]
    pub data_backend: DataBackendConfig,

    pub users: UsersConfig,

    #[serde(rename = "vcs-errors", default)]
    pub vcs_errors: VcsErrorsConfig,

    pub debug: DebugConfig,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct DebugConfig {
    #[serde(default = "false_f")]
    pub debug_data: bool,

    #[serde(default)]
    pub graphql: GqlDebugConfig,
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
