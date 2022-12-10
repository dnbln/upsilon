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

use clap::Parser;

use crate::ShaShaRefLines;

#[derive(Parser, Debug)]
pub struct PreReceive {
    #[clap(skip)]
    pub lines: ShaShaRefLines,
}

#[derive(Parser, Debug)]
pub struct Update {
    pub ref_name: String,
    pub old_oid: String,
    pub new_oid: String,
}

#[derive(Parser, Debug)]
pub struct PostReceive {
    #[clap(skip)]
    pub lines: ShaShaRefLines,
}

include!(concat!(env!("OUT_DIR"), "/app.rs"));
