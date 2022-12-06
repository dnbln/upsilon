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

#![feature(drain_filter)]

mod config;
mod daemon;
mod http_backend;

use std::path::{Path, PathBuf};
use std::result::Result as StdResult;

pub use git2::{BranchType, TreeWalkMode, TreeWalkResult};
use git2::{ConfigLevel, TreeEntry, TreeIter};
pub use http_backend::{
    handle as http_backend_handle, GitBackendCgiRequest, GitBackendCgiRequestMethod, GitBackendCgiResponse, HandleError as HttpBackendHandleError
};

pub use self::config::UpsilonVcsConfig;
pub use self::daemon::{spawn_daemon, SpawnDaemonError};
use crate::config::{GitHttpProtocol, GitProtocol};

impl UpsilonVcsConfig {
    pub fn repo_dir(&self, repo: impl AsRef<Path>) -> PathBuf {
        self.get_path().join(repo)
    }
}

pub struct Repository {
    repo: git2::Repository,
}

impl Repository {
    pub fn is_bare(&self) -> bool {
        self.repo.is_bare()
    }

    pub fn find_commit(&self, commit: &str) -> Result<Commit> {
        Ok(Commit {
            commit: self.repo.find_commit(commit.parse()?)?,
        })
    }

    pub fn branches(&self, filter: Option<BranchType>) -> Result<Branches<'_>> {
        Ok(Branches {
            branches: self.repo.branches(filter)?,
        })
    }

    pub fn find_branch(&self, name: &str) -> Result<Branch> {
        match self
            .branches(None)?
            .find_map(|it| -> Option<Result<Branch>> {
                it.and_then(|(b, _)| Ok((b.name()? == Some(name)).then_some(b)))
                    .transpose()
            }) {
            Some(Ok(b)) => Ok(b),
            Some(Err(e)) => Err(e),
            None => Err(Error::Unknown),
        }
    }
}

pub struct Branches<'r> {
    branches: git2::Branches<'r>,
}

impl<'r> Iterator for Branches<'r> {
    type Item = Result<(Branch<'r>, BranchType)>;

    fn next(&mut self) -> Option<Self::Item> {
        let r = self.branches.next()?;
        let (branch, branch_type) = match r {
            Ok(b) => b,
            Err(e) => return Some(Err(e.into())),
        };

        Some(Ok((Branch { branch }, branch_type)))
    }
}

pub struct Branch<'r> {
    branch: git2::Branch<'r>,
}

impl<'r> Branch<'r> {
    pub fn name(&self) -> Result<Option<&str>> {
        Ok(self.branch.name()?)
    }

    pub fn get_commit(&self) -> Result<Commit> {
        Ok(Commit {
            commit: self.branch.get().peel_to_commit()?,
        })
    }
}

#[derive(Clone)]
pub struct Commit<'r> {
    commit: git2::Commit<'r>,
}

impl<'r> Commit<'r> {
    pub fn message(&self) -> Option<&str> {
        self.commit.message()
    }

    pub fn displayable_message(&self) -> &str {
        self.message().unwrap_or("<invalid UTF-8>")
    }

    pub fn author(&self) -> Signature {
        Signature {
            signature: self.commit.author(),
        }
    }

    pub fn committer(&self) -> Signature {
        Signature {
            signature: self.commit.committer(),
        }
    }

    pub fn parent(&self, i: usize) -> Result<Commit<'r>> {
        Ok(Commit {
            commit: self.commit.parent(i)?,
        })
    }

    pub fn parent_count(&self) -> usize {
        self.commit.parent_count()
    }

    pub fn parents<'a>(&'a self) -> CommitParents<'a, 'r> {
        CommitParents {
            parents: self.commit.parents(),
        }
    }

    pub fn is_root(&self) -> bool {
        self.parent_count() == 0
    }

    pub fn is_merge_commit(&self) -> bool {
        self.parent_count() >= 2
    }

    pub fn last_common_commit(&self, other: &Commit<'r>) -> Option<Commit<'r>> {
        self.common_ascendants_ignoring_errors(other).last()
    }

    fn common_ascendants_ignoring_errors(
        &self,
        other: &Commit<'r>,
    ) -> impl Iterator<Item = Commit<'r>> {
        let mut self_ascendants = self.self_and_all_ascendants().collect::<Vec<_>>();
        let mut other_ascendants = other.self_and_all_ascendants().collect::<Vec<_>>();

        self_ascendants.reverse();
        other_ascendants.reverse();

        self_ascendants
            .into_iter()
            .zip(other_ascendants.into_iter())
            .map(|p| match p {
                (Ok(a), Ok(b)) => Ok((a, b)),
                (Err(a), _) => Err(a),
                (_, Err(b)) => Err(b),
            })
            .filter_map(|r| r.ok())
            .take_while(|(a, b)| a.commit.id() == b.commit.id())
            .map(|(a, _)| a)
    }

    pub fn all_ascendants(&self) -> AllCommitAscendants<'r> {
        AllCommitAscendants {
            current: self.clone(),
        }
    }

    pub fn self_and_all_ascendants(&self) -> impl Iterator<Item = Result<Commit<'r>>> {
        std::iter::once(Ok(self.clone())).chain(self.all_ascendants())
    }

    pub fn tree(&self) -> Result<Tree<'r>> {
        Ok(Tree {
            tree: self.commit.tree()?,
        })
    }
}

