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

use juniper::{graphql_object, FieldResult};
use upsilon_asyncvcs::refs::SignatureRef;
use upsilon_vcs::git2::{DiffLineType, ObjectType};
use upsilon_vcs::{DiffRepr, ReadmeKind};

use super::GraphQLContext;
use crate::graphql::UserRef;

pub struct RepoGit(pub(crate) upsilon_asyncvcs::Client);

#[graphql_object(context = GraphQLContext)]
impl RepoGit {
    async fn commit(&self, sha: String) -> FieldResult<GitCommit> {
        let commit = self
            .0
            .send(upsilon_asyncvcs::commit::CommitQuery(sha))
            .await
            .0?;

        Ok(GitCommit(self.0.clone(), commit))
    }

    async fn branch(&self, name: String) -> FieldResult<GitBranch> {
        let branch = self
            .0
            .send(upsilon_asyncvcs::branch::BranchQuery(name))
            .await
            .0?;

        Ok(GitBranch(self.0.clone(), branch))
    }

    async fn revspec(&self, revspec: String) -> FieldResult<GitRevspec> {
        let r = self
            .0
            .send(upsilon_asyncvcs::git_revspec::GitRevspecQuery(revspec))
            .await
            .0?;

        Ok(GitRevspec(self.0.clone(), r))
    }
}

pub struct GitRevspec(upsilon_asyncvcs::Client, upsilon_asyncvcs::refs::RevspecRef);

#[graphql_object(context = GraphQLContext)]
impl GitRevspec {
    async fn commit_from(&self) -> FieldResult<Option<GitCommit>> {
        let commit = self
            .0
            .send(upsilon_asyncvcs::git_revspec::GitRevspecFromCommitQuery(
                self.1,
            ))
            .await
            .0?;

        match commit {
            Some(commit) => Ok(Some(GitCommit(self.0.clone(), commit))),
            None => Ok(None),
        }
    }

    async fn commit_to(&self) -> FieldResult<Option<GitCommit>> {
        let commit = self
            .0
            .send(upsilon_asyncvcs::git_revspec::GitRevspecToCommitQuery(
                self.1,
            ))
            .await
            .0?;

        match commit {
            Some(commit) => Ok(Some(GitCommit(self.0.clone(), commit))),
            None => Ok(None),
        }
    }

    async fn diff(&self) -> FieldResult<Option<GitDiff>> {
        let diff = self
            .0
            .send(upsilon_asyncvcs::git_revspec::GitRevspecDiffQuery(self.1))
            .await
            .0?;

        match diff {
            Some(diff) => Ok(Some(GitDiff(self.0.clone(), diff))),
            None => Ok(None),
        }
    }
}

pub struct GitDiff(upsilon_asyncvcs::Client, DiffRepr);

#[graphql_object(context = GraphQLContext)]
impl GitDiff {
    fn stats(&self) -> GitDiffStats {
        GitDiffStats {
            files_changed: self.1.files_changed,
            insertions: self.1.insertions,
            deletions: self.1.deletions,
        }
    }

    fn files(&self) -> Vec<GitDiffFile> {
        self.1
            .files
            .iter()
            .map(|f| GitDiffFile(self.0.clone(), f.clone()))
            .collect()
    }
}

pub struct GitDiffFile(upsilon_asyncvcs::Client, upsilon_vcs::DiffFileRepr);

#[graphql_object(context = GraphQLContext)]
impl GitDiffFile {
    fn old_path(&self) -> &str {
        self.1.from_path.to_str().unwrap()
    }

    fn new_path(&self) -> &str {
        self.1.to_path.to_str().unwrap()
    }

    fn hunks(&self) -> Vec<GitDiffHunk> {
        self.1
            .hunks
            .iter()
            .map(|h| GitDiffHunk(self.0.clone(), h.clone()))
            .collect()
    }
}

pub struct GitDiffStats {
    files_changed: usize,
    insertions: usize,
    deletions: usize,
}

#[graphql_object(context = GraphQLContext)]
impl GitDiffStats {
    fn files_changed(&self) -> i32 {
        i32::try_from(self.files_changed).unwrap()
    }

    fn insertions(&self) -> i32 {
        i32::try_from(self.insertions).unwrap()
    }

    fn deletions(&self) -> i32 {
        i32::try_from(self.deletions).unwrap()
    }
}

pub struct GitDiffHunk(upsilon_asyncvcs::Client, upsilon_vcs::DiffHunkRepr);

#[graphql_object(context = GraphQLContext)]
impl GitDiffHunk {
    fn old_start(&self) -> i32 {
        i32::try_from(self.1.from_start).unwrap()
    }

    fn old_lines(&self) -> i32 {
        i32::try_from(self.1.from_lines).unwrap()
    }

    fn new_start(&self) -> i32 {
        i32::try_from(self.1.to_start).unwrap()
    }

    fn new_lines(&self) -> i32 {
        i32::try_from(self.1.to_lines).unwrap()
    }

