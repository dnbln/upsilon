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

use std::collections::HashMap;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync::Arc;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::NamedFile;
use rocket::http::uri::error::PathError;
use rocket::http::Status;
use rocket::request::{FromRequest, FromSegments, Outcome};
use rocket::{async_trait, get, routes, Build, Request, Rocket, State};

pub struct WebFairing {
    frontend_root: PathBuf,
}

impl WebFairing {
    pub fn new(frontend_root: PathBuf) -> Self {
        WebFairing { frontend_root }
    }
}

#[derive(Clone)]
struct WebInterfaceConfig {
    frontend_root: Arc<PathBuf>,
    frontend_root_index: Arc<PathBuf>,
}

#[async_trait]
impl Fairing for WebFairing {
    fn info(&self) -> Info {
        Info {
            name: "Web fairing",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        Ok(rocket
            .mount("/", routes![get_all, get_index])
            .manage(WebInterfaceConfig {
                frontend_root: Arc::new(self.frontend_root.clone()),
                frontend_root_index: Arc::new(self.frontend_root.join("index.html")),
            }))
    }
}

struct ExistentWebFrontendPath {
    path: PathBuf,
    target: PathBuf,
}

#[derive(Debug, thiserror::Error)]
enum ExistentWebFrontendPathError {
    #[error("Path error: {0:?}")]
    PathError(PathError),
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
}

impl From<PathError> for ExistentWebFrontendPathError {
    fn from(e: PathError) -> Self {
        ExistentWebFrontendPathError::PathError(e)
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for ExistentWebFrontendPath {
    type Error = ExistentWebFrontendPathError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let path = match request.segments::<PathBuf>(0..) {
            Ok(path) => path,
            Err(e) => return Outcome::Failure((Status::InternalServerError, e.into())),
        };
        let state = request.rocket().state::<WebInterfaceConfig>().unwrap();

        let target = state.frontend_root.join(&path);

        let meta = tokio::fs::metadata(&target).await;

        match meta {
            Ok(meta) => {
                if !meta.is_file() {
                    return Outcome::Forward(());
                }
            }
            Err(e) => {
                return if e.kind() == ErrorKind::NotFound {
                    Outcome::Forward(())
                } else {
                    Outcome::Failure((Status::InternalServerError, e.into()))
                }
            }
        }

        Outcome::Success(ExistentWebFrontendPath { path, target })
    }
}

#[get("/<_..>", rank = 100)]
async fn get_all(path: ExistentWebFrontendPath) -> Result<NamedFile, std::io::Error> {
    NamedFile::open(&path.target).await
}

#[get("/<path..>", rank = 101)]
async fn get_index(
    path: PathBuf,
    web_interface_config: &State<WebInterfaceConfig>,
) -> Result<NamedFile, std::io::Error> {
    let _ = path;
    NamedFile::open(web_interface_config.frontend_root_index.as_path()).await
}
