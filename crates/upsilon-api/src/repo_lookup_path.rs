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

use std::ops::Index;
use std::path::PathBuf;

use rocket::http::uri::fmt::Path;
use rocket::http::uri::Segments;
use rocket::request::{FromParam, FromSegments};
use upsilon_data::DataQueryMaster;
use upsilon_models::namespace::{NamespaceId, PlainNamespaceFragment, PlainNamespaceFragmentRef};
use upsilon_models::organization::{Organization, Team};
use upsilon_models::repo::{Repo, RepoNameRef, RepoNamespace};
use upsilon_models::users::User;

use crate::error::ApiResult;

const LOOKUP_PATH_SEGMENT_SEPARATOR: char = '.';

pub struct RepoLookupPath {
    path: Vec<PlainNamespaceFragment>,
}

impl RepoLookupPath {
    pub(crate) fn from_iter<T, I>(iter: I) -> Result<Self, NsLookupPathError>
    where
        I: IntoIterator<Item = T>,
        I::IntoIter: ExactSizeIterator,
        T: Into<PlainNamespaceFragment>,
    {
        let iter = iter.into_iter();

        if iter.len() == 0 {
            return Err(NsLookupPathError::Empty);
        }

        if iter.len() > 3 {
            return Err(NsLookupPathError::TooManySegments);
        }

        Ok(RepoLookupPath {
            path: iter.map(Into::into).collect(),
        })
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn last(&self) -> PlainNamespaceFragmentRef {
        self.path[self.len() - 1].as_ref()
    }

    pub fn repo_name(&self) -> RepoNameRef {
        RepoNameRef::from(self.last())
    }
}

impl Index<usize> for RepoLookupPath {
    type Output = PlainNamespaceFragment;

    fn index(&self, index: usize) -> &Self::Output {
        &self.path[index]
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NsLookupPathError {
    #[error("empty")]
    Empty,
    #[error("too many segments")]
    TooManySegments,
}

impl<'r> FromSegments<'r> for RepoLookupPath {
    type Error = NsLookupPathError;

    fn from_segments(segments: Segments<'r, Path>) -> Result<Self, Self::Error> {
        struct SegmentsWrapper<'r>(Segments<'r, Path>);

        impl<'r> Iterator for SegmentsWrapper<'r> {
            type Item = &'r str;

            fn next(&mut self) -> Option<Self::Item> {
                self.0.next()
            }
        }

        impl<'r> ExactSizeIterator for SegmentsWrapper<'r> {
            fn len(&self) -> usize {
                self.0.len()
            }
        }

        Self::from_iter(SegmentsWrapper(segments))
    }
}

impl<'r> FromParam<'r> for RepoLookupPath {
    type Error = NsLookupPathError;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Self::from_iter(
            param
                .split(LOOKUP_PATH_SEGMENT_SEPARATOR)
                .collect::<Vec<_>>(),
        )
    }
}

pub enum ResolvedRepoAndNamespace {
    GlobalNamespace(Repo),
    User(User, Repo),
    Organization(Organization, Repo),
    Team(Organization, Team, Repo),
}

impl ResolvedRepoAndNamespace {
    pub fn path(&self) -> PathBuf {
        match self {
            ResolvedRepoAndNamespace::GlobalNamespace(repo) => PathBuf::from(repo.name.as_str()),
            ResolvedRepoAndNamespace::User(user, repo) => {
                let mut path = PathBuf::from(user.username.as_str());
                path.push(repo.name.as_str());
                path
            }
            ResolvedRepoAndNamespace::Organization(org, repo) => {
                let mut path = PathBuf::from(org.name.as_str());
                path.push(repo.name.as_str());
                path
            }
            ResolvedRepoAndNamespace::Team(org, team, repo) => {
                let mut path = PathBuf::from(org.name.as_str());
                path.push(team.name.as_str());
                path.push(repo.name.as_str());
                path
            }
        }
    }
}

pub enum ResolvedRepoNamespace {
    GlobalNamespace,
    User(User),
    Organization(Organization),
    Team(Organization, Team),
}

impl ResolvedRepoNamespace {
    fn resolved(self, repo: Repo) -> ResolvedRepoAndNamespace {
        match self {
            ResolvedRepoNamespace::GlobalNamespace => {
                ResolvedRepoAndNamespace::GlobalNamespace(repo)
            }
            ResolvedRepoNamespace::User(user) => ResolvedRepoAndNamespace::User(user, repo),
            ResolvedRepoNamespace::Organization(org) => {
                ResolvedRepoAndNamespace::Organization(org, repo)
            }
            ResolvedRepoNamespace::Team(org, team) => {
                ResolvedRepoAndNamespace::Team(org, team, repo)
            }
        }
    }

    pub(crate) fn namespace_id(&self) -> NamespaceId {
        match self {
            ResolvedRepoNamespace::GlobalNamespace => NamespaceId::GlobalNamespace,
            ResolvedRepoNamespace::User(user) => NamespaceId::User(user.id),
            ResolvedRepoNamespace::Organization(org) => NamespaceId::Organization(org.id),
            ResolvedRepoNamespace::Team(org, team) => NamespaceId::Team(org.id, team.id),
        }
    }
}

pub async fn resolve_namespace<'a>(
    qm: &'a DataQueryMaster<'a>,
    repo_ns: &'a RepoLookupPath,
) -> ApiResult<ResolvedRepoNamespace> {
    let mut namespace = ResolvedRepoNamespace::GlobalNamespace;

    // get the namespace
    for i in 0..(repo_ns.len() - 1) {
        let fragment = &repo_ns[i];

        namespace = match namespace {
            ResolvedRepoNamespace::GlobalNamespace => {
                match qm.query_organization_by_name(fragment).await? {
                    Some(org) => ResolvedRepoNamespace::Organization(org),
                    None => match qm.query_user_by_username(fragment).await? {
                        Some(user) => ResolvedRepoNamespace::User(user),
                        None => return Err(crate::error::Error::ResolveImpossible),
                    },
                }
            }
            ResolvedRepoNamespace::User(_) => return Err(crate::error::Error::ResolveImpossible),
            ResolvedRepoNamespace::Organization(org) => {
                match qm.query_team_by_name(org.id, fragment).await? {
                    Some(team) => ResolvedRepoNamespace::Team(org, team),
                    None => return Err(crate::error::Error::ResolveImpossible),
                }
            }
            ResolvedRepoNamespace::Team(_, _) => {
                return Err(crate::error::Error::ResolveImpossible)
            }
        };
    }

    Ok(namespace)
}

pub async fn resolve<'a>(
    qm: &'a DataQueryMaster<'a>,
    repo_ns: &'a RepoLookupPath,
) -> ApiResult<Option<ResolvedRepoAndNamespace>> {
    let namespace = resolve_namespace(qm, repo_ns).await?;

    let repo_name = repo_ns.last();
    let ns_id = match &namespace {
        ResolvedRepoNamespace::GlobalNamespace => NamespaceId::GlobalNamespace,
        ResolvedRepoNamespace::User(user) => NamespaceId::User(user.id),
        ResolvedRepoNamespace::Organization(org) => NamespaceId::Organization(org.id),
        ResolvedRepoNamespace::Team(org, team) => NamespaceId::Team(org.id, team.id),
    };

    let repo = qm
        .query_repo_by_name(repo_name, &RepoNamespace(ns_id))
        .await?;

    Ok(repo.map(|repo| namespace.resolved(repo)))
}
