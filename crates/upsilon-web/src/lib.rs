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
use std::process::Stdio;
use std::sync::Arc;

use lazy_static::lazy_static;
use rocket::data::ByteUnit;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::NamedFile;
use rocket::http::uri::fmt::ValidRoutePrefix;
use rocket::http::uri::Origin;
use rocket::http::{Header, HeaderMap, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::response::Responder;
use rocket::{error, info, routes, trace, Build, Data, Orbit, Request, Response, Rocket, State};
use rocket_basicauth::{BasicAuth, BasicAuthError};
use serde::{Deserialize, Deserializer};
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use upsilon_api::auth::{AuthContext, AuthToken, AuthTokenError};
use upsilon_core::config::{Cfg, UsersConfig};
use upsilon_data::upsilon_models::repo::{Repo, RepoId, RepoPermissions};
use upsilon_data::{DataClient, DataClientMasterHolder, DataQueryMaster};
use upsilon_data_inmemory::InMemoryStorageSaveStrategy;
use upsilon_vcs::{
    AuthRequiredPermissionsKind, GitBackendCgiRequest, GitBackendCgiRequestMethod, GitBackendCgiResponse, SpawnDaemonError, UpsilonVcsConfig
};

#[derive(Deserialize, Debug)]
pub struct Config {
    vcs: UpsilonVcsConfig,
    #[serde(rename = "data-backend")]
    data_backend: DataBackendConfig,

    users: UsersConfig,

    #[serde(rename = "vcs-errors", default)]
    vcs_errors: VcsErrorsConfig,

    debug: DebugConfig,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct DebugConfig {
    #[serde(default = "false_f")]
    debug_data: bool,
}

fn false_f() -> bool {
    false
}

#[derive(Debug)]
pub struct VcsErrorsConfig {
    leak_hidden_repos: bool,
    verbose: bool,
}

impl VcsErrorsConfig {
    fn debug_default() -> Self {
        Self {
            leak_hidden_repos: true,
            verbose: true,
        }
    }

    fn release_default() -> Self {
        Self {
            leak_hidden_repos: false,
            verbose: false,
        }
    }

    fn if_verbose<T, F>(&self, f: F) -> T
    where
        F: FnOnce() -> T,
        T: for<'a> From<&'a str>,
    {
        if self.verbose {
            f()
        } else {
            T::from("")
        }
    }
}

impl Default for VcsErrorsConfig {
    fn default() -> Self {
        #[cfg(debug_assertions)]
        {
            Self::debug_default()
        }
        #[cfg(not(debug_assertions))]
        {
            Self::release_default()
        }
    }
}

impl<'de> Deserialize<'de> for VcsErrorsConfig {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "kebab-case")]
        struct VcsErrorsConfigPatch {
            leak_hidden_repos: Option<bool>,
            verbose: Option<bool>,
        }

        let patch = VcsErrorsConfigPatch::deserialize(deserializer)?;
        let mut patched_value = Self::default();

        if let Some(leak_hidden_repos) = patch.leak_hidden_repos {
            patched_value.leak_hidden_repos = leak_hidden_repos;
        }

        if let Some(verbose) = patch.verbose {
            patched_value.verbose = verbose;
        }

        Ok(patched_value)
    }
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
                "path is required when save is true",
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
            data_backend,
            users,
            vcs_errors,
            debug,
        } = app_config;

        match data_backend {
            DataBackendConfig::InMemory(config) => {
                rocket = rocket.attach(InMemoryDataBackendFairing(config));
            }
            DataBackendConfig::Postgres(config) => {
                rocket = rocket.attach(PostgresDataBackendFairing(config));
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

        if debug.debug_data {
            rocket = rocket.attach(DebugDataDriverFairing);
        }

        Ok(rocket
            .manage(Cfg::new(vcs))
            .manage(Cfg::new(vcs_errors))
            .manage(Cfg::new(users)))
    }

    async fn on_shutdown(&self, rocket: &Rocket<Orbit>) {
        let vcs_config = rocket.state::<Cfg<UpsilonVcsConfig>>().unwrap();

        match vcs_config.shutdown().await {
            Ok(_) => {}
            Err(e) => {
                panic!("Failed to shutdown git backend: {}", e);
            }
        }
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
                error!("Failed to initialize in-memory data backend: {}", e);
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
            .await
            .expect("Failed to kill git daemon");
    }
}

