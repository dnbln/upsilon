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

use std::path::Path;

use anyhow::bail;
use git2::{BranchType, Repository};

use crate::result::XtaskResult;

pub fn get_repo(path: &Path) -> XtaskResult<Repository> {
    let repo = Repository::discover(path)?;

    Ok(repo)
}

pub fn linear_history(repo: &Repository) -> XtaskResult<()> {
    let br = repo.find_branch("trunk", BranchType::Local)?;

    let mut commit = br.get().peel_to_commit()?;

    while commit.parent_count() != 0 {
        if commit.parent_count() > 1 {
            bail!(
                "\
trunk has merge commits: {}

Please rebase your branch on trunk.",
                commit.id()
            );
        }

        let parent = commit.parent(0)?;

        commit = parent;
    }

    Ok(())
}
