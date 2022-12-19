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

use crate::{IdHolder, TestCx, TestCxConfig, TestResult};

pub async fn register_dummy_user(cx: &mut TestCx) {
    #[derive(serde::Deserialize)]
    struct CreateUserResponse {
        #[serde(rename = "createUser")]
        token: String,
    }

    let response = cx
        .with_client(|c| async move {
            c.gql_query::<CreateUserResponse>(
                r#"
            mutation {
                createUser(username: "test", password: "test", email: "test")
            }
            "#,
            )
            .await
        })
        .await
        .expect("Failed to create user");

    cx.client.set_token(response.token);
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

pub async fn make_global_mirror(cx: &mut TestCx) -> TestResult<String> {
    #[derive(serde::Deserialize)]
    struct GlobalMirror {
        #[serde(rename = "globalMirror")]
        global_mirror: IdHolder,
    }

    let result = cx
        .with_client(|cl| async move {
            cl.gql_query::<GlobalMirror>(
                r#"
                mutation {
                    globalMirror(
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
