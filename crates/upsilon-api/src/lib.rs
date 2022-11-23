#[macro_use]
extern crate rocket;
#[macro_use(v1, api_routes)]
extern crate upsilon_procx;

use rocket::{Build, Rocket};

mod routes;

mod error;

#[upsilon_procx::api_configurator]
pub struct ApiConfigurator;
