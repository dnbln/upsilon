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

use crate::private::{FromFlatResponse, ToFlatMessage};
use crate::message::{Message, Response};
use crate::refs::CommitRef;
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
