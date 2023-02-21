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

use rocket::fairing::Fairing;
use serde::Deserialize;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum PluginApiVersion {
    V0Alpha1 = 0,

    __LAST,
}

impl PluginApiVersion {
    fn from_u32(v: u32) -> Option<Self> {
        if v >= Self::__LAST as u32 {
            return None;
        }

        match v {
            0 => Some(PluginApiVersion::V0Alpha1),
            _ => None,
        }
    }
}

pub struct PluginMetadata {
    pub name: &'static str,
    pub version: &'static str,
    plugin_api_version: u32,
    pub create: fn(PluginConfig) -> Result<Box<dyn Plugin>, PluginError>,
}

pub const CURRENT_PLUGIN_API_VERSION: PluginApiVersion = PluginApiVersion::V0Alpha1;

impl PluginMetadata {
    pub fn new(
        name: &'static str,
        version: &'static str,
        create: fn(PluginConfig) -> Result<Box<dyn Plugin>, PluginError>,
    ) -> Self {
        Self {
            name,
            version,
            plugin_api_version: CURRENT_PLUGIN_API_VERSION as u32,
            create,
        }
    }

    pub fn get_plugin_api_version(&self) -> Option<PluginApiVersion> {
        PluginApiVersion::from_u32(self.plugin_api_version)
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct PluginConfig(Box<serde_yaml::Value>);

impl PluginConfig {
    pub fn new(v: serde_yaml::Value) -> Self {
        Self(Box::new(v))
    }

    pub fn default_for_deps() -> Self {
        Self::new(serde_yaml::Value::Null)
    }

    pub fn deserialize<T: serde::de::DeserializeOwned>(self) -> Result<T, PluginError> {
        Ok(serde_yaml::from_value(*self.0)?)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginError {
    #[error("serde_json error: {0}")]
    SerdeYaml(#[from] serde_yaml::Error),
    #[error("Plugin failed to initialize (other: {0})")]
    Other(#[from] Box<dyn std::error::Error + Send>),
}

pub trait Plugin: Send + 'static {
    fn init<'a, 'b, 'registry, 'fut>(
        &'a mut self,
        load: &'b mut dyn PluginLoadApi<'registry>,
    ) -> Pin<Box<dyn Future<Output = Result<(), PluginError>> + Send + 'fut>>
    where
        'a: 'fut,
        'registry: 'b,
        'b: 'fut;
}

#[rocket::async_trait]
pub trait PluginLoadApi<'registry>: Send + 'registry {
    async fn _register_fairing(&mut self, fairing: Arc<dyn Fairing>);
}

impl<'registry> dyn PluginLoadApi<'registry> + 'registry {
    pub async fn register_fairing<F: Fairing>(&mut self, fairing: F) {
        self._register_fairing(Arc::new(fairing)).await;
    }
}

/*
#[cfg_attr(feature = "dynamic-plugins", no_mangle)]
pub fn __upsilon_plugin() -> PluginMetadata {
    PluginMetadata::new("upsilon-sample", "0.0.1", load_sample)
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
        load: &'b mut dyn PluginLoadApi<'registry>,
    ) -> Pin<Box<dyn Future<Output = Result<(), PluginError>> + 'fut>>
        where
            'a: 'fut,
            'registry: 'b,
            'b: 'fut,
    {
        Box::pin(std::future::ready(Ok(())))
    }
}
*/
