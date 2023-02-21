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

use std::cell::RefCell;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::Arc;

use rocket::fairing::Fairing;
use rocket::{Build, error, Rocket, Route};
use tokio::sync::Mutex;
use upsilon_plugin_core::{
    Plugin, PluginApiVersion, PluginConfig, PluginMetadata, CURRENT_PLUGIN_API_VERSION
};

pub trait PluginHolder: Send + 'static {
    fn plugin(&mut self) -> &mut dyn Plugin;
}

impl PluginHolder for Box<dyn Plugin> {
    fn plugin(&mut self) -> &mut dyn Plugin {
        &mut **self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PluginLoaderError {
    #[error("loading library error: {0}")]
    LoadingLibraryError(#[from] libloading::Error),
    #[error("Unknown plugin")]
    UnknownPlugin,
    #[error("Plugin API version mismatch")]
    PluginApiVersionMismatch(#[from] ApiVersionMismatchError),
    #[error("Plugin error: {0}")]
    PluginError(#[from] upsilon_plugin_core::PluginError),
    #[error("Plugin already loaded")]
    PluginAlreadyLoaded,
    #[error("Plugin loading finished, cannot load other plugins")]
    PluginLoadingFinished,
    #[error("Other error: {0}")]
    OtherError(#[from] Box<dyn std::error::Error>),
}

pub trait PluginLoader {
    type Error: Debug + Into<PluginLoaderError>;
    type Holder: PluginHolder;

    fn load_plugin(&self, name: &str, config: PluginConfig) -> Result<Self::Holder, Self::Error>;
}

pub trait PluginLoaderWrapper: Send {
    fn load_plugin(
        &self,
        name: &str,
        config: PluginConfig,
    ) -> Result<Box<dyn PluginHolder>, PluginLoaderError>;
}

impl<Error, Holder, T> PluginLoaderWrapper for T
where
    T: PluginLoader<Error = Error, Holder = Holder> + Send,
    Error: Debug,
    PluginLoaderError: From<Error>,
    Holder: PluginHolder + 'static,
{
    fn load_plugin(
        &self,
        name: &str,
        config: PluginConfig,
    ) -> Result<Box<dyn PluginHolder>, PluginLoaderError> {
        let holder = self.load_plugin(name, config)?;

        Ok(Box::new(holder))
    }
}

pub struct StaticPluginLoader {
    known_plugins: HashMap<&'static str, PluginMetadata>,
}

impl StaticPluginLoader {
    pub fn new<I: IntoIterator<Item = PluginMetadata>>(known_plugins: I) -> Self {
        let known_plugins = known_plugins
            .into_iter()
            .map(|plugin| (plugin.name, plugin))
            .collect::<HashMap<_, _>>();

        Self { known_plugins }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Plugin API version mismatch: manager: {manager:?}, plugin: {plugin:?}")]
pub struct ApiVersionMismatchError {
    manager: PluginApiVersion,
    plugin: Option<PluginApiVersion>,
}

#[derive(Debug, thiserror::Error)]
pub enum StaticPluginLoaderError {
    #[error("Unknown plugin")]
    UnknownPlugin,
    #[error("Plugin API version mismatch")]
    PluginApiVersionMismatch(#[from] ApiVersionMismatchError),
    #[error("Plugin error: {0}")]
    PluginError(#[from] upsilon_plugin_core::PluginError),
}

impl From<StaticPluginLoaderError> for PluginLoaderError {
    fn from(value: StaticPluginLoaderError) -> Self {
        match value {
            StaticPluginLoaderError::UnknownPlugin => Self::UnknownPlugin,
            StaticPluginLoaderError::PluginApiVersionMismatch(e) => {
                Self::PluginApiVersionMismatch(e)
            }
            StaticPluginLoaderError::PluginError(e) => Self::PluginError(e),
        }
    }
}

fn check_version(plugin: &PluginMetadata) -> Result<(), ApiVersionMismatchError> {
    let manager = CURRENT_PLUGIN_API_VERSION;
    let plugin = plugin.get_plugin_api_version();

    if Some(manager) != plugin {
        return Err(ApiVersionMismatchError { manager, plugin });
    }

    Ok(())
}

impl PluginLoader for StaticPluginLoader {
    type Error = StaticPluginLoaderError;
    type Holder = Box<dyn Plugin>;

    fn load_plugin(&self, name: &str, config: PluginConfig) -> Result<Self::Holder, Self::Error> {
        let metadata = match self.known_plugins.get(name) {
            None => {
                return Err(StaticPluginLoaderError::UnknownPlugin);
            }
            Some(metadata) => metadata,
        };

        check_version(metadata)?;

        let plugin_create = metadata.create;

        let plugin = plugin_create(config)?;

        Ok(plugin)
    }
}

#[cfg(feature = "dynamic-plugins")]
mod dynamic_plugin_loader;

pub use dynamic_plugin_loader::{DynamicPluginLoader, DynamicPluginLoaderError};

pub struct PluginManager {
    plugin_loader: Box<dyn PluginLoaderWrapper>,
    plugins: HashMap<String, Box<dyn PluginHolder>>,
    finished_loading: bool,
    rocket: Option<Rocket<Build>>,
}

pub struct PluginRegistryMutator {
    plugin_name: String,
    rocket: Arc<Mutex<Option<Rocket<Build>>>>,
}

#[rocket::async_trait]
impl<'a> upsilon_plugin_core::PluginRegistryMutator<'a> for PluginRegistryMutator {
    async fn _register_fairing(&mut self, fairing: Arc<dyn Fairing>) {
        let mut lock = self.rocket.lock().await;

        error!("Registering fairing for plugin {}", self.plugin_name);

        let r = lock.take().unwrap();
        let r = r.attach(fairing);
        *lock = Some(r);
    }
}

impl PluginManager {
    pub fn new(plugin_loader: Box<dyn PluginLoaderWrapper>, rocket: Rocket<Build>) -> Self {
        Self {
            plugin_loader,
            plugins: HashMap::new(),
            finished_loading: false,
            rocket: Some(rocket),
        }
    }

    pub fn finish(&mut self) -> Rocket<Build> {
        self.rocket.take().unwrap()
    }

    pub async fn load_plugin(
        &mut self,
        name: &str,
        config: PluginConfig,
    ) -> Result<(), PluginLoaderError> {
        if self.finished_loading {
            return Err(PluginLoaderError::PluginLoadingFinished);
        }

        let plugin = self.plugin_loader.load_plugin(name, config)?;

        let plugin = match self.plugins.entry(name.to_string()) {
            Entry::Occupied(_) => {
                return Err(PluginLoaderError::PluginAlreadyLoaded);
            }
            Entry::Vacant(entry) => entry.insert(plugin),
        };

        let p = plugin.plugin();

        let mut mutator = PluginRegistryMutator {
            plugin_name: name.to_string(),
            rocket: Arc::new(Mutex::new(self.rocket.take())),
        };

        let r = p.init(&mut mutator).await;

        let PluginRegistryMutator { rocket, .. } = mutator;

        self.rocket = Some(Arc::try_unwrap(rocket).unwrap().into_inner().unwrap());

        r?;

        Ok(())
    }

    pub fn get_plugin(&mut self, name: &str) -> Option<&mut dyn Plugin> {
        self.plugins.get_mut(name).map(|p| p.plugin())
    }
}
