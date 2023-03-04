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

#![feature(drain_filter)]

pub extern crate git2;
pub extern crate upsilon_git_hooks;

mod config;
mod daemon;
mod http_backend;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::result::Result as StdResult;

pub use git2::{BranchType, TreeWalkMode, TreeWalkResult};
use git2::{ConfigLevel, DiffDelta, DiffHunk, DiffLine, DiffLineType, ErrorCode, Oid};
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
    path: PathBuf,
}

impl AsRef<Path> for Repository {
    fn as_ref(&self) -> &Path {
        self.path.as_ref()
    }
}

impl Repository {
    pub fn is_bare(&self) -> bool {
        self.repo.is_bare()
    }

    pub fn find_commit(&self, commit: &str) -> Result<Commit> {
        self.find_commit_oid(commit.parse()?)
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

    pub fn merge_base_many(&self, oids: &[Oid]) -> Result<Oid> {
        Ok(self.repo.merge_base_many(oids)?)
    }

    pub fn find_commit_oid(&self, oid: Oid) -> Result<Commit> {
        Ok(Commit {
            commit: self.repo.find_commit(oid)?,
        })
    }

    pub fn find_reference(&self, name: &str) -> Result<Reference> {
        Ok(Reference {
            reference: self.repo.find_reference(name)?,
        })
    }

    pub fn parse_revspec(&self, revspec: &str) -> Result<Revspec> {
        Ok(Revspec {
            revision: self.repo.revparse(revspec)?,
        })
    }
}

pub struct Revspec<'r> {
    revision: git2::Revspec<'r>,
}

impl<'r> Revspec<'r> {
    pub fn from<'revspec>(&'revspec self) -> Option<ObjectRef<'revspec, 'r>> {
        self.revision.from().map(|o| ObjectRef { object: o })
    }

    pub fn to<'revspec>(&'revspec self) -> Option<ObjectRef<'revspec, 'r>> {
        self.revision.to().map(|o| ObjectRef { object: o })
    }
}

pub struct ObjectRef<'ref_lifetime, 'r> {
    object: &'ref_lifetime git2::Object<'r>,
}

impl<'ref_lifetime, 'r> ObjectRef<'ref_lifetime, 'r> {
    pub fn peel_to_commit(&self) -> Result<Commit<'r>> {
        Ok(Commit {
            commit: self.object.peel_to_commit()?,
        })
    }
}

pub struct Reference<'r> {
    reference: git2::Reference<'r>,
}

impl<'r> Reference<'r> {
    pub fn peel_to_commit(&self) -> Result<Commit<'r>> {
        Ok(Commit {
            commit: self.reference.peel_to_commit()?,
        })
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

    pub fn get_commit(&self) -> Result<Commit<'r>> {
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
    pub fn sha(&self) -> String {
        self.commit.id().to_string()
    }

    pub fn oid(&self) -> Oid {
        self.commit.id()
    }

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

    pub fn only_parent_is(&self, parent_oid: Oid) -> Result<bool> {
        Ok(self.parent_count() == 1 && self.parent(0)?.oid() == parent_oid)
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

    pub fn blob(&self, repo: &'r Repository, path: &str) -> Result<Option<Blob>> {
        let entry = self.commit.tree()?.get_path(Path::new(path))?;
        let obj = entry.to_object(&repo.repo)?;

        match obj.into_blob() {
            Ok(blob) => Ok(Some(Blob { blob })),
            Err(_) => Ok(None),
        }
    }

    pub fn readme_blob(&self, repo: &'r Repository, dir: &str) -> Result<Option<ReadmeBlob>> {
        let t = self.commit.tree()?;

        let join_root = if dir.is_empty() {
            "".to_owned()
        } else if dir.ends_with('/') {
            dir.to_owned()
        } else {
            format!("{dir}/")
        };

        for readme_kind in ReadmeKind::variants() {
            for ext in readme_kind.extensions() {
                let readme_path = format!("{join_root}README{ext}");
                let r = t.get_path(Path::new(&readme_path));
                let entry = match r {
                    Ok(entry) => entry,
                    Err(e) => {
                        if e.code() == ErrorCode::NotFound {
                            continue;
                        } else {
                            return Err(e.into());
                        }
                    }
                };
                let blob = entry.to_object(&repo.repo)?.into_blob().unwrap();
                {
                    return Ok(Some(ReadmeBlob {
                        blob: Blob { blob },
                        path: readme_path,
                        readme_kind: *readme_kind,
                    }));
                }
            }
        }

        Ok(None)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ReadmeKind {
    Markdown,
    Text,
    RST,
}

impl ReadmeKind {
    pub fn variants() -> &'static [ReadmeKind] {
        &[ReadmeKind::Markdown, ReadmeKind::Text, ReadmeKind::RST]
    }

    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            ReadmeKind::Markdown => &[
                ".md",
                ".mkd",
                ".mdwn",
                ".mdown",
                ".mdtxt",
                ".mdtext",
                ".markdown",
            ],
            ReadmeKind::Text => &[".txt", ""],
            ReadmeKind::RST => &[".rst"],
        }
    }
}

