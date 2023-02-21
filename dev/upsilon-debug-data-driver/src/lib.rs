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
use std::pin::Pin;
use std::sync::Arc;

use log::info;
use rocket::fairing::{Fairing, Info, Kind};
use upsilon_plugin_core::{
    Plugin, PluginConfig, PluginError, PluginMetadata, PluginRegistryMutator
};

#[cfg_attr(feature = "dynamic-plugins", no_mangle)]
pub fn __upsilon_plugin() -> PluginMetadata {
    PluginMetadata::new("upsilon-debug-data-driver", "0.0.1", load_sample)
}

fn load_sample(config: PluginConfig) -> Result<Box<dyn Plugin>, PluginError> {
    let config = config.deserialize::<SamplePluginConfig>()?;
    Ok(Box::new(SamplePlugin { config }))
}

#[derive(serde::Deserialize)]
pub struct SamplePluginConfig {
    a: usize,
    b: usize,
}

pub struct SamplePlugin {
    config: SamplePluginConfig,
}

impl Plugin for SamplePlugin {
    fn init<'a, 'b, 'registry, 'fut>(
        &'a mut self,
        mutator: &'b mut dyn PluginRegistryMutator<'registry>,
    ) -> Pin<Box<dyn Future<Output = Result<(), PluginError>> + Send + 'fut>>
    where
        'a: 'fut,
        'registry: 'b,
        'b: 'fut,
    {
        let fut = async move {
            mutator.register_fairing(DebugDataDriverFairing).await;

            Ok::<_, PluginError>(())
        };

        Box::pin(fut)
    }
}

pub struct DebugDataDriverFairing;

#[rocket::async_trait]
impl Fairing for DebugDataDriverFairing {
    fn info(&self) -> Info {
        Info {
            name: "Debug Data Driver Fairing aaa",
            kind: Kind::Singleton | Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &rocket::Rocket<rocket::Orbit>) {
        info!("Debug Data Driver Fairing: on_liftoff");
    }
}
