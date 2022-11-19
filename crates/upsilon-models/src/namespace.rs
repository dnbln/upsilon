use crate::organization::{Organization, OrganizationId, Team, TeamId};
use crate::users::{User, UserId};

#[derive(Copy, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum NamespaceId {
    GlobalNamespace,
    User(UserId),
    Organization(OrganizationId),
    Team(OrganizationId, TeamId),
}

#[derive(Debug, Clone)]
pub enum Namespace {
    GlobalNamespace,
    User(User),
    Organization(Organization),
    Team(Organization, Team),
}

impl Namespace {
    pub fn id(&self) -> NamespaceId {
        match self {
            Namespace::GlobalNamespace => NamespaceId::GlobalNamespace,
            Namespace::User(user) => NamespaceId::User(user.id),
            Namespace::Organization(org) => NamespaceId::Organization(org.id),
            Namespace::Team(org, team) => NamespaceId::Team(org.id, team.id),
        }
    }
}

crate::utils::str_newtype!(PlainNamespaceFragment, PlainNamespaceFragmentRef);

#[derive(Debug, Clone)]
pub struct PlainNamespace {
    pub fragments: Vec<PlainNamespaceFragment>,
}

impl PlainNamespace {
    pub fn new<T, I>(iter: I) -> Self
    where
        T: Into<PlainNamespaceFragment>,
        I: IntoIterator<Item = T>,
    {
        Self {
            fragments: iter.into_iter().map(Into::into).collect(),
        }
    }
}
