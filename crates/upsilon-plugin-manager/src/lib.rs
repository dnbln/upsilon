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

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

use rocket::fairing::{Fairing, Info, Kind};
use rocket::{error, info, Build, Orbit, Rocket};
use tokio::fs;
use tokio::sync::Mutex;
use upsilon_plugin_core::{
    BoxedLiftoffHook, Plugin, PluginApiVersion, PluginConfig, PluginLoad, PluginMetadata, CURRENT_PLUGIN_API_VERSION
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
pub enum PluginManagerError {
    #[error("loading library error: {0}")]
    LoadingLibraryError(#[from] libloading::Error),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
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
    #[error("Serde yaml error: {0}")]
    SerdeYamlError(#[from] serde_yaml::Error),
    #[error("Solvent error: {0}")]
    SolventError(#[from] solvent::SolventError),
    #[error("Load plugin error({0}): {1}")]
    LoadPluginError(PluginName, #[source] Box<PluginManagerError>),
    #[error("Other error: {0}")]
    OtherError(#[from] Box<dyn std::error::Error + Send>),
}

pub trait PluginLoader {
    type Error: Debug + Into<PluginManagerError>;
    type Holder: PluginHolder;

    fn load_plugin(&self, name: &str, config: &PluginConfig) -> Result<Self::Holder, Self::Error>;
}

pub trait PluginLoaderWrapper: Send {
    fn load_plugin(
        &self,
        name: &str,
        config: &PluginConfig,
    ) -> Result<Box<dyn PluginHolder>, PluginManagerError>;
}

impl<Error, Holder, T> PluginLoaderWrapper for T
where
    T: PluginLoader<Error = Error, Holder = Holder> + Send,
    Error: Debug,
    PluginManagerError: From<Error>,
    Holder: PluginHolder + 'static,
{
    fn load_plugin(
        &self,
        name: &str,
        config: &PluginConfig,
    ) -> Result<Box<dyn PluginHolder>, PluginManagerError> {
        let holder = self.load_plugin(name, config)?;

        Ok(Box::new(holder))
    }
}

pub struct StaticPluginLoader {
    known_plugins: HashMap<&'static str, (PluginMetadata, PluginLoad)>,
}

impl StaticPluginLoader {
    pub fn new<I: IntoIterator<Item = (PluginMetadata, PluginLoad)>>(known_plugins: I) -> Self {
        let known_plugins = known_plugins
            .into_iter()
            .map(|(metadata, load)| (metadata.name, (metadata, load)))
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

impl From<StaticPluginLoaderError> for PluginManagerError {
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

    fn load_plugin(&self, name: &str, config: &PluginConfig) -> Result<Self::Holder, Self::Error> {
        let (metadata, plugin_load) = match self.known_plugins.get(name) {
            None => {
                return Err(StaticPluginLoaderError::UnknownPlugin);
            }
            Some(plugin) => plugin,
        };

        check_version(metadata)?;

        let plugin = plugin_load(config.clone())?;

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

    liftoff_hooks: Arc<Mutex<Vec<(RegisteredHookInfo, BoxedLiftoffHook)>>>,
}

#[derive(Debug)]
pub struct RegisteredHookInfo {
    pub plugin: PluginName,
    pub name: String,
}

pub struct PluginLoadApiImpl {
    plugin_name: PluginName,
    rocket: Arc<Mutex<Option<Rocket<Build>>>>,
    liftoff_hooks: Arc<Mutex<Vec<(RegisteredHookInfo, BoxedLiftoffHook)>>>,
}

#[rocket::async_trait]
impl<'a> upsilon_plugin_core::PluginLoadApi<'a> for PluginLoadApiImpl {
    async fn _register_fairing(&mut self, fairing: Arc<dyn Fairing>) {
        let mut lock = self.rocket.lock().await;

        let r = lock.take().unwrap();
        let r = r.attach(fairing);
        *lock = Some(r);
    }

    async fn _register_liftoff_hook(&mut self, name: String, hook: BoxedLiftoffHook) {
        let hook_info = RegisteredHookInfo {
            plugin: self.plugin_name.clone(),
            name,
        };

        let mut lock = self.liftoff_hooks.lock().await;

        lock.push((hook_info, hook));
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PluginName(pub String);

impl fmt::Display for PluginName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Hash)]
pub struct PluginData {
    pub dependencies: Vec<PluginName>,
}

pub struct PluginRegistry {
    plugins: HashMap<PluginName, PluginData>,
}

impl PluginRegistry {
    pub fn new(plugins: HashMap<PluginName, PluginData>) -> Self {
        Self { plugins }
    }

    pub fn load_from_str(s: &str) -> Result<Self, PluginManagerError> {
        let plugins: HashMap<PluginName, PluginData> = serde_yaml::from_str(&s)?;

        Ok(Self::new(plugins))
    }

    pub async fn load_from_file(f: &Path) -> Result<Self, PluginManagerError> {
        let s = fs::read_to_string(f).await?;
        Self::load_from_str(&s)
    }

    pub async fn resolve_plugins_to_load(
        &self,
        plugins: Vec<PluginName>,
    ) -> Result<Vec<PluginName>, PluginManagerError> {
        let mut depgraph = solvent::DepGraph::new();

        for (name, data) in &self.plugins {
            let deps = data.dependencies.clone();
            depgraph.register_dependencies(name.clone(), deps);
        }

        let root = PluginName("__upsilon_plugin_root".to_string());

        depgraph.register_dependencies(root.clone(), plugins);

        let mut to_load = Vec::new();

        for plugin in depgraph.dependencies_of(&root)? {
            let plugin = plugin?;

            if *plugin == root {
                continue;
            }

            to_load.push(plugin.clone());
        }

        Ok(to_load)
    }
}

impl PluginManager {
    pub fn new(plugin_loader: Box<dyn PluginLoaderWrapper>, rocket: Rocket<Build>) -> Self {
        Self {
            plugin_loader,
            plugins: HashMap::new(),
            finished_loading: false,
            rocket: Some(rocket),
            liftoff_hooks: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn finish(mut self) -> Rocket<Build> {
        let mut rocket = self.rocket.take().unwrap();

        let hook_runner = PluginHookRunner {
            liftoff_hooks: self.liftoff_hooks.clone(),
        };

        if hook_runner.attach().await {
            rocket = rocket.attach(hook_runner);
        }

        rocket
    }

    pub async fn load_plugins(
        &mut self,
        registry: &PluginRegistry,
        plugins: &HashMap<PluginName, PluginConfig>,
    ) -> Result<(), PluginManagerError> {
        let p = plugins.keys().cloned().collect::<Vec<_>>();

        let plugins_to_load = registry.resolve_plugins_to_load(p).await?;

        let default_config = PluginConfig::default_for_deps();

        for plugin in plugins_to_load {
            let config = plugins.get(&plugin).unwrap_or(&default_config);

            let r = self.load_plugin(plugin.clone(), &config).await;

            if let Err(e) = r {
                return Err(PluginManagerError::LoadPluginError(plugin, Box::new(e)));
            }
        }

        Ok(())
    }

    pub async fn load_plugin(
        &mut self,
        name: PluginName,
        config: &PluginConfig,
    ) -> Result<(), PluginManagerError> {
        if self.finished_loading {
            return Err(PluginManagerError::PluginLoadingFinished);
        }

        let plugin = self.plugin_loader.load_plugin(&name.0, config)?;

        let plugin = match self.plugins.entry(name.0.clone()) {
            Entry::Occupied(_) => {
                return Err(PluginManagerError::PluginAlreadyLoaded);
            }
            Entry::Vacant(entry) => entry.insert(plugin),
        };

        let p = plugin.plugin();

        let mut mutator = PluginLoadApiImpl {
            plugin_name: name,
            rocket: Arc::new(Mutex::new(self.rocket.take())),
            liftoff_hooks: self.liftoff_hooks.clone(),
        };

        let r = p.init(&mut mutator).await;

        let PluginLoadApiImpl { rocket, .. } = mutator;

        self.rocket = Some(Arc::try_unwrap(rocket).unwrap().into_inner().unwrap());

        r?;

        Ok(())
    }

    pub fn get_plugin(&mut self, name: &str) -> Option<&mut dyn Plugin> {
        self.plugins.get_mut(name).map(|p| p.plugin())
    }
}

pub struct PluginHookRunner {
    liftoff_hooks: Arc<Mutex<Vec<(RegisteredHookInfo, BoxedLiftoffHook)>>>,
}

impl PluginHookRunner {
    async fn attach(&self) -> bool {
        let liftoff = !self.liftoff_hooks.lock().await.is_empty();

        liftoff
    }
}

#[rocket::async_trait]
impl Fairing for PluginHookRunner {
    fn info(&self) -> Info {
        Info {
            name: "Plugin Hook Runner",
            kind: Kind::Ignite | Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, rocket: &Rocket<Orbit>) {
        let mut hooks = self.liftoff_hooks.lock().await;

        for (hook_info, hook) in hooks.drain(..) {
            info!(
                "Running liftoff hook for plugin '{}' ({})",
                hook_info.plugin, hook_info.name
            );
            hook(rocket).await;
            info!(
                "Finished running liftoff hook for plugin '{}' ({})",
                hook_info.plugin, hook_info.name
            );
        }
    }
}
