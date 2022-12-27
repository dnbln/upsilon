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

use std::error::Error;
use std::sync::Arc;

use moka::future::Cache;
use upsilon_data::upsilon_models::organization::{
    Organization, OrganizationDisplayName, OrganizationId, OrganizationName, OrganizationNameRef, Team, TeamDisplayName, TeamId, TeamName, TeamNameRef
};
use upsilon_data::upsilon_models::repo::{Repo, RepoId, RepoName, RepoNameRef, RepoNamespace};
use upsilon_data::upsilon_models::users::{User, UserId, Username, UsernameRef};
use upsilon_data::{
    async_trait, CommonDataClientError, CommonDataClientErrorExtractor, DataClient, DataClientMaster, DataClientQueryImpl, DataClientQueryMaster
};
use upsilon_models::organization::OrganizationMember;
use upsilon_models::repo::RepoPermissions;

#[derive(thiserror::Error, Debug)]
pub enum CacheInMemoryError {
    #[error("inner error: {0}")]
    Inner(#[from] CommonDataClientError),
}

impl CommonDataClientErrorExtractor for CacheInMemoryError {
    fn into_common_error(self) -> CommonDataClientError {
        match self {
            CacheInMemoryError::Inner(inner) => inner,
        }
    }
}

struct CacheInMemoryStore {
    users: Cache<UserId, User>,
    repos: Cache<RepoId, Repo>,
    orgs: Cache<OrganizationId, Organization>,
    org_members: Cache<(OrganizationId, UserId), OrganizationMember>,
    teams: Cache<TeamId, Team>,
    repo_permissions: Cache<(RepoId, UserId), RepoPermissions>,
}

pub struct CacheInMemoryDataClient {
    cache: Arc<CacheInMemoryStore>,
    inner: Box<dyn DataClientMaster>,
}

#[async_trait]
impl DataClientMaster for CacheInMemoryDataClient {
    fn query_master<'a>(&'a self) -> Box<dyn DataClientQueryMaster + 'a> {
        self.data_client_query_impl().into_query_master()
    }

    async fn on_shutdown(&self) -> Result<(), Box<dyn Error>> {
        self.inner.on_shutdown().await
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct CacheInMemoryConfigSizes {
    pub max_users: usize,
    pub max_repos: usize,
    pub max_orgs: usize,
    pub max_repo_permissions: usize,
    pub max_org_members: usize,
    pub max_teams: usize,
}

pub struct CacheInMemoryConfig {
    sizes: CacheInMemoryConfigSizes,
    inner: Box<dyn DataClientMaster>,
}

impl CacheInMemoryConfig {
    pub fn new(sizes: CacheInMemoryConfigSizes, inner: Box<dyn DataClientMaster>) -> Self {
        Self { sizes, inner }
    }
}

#[async_trait]
impl DataClient for CacheInMemoryDataClient {
    type Error = CacheInMemoryError;
    type InnerConfiguration = CacheInMemoryConfig;
    type QueryImpl<'a>
    where
        Self: 'a,
    = CacheInMemoryQueryImpl<'a>;

    async fn init_client(config: Self::InnerConfiguration) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        fn cache<K, V>(s: usize) -> Cache<K, V>
        where
            K: std::hash::Hash + Eq + Send + Sync + 'static,
            V: Clone + Send + Sync + 'static,
        {
            Cache::new(s as u64)
        }

        Ok(Self {
            cache: Arc::new(CacheInMemoryStore {
                users: cache(config.sizes.max_users),
                repos: cache(config.sizes.max_repos),
                orgs: cache(config.sizes.max_orgs),
                org_members: cache(config.sizes.max_org_members),
                teams: cache(config.sizes.max_teams),
                repo_permissions: cache(config.sizes.max_repo_permissions),
            }),
            inner: config.inner,
        })
    }

    fn data_client_query_impl(&self) -> Self::QueryImpl<'_> {
        CacheInMemoryQueryImpl {
            client: self,
            inner: self.inner.query_master(),
        }
    }
}

pub struct CacheInMemoryQueryImpl<'a> {
    client: &'a CacheInMemoryDataClient,
    inner: Box<dyn DataClientQueryMaster + 'a>,
}

impl<'a> CacheInMemoryQueryImpl<'a> {
    fn store(&self) -> &CacheInMemoryStore {
        &self.client.cache
    }
}

