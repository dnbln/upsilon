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
use tokio::process::{Child, Command};

use crate::config::GitProtocol;
use crate::UpsilonVcsConfig;

#[derive(thiserror::Error, Debug)]
pub enum SpawnDaemonError {
    #[error("git daemon is disabled")]
    Disabled,

    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

pub fn spawn_daemon(config: &UpsilonVcsConfig) -> Result<Child, SpawnDaemonError> {
    let GitProtocol::Enabled(protocol_config) = &config.git_protocol else {Err(SpawnDaemonError::Disabled)?};
    let GitDaemon::Enabled(daemon_config) = &protocol_config.git_daemon else {Err(SpawnDaemonError::Disabled)?};

    let access_hook_path = upsilon_core::alt_exe("upsilon-git-protocol-accesshook");

    let mut cmd = Command::new("git");

    cmd.arg("daemon")
        .arg(format!("--base-path={}", &config.get_path().display()))
        .arg(format!("--port={}", daemon_config.port))
        .arg("--reuseaddr")
        .arg(format!(r#"--access-hook="{}""#, access_hook_path.display()));

    if let Some(pid_file) = &daemon_config.pid_file {
        cmd.arg(format!("--pid-file={}", pid_file.display()));
    }

    fn patch_cmd_for_service(cmd: &mut Command, service: &GitDaemonService, service_name: &str) {
        fn patch_cmd_for_override(
            cmd: &mut Command,
            override_kind: &GitDaemonServiceOverride,
            service_name: &str,
        ) {
            match override_kind {
                GitDaemonServiceOverride::Allow => {
                    cmd.arg(format!("--allow-override={service_name}"));
                }
                GitDaemonServiceOverride::Forbid => {
                    cmd.arg(format!("--forbid-override={service_name}"));
                }
                GitDaemonServiceOverride::Default => {}
            }
        }

        match service {
            GitDaemonService::Enabled(s) => {
                cmd.arg(format!("--enable={service_name}"));

                patch_cmd_for_override(cmd, &s.override_kind, service_name);
            }
            GitDaemonService::Disabled(s) => {
                cmd.arg(format!("--disable={service_name}"));

                patch_cmd_for_override(cmd, &s.override_kind, service_name);
            }
        }
    }

    patch_cmd_for_service(&mut cmd, &daemon_config.services.upload_pack, "upload-pack");
    patch_cmd_for_service(
        &mut cmd,
        &daemon_config.services.upload_archive,
        "upload-archive",
    );
    patch_cmd_for_service(
        &mut cmd,
        &daemon_config.services.receive_pack,
        "receive-pack",
    );

    cmd.arg(config.get_path());

    cmd.kill_on_drop(true);

    let child = cmd.spawn()?;

    Ok(child)
}

#[derive(Clone, Debug)]
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

#[derive(Deserialize, Clone, Debug)]
pub struct GitDaemonConfig {
    #[serde(default = "default_git_daemon_port")]
    pub port: u16,

    #[serde(default)]
    pub pid_file: Option<PathBuf>,

    #[serde(default)]
    pub services: GitDaemonServices,
}

fn default_git_daemon_port() -> u16 {
    9418
}

#[derive(Deserialize, Clone, Debug)]
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
    GitDaemonService::Disabled(GitDaemonServiceConfig {
        override_kind: GitDaemonServiceOverride::Default,
    })
}

#[derive(Debug, Clone)]
pub enum GitDaemonService {
    Enabled(GitDaemonServiceConfig),
    Disabled(GitDaemonServiceConfig),
}

impl<'de> Deserialize<'de> for GitDaemonService {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct GitDaemonServiceDesc {
            enable: bool,
            #[serde(flatten)]
            config: GitDaemonServiceConfig,
        }

        let desc = GitDaemonServiceDesc::deserialize(deserializer)?;

        match desc {
            GitDaemonServiceDesc {
                enable: true,
                config,
            } => Ok(GitDaemonService::Enabled(config)),
            GitDaemonServiceDesc {
                enable: false,
                config,
            } => Ok(GitDaemonService::Disabled(config)),
        }
    }
}

impl Default for GitDaemonService {
    fn default() -> Self {
        GitDaemonService::Enabled(GitDaemonServiceConfig::default())
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct GitDaemonServiceConfig {
    #[serde(rename = "override")]
    pub override_kind: GitDaemonServiceOverride,
}

#[derive(Deserialize, Clone, Copy, Debug, Default)]
pub enum GitDaemonServiceOverride {
    #[serde(rename = "allow")]
    Allow,
    #[serde(rename = "forbid")]
    Forbid,
    #[serde(rename = "default")]
    #[default]
    Default,
}
