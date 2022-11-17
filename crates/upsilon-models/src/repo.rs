use crate::organization::{OrganizationId, TeamId};
use crate::users::UserId;
upsilon_id::id_ty! {
    #[uuid]
    #[timestamped]
    pub struct RepoId;
}

crate::utils::str_newtype!(RepoName);

#[derive(Copy, Clone, Debug)]
pub enum RepoNamespace {
    GlobalNamespace,
    User(UserId),
    Organization(OrganizationId),
    Team(OrganizationId, TeamId),
}

#[derive(Debug, Clone)]
pub struct Repo {
    pub id: RepoId,
    pub name: RepoName,
    pub namespace: RepoNamespace,
}
