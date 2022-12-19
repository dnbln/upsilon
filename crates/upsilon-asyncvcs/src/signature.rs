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

use crate::message::{Message, Response};
use crate::private::{FromFlatResponse, ToFlatMessage};
use crate::refs::SignatureRef;
use crate::{FlatMessage, FlatResponse};

pub struct SignatureNameQuery(pub SignatureRef);

impl ToFlatMessage for SignatureNameQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::SignatureName(self.0)
    }
}

impl Message for SignatureNameQuery {
    type Res = SignatureNameQueryResponse;
}

pub struct SignatureNameQueryResponse(pub Option<String>);

impl FromFlatResponse for SignatureNameQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::SignatureName(n) => Self(n),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for SignatureNameQueryResponse {}

pub struct SignatureEmailQuery(pub SignatureRef);

impl ToFlatMessage for SignatureEmailQuery {
    fn to_flat_message(self) -> FlatMessage {
        FlatMessage::SignatureEmail(self.0)
    }
}

impl Message for SignatureEmailQuery {
    type Res = SignatureEmailQueryResponse;
}

pub struct SignatureEmailQueryResponse(pub Option<String>);

impl FromFlatResponse for SignatureEmailQueryResponse {
    fn from_flat_response(flat_response: FlatResponse) -> Self {
        match flat_response {
            FlatResponse::SignatureEmail(e) => Self(e),
            _ => panic!("Invalid response type"),
        }
    }
}

impl Response for SignatureEmailQueryResponse {}
