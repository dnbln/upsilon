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

use std::sync::Arc;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{error, info, Orbit, Rocket};
use tokio::process::Child;
use tokio::sync::Mutex;

pub(crate) struct GitProtocolDaemonFairing {
    child: Arc<Mutex<Child>>,
}

impl GitProtocolDaemonFairing {
    pub(crate) fn new(child: Child) -> Self {
        Self {
            child: Arc::new(Mutex::new(child)),
        }
    }
}

#[rocket::async_trait]
impl Fairing for GitProtocolDaemonFairing {
    fn info(&self) -> Info {
        Info {
            name: "Git protocol daemon fairing",
            kind: Kind::Shutdown | Kind::Singleton,
        }
    }

    async fn on_shutdown(&self, _rocket: &Rocket<Orbit>) {
        info!("Killing git protocol daemon");

        let mut lock = self.child.lock().await;

        match lock.kill().await {
            Ok(_) => {
                let status = lock
                    .wait()
                    .await
                    .expect("Could not wait for git daemon to exit");

                info!("Git protocol daemon exited with status {status}");
            }
            Err(e) => {
                error!("Failed to kill git protocol daemon: {e}");
            }
        }
    }
}
