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

use std::process::Stdio;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{error, info, trace, Orbit, Rocket};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use upsilon_core::config::Cfg;

pub(crate) struct DebugDataDriverFairing;

#[rocket::async_trait]
impl Fairing for DebugDataDriverFairing {
    fn info(&self) -> Info {
        Info {
            name: "Debug Data Driver",
            kind: Kind::Singleton | Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let vcs_cfg = rocket
            .state::<Cfg<upsilon_vcs::UpsilonVcsConfig>>()
            .expect("Missing vcs config");

        let linux_exists = upsilon_vcs::exists_global(vcs_cfg, "linux");
        let upsilon_exists = upsilon_vcs::exists_global(vcs_cfg, "upsilon");

        let port = rocket.config().port;

        async fn debug_data_driver_task(
            port: u16,
            linux_exists: bool,
            upsilon_exists: bool,
        ) -> Result<(), std::io::Error> {
            let debug_data_driver = upsilon_core::alt_exe("upsilon-debug-data-driver");

            let mut cmd = Command::new(debug_data_driver);

            cmd.arg("--port")
                .arg(port.to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .env("RUST_LOG", "INFO");

            if linux_exists {
                cmd.arg("--linux-repo-exists");
            }

            if upsilon_exists {
                cmd.arg("--upsilon-repo-exists");
            }

            let mut child = cmd.spawn()?;

            trace!("Waiting for debug data driver");

            let exit_status = child.wait().await?;

            info!("Debug data driver exited with status: {}", exit_status);

            let stdout_pipe = child.stdout.as_mut().expect("failed to get stdout pipe");
            let stderr_pipe = child.stderr.as_mut().expect("failed to get stderr pipe");

            let mut stdout_str = String::new();
            let mut stderr_str = String::new();

            stdout_pipe.read_to_string(&mut stdout_str).await?;
            stderr_pipe.read_to_string(&mut stderr_str).await?;

            use std::io::Write;

            let mut stdout = std::io::stdout();
            let guard = "=".repeat(30);

            if !stdout_str.is_empty() {
                write!(
                    &mut stdout,
                    "Debug Data Driver stdout:\n{guard}\n{}{guard}\n",
                    stdout_str,
                    guard = guard
                )?;
            }

            if !stderr_str.is_empty() {
                write!(
                    &mut stdout,
                    "Debug Data Driver stderr:\n{guard}\n{}{guard}\n",
                    stderr_str
                )?;
            }

            if !exit_status.success() {
                error!(
                    "Debug data driver exited with non-zero status code: {}",
                    exit_status
                );
            } else {
                info!("Debug data driver finished successfully");
            }

            Ok(())
        }

        tokio::spawn(async move {
            let result = debug_data_driver_task(port, linux_exists, upsilon_exists).await;

            if let Err(e) = result {
                error!("Failed to run debug data driver: {}", e);
            }
        });
    }
}
