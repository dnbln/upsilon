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

#![deny(clippy::map_clone)]

use std::collections::BTreeMap;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use upsilon_data::{
    async_trait, query_master_impl_trait, CommonDataClientError, CommonDataClientErrorExtractor, DataClient, DataClientMaster, DataClientQueryImpl, DataClientQueryMaster
};
use upsilon_models::namespace::{NamespaceId, NamespaceKind};
use upsilon_models::organization::{
    Organization, OrganizationDisplayName, OrganizationId, OrganizationMember, OrganizationName, OrganizationNameRef, Team, TeamDisplayName, TeamId, TeamName, TeamNameRef
};
use upsilon_models::repo::{Repo, RepoId, RepoName, RepoNameRef, RepoNamespace, RepoPermissions};
use upsilon_models::users::{User, UserId, UserSshKey, Username, UsernameRef};
use upsilon_stdx::TakeIfUnless;

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
    #[error("Perms already exist")]
    PermsAlreadyExist,
    #[error("Perms not found")]
    PermsNotFound,
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
            InMemoryError::NameConflict => CommonDataClientError::NameConflict,
            InMemoryError::PermsAlreadyExist => CommonDataClientError::PermsAlreadyExist,
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
    users: Arc<RwLock<BTreeMap<UserId, User>>>,
    repos: Arc<RwLock<BTreeMap<RepoId, Repo>>>,
    organizations: Arc<RwLock<BTreeMap<OrganizationId, Organization>>>,
    organization_members:
        Arc<RwLock<BTreeMap<OrganizationId, BTreeMap<UserId, OrganizationMember>>>>,
    teams: Arc<RwLock<BTreeMap<TeamId, Team>>>,
    repo_permissions: Arc<RwLock<BTreeMap<RepoId, BTreeMap<UserId, RepoPermissions>>>>,
    ssh_key_map: Arc<RwLock<Vec<(UserSshKey, UserId)>>>,
}

impl InMemoryDataStore {
    fn new() -> Self {
        fn new_map<K, V>() -> Arc<RwLock<BTreeMap<K, V>>> {
            Arc::new(RwLock::new(BTreeMap::new()))
        }

        Self {
            users: new_map(),
            repos: new_map(),
            organizations: new_map(),
            organization_members: new_map(),
            teams: new_map(),
            repo_permissions: new_map(),
            ssh_key_map: Arc::new(RwLock::new(vec![])),
        }
    }
}

pub struct InMemoryDataClient(InMemoryStorageConfiguration, Box<InMemoryDataStore>);

#[async_trait]
impl DataClient for InMemoryDataClient {
    type Error = InMemoryError;
    type InnerConfiguration = InMemoryStorageConfiguration;
    type QueryImpl<'a> = InMemoryQueryImpl<'a>;

    async fn init_client(config: Self::InnerConfiguration) -> Result<Self, Self::Error>
    where
        Self: Sized,
    {
        Ok(Self(config, Box::new(InMemoryDataStore::new())))
    }

    fn data_client_query_impl(&self) -> Self::QueryImpl<'_> {
        InMemoryQueryImpl(self)
    }
}

#[async_trait]
impl DataClientMaster for InMemoryDataClient {
    fn query_master<'a>(&'a self) -> Box<dyn DataClientQueryMaster + 'a> {
        self.data_client_query_impl().into_query_master()
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

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[repr(u8)]
enum RwGuardKind {
    None = 0,
    Read = 1,
    Write = 2,
}

impl RwGuardKind {
    fn promote(&mut self, other: Self) {
        if *self < other {
            *self = other;
        }
    }
}