pub struct ReadmeBlob<'r> {
    pub readme_kind: ReadmeKind,
    pub path: String,
    pub blob: Blob<'r>,
}

pub struct Blob<'r> {
    blob: git2::Blob<'r>,
}

impl<'r> Blob<'r> {
    pub fn to_string(&self) -> Result<String> {
        Ok(String::from_utf8(self.to_bytes())?)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.as_bytes().to_vec()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.blob.content()
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

pub struct TreeEntry<'tree> {
    entry: git2::TreeEntry<'tree>,
}

pub struct TreeEntryRef<'tree, 'r> {
    entry: &'r git2::TreeEntry<'tree>,
}

impl<'tree, 'r> TreeEntryRef<'tree, 'r> {
    pub fn name(&self) -> &str {
        self.entry.name().unwrap_or("<invalid UTF-8>")
    }

    pub fn kind(&self) -> Option<git2::ObjectType> {
        self.entry.kind()
    }
}

impl<'tree> TreeEntry<'tree> {
    pub fn name(&self) -> &str {
        self.entry.name().unwrap_or("<invalid UTF-8>")
    }

    pub fn kind(&self) -> Option<git2::ObjectType> {
        self.entry.kind()
    }
}

pub struct TreeIter<'tree> {
    inner: git2::TreeIter<'tree>,
}

impl<'tree> Iterator for TreeIter<'tree> {
    type Item = TreeEntry<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|entry| TreeEntry { entry })
    }
}

impl<'tree> DoubleEndedIterator for TreeIter<'tree> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.inner.next_back().map(|entry| TreeEntry { entry })
    }
}

impl<'r> Tree<'r> {
    pub fn iter(&self) -> TreeIter<'_> {
        TreeIter {
            inner: self.tree.iter(),
        }
    }

    pub fn walk<C, T>(&self, mode: TreeWalkMode, mut callback: C) -> Result<()>
    where
        C: FnMut(&str, TreeEntryRef<'_, '_>) -> T,
        T: Into<i32>,
    {
        Ok(self
            .tree
            .walk(mode, |name, entry| callback(name, TreeEntryRef { entry }))?)
    }

    pub fn try_walk<C, T, E>(&self, mode: TreeWalkMode, mut callback: C) -> Result<StdResult<(), E>>
    where
        C: FnMut(&str, TreeEntryRef<'_, '_>) -> StdResult<T, E>,
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

    #[error("from utf8: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("unknown object")]
    Unknown,

    #[error("no such repo")]
    NoSuchRepo,
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
    let repo = git2::Repository::init_bare(&path)?;

    let path = path.as_ref().to_path_buf();
    repo_setup(config, &path, &repo, &repo_config)?;

    Ok(Repository { repo, path })
}

fn check_repo_exists_absolute(_config: &UpsilonVcsConfig, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(Error::NoSuchRepo);
    }

    if !path.join(REPO_ID_FILE).exists() {
        return Err(Error::NoSuchRepo);
    }

    Ok(())
}

const REPO_ID_FILE: &str = "upsilon-repoid";

pub fn silent_setup_repo_absolute(
    config: &UpsilonVcsConfig,
    path: impl AsRef<Path>,
    repo: &Repository,
    repo_config: &RepoConfig,
) -> Result<()> {
    repo_setup(config, path, &repo.repo, repo_config)?;

    Ok(())
}

