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

use upsilon_test_support::{upsilon_test, Anything, IdHolder, TestCx, TestCxConfig, TestResult};
use upsilon_test_support::anyhow::bail;
use upsilon_test_support::git2::BranchType;

fn upsilon_basic_config(cfg: &mut TestCxConfig) {
    cfg.with_config(
        r#"
vcs:
  path: ./vcs/repos
  jailed: true
  git-protocol:
    enable: true
    git-daemon:
      start: false
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

async fn make_global_mirror(cx: &mut TestCx) -> TestResult<String> {
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

#[upsilon_test]
async fn github_mirror(
    #[cfg_setup(upsilon_basic_config)]
    #[setup(upsilon_test_support::helpers::register_dummy_user)]
    cx: &mut TestCx,
) -> TestResult {
    let global_mirror_id = make_global_mirror(cx).await?;

    Ok(())
}

#[upsilon_test]
async fn can_clone_to_local(
    #[cfg_setup(upsilon_basic_config)]
    #[setup(upsilon_test_support::helpers::register_dummy_user)]
    cx: &mut TestCx,
) -> TestResult {
    make_global_mirror(cx).await?;

    let (_, clone) = cx.clone("clone-upsilon", "upsilon").await?;

    Ok(())
}

#[upsilon_test]
async fn clone_twice_same_output(
    #[cfg_setup(upsilon_basic_config)]
    #[setup(upsilon_test_support::helpers::register_dummy_user)]
    cx: &mut TestCx,
) -> TestResult {
    make_global_mirror(cx).await?;

    let (_, clone1) = cx.clone("clone-upsilon-1", "upsilon").await?;
    let (_, clone2) = cx.clone("clone-upsilon-2", "upsilon").await?;

    let trunk1 = clone1.find_branch("trunk", BranchType::Local)?;
    let trunk2 = clone2.find_branch("trunk", BranchType::Local)?;

    let trunk1_commit = trunk1.get().peel_to_commit()?;
    let trunk2_commit = trunk2.get().peel_to_commit()?;

    if trunk1_commit.id() != trunk2_commit.id() {
        bail!("Clones' trunk refs point to different commits");
    }

    Ok(())
}