enum OptRwGuard<'a, T> {
    Read(RwLockReadGuard<'a, T>),
    Write(RwLockWriteGuard<'a, T>),
    None,
}

impl<'a, T> OptRwGuard<'a, T> {
    async fn from(v: &'a RwLock<T>, kind: RwGuardKind) -> OptRwGuard<'a, T> {
        match kind {
            RwGuardKind::Read => Self::Read(v.read().await),
            RwGuardKind::Write => Self::Write(v.write().await),
            RwGuardKind::None => Self::None,
        }
    }

    fn expect(&'a self, s: &str) -> &'a T {
        match self {
            OptRwGuard::Read(r) => r,
            OptRwGuard::Write(w) => w,
            OptRwGuard::None => panic!("OptRwGuard::expect called on OptRwGuard::None: {s}"),
        }
    }

    fn expect_mut(&mut self, s: &str) -> &mut T {
        match self {
            OptRwGuard::Read(_r) => {
                panic!("OptRwGuard::expect_mut called on OptRwGuard::Read: {s}")
            }
            OptRwGuard::Write(w) => w,
            OptRwGuard::None => panic!("OptRwGuard::expect_mut called on OptRwGuard::None: {s}"),
        }
    }
}

struct InMemoryNamespaceMutQueryLock<'a> {
    users: OptRwGuard<'a, BTreeMap<UserId, User>>,
    orgs: OptRwGuard<'a, BTreeMap<OrganizationId, Organization>>,
    teams: OptRwGuard<'a, BTreeMap<TeamId, Team>>,
    repos: OptRwGuard<'a, BTreeMap<RepoId, Repo>>,

    namespace_kind: NamespaceKind,
    configuration: RwGuardKinds,
}

#[derive(Debug, Copy, Clone)]
struct RwGuardKinds {
    users: RwGuardKind,
    orgs: RwGuardKind,
    teams: RwGuardKind,
    repos: RwGuardKind,
}

impl RwGuardKinds {
    fn need_for_kind(kind: NamespaceKind) -> Self {
        match kind {
            NamespaceKind::GlobalNamespace => Self {
                users: RwGuardKind::Read,
                orgs: RwGuardKind::Read,
                teams: RwGuardKind::None,
                repos: RwGuardKind::Read,
            },
            NamespaceKind::User => Self {
                users: RwGuardKind::Read,
                orgs: RwGuardKind::None,
                teams: RwGuardKind::None,
                repos: RwGuardKind::Read,
            },
            NamespaceKind::Organization => Self {
                users: RwGuardKind::None,
                orgs: RwGuardKind::Read,
                teams: RwGuardKind::Read,
                repos: RwGuardKind::Read,
            },
            NamespaceKind::Team => Self {
                users: RwGuardKind::None,
                orgs: RwGuardKind::None,
                teams: RwGuardKind::None,
                repos: RwGuardKind::Read,
            },
        }
    }

    fn has_all_perms_for(&self, kind: NamespaceKind) -> bool {
        let needed_guard_kinds = Self::need_for_kind(kind);

        self.users >= needed_guard_kinds.users
            && self.orgs >= needed_guard_kinds.orgs
            && self.teams >= needed_guard_kinds.teams
            && self.repos >= needed_guard_kinds.repos
    }

    fn assert_has_all_perms_for(&self, kind: NamespaceKind) {
        if !self.has_all_perms_for(kind) {
            panic!("RwGuardKinds::assert_has_all_perms_for: missing permissions for namespace kind: {kind:?}, have: {self:?}");
        }
    }

    fn need_users(mut self, users: RwGuardKind) -> Self {
        self.users.promote(users);
        self
    }

    fn need_orgs(mut self, orgs: RwGuardKind) -> Self {
        self.orgs.promote(orgs);
        self
    }

    fn need_teams(mut self, teams: RwGuardKind) -> Self {
        self.teams.promote(teams);
        self
    }

    fn need_repos(mut self, repos: RwGuardKind) -> Self {
        self.repos.promote(repos);
        self
    }
}

impl<'a> InMemoryNamespaceMutQueryLock<'a> {
    async fn from_configuration(
        store: &'a InMemoryDataStore,
        namespace_kind: NamespaceKind,
        configuration: RwGuardKinds,
    ) -> InMemoryNamespaceMutQueryLock<'a> {
        Self {
            users: OptRwGuard::from(&store.users, configuration.users).await,
            orgs: OptRwGuard::from(&store.organizations, configuration.orgs).await,
            teams: OptRwGuard::from(&store.teams, configuration.teams).await,
            repos: OptRwGuard::from(&store.repos, configuration.repos).await,
            namespace_kind,
            configuration,
        }
    }

    async fn for_namespace_kind<F>(
        store: &'a InMemoryDataStore,
        namespace_kind: NamespaceKind,
        patch: F,
    ) -> InMemoryNamespaceMutQueryLock<'a>
    where
        F: FnOnce(RwGuardKinds) -> RwGuardKinds,
    {
        Self::from_configuration(
            store,
            namespace_kind,
            patch(RwGuardKinds::need_for_kind(namespace_kind)),
        )
        .await
    }

    fn users(&self) -> &BTreeMap<UserId, User> {
        self.users.expect("missing users lock")
    }

    fn orgs(&self) -> &BTreeMap<OrganizationId, Organization> {
        self.orgs.expect("missing orgs lock")
    }

    fn teams(&self) -> &BTreeMap<TeamId, Team> {
        self.teams.expect("missing teams lock")
    }

    fn repos(&self) -> &BTreeMap<RepoId, Repo> {
        self.repos.expect("missing repos lock")
    }

    fn users_mut(&mut self) -> &mut BTreeMap<UserId, User> {
        self.users.expect_mut("missing/wrong users lock")
    }

    fn orgs_mut(&mut self) -> &mut BTreeMap<OrganizationId, Organization> {
        self.orgs.expect_mut("missing/wrong orgs lock")
    }

    fn teams_mut(&mut self) -> &mut BTreeMap<TeamId, Team> {
        self.teams.expect_mut("missing/wrong teams lock")
    }

    fn repos_mut(&mut self) -> &mut BTreeMap<RepoId, Repo> {
        self.repos.expect_mut("missing/wrong repos lock")
    }

    fn check_allows_name_in_namespace(
        &self,
        name: &str,
        namespace: NamespaceId,
    ) -> Result<(), InMemoryError> {
        if !self.allows_name_in_namespace(name, namespace) {
            return Err(InMemoryError::NameConflict);
        }

        Ok(())
    }

    fn allows_name_in_namespace(&self, name: &str, namespace: NamespaceId) -> bool {
        if namespace.kind() != self.namespace_kind {
            self.configuration
                .assert_has_all_perms_for(namespace.kind());
        }

        match namespace {
            NamespaceId::GlobalNamespace => {
                if self.orgs().values().any(|it| it.name == name) {
                    return false;
                }

                if self.users().values().any(|it| it.username == name) {
                    return false;
                }

                if self
                    .repos()
                    .values()
                    .any(|it| it.namespace == NamespaceId::GlobalNamespace && it.name == name)
                {
                    return false;
                }
            }
            NamespaceId::User(user) => {
                if self
                    .repos()
                    .values()
                    .any(|it| it.namespace == NamespaceId::User(user) && it.name == name)
                {
                    return false;
                }
            }
            NamespaceId::Organization(org) => {
                if self
                    .teams()
                    .values()
                    .any(|it| it.organization_id == org && it.name == name)
                {
                    return false;
                }

                if self
                    .repos()
                    .values()
                    .any(|it| it.namespace == NamespaceId::Organization(org) && it.name == name)
                {
                    return false;
                }
            }
            NamespaceId::Team(org, team) => {
                if self
                    .repos()
                    .values()
                    .any(|it| it.namespace == NamespaceId::Team(org, team) && it.name == name)
                {
                    return false;
                }
            }
        }

        true
    }
}

#[async_trait]
impl<'a> DataClientQueryImpl<'a> for InMemoryQueryImpl<'a> {
    type Error = InMemoryError;

    async fn create_user(&self, user: User) -> Result<(), Self::Error> {
        let mut ns_query_lock = InMemoryNamespaceMutQueryLock::for_namespace_kind(
            self.store(),
            NamespaceKind::GlobalNamespace,
            |it| it.need_users(RwGuardKind::Write),
        )
        .await;

        if ns_query_lock.users().contains_key(&user.id) {
            return Err(InMemoryError::UserAlreadyExists);
        }

        ns_query_lock
            .check_allows_name_in_namespace(user.username.as_str(), NamespaceId::GlobalNamespace)?;

        ns_query_lock.users_mut().insert(user.id, user);

        Ok(())
    }

    async fn query_user(&self, user_id: UserId) -> Result<User, Self::Error> {
        let lock = self.store().users.read().await;

        lock.get(&user_id)
            .cloned()
            .ok_or(InMemoryError::UserNotFound)
    }

    async fn query_user_by_username_email(
        &self,
        username_email: &str,
    ) -> Result<Option<User>, Self::Error> {
        let lock = self.store().users.read().await;

        let user = lock
            .values()
            .find(|user| user.username == username_email || user.emails.contains(username_email));

        Ok(user.cloned())
    }

    async fn query_user_by_username<'self_ref>(
        &'self_ref self,
        username: UsernameRef<'self_ref>,
    ) -> Result<Option<User>, Self::Error> {
        let lock = self.store().users.read().await;

        let user = lock.values().find(|user| user.username == username);

        Ok(user.cloned())
    }

    async fn set_user_name(&self, user_id: UserId, user_name: Username) -> Result<(), Self::Error> {
        let mut ns_query_lock = InMemoryNamespaceMutQueryLock::for_namespace_kind(
            self.store(),
            NamespaceKind::GlobalNamespace,
            |it| it.need_users(RwGuardKind::Write),
        )
        .await;

        ns_query_lock
            .check_allows_name_in_namespace(user_name.as_str(), NamespaceId::GlobalNamespace)?;

        ns_query_lock
            .users_mut()
            .get_mut(&user_id)
            .map(|user| user.username = user_name)
            .ok_or(InMemoryError::UserNotFound)
    }

    async fn add_user_ssh_key(
        &self,
        user_id: UserId,
        key: UserSshKey,
    ) -> Result<bool, Self::Error> {
        let mut lock = self.store().ssh_key_map.write().await;

        if lock.iter().any(|it| it.0 == key) {
            return Ok(false);
        }

        dbg!(user_id);
        dbg!(&key);

        lock.push((key, user_id));

        Ok(true)
    }

    async fn query_user_ssh_key(&self, key: UserSshKey) -> Result<Option<UserId>, Self::Error> {
        let lock = self.store().ssh_key_map.read().await;

        Ok(lock.iter().find_map(|(k, u)| (k == &key).then_some(*u)))
    }

    async fn create_repo(&self, repo: Repo) -> Result<(), Self::Error> {
        let mut ns_query_lock = InMemoryNamespaceMutQueryLock::for_namespace_kind(
            self.store(),
            repo.namespace.kind(),
            |it| it.need_repos(RwGuardKind::Write),
        )
        .await;

        if ns_query_lock.repos().contains_key(&repo.id) {
            return Err(InMemoryError::RepoAlreadyExists);
        }

        ns_query_lock.check_allows_name_in_namespace(repo.name.as_str(), repo.namespace.0)?;

        ns_query_lock.repos_mut().insert(repo.id, repo);

        Ok(())
    }

    async fn query_repo(&self, repo_id: RepoId) -> Result<Repo, Self::Error> {
        let lock = self.store().repos.read().await;

        lock.get(&repo_id)
            .cloned()
            .ok_or(InMemoryError::RepoNotFound)
    }

    async fn query_repo_by_name<'self_ref>(
        &'self_ref self,
        repo_name: RepoNameRef<'self_ref>,
        repo_namespace: &RepoNamespace,
    ) -> Result<Option<Repo>, Self::Error> {
        let lock = self.store().repos.read().await;

        let repo = lock
            .values()
            .find(|repo| repo.name == repo_name && repo.namespace == *repo_namespace);

        Ok(repo.cloned())
    }

    async fn set_repo_name(&self, repo_id: RepoId, repo_name: RepoName) -> Result<(), Self::Error> {
        let mut ns_query_lock = InMemoryNamespaceMutQueryLock::for_namespace_kind(
            self.store(),
            NamespaceKind::GlobalNamespace,
            |it| {
                it.need_users(RwGuardKind::Read)
                    .need_orgs(RwGuardKind::Read)
                    .need_teams(RwGuardKind::Read)
                    .need_repos(RwGuardKind::Write)
            },
        )
        .await;

        let repo_ns = ns_query_lock
            .repos()
            .get(&repo_id)
            .map(|repo| repo.namespace)
            .ok_or(InMemoryError::RepoNotFound)?;

        ns_query_lock.check_allows_name_in_namespace(repo_name.as_str(), repo_ns.0)?;

        ns_query_lock
            .repos_mut()
            .get_mut(&repo_id)
            .map(|repo| repo.name = repo_name)
            .ok_or(InMemoryError::RepoNotFound)
    }

    async fn init_repo_user_perms(
        &self,
        repo_id: RepoId,
        user_id: UserId,
    ) -> Result<(), Self::Error> {
        let mut repo_perms_lock = self.store().repo_permissions.write().await;

        let repo_perms_map = repo_perms_lock.entry(repo_id).or_default();

        if repo_perms_map.contains_key(&user_id) {
            return Err(InMemoryError::PermsAlreadyExist);
        }

        let repos_lock = self.store().repos.read().await;

        let repo = repos_lock
            .get(&repo_id)
            .ok_or(InMemoryError::RepoNotFound)?;
        let global_perms = repo.global_permissions;

        repo_perms_map.insert(user_id, global_perms);

        Ok(())
    }

    async fn query_repo_user_perms(
        &self,
        repo_id: RepoId,
        user_id: UserId,
    ) -> Result<Option<RepoPermissions>, Self::Error> {
        let lock = self.store().repo_permissions.read().await;

        Ok(lock
            .get(&repo_id)
            .and_then(|map| map.get(&user_id).cloned()))
    }

    async fn add_repo_user_perms(
        &self,
        repo_id: RepoId,
        user_id: UserId,
        perms: RepoPermissions,
    ) -> Result<RepoPermissions, Self::Error> {
        let mut lock = self.store().repo_permissions.write().await;

        let repo_perms_map = lock.entry(repo_id).or_default();

        repo_perms_map
            .get_mut(&user_id)
            .map(|existing_perms| {
                *existing_perms |= perms;

                *existing_perms
            })
            .ok_or(InMemoryError::PermsNotFound)
    }

    async fn remove_repo_user_perms(
        &self,
        repo_id: RepoId,
        user_id: UserId,
        perms: RepoPermissions,
    ) -> Result<RepoPermissions, Self::Error> {
        let mut lock = self.store().repo_permissions.write().await;

        let repo_perms_map = lock.entry(repo_id).or_default();

        repo_perms_map
            .get_mut(&user_id)
            .map(|existing_perms| {
                *existing_perms &= !perms;

                *existing_perms
            })
            .ok_or(InMemoryError::PermsNotFound)
    }

    async fn create_organization(&self, org: Organization) -> Result<(), Self::Error> {
        let mut ns_query_lock = InMemoryNamespaceMutQueryLock::for_namespace_kind(
            self.store(),
            NamespaceKind::GlobalNamespace,
            |it| it.need_orgs(RwGuardKind::Write),
        )
        .await;

        ns_query_lock
            .check_allows_name_in_namespace(org.name.as_str(), NamespaceId::GlobalNamespace)?;

        ns_query_lock.orgs_mut().insert(org.id, org);

        Ok(())
    }

    async fn query_organization(
        &self,
        org_id: OrganizationId,
    ) -> Result<Organization, Self::Error> {
        let lock = self.store().organizations.read().await;

        lock.get(&org_id)
            .cloned()
            .ok_or(InMemoryError::OrganizationNotFound)
    }

    async fn query_organization_by_name<'self_ref>(
        &'self_ref self,
        org_name: OrganizationNameRef<'self_ref>,
    ) -> Result<Option<Organization>, Self::Error> {
        let lock = self.store().organizations.read().await;

        let org = lock.values().find(|org| org.name == org_name);

        Ok(org.cloned())
    }

    async fn set_organization_name(
        &self,
        org_id: OrganizationId,
        org_name: OrganizationName,
    ) -> Result<(), Self::Error> {
        let mut ns_query_lock = InMemoryNamespaceMutQueryLock::for_namespace_kind(
            self.store(),
            NamespaceKind::GlobalNamespace,
            |it| it.need_orgs(RwGuardKind::Write),
        )
        .await;

        ns_query_lock
            .check_allows_name_in_namespace(org_name.as_str(), NamespaceId::GlobalNamespace)?;

        ns_query_lock
            .orgs_mut()
            .get_mut(&org_id)
            .map(|org| org.name = org_name)
            .ok_or(InMemoryError::OrganizationNotFound)
    }

    async fn set_organization_display_name(
        &self,
        org_id: OrganizationId,
        org_display_name: Option<OrganizationDisplayName>,
    ) -> Result<(), Self::Error> {
        let mut lock = self.store().organizations.write().await;

        lock.get_mut(&org_id)
            .map(|org| org.display_name = org_display_name)
            .ok_or(InMemoryError::OrganizationNotFound)
    }

    async fn query_organization_member(
        &self,
        org_id: OrganizationId,
        user_id: UserId,
    ) -> Result<Option<OrganizationMember>, Self::Error> {
        let lock = self.store().organization_members.read().await;

        Ok(lock
            .get(&org_id)
            .and_then(|members| members.get(&user_id))
            .cloned())
    }

    async fn query_organization_members(
        &self,
        org_id: OrganizationId,
    ) -> Result<Vec<OrganizationMember>, Self::Error> {
        let lock = self.store().organization_members.read().await;

        lock.get(&org_id)
            .map(|members| members.values().cloned().collect())
            .ok_or(InMemoryError::OrganizationMembersNotFound)
    }

    async fn query_user_organizations(
        &self,
        user_id: UserId,
    ) -> Result<Vec<OrganizationMember>, Self::Error> {
        let lock = self.store().organization_members.read().await;

        Ok(lock
            .values()
            .filter_map(|members| members.take_if(|members| members.contains_key(&user_id)))
            .flat_map(|members| members.values().cloned())
            .collect())
    }

    async fn create_team(&self, team: Team) -> Result<(), Self::Error> {
        let mut ns_query_lock = InMemoryNamespaceMutQueryLock::for_namespace_kind(
            self.store(),
            NamespaceKind::Organization,
            |it| it.need_teams(RwGuardKind::Write),
        )
        .await;

        ns_query_lock.check_allows_name_in_namespace(
            team.name.as_str(),
            NamespaceId::Organization(team.organization_id),
        )?;

        ns_query_lock.teams_mut().insert(team.id, team);

        Ok(())
    }

    async fn query_team(&self, team_id: TeamId) -> Result<Team, Self::Error> {
        let lock = self.store().teams.read().await;

        lock.get(&team_id)
            .cloned()
            .ok_or(InMemoryError::TeamNotFound)
    }

    async fn query_organization_teams(
        &self,
        org_id: OrganizationId,
    ) -> Result<Vec<Team>, Self::Error> {
        let lock = self.store().teams.read().await;

        Ok(lock
            .values()
            .filter(|team| team.organization_id == org_id)
            .cloned()
            .collect())
    }

    async fn query_team_by_name<'self_ref>(
        &'self_ref self,
        org_id: OrganizationId,
        team_name: TeamNameRef<'self_ref>,
    ) -> Result<Option<Team>, Self::Error> {
        let lock = self.store().teams.read().await;

        let team = lock
            .values()
            .find(|team| team.organization_id == org_id && team.name == team_name);

        Ok(team.cloned())
    }

    async fn set_team_name(&self, team_id: TeamId, team_name: TeamName) -> Result<(), Self::Error> {
        let mut ns_query_lock = InMemoryNamespaceMutQueryLock::for_namespace_kind(
            self.store(),
            NamespaceKind::Organization,
            |it| it.need_teams(RwGuardKind::Write),
        )
        .await;

        let org_id = ns_query_lock
            .teams()
            .get(&team_id)
            .map(|team| team.organization_id)
            .ok_or(InMemoryError::TeamNotFound)?;

        ns_query_lock.check_allows_name_in_namespace(
            team_name.as_str(),
            NamespaceId::Organization(org_id),
        )?;

        ns_query_lock
            .teams_mut()
            .get_mut(&team_id)
            .map(|team| team.name = team_name)
            .ok_or(InMemoryError::TeamNotFound)
    }

    async fn set_team_display_name(
        &self,
        team_id: TeamId,
        team_display_name: Option<TeamDisplayName>,
    ) -> Result<(), Self::Error> {
        let mut lock = self.store().teams.write().await;

        lock.get_mut(&team_id)
            .map(|team| team.display_name = team_display_name)
            .ok_or(InMemoryError::TeamNotFound)
    }

    async fn query_organization_and_team(
        &self,
        team_id: TeamId,
    ) -> Result<(Organization, Team), Self::Error> {
        let team = {
            let lock = self.store().teams.read().await;

            lock.get(&team_id)
                .cloned()
                .ok_or(InMemoryError::TeamNotFound)?
        };

        let org_id = team.organization_id;

        let org = {
            let lock = self.store().organizations.read().await;

            lock.get(&org_id)
                .cloned()
                .ok_or(InMemoryError::OrganizationNotFound)?
        };

        Ok((org, team))
    }

    async fn query_team_members(
        &self,
        organization_id: OrganizationId,
        team_id: TeamId,
    ) -> Result<Vec<OrganizationMember>, Self::Error> {
        let lock = self.store().organization_members.read().await;

        Ok(lock
            .get(&organization_id)
            .map(|members: &BTreeMap<UserId, OrganizationMember>| {
                members
                    .values()
                    .filter(|member| member.teams.contains(&team_id))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default())
    }

    fn into_query_master(self) -> Box<dyn DataClientQueryMaster + 'a> {
        Box::new(InMemoryQueryMaster(self))
    }
}

query_master_impl_trait!(InMemoryQueryMaster, InMemoryQueryImpl);
