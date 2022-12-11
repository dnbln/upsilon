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

use std::future::Future;
use std::sync::Arc;

use juniper::{graphql_object, FieldResult};

use super::GraphQLContext;

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

    async fn branch(&self, name: String) -> FieldResult<RepoBranch> {
        let branch = self
            .0
            .send(upsilon_asyncvcs::branch::BranchQuery(name))
            .await
            .0?;

        Ok(RepoBranch(self.0.clone(), branch))
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

    // fn parents(&self) -> Vec<GitCommit<'r>> {
    //     let parents = self.0.parents().map(|p| GitCommit(Arc::new(p))).collect();
    //
    //     parents
    // }
    //
    // fn tree(&self) -> FieldResult<GitTree<'r>> {
    //     Ok(GitTree(Arc::new(self.0.tree()?)))
    // }
}

pub struct GitSignature(
    upsilon_asyncvcs::Client,
    upsilon_asyncvcs::refs::SignatureRef,
);

#[graphql_object(context = GraphQLContext)]
impl GitSignature {
    async fn name(&self) -> Option<String> {
        self.0
            .send(upsilon_asyncvcs::signature::SignatureNameQuery(self.1))
            .await
            .0
    }

    async fn email(&self) -> Option<String> {
        self.0
            .send(upsilon_asyncvcs::signature::SignatureEmailQuery(self.1))
            .await
            .0
    }

    // fn time(&self) -> GitTime {
    //     GitTime(self.0.when())
    // }
}

// pub struct GitTree<'r>(Arc<upsilon_vcs::Tree<'r>>);
//
// #[graphql_object(context = GraphQLContext)]
// impl<'r> GitTree<'r> {
//     fn entries(&self) -> Vec<GitTreeEntry<'_>> {
//         let entries = self.0.iter().map(|e| GitTreeEntry(Arc::new(e))).collect();
//
//         entries
//     }
// }
//
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

pub struct RepoBranch(upsilon_asyncvcs::Client, upsilon_asyncvcs::refs::BranchRef);

#[graphql_object(context = GraphQLContext)]
impl RepoBranch {
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
}
