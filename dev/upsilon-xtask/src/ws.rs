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

use std::path::Path;

#[macro_export]
macro_rules! ws_path {
    ($($s:tt)/ * ) => {
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

#[macro_export]
macro_rules! ws_root {
    () => {{
        use std::borrow::ToOwned;
        $crate::ws::workspace_root().to_owned()
    }};
}

#[macro_export]
macro_rules! ws_path_str {
    ($($s:tt)*) => {
        $crate::ws::ws_path!($($s)/ *).to_str().unwrap().to_string()
    }
}

#[macro_export]
macro_rules! ws_path_join {
    (#[clone] $root:ident / $($s:tt)/ *) => {
        {
            let mut p = $root.clone();
            $(
                p.push($s);
            )*
            p
        }
    };
    ($root:ident / $($s:tt)/ *) => {
        {
            let mut p = $root;
            $(
                p.push($s);
            )*
            p
        }
    };
}

#[macro_export]
macro_rules! ws_bin_path {
    (profile = $profile:tt, name = $name:tt) => {{
        let mut p = $crate::ws_path!("target" / $profile / $name);
        p.set_extension(std::env::consts::EXE_EXTENSION);
        p
    }};
}

#[macro_export]
macro_rules! ws_glob {
    ($($p:tt)/ *) => {
        (|| -> $crate::result::XtaskResult<Vec<_>> {
            let ws_path = $crate::ws_path!($($p)/ *);
            let ws_path_str = ws_path.to_str().ok_or_else(|| format_err!("invalid path: {:?}", ws_path))?;
            let paths = $crate::glob::glob(ws_path_str)?;
            let paths = paths.collect::<Result<Vec<_>, _>>()?;
            Ok(paths)
        })()
    };
}

pub fn workspace_root() -> &'static Path {
    let xtask_dir: &Path = env!("CARGO_MANIFEST_DIR").as_ref();
    // parent of upsilon-xtask = dev,
    // parent of crates = workspace root
    xtask_dir.parent().unwrap().parent().unwrap()
}
