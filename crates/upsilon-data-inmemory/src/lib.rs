use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use upsilon_data::{
    async_trait, query_master_impl_trait, CommonDataClientError, CommonDataClientErrorExtractor,
    DataClient, DataClientMaster, DataClientQueryImpl, DataClientQueryMaster,
};
use upsilon_models::organization::{
    Organization, OrganizationDisplayName, OrganizationId, OrganizationMember, OrganizationName,
    OrganizationNameRef, Team, TeamDisplayName, TeamId, TeamName, TeamNameRef,
};
use upsilon_models::repo::{Repo, RepoId, RepoName, RepoNameRef, RepoNamespace};
use upsilon_models::users::{User, UserId, Username, UsernameRef};

#[derive(Debug, thiserror::Error)]
pub enum InMemoryError {
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Repo not found")]
    RepoNotFound,
    #[error("Repo already exists")]
    RepoAlreadyExists,
    #[error("Organization not found")]
    OrganizationNotFound,
    #[error("Organization members not found")]
    OrganizationMembersNotFound,
    #[error("Team not found")]
    TeamNotFound,

    #[error("Name conflict")]
    NameConflict,
}

impl CommonDataClientErrorExtractor for InMemoryError {
    fn into_common_error(self) -> CommonDataClientError {
        match self {
            InMemoryError::UserNotFound => CommonDataClientError::UserNotFound,
            InMemoryError::UserAlreadyExists => CommonDataClientError::UserAlreadyExists,
            _ => CommonDataClientError::Other(Box::new(self)),
        }
    }
}

#[derive(Clone, Debug)]
pub enum InMemoryStorageSaveStrategy {
    Save { path: PathBuf },
    DontSave,
}

#[derive(Clone, Debug)]
pub struct InMemoryStorageConfiguration {
    pub save_strategy: InMemoryStorageSaveStrategy,
}

struct InMemoryDataStore {
    users: Arc<Mutex<BTreeMap<UserId, User>>>,
    repos: Arc<Mutex<BTreeMap<RepoId, Repo>>>,
    organizations: Arc<Mutex<BTreeMap<OrganizationId, Organization>>>,
    organization_members:
        Arc<Mutex<BTreeMap<OrganizationId, BTreeMap<UserId, OrganizationMember>>>>,
    teams: Arc<Mutex<BTreeMap<TeamId, Team>>>,
}

impl InMemoryDataStore {
    fn new() -> Self {
        fn new_map<K, V>() -> Arc<Mutex<BTreeMap<K, V>>> {
            Arc::new(Mutex::new(BTreeMap::new()))
        }

        Self {
            users: new_map(),
            repos: new_map(),
            organizations: new_map(),
            organization_members: new_map(),
            teams: new_map(),
        }
    }
}

pub struct InMemoryDataClient(InMemoryStorageConfiguration, Box<InMemoryDataStore>);

#[async_trait]
impl DataClient for InMemoryDataClient {
    type InnerConfiguration = InMemoryStorageConfiguration;
    type Error = InMemoryError;
    type QueryImpl<'a> = InMemoryQueryImpl<'a>;

    async fn init_client(config: Self::InnerConfiguration) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self(config, Box::new(InMemoryDataStore::new())))
    }

    fn data_client_query_impl<'a>(&'a self) -> Self::QueryImpl<'a> {
        InMemoryQueryImpl(self)
    }
}

#[async_trait]
impl DataClientMaster for InMemoryDataClient {
    fn query_master<'a>(&'a self) -> Box<dyn DataClientQueryMaster + 'a> {
        self.data_client_query_impl().as_query_master()
    }

    async fn on_shutdown(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

pub struct InMemoryQueryImpl<'a>(&'a InMemoryDataClient);

impl<'a> InMemoryQueryImpl<'a> {
    fn store(&self) -> &InMemoryDataStore {
        &self.0 .1
    }
}

