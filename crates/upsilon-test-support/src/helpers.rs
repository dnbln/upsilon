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

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use git2::{BranchType, Repository};
use serde_json::json;

use crate::{env_var, gql_vars, IdHolder, TestCx, TestCxConfig, TestResult, Token, Username};

pub async fn register_dummy_user(cx: &mut TestCx) {
    cx.create_user("test", "test", "test")
        .await
        .expect("Failed to create user");
}

pub fn upsilon_basic_config(cfg: &mut TestCxConfig) {
    cfg.with_config(
        r#"
vcs:
  path: ./vcs/repos
  jailed: true
  git-protocol:
    enable: false
  http-protocol:
    enable: true
    push-auth-required: true

vcs-errors:
  leak-hidden-repos: true
  verbose: true

web:
  api:
    origin: "https://api.upsilon.dnbln.dev"
  web-interface:
    origin: "https://upsilon.dnbln.dev"
  docs:
    origin: "https://docs.upsilon.dnbln.dev"

debug:
  debug-data: false

data-backend:
  type: in-memory
  save: false

  cache:
    max-users: 1

users:
  register:
    enabled: true
  auth:
    password:
      type: argon2
    "#,
    );
}

pub fn upsilon_basic_config_with_git_daemon(cfg: &mut TestCxConfig) {
    cfg.with_config(
        r#"
vcs:
  path: ./vcs/repos
  jailed: true
  git-protocol:
    enable: true
    git-daemon:
      start: true
      services:
        receive-pack:
          enable: false
          override: allow
        upload-archive:
          enable: false
          override: allow
        upload-pack:
          enable: false
          override: allow
  http-protocol:
    enable: true
    push-auth-required: true

vcs-errors:
  leak-hidden-repos: true
  verbose: true

web:
  api:
    origin: "https://api.upsilon.dnbln.dev"
  web-interface:
    origin: "https://upsilon.dnbln.dev"
  docs:
    origin: "https://docs.upsilon.dnbln.dev"

debug:
  debug-data: false

data-backend:
  type: in-memory
  save: false

  cache:
    max-users: 1

users:
  register:
    enabled: true
  auth:
    password:
      type: argon2
    "#,
    )
    .with_git_daemon_port(portpicker::pick_unused_port().expect("Cannot find an unused port"));
}

pub async fn make_global_mirror_from_github(cx: &mut TestCx) -> TestResult<String> {
    cx.require_online().await;

    #[derive(serde::Deserialize)]
    struct GlobalMirror {
        #[serde(rename = "_debug__globalMirror")]
        global_mirror: IdHolder,
    }

    let result = cx
        .with_client(|cl| async move {
            cl.gql_query::<GlobalMirror>(
                r#"
mutation {
  _debug__globalMirror(
    name: "upsilon",
    url: "https://github.com/dnbln/upsilon"
  ) {
    id
  }
}
"#,
            )
            .await
        })
        .await?;

    Ok(result.global_mirror.id)
}

pub async fn make_global_mirror_from_local(cx: &mut TestCx) -> TestResult<String> {
    let upsilon_repo = upsilon_cloned_repo_path();
    if !upsilon_repo.exists() {
        panic!("Upsilon repository not found. Use `cargo xtask test` to run the tests.");
    }

    #[derive(serde::Deserialize)]
    struct CopyRepoFromLocalPath {
        #[serde(rename = "_debug__cpGlrFromLocal")]
        copy: IdHolder,
    }

    let id = cx
        .with_client(|cl| async move {
            cl.gql_query_with_variables::<CopyRepoFromLocalPath>(
                r#"
mutation($localPath: String!) {
  _debug__cpGlrFromLocal(name: "upsilon", localPath: $localPath) {
    id
  }
}
"#,
                gql_vars! {"localPath": upsilon_repo},
            )
            .await
        })
        .await?
        .copy
        .id;

    Ok(id)
}