lazy_static! {
    // regexes from `git http-backend --help`
    static ref GIT_HTTP_PROTOCOL_PATHS: regex::Regex = regex::Regex::new(
        //language=regexp
        "^/((.*)/(HEAD|info/refs|objects/(info/[^/]+|[0-9a-f]{2}/[0-9a-f]{38}|pack/pack-[0-9a-f]{40}\\.(pack|idx))|git-(upload|receive)-pack))$"
    )
    .unwrap();


    // (Accelerated static Apache 2.x)
    static ref GIT_HTTP_PROTOCOL_STATIC_PATHS: regex::Regex = regex::Regex::new(
        //language=regexp
        "^/((.*)/objects/([0-9a-f]{2}/[0-9a-f]{38}|pack/pack-[0-9a-f]{40}\\.(pack|idx)))$"
    ).unwrap();
}

const PRIVATE_GIT_STATIC_ROOT: &str = "/__priv-git-static";
const PRIVATE_GIT_HTTP_BACKEND_ROOT: &str = "/__priv-git-http-backend-cgi";

lazy_static! {
    static ref PRIVATE_GIT_STATIC_ROOT_ORIGIN: Origin<'static> =
        Origin::parse(PRIVATE_GIT_STATIC_ROOT).unwrap();
    static ref PRIVATE_GIT_HTTP_BACKEND_ROOT_ORIGIN: Origin<'static> =
        Origin::parse(PRIVATE_GIT_HTTP_BACKEND_ROOT).unwrap();
}

#[derive(Debug)]
struct RepoPathRaw<'r>(&'r str);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RepoPathRaw<'r> {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let uri_path = request.uri().path();
        let uri_str = uri_path.as_str();

        if let Some(path) = uri_str.strip_prefix(PRIVATE_GIT_HTTP_BACKEND_ROOT) {
            let Some(captures) = GIT_HTTP_PROTOCOL_PATHS.captures(path) else {
                return Outcome::Failure((Status::BadRequest, ()));
            };

            let repo_path = captures.get(2).expect("Didn't match").as_str();

            Outcome::Success(Self(repo_path))
        } else if let Some(path) = uri_str.strip_prefix(PRIVATE_GIT_STATIC_ROOT) {
            let Some(captures) = GIT_HTTP_PROTOCOL_STATIC_PATHS.captures(path) else {
                return Outcome::Failure((Status::BadRequest, ()));
            };

            let repo_path = captures.get(2).expect("Didn't match").as_str();

            Outcome::Success(Self(repo_path))
        } else {
            Outcome::Failure((Status::BadRequest, ()))
        }
    }
}

#[derive(Debug)]
struct RepoPath(PathBuf);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for RepoPath {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        RepoPathRaw::from_request(request)
            .await
            .map(|it| Self(PathBuf::from(it.0)))
    }
}

