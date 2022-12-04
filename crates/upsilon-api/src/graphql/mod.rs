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

use std::future::Future;
use std::pin::Pin;

use chrono::Duration;
use futures::{Stream, StreamExt, TryStreamExt};
use juniper::{graphql_object, graphql_subscription, FieldError, FieldResult};
use upsilon_core::config::{Cfg, UsersConfig};
use upsilon_data::DataQueryMaster;
use upsilon_models::assets::ImageAssetId;
use upsilon_models::email::Email;
use upsilon_models::namespace::NamespaceId;
use upsilon_models::organization::{
    OrganizationDisplayName, OrganizationId, OrganizationMember, OrganizationName, Team, TeamDisplayName, TeamId, TeamName
};
use upsilon_models::repo::{Repo, RepoId, RepoName, RepoNamespace};
use upsilon_models::users::emails::UserEmails;
use upsilon_models::users::password::{PasswordHashAlgorithmDescriptor, PlainPassword};
use upsilon_models::users::{User, UserDisplayName, UserId, Username};

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

    async fn query<'a, 'b, F, T, E, Fut>(&'a self, f: F) -> FieldResult<T>
    where
        F: FnOnce(DataQueryMaster<'a>) -> Fut,
        Fut: Future<Output = Result<T, E>> + 'b,
        E: Into<FieldError>,
        'b: 'a,
    {
        let qm = self.db.query_master();
        f(qm).await.map_err(Into::into)
    }

    async fn query_org(&self, org_id: OrganizationId) -> FieldResult<OrganizationRef> {
        self.query(|qm| async move { qm.query_organization(org_id).await })
            .await
            .map(OrganizationRef)
    }
}

impl juniper::Context for GraphQLContext {}

pub struct QueryRoot;

#[graphql_object(Context = GraphQLContext)]
impl QueryRoot {
    fn api_version() -> &str {
        "v1"
    }

    async fn user(context: &GraphQLContext, user_id: UserId) -> FieldResult<UserRef> {
        context
            .query(|qm| async move { qm.query_user(user_id).await.map(UserRef) })
            .await
    }

    async fn user_by_username(
        context: &GraphQLContext,
        username: Username,
    ) -> FieldResult<Option<UserRef>> {
        context
            .query(|qm| async move { qm.query_user_by_username(&username).await })
            .await
            .map(|opt| opt.map(UserRef))
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
    ) -> FieldResult<String> {
        if !context.users_config.register.enabled {
            Err(Error::Forbidden)?;
        }

        let id = UserId::new();
        let password_hash =
            PasswordHashAlgorithmDescriptor::from(context.users_config.auth.password)
                .hash_password(&password, &id.chrono_ts().timestamp().to_le_bytes());

        let user = User {
            id,
            username,
            password: password_hash,
            display_name: None,
            emails: UserEmails::new(email),
            avatar: None,
        };

        context
            .query(|qm| async move { qm.create_user(user.clone()).await })
            .await?;

        let token = context
            .auth_context
            .sign(AuthTokenClaims::new(id, Duration::days(15)));

        Ok(token.to_string())
    }

    async fn login(
        context: &GraphQLContext,
        username_or_email: String,
        password: PlainPassword,
    ) -> FieldResult<String> {
        let user = context
            .query(|qm| async move { qm.query_user_by_username_email(&username_or_email).await })
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
    ) -> FieldResult<OrganizationRef> {
        let auth = context.auth.as_ref().ok_or(Error::Unauthorized)?;

        let org = upsilon_models::organization::Organization {
            id: OrganizationId::new(),
            owner: auth.claims.sub,
            name,
            display_name: None,
            email: None,
        };

        let org_clone = org.clone();

        context
            .query(|qm| async move { qm.create_organization(org_clone).await })
            .await?;

        Ok(OrganizationRef(org))
    }

    async fn create_repo(context: &GraphQLContext, name: RepoName) -> FieldResult<RepoRef> {
        let auth = context.auth.as_ref().ok_or(Error::Unauthorized)?;

        let repo = Repo {
            id: RepoId::new(),
            namespace: RepoNamespace(NamespaceId::User(auth.claims.sub)),
            name,
            display_name: None,
        };

        let repo_clone = repo.clone();
        context
            .query(|qm| async move { qm.create_repo(repo_clone).await })
            .await?;

        Ok(RepoRef(repo))
    }

    async fn create_repo_in_organization(
        context: &GraphQLContext,
        name: RepoName,
        organization_id: OrganizationId,
    ) -> FieldResult<RepoRef> {
        let auth = context.auth.as_ref().ok_or(Error::Unauthorized)?;

        let org = context
            .query(|qm| async move { qm.query_organization(organization_id).await })
            .await?;

        if org.owner != auth.claims.sub {
            Err(Error::Forbidden)?;
        }

        let repo = Repo {
            id: RepoId::new(),
            namespace: RepoNamespace(NamespaceId::Organization(organization_id)),
            name,
            display_name: None,
        };

        let repo_clone = repo.clone();

        context
            .query(|qm| async move { qm.create_repo(repo_clone).await })
            .await?;

        Ok(RepoRef(repo))
    }

    async fn create_repo_in_team(
        context: &GraphQLContext,
        name: RepoName,
        team_id: TeamId,
    ) -> FieldResult<RepoRef> {
        let auth = context.auth.as_ref().ok_or(Error::Unauthorized)?;

        let team = context
            .query(|qm| async move { qm.query_team(team_id).await })
            .await?;

        let organization = context
            .query(|qm| async move { qm.query_organization(team.organization_id).await })
            .await?;

        if organization.owner != auth.claims.sub {
            Err(Error::Forbidden)?;
        }

        let repo = Repo {
            id: RepoId::new(),
            namespace: RepoNamespace(NamespaceId::Team(team.organization_id, team_id)),
            name,
            display_name: None,
        };

        let repo_clone = repo.clone();

        context
            .query(|qm| async move { qm.create_repo(repo_clone).await })
            .await?;

        Ok(RepoRef(repo))
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

pub struct UserRef(User);

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

    fn display_name(&self) -> Option<&UserDisplayName> {
        self.0.display_name.as_ref()
    }

    async fn repo(&self, context: &GraphQLContext, name: RepoName) -> FieldResult<Option<RepoRef>> {
        context
            .query(|qm| async move {
                qm.query_repo_by_name(&name, &RepoNamespace(NamespaceId::User(self.0.id)))
                    .await
            })
            .await
            .map(|repo| repo.map(RepoRef))
    }

    async fn organizations(
        &self,
        context: &GraphQLContext,
    ) -> FieldResult<Vec<OrganizationMemberRef>> {
        context
            .query(|qm| async move { qm.query_user_organizations(self.0.id).await })
            .await
            .map(|v| v.wrap(OrganizationMemberRef))
    }
}

struct RepoRef(Repo);

#[graphql_object(name = "Repo", context = GraphQLContext)]
impl RepoRef {
    fn id(&self) -> RepoId {
        self.0.id
    }

    fn name(&self) -> &RepoName {
        &self.0.name
    }
}

struct OrganizationRef(upsilon_models::organization::Organization);

#[graphql_object(name = "Organization", context = GraphQLContext)]
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
        context
            .query(|qm| async move { qm.query_user(self.0.owner).await })
            .await
            .map(UserRef)
    }

    async fn members(&self, context: &GraphQLContext) -> FieldResult<Vec<OrganizationMemberRef>> {
        Ok(context
            .query(|qm| async move { qm.query_organization_members(self.0.id).await })
            .await?
            .wrap(OrganizationMemberRef))
    }

    async fn teams(&self, context: &GraphQLContext) -> FieldResult<Vec<TeamRef>> {
        Ok(context
            .query(|qm| async move { qm.query_organization_teams(self.0.id).await })
            .await?
            .wrap(TeamRef))
    }

    async fn repo(&self, context: &GraphQLContext, name: RepoName) -> FieldResult<Option<RepoRef>> {
        context
            .query(|qm| async move {
                qm.query_repo_by_name(&name, &RepoNamespace(NamespaceId::Organization(self.0.id)))
                    .await
            })
            .await
            .map(|repo| repo.map(RepoRef))
    }
}

