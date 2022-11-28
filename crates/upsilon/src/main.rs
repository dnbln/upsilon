#[macro_use]
extern crate rocket;

use figment::providers::Format;
use rocket::figment::providers::Yaml;
use rocket::serde::json::Json;
use serde::Serialize;

#[derive(Serialize)]
struct Message {
    value: i32,
}

#[get("/")]
fn hello_world() -> Json<Message> {
    Json(Message { value: 10 })
}

#[launch]
fn rocket() -> rocket::Rocket<rocket::Build> {
    let figment = rocket::Config::figment().merge(Yaml::file("upsilon.yaml"));

    rocket::custom(figment)
        .mount("/", routes![hello_world])
        .attach(upsilon_api::ApiConfigurator)
        .attach(upsilon::ConfigManager)
}
