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

#[macro_use]
extern crate rocket;

use figment::providers::{Env, Format, Yaml};
use figment::{Figment, Profile};
use rocket::config::Ident;

const DEV_PROFILE: Profile = Profile::const_new("dev");
const RELEASE_PROFILE: Profile = Profile::const_new("release");

#[cfg(debug_assertions)]
const DEFAULT_PROFILE: Profile = DEV_PROFILE;

#[cfg(not(debug_assertions))]
const DEFAULT_PROFILE: Profile = RELEASE_PROFILE;

#[launch]
async fn rocket() -> rocket::Rocket<rocket::Build> {
    let profile = Profile::from_env_or("UPSILON_PROFILE", DEFAULT_PROFILE);

    let default_rocket_config = match &profile {
        x if x == DEV_PROFILE => rocket::Config::debug_default(),
        x if x == RELEASE_PROFILE => rocket::Config::release_default(),
        _ => panic!("Invalid profile: {profile}"),
    };

    let default_rocket_config = rocket::Config {
        ident: Ident::try_new("upsilon").unwrap(),
        ..default_rocket_config
    };

    let figment = Figment::from(default_rocket_config)
        .merge(Yaml::file(Env::var_or("UPSILON_ROCKET_CONFIG", "upsilon-rocket.yaml")).nested())
        .merge(Yaml::file(Env::var_or("UPSILON_CONFIG", "upsilon.dev.yaml")).profile(DEV_PROFILE))
        .merge(Yaml::file(Env::var_or("UPSILON_CONFIG", "upsilon.yaml")).profile(RELEASE_PROFILE))
        .merge(
            Env::prefixed("UPSILON_")
                .ignore(&["PROFILE"])
                .global()
                .split("_"),
        )
        .select(profile);

    rocket::custom(figment).attach(upsilon_web::ConfigManager)
}