trait Wrap {
    type Item;
    fn wrap<T, F>(self, f: F) -> Vec<T>
    where
        F: Fn(Self::Item) -> T;
}

impl<T, It> Wrap for It
where
    It: IntoIterator<Item = T>,
{
    type Item = T;
    fn wrap<U, F>(self, f: F) -> Vec<U>
    where
        F: Fn(Self::Item) -> U,
    {
        self.into_iter().map(f).collect()
    }
}

pub struct OrganizationMemberRef(OrganizationMember);

#[graphql_object(name = "OrganizationMember", context = GraphQLContext)]
impl OrganizationMemberRef {
    fn user_id(&self) -> UserId {
        self.0.user_id
    }

    fn organization_id(&self) -> OrganizationId {
        self.0.organization_id
    }

    fn team_ids(&self) -> &Vec<TeamId> {
        &self.0.teams
    }

    async fn user(&self, context: &GraphQLContext) -> FieldResult<UserRef> {
        context
            .query(|qm| async move { qm.query_user(self.0.user_id).await })
            .await
            .map(UserRef)
    }

    async fn organization(&self, context: &GraphQLContext) -> FieldResult<OrganizationRef> {
        context.query_org(self.0.organization_id).await
    }

    async fn teams(&self, context: &GraphQLContext) -> FieldResult<Vec<TeamRef>> {
        futures::stream::iter(self.0.teams.iter())
            .then(|team_id| async move {
                context
                    .query(|qm| async move { qm.query_team(*team_id).await })
                    .await
                    .map(TeamRef)
            })
            .try_collect()
            .await
    }
}

pub struct TeamRef(Team);

#[graphql_object(name = "Team", context = GraphQLContext)]
impl TeamRef {
    fn id(&self) -> TeamId {
        self.0.id
    }

    fn name(&self) -> &TeamName {
        &self.0.name
    }

    fn display_name(&self) -> Option<&TeamDisplayName> {
        self.0.display_name.as_ref()
    }

    fn organization_id(&self) -> OrganizationId {
        self.0.organization_id
    }

    async fn organization(&self, context: &GraphQLContext) -> FieldResult<OrganizationRef> {
        context.query_org(self.0.organization_id).await
    }

    async fn members(&self, context: &GraphQLContext) -> FieldResult<Vec<OrganizationMemberRef>> {
        context
            .query(|qm| async move {
                qm.query_team_members(self.0.organization_id, self.0.id)
                    .await
            })
            .await
            .map(|v| v.wrap(OrganizationMemberRef))
    }

    async fn repo(&self, context: &GraphQLContext, name: RepoName) -> FieldResult<Option<RepoRef>> {
        context
            .query(|qm| async move {
                qm.query_repo_by_name(
                    &name,
                    &RepoNamespace(NamespaceId::Team(self.0.organization_id, self.0.id)),
                )
                .await
            })
            .await
            .map(|repo| repo.map(RepoRef))
    }
}
