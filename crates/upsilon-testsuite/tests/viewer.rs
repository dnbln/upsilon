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

use upsilon_test_support::prelude::*;

#[derive(serde::Deserialize, Debug, PartialEq)]
struct ViewerId {
    id: String,
}

#[derive(serde::Deserialize)]
struct ViewerIdUsername {
    #[serde(flatten)]
    id: ViewerId,
    username: String,
}

#[derive(serde::Deserialize)]
struct QueryResult<T> {
    viewer: T,
}

async fn query_viewer_id(cl: Client) -> TestResult<ViewerId> {
    Ok(cl
        .gql_query::<QueryResult<ViewerId>>(
            r#"
query {
  viewer {
    id
  }
}
"#,
        )
        .await?
        .viewer)
}

async fn query_viewer_username_and_id(cl: Client) -> TestResult<ViewerIdUsername> {
    Ok(cl
        .gql_query::<QueryResult<ViewerIdUsername>>(
            r#"
query {
  viewer {
    id
    username
  }
}
"#,
        )
        .await?
        .viewer)
}

#[upsilon_test]
async fn viewer_id_and_username(cx: &mut TestCx) -> TestResult {
    cx.create_user("test", "test", "test@example.org").await?;

    let username_and_id = cx
        .with_client_as_user("test", query_viewer_username_and_id)
        .await?;

    assert_eq!(username_and_id.username, "test");

    let id = cx.with_client_as_user("test", query_viewer_id).await?;

    assert_eq!(username_and_id.id, id);

    Ok(())
}

#[upsilon_test]
async fn multiple_viewers(cx: &mut TestCx) -> TestResult {
    cx.create_user("usera", "test", "test1@example.org")
        .await?;
    cx.create_user("userb", "test", "test2@example.org")
        .await?;

    let for_user_a = cx
        .with_client_as_user("usera", query_viewer_username_and_id)
        .await?;
    assert_eq!(for_user_a.username, "usera");

    let for_user_b = cx
        .with_client_as_user("userb", query_viewer_username_and_id)
        .await?;
    assert_eq!(for_user_b.username, "userb");

    let id_for_user_a = cx.with_client_as_user("usera", query_viewer_id).await?;
    assert_eq!(for_user_a.id, id_for_user_a);

    let id_for_user_b = cx.with_client_as_user("userb", query_viewer_id).await?;
    assert_eq!(for_user_b.id, id_for_user_b);

    Ok(())
}
