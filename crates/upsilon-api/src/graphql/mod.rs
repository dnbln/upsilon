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

mod git;

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use chrono::Duration;
use futures::{Stream, StreamExt, TryStreamExt};
use juniper::{graphql_object, graphql_subscription, FieldError, FieldResult};
use path_slash::PathBufExt;
use rocket::outcome::try_outcome;
use rocket::request::{FromRequest, Outcome};
use rocket::{Ignite, Request, Rocket, Sentinel, State};
use upsilon_core::config::{Cfg, GqlDebugConfig, UsersConfig};
use upsilon_data::{CommonDataClientError, DataQueryMaster};
use upsilon_models::assets::ImageAssetId;
use upsilon_models::email::Email;
use upsilon_models::namespace::NamespaceId;
use upsilon_models::organization::{
    Organization, OrganizationDisplayName, OrganizationId, OrganizationMember, OrganizationName, Team, TeamDisplayName, TeamId, TeamName
};
use upsilon_models::repo::{Repo, RepoId, RepoName, RepoNamespace, RepoPermissions};
use upsilon_models::users::emails::UserEmails;
use upsilon_models::users::password::{
    HashedPassword, PasswordHashAlgorithmDescriptor, PlainPassword
};
use upsilon_models::users::{User, UserDisplayName, UserId, UserSshKey, Username};
use upsilon_vcs::{RepoConfig, RepoVisibility, UpsilonVcsConfig};

use crate::auth::{AuthContext, AuthToken, AuthTokenClaims};
use crate::entity_lookup_path::{EntityLookupPath, ResolvedEntity};
use crate::error::Error;

pub type Schema = juniper::RootNode<'static, QueryRoot, MutationRoot, SubscriptionRoot>;

#[derive(Clone)]
pub struct UshArgs(Vec<String>);

impl UshArgs {
    pub fn new(args: Vec<String>) -> Self {
        UshArgs(args)
    }
}

#[derive(Clone)]
pub struct GraphQLContext {
    db: upsilon_data::DataClientMasterHolder,
    vcs_config: Cfg<UpsilonVcsConfig>,
    users_config: Cfg<UsersConfig>,
    debug_config: Cfg<GqlDebugConfig>,
    ush_args: Cfg<UshArgs>,
    host: Option<(String, Option<u16>)>,
    http_port: u16,
    auth_context: AuthContext,
    auth: Option<AuthToken>,
}

#[async_trait]
impl<'r> FromRequest<'r> for GraphQLContext {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let db = try_outcome!(
            request
                .guard::<&State<upsilon_data::DataClientMasterHolder>>()
                .await
        );
        let vcs_config = try_outcome!(request.guard::<&State<Cfg<UpsilonVcsConfig>>>().await);
        let users_config = try_outcome!(request.guard::<&State<Cfg<UsersConfig>>>().await);
        let debug_config = try_outcome!(request.guard::<&State<Cfg<GqlDebugConfig>>>().await);
        let ush_args = try_outcome!(request.guard::<&State<Cfg<UshArgs>>>().await);
        let http_port = request.rocket().config().port;
        let host = request
            .host()
            .map(|it| (it.domain().to_string(), it.port()));
        let auth_context = try_outcome!(request.guard::<&State<AuthContext>>().await);
        let auth = request.guard::<Option<AuthToken>>().await.unwrap();

        Outcome::Success(Self {
            db: db.inner().clone(),
            vcs_config: vcs_config.inner().clone(),
            users_config: users_config.inner().clone(),
            debug_config: debug_config.inner().clone(),
            ush_args: ush_args.inner().clone(),
            host,
            http_port,
            auth_context: auth_context.inner().clone(),
            auth,
        })
    }
}

impl Sentinel for GraphQLContext {
    fn abort(rocket: &Rocket<Ignite>) -> bool {
        <&State<upsilon_data::DataClientMasterHolder>>::abort(rocket)
            || <&State<Cfg<UpsilonVcsConfig>>>::abort(rocket)
            || <&State<Cfg<GqlDebugConfig>>>::abort(rocket)
            || <&State<Cfg<UsersConfig>>>::abort(rocket)
            || <&State<Cfg<UshArgs>>>::abort(rocket)
            || <&State<AuthContext>>::abort(rocket)
    }
}

