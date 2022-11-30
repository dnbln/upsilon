/*
 *        Copyright (c) 2022 Dinu Blanovschi
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

use std::path::Path;
macro_rules! ws_path {
    ($($s:literal)/ * ) => {
        {
            use std::borrow::ToOwned;
            let mut p = $crate::ws::workspace_root().to_owned();
            $(
                p.push($s);
            )*
            p
        }
    }
}

macro_rules! ws_path_str {
    ($($s:literal)/ * ) => {
        $crate::ws::ws_path!($($s)/ *).to_str().unwrap().to_string()
    }
}

macro_rules! ws_path_join {
    (#[clone] $root:ident / $($s:literal)/ *) => {
        {
            let mut p = $root.clone();
            $(
                p.push($s);
            )*
            p
        }
    };
    ($root:ident / $($s:literal)/ *) => {
        {
            let mut p = $root;
            $(
                p.push($s);
            )*
            p
        }
    };
}

pub fn workspace_root() -> &'static Path {
    let xtask_dir: &Path = env!("CARGO_MANIFEST_DIR").as_ref();
    // parent of upsilon-xtask = crates,
    // parent of crates = workspace root
    xtask_dir.parent().unwrap().parent().unwrap()
}

pub(crate) use {ws_path, ws_path_join, ws_path_str};
