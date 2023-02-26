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

use std::fmt;
use std::fmt::Formatter;
use std::sync::Arc;

use jwt::{AlgorithmType, PKeyWithDigest, SignWithKey, Token, VerifyWithKey};
use openssl::pkey::{PKey, Private, Public};
use openssl::rsa::Rsa;
use rocket::http::Status;
use rocket::request::{FromRequest, Outcome};
use rocket::{Request, State};
use upsilon_models::users::UserId;

#[derive(Clone, Debug)]
pub struct AuthToken {
    pub claims: AuthTokenClaims,
    token: String,
}

impl AuthToken {
    pub fn from_string(token: String, cx: &AuthContext) -> Result<Self, AuthTokenError> {
        let claims = match cx.verify(&token) {
            Ok(claims) => claims,
            Err(jwt) => return Err(jwt.into()),
        };

        Ok(AuthToken { claims, token })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthTokenError {
    #[error("No authorization header")]
    NoAuthorizationHeader,
    #[error("Invalid authorization header")]
    InvalidAuthorizationHeader,
    #[error("jwt: {0}")]
    Jwt(#[from] jwt::error::Error),
    #[error("internal error")]
    InternalError,
}

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthToken {
    type Error = AuthTokenError;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let Some(token) = request.headers().get_one("Authorization") else {
            return Outcome::Failure((Status::Unauthorized, AuthTokenError::NoAuthorizationHeader));
        };

        let Some(token) = token.strip_prefix("Bearer ") else {
            return Outcome::Failure((Status::Unauthorized, AuthTokenError::InvalidAuthorizationHeader));
        };

        let cx = match <&State<AuthContext>>::from_request(request).await {
            Outcome::Success(cx) => cx.inner().clone(),
            Outcome::Failure((status, _)) => {
                return Outcome::Failure((status, AuthTokenError::InternalError))
            }
            Outcome::Forward(_) => return Outcome::Forward(()),
        };

        let claims = match cx.verify(token) {
            Ok(claims) => claims,
            Err(jwt) => return Outcome::Failure((Status::Unauthorized, jwt.into())),
        };

        Outcome::Success(AuthToken {
            claims,
            token: token.to_owned(),
        })
    }
}

impl fmt::Display for AuthToken {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.token.fmt(f)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuthTokenClaims {
    pub sub: UserId,
    exp: usize,
}

impl AuthTokenClaims {
    pub fn new(sub: UserId, expires_in: chrono::Duration) -> Self {
        let exp = (chrono::Utc::now() + expires_in).timestamp() as usize;

        Self { sub, exp }
    }
}

#[derive(Clone)]
pub struct AuthContext(Arc<AuthContextInternal>);

impl AuthContext {
    pub fn new(bits: u32) -> Self {
        let pkey = Rsa::generate(bits).unwrap();
        let k0pkey = pkey.public_key_to_pem().unwrap();
        let pubkey = Rsa::public_key_from_pem(&k0pkey).unwrap();
        Self(Arc::new(AuthContextInternal {
            private_key: PKey::from_rsa(pkey).unwrap(),
            public_key: PKey::from_rsa(pubkey).unwrap(),
        }))
    }

    pub fn sign(&self, claims: AuthTokenClaims) -> AuthToken {
        self.0.sign(claims)
    }

    pub fn verify(&self, token: &str) -> Result<AuthTokenClaims, jwt::error::Error> {
        self.0.verify(token)
    }
}

struct AuthContextInternal {
    private_key: PKey<Private>,
    public_key: PKey<Public>,
}

impl AuthContextInternal {
    fn sign(&self, claims: AuthTokenClaims) -> AuthToken {
        let token = Token::new(
            jwt::Header {
                algorithm: AlgorithmType::Rs256,
                ..Default::default()
            },
            claims.clone(),
        )
        .sign_with_key(&PKeyWithDigest {
            digest: openssl::hash::MessageDigest::sha256(),
            key: self.private_key.clone(),
        })
        .unwrap();

        AuthToken {
            claims,
            token: token.as_str().to_owned(),
        }
    }

    fn verify(&self, token: &str) -> Result<AuthTokenClaims, jwt::Error> {
        let claims: AuthTokenClaims = token.verify_with_key(&PKeyWithDigest {
            digest: openssl::hash::MessageDigest::sha256(),
            key: self.public_key.clone(),
        })?;
        Ok(claims)
    }
}
