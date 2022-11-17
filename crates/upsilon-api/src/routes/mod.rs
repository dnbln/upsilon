use rocket::State;
use std::fmt::Write;
use upsilon_vcs::{TreeWalkResult, UpsilonVcsConfig};

#[get("/")]
pub async fn get_api_root() -> &'static str {
    "Hello world"
}

#[post("/users")]
pub async fn create_user() {}

#[post("/repos/<repo>")]
pub async fn create_repo(repo: String, vcs_config: &State<UpsilonVcsConfig>) -> String {
    std::fs::create_dir_all(vcs_config.repo_dir(&repo)).expect("create dir");
    let r = upsilon_vcs::init_repo(vcs_config, &repo).expect("repo");

    repo
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
) -> String {
    let r = upsilon_vcs::get_repo(vcs_config, &repo).expect("repo");

    let br = r.find_branch(&branch).expect("branch");

    let cm = br.get_commit().expect("commit");

    cm.tree()
        .expect("tree")
        .walk(upsilon_vcs::TreeWalkMode::PreOrder, |name, entry| {
            println!("{name}{}", entry.name().expect("name"));
            let id = entry.id();
            println!("{id}");

            TreeWalkResult::Ok
        })
        .expect("walk");

    cm.message()
        .map_or_else(|| "aaa".to_string(), |message| message.to_string())
}

#[get("/repos/<repo>/branch/<branch>/history")]
pub async fn get_branch_history(
    repo: String,
    branch: String,
    vcs_config: &State<UpsilonVcsConfig>,
) -> String {
    let r = upsilon_vcs::get_repo(vcs_config, &repo).expect("repo");

    let br = r.find_branch(&branch).expect("branch");

    let cm = br.get_commit().expect("commit");

    let mut history = String::new();

    cm.self_and_all_ascendants()
        .try_for_each(|it| {
            let commit = it?;

            writeln!(history, "{}", commit.displayable_message()).unwrap();

            Ok::<_, upsilon_vcs::Error>(())
        })
        .unwrap();

    history
}

#[get("/repos/<repo>/commit/<commit>")]
pub async fn get_commit(
    repo: String,
    commit: String,
    vcs_config: &State<UpsilonVcsConfig>,
) -> String {
    let r = upsilon_vcs::get_repo(vcs_config, &repo).expect("repo");

    let cm = r.find_commit(&commit).expect("commit");

    cm.displayable_message().to_string()
}
