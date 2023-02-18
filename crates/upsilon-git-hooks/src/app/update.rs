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

use std::process::exit;

use clap::Parser;
use upsilon_git_hooks::repo_config::RepoConfig;
use upsilon_git_hooks::user_config::UserConfig;

use crate::app::GitHook;
use crate::GitHookResult;

#[derive(Parser, Debug)]
pub struct Update {
    pub ref_name: String,
    pub old_oid: String,
    pub new_oid: String,

    #[clap(skip = RepoConfig::from_env())]
    pub repo_config: RepoConfig,
    #[clap(skip = UserConfig::from_env())]
    pub user_config: UserConfig,
}

fn run_hook(hook: Update) -> GitHookResult<()> {
    let Update {
        ref_name,
        old_oid,
        new_oid,
        repo_config,
        user_config,
    } = hook;

    println!("update {ref_name} {old_oid} {new_oid}");
    dbg!(&repo_config);

    for rule in repo_config
        .protected_branches
        .iter()
        .filter(|it| format!("refs/heads/{}", it.name) == ref_name)
    {
        if rule.needs_admin && !user_config.permissions.has_admin {
            println!("update: protected branch, needs admin");
            exit(1);
        }
    }

    Ok(())
}

super::defer_impl_to!(Update => run_hook);
