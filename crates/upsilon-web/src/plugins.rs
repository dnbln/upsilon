/*
 *        Copyright (c) 2023 Dinu Blanovschi
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

use std::collections::HashMap;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{error, Build, Rocket};
use upsilon_plugin_core::PluginConfig;
use upsilon_plugin_manager::{PluginManager, PluginName};
use upsilon_plugins_static::static_plugins;

pub struct PluginsFairing {
    pub plugins: PluginsConfigMap,
}

#[derive(serde::Deserialize, Debug)]
#[serde(transparent)]
pub struct PluginsConfigMap {
    plugins: HashMap<PluginName, PluginConfig>,
}

#[rocket::async_trait]
impl Fairing for PluginsFairing {
    fn info(&self) -> Info {
        Info {
            name: "Plugins Fairing",
            kind: Kind::Ignite | Kind::Singleton,
        }
    }

    async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
        #[cfg(feature = "static-plugins")]
        let (registry, loader) = static_plugins();

        let mut plugin_manager = PluginManager::new(Box::new(loader), rocket);

        let r = plugin_manager
            .load_plugins(&registry, &self.plugins.plugins)
            .await;

        let rocket = plugin_manager.finish().await;

        match r {
            Ok(()) => Ok(rocket),
            Err(e) => {
                error!("Failed to load plugins: {e}");
                Err(rocket)
            }
        }
    }
}