impl GraphQLContext {
    async fn query_no_error_cast<'a, 'b, F, T, E, Fut>(&'a self, f: F) -> Result<T, E>
    where
        F: FnOnce(DataQueryMaster<'a>) -> Fut,
        Fut: Future<Output = Result<T, E>> + 'b,
        'b: 'a,
    {
        let qm = self.db.query_master();
        f(qm).await
    }

    async fn query<'a, 'b, F, T, E, Fut>(&'a self, f: F) -> FieldResult<T>
    where
        F: FnOnce(DataQueryMaster<'a>) -> Fut,
        Fut: Future<Output = Result<T, E>> + 'b,
        E: Into<FieldError>,
        'b: 'a,
    {
        self.query_no_error_cast(f).await.map_err(Into::into)
    }

    async fn query_user(&self, user_id: UserId) -> FieldResult<UserRef> {
        self.query(|qm| async move { qm.query_user(user_id).await })
            .await
            .map(UserRef)
    }

    async fn query_org(&self, org_id: OrganizationId) -> FieldResult<OrganizationRef> {
        self.query(|qm| async move { qm.query_organization(org_id).await })
            .await
            .map(OrganizationRef)
    }

    async fn init_repo(&self, repo_config: RepoConfig, path: PathBuf) -> FieldResult<()> {
        let vcs_config_clone = self.vcs_config.clone();

        tokio::task::spawn_blocking(move || {
            let _ = upsilon_vcs::init_repo_absolute(&vcs_config_clone, repo_config, &path)?;
            // drop the repository on the same thread

            Ok::<_, FieldError>(())
        })
        .await??;

        Ok(())
    }

    async fn init_repo_user_permissions(
        &self,
        repo_id: RepoId,
        user_id: UserId,
    ) -> FieldResult<()> {
        match self
            .query_no_error_cast(
                |qm| async move { qm.init_repo_user_perms(repo_id, user_id).await },
            )
            .await
        {
            Ok(_) => {}
            Err(upsilon_data::CommonDataClientError::PermsAlreadyExist) => {}
            Err(e) => Err(e)?,
        }

        Ok(())
    }

    fn require_debug(&self) -> FieldResult<()> {
        if !self.debug_config.debug_enabled {
            Err(DebugModeNotEnabled)?;
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Debug mode is not enabled")]
struct DebugModeNotEnabled;

impl juniper::Context for GraphQLContext {}

pub struct QueryRoot;

#[graphql_object(Context = GraphQLContext)]
impl QueryRoot {
    fn api_version() -> &'static str {
        "v1"
    }

    fn ush_cli_args(context: &GraphQLContext) -> Vec<String> {
        if !context.debug_config.debug_enabled {
            // we are in production
            // do not return an error, but also do not return the args
            return vec!["--upsilon-is-not-debug-rerun-shell-with-'--no-reconfigure'".to_string()];
        }

        let mut v = (*context.ush_args).0.clone();
        let mut port = context.http_port;

        if let Some((hn, http_port)) = &context.host {
            v.extend(["--hostname".to_string(), hn.clone()]);

            if let Some(http_port) = *http_port {
                port = http_port;
            }
        }

        v.extend(["--http-port".to_string(), port.to_string()]);

        v
    }

    async fn user(context: &GraphQLContext, user_id: UserId) -> FieldResult<UserRef> {
        context.query_user(user_id).await
    }

    async fn viewer(context: &GraphQLContext) -> FieldResult<Option<UserRef>> {
        match &context.auth {
            Some(auth) => Ok(Some(context.query_user(auth.claims.sub).await?)),
            None => Ok(None),
        }
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

    async fn organization(
        context: &GraphQLContext,
        org_id: OrganizationId,
    ) -> FieldResult<OrganizationRef> {
        context.query_org(org_id).await
    }

    async fn organization_by_name(
        context: &GraphQLContext,
        name: OrganizationName,
    ) -> FieldResult<Option<OrganizationRef>> {
        context
            .query(|qm| async move { qm.query_organization_by_name(&name).await })
            .await
            .map(|opt| opt.map(OrganizationRef))
    }

    async fn repo(context: &GraphQLContext, repo_id: RepoId) -> FieldResult<RepoRef> {
        context
            .query(|qm| async move { qm.query_repo(repo_id).await })
            .await
            .map(RepoRef)
    }

    async fn lookup_entity(
        context: &GraphQLContext,
        path: String,
    ) -> FieldResult<Option<EntityRef>> {
        let path = EntityLookupPath::from_iter(path.split('/').collect::<Vec<_>>().into_iter())?;

        let resolved = match context
            .query(|qm| async move { crate::entity_lookup_path::resolve(&qm, &path).await })
            .await?
        {
            Some(entity) => entity,
            None => return Ok(None),
        };

        let entity_ref = match resolved {
            ResolvedEntity::GlobalNamespace => EntityRef::GlobalNamespace,
            ResolvedEntity::User(user) => EntityRef::User(user),
            ResolvedEntity::Organization(org) => EntityRef::Organization(org),
            ResolvedEntity::Team(org, team) => EntityRef::Team(org, team),
            ResolvedEntity::Repo { repo, .. } => EntityRef::Repo(repo),
        };

        Ok(Some(entity_ref))
    }

    async fn lookup_repo(context: &GraphQLContext, path: String) -> FieldResult<Option<RepoRef>> {
        let path = EntityLookupPath::from_iter(path.split('/').collect::<Vec<_>>().into_iter())?;

        let repo = match context
            .query(|qm| async move { crate::entity_lookup_path::resolve(&qm, &path).await })
            .await?
        {
            Some(ResolvedEntity::Repo { repo, .. }) => repo,
            _ => return Ok(None),
        };

        Ok(Some(RepoRef(repo)))
    }
}

pub enum EntityRef {
    GlobalNamespace,
    User(User),
    Organization(Organization),
    Team(Organization, Team),
    Repo(Repo),
}

#[graphql_object(Context = GraphQLContext)]
impl EntityRef {
    fn user_id(&self) -> Option<UserId> {
        match self {
            EntityRef::User(user) => Some(user.id),
            _ => None,
        }
    }

    async fn user(&self) -> Option<UserRef> {
        match self {
            EntityRef::User(user) => Some(UserRef(user.clone())),
            _ => None,
        }
    }

    fn organization_id(&self) -> Option<OrganizationId> {
        match self {
            EntityRef::Organization(org) => Some(org.id),
            EntityRef::Team(org, _) => Some(org.id),
            _ => None,
        }
    }

    async fn organization(&self) -> Option<OrganizationRef> {
        match self {
            EntityRef::Organization(org) => Some(OrganizationRef(org.clone())),
            EntityRef::Team(org, _) => Some(OrganizationRef(org.clone())),
            _ => None,
        }
    }

    fn team_id(&self) -> Option<TeamId> {
        match self {
            EntityRef::Team(_, team) => Some(team.id),
            _ => None,
        }
    }

    async fn team(&self) -> Option<TeamRef> {
        match self {
            EntityRef::Team(_org, team) => Some(TeamRef(team.clone())),
            _ => None,
        }
    }

    fn repo_id(&self) -> Option<RepoId> {
        match self {
            EntityRef::Repo(repo) => Some(repo.id),
            _ => None,
        }
    }

    async fn repo(&self) -> Option<RepoRef> {
        match self {
            EntityRef::Repo(repo) => Some(RepoRef(repo.clone())),
            _ => None,
        }
    }
}

pub struct MutationRoot;

fn default_repo_config() -> upsilon_models::repo::RepoConfig {
    upsilon_models::repo::RepoConfig {
        global_permissions: RepoPermissions::READ,
        protected_branches: Vec::new(),
    }
}

impl MutationRoot {
    async fn make_global_mirror(
        context: &GraphQLContext,
        name: String,
        url: String,
    ) -> FieldResult<RepoRef> {
        let path = context.vcs_config.repo_dir(&name);

        tokio::fs::create_dir_all(&path).await?;

        let vcs_config_clone = context.vcs_config.clone();

        let repo = Repo {
            id: RepoId::new(),
            namespace: RepoNamespace(NamespaceId::GlobalNamespace),
            name: RepoName::from(name),
            display_name: None,
            repo_config: default_repo_config(),
        };

        let repo_clone = repo.clone();

        context
            .query(|qm| async move { qm.create_repo(repo_clone).await })
            .await?;

        let repo_id_string = repo.id.to_string();

        tokio::task::spawn_blocking(move || {
            let _ = upsilon_vcs::setup_mirror_absolute(
                &vcs_config_clone,
                url,
                &RepoConfig::new(RepoVisibility::Public, repo_id_string),
                path,
            )?;

            Ok::<_, FieldError>(())
        })
        .await??;

        Ok(RepoRef(repo))
    }
}

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
        let password_hash_algo =
            PasswordHashAlgorithmDescriptor::from(context.users_config.auth.password);
        let password_hash = tokio::task::spawn_blocking(move || {
            password_hash_algo.hash_password(&password, &id.chrono_ts().timestamp().to_le_bytes())
        })
        .await?;

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

    #[graphql(name = "_debug__createTestUser")]
    async fn create_test_user(
        context: &GraphQLContext,
        username: Username,
        email: Email,
        password: PlainPassword,
    ) -> FieldResult<String> {
        context.require_debug()?;

        if !context.users_config.register.enabled {
            Err(Error::Forbidden)?;
        }

        let id = UserId::new();
        let password_hash_algo =
            PasswordHashAlgorithmDescriptor::from(context.users_config.auth.password);
        let password_hash = 'password_hash: {
            if password == "test" {
                break 'password_hash HashedPassword::from("test_hash");
            }

            tokio::task::spawn_blocking(move || {
                password_hash_algo
                    .hash_password(&password, &id.chrono_ts().timestamp().to_le_bytes())
            })
            .await?
        };

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

        let password_hash_algo =
            PasswordHashAlgorithmDescriptor::from(context.users_config.auth.password);
        let password_check = tokio::task::spawn_blocking(move || {
            password_hash_algo.verify_password(&password, &user.password)
        })
        .await?;

        if !password_check {
            Err(Error::Unauthorized)?;
        }

        let token = context
            .auth_context
            .sign(AuthTokenClaims::new(user.id, Duration::days(15)));

        Ok(token.to_string())
    }

    #[graphql(name = "_debug__loginTestUser")]
    async fn login_test_user(
        context: &GraphQLContext,
        username_or_email: String,
        password: PlainPassword,
    ) -> FieldResult<String> {
        context.require_debug()?;

        let user = context
            .query(|qm| async move { qm.query_user_by_username_email(&username_or_email).await })
            .await?
            .ok_or(Error::Unauthorized)?;

        let password_hash_algo =
            PasswordHashAlgorithmDescriptor::from(context.users_config.auth.password);
        let password_check = 'password_check: {
            if password == "test" && user.password == "test_hash" {
                break 'password_check true;
            }

            tokio::task::spawn_blocking(move || {
                password_hash_algo.verify_password(&password, &user.password)
            })
            .await?
        };

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

        let org = Organization {
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

        let user = context
            .query(|qm| async move { qm.query_user(auth.claims.sub).await })
            .await?;

        let repo = Repo {
            id: RepoId::new(),
            namespace: RepoNamespace(NamespaceId::User(auth.claims.sub)),
            name: name.clone(),
            display_name: None,
            repo_config: default_repo_config(),
        };

        let repo_clone = repo.clone();
        context
            .query(|qm| async move { qm.create_repo(repo_clone).await })
            .await?;

        let mut pb = PathBuf::new();
        pb.push(user.username.as_str());
        pb.push(name.as_str());

        let path = context.vcs_config.repo_dir(pb);

        tokio::fs::create_dir_all(&path).await?;

        context
            .init_repo(
                RepoConfig::new(RepoVisibility::Public, repo.id.to_string()),
                path,
            )
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
            name: name.clone(),
            display_name: None,
            repo_config: default_repo_config(),
        };

        let repo_clone = repo.clone();

        context
            .query(|qm| async move { qm.create_repo(repo_clone).await })
            .await?;

        let mut pb = PathBuf::new();
        pb.push(org.name.as_str());
        pb.push(name.as_str());

        let path = context.vcs_config.repo_dir(pb);

        tokio::fs::create_dir_all(&path).await?;

        context
            .init_repo(
                RepoConfig::new(RepoVisibility::Public, repo.id.to_string()),
                path,
            )
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
            name: name.clone(),
            display_name: None,
            repo_config: default_repo_config(),
        };

        let repo_clone = repo.clone();

        context
            .query(|qm| async move { qm.create_repo(repo_clone).await })
            .await?;

        let mut pb = PathBuf::new();
        pb.push(organization.name.as_str());
        pb.push(team.name.as_str());
        pb.push(name.as_str());

        let path = context.vcs_config.repo_dir(pb);

        tokio::fs::create_dir_all(&path).await?;
        context
            .init_repo(
                RepoConfig::new(RepoVisibility::Public, repo.id.to_string()),
                path,
            )
            .await?;

        Ok(RepoRef(repo))
    }

    #[graphql(name = "_debug__globalMirror")]
    async fn global_mirror(
        context: &GraphQLContext,
        name: String,
        url: String,
    ) -> FieldResult<RepoRef> {
        context.require_debug()?;

        Self::make_global_mirror(context, name, url).await
    }

    #[graphql(name = "_debug__silentInitGlobal")]
    async fn silent_init_global(name: String, context: &GraphQLContext) -> FieldResult<RepoRef> {
        context.require_debug()?;

        let path = context.vcs_config.repo_dir(&name);

        let vcs_config_clone = context.vcs_config.clone();

        let repo = Repo {
            id: RepoId::new(),
            namespace: RepoNamespace(NamespaceId::GlobalNamespace),
            name: RepoName::from(name),
            display_name: None,
            repo_config: default_repo_config(),
        };

        let repo_clone = repo.clone();

        context
            .query(|qm| async move { qm.create_repo(repo_clone).await })
            .await?;

        if let Some(auth) = &context.auth {
            let user_id = auth.claims.sub;
            context
                .query(|qm| async move {
                    qm.init_repo_user_perms(repo.id, user_id).await?;

                    qm.add_repo_user_perms(
                        repo.id,
                        user_id,
                        RepoPermissions::ADMIN | RepoPermissions::WRITE | RepoPermissions::READ,
                    )
                    .await?;

                    Ok::<_, CommonDataClientError>(())
                })
                .await?;
        }

        let repo_id_string = repo.id.to_string();

        tokio::task::spawn_blocking(move || {
            let repo = upsilon_vcs::get_repo_absolute_no_check(&vcs_config_clone, &path)?;

            upsilon_vcs::silent_setup_repo_absolute(
                &vcs_config_clone,
                &path,
                &repo,
                &RepoConfig::new(RepoVisibility::Public, repo_id_string),
            )?;

            Ok::<_, FieldError>(())
        })
        .await??;

        Ok(RepoRef(repo))
    }

    #[graphql(name = "_debug__cpGlrFromLocal")]
    async fn cp_glr_from_local(
        context: &GraphQLContext,
        name: String,
        local_path: String,
    ) -> FieldResult<RepoRef> {
        context.require_debug()?;

        let local_path = PathBuf::from(local_path);

        Self::make_global_mirror(
            context,
            name,
            local_path.to_str().expect("Invalid path").to_string(),
        )
        .await
    }

    async fn add_user_repo_perms(
        context: &GraphQLContext,
        repo: RepoId,
        user: UserId,
        perms: RepoPermissions,
    ) -> FieldResult<RepoPermissions> {
        let auth = context.auth.as_ref().ok_or(Error::Unauthorized)?;

        let repo = context
            .query(|qm| async move { qm.query_repo(repo).await })
            .await?;

        if repo.namespace != NamespaceId::User(auth.claims.sub) {
            Err(Error::Forbidden)?;
        }

        context.init_repo_user_permissions(repo.id, user).await?;

        let new_perms = context
            .query(|qm| async move { qm.add_repo_user_perms(repo.id, user, perms).await })
            .await?;

        Ok(new_perms)
    }

    async fn rm_user_repo_perms(
        context: &GraphQLContext,
        repo: RepoId,
        user: UserId,
        perms: RepoPermissions,
    ) -> FieldResult<RepoPermissions> {
        let auth = context.auth.as_ref().ok_or(Error::Unauthorized)?;

        let repo = context
            .query(|qm| async move { qm.query_repo(repo).await })
            .await?;

        if repo.namespace != NamespaceId::User(auth.claims.sub) {
            Err(Error::Forbidden)?;
        }

        context.init_repo_user_permissions(repo.id, user).await?;

        let new_perms = context
            .query(|qm| async move { qm.remove_repo_user_perms(repo.id, user, perms).await })
            .await?;

        Ok(new_perms)
    }

    async fn add_user_ssh_key(context: &GraphQLContext, key: String) -> FieldResult<bool> {
        let auth = context.auth.as_ref().ok_or(Error::Unauthorized)?;

        let key = key.parse::<UserSshKey>()?;

        let result = context
            .query(|qm| async move { qm.add_user_ssh_key(auth.claims.sub, key).await })
            .await?;

        Ok(result)
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
            .map(|opt| opt.map(RepoRef))
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

impl RepoRef {
    async fn ns_path(
        &self,
        qm: DataQueryMaster<'_>,
    ) -> Result<PathBuf, upsilon_data::CommonDataClientError> {
        let res = match self.0.namespace.0 {
            NamespaceId::GlobalNamespace => {
                let mut pb = PathBuf::new();
                pb.push(self.0.name.as_str());
                pb
            }
            NamespaceId::User(user) => {
                let user = qm.query_user(user).await?;
                let mut pb = PathBuf::new();
                pb.push(user.username.as_str());
                pb.push(self.0.name.as_str());
                pb
            }
            NamespaceId::Organization(org) => {
                let org = qm.query_organization(org).await?;
                let mut pb = PathBuf::new();
                pb.push(org.name.as_str());
                pb.push(self.0.name.as_str());
                pb
            }
            NamespaceId::Team(org, team) => {
                let team = qm.query_team(team).await?;
                let org = qm.query_organization(org).await?;
                let mut pb = PathBuf::new();
                pb.push(org.name.as_str());
                pb.push(team.name.as_str());
                pb.push(self.0.name.as_str());
                pb
            }
        };

        Ok(res)
    }
}

#[graphql_object(name = "Repo", context = GraphQLContext)]
impl RepoRef {
    fn id(&self) -> RepoId {
        self.0.id
    }

    fn name(&self) -> &RepoName {
        &self.0.name
    }

    async fn path(&self, context: &GraphQLContext) -> FieldResult<String> {
        let path = context
            .query(|qm| async move { Self::ns_path(self, qm).await })
            .await?;

        Ok(path.to_slash_lossy().into_owned())
    }

    async fn git(&self, context: &GraphQLContext) -> FieldResult<git::RepoGit> {
        let ns_path = self.ns_path(context.db.query_master()).await?;

        let repo_dir = context.vcs_config.repo_dir(ns_path);
        let vcs_config = context.vcs_config.clone();

        Ok(git::RepoGit(
            upsilon_asyncvcs::Client::new(move || {
                upsilon_vcs::get_repo_absolute(&vcs_config, &repo_dir).expect("Failed to get repo")
            })
            .await,
        ))
    }
}

struct OrganizationRef(Organization);

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
            .map(|opt| opt.map(RepoRef))
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
            .map(|opt| opt.map(RepoRef))
    }
}
