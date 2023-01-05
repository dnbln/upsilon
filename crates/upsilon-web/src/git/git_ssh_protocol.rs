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

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{async_trait, error, Build, Orbit, Rocket};
use upsilon_ssh::{SSHServer, SSHServerHolder, SSHServerInitializer};
use upsilon_ssh_russh::RusshServer;

use crate::config::GitSshProtocol;

pub struct GitSshFairing {
    config: GitSshProtocol,
}

impl GitSshFairing {
    pub(crate) fn new(config: GitSshProtocol) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Fairing for GitSshFairing {
    fn info(&self) -> Info {
        Info {
            name: "Git SSH Protocol Fairing",
            kind: Kind::Ignite,
        }
    }

    async fn on_ignite(&self, mut rocket: Rocket<Build>) -> rocket::fairing::Result {
        match &self.config {
            GitSshProtocol::Russh(russh_config) => {
                match <RusshServer as SSHServer>::Initializer::new(russh_config.clone())
                    .init()
                    .await
                {
                    Ok(server) => {
                        rocket = rocket
                            .manage(SSHServerHolder::new(server.into_wrapper()))
                            .attach(GitSshRusshServerFairing);
                    }
                    Err(e) => {
                        error!("Failed to initialize Russh SSH server: {e}");
                        return Err(rocket);
                    }
                }
            }
        }

        Ok(rocket)
    }
}

struct GitSshRusshServerFairing;

#[async_trait]
impl Fairing for GitSshRusshServerFairing {
    fn info(&self) -> Info {
        Info {
            name: "Git Russh SSH Fairing",
            kind: Kind::Shutdown,
        }
    }

    async fn on_shutdown(&self, rocket: &Rocket<Orbit>) {
        let Some(server) = rocket.state::<SSHServerHolder>() else {
            error!("Failed to get russh SSH server from state");
            return;
        };

        if let Err(e) = server.stop().await {
            error!("Failed to stop russh SSH server: {e}");
        }
    }
}
