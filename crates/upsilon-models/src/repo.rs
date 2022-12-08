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

use std::fmt;
use std::fmt::Formatter;

use bitflags::bitflags;

use crate::namespace::{
    NamespaceId, NamespaceKind, PlainNamespaceFragment, PlainNamespaceFragmentRef
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
    /// Permissions all users have by default.
    pub global_permissions: RepoPermissions,
}

bitflags! {
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct RepoPermissions: u64 {
        const NONE = 0;
        const READ = 0b0000_0001;
        const WRITE = 0b0000_0010;
        const ADMIN = 0b0000_0100;
    }
}

impl RepoPermissions {
    pub fn can_read(&self) -> bool {
        self.contains(RepoPermissions::READ)
    }

    pub fn can_write(&self) -> bool {
        self.contains(RepoPermissions::WRITE)
    }

    pub fn has_admin(&self) -> bool {
        self.contains(RepoPermissions::ADMIN)
    }
}

impl fmt::Debug for RepoPermissions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let mut wrote_any = false;

        write!(f, "RepoPermissions(")?;
        if self.contains(RepoPermissions::READ) {
            write!(f, "READ")?;
            wrote_any = true;
        }

        if self.contains(RepoPermissions::WRITE) {
            if wrote_any {
                write!(f, ", ")?;
            }
            write!(f, "WRITE")?;
            wrote_any = true;
        }

        if self.contains(RepoPermissions::ADMIN) {
            if wrote_any {
                write!(f, ", ")?;
            }
            write!(f, "ADMIN")?;
            wrote_any = true;
        }

        if !wrote_any {
            write!(f, "NONE")?;
        }

        write!(f, ")")?;

        Ok(())
    }
}
