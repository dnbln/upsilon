use crate::error::ApiResult;
use rocket::{tokio, State};
use std::fmt::Write;
use upsilon_vcs::{TreeWalkResult, UpsilonVcsConfig};

#[post("/repos/<repo>")]
pub async fn create_repo(repo: String, vcs_config: &State<UpsilonVcsConfig>) -> ApiResult<String> {
    tokio::fs::create_dir_all(vcs_config.repo_dir(&repo)).await?;
    let _ = upsilon_vcs::init_repo(vcs_config, &repo)?;

    Ok(repo)
}

#[get("/repos/<repo>")]
pub async fn get_repo(repo: String) -> String {
    repo
}

#[get("/repos/<repo>/branch/<branch>/top")]
pub async fn get_branch_top(
    repo: String,
    branch: String,
    vcs_config: &State<UpsilonVcsConfig>,
) -> ApiResult<String> {
    let r = upsilon_vcs::get_repo(vcs_config, &repo)?;
    let br = r.find_branch(&branch)?;
    let cm = br.get_commit()?;

    cm.tree()?
        .walk(upsilon_vcs::TreeWalkMode::PreOrder, |name, entry| {
            println!("{name}{}", entry.name().expect("Invalid UTF-8"));
            let id = entry.id();
            println!("{id}");

            TreeWalkResult::Ok
        })?;

    Ok(cm.displayable_message().to_string())
}

#[get("/repos/<repo>/branch/<branch>/history")]
pub async fn get_branch_history(
    repo: String,
    branch: String,
    vcs_config: &State<UpsilonVcsConfig>,
) -> ApiResult<String> {
    let r = upsilon_vcs::get_repo(vcs_config, &repo)?;
    let br = r.find_branch(&branch)?;
    let cm = br.get_commit()?;

    let mut history = String::new();

    cm.self_and_all_ascendants()
        .try_for_each(|it| {
            let commit = it?;

            writeln!(history, "{}", commit.displayable_message()).unwrap();

            Ok::<_, upsilon_vcs::Error>(())
        })?;

    Ok(history)
}

#[get("/repos/<repo>/commit/<commit>")]
pub async fn get_commit(
    repo: String,
    commit: String,
    vcs_config: &State<UpsilonVcsConfig>,
) -> ApiResult<String> {
    let r = upsilon_vcs::get_repo(vcs_config, &repo)?;
    let cm = r.find_commit(&commit)?;

    Ok(cm.displayable_message().to_string())
}
