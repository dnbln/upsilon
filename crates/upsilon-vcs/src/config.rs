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

#[derive(Deserialize, Debug)]
pub struct UpsilonVcsConfig {
    pub(crate) path: PathBuf,
    #[serde(rename = "git-protocol")]
    pub(crate) git_protocol: GitProtocol,
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
pub enum GitDaemon {
    Enabled(GitDaemonConfig),
    Disabled,
}

impl<'de> Deserialize<'de> for GitDaemon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct GitDaemonDesc {
            start: bool,
            #[serde(flatten, default)]
            config: Option<GitDaemonConfig>,
        }

        let desc = GitDaemonDesc::deserialize(deserializer)?;

        match desc {
            GitDaemonDesc {
                start: true,
                config: Some(config),
            } => Ok(GitDaemon::Enabled(config)),
            GitDaemonDesc {
                start: true,
                config: None,
            } => Err(serde::de::Error::custom("Missing config for start = true")),
            GitDaemonDesc {
                start: false,
                config: _,
            } => Ok(GitDaemon::Disabled),
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct GitDaemonConfig {
    #[serde(default = "default_git_daemon_port")]
    pub port: u16,

    #[serde(default)]
    pub services: GitDaemonServices,
}

fn default_git_daemon_port() -> u16 {
    9418
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct GitDaemonServices {
    #[serde(default)]
    pub upload_pack: GitDaemonService,
    #[serde(default)]
    pub upload_archive: GitDaemonService,
    #[serde(default = "receive_pack_default")]
    pub receive_pack: GitDaemonService,
}

impl Default for GitDaemonServices {
    fn default() -> Self {
        GitDaemonServices {
            upload_pack: GitDaemonService::default(),
            upload_archive: GitDaemonService::default(),
            receive_pack: receive_pack_default(),
        }
    }
}

fn receive_pack_default() -> GitDaemonService {
    GitDaemonService::Disabled
}

#[derive(Debug)]
pub enum GitDaemonService {
    Enabled(GitDaemonServiceConfig),
    Disabled,
}

impl<'de> Deserialize<'de> for GitDaemonService {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct GitDaemonServiceDesc {
            enable: bool,
            #[serde(flatten, default)]
            config: Option<GitDaemonServiceConfig>,
        }

        let desc = GitDaemonServiceDesc::deserialize(deserializer)?;

        match desc {
            GitDaemonServiceDesc {
                enable: true,
                config: Some(config),
            } => Ok(GitDaemonService::Enabled(config)),
            GitDaemonServiceDesc {
                enable: true,
                config: None,
            } => Err(serde::de::Error::custom("Missing config for enable = true")),
            GitDaemonServiceDesc {
                enable: false,
                config: _,
            } => Ok(GitDaemonService::Disabled),
        }
    }
}

impl Default for GitDaemonService {
    fn default() -> Self {
        GitDaemonService::Enabled(GitDaemonServiceConfig::default())
    }
}

#[derive(Deserialize, Debug, Default)]
pub struct GitDaemonServiceConfig {
    #[serde(rename = "override")]
    pub override_kind: GitDaemonServiceOverride,
}

#[derive(Deserialize, Debug, Default)]
pub enum GitDaemonServiceOverride {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "forbid")]
    Forbid,
    #[serde(rename = "default")]
    #[default]
    Default,
}
