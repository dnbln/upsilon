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

use std::path::PathBuf;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{error, Build, Orbit, Rocket};
use serde::{Deserialize, Deserializer};
use upsilon_data::{DataClient, DataClientMaster, DataClientMasterHolder};
use upsilon_data_cache_inmemory::CacheInMemoryConfig;
use upsilon_data_inmemory::InMemoryStorageSaveStrategy;

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
                "path is required when save is true",
            )),
            SaveStrategy {
                save: false,
                path: _,
            } => Ok(Self::DontSave),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
struct CacheInMemoryConfigSizes {
    #[serde(default = "default_cache_size")]
    pub max_users: usize,
    #[serde(default = "default_cache_size")]
    pub max_repos: usize,
    #[serde(default = "default_cache_size")]
    pub max_orgs: usize,
    #[serde(default = "default_cache_size")]
    pub max_repo_permissions: usize,
    #[serde(default = "default_cache_size")]
    pub max_org_members: usize,
    #[serde(default = "default_cache_size")]
    pub max_teams: usize,
}

fn default_cache_size() -> usize {
    10
}

impl From<CacheInMemoryConfigSizes> for upsilon_data_cache_inmemory::CacheInMemoryConfigSizes {
    fn from(value: CacheInMemoryConfigSizes) -> Self {
        Self {
            max_users: value.max_users,
            max_repos: value.max_repos,
            max_orgs: value.max_orgs,
            max_repo_permissions: value.max_repo_permissions,
            max_org_members: value.max_org_members,
            max_teams: value.max_teams,
        }
    }
}

#[derive(Deserialize, Debug, Clone)]
pub struct InMemoryDataBackendConfig {
    #[serde(flatten)]
    save_strategy: InMemoryConfigSaveStrategy,
    cache: Option<CacheInMemoryConfigSizes>,
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

pub(crate) struct InMemoryDataBackendFairing(InMemoryDataBackendConfig);

impl InMemoryDataBackendFairing {
    pub fn new(config: InMemoryDataBackendConfig) -> Self {
        Self(config)
    }
}

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
                error!("Failed to initialize in-memory data backend: {}", e);
                return Err(rocket);
            }
        };

        let client_master_holder = match self.0.cache {
            None => DataClientMasterHolder::new(client),
            Some(cache_sizes) => {
                let client =
                    match upsilon_data_cache_inmemory::CacheInMemoryDataClient::init_client(
                        CacheInMemoryConfig::new(cache_sizes.into(), Box::new(client)),
                    )
                    .await
                    {
                        Ok(client) => client,
                        Err(e) => {
                            error!("Failed to initialize in-memory data backend: {}", e);
                            return Err(rocket);
                        }
                    };

                DataClientMasterHolder::new(client)
            }
        };

        Ok(rocket
            .manage(client_master_holder)
            .attach(DataBackendShutdownFairing))
    }
}

pub(crate) struct PostgresDataBackendFairing(PostgresDataBackendConfig);

impl PostgresDataBackendFairing {
    pub fn new(config: PostgresDataBackendConfig) -> Self {
        Self(config)
    }
}

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

        match holder.on_shutdown().await {
            Ok(_) => (),
            Err(e) => error!("Failed to shutdown data backend: {}", e),
        }
    }
}