pub async fn make_global_mirror_from_host_repo(cx: &mut TestCx) -> TestResult<String> {
    let upsilon_repo = upsilon_host_repo_git();
    if !upsilon_repo.exists() {
        panic!(
            "\
Upsilon repository .git folder not found. Use `cargo xtask test` to run the tests,
and make sure to do so in a valid git directory."
        );
    }

    #[derive(serde::Deserialize)]
    struct CopyRepoFromLocalPath {
        #[serde(rename = "_debug__cpGlrFromLocal")]
        copy: IdHolder,
    }

    let id = cx
        .with_client(|cl| async move {
            cl.gql_query_with_variables::<CopyRepoFromLocalPath>(
                r#"
mutation($localPath: String!) {
    _debug__cpGlrFromLocal(name: "upsilon", localPath: $localPath) {
        id
    }
}
"#,
                gql_vars! {"localPath": upsilon_repo},
            )
            .await
        })
        .await?
        .copy
        .id;

    Ok(id)
}

pub fn upsilon_cloned_repo_path() -> PathBuf {
    let setup_env = env_var("UPSILON_SETUP_TESTENV");

    let setup_env = PathBuf::from(setup_env);

    setup_env.join("repo/upsilon")
}

fn upsilon_host_repo_git() -> PathBuf {
    let host_repo_path = env_var("UPSILON_HOST_REPO_GIT");

    PathBuf::from(host_repo_path)
}

impl TestCx {
    async fn _clone_repo(&self, path: PathBuf, target_url: String) -> TestResult<Repository> {
        {
            let path = path.clone();
            tokio::task::spawn_blocking(move || {
                Repository::clone(&target_url, &path)?;

                Ok::<_, git2::Error>(())
            })
            .await??;
        }

        let repo = Repository::open(&path)?;

        Ok(repo)
    }

    pub async fn clone(&self, name: &str, remote_path: &str) -> TestResult<(PathBuf, Repository)> {
        let path = self.tempdir(name).await?;
        let target_url = format!("{}/{remote_path}", self.root);
        let repo = self._clone_repo(path.clone(), target_url).await?;

        Ok((path, repo))
    }

    pub async fn clone_over_git_protocol(
        &self,
        name: &str,
        remote_path: &str,
    ) -> TestResult<(PathBuf, Repository)> {
        let path = self.tempdir(name).await?;
        let target_url = format!("{}/{remote_path}", self.git_protocol_root);
        let repo = self._clone_repo(path.clone(), target_url).await?;

        Ok((path, repo))
    }

    pub async fn lookup(&self, path: &str) -> TestResult<String> {
        #[derive(serde::Deserialize)]
        struct LookupResult {
            #[serde(rename = "lookupRepo")]
            lookup_repo: IdHolder,
        }

        Ok(self
            .with_client(|cl| async move {
                cl.gql_query_with_variables::<LookupResult>(
                    r#"query($path: String!) {lookupRepo(path: $path) { id }}"#,
                    gql_vars! {"path": path},
                )
                .await
            })
            .await?
            .lookup_repo
            .id)
    }

    pub async fn create_user(
        &mut self,
        username: &str,
        password: &str,
        email: &str,
    ) -> TestResult<CreateUserResult> {
        #[derive(serde::Deserialize)]
        struct CreateUserToken {
            #[serde(rename = "_debug__createTestUser")]
            result: CreateUserResult,
        }

        let result = self
            .with_client(|cl| async move {
                cl.gql_query_with_variables::<CreateUserToken>(
                    r#"
mutation ($username: Username!, $password: PlainPassword!, $email: Email!) {
  _debug__createTestUser(username: $username, password: $password, email: $email)
}
"#,
                    gql_vars! {
                        "username": username,
                        "password": password,
                        "email": email,
                    },
                )
                .await
            })
            .await?
            .result;

        self.tokens
            .insert(Username(username.to_string()), Token(result.token.clone()));

        Ok(result)
    }
}

#[derive(serde::Deserialize)]
#[serde(transparent)]
pub struct CreateUserResult {
    pub token: String,
}

pub fn branch_commit<'repo>(
    repo: &'repo Repository,
    branch_name: &str,
) -> TestResult<git2::Commit<'repo>> {
    let br = repo.find_branch(branch_name, BranchType::Local)?;
    let commit = br.get().peel_to_commit()?;

    Ok(commit)
}

pub fn assert_same_trunk(repo_a: &Repository, repo_b: &Repository) -> TestResult<()> {
    let commit_a = branch_commit(repo_a, "trunk")?;
    let commit_b = branch_commit(repo_b, "trunk")?;

    assert_eq!(commit_a.id(), commit_b.id());

    Ok(())
}
