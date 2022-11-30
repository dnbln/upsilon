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
