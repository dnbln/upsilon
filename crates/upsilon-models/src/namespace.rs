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

use crate::organization::{Organization, OrganizationId, Team, TeamId};
use crate::users::{User, UserId};

#[derive(Copy, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum NamespaceId {
    GlobalNamespace,
    User(UserId),
    Organization(OrganizationId),
    Team(OrganizationId, TeamId),
}

impl NamespaceId {
    pub fn kind(&self) -> NamespaceKind {
        match self {
            Self::GlobalNamespace => NamespaceKind::GlobalNamespace,
            Self::User(_) => NamespaceKind::User,
            Self::Organization(_) => NamespaceKind::Organization,
            Self::Team(_, _) => NamespaceKind::Team,
        }
    }
}

#[derive(Copy, Debug, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum NamespaceKind {
    GlobalNamespace,
    User,
    Organization,
    Team,
}

#[derive(Debug, Clone)]
pub enum Namespace {
    GlobalNamespace,
    User(User),
    Organization(Organization),
    Team(Organization, Team),
}

impl Namespace {
    pub fn kind(&self) -> NamespaceKind {
        match self {
            Self::GlobalNamespace => NamespaceKind::GlobalNamespace,
            Self::User(_) => NamespaceKind::User,
            Self::Organization(_) => NamespaceKind::Organization,
            Self::Team(_, _) => NamespaceKind::Team,
        }
    }
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
