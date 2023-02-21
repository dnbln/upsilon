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

use std::collections::HashMap;

use clap::Parser;
use log::{debug, info};
use serde::Deserialize;
use serde_json::json;
use upsilon_debug_data_driver::DebugDataDriverConfig;

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

    let config = DebugDataDriverConfig::from_env()?;

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

    info!("Creating users...");

    for (username, userinfo) in &config.users.users {
        info!("Creating user {}", username.0);
        let token = client
            .gql_mutation_with_variables::<CreateUserResponse>(
                r#"
mutation ($username: Username!, $password: PlainPassword!, $email: Email!) {
    createUser(username: $username, password: $password, email: $email)
}
"#,
                HashMap::from([
                    ("username", json!(&username.0)),
                    ("password", json!(&userinfo.password)),
                    ("email", json!(&userinfo.email)),
                ]),
            )
            .await?
            .create_user;

        info!("Created user {}", username.0);

        client.set_token(token.token.clone());

        println!("{}: token: {}", username.0, token.token);
    }

    info!("Created users");

    #[derive(Deserialize)]
    struct GlobalMirrorId {
        #[serde(rename = "_debug__globalMirror")]
        global_mirror: IdHolder,
    }

    #[derive(Deserialize)]
    struct SilentInitGlobal {
        #[serde(rename = "_debug__silentInitGlobal")]
        silent_init_global: IdHolder,
    }

    #[derive(Deserialize)]
    struct IdHolder {
        id: String,
    }

    info!("Creating repos...");

    for (repo_name, repo_config) in &config.repos {
        match &repo_config.setup_mirror_from {
            Some(setup_from) => {
                if repo_config.exists {
                    info!("Repo {} already exists, initializing...", repo_name.0);

                    let repo_id = client
                        .gql_mutation_with_variables::<SilentInitGlobal>(
                            r#"
mutation ($name: String!) {
    _debug__silentInitGlobal(name: $name) {
        id
    }
}
"#,
                            HashMap::from([("name", json!(&repo_name.0))]),
                        )
                        .await?
                        .silent_init_global
                        .id;

                    info!("Initialized repo {}", repo_name.0);

                    println!("{}: repo_id: {repo_id}", repo_name.0);
                } else {
                    info!("Creating repo {} from {setup_from}...", repo_name.0);

                    let repo_id = client
                        .gql_mutation_with_variables::<GlobalMirrorId>(
                            r#"
mutation ($name: String!, $url: String!) {
    _debug__globalMirror(name: $name, url: $url) {
        id
    }
}
"#,
                            HashMap::from([
                                ("name", json!(&repo_name.0)),
                                ("url", json!(&setup_from)),
                            ]),
                        )
                        .await?
                        .global_mirror
                        .id;

                    info!("Created repo {}", repo_name.0);

                    println!("{}: repo_id: {repo_id}", repo_name.0);
                }
            }
            None => {
                if repo_config.exists {
                    info!("Repo {} exists, initializing...", repo_name.0);

                    let repo_id = client
                        .gql_mutation_with_variables::<SilentInitGlobal>(
                            r#"
mutation ($name: String!) {
    _debug__silentInitGlobal(name: $name) {
        id
    }
}
"#,
                            HashMap::from([("name", json!(&repo_name.0))]),
                        )
                        .await?
                        .silent_init_global
                        .id;

                    info!("Initialized repo {}", repo_name.0);

                    println!("{}: repo_id: {repo_id}", repo_name.0);
                }
            }
        }
    }

    info!("Created repos");

    Ok(())
}
