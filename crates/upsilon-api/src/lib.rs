#[macro_use]
extern crate rocket;
#[macro_use(v1, api_routes)]
extern crate upsilon_procx;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Build, Rocket, State};
use upsilon_graphql::GraphQLContext;

mod routes;

mod error;

#[upsilon_procx::api_configurator]
pub struct ApiConfigurator;

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
            .manage(upsilon_graphql::Schema::new(
                upsilon_graphql::QueryRoot,
                upsilon_graphql::MutationRoot,
                upsilon_graphql::SubscriptionRoot,
            )))
    }
}

#[rocket::get("/")]
fn graphiql() -> rocket::response::content::RawHtml<String> {
    juniper_rocket::graphiql_source("/graphql", None)
}

#[rocket::get("/graphql?<request>")]
async fn get_graphql_handler(
    request: juniper_rocket::GraphQLRequest,
    schema: &State<upsilon_graphql::Schema>,
    db: &State<upsilon_data::DataClientMasterHolder>,
) -> juniper_rocket::GraphQLResponse {
    let context = GraphQLContext::new(db.inner().clone());
    request.execute(&**schema, &context).await
}

#[rocket::post("/graphql", data = "<request>")]
async fn post_graphql_handler(
    request: juniper_rocket::GraphQLRequest,
    schema: &State<upsilon_graphql::Schema>,
    db: &State<upsilon_data::DataClientMasterHolder>,
) -> juniper_rocket::GraphQLResponse {
    let context = GraphQLContext::new(db.inner().clone());
    request.execute(&**schema, &context).await
}
