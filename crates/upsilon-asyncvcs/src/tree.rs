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
use crate::refs::{TreeEntryRef, TreeRef};
use crate::{FlatMessage, FlatResponse};

pub struct TreeEntriesQuery(pub TreeRef);

impl ToFlatMessage for TreeEntriesQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::TreeEntries(self.0)
    }
}

impl Message for TreeEntriesQuery {
    type Res = TreeEntriesQueryResponse;
}

pub struct TreeEntriesQueryResponse(pub Vec<(String, TreeEntryRef)>);

impl FromFlatResponse for TreeEntriesQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::TreeEntries(e) => Self(e),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for TreeEntriesQueryResponse {}

pub struct WholeTreeEntriesQuery(pub TreeRef);

impl ToFlatMessage for WholeTreeEntriesQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::WholeTreeEntries(self.0)
    }
}

impl Message for WholeTreeEntriesQuery {
    type Res = WholeTreeEntriesQueryResponse;
}

pub struct WholeTreeEntriesQueryResponse(pub upsilon_vcs::Result<Vec<(String, TreeEntryRef)>>);

impl FromFlatResponse for WholeTreeEntriesQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::TreeEntries(e) => Self(Ok(e)),
            FlatResponse::Error(e) => Self(Err(e)),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for WholeTreeEntriesQueryResponse {}