    fn lines(&self) -> Vec<GitDiffLine> {
        self.1
            .lines
            .iter()
            .map(|l| GitDiffLine(self.0.clone(), l.clone()))
            .collect()
    }
}

pub struct GitDiffLine(upsilon_asyncvcs::Client, upsilon_vcs::DiffLineRepr);

#[graphql_object(context = GraphQLContext)]
impl GitDiffLine {
    fn old_lineno(&self) -> Option<i32> {
        self.1.old_line_no.map(|n| i32::try_from(n).unwrap())
    }

    fn new_lineno(&self) -> Option<i32> {
        self.1.new_line_no.map(|n| i32::try_from(n).unwrap())
    }

    fn content(&self) -> &str {
        self.1.line.as_str()
    }

    fn line_type(&self) -> String {
        match self.1.diff_type {
            DiffLineType::Context => " ",
            DiffLineType::Addition => "+",
            DiffLineType::Deletion => "-",
            DiffLineType::ContextEOFNL => " ",
            DiffLineType::AddEOFNL => ">",
            DiffLineType::DeleteEOFNL => "<",
            DiffLineType::FileHeader => " ",
            DiffLineType::HunkHeader => " ",
            DiffLineType::Binary => " ",
        }
        .to_owned()
    }
}

pub struct GitCommit(upsilon_asyncvcs::Client, upsilon_asyncvcs::refs::CommitRef);

#[graphql_object(context = GraphQLContext)]
impl GitCommit {
    async fn sha(&self) -> String {
        self.0
            .send(upsilon_asyncvcs::commit::CommitShaQuery(self.1))
            .await
            .0
    }

    async fn message(&self) -> Option<String> {
        self.0
            .send(upsilon_asyncvcs::commit::CommitMessageQuery(self.1))
            .await
            .0
    }

    async fn author(&self) -> FieldResult<GitSignature> {
        let author = self
            .0
            .send(upsilon_asyncvcs::commit::CommitAuthorQuery(self.1))
            .await
            .0;

        Ok(GitSignature(self.0.clone(), author))
    }

    async fn committer(&self) -> FieldResult<GitSignature> {
        let committer = self
            .0
            .send(upsilon_asyncvcs::commit::CommitCommitterQuery(self.1))
            .await
            .0;

        Ok(GitSignature(self.0.clone(), committer))
    }

    async fn parents(&self) -> Vec<GitCommit> {
        let parents = self
            .0
            .send(upsilon_asyncvcs::commit::CommitParentsQuery(self.1))
            .await
            .0;

        parents
            .into_iter()
            .map(|c| GitCommit(self.0.clone(), c))
            .collect()
    }

    #[graphql(arguments(i(default = 0)))]
    async fn parent(&self, i: i32) -> FieldResult<GitCommit> {
        let parent = self
            .0
            .send(upsilon_asyncvcs::commit::CommitParentQuery(
                self.1, i as usize,
            ))
            .await
            .0?;

        Ok(GitCommit(self.0.clone(), parent))
    }

    async fn tree(&self) -> FieldResult<GitTree> {
        let tree = self
            .0
            .send(upsilon_asyncvcs::commit::CommitTreeQuery(self.1))
            .await
            .0?;

        Ok(GitTree(self.0.clone(), tree))
    }

    async fn blob_string(&self, path: String) -> FieldResult<Option<String>> {
        let blob = self
            .0
            .send(upsilon_asyncvcs::commit::CommitBlobStringQuery(
                self.1,
                upsilon_asyncvcs::commit::BlobPath(path),
            ))
            .await
            .0?;

        Ok(blob)
    }

    async fn readme_blob(&self, dir_path: String) -> FieldResult<Option<GitReadmeBlob>> {
        let blob = self
            .0
            .send(upsilon_asyncvcs::commit::CommitReadmeBlobStringQuery(
                self.1, dir_path,
            ))
            .await
            .0?;

        Ok(blob.map(|(kind, path, content)| GitReadmeBlob(self.0.clone(), kind, path, content)))
    }
}

pub struct GitReadmeBlob(upsilon_asyncvcs::Client, ReadmeKind, String, String);

#[graphql_object(context = GraphQLContext)]
impl GitReadmeBlob {
    fn kind(&self) -> String {
        match self.1 {
            ReadmeKind::Markdown => "markdown",
            ReadmeKind::RST => "rst",
            ReadmeKind::Text => "text",
        }
        .to_owned()
    }

    fn path(&self) -> &str {
        &self.2
    }

    fn content(&self) -> &str {
        &self.3
    }
}

pub struct GitSignature(upsilon_asyncvcs::Client, SignatureRef);

impl GitSignature {
    async fn _name(&self) -> Option<String> {
        self.0
            .send(upsilon_asyncvcs::signature::SignatureNameQuery(self.1))
            .await
            .0
    }

    async fn _email(&self) -> Option<String> {
        self.0
            .send(upsilon_asyncvcs::signature::SignatureEmailQuery(self.1))
            .await
            .0
    }
}

