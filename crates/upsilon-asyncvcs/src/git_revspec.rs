/*
 *        Copyright (c) 2023 Dinu Blanovschi
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

use crate::message::{Message, Response};
use crate::private::{FromFlatResponse, ToFlatMessage};
use crate::refs::{CommitRef, RevspecRef};
use crate::{FlatMessage, FlatResponse};

pub struct GitRevspecQuery(pub String);

impl ToFlatMessage for GitRevspecQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::GitRevspec(self.0)
    }
}

impl Message for GitRevspecQuery {
    type Res = GitRevspecQueryResponse;
}

pub struct GitRevspecQueryResponse(pub upsilon_vcs::Result<RevspecRef>);

impl FromFlatResponse for GitRevspecQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::GitRevspec(c) => Self(Ok(c)),
            FlatResponse::Error(e) => Self(Err(e)),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for GitRevspecQueryResponse {}

pub struct GitRevspecFromCommitQuery(pub RevspecRef);

impl ToFlatMessage for GitRevspecFromCommitQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::GitRevspecFromCommit(self.0)
    }
}

impl Message for GitRevspecFromCommitQuery {
    type Res = GitRevspecCommitQueryResponse;
}

pub struct GitRevspecCommitQueryResponse(pub upsilon_vcs::Result<Option<CommitRef>>);

impl FromFlatResponse for GitRevspecCommitQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::Commit(c) => Self(Ok(Some(c))),
            FlatResponse::None => Self(Ok(None)),
            FlatResponse::Error(e) => Self(Err(e)),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for GitRevspecCommitQueryResponse {}

pub struct GitRevspecToCommitQuery(pub RevspecRef);

impl ToFlatMessage for GitRevspecToCommitQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::GitRevspecToCommit(self.0)
    }
}

impl Message for GitRevspecToCommitQuery {
    type Res = GitRevspecCommitQueryResponse;
}

pub struct GitRevspecDiffQuery(pub RevspecRef);

impl ToFlatMessage for GitRevspecDiffQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::GitRevspecDiff(self.0)
    }
}

impl Message for GitRevspecDiffQuery {
    type Res = GitRevspecDiffQueryResponse;
}

pub struct GitRevspecDiffQueryResponse(pub upsilon_vcs::Result<Option<upsilon_vcs::DiffRepr>>);

impl FromFlatResponse for GitRevspecDiffQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::Diff(c) => Self(Ok(Some(c))),
            FlatResponse::None => Self(Ok(None)),
            FlatResponse::Error(e) => Self(Err(e)),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for GitRevspecDiffQueryResponse {}
