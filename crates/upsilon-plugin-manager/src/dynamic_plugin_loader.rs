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

use std::sync::Arc;

use libloading::{Library, Symbol};
use upsilon_plugin_core::{Plugin, PluginConfig, PluginMetadata};

use crate::{
    check_version, ApiVersionMismatchError, PluginHolder, PluginLoader, PluginManagerError
};

pub struct DynamicPluginLoader {
    plugin_dir: std::path::PathBuf,
}

#[derive(Debug, thiserror::Error)]
pub enum DynamicPluginLoaderError {
    #[error("loading library error: {0}")]
    LoadingLibraryError(#[from] libloading::Error),
    #[error("Unknown plugin")]
    UnknownPlugin,
    #[error("Plugin API version mismatch")]
    PluginApiVersionMismatch(#[from] ApiVersionMismatchError),
    #[error("Plugin error: {0}")]
    PluginError(#[from] upsilon_plugin_core::PluginError),
}

impl From<DynamicPluginLoaderError> for PluginManagerError {
    fn from(value: DynamicPluginLoaderError) -> Self {
        match value {
            DynamicPluginLoaderError::LoadingLibraryError(e) => {
                PluginManagerError::LoadingLibraryError(e)
            }
            DynamicPluginLoaderError::UnknownPlugin => PluginManagerError::UnknownPlugin,
            DynamicPluginLoaderError::PluginApiVersionMismatch(e) => {
                PluginManagerError::PluginApiVersionMismatch(e)
            }
            DynamicPluginLoaderError::PluginError(e) => PluginManagerError::PluginError(e),
        }
    }
}

pub struct DynamicPluginHolder {
    plugin: Box<dyn Plugin>,
    lib: Arc<Library>,
}

impl PluginHolder for DynamicPluginHolder {
    fn plugin(&mut self) -> &mut dyn Plugin {
        &mut *self.plugin
    }
}

impl PluginLoader for DynamicPluginLoader {
    type Error = DynamicPluginLoaderError;
    type Holder = DynamicPluginHolder;

    fn load_plugin(&self, name: &str, config: &PluginConfig) -> Result<Self::Holder, Self::Error> {
        let plugin_lib_name = libloading::library_filename(name);

        let plugin_lib_path = self.plugin_dir.join(plugin_lib_name);

        let plugin_lib = unsafe { Library::new(plugin_lib_path)? };
        let plugin_lib = Arc::new(plugin_lib);

        let symbol: Symbol<extern "C" fn() -> PluginMetadata> =
            unsafe { plugin_lib.get(b"__upsilon_plugin")? };

        let metadata = symbol();

        check_version(&metadata)?;

        let create_fn = metadata.create;

        let plugin = create_fn(config.clone())?;

        Ok(DynamicPluginHolder {
            plugin,
            lib: plugin_lib,
        })
    }
}
