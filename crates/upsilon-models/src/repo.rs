use crate::namespace::{
    NamespaceId, NamespaceKind, PlainNamespaceFragment, PlainNamespaceFragmentRef,
};

upsilon_id::id_ty! {
    #[uuid]
    #[timestamped]
    pub struct RepoId;
}

crate::utils::str_newtype!(RepoName, RepoNameRef @derives [PartialOrd, Ord]);
crate::utils::str_newtype! {
    @conversions #[all]
    RepoName, RepoNameRef,
    PlainNamespaceFragment, PlainNamespaceFragmentRef
}

crate::utils::str_newtype!(RepoDisplayName, RepoDisplayNameRef);

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct RepoNamespace(pub NamespaceId);

impl RepoNamespace {
    pub fn kind(&self) -> NamespaceKind {
        self.0.kind()
    }
}

impl PartialEq<NamespaceId> for RepoNamespace {
    fn eq(&self, other: &NamespaceId) -> bool {
        self.0 == *other
    }
}

#[derive(Debug, Clone)]
pub struct Repo {
    pub id: RepoId,
    pub name: RepoName,
    pub namespace: RepoNamespace,
    pub display_name: Option<RepoDisplayName>,
}

impl Repo {
    pub fn new(id: RepoId, name: RepoName, namespace: RepoNamespace) -> Self {
        Self {
            id,
            name,
            namespace,
            display_name: None,
        }
    }
}