fn repo_setup(
    config: &UpsilonVcsConfig,
    path: impl AsRef<Path>,
    repo: &git2::Repository,
    repo_config: &RepoConfig,
) -> Result<()> {
    let daemon_export = path.as_ref().join("git-daemon-export-ok");
    let mut need_daemon_export = false;

    let mut repo_local_config = repo.config()?.open_level(ConfigLevel::Local)?;

    if let GitProtocol::Enabled(_) = &config.git_protocol {
        need_daemon_export = true;

        if repo_config.visibility == RepoVisibility::Private {
            // disable git:// daemon services if private:
            repo_local_config.set_bool("daemon.uploadpack", false)?;
        } else {
            repo_local_config.set_bool("daemon.uploadpack", true)?;
        }
        // uploadarchive and receivepack is always disabled on git://
        repo_local_config.set_bool("daemon.uploadarchive", false)?;
        repo_local_config.set_bool("daemon.receivepack", false)?;
    }

    if let GitHttpProtocol::Enabled(_) = &config.http_protocol {
        need_daemon_export = true;

        // everything enabled on http://, middleware will handle auth.
        repo_local_config.set_bool("http.uploadpack", true)?;
        repo_local_config.set_bool("http.uploadarchive", true)?;
        repo_local_config.set_bool("http.receivepack", true)?;
    }

    if need_daemon_export {
        std::fs::write(daemon_export, "")?;
    }

    setup_hooks(path.as_ref())?;

    std::fs::write(path.as_ref().join(REPO_ID_FILE), &repo_config.id)?;

    Ok(())
}

fn setup_hooks(repo: impl AsRef<Path>) -> Result<()> {
    let hook_exe_path = upsilon_core::alt_exe("upsilon-git-hooks");
    let repo = repo.as_ref();

    let hooks_path = repo.join("hooks");

    for hook in upsilon_git_hooks::HOOKS_TO_REGISTER {
        let hook_path = hooks_path.join(hook);

        setup_hook(&hook_exe_path, &hook_path, hook)?;
    }

    Ok(())
}

fn setup_hook(hooks_exe: &Path, hook_path: &Path, hook_name: &str) -> Result<()> {
    std::fs::write(
        hook_path,
        format!(
            r#"#!/bin/bash

{hooks_exe:?} {hook_name} $@
"#,
        ),
    )?;

    Ok(())
}

pub async fn read_repo_id_absolute(
    _config: &UpsilonVcsConfig,
    repo_path: impl AsRef<Path>,
) -> Result<String> {
    if !repo_path.as_ref().exists() {
        return Err(Error::NoSuchRepo);
    }

    let repo_id_file = repo_path.as_ref().join(REPO_ID_FILE);

    if !repo_id_file.exists() {
        return Err(Error::NoSuchRepo);
    }

    Ok(tokio::fs::read_to_string(repo_id_file).await?)
}

pub async fn read_repo_id(
    config: &UpsilonVcsConfig,
    repo_path: impl AsRef<Path>,
) -> Result<String> {
    read_repo_id_absolute(config, config.repo_dir(repo_path)).await
}

pub fn setup_mirror(
    config: &UpsilonVcsConfig,
    url: impl AsRef<str>,
    repo_config: &RepoConfig,
    path: impl AsRef<Path>,
) -> Result<Repository> {
    setup_mirror_absolute(config, url, repo_config, config.repo_dir(path))
}

pub fn setup_mirror_absolute(
    config: &UpsilonVcsConfig,
    mirror_url: impl AsRef<str>,
    repo_config: &RepoConfig,
    path: impl AsRef<Path>,
) -> Result<Repository> {
    let mirror_url_clone = mirror_url.as_ref().to_owned();
    let path_clone = path.as_ref().to_path_buf();

    let repo = git2::build::RepoBuilder::new()
        .bare(true)
        .clone(&mirror_url_clone, &path_clone)?;

    let path = path.as_ref().to_path_buf();

    repo_setup(config, &path, &repo, repo_config)?;

    Ok(Repository { repo, path })
}

pub fn get_repo(config: &UpsilonVcsConfig, path: impl AsRef<Path>) -> Result<Repository> {
    let repo_dir = config.repo_dir(path);

    get_repo_absolute(config, repo_dir)
}

pub fn get_repo_absolute(config: &UpsilonVcsConfig, path: impl AsRef<Path>) -> Result<Repository> {
    let path = path.as_ref();

    check_repo_exists_absolute(config, path)?;

    get_repo_absolute_no_check(config, path)
}

pub fn get_repo_absolute_no_check(
    _config: &UpsilonVcsConfig,
    path: impl AsRef<Path>,
) -> Result<Repository> {
    let path = path.as_ref().to_path_buf();

    Ok(Repository {
        repo: git2::Repository::open_bare(&path)?,
        path,
    })
}

pub fn exists_global(config: &UpsilonVcsConfig, path: impl AsRef<Path>) -> bool {
    let path = config.repo_dir(path);

    path.exists()
}

pub struct RepoConfig {
    visibility: RepoVisibility,
    id: String,
}

