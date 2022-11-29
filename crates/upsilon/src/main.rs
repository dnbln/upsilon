#[macro_use]
extern crate rocket;

use figment::providers::Format;
use rocket::figment::providers::Yaml;

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    let figment = rocket::Config::figment().merge(Yaml::file("upsilon.yaml"));

    rocket::custom(figment)
        .attach(upsilon_api::ApiConfigurator)
        .attach(upsilon_api::GraphQLApiConfigurator)
        .attach(upsilon::ConfigManager)
}
