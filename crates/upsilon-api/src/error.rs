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

use rocket::response::Responder;
use rocket::Request;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("VCS Error: {0}")]
    VcsError(#[from] upsilon_vcs::Error),
    #[error("data backend error: {0}")]
    DataBackendError(#[from] upsilon_data::CommonDataClientError),

    #[error("Repo not found")]
    RepoNotFound,
    #[error("Repo already exists")]
    RepoAlreadyExists,

    #[error("Resolve impossible")]
    ResolveImpossible,

    #[error("Unauthorized")]
    Unauthorized,
    #[error("Forbidden")]
    Forbidden,
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'o> {
        let status = match &self {
            Error::IoError(_) => rocket::http::Status::InternalServerError,
            Error::VcsError(_) => rocket::http::Status::InternalServerError,
            Error::DataBackendError(_) => rocket::http::Status::InternalServerError,

            Error::RepoNotFound => rocket::http::Status::NotFound,
            Error::RepoAlreadyExists => rocket::http::Status::Conflict,
            Error::ResolveImpossible => rocket::http::Status::Conflict,

            Error::Unauthorized => rocket::http::Status::Unauthorized,
            Error::Forbidden => rocket::http::Status::Forbidden,
        };

        let response = rocket::response::status::Custom(status, self.to_string());

        response.respond_to(request)
    }
}

pub type ApiResult<T = ()> = Result<T, Error>;
