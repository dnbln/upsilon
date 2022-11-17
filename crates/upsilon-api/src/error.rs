use rocket::response::Responder;
use rocket::Request;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("IO Error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("VCS Error: {0}")]
    VcsError(#[from] upsilon_vcs::Error),
}

impl<'r, 'o: 'r> Responder<'r, 'o> for Error {
    fn respond_to(self, request: &'r Request<'_>) -> rocket::response::Result<'o> {
        let status = match &self {
            Error::IoError(_) => rocket::http::Status::InternalServerError,
            Error::VcsError(_) => rocket::http::Status::InternalServerError,
        };

        let response = rocket::response::status::Custom(status, self.to_string());

        response.respond_to(request)
    }
}

pub type ApiResult<T = ()> = Result<T, Error>;
