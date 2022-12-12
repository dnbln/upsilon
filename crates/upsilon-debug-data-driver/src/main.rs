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

use clap::Parser;
use log::info;
use serde::Deserialize;
use serde_json::json;

use crate::client::Client;

#[derive(Parser, Debug)]
struct App {
    #[clap(short, long, default_value_t = 8000)]
    port: u16,
}

mod any;
mod client;

type DDDResult<T> = anyhow::Result<T>;

#[tokio::main]
async fn main() -> DDDResult<()> {
    pretty_env_logger::init();
    let app = App::parse();

    let mut client = Client::new(app.port);

    #[derive(Deserialize)]
    struct CreateUserResponse {
        #[serde(rename = "createUser")]
        create_user: Token,
    }

    #[derive(Deserialize)]
    #[serde(transparent)]
    struct Token {
        token: String,
    }

    info!("Creating user...");

    let token = client
        .gql_mutation::<CreateUserResponse>(
            r#"
mutation {
    createUser(username: "dinu", password: "aaa", email: "git@dnbln.dev")
}
"#,
        )
        .await?
        .create_user;

    info!("Created user");

    client.set_token(token.token.clone());

    println!("token: {}", token.token);

    info!("Creating github mirror...");

    #[derive(Deserialize)]
    struct GlobalMirrorId {
        #[serde(rename = "globalMirror")]
        global_mirror: IdHolder,
    }

    #[derive(Deserialize)]
    struct IdHolder {
        id: String,
    }

//     let repo_id = client
//         .gql_mutation::<GlobalMirrorId>(
//             r#"
// mutation {
//     globalMirror(name:"upsilon", url:"https://github.com/dnbln/upsilon") {
//         id
//     }
// }
// "#,
//         )
//         .await?
//         .global_mirror
//         .id;

    let repo_id = client
        .gql_mutation::<SilentInitGlobal>(
            r#"
mutation {
    silentInitGlobal(name: "upsilon") {
        id
    }
}
"#,
        )
        .await?
        .silent_init_global
        .id;

    println!("repo_id: {repo_id}");

    info!("Created github mirror");

    info!("Initializing linux repo...");

    #[derive(Deserialize)]
    struct SilentInitGlobal {
        #[serde(rename = "silentInitGlobal")]
        silent_init_global: IdHolder,
    }

    let linux_repo_id = client
        .gql_mutation::<SilentInitGlobal>(
            r#"
mutation {
    silentInitGlobal(name: "linux") {
        id
    }
}
"#,
        )
        .await?
        .silent_init_global
        .id;

    info!("Initialized linux repo");

    println!("linux_repo_id: {linux_repo_id}");

    info!("Testing cache ...");

    #[derive(Deserialize)]
    struct UserByUsernameResponse {
        #[serde(rename = "userByUsername")]
        user_by_username: IdHolder,
    }

    // load user by username in the cache, with the first query
    let id = client
        .gql_query::<UserByUsernameResponse>(
            r#"
query {
    userByUsername(username: "dinu") {
        id
    }
}
"#,
        )
        .await?
        .user_by_username
        .id;

    // query the user again, this time it should be in the cache
    client
        .gql_query_with_variables::<any::Any>(
            r#"
query($id: UserId!) {
    user(userId: $id) {
        id
    }
}
"#,
            HashMap::from([("id", json!(id))]),
        )
        .await?;

    info!("Successfully tried out cache");

    info!("Querying some git state ...");

    client
        .gql_query_with_variables::<any::Any>(
            r#"
query($repoId: RepoId!){
    repo(repoId: $repoId) {
        id
        name
        git {
            commit(sha:"138f92b30c111f9e91005bc60b528fc76ab20692") {
                sha
                message
            }

            branch(name: "trunk") {
                name
                commit {
                    sha
                    message
                }
            }
        }
    }
}
"#,
            HashMap::from([("repoId", json!(repo_id))]),
        )
        .await?;

    info!("Successfully queried some git state");

    Ok(())
}
