use rocket::serde::json::Json;
use rocket::State;
use upsilon_core::config::UsersConfig;
use upsilon_data::DataClientMasterHolder;
use upsilon_models::email::Email;
use upsilon_models::users::emails::UserEmails;
use upsilon_models::users::password::{PasswordHashAlgorithmDescriptor, PlainPassword};
use upsilon_models::users::{User, UserId, Username};

use crate::error::{ApiResult, Error};

#[derive(serde::Deserialize)]
pub struct CreateUserRequest {
    username: Username,
    password: PlainPassword,
    email: Email,
}

#[v1]
#[post("/users", data = "<user>")]
pub async fn create_user(
    user: Json<CreateUserRequest>,
    data: &State<DataClientMasterHolder>,
    users_config: &State<UsersConfig>,
) -> ApiResult<()> {
    if !users_config.register.enabled {
        return Err(Error::Forbidden);
    }

    let query_master = data.query_master();
    let Json(CreateUserRequest {
        username,
        password,
        email,
    }) = user;

    let id = UserId::new();
    let password_hash = PasswordHashAlgorithmDescriptor::from(users_config.auth.password)
        .hash_password(&password, &id.chrono_ts().timestamp().to_le_bytes());

    query_master
        .create_user(User {
            id,
            username,
            password: password_hash,
            name: None,
            emails: UserEmails::new(email),
            avatar: None,
        })
        .await?;

    Ok(())
}

#[derive(serde::Deserialize)]
pub struct LoginUserRequest {
    username_email: String,
    password: PlainPassword,
}

#[v1]
#[post("/users/login", data = "<user>")]
pub async fn login_user(
    user: Json<LoginUserRequest>,
    data: &State<DataClientMasterHolder>,
    users_config: &State<UsersConfig>,
) -> ApiResult<String> {
    let query_master = data.query_master();

    let Json(LoginUserRequest {
        username_email,
        password,
    }) = user;

    let user = query_master
        .query_user_by_username_email(&username_email)
        .await?
        .ok_or(Error::Unauthorized)?;

    let password_check = PasswordHashAlgorithmDescriptor::from(users_config.auth.password)
        .verify_password(&password, &user.password);

    if !password_check {
        return Err(Error::Unauthorized.into());
    }

    Ok("<token>".to_string())
}

api_routes!();
