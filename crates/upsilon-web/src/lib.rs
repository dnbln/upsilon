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

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::Cursor;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Child;
use std::sync::Arc;

use lazy_static::lazy_static;
use rocket::data::ByteUnit;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::http::uri::fmt::ValidRoutePrefix;
use rocket::http::uri::Origin;
use rocket::http::{Header, HeaderMap, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::response::Responder;
use rocket::{error, routes, Build, Data, Orbit, Request, Response, Rocket, State};
use rocket_basicauth::{BasicAuth, BasicAuthError};
use serde::{Deserialize, Deserializer};
use tokio::io::AsyncReadExt;
use tokio::sync::Mutex;
use upsilon_api::auth::{AuthContext, AuthToken, AuthTokenError};
use upsilon_core::config::Cfg;
use upsilon_data::{DataClient, DataClientMasterHolder};
use upsilon_data_inmemory::InMemoryStorageSaveStrategy;
use upsilon_vcs::{
    GitBackendCgiRequest, GitBackendCgiRequestMethod, SpawnDaemonError, UpsilonVcsConfig
};

#[derive(Deserialize, Debug)]
pub struct Config {
    vcs: UpsilonVcsConfig,
    #[serde(rename = "data-backend")]
    data_backend: DataBackendConfig,

    users: upsilon_core::config::UsersConfig,
}

#[derive(Debug, Clone)]
pub enum InMemoryConfigSaveStrategy {
    Save { path: PathBuf },
    DontSave,
}

impl<'de> Deserialize<'de> for InMemoryConfigSaveStrategy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SaveStrategy {
            save: bool,
            path: Option<PathBuf>,
        }

        let s = SaveStrategy::deserialize(deserializer)?;

        match s {
            SaveStrategy {
                save: true,
                path: Some(path),
            } => Ok(Self::Save { path }),
            SaveStrategy {
                save: true,
                path: None,
            } => Err(serde::de::Error::custom(
                "Path is required when save is true",
            )),
            SaveStrategy {
                save: false,
                path: _,
            } => Ok(Self::DontSave),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct InMemoryDataBackendConfig {
    #[serde(flatten)]
    save_strategy: InMemoryConfigSaveStrategy,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PostgresDataBackendConfig {
    host: String,
    port: u16,
    user: String,
    password: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DataBackendConfig {
    #[serde(rename = "in-memory")]
    InMemory(InMemoryDataBackendConfig),
    #[serde(rename = "postgres")]
    Postgres(PostgresDataBackendConfig),
}

pub struct ConfigManager;

#[rocket::async_trait]
impl Fairing for ConfigManager {
    fn info(&self) -> Info {
        Info {
            name: "API fairing configurator",
            kind: Kind::Ignite | Kind::Singleton,
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
            vcs,
            data_backend,
            users,
        } = app_config;

        rocket = match data_backend {
            DataBackendConfig::InMemory(config) => {
                rocket.attach(InMemoryDataBackendFairing(config))
            }
            DataBackendConfig::Postgres(config) => {
                rocket.attach(PostgresDataBackendFairing(config))
            }
        };

        match upsilon_vcs::spawn_daemon(&vcs) {
            Ok(child) => {
                rocket = rocket.attach(GitProtocolDaemonFairing {
                    child: Arc::new(Mutex::new(child)),
                });
            }
            Err(SpawnDaemonError::Disabled) => {}
            Err(io_err @ SpawnDaemonError::IoError(_)) => {
                error!("Failed to spawn git protocol daemon: {}", io_err);

                return Err(rocket);
            }
        }

        if vcs.http_protocol_enabled() {
            rocket = rocket.attach(GitHttpProtocolFairing);
        }

        Ok(rocket.manage(Cfg::new(vcs)).manage(Cfg::new(users)))
    }
}

struct InMemoryDataBackendFairing(InMemoryDataBackendConfig);

#[rocket::async_trait]
impl Fairing for InMemoryDataBackendFairing {
    fn info(&self) -> Info {
        Info {
            name: "In-memory data backend",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        let cfg = upsilon_data_inmemory::InMemoryStorageConfiguration {
            save_strategy: match &self.0.save_strategy {
                InMemoryConfigSaveStrategy::Save { path } => {
                    InMemoryStorageSaveStrategy::Save { path: path.clone() }
                }
                InMemoryConfigSaveStrategy::DontSave => InMemoryStorageSaveStrategy::DontSave,
            },
        };

        let client = match upsilon_data_inmemory::InMemoryDataClient::init_client(cfg).await {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Failed to initialize in-memory data backend client: {e}");
                return Err(rocket);
            }
        };

        let client_master_holder = DataClientMasterHolder::new(client);

        Ok(rocket
            .manage(client_master_holder)
            .attach(DataBackendShutdownFairing))
    }
}

struct PostgresDataBackendFairing(PostgresDataBackendConfig);

#[rocket::async_trait]
impl Fairing for PostgresDataBackendFairing {
    fn info(&self) -> Info {
        Info {
            name: "Postgres data backend",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        Ok(rocket)
    }
}

struct DataBackendShutdownFairing;

#[rocket::async_trait]
impl Fairing for DataBackendShutdownFairing {
    fn info(&self) -> Info {
        Info {
            name: "Data backend shutdown",
            kind: Kind::Shutdown | Kind::Singleton,
        }
    }

    async fn on_shutdown(&self, rocket: &Rocket<Orbit>) {
        let holder = rocket
            .state::<DataClientMasterHolder>()
            .expect("Missing state");

        holder
            .on_shutdown()
            .await
            .expect("Data backend shutdown error");
    }
}

struct GitProtocolDaemonFairing {
    child: Arc<Mutex<Child>>,
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
        self.child
            .lock()
            .await
            .kill()
            .expect("Failed to kill git daemon");
    }
}

lazy_static! {
    // regexes from `git http-backend --help`
    static ref GIT_HTTP_PROTOCOL_PATHS: regex::Regex = regex::Regex::new(
        //language=regexp
        "^/(.*/(HEAD|info/refs|objects/(info/[^/]+|[0-9a-f]{2}/[0-9a-f]{38}|pack/pack-[0-9a-f]{40}\\.(pack|idx))|git-(upload|receive)-pack))$"
    )
    .unwrap();


    // (Accelerated static Apache 2.x)
    static ref GIT_HTTP_PROTOCOL_STATIC_PATHS: regex::Regex = regex::Regex::new(
        //language=regexp
        "^/(.*/objects/([0-9a-f]{2}/[0-9a-f]{38}|pack/pack-[0-9a-f]{40}\\.(pack|idx)))$"
    ).unwrap();
}

struct GitHttpProtocolFairing;

#[rocket::async_trait]
impl Fairing for GitHttpProtocolFairing {
    fn info(&self) -> Info {
        Info {
            name: "Git HTTP protocol fairing",
            kind: Kind::Ignite | Kind::Request | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, mut rocket: Rocket<Build>) -> rocket::fairing::Result {
        let vcs_config = rocket
            .state::<Cfg<UpsilonVcsConfig>>()
            .expect("Missing state")
            .clone();

        if vcs_config.http_protocol_enabled() {
            rocket = rocket
                .mount(
                    "/__priv-git-http-backend-cgi",
                    routes![git_http_backend_cgi_get, git_http_backend_cgi_post],
                )
                .mount(
                    "/__priv-git-static",
                    rocket::fs::FileServer::from(&vcs_config.path),
                );
        }

        Ok(rocket)
    }

    async fn on_request(&self, req: &mut Request<'_>, _data: &mut Data<'_>) {
        let vcs_config = <&State<Cfg<UpsilonVcsConfig>>>::from_request(req)
            .await
            .unwrap()
            .inner()
            .clone();

        if !vcs_config.http_protocol_enabled() {
            return;
        }

        let uri = req.uri();

        let p = uri.path();
        let p_str = p.as_str();

        if GIT_HTTP_PROTOCOL_STATIC_PATHS.is_match(p_str) {
            let Some(captures) = GIT_HTTP_PROTOCOL_STATIC_PATHS.captures(p_str) else {
                return;
            };
            let forward_path = format!("/{}", &captures[1]);

            let query = uri.query().map(|it| Cow::Owned(it.to_string()));

            req.set_uri(
                Origin::parse("/__priv-git-static/")
                    .unwrap()
                    .append(Cow::Owned(forward_path), query),
            );
        } else if GIT_HTTP_PROTOCOL_PATHS.is_match(p_str) {
            let Some(captures) = GIT_HTTP_PROTOCOL_PATHS.captures(p_str) else {
                return;
            };
            let forward_path = format!("/{}", &captures[1]);

            let query = uri.query().map(|it| Cow::Owned(it.to_string()));

            req.set_uri(
                Origin::parse("/__priv-git-http-backend-cgi/")
                    .unwrap()
                    .append(Cow::Owned(forward_path), query),
            );
        }
    }
}

struct HMap<'r>(&'r HeaderMap<'r>);

impl<'r> HMap<'r> {
    fn to_headers_list(&self) -> Vec<(String, String)> {
        self.0
            .iter()
            .map(|h| (h.name.to_string(), h.value.to_string()))
            .collect()
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for HMap<'r> {
    type Error = Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        Outcome::Success(Self(request.headers()))
    }
}

#[derive(thiserror::Error, Debug)]
enum GitHttpBackendError {
    #[error("Failed to handle git-http-backend: {0}")]
    HandleGitHttpBackend(#[from] upsilon_vcs::HttpBackendHandleError),
    #[error("Failed to read response: {0}")]
    IO(#[from] std::io::Error),
    #[error("Auth required")]
    AuthRequired,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for GitHttpBackendError {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'o> {
        match self {
            GitHttpBackendError::HandleGitHttpBackend(_) => {
                (Status::InternalServerError, self.to_string()).respond_to(request)
            }
            GitHttpBackendError::IO(_) => {
                (Status::InternalServerError, self.to_string()).respond_to(request)
            }
            GitHttpBackendError::AuthRequired => Response::build()
                .status(Status::Unauthorized)
                .header(Header::new("WWW-Authenticate", "Basic"))
                .ok(),
        }
    }
}

fn status_code_from_status_line(status_line: &str) -> Status {
    Status::from_code(
        status_line
            .split(' ')
            .next()
            .expect("Missing code")
            .parse()
            .expect("Code is not a number"),
    )
    .expect("Invalid code")
}

#[derive(Debug)]
struct AuthTokenBasic {
    username: String,
    token: AuthToken,
}

#[derive(Debug, thiserror::Error)]
enum AuthTokenBasicError {
    #[error("basic auth error: {0:?}")]
    BasicAuthError(BasicAuthError),
    #[error("auth token error: {0}")]
    AuthTokenError(#[from] AuthTokenError),
}

impl From<BasicAuthError> for AuthTokenBasicError {
    fn from(value: BasicAuthError) -> Self {
        Self::BasicAuthError(value)
    }
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthTokenBasic {
    type Error = AuthTokenBasicError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let basic_auth = match BasicAuth::from_request(request).await {
            Outcome::Success(auth) => auth,
            Outcome::Failure((status, error)) => return Outcome::Failure((status, error.into())),
            Outcome::Forward(_) => {
                return Outcome::Forward(());
            }
        };

        let BasicAuth { username, password } = basic_auth;

        let cx = <&State<AuthContext>>::from_request(request).await.unwrap();

        match AuthToken::from_string(password, cx) {
            Ok(token) => Outcome::Success(AuthTokenBasic { username, token }),
            Err(e) => Outcome::Failure((Status::Unauthorized, e.into())),
        }
    }
}

struct ResponseHeaders(Vec<(String, String)>);

struct GitHttpBackendResponder(Status, ResponseHeaders, Vec<u8>);

impl<'r, 'o: 'r> Responder<'r, 'o> for GitHttpBackendResponder {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'o> {
        let GitHttpBackendResponder(status, headers, body) = self;

        let mut response = Response::build();

        response.status(status);

        for (name, value) in headers.0 {
            response.header(Header::new(name, value));
        }

        response.sized_body(body.len(), Cursor::new(body));

        response.ok()
    }
}

#[rocket::get("/<path..>?<query..>")]
async fn git_http_backend_cgi_get(
    path: PathBuf,
    query: Option<HashMap<String, String>>,
    headers: HMap<'_>,
    remote_addr: SocketAddr,
    vcs_config: &State<Cfg<UpsilonVcsConfig>>,
    auth_token: Option<AuthTokenBasic>,
) -> Result<GitHttpBackendResponder, GitHttpBackendError> {
    let path = PathBuf::from("/").join(path); // add the root /

    let mut req = GitBackendCgiRequest::new(
        GitBackendCgiRequestMethod::Get,
        path,
        query,
        headers.to_headers_list(),
        remote_addr,
        std::io::Cursor::new(""),
    );

    let auth_required = req.auth_required(vcs_config);

    if auth_required {
        if let Some(auth_token) = &auth_token {
            // TODO: check perms for repo
            req.auth();
        } else {
            Err(GitHttpBackendError::AuthRequired)?;
        }
    }

    // dbg!(&req);

    let mut response = upsilon_vcs::http_backend_handle(vcs_config, req).await?;
    let status = status_code_from_status_line(&response.status_line);

    const RESP_BUF_SIZE: usize = 1024 * 1024;

    let mut resp_buf = Vec::with_capacity(RESP_BUF_SIZE);
    response.read_to_end(&mut resp_buf).await?;

    dbg!(unsafe { std::str::from_utf8_unchecked(&resp_buf) });

    Ok(GitHttpBackendResponder(
        status,
        ResponseHeaders(response.headers),
        resp_buf,
    ))
}

#[rocket::post("/<path..>?<query..>", data = "<data>")]
async fn git_http_backend_cgi_post(
    path: PathBuf,
    query: Option<HashMap<String, String>>,
    headers: HMap<'_>,
    remote_addr: SocketAddr,
    vcs_config: &State<Cfg<UpsilonVcsConfig>>,
    data: Data<'_>,
    auth_token: Option<AuthTokenBasic>,
) -> Result<GitHttpBackendResponder, GitHttpBackendError> {
    let path = PathBuf::from("/").join(path); // add the root /

    let data_stream = data.open(ByteUnit::Mebibyte(20));
    let mut req = GitBackendCgiRequest::new(
        GitBackendCgiRequestMethod::Post,
        path,
        query,
        headers.to_headers_list(),
        remote_addr,
        data_stream,
    );

    let auth_required = req.auth_required(vcs_config);

    if auth_required {
        if let Some(auth_token) = &auth_token {
            // TODO: check perms for repo
            req.auth();
        } else {
            Err(GitHttpBackendError::AuthRequired)?;
        }
    }

    let mut response = upsilon_vcs::http_backend_handle(vcs_config, req).await?;
    let status = status_code_from_status_line(&response.status_line);

    const RESP_BUF_SIZE: usize = 1024 * 1024;

    let mut resp_buf = Vec::with_capacity(RESP_BUF_SIZE);
    response.read_to_end(&mut resp_buf).await?;

    Ok(GitHttpBackendResponder(
        status,
        ResponseHeaders(response.headers),
        resp_buf,
    ))
}
