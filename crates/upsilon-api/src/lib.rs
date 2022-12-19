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

#[macro_use]
extern crate rocket;
extern crate upsilon_procx;

use graphql::GraphQLContext;
use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Build, Rocket, State};

use crate::auth::AuthContext;

pub mod auth;
mod graphql;

mod error;
mod repo_lookup_path;

pub struct GraphQLApiConfigurator;

#[rocket::async_trait]
impl Fairing for GraphQLApiConfigurator {
    fn info(&self) -> Info {
        Info {
            name: "GraphQL API configurator",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        Ok(rocket
            .mount(
                "/",
                routes![graphiql, get_graphql_handler, post_graphql_handler],
            )
            .manage(graphql::Schema::new(
                graphql::QueryRoot,
                graphql::MutationRoot,
                graphql::SubscriptionRoot,
            ))
            .manage(AuthContext::new(2048)))
    }
}

#[rocket::get("/")]
fn graphiql() -> rocket::response::content::RawHtml<String> {
    juniper_rocket::graphiql_source("/graphql", None)
}

#[rocket::get("/graphql?<request>")]
async fn get_graphql_handler(
    request: juniper_rocket::GraphQLRequest,
    schema: &State<graphql::Schema>,
    context: GraphQLContext,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&**schema, &context).await
}

#[rocket::post("/graphql", data = "<request>")]
async fn post_graphql_handler(
    request: juniper_rocket::GraphQLRequest,
    schema: &State<graphql::Schema>,
    context: GraphQLContext,
) -> juniper_rocket::GraphQLResponse {
    request.execute(&**schema, &context).await
}

pub fn graphql_schema() -> String {
    let schema = graphql::Schema::new(
        graphql::QueryRoot,
        graphql::MutationRoot,
        graphql::SubscriptionRoot,
    );

    schema.as_schema_language()
}