#[async_trait]
impl<'a> DataClientQueryImpl<'a> for InMemoryQueryImpl<'a> {
    type Error = InMemoryError;

    async fn create_user(self: &Self, user: User) -> Result<(), Self::Error> {
        let mut lock = self.store().users.lock().await;
        let orgs_lock = self.store().organizations.lock().await;

        if lock.contains_key(&user.id) {
            return Err(InMemoryError::UserAlreadyExists);
        }

        if lock.values().any(|it| it.username == user.username) {
            return Err(InMemoryError::NameConflict);
        }

        if orgs_lock.values().any(|it| it.name == user.username) {
            return Err(InMemoryError::NameConflict);
        }

        lock.insert(user.id, user);

        Ok(())
    }

    async fn query_user(&self, user_id: UserId) -> Result<User, Self::Error> {
        let lock = self.store().users.lock().await;

        lock.get(&user_id)
            .map(|user| user.clone())
            .ok_or(InMemoryError::UserNotFound)
    }

    async fn query_user_by_username_email(
        &self,
        username_email: &str,
    ) -> Result<Option<User>, Self::Error> {
        let lock = self.store().users.lock().await;

        let user = lock
            .values()
            .find(|user| user.username == username_email || user.emails.contains(username_email));

        Ok(user.map(|user| user.clone()))
    }

    async fn query_user_by_username<'self_ref>(
        &'self_ref self,
        username: UsernameRef<'self_ref>,
    ) -> Result<Option<User>, Self::Error> {
        let lock = self.store().users.lock().await;

        let user = lock.values().find(|user| user.username == username);

        Ok(user.map(|user| user.clone()))
    }

    async fn set_user_name(&self, user_id: UserId, user_name: Username) -> Result<(), Self::Error> {
        let mut lock = self.store().users.lock().await;
        let orgs_lock = self.store().organizations.lock().await;

        if lock.values().any(|it| it.username == user_name) {
            return Err(InMemoryError::NameConflict);
        }

        if orgs_lock.values().any(|it| it.name == user_name) {
            return Err(InMemoryError::NameConflict);
        }

        lock.get_mut(&user_id)
            .map(|user| user.username = user_name)
            .ok_or(InMemoryError::UserNotFound)
    }

    async fn create_repo(&self, repo: Repo) -> Result<(), Self::Error> {
        let mut lock = self.store().repos.lock().await;

        if lock.contains_key(&repo.id) {
            return Err(InMemoryError::RepoAlreadyExists);
        }

        if lock
            .values()
            .any(|it| it.namespace == repo.namespace && it.name == repo.name)
        {
            return Err(InMemoryError::RepoAlreadyExists);
        }

        lock.insert(repo.id, repo);

        Ok(())
    }

    async fn query_repo(&self, repo_id: RepoId) -> Result<Repo, Self::Error> {
        let lock = self.store().repos.lock().await;

        lock.get(&repo_id)
            .map(|repo| repo.clone())
            .ok_or(InMemoryError::RepoNotFound)
    }

    async fn query_repo_by_name<'self_ref>(
        &'self_ref self,
        repo_name: RepoNameRef<'self_ref>,
        repo_namespace: &RepoNamespace,
    ) -> Result<Option<Repo>, Self::Error> {
        let lock = self.store().repos.lock().await;

        let repo = lock
            .values()
            .find(|repo| repo.name == repo_name && repo.namespace == *repo_namespace);

        Ok(repo.map(|repo| repo.clone()))
    }

    async fn set_repo_name(&self, repo_id: RepoId, repo_name: RepoName) -> Result<(), Self::Error> {
        let mut lock = self.store().repos.lock().await;

        lock.get_mut(&repo_id)
            .map(|repo| repo.name = repo_name)
            .ok_or(InMemoryError::RepoNotFound)
    }

    async fn create_organization(&self, org: Organization) -> Result<(), Self::Error> {
        let users_lock = self.store().users.lock().await;
        let mut lock = self.store().organizations.lock().await;

        if users_lock.values().any(|it| it.username == org.name) {
            return Err(InMemoryError::NameConflict);
        }

        if lock.values().any(|it| it.name == org.name) {
            return Err(InMemoryError::NameConflict);
        }

        lock.insert(org.id, org);

        Ok(())
    }

    async fn query_organization(
        &self,
        org_id: OrganizationId,
    ) -> Result<Organization, Self::Error> {
        let lock = self.store().organizations.lock().await;

        lock.get(&org_id)
            .map(|org| org.clone())
            .ok_or(InMemoryError::OrganizationNotFound)
    }

    async fn query_organization_by_name<'self_ref>(
        &'self_ref self,
        org_name: OrganizationNameRef<'self_ref>,
    ) -> Result<Option<Organization>, Self::Error> {
        let lock = self.store().organizations.lock().await;

        let org = lock.values().find(|org| org.name == org_name);

        Ok(org.map(|org| org.clone()))
    }

    async fn set_organization_name(
        &self,
        org_id: OrganizationId,
        org_name: OrganizationName,
    ) -> Result<(), Self::Error> {
        let mut lock = self.store().organizations.lock().await;

        lock.get_mut(&org_id)
            .map(|org| org.name = org_name)
            .ok_or(InMemoryError::OrganizationNotFound)
    }

    async fn set_organization_display_name(
        &self,
        org_id: OrganizationId,
        org_display_name: Option<OrganizationDisplayName>,
    ) -> Result<(), Self::Error> {
        let mut lock = self.store().organizations.lock().await;

        lock.get_mut(&org_id)
            .map(|org| org.display_name = org_display_name)
            .ok_or(InMemoryError::OrganizationNotFound)
    }

    async fn query_organization_member(
        &self,
        org_id: OrganizationId,
        user_id: UserId,
    ) -> Result<Option<OrganizationMember>, Self::Error> {
        let lock = self.store().organization_members.lock().await;

        Ok(lock
            .get(&org_id)
            .and_then(|members| members.get(&user_id))
            .map(|member| member.clone()))
    }

    async fn query_organization_members(
        &self,
        org_id: OrganizationId,
    ) -> Result<Vec<OrganizationMember>, Self::Error> {
        let lock = self.store().organization_members.lock().await;

        lock.get(&org_id)
            .map(|members| members.values().cloned().collect())
            .ok_or(InMemoryError::OrganizationMembersNotFound)
    }

    async fn create_team(&self, team: Team) -> Result<(), Self::Error> {
        let mut lock = self.store().teams.lock().await;

        lock.insert(team.id, team);

        Ok(())
    }

    async fn query_team(&self, team_id: TeamId) -> Result<Team, Self::Error> {
        let lock = self.store().teams.lock().await;

        lock.get(&team_id)
            .map(|team| team.clone())
            .ok_or(InMemoryError::TeamNotFound)
    }

    async fn query_team_by_name<'self_ref>(
        &'self_ref self,
        org_id: OrganizationId,
        team_name: TeamNameRef<'self_ref>,
    ) -> Result<Option<Team>, Self::Error> {
        let lock = self.store().teams.lock().await;

        let team = lock
            .values()
            .find(|team| team.organization_id == org_id && team.name == team_name);

        Ok(team.map(|team| team.clone()))
    }

    async fn set_team_name(&self, team_id: TeamId, team_name: TeamName) -> Result<(), Self::Error> {
        let mut lock = self.store().teams.lock().await;

        lock.get_mut(&team_id)
            .map(|team| team.name = team_name)
            .ok_or(InMemoryError::TeamNotFound)
    }

    async fn set_team_display_name(
        &self,
        team_id: TeamId,
        team_display_name: Option<TeamDisplayName>,
    ) -> Result<(), Self::Error> {
        let mut lock = self.store().teams.lock().await;

        lock.get_mut(&team_id)
            .map(|team| team.display_name = team_display_name)
            .ok_or(InMemoryError::TeamNotFound)
    }

    async fn query_organization_and_team(
        &self,
        team_id: TeamId,
    ) -> Result<(Organization, Team), Self::Error> {
        let team = {
            let lock = self.store().teams.lock().await;

            lock.get(&team_id)
                .map(|team| team.clone())
                .ok_or(InMemoryError::TeamNotFound)?
        };

        let org_id = team.organization_id;

        let org = {
            let lock = self.store().organizations.lock().await;

            lock.get(&org_id)
                .map(|org| org.clone())
                .ok_or(InMemoryError::OrganizationNotFound)?
        };

        Ok((org, team))
    }

    fn as_query_master(self) -> Box<dyn DataClientQueryMaster + 'a> {
        Box::new(InMemoryQueryMaster(self))
    }
}

query_master_impl_trait!(InMemoryQueryMaster, InMemoryQueryImpl);
