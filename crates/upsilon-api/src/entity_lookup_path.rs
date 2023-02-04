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

use std::ops::Index;

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

pub struct EntityLookupPath {
    path: Vec<PlainNamespaceFragment>,
}

impl EntityLookupPath {
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

        Ok(EntityLookupPath {
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

impl Index<usize> for EntityLookupPath {
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

impl<'r> FromSegments<'r> for EntityLookupPath {
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

impl<'r> FromParam<'r> for EntityLookupPath {
    type Error = NsLookupPathError;

    fn from_param(param: &'r str) -> Result<Self, Self::Error> {
        Self::from_iter(
            param
                .split(LOOKUP_PATH_SEGMENT_SEPARATOR)
                .collect::<Vec<_>>(),
        )
    }
}

pub enum ResolvedEntity {
    GlobalNamespace,
    User(User),
    Organization(Organization),
    Team(Organization, Team),
    Repo {
        parent_ns: Box<ResolvedEntity>,
        repo: Repo,
    },
}

pub async fn resolve<'a>(
    qm: &'a DataQueryMaster<'a>,
    path: &'a EntityLookupPath,
) -> ApiResult<Option<ResolvedEntity>> {
    let mut namespace = ResolvedEntity::GlobalNamespace;

    // get the namespace
    for i in 0..path.len() {
        let fragment = &path[i];

        let new_ns = match namespace {
            ns @ ResolvedEntity::GlobalNamespace => {
                match qm.query_organization_by_name(fragment).await? {
                    Some(org) => ResolvedEntity::Organization(org),
                    None => match qm.query_user_by_username(fragment).await? {
                        Some(user) => ResolvedEntity::User(user),
                        None => match qm
                            .query_repo_by_name(
                                fragment,
                                &RepoNamespace(NamespaceId::GlobalNamespace),
                            )
                            .await?
                        {
                            Some(repo) => ResolvedEntity::Repo {
                                parent_ns: Box::new(ns),
                                repo,
                            },
                            None => return Ok(None),
                        },
                    },
                }
            }
            ResolvedEntity::User(user) => {
                match qm
                    .query_repo_by_name(fragment, &RepoNamespace(NamespaceId::User(user.id)))
                    .await?
                {
                    Some(repo) => ResolvedEntity::Repo {
                        parent_ns: Box::new(ResolvedEntity::User(user)),
                        repo,
                    },
                    None => return Ok(None),
                }
            }
            ResolvedEntity::Organization(org) => {
                match qm.query_team_by_name(org.id, fragment).await? {
                    Some(team) => ResolvedEntity::Team(org, team),
                    None => match qm
                        .query_repo_by_name(
                            fragment,
                            &RepoNamespace(NamespaceId::Organization(org.id)),
                        )
                        .await?
                    {
                        Some(repo) => ResolvedEntity::Repo {
                            parent_ns: Box::new(ResolvedEntity::Organization(org)),
                            repo,
                        },
                        None => return Ok(None),
                    },
                }
            }
            ResolvedEntity::Team(org, team) => match qm
                .query_repo_by_name(fragment, &RepoNamespace(NamespaceId::Team(org.id, team.id)))
                .await?
            {
                Some(repo) => ResolvedEntity::Repo {
                    parent_ns: Box::new(ResolvedEntity::Team(org, team)),
                    repo,
                },
                None => return Ok(None),
            },
            ResolvedEntity::Repo { .. } => return Err(crate::error::Error::ResolveImpossible),
        };
        namespace = new_ns;
    }

    Ok(Some(namespace))
}
