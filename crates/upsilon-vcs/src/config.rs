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

use std::path::PathBuf;

use serde::{Deserialize, Deserializer};

use crate::daemon::GitDaemon;

#[derive(Deserialize, Debug)]
pub struct UpsilonVcsConfig {
    pub path: PathBuf,
    #[serde(rename = "git-protocol")]
    pub(crate) git_protocol: GitProtocol,
    #[serde(rename = "http-protocol")]
    pub(crate) http_protocol: GitHttpProtocol,
}

impl UpsilonVcsConfig {
    pub fn http_protocol_enabled(&self) -> bool {
        matches!(self.http_protocol, GitHttpProtocol::Enabled(_))
    }
}

#[derive(Debug)]
pub enum GitProtocol {
    Enabled(GitProtocolConfig),
    Disabled,
}

impl<'de> Deserialize<'de> for GitProtocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct GitProtocolDesc {
            enable: bool,
            #[serde(flatten, default)]
            config: Option<GitProtocolConfig>,
        }

        let desc = GitProtocolDesc::deserialize(deserializer)?;

        match desc {
            GitProtocolDesc {
                enable: true,
                config: Some(config),
            } => Ok(GitProtocol::Enabled(config)),
            GitProtocolDesc {
                enable: true,
                config: None,
            } => Err(serde::de::Error::custom("Missing config for enable = true")),
            GitProtocolDesc {
                enable: false,
                config: _,
            } => Ok(GitProtocol::Disabled),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct GitProtocolConfig {
    #[serde(rename = "git-daemon")]
    pub git_daemon: GitDaemon,
}

#[derive(Debug)]
pub enum GitHttpProtocol {
    Enabled(GitHttpProtocolConfig),
    Disabled,
}

impl<'de> Deserialize<'de> for GitHttpProtocol {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct GitHttpProtocolDesc {
            enable: bool,
            #[serde(flatten, default)]
            config: Option<GitHttpProtocolConfig>,
        }

        let desc = GitHttpProtocolDesc::deserialize(deserializer)?;

        match desc {
            GitHttpProtocolDesc {
                enable: true,
                config: Some(config),
            } => Ok(GitHttpProtocol::Enabled(config)),
            GitHttpProtocolDesc {
                enable: true,
                config: None,
            } => Err(serde::de::Error::custom("Missing config for enable = true")),
            GitHttpProtocolDesc {
                enable: false,
                config: _,
            } => Ok(GitHttpProtocol::Disabled),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct GitHttpProtocolConfig {}