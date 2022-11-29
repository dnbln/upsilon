use std::path::PathBuf;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{Build, Orbit, Rocket};
use serde::{Deserialize, Deserializer};
use upsilon_core::config::Cfg;
use upsilon_data::{DataClient, DataClientMasterHolder};
use upsilon_data_inmemory::InMemoryStorageSaveStrategy;

#[derive(Deserialize, Debug)]
pub struct Config {
    vcs: upsilon_vcs::UpsilonVcsConfig,
    #[serde(rename = "data-backend")]
    data_backend: DataBackendConfig,

    users: upsilon_core::config::UsersConfig,
}

#[derive(Debug, Clone)]
pub enum InMemoryConfigSaveStrategy {
    Save { path: PathBuf },
    DontSave,
}

impl<'de> Deserialize<'de> for InMemoryConfigSaveStrategy {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct SaveStrategy {
            save: bool,
            path: Option<PathBuf>,
        }

        let s = SaveStrategy::deserialize(deserializer)?;

        match s {
            SaveStrategy {
                save: true,
                path: Some(path),
            } => Ok(Self::Save { path }),
            SaveStrategy {
                save: true,
                path: None,
            } => Err(serde::de::Error::custom(
                "Path is required when save is true",
            )),
            SaveStrategy {
                save: false,
                path: _,
            } => Ok(Self::DontSave),
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct InMemoryDataBackendConfig {
    #[serde(flatten)]
    save_strategy: InMemoryConfigSaveStrategy,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PostgresDataBackendConfig {
    host: String,
    port: u16,
    user: String,
    password: String,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum DataBackendConfig {
    #[serde(rename = "in-memory")]
    InMemory(InMemoryDataBackendConfig),
    #[serde(rename = "postgres")]
    Postgres(PostgresDataBackendConfig),
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

    async fn on_ignite(&self, mut rocket: Rocket<Build>) -> rocket::fairing::Result {
        let app_config = match rocket.figment().extract::<Config>() {
            Ok(config) => config,
            Err(e) => {
                rocket::config::pretty_print_error(e);
                return Err(rocket);
            }
        };

        let Config {
            vcs,
            data_backend,
            users,
        } = app_config;

        rocket = match data_backend {
            DataBackendConfig::InMemory(config) => {
                rocket.attach(InMemoryDataBackendFairing(config))
            }
            DataBackendConfig::Postgres(config) => {
                rocket.attach(PostgresDataBackendFairing(config))
            }
        }
        .attach(DataBackendShutdownFairing);

        Ok(rocket.manage(Cfg::new(vcs)).manage(Cfg::new(users)))
    }
}

struct InMemoryDataBackendFairing(InMemoryDataBackendConfig);

#[rocket::async_trait]
impl Fairing for InMemoryDataBackendFairing {
    fn info(&self) -> Info {
        Info {
            name: "In-memory data backend",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        let cfg = upsilon_data_inmemory::InMemoryStorageConfiguration {
            save_strategy: match &self.0.save_strategy {
                InMemoryConfigSaveStrategy::Save { path } => {
                    InMemoryStorageSaveStrategy::Save { path: path.clone() }
                }
                InMemoryConfigSaveStrategy::DontSave => InMemoryStorageSaveStrategy::DontSave,
            },
        };

        let client = match upsilon_data_inmemory::InMemoryDataClient::init_client(cfg).await {
            Ok(client) => client,
            Err(e) => {
                eprintln!("Failed to initialize in-memory data backend client: {e}");
                return Err(rocket);
            }
        };

        let client_master_holder = DataClientMasterHolder::new(client);

        Ok(rocket.manage(client_master_holder))
    }
}

struct PostgresDataBackendFairing(PostgresDataBackendConfig);

#[rocket::async_trait]
impl Fairing for PostgresDataBackendFairing {
    fn info(&self) -> Info {
        Info {
            name: "Postgres data backend",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        Ok(rocket)
    }
}

struct DataBackendShutdownFairing;

#[rocket::async_trait]
impl Fairing for DataBackendShutdownFairing {
    fn info(&self) -> Info {
        Info {
            name: "Data backend shutdown",
            kind: Kind::Shutdown | Kind::Singleton,
        }
    }

    async fn on_shutdown(&self, rocket: &Rocket<Orbit>) {
        let holder = rocket
            .state::<DataClientMasterHolder>()
            .expect("Missing state");

        holder
            .on_shutdown()
            .await
            .expect("Data backend shutdown error");
    }
}
