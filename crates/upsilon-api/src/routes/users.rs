use rocket::serde::json::Json;
use rocket::State;
use upsilon_core::config::UsersConfig;
use upsilon_data::DataClientMasterHolder;
use upsilon_models::email::Email;
use upsilon_models::users::emails::UserEmails;
use upsilon_models::users::password::PlainPassword;
use upsilon_models::users::{User, UserId, Username};

#[derive(serde::Deserialize)]
pub struct CreateUserRequest {
    username: Username,
    password: PlainPassword,
    email: Email,
}

#[post("/users", data = "<user>")]
pub async fn create_user(
    user: Json<CreateUserRequest>,
    data: &State<DataClientMasterHolder>,
    users_config: &State<UsersConfig>,
) {
    let query_master = data.query_master();
    let Json(CreateUserRequest {
        username,
        password,
        email,
    }) = user;

    let id = UserId::new();
    let password_hash = upsilon_models::users::password::PasswordHashAlgorithmDescriptor::from(
        users_config.auth.password,
    )
    .hash_password(&password, &id.chrono_ts().timestamp().to_le_bytes());

    query_master.create_user(User {
        id,
        username,
        password: password_hash,
        name: None,
        emails: UserEmails::new(email),
        avatar: None,
    }).await.expect("Create user");
}
