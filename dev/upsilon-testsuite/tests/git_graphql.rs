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

use upsilon_test_support::prelude::*;

#[upsilon_test]
async fn get_last_commit_on_branch_same_as_cloned_info(cx: &mut TestCx) -> TestResult {
    let global_mirror_id = make_global_mirror_from_host_repo(cx).await?;

    let (_, clone) = cx
        .clone_without_credentials("clone-upsilon", upsilon_global)
        .await?;

    const BRANCH_NAME: &str = "trunk";

    let trunk_commit = branch_commit(&clone, BRANCH_NAME)?;
    let commit_id = trunk_commit.id();
    let commit_message = trunk_commit.message().expect("Commit message is not UTF-8");

    let result = cx
        .with_client(|cl| async move {
            cl.gql_query_with_variables::<serde_json::Value>(
                r#"
query($repoId: RepoId!, $branch: String!) {
  repo(repoId: $repoId) {
    name
    git {
      branch(name: $branch) {
        commit {
          sha
          message
        }
      }
    }
  }
}
"#,
                gql_vars! {
                    "repoId": global_mirror_id,
                    "branch": BRANCH_NAME,
                },
            )
            .await
        })
        .await?;

    assert_json_eq!(
        result,
        {
            "repo": {
                "name": "upsilon",
                "git": {
                    "branch": {
                        "commit": {
                            "sha": commit_id.to_string(),
                            "message": commit_message,
                        }
                    }
                }
            }
        }
    );

    Ok(())
}