#[derive(Debug, thiserror::Error)]
enum GetRepoError {
    #[error("IO error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("VCS error: {0}")]
    VcsError(#[from] upsilon_vcs::Error),
    #[error("Invalid UUID: {0}")]
    InvalidUUID(#[from] upsilon_id::FromStrError),
    #[error("Data backend error: {0}")]
    DataBackendError(#[from] upsilon_data::CommonDataClientError),
}

impl RepoPath {
    async fn get_repo_id(&self, vcs_config: &UpsilonVcsConfig) -> Result<RepoId, GetRepoError> {
        Ok(upsilon_vcs::read_repo_id(vcs_config, &self.0)
            .await?
            .parse()?)
    }

    async fn get_repo<'a>(
        &self,
        vcs_config: &UpsilonVcsConfig,
        data: &DataQueryMaster<'a>,
    ) -> Result<Repo, GetRepoError> {
        let id = self.get_repo_id(vcs_config).await?;

        Ok(data.query_repo(id).await?)
    }
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
                    PRIVATE_GIT_HTTP_BACKEND_ROOT,
                    routes![git_http_backend_cgi_get, git_http_backend_cgi_post],
                )
                .mount(PRIVATE_GIT_STATIC_ROOT, routes![git_static_get]);
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
            let forward_path = p_str.to_string();

            let query = uri.query().map(|it| Cow::Owned(it.to_string()));

            req.set_uri(
                PRIVATE_GIT_STATIC_ROOT_ORIGIN
                    .clone()
                    .append(Cow::Owned(forward_path), query),
            );
        } else if GIT_HTTP_PROTOCOL_PATHS.is_match(p_str) {
            let forward_path = p_str.to_string();

            let query = uri.query().map(|it| Cow::Owned(it.to_string()));

            req.set_uri(
                PRIVATE_GIT_HTTP_BACKEND_ROOT_ORIGIN
                    .clone()
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
    #[error("Failed to get repo: {0}")]
    GetRepoError(#[from] GetRepoError),
    #[error("Auth required")]
    AuthRequired,
    #[error("Hidden repository")]
    HiddenRepo,
    #[error("Missing write permissions")]
    MissingWritePermissions,
    #[error("Data backend error: {0}")]
    DataBackendError(#[from] upsilon_data::CommonDataClientError),
}

impl<'r, 'o: 'r> Responder<'r, 'o> for GitHttpBackendError {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'o> {
        let vcs_errors = request.rocket().state::<Cfg<VcsErrorsConfig>>().unwrap();

        match self {
            GitHttpBackendError::HandleGitHttpBackend(_) => (
                Status::InternalServerError,
                vcs_errors.if_verbose(|| self.to_string()),
            )
                .respond_to(request),
            GitHttpBackendError::IO(_) => (
                Status::InternalServerError,
                vcs_errors.if_verbose(|| self.to_string()),
            )
                .respond_to(request),
            GitHttpBackendError::GetRepoError(_) => (
                Status::InternalServerError,
                vcs_errors.if_verbose(|| self.to_string()),
            )
                .respond_to(request),
            GitHttpBackendError::AuthRequired => Response::build()
                .status(Status::Unauthorized)
                .header(Header::new("WWW-Authenticate", "Basic"))
                .ok(),
            GitHttpBackendError::HiddenRepo => match vcs_errors.leak_hidden_repos {
                true => (
                    Status::Forbidden,
                    "There appears to be a hidden repository here...",
                )
                    .respond_to(request),
                false => (Status::NotFound, "").respond_to(request),
            },
            GitHttpBackendError::MissingWritePermissions => {
                (Status::Forbidden, self.to_string()).respond_to(request)
            }
            GitHttpBackendError::DataBackendError(_) => (
                Status::InternalServerError,
                vcs_errors.if_verbose(|| self.to_string()),
            )
                .respond_to(request),
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

struct GitHttpBackendResponder(Status, GitBackendCgiResponse);

impl<'r, 'o: 'r> Responder<'r, 'o> for GitHttpBackendResponder {
    fn respond_to(self, _request: &'r Request<'_>) -> rocket::response::Result<'o> {
        let GitHttpBackendResponder(status, mut cgi_response) = self;

        let mut response = Response::build();

        response.status(status);

        for (name, value) in cgi_response.headers.drain(..) {
            response.header(Header::new(name, value));
        }

        response.streamed_body(cgi_response);

        response.ok()
    }
}

#[allow(clippy::too_many_arguments)]
#[rocket::get("/<path..>?<query..>")]
async fn git_http_backend_cgi_get(
    path: PathBuf,
    query: Option<HashMap<String, String>>,
    headers: HMap<'_>,
    remote_addr: SocketAddr,
    vcs_config: &State<Cfg<UpsilonVcsConfig>>,
    auth_token: Option<AuthTokenBasic>,
    repo_path: RepoPath,
    data: &State<DataClientMasterHolder>,
) -> Result<GitHttpBackendResponder, GitHttpBackendError> {
    let qm = data.query_master();
    let repo = repo_path.get_repo(vcs_config, &qm).await?;
    let repo_perms_for_user = match &auth_token {
        Some(auth_token) => qm
            .query_repo_user_perms(repo.id, auth_token.token.claims.sub)
            .await?
            .unwrap_or(repo.global_permissions),
        None => repo.global_permissions,
    };

    let path = PathBuf::from("/").join(path); // add the root /

    let mut req = GitBackendCgiRequest::new(
        GitBackendCgiRequestMethod::Get,
        path,
        query,
        headers.to_headers_list(),
        remote_addr,
        Cursor::new(""),
    );

    let auth_required_kind = req.auth_required_permissions_kind(vcs_config);

    auth_if_user_has_necessary_permissions(
        repo_perms_for_user,
        auth_required_kind,
        auth_token.is_some(),
        &mut req,
    )?;

    let response = upsilon_vcs::http_backend_handle(vcs_config, req).await?;
    let status = status_code_from_status_line(&response.status_line);

    Ok(GitHttpBackendResponder(status, response))
}

#[allow(clippy::too_many_arguments)]
#[rocket::post("/<path..>?<query..>", data = "<data>")]
async fn git_http_backend_cgi_post(
    path: PathBuf,
    query: Option<HashMap<String, String>>,
    headers: HMap<'_>,
    remote_addr: SocketAddr,
    vcs_config: &State<Cfg<UpsilonVcsConfig>>,
    data: Data<'_>,
    repo_path: RepoPath,
    auth_token: Option<AuthTokenBasic>,
    data_client_master: &State<DataClientMasterHolder>,
) -> Result<GitHttpBackendResponder, GitHttpBackendError> {
    let qm = data_client_master.query_master();
    let repo = repo_path.get_repo(vcs_config, &qm).await?;
    let repo_perms_for_user = match &auth_token {
        Some(auth_token) => qm
            .query_repo_user_perms(repo.id, auth_token.token.claims.sub)
            .await?
            .unwrap_or(repo.global_permissions),
        None => repo.global_permissions,
    };

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

    let auth_required_kind = req.auth_required_permissions_kind(vcs_config);

    auth_if_user_has_necessary_permissions(
        repo_perms_for_user,
        auth_required_kind,
        auth_token.is_some(),
        &mut req,
    )?;

    let response = upsilon_vcs::http_backend_handle(vcs_config, req).await?;
    let status = status_code_from_status_line(&response.status_line);

    Ok(GitHttpBackendResponder(status, response))
}

#[rocket::get("/<path..>")]
async fn git_static_get(
    path: PathBuf,
    repo_path: RepoPath,
    vcs_config: &State<Cfg<UpsilonVcsConfig>>,
    data: &State<DataClientMasterHolder>,
    auth: Option<AuthTokenBasic>,
) -> Result<NamedFile, GitHttpBackendError> {
    let qm = data.query_master();
    let repo = repo_path.get_repo(vcs_config, &qm).await?;
    let repo_perms_for_user = match &auth {
        Some(auth) => qm
            .query_repo_user_perms(repo.id, auth.token.claims.sub)
            .await?
            .unwrap_or(repo.global_permissions),
        None => repo.global_permissions,
    };

    // we only need read perms to send static files
    if !repo_perms_for_user.can_read() {
        if auth.is_some() {
            Err(GitHttpBackendError::AuthRequired)?;
        } else {
            Err(GitHttpBackendError::HiddenRepo)?;
        }
    }

    let file_path = vcs_config.get_path().join(path);

    Ok(NamedFile::open(file_path).await?)
}

fn auth_if_user_has_necessary_permissions<B: AsyncRead>(
    user_permissions: RepoPermissions,
    required_permissions: AuthRequiredPermissionsKind,
    auth: bool,
    req: &mut GitBackendCgiRequest<B>,
) -> Result<(), GitHttpBackendError> {
    if required_permissions.read() && !user_permissions.can_read() {
        if !auth {
            Err(GitHttpBackendError::AuthRequired)?;
        } else {
            Err(GitHttpBackendError::HiddenRepo)?;
        }
    }

    if required_permissions.write() && !user_permissions.can_write() {
        if !auth {
            Err(GitHttpBackendError::AuthRequired)?;
        } else {
            Err(GitHttpBackendError::MissingWritePermissions)?;
        }
    }

    // if we got here, the user has all the necessary permissions,
    // so authorize the request.
    req.auth();

    Ok(())
}

struct DebugDataDriverFairing;

#[rocket::async_trait]
impl Fairing for DebugDataDriverFairing {
    fn info(&self) -> Info {
        Info {
            name: "Debug Data Driver",
            kind: Kind::Singleton | Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let port = rocket.config().port;

        async fn debug_data_driver_task(port: u16) -> Result<(), std::io::Error> {
            let debug_data_driver = upsilon_core::alt_exe("upsilon-debug-data-driver");

            let mut child = Command::new(debug_data_driver)
                .arg("--port")
                .arg(port.to_string())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .env("RUST_LOG", "INFO")
                .spawn()?;

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
            let result = debug_data_driver_task(port).await;

            if let Err(e) = result {
                error!("Failed to run debug data driver: {}", e);
            }
        });
    }
}
