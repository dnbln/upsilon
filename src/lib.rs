use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Build, Rocket};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct Config {
    vcs: upsilon_vcs::UpsilonVcsConfig,
}

pub struct ConfigManager;

#[rocket::async_trait]
impl Fairing for ConfigManager {
    fn info(&self) -> Info {
        Info {
            name: "API fairing configurator",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        let app_config = match rocket.figment().extract::<Config>() {
            Ok(config) => config,
            Err(e) => {
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };

        let Config { vcs } = app_config;

        Ok(rocket.manage(vcs))
    }
}
