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
use std::path::PathBuf;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CoreTestDesc {
    pub pkg_name: String,
    pub crate_name: String,
    pub bin_name: Option<String>,
    pub bin_path: PathBuf,
    pub test_name: String,

    pub other_fields: HashMap<String, String>,
}

pub const CARGO_DIFFTESTS_VERSION_FILENAME: &str = "cargo_difftests_version";
pub const CARGO_DIFFTESTS_SELF_JSON_FILENAME: &str = "self.json";
pub const CARGO_DIFFTESTS_SELF_PROFILE_FILENAME: &str = "self.profraw";
pub const CARGO_DIFFTESTS_OTHER_PROFILE_FILENAME_TEMPLATE: &str = "%m_%p.profraw";
