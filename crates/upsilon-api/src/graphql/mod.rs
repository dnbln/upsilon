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

use std::pin::Pin;

use chrono::Duration;
use juniper::futures::Stream;
use juniper::{futures, graphql_object, graphql_subscription, FieldError, FieldResult};
use upsilon_core::config::{Cfg, UsersConfig};
use upsilon_models::assets::ImageAssetId;
use upsilon_models::email::Email;
use upsilon_models::organization::{OrganizationDisplayName, OrganizationId, OrganizationName};
use upsilon_models::users::emails::UserEmails;
use upsilon_models::users::password::{PasswordHashAlgorithmDescriptor, PlainPassword};
use upsilon_models::users::{UserId, Username};

use crate::auth::{AuthContext, AuthToken, AuthTokenClaims};
use crate::error::Error;

pub type Schema = juniper::RootNode<'static, QueryRoot, MutationRoot, SubscriptionRoot>;

pub struct GraphQLContext {
    db: upsilon_data::DataClientMasterHolder,
    users_config: Cfg<UsersConfig>,
    auth_context: AuthContext,
    auth: Option<AuthToken>,
}

impl GraphQLContext {
    pub fn new(
        db: upsilon_data::DataClientMasterHolder,
        users_config: Cfg<UsersConfig>,
        auth_context: AuthContext,
        auth: Option<AuthToken>,
    ) -> Self {
        Self {
            db,
            users_config,
            auth_context,
            auth,
        }
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
        username: Username,
        email: Email,
        password: PlainPassword,
    ) -> FieldResult<UserId> {
        if !context.users_config.register.enabled {
            Err(Error::Forbidden)?;
        }

        let id = UserId::new();
        let password_hash =
            PasswordHashAlgorithmDescriptor::from(context.users_config.auth.password)
                .hash_password(&password, &id.chrono_ts().timestamp().to_le_bytes());

        context
            .db
            .query_master()
            .create_user(upsilon_models::users::User {
                id,
                username,
                password: password_hash,
                name: None,
                emails: UserEmails::new(email),
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

        let token = context
            .auth_context
            .sign(AuthTokenClaims::new(user.id, Duration::days(15)));

        Ok(token.to_string())
    }

    async fn create_organization(
        context: &GraphQLContext,
        name: OrganizationName,
    ) -> FieldResult<OrganizationId> {
        let auth = context.auth.as_ref().ok_or(Error::Unauthorized)?;

        let id = OrganizationId::new();
        context
            .db
            .query_master()
            .create_organization(upsilon_models::organization::Organization {
                id,
                owner: auth.claims.sub,
                name,
                display_name: None,
                email: None,
            })
            .await?;

        Ok(id)
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

#[graphql_object(name = "User", context = GraphQLContext)]
impl UserRef {
    fn id(&self) -> UserId {
        self.0.id
    }

    fn username(&self) -> &Username {
        &self.0.username
    }

    fn public_email(&self) -> Option<&Email> {
        self.0.emails.public_email()
    }

    fn avatar(&self) -> Option<&ImageAssetId> {
        self.0.avatar.as_ref()
    }

    fn display_name(&self) -> Option<&upsilon_models::users::Name> {
        self.0.name.as_ref()
    }
}

struct OrganizationRef(upsilon_models::organization::Organization);

#[graphql_object(context = GraphQLContext)]
impl OrganizationRef {
    fn id(&self) -> OrganizationId {
        self.0.id
    }

    fn name(&self) -> &OrganizationName {
        &self.0.name
    }

    fn display_name(&self) -> Option<&OrganizationDisplayName> {
        self.0.display_name.as_ref()
    }

    fn owner_id(&self) -> UserId {
        self.0.owner
    }

    async fn owner(&self, context: &GraphQLContext) -> FieldResult<UserRef> {
        Ok(UserRef(
            context.db.query_master().query_user(self.0.owner).await?,
        ))
    }
}
