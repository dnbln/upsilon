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

use upsilon_plugin_manager::{PluginData, PluginName, PluginRegistry, StaticPluginLoader};

pub fn static_plugins() -> (PluginRegistry, StaticPluginLoader) {
    let registry = PluginRegistry::new(HashMap::from([(
        PluginName("upsilon-debug-data-driver".to_string()),
        PluginData {
            dependencies: vec![],
        },
    )]));

    macro_rules! crate_plugin {
        ($krate:ident) => {
            ($krate::__UPSILON_METADATA, $krate::__UPSILON_PLUGIN)
        };
    }

    let loader = StaticPluginLoader::new([
        crate_plugin!(upsilon_debug_data_driver),
        crate_plugin!(upsilon_portfile_writer),
    ]);

    (registry, loader)
}