#[async_trait]
impl<'a> DataClientQueryImpl<'a> for CacheInMemoryQueryImpl<'a> {
    type Error = CacheInMemoryError;

    async fn create_user(&self, user: User) -> Result<(), Self::Error> {
        self.store().users.insert(user.id, user.clone()).await;

        self.inner.create_user(user).await.convert_error()
    }

    async fn query_user(&self, user_id: UserId) -> Result<User, Self::Error> {
        let users = &self.store().users;
        match users.get(&user_id) {
            Some(user) => Ok(user),
            None => {
                let user = self.inner.query_user(user_id).await.convert_error()?;
                users.insert(user_id, user.clone()).await;
                Ok(user)
            }
        }
    }

    async fn query_user_by_username_email(
        &self,
        username_email: &str,
    ) -> Result<Option<User>, Self::Error> {
        self.inner
            .query_user_by_username_email(username_email)
            .await
            .convert_error()
    }

    async fn query_user_by_username<'self_ref>(
        &'self_ref self,
        username: UsernameRef<'self_ref>,
    ) -> Result<Option<User>, Self::Error> {
        let user = self
            .inner
            .query_user_by_username(username)
            .await
            .convert_error()?;
        if let Some(ref user) = user {
            self.store().users.insert(user.id, user.clone()).await;
        }
        Ok(user)
    }

    async fn set_user_name(&self, user_id: UserId, user_name: Username) -> Result<(), Self::Error> {
        self.store().users.invalidate(&user_id).await;

        self.inner
            .set_user_name(user_id, user_name)
            .await
            .convert_error()
    }

    async fn create_repo(&self, repo: Repo) -> Result<(), Self::Error> {
        self.store().repos.insert(repo.id, repo.clone()).await;

        self.inner.create_repo(repo).await.convert_error()
    }

    async fn query_repo(&self, repo_id: RepoId) -> Result<Repo, Self::Error> {
        match self.store().repos.get(&repo_id) {
            Some(repo) => Ok(repo),
            None => {
                let repo = self.inner.query_repo(repo_id).await.convert_error()?;
                self.store().repos.insert(repo_id, repo.clone()).await;
                Ok(repo)
            }
        }
    }

    async fn query_repo_by_name<'self_ref>(
        &'self_ref self,
        repo_name: RepoNameRef<'self_ref>,
        repo_namespace: &RepoNamespace,
    ) -> Result<Option<Repo>, Self::Error> {
        let repo = self
            .inner
            .query_repo_by_name(repo_name, repo_namespace)
            .await
            .convert_error()?;
        if let Some(ref repo) = repo {
            self.store().repos.insert(repo.id, repo.clone()).await;
        }
        Ok(repo)
    }

    async fn set_repo_name(&self, repo_id: RepoId, repo_name: RepoName) -> Result<(), Self::Error> {
        self.store().repos.invalidate(&repo_id).await;

        self.inner
            .set_repo_name(repo_id, repo_name)
            .await
            .convert_error()
    }

    async fn init_repo_user_perms(
        &self,
        repo_id: RepoId,
        user_id: UserId,
    ) -> Result<(), Self::Error> {
        self.inner
            .init_repo_user_perms(repo_id, user_id)
            .await
            .convert_error()
    }

    async fn query_repo_user_perms(
        &self,
        repo_id: RepoId,
        user_id: UserId,
    ) -> Result<Option<RepoPermissions>, Self::Error> {
        match self.store().repo_permissions.get(&(repo_id, user_id)) {
            Some(permissions) => Ok(Some(permissions)),
            None => {
                let permissions = self
                    .inner
                    .query_repo_user_perms(repo_id, user_id)
                    .await
                    .convert_error()?;
                if let Some(permissions) = permissions {
                    self.store()
                        .repo_permissions
                        .insert((repo_id, user_id), permissions)
                        .await;
                }
                Ok(permissions)
            }
        }
    }

    async fn add_repo_user_perms(
        &self,
        repo_id: RepoId,
        user_id: UserId,
        perms: RepoPermissions,
    ) -> Result<RepoPermissions, Self::Error> {
        self.inner
            .add_repo_user_perms(repo_id, user_id, perms)
            .await
            .convert_error()
    }

    async fn remove_repo_user_perms(
        &self,
        repo_id: RepoId,
        user_id: UserId,
        perms: RepoPermissions,
    ) -> Result<RepoPermissions, Self::Error> {
        self.inner
            .remove_repo_user_perms(repo_id, user_id, perms)
            .await
            .convert_error()
    }

    async fn create_organization(&self, org: Organization) -> Result<(), Self::Error> {
        self.store().orgs.insert(org.id, org.clone()).await;

        self.inner.create_organization(org).await.convert_error()
    }

    async fn query_organization(
        &self,
        org_id: OrganizationId,
    ) -> Result<Organization, Self::Error> {
        match self.store().orgs.get(&org_id) {
            Some(org) => Ok(org),
            None => {
                let org = self
                    .inner
                    .query_organization(org_id)
                    .await
                    .convert_error()?;
                self.store().orgs.insert(org_id, org.clone()).await;
                Ok(org)
            }
        }
    }

    async fn query_organization_by_name<'self_ref>(
        &'self_ref self,
        org_name: OrganizationNameRef<'self_ref>,
    ) -> Result<Option<Organization>, Self::Error> {
        let org = self
            .inner
            .query_organization_by_name(org_name)
            .await
            .convert_error()?;
        if let Some(ref org) = org {
            self.store().orgs.insert(org.id, org.clone()).await;
        }
        Ok(org)
    }

    async fn set_organization_name(
        &self,
        org_id: OrganizationId,
        org_name: OrganizationName,
    ) -> Result<(), Self::Error> {
        self.store().orgs.invalidate(&org_id).await;

        self.inner
            .set_organization_name(org_id, org_name)
            .await
            .convert_error()
    }

    async fn set_organization_display_name(
        &self,
        org_id: OrganizationId,
        org_display_name: Option<OrganizationDisplayName>,
    ) -> Result<(), Self::Error> {
        self.store().orgs.invalidate(&org_id).await;

        self.inner
            .set_organization_display_name(org_id, org_display_name)
            .await
            .convert_error()
    }

    async fn query_organization_member(
        &self,
        org_id: OrganizationId,
        user_id: UserId,
    ) -> Result<Option<OrganizationMember>, Self::Error> {
        match self.store().org_members.get(&(org_id, user_id)) {
            Some(member) => Ok(Some(member)),
            None => {
                let member = self
                    .inner
                    .query_organization_member(org_id, user_id)
                    .await
                    .convert_error()?;
                if let Some(ref member) = member {
                    self.store()
                        .org_members
                        .insert((org_id, user_id), member.clone())
                        .await;
                }
                Ok(member)
            }
        }
    }

    async fn query_organization_members(
        &self,
        org_id: OrganizationId,
    ) -> Result<Vec<OrganizationMember>, Self::Error> {
        // no way to cache this

        self.inner
            .query_organization_members(org_id)
            .await
            .convert_error()
    }

    async fn query_user_organizations(
        &self,
        user_id: UserId,
    ) -> Result<Vec<OrganizationMember>, Self::Error> {
        // no way to cache this

        self.inner
            .query_user_organizations(user_id)
            .await
            .convert_error()
    }

    async fn create_team(&self, team: Team) -> Result<(), Self::Error> {
        self.store().teams.insert(team.id, team.clone()).await;

        self.inner.create_team(team).await.convert_error()
    }

    async fn query_team(&self, team_id: TeamId) -> Result<Team, Self::Error> {
        match self.store().teams.get(&team_id) {
            Some(team) => Ok(team),
            None => {
                let team = self.inner.query_team(team_id).await.convert_error()?;
                self.store().teams.insert(team_id, team.clone()).await;
                Ok(team)
            }
        }
    }

    async fn query_organization_teams(
        &self,
        org_id: OrganizationId,
    ) -> Result<Vec<Team>, Self::Error> {
        // no way to cache this

        self.inner
            .query_organization_teams(org_id)
            .await
            .convert_error()
    }

    async fn query_team_by_name<'self_ref>(
        &'self_ref self,
        org_id: OrganizationId,
        team_name: TeamNameRef<'self_ref>,
    ) -> Result<Option<Team>, Self::Error> {
        let team = self
            .inner
            .query_team_by_name(org_id, team_name)
            .await
            .convert_error()?;
        if let Some(ref team) = team {
            self.store().teams.insert(team.id, team.clone()).await;
        }
        Ok(team)
    }

    async fn set_team_name(&self, team_id: TeamId, team_name: TeamName) -> Result<(), Self::Error> {
        self.store().teams.invalidate(&team_id).await;

        self.inner
            .set_team_name(team_id, team_name)
            .await
            .convert_error()
    }

    async fn set_team_display_name(
        &self,
        team_id: TeamId,
        team_display_name: Option<TeamDisplayName>,
    ) -> Result<(), Self::Error> {
        self.store().teams.invalidate(&team_id).await;

        self.inner
            .set_team_display_name(team_id, team_display_name)
            .await
            .convert_error()
    }

    async fn query_organization_and_team(
        &self,
        team_id: TeamId,
    ) -> Result<(Organization, Team), Self::Error> {
        // use the cache if available
        let team = self.query_team(team_id).await?;
        let org = self.query_organization(team.organization_id).await?;
        Ok((org, team))
    }

    async fn query_team_members(
        &self,
        organization_id: OrganizationId,
        team_id: TeamId,
    ) -> Result<Vec<OrganizationMember>, Self::Error> {
        // no way to cache this

        self.inner
            .query_team_members(organization_id, team_id)
            .await
            .convert_error()
    }

    fn into_query_master(self) -> Box<dyn DataClientQueryMaster + 'a> {
        Box::new(CacheInMemoryQueryMaster(self))
    }
}

upsilon_data::query_master_impl_trait!(CacheInMemoryQueryMaster, CacheInMemoryQueryImpl);

trait ConvertError<T> {
    fn convert_error(self) -> Result<T, CacheInMemoryError>;
}

impl<T, E> ConvertError<T> for Result<T, E>
where
    CacheInMemoryError: From<E>,
{
    fn convert_error(self) -> Result<T, CacheInMemoryError> {
        self.map_err(|e| CacheInMemoryError::from(e))
    }
}
