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

mod config;
mod data;
mod debug;
mod git;

use std::path::PathBuf;

use config::Config;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{async_trait, error, Build, Orbit, Rocket, Shutdown};
use upsilon_core::config::Cfg;
use upsilon_vcs::{SpawnDaemonError, UpsilonVcsConfig};

use crate::config::{DebugConfig, GitSshProtocol};
use crate::data::{DataBackendConfig, InMemoryDataBackendFairing, PostgresDataBackendFairing};

pub struct ConfigManager;

#[async_trait]
impl Fairing for ConfigManager {
    fn info(&self) -> Info {
        Info {
            name: "Configurator fairing",
            kind: Kind::Ignite | Kind::Shutdown | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, mut rocket: Rocket<Build>) -> rocket::fairing::Result {
        let app_config = match rocket.figment().extract::<Config>() {
            Ok(config) => config,
            Err(e) => {
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };

        let Config {
            mut vcs,
            git_ssh,
            data_backend,
            users,
            vcs_errors,
            debug,
        } = app_config;

        match data_backend {
            DataBackendConfig::InMemory(config) => {
                rocket = rocket.attach(InMemoryDataBackendFairing::new(config));
            }
            DataBackendConfig::Postgres(config) => {
                rocket = rocket.attach(PostgresDataBackendFairing::new(config));
            }
        }

        match vcs.setup().await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to setup git backend: {}", e);

                return Err(rocket);
            }
        }

        match upsilon_vcs::spawn_daemon(&vcs) {
            Ok(child) => {
                rocket = rocket.attach(git::GitProtocolDaemonFairing::new(child));
            }
            Err(SpawnDaemonError::Disabled) => {}
            Err(io_err @ SpawnDaemonError::IoError(_)) => {
                error!("Failed to spawn git protocol daemon: {}", io_err);

                return Err(rocket);
            }
        }

        if vcs.http_protocol_enabled() {
            rocket = rocket.attach(git::GitHttpProtocolFairing);
        }

        if let Some(mut git_ssh) = git_ssh {
            match &mut git_ssh {
                GitSshProtocol::Russh(russh) => {
                    russh.complete(vcs.clone());
                }
            }
            rocket = rocket.attach(git::GitSshFairing::new(git_ssh));
        }

        let DebugConfig {
            debug_data,
            graphql,
            shutdown_endpoint,
        } = debug;

        if debug_data {
            rocket = rocket.attach(debug::DebugDataDriverFairing);
        }

        if shutdown_endpoint {
            rocket = rocket.mount("/", rocket::routes![shutdown_endpoint]);
        }

        Ok(rocket
            .manage(Cfg::new(vcs))
            .manage(Cfg::new(vcs_errors))
            .manage(Cfg::new(graphql))
            .manage(Cfg::new(users)))
    }

    async fn on_shutdown(&self, rocket: &Rocket<Orbit>) {
        let vcs_config = rocket.state::<Cfg<UpsilonVcsConfig>>().unwrap();

        match vcs_config.shutdown().await {
            Ok(_) => {}
            Err(e) => {
                error!("Failed to shutdown git backend: {}", e);
            }
        }
    }
}

pub struct PortFileWriter(pub PathBuf);

#[async_trait]
impl Fairing for PortFileWriter {
    fn info(&self) -> Info {
        Info {
            name: "Port file writer fairing",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let port = rocket.config().port;
        let portfile = &self.0;

        if let Err(e) = tokio::fs::write(portfile, port.to_string()).await {
            error!("Failed to write port file: {}", e);
        }
    }
}

#[rocket::post("/api/shutdown")]
async fn shutdown_endpoint(shutdown: Shutdown) {
    shutdown.notify();
}
