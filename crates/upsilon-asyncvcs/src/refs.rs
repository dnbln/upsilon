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

macro_rules! refty {
    ($name:ident) => {
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
        pub struct $name {
            pub(crate) id: usize,
        }
    };
}

refty!(BranchRef);
refty!(CommitRef);

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SignatureRef {
    pub(crate) commit_id: CommitRef,
    pub(crate) kind: SignatureKind,
}

#[derive(Copy, Clone, Debug, Ord, PartialOrd, Eq, PartialEq)]
pub(crate) enum SignatureKind {
    Author,
    Committer,
}

refty!(TreeRef);

#[derive(Clone, Debug)]
pub struct TreeEntryRef {
    pub(crate) tree_id: TreeRef,
    pub(crate) name: String,
}
