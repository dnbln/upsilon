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

use clap::Parser;
use log::info;
use serde::Deserialize;

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
    createUser(username: "a", password: "b", email: "c")
}
"#,
        )
        .await?
        .create_user;

    info!("Created user");

    client.set_token(token.token.clone());

    println!("token: {}", token.token);

    info!("Creating github mirror...");

    client
        .gql_mutation::<any::Any>(
            r#"
mutation {
    globalMirror(name:"upsilon", url:"https://github.com/dnbln/upsilon") {
        id
    }
}
"#,
        )
        .await?;

    info!("Created github mirror");

    Ok(())
}