#[graphql_object(context = GraphQLContext)]
impl GitSignature {
    async fn name(&self) -> Option<String> {
        self._name().await
    }

    async fn email(&self) -> Option<String> {
        self._email().await
    }

    async fn user(&self, context: &GraphQLContext) -> FieldResult<Option<UserRef>> {
        let email = match self._email().await {
            Some(email) => email,
            None => return Ok(None),
        };

        let user = context
            .query(|qm| async move { qm.query_user_by_username_email(&email).await })
            .await?;

        Ok(user.map(UserRef))
    }

    // fn time(&self) -> GitTime {
    //     GitTime(self.0.when())
    // }
}

pub struct GitTree(upsilon_asyncvcs::Client, upsilon_asyncvcs::refs::TreeRef);

#[graphql_object(context = GraphQLContext)]
impl GitTree {
    #[graphql(arguments(whole_tree(default = false)))]
    async fn entries(&self, whole_tree: bool) -> FieldResult<Vec<GitTreeEntry>> {
        let entries = match whole_tree {
            true => {
                self.0
                    .send(upsilon_asyncvcs::tree::WholeTreeEntriesQuery(self.1))
                    .await
                    .0?
            }
            false => {
                self.0
                    .send(upsilon_asyncvcs::tree::TreeEntriesQuery(self.1))
                    .await
                    .0
            }
        };

        Ok(entries
            .into_iter()
            .map(|(name, kind, entry)| GitTreeEntry(self.0.clone(), name, kind, entry))
            .collect())
    }
}

pub struct GitTreeEntry(
    upsilon_asyncvcs::Client,
    String,
    Option<ObjectType>,
    upsilon_asyncvcs::refs::TreeEntryRef,
);

#[graphql_object(context = GraphQLContext)]
impl GitTreeEntry {
    fn name(&self) -> String {
        self.1.clone()
    }

    fn kind(&self) -> Option<&'static str> {
        let Some(kind) = self.2 else {
            return None;
        };

        let kind_str = match kind {
            ObjectType::Any => "any",
            ObjectType::Commit => "commit",
            ObjectType::Tree => "tree",
            ObjectType::Blob => "blob",
            ObjectType::Tag => "tag",
        };

        Some(kind_str)
    }
}

// pub struct GitTime(upsilon_vcs::Time);
//
// #[graphql_object(context = GraphQLContext)]
// impl GitTime {
//     fn seconds(&self) -> String {
//         self.0.seconds().to_string()
//     }
//
//     fn sign(&self) -> String {
//         self.0.sign().to_string()
//     }
//
//     fn offset_minutes(&self) -> i32 {
//         self.0.offset_minutes()
//     }
// }
//
// pub struct GitTreeEntry<'tree>(Arc<upsilon_vcs::TreeEntry<'tree>>);
//
// #[graphql_object(context = GraphQLContext)]
// impl<'tree> GitTreeEntry<'tree> {
//     fn name(&self) -> Option<&str> {
//         self.0.name()
//     }
// }

pub struct GitBranch(upsilon_asyncvcs::Client, upsilon_asyncvcs::refs::BranchRef);

#[graphql_object(context = GraphQLContext)]
impl GitBranch {
    async fn name(&self) -> FieldResult<Option<String>> {
        Ok(self
            .0
            .send(upsilon_asyncvcs::branch::BranchNameQuery(self.1))
            .await
            .0?)
    }

    async fn commit(&self) -> FieldResult<GitCommit> {
        let commit = self
            .0
            .send(upsilon_asyncvcs::branch::BranchCommitQuery(self.1))
            .await
            .0?;

        Ok(GitCommit(self.0.clone(), commit))
    }

    #[graphql(name = "_debug__contributors")]
    async fn contributors(
        &self,
        context: &GraphQLContext,
    ) -> FieldResult<Vec<GitSignatureContributions>> {
        context.require_debug()?;

        let contributors = self
            .0
            .send(upsilon_asyncvcs::branch::BranchContributorsQuery(self.1))
            .await
            .0?
            .into_iter()
            .map(|(email, count)| GitSignatureContributions(self.0.clone(), email, count))
            .collect::<Vec<_>>();

        Ok(contributors)
    }
}

pub struct GitSignatureContributions(upsilon_asyncvcs::Client, String, usize);

impl GitSignatureContributions {
    fn _email(&self) -> &str {
        &self.1
    }
}

#[graphql_object(context = GraphQLContext)]
impl GitSignatureContributions {
    fn email(&self) -> &str {
        self._email()
    }

    async fn user(&self, context: &GraphQLContext) -> FieldResult<Option<UserRef>> {
        let email = self._email();

        let user = context
            .query(|qm| async move { qm.query_user_by_username_email(email).await })
            .await?;

        Ok(user.map(UserRef))
    }

    fn contributions(&self) -> i32 {
        i32::try_from(self.2).unwrap()
    }
}
