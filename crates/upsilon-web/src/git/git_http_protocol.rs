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

use std::borrow::Cow;
use std::collections::HashMap;
use std::convert::Infallible;
use std::io::Cursor;
use std::net::SocketAddr;
use std::path::PathBuf;

use lazy_static::lazy_static;
use path_slash::PathExt;
use rocket::data::ByteUnit;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::fs::NamedFile;
use rocket::http::uri::fmt::ValidRoutePrefix;
use rocket::http::uri::Origin;
use rocket::http::{Header, HeaderMap, Status};
use rocket::request::{FromRequest, Outcome};
use rocket::response::Responder;
use rocket::{routes, Build, Data, Request, Response, Rocket, State};
use rocket_basicauth::{BasicAuth, BasicAuthError};
use upsilon_api::auth::{AuthContext, AuthToken, AuthTokenError};
use upsilon_core::config::Cfg;
use upsilon_data::upsilon_models::repo::{Repo, RepoId};
use upsilon_data::{DataClientMasterHolder, DataQueryMaster};
use upsilon_vcs::{
    GitBackendCgiRequest, GitBackendCgiRequestMethod, GitBackendCgiResponse, UpsilonVcsConfig
};
use upsilon_vcs_permissions::{GitService, LackingPermissionsError};

use crate::config::VcsErrorsConfig;

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

pub(crate) struct GitHttpProtocolFairing;

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
    #[error("Unknown git service")]
    UnknownGitService,
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
            GitHttpBackendError::UnknownGitService => {
                (Status::BadRequest, "Requested unknown git service").respond_to(request)
            }
        }
    }
}

fn status_code_from_status_line(status_line: &str) -> Status {
    let status_num = status_line
        .bytes()
        .position(|it| it == b' ')
        .map_or(status_line, |it| &status_line[..it]);

    Status::from_code(status_num.parse().expect("Code is not a number")).expect("Invalid code")
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

async fn get_service_for_path_and_query(
    path: &std::path::Path,
    query: Option<&HashMap<String, String>>,
) -> Result<GitService, GitHttpBackendError> {
    fn query_service_is(query: Option<&HashMap<String, String>>, service_name: &str) -> bool {
        query
            .and_then(|it| it.get("service"))
            .map_or(false, |it| it == service_name)
    }

    let path = path.to_slash().expect("Path is not valid UTF-8");

    let service = if path.ends_with("/git-upload-pack")
        || query_service_is(query, "git-upload-pack")
    {
        GitService::UploadPack
    } else if path.ends_with("/git-receive-pack") || query_service_is(query, "git-receive-pack") {
        GitService::ReceivePack
    } else if path.ends_with("/git-upload-archive") || query_service_is(query, "git-upload-archive")
    {
        GitService::UploadArchive
    } else {
        return Err(GitHttpBackendError::UnknownGitService);
    };

    Ok(service)
}

fn cast_lacking_perms_error<T>(
    result: Result<T, LackingPermissionsError>,
    has_auth: bool,
) -> Result<T, GitHttpBackendError> {
    match result {
        Ok(v) => Ok(v),
        Err(LackingPermissionsError::Read) => {
            if has_auth {
                Err(GitHttpBackendError::HiddenRepo)
            } else {
                Err(GitHttpBackendError::AuthRequired)
            }
        }
        Err(LackingPermissionsError::Write) => {
            if has_auth {
                Err(GitHttpBackendError::MissingWritePermissions)
            } else {
                Err(GitHttpBackendError::AuthRequired)
            }
        }
        Err(LackingPermissionsError::DataError(e)) => Err(GitHttpBackendError::DataBackendError(e)),
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
    let path = PathBuf::from("/").join(path); // add the root /

    let service = get_service_for_path_and_query(&path, query.as_ref()).await?;

    let result = upsilon_vcs_permissions::check_user_has_permissions(
        &repo,
        service,
        &qm,
        auth_token.as_ref().map(|it| it.token.claims.sub),
    )
    .await;

    let (repo_config, user_config) = cast_lacking_perms_error(result, auth_token.is_some())?;

    let req = GitBackendCgiRequest::new(
        GitBackendCgiRequestMethod::Get,
        path,
        query,
        headers.to_headers_list(),
        remote_addr,
        Cursor::new(""),
        repo_config,
        user_config,
    );

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
    let path = PathBuf::from("/").join(path); // add the root /

    let service = get_service_for_path_and_query(&path, query.as_ref()).await?;

    let result = upsilon_vcs_permissions::check_user_has_permissions(
        &repo,
        service,
        &qm,
        auth_token.as_ref().map(|it| it.token.claims.sub),
    )
    .await;

    let (repo_config, user_config) = cast_lacking_perms_error(result, auth_token.is_some())?;

    let data_stream = data.open(ByteUnit::Gigabyte(1));
    let req = GitBackendCgiRequest::new(
        GitBackendCgiRequestMethod::Post,
        path,
        query,
        headers.to_headers_list(),
        remote_addr,
        data_stream,
        repo_config,
        user_config,
    );

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
            .unwrap_or(repo.repo_config.global_permissions),
        None => repo.repo_config.global_permissions,
    };

    // we only need read perms to send static files
    if !repo_perms_for_user.can_read() {
        return if auth.is_some() {
            Err(GitHttpBackendError::AuthRequired)
        } else {
            Err(GitHttpBackendError::HiddenRepo)
        };
    }

    let file_path = vcs_config.get_path().join(path);

    Ok(NamedFile::open(file_path).await?)
}
