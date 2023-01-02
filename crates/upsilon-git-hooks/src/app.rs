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

use clap::Parser;
use upsilon_git_hooks::repo_config::RepoConfig;

use crate::sha_sha_ref::ShaShaRefLines;
use crate::GitHookResult;

mod post_receive;
mod pre_receive;
mod update;

pub use post_receive::PostReceive;
pub use pre_receive::PreReceive;
pub use update::Update;

trait GitHook {
    fn run(self) -> GitHookResult<()>;
}

macro_rules! defer_impl_to {
    ($name:ident => $to:ident) => {
        impl GitHook for $name {
            fn run(self) -> GitHookResult<()> {
                $to(self)
            }
        }
    };
}

use defer_impl_to;

include!(concat!(env!("OUT_DIR"), "/app.rs"));
