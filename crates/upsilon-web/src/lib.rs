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
mod git;
#[cfg(feature = "plugins")]
mod plugins;

use config::Config;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{async_trait, error, Build, Orbit, Rocket, Shutdown};
use rocket_cors::{AllowedHeaders, AllowedMethods, AllowedOrigins, Cors, CorsOptions, Method};
use upsilon_api::{GraphQLApiConfigurator, UshArgs};
use upsilon_core::config::Cfg;
use upsilon_vcs::{SpawnDaemonError, UpsilonVcsConfig};
use upsilon_web_interface::WebFairing;

use crate::config::{DebugConfig, FrontendConfig, GitSshProtocol};
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
            frontend,
            plugins,
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

        let ssh_port = if let Some(mut git_ssh) = git_ssh {
            let ssh_port = match &mut git_ssh {
                GitSshProtocol::Russh(russh) => {
                    russh.complete(vcs.clone());
                    russh.port()
                }
            };

            rocket = rocket.attach(git::GitSshFairing::new(git_ssh));

            Some(ssh_port)
        } else {
            None
        };

        let DebugConfig {
            graphql,
            shutdown_endpoint,
        } = debug;

        if shutdown_endpoint {
            rocket = rocket.mount("/", rocket::routes![shutdown_endpoint]);
        }

        let mut ush_args = vec![];

        if let Some(ssh_port) = ssh_port {
            ush_args.extend([
                "--ssh".to_owned(),
                "--ssh-port".to_owned(),
                ssh_port.to_string(),
            ]);
        }

        if vcs.http_protocol_enabled() {
            ush_args.push("--git-http".to_owned());
        }

        if let Some(git_port) = vcs.git_daemon_port() {
            ush_args.extend([
                "--git-protocol".to_owned(),
                "--git-protocol-port".to_owned(),
                git_port.to_string(),
            ]);
        }

        rocket = rocket.attach(GraphQLApiConfigurator::new(UshArgs::new(ush_args)));

        let cors = Cors::from_options(
            &CorsOptions::default()
                .allowed_headers(AllowedHeaders::some(&[
                    "Authorization",
                    "Accept",
                    "Content-Type",
                ]))
                .allowed_methods(AllowedMethods::from([
                    Method::from(rocket::http::Method::Get),
                    Method::from(rocket::http::Method::Post),
                    Method::from(rocket::http::Method::Options),
                ]))
                // .send_wildcard(true)
                .allow_credentials(true)
                .allowed_origins(AllowedOrigins::some_exact(&[
                    "http://localhost:5173",
                    "http://127.0.0.1:5173",
                    "http://localhost:8000",
                    "http://[::1]:5173",
                ])),
        );

        let cors = match cors {
            Ok(cors) => cors,
            Err(e) => {
                error!("Failed to create CORS fairing: {}", e);
                return Err(rocket);
            }
        };

        rocket = rocket.attach(cors);

        match frontend {
            FrontendConfig::Enabled { frontend_root } => {
                rocket = rocket.attach(WebFairing::new(frontend_root));
            }
            FrontendConfig::Disabled => {}
        }

        #[cfg(feature = "plugins")]
        if let Some(plugins) = plugins {
            rocket = rocket.attach(crate::plugins::PluginsFairing { plugins });
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

#[rocket::post("/api/shutdown")]
async fn shutdown_endpoint(shutdown: Shutdown) {
    shutdown.notify();
}
