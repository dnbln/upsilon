use anyhow::bail;
use git2::{BranchType, Repository};

use crate::result::XtaskResult;

pub fn get_repo() -> XtaskResult<Repository> {
    let repo = Repository::discover(std::env::current_dir()?)?;

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
