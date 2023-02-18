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

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserConfig {
    pub permissions: UserPermissions,
}

impl UserConfig {
    pub fn from_env() -> Self {
        let config = std::env::var(ENV_VAR_USER_CONFIG).expect("UPSILON_USER_CONFIG not set");
        serde_json::from_str(&config).expect("Cannot parse UPSILON_USER_CONFIG")
    }

    pub fn serialized(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize UserConfig")
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserPermissions {
    #[serde(rename = "r")]
    pub has_read: bool,
    #[serde(rename = "w")]
    pub has_write: bool,
    #[serde(rename = "a")]
    pub has_admin: bool,
}

pub const ENV_VAR_USER_CONFIG: &str = "UPSILON_USER_CONFIG";