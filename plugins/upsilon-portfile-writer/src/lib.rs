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

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;

use log::error;
use upsilon_plugin_core::rocket::fairing::{Fairing, Info, Kind};
use upsilon_plugin_core::rocket::{async_trait, Orbit, Rocket};
use upsilon_plugin_core::{
    Plugin, PluginConfig, PluginError, PluginLoad, PluginLoadApi, PluginMetadata
};

#[cfg_attr(feature = "dynamic-plugins", no_mangle)]
pub const __UPSILON_METADATA: PluginMetadata = PluginMetadata::new("portfile-writer", "0.0.1");

#[cfg_attr(feature = "dynamic-plugins", no_mangle)]
pub const __UPSILON_PLUGIN: PluginLoad = load_plugin;

fn load_plugin(config: PluginConfig) -> Result<Box<dyn Plugin>, PluginError> {
    let config = config.deserialize::<PortfileWriterConfig>()?;
    Ok(Box::new(PortfileWriterPlugin { config }))
}

#[derive(serde::Deserialize)]
pub struct PortfileWriterConfig {
    portfile: PathBuf,
}

pub struct PortfileWriterPlugin {
    config: PortfileWriterConfig,
}

impl Plugin for PortfileWriterPlugin {
    fn init<'a, 'b, 'registry, 'fut>(
        &'a mut self,
        load: &'b mut dyn PluginLoadApi<'registry>,
    ) -> Pin<Box<dyn Future<Output = Result<(), PluginError>> + Send + 'fut>>
    where
        'a: 'fut,
        'registry: 'b,
        'b: 'fut,
    {
        let fut = async move {
            load.register_fairing(PortFileWriter(self.config.portfile.clone()))
                .await;

            Ok(())
        };
        Box::pin(fut)
    }
}

struct PortFileWriter(PathBuf);

#[async_trait]
impl Fairing for PortFileWriter {
    fn info(&self) -> Info {
        Info {
            name: "Port file writer fairing",
            kind: Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let port = rocket.config().port;
        let portfile = &self.0;

        if let Err(e) = tokio::fs::write(portfile, port.to_string()).await {
            error!("Failed to write port file: {e}");
        }
    }
}