impl RepoConfig {
    pub fn new(visibility: RepoVisibility, id: impl Into<String>) -> Self {
        Self {
            visibility,
            id: id.into(),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RepoVisibility {
    Public,
    Private,
}

impl<'r> Revspec<'r> {
    pub fn diff(&self, repo: &'r Repository) -> Result<Option<DiffRepr>> {
        let (Some(from), Some(to)) = (self.from(), self.to()) else {
            return Ok(None);
        };

        let from_commit = from.peel_to_commit()?;
        let to_commit = to.peel_to_commit()?;

        let mut diff_opts = git2::DiffOptions::new();

        diff_opts.context_lines(30);

        let diff = repo.repo.diff_tree_to_tree(
            Some(&from_commit.tree()?.tree),
            Some(&to_commit.tree()?.tree),
            Some(&mut diff_opts),
        )?;

        let stats = diff.stats()?;

        let files_changed = stats.files_changed();
        let insertions = stats.insertions();
        let deletions = stats.deletions();

        let diff_repr = Rc::new(RefCell::new(DiffRepr {
            files_changed,
            insertions,
            deletions,
            files: Vec::new(),
        }));

        {
            let mut file_cb = {
                let diff_repr = Rc::clone(&diff_repr);

                move |delta: DiffDelta, _progress: f32| {
                    let from_path = delta.old_file().path().unwrap().to_path_buf();
                    let to_path = delta.new_file().path().unwrap().to_path_buf();

                    let mut diff_file = DiffFileRepr {
                        from_path,
                        to_path,
                        hunks: Vec::new(),
                    };

                    diff_repr.borrow_mut().files.push(diff_file);

                    true
                }
            };

            let mut hunk_cb = {
                let diff_repr = Rc::clone(&diff_repr);

                move |delta: DiffDelta, hunk: DiffHunk| {
                    let from_start = hunk.old_start();
                    let from_lines = hunk.old_lines();
                    let to_start = hunk.new_start();
                    let to_lines = hunk.new_lines();

                    let diff_hunk = DiffHunkRepr {
                        from_start: from_start as usize,
                        from_lines: from_lines as usize,
                        to_start: to_start as usize,
                        to_lines: to_lines as usize,
                        lines: Vec::new(),
                    };

                    let mut dr = diff_repr.borrow_mut();

                    let Some(f) = dr.files.last_mut() else {
                        return false;
                    };

                    f.hunks.push(diff_hunk);

                    true
                }
            };

            let mut line_cb = {
                let diff_repr = Rc::clone(&diff_repr);

                move |delta: DiffDelta, hunk: Option<DiffHunk>, line: DiffLine| {
                    let l = std::str::from_utf8(line.content()).unwrap().to_owned();

                    let old_line_no = line.old_lineno();
                    let new_line_no = line.new_lineno();

                    let line = DiffLineRepr {
                        line: l,
                        old_line_no: old_line_no.map(|it| it as usize),
                        new_line_no: new_line_no.map(|it| it as usize),
                        diff_type: line.origin_value(),
                    };

                    let mut dr = diff_repr.borrow_mut();

                    let Some(f) = dr.files.last_mut() else {
                        return false;
                    };

                    debug_assert_eq!(f.from_path, delta.old_file().path().unwrap());

                    let Some(h) = f.hunks.last_mut() else {
                        return false;
                    };

                    debug_assert_eq!(h.from_start, hunk.unwrap().old_start() as usize);

                    h.lines.push(line);

                    true
                }
            };

            diff.foreach(&mut file_cb, None, Some(&mut hunk_cb), Some(&mut line_cb))?;
        }

        let diff_repr = Rc::try_unwrap(diff_repr).unwrap().into_inner();

        Ok(Some(diff_repr))
    }
}

#[derive(Clone, Debug)]
pub struct DiffRepr {
    pub files_changed: usize,
    pub insertions: usize,
    pub deletions: usize,
    pub files: Vec<DiffFileRepr>,
}

#[derive(Clone, Debug)]
pub struct DiffFileRepr {
    pub from_path: PathBuf,
    pub to_path: PathBuf,

    pub hunks: Vec<DiffHunkRepr>,
}

#[derive(Clone, Debug)]
pub struct DiffHunkRepr {
    pub from_start: usize,
    pub from_lines: usize,
    pub to_start: usize,
    pub to_lines: usize,

    pub lines: Vec<DiffLineRepr>,
}

#[derive(Clone, Debug)]
pub struct DiffLineRepr {
    pub line: String,
    pub old_line_no: Option<usize>,
    pub new_line_no: Option<usize>,
    pub diff_type: DiffLineType,
}
