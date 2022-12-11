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
use crate::refs::BranchRef;
use crate::{FlatMessage, FlatResponse};

pub struct BranchQuery(pub String);

impl ToFlatMessage for BranchQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::Branch(self.0)
    }
}

impl Message for BranchQuery {
    type Res = BranchQueryResponse;
}

pub struct BranchQueryResponse(pub upsilon_vcs::Result<BranchRef>);

impl FromFlatResponse for BranchQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::Branch(b) => Self(Ok(b)),
            FlatResponse::Error(e) => Self(Err(e)),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for BranchQueryResponse {}
