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
use upsilon_git_hooks::repo_config::RepoConfig;

use crate::app::GitHook;
use crate::sha_sha_ref::ShaShaRefLines;
use crate::GitHookResult;

#[derive(Parser, Debug)]
pub struct PreReceive {
    #[clap(skip = ShaShaRefLines::from_stdin())]
    pub lines: ShaShaRefLines,

    #[clap(skip = RepoConfig::from_env())]
    pub repo_config: RepoConfig,
}

fn run_hook(hook: PreReceive) -> GitHookResult<()> {
    let PreReceive { lines, repo_config } = hook;

    println!("pre-receive");
    dbg!(&repo_config);

    for line in lines.iter() {
        println!(
            "pre-receive: {} {} {}",
            line.old_sha, line.new_sha, line.ref_name
        );
    }

    Ok(())
}

super::defer_impl_to!(PreReceive => run_hook);
