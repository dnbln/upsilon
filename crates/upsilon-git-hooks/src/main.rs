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

mod app;
mod sha_sha_ref;

use std::process::exit;

use clap::Parser;

use crate::app::{run_hook, App};

type GitHookResult<T> = anyhow::Result<T>;

fn main() -> GitHookResult<()> {
    let app = App::parse();

    run_hook(app)
}
