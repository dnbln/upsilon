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
use std::path::PathBuf;

use git2::Repository;
use serde_json::json;

use crate::{IdHolder, TestCx, TestCxConfig, TestResult, Token, Username};

pub async fn register_dummy_user(cx: &mut TestCx) {
    cx
        .create_user("test", "test", "test")
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

pub async fn make_global_mirror_from_github(cx: &mut TestCx) -> TestResult<String> {
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
                HashMap::from([("localPath".to_string(), json!(upsilon_repo))]),
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
                HashMap::from([("localPath".to_string(), json!(upsilon_repo))]),
            )
            .await
        })
        .await?
        .copy
        .id;

    Ok(id)
}

pub fn upsilon_cloned_repo_path() -> PathBuf {
    let setup_env = std::env::var("UPSILON_SETUP_TESTENV")
        .expect("UPSILON_SETUP_TESTENV not set; did you use `cargo xtask test` to run the tests?");

    let setup_env = PathBuf::from(setup_env);

    setup_env.join("repo/upsilon")
}

fn upsilon_host_repo_git() -> PathBuf {
    let host_repo_path = std::env::var("UPSILON_HOST_REPO_GIT")
        .expect("UPSILON_HOST_REPO_GIT not set; did you use `cargo xtask test` to run the tests?");

    PathBuf::from(host_repo_path)
}

impl TestCx {
    pub async fn clone(&self, name: &str, remote_path: &str) -> TestResult<(PathBuf, Repository)> {
        let path = self.tempdir(name).await?;
        let target_path = format!("{}/{remote_path}", self.root);
        let repo = Repository::clone(&target_path, &path)?;

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
                    HashMap::from([("path".to_string(), serde_json::json!(path))]),
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
                    HashMap::from([
                        ("username".to_string(), json!(username)),
                        ("password".to_string(), json!(password)),
                        ("email".to_string(), json!(email)),
                    ]),
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
