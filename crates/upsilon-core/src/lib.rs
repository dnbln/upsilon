/*
 *        Copyright (c) 2022-2023 Dinu Blanovschi
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

use std::path::{Path, PathBuf};

pub mod config;

fn bin_folder() -> PathBuf {
    if let Ok(var) = std::env::var("UPSILON_BIN_DIR") {
        return PathBuf::from(var);
    }

    let mut path = std::env::current_exe().unwrap();
    path.pop();
    path
}

pub fn upsilon_exe() -> PathBuf {
    alt_exe("upsilon")
}

pub fn alt_exe(name: impl AsRef<Path>) -> PathBuf {
    let mut path = bin_folder();
    path.push(name);
    path.set_extension(std::env::consts::EXE_EXTENSION);
    path
}
