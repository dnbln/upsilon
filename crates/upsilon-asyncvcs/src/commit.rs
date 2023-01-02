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

use crate::message::{Message, Response};
use crate::private::{FromFlatResponse, ToFlatMessage};
use crate::refs::{CommitRef, SignatureRef, TreeRef};
use crate::{FlatMessage, FlatResponse};

pub struct CommitQuery(pub String);

impl ToFlatMessage for CommitQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::Commit(self.0)
    }
}

impl Message for CommitQuery {
    type Res = CommitQueryResponse;
}

pub struct CommitQueryResponse(pub upsilon_vcs::Result<CommitRef>);

impl FromFlatResponse for CommitQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::Commit(c) => Self(Ok(c)),
            FlatResponse::Error(e) => Self(Err(e)),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for CommitQueryResponse {}

pub struct CommitShaQuery(pub CommitRef);

impl ToFlatMessage for CommitShaQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::CommitSha(self.0)
    }
}

impl Message for CommitShaQuery {
    type Res = CommitShaQueryResponse;
}

pub struct CommitShaQueryResponse(pub String);

impl FromFlatResponse for CommitShaQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::CommitSha(s) => Self(s),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for CommitShaQueryResponse {}

pub struct CommitMessageQuery(pub CommitRef);

impl ToFlatMessage for CommitMessageQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::CommitMessage(self.0)
    }
}

impl Message for CommitMessageQuery {
    type Res = CommitMessageQueryResponse;
}

pub struct CommitMessageQueryResponse(pub Option<String>);

impl FromFlatResponse for CommitMessageQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::CommitMessage(m) => Self(m),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for CommitMessageQueryResponse {}

pub struct CommitAuthorQuery(pub CommitRef);

impl ToFlatMessage for CommitAuthorQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::CommitAuthor(self.0)
    }
}

impl Message for CommitAuthorQuery {
    type Res = CommitAuthorQueryResponse;
}

pub struct CommitAuthorQueryResponse(pub SignatureRef);

impl FromFlatResponse for CommitAuthorQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::CommitAuthor(a) => Self(a),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for CommitAuthorQueryResponse {}

pub struct CommitCommitterQuery(pub CommitRef);

impl ToFlatMessage for CommitCommitterQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::CommitCommitter(self.0)
    }
}

impl Message for CommitCommitterQuery {
    type Res = CommitCommitterQueryResponse;
}

pub struct CommitCommitterQueryResponse(pub SignatureRef);

impl FromFlatResponse for CommitCommitterQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::CommitCommitter(c) => Self(c),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for CommitCommitterQueryResponse {}

pub struct CommitTreeQuery(pub CommitRef);

impl ToFlatMessage for CommitTreeQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::CommitTree(self.0)
    }
}

impl Message for CommitTreeQuery {
    type Res = CommitTreeQueryResponse;
}

pub struct CommitTreeQueryResponse(pub upsilon_vcs::Result<TreeRef>);

impl FromFlatResponse for CommitTreeQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::CommitTree(t) => Self(Ok(t)),
            FlatResponse::Error(e) => Self(Err(e)),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for CommitTreeQueryResponse {}

pub struct CommitParentQuery(pub CommitRef, pub usize);

impl ToFlatMessage for CommitParentQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::CommitParent(self.0, self.1)
    }
}

impl Message for CommitParentQuery {
    type Res = CommitParentResponse;
}

pub struct CommitParentResponse(pub upsilon_vcs::Result<CommitRef>);

impl FromFlatResponse for CommitParentResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::Commit(c) => Self(Ok(c)),
            FlatResponse::Error(e) => Self(Err(e)),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for CommitParentResponse {}
