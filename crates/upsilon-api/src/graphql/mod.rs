use std::pin::Pin;

use juniper::futures::Stream;
use juniper::{futures, graphql_object, graphql_subscription, FieldError, FieldResult, Value};
use upsilon_core::config::{Cfg, UsersConfig};
use upsilon_models::users::password::{PasswordHashAlgorithmDescriptor, PlainPassword};
use upsilon_models::users::{UserId, Username};

use crate::error::Error;

pub type Schema = juniper::RootNode<'static, QueryRoot, MutationRoot, SubscriptionRoot>;

pub struct GraphQLContext {
    db: upsilon_data::DataClientMasterHolder,
    users_config: Cfg<UsersConfig>,
}

impl GraphQLContext {
    pub fn new(db: upsilon_data::DataClientMasterHolder, users_config: Cfg<UsersConfig>) -> Self {
        Self { db, users_config }
    }
}

impl juniper::Context for GraphQLContext {}

pub struct QueryRoot;

#[graphql_object(Context = GraphQLContext)]
impl QueryRoot {
    async fn api_version() -> &str {
        "v1"
    }

    async fn user(context: &GraphQLContext, user_id: UserId) -> FieldResult<UserRef> {
        Ok(UserRef(
            context.db.query_master().query_user(user_id).await?,
        ))
    }

    async fn user_by_username(
        context: &GraphQLContext,
        username: Username,
    ) -> FieldResult<Option<UserRef>> {
        Ok(context
            .db
            .query_master()
            .query_user_by_username(&username)
            .await?
            .map(UserRef))
    }
}

pub struct MutationRoot;

#[graphql_object(Context = GraphQLContext)]
impl MutationRoot {
    async fn create_user(
        context: &GraphQLContext,
        username: String,
        email: String,
        password: String,
    ) -> FieldResult<UserId> {
        if !context.users_config.register.enabled {
            Err(Error::Forbidden)?;
        }

        let id = UserId::new();
        let password_hash =
            PasswordHashAlgorithmDescriptor::from(context.users_config.auth.password)
                .hash_password(
                    &PlainPassword::from(password),
                    &id.chrono_ts().timestamp().to_le_bytes(),
                );

        context
            .db
            .query_master()
            .create_user(upsilon_models::users::User {
                id,
                username: upsilon_models::users::Username::from(username),
                password: password_hash,
                name: None,
                emails: upsilon_models::users::emails::UserEmails::new(
                    upsilon_models::email::Email::from(email),
                ),
                avatar: None,
            })
            .await?;

        Ok(id)
    }

    async fn login(
        context: &GraphQLContext,
        username_or_email: String,
        password: PlainPassword,
    ) -> FieldResult<String> {
        let query_master = context.db.query_master();

        let user = query_master
            .query_user_by_username_email(&username_or_email)
            .await?
            .ok_or(Error::Unauthorized)?;

        let password_check =
            PasswordHashAlgorithmDescriptor::from(context.users_config.auth.password)
                .verify_password(&password, &user.password);

        if !password_check {
            Err(Error::Unauthorized)?;
        }

        Ok("<token>".to_string())
    }
}

pub struct SubscriptionRoot;

type StringStream = Pin<Box<dyn Stream<Item = Result<String, FieldError>> + Send>>;

#[graphql_subscription(context = GraphQLContext)]
impl SubscriptionRoot {
    async fn hello_world() -> StringStream {
        let stream =
            futures::stream::iter(vec![Ok(String::from("Hello")), Ok(String::from("World!"))]);
        Box::pin(stream)
    }
}

pub struct UserRef(upsilon_models::users::User);

#[graphql_object(name = "User")]
impl UserRef {
    fn id(&self) -> UserId {
        self.0.id
    }

    fn username(&self) -> &Username {
        &self.0.username
    }
}
