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

    // async fn branch(&self, name: String) -> FieldResult<RepoBranch> {
    //     let branch = self
    //         .0
    //         .send(upsilon_asyncvcs::branch::BranchQuery(name))
    //         .await
    //         .0?;
    //
    //     Ok(RepoBranch(branch))
    // }
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

    // fn author(&self) -> FieldResult<GitSignature> {
    //     let sig = Arc::new(self.0.author());
    //
    //     Ok(GitSignature(sig))
    // }
    //
    // fn committer(&self) -> FieldResult<GitSignature> {
    //     let sig = Arc::new(self.0.committer());
    //
    //     Ok(GitSignature(sig))
    // }
    //
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

// pub struct GitSignature<'r>(Arc<upsilon_vcs::Signature<'r>>);
//
// #[graphql_object(context = GraphQLContext)]
// impl<'r> GitSignature<'r> {
//     fn name(&self) -> Option<&str> {
//         self.0.name()
//     }
//
//     fn email(&self) -> Option<&str> {
//         self.0.email()
//     }
//
//     fn time(&self) -> GitTime {
//         GitTime(self.0.when())
//     }
// }
//
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