pub struct CommitParents<'c, 'r> {
    parents: git2::Parents<'c, 'r>,
}

impl<'a, 'r> Iterator for CommitParents<'a, 'r> {
    type Item = Commit<'r>;

    fn next(&mut self) -> Option<Self::Item> {
        self.parents.next().map(|commit| Commit { commit })
    }
}

pub struct AllCommitAscendants<'r> {
    current: Commit<'r>,
}

impl<'r> Iterator for AllCommitAscendants<'r> {
    type Item = Result<Commit<'r>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current.is_root() {
            return None;
        }

        let commit = match self.current.parent(0) {
            Ok(c) => c,
            Err(e) => return Some(Err(e)),
        };

        self.current = commit;

        Some(Ok(self.current.clone()))
    }
}
pub struct Signature<'r> {
    signature: git2::Signature<'r>,
}

impl<'r> Signature<'r> {
    pub fn name(&self) -> Option<&str> {
        self.signature.name()
    }

    pub fn email(&self) -> Option<&str> {
        self.signature.email()
    }

    pub fn when(&self) -> Time {
        Time {
            time: self.signature.when(),
        }
    }
}

pub struct Time {
    time: git2::Time,
}

impl Time {
    pub fn seconds(&self) -> i64 {
        self.time.seconds()
    }

    pub fn offset_minutes(&self) -> i32 {
        self.time.offset_minutes()
    }

    pub fn sign(&self) -> char {
        self.time.sign()
    }
}

pub struct Tree<'r> {
    tree: git2::Tree<'r>,
}

impl<'r> Tree<'r> {
    pub fn iter(&self) -> TreeIter<'_> {
        self.tree.iter()
    }

    pub fn walk<C, T>(&self, mode: TreeWalkMode, callback: C) -> Result<()>
    where
        C: FnMut(&str, &TreeEntry<'_>) -> T,
        T: Into<i32>,
    {
        Ok(self.tree.walk(mode, callback)?)
    }

    pub fn try_walk<C, T, E>(&self, mode: TreeWalkMode, mut callback: C) -> Result<StdResult<(), E>>
    where
        C: FnMut(&str, &TreeEntry<'_>) -> StdResult<T, E>,
        T: Into<TreeWalkResult>,
    {
        let mut all_result = Ok(());
        self.walk(mode, |path, entry| match callback(path, entry) {
            Ok(r) => r.into(),
            Err(e) => {
                all_result = Err(e);
                TreeWalkResult::Abort
            }
        })?;

        Ok(all_result)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("git error: {0}")]
    Git(#[from] git2::Error),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("unknown object")]
    Unknown,
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn init_repo(
    config: &UpsilonVcsConfig,
    repo_config: RepoConfig,
    path: impl AsRef<Path>,
) -> Result<Repository> {
    init_repo_absolute(config, repo_config, config.repo_dir(path))
}

pub fn init_repo_absolute(
    config: &UpsilonVcsConfig,
    repo_config: RepoConfig,
    path: impl AsRef<Path>,
) -> Result<Repository> {
    if let GitProtocol::Enabled(_) = &config.git_protocol {
        if repo_config.visibility == RepoVisibility::Public {
            std::fs::write(path.as_ref().join("git-daemon-export-ok"), "")?;
        }
    }

    let repo = git2::Repository::init_bare(&path)?;

    if let GitHttpProtocol::Enabled(_) = &config.http_protocol {
        repo.config()?
            .open_level(ConfigLevel::Local)?
            .set_bool("http.receivepack", true)?;
    }

    Ok(Repository { repo })
}

pub fn get_repo(config: &UpsilonVcsConfig, path: impl AsRef<Path>) -> Result<Repository> {
    Ok(Repository {
        repo: git2::Repository::open_bare(config.repo_dir(path))?,
    })
}

pub struct RepoConfig {
    visibility: RepoVisibility,
}

impl RepoConfig {
    pub fn new(visibility: RepoVisibility) -> Self {
        Self { visibility }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RepoVisibility {
    Public,
    Private,
}
