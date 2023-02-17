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

use anyhow::bail;
use cargo::core::Workspace;
use cargo::util::command_prelude::ArgMatchesExt;
use clap::ArgMatches;

use crate::{ws_root, XtaskResult};

pub fn cargo_config() -> XtaskResult<cargo::Config> {
    let dir = ws_root!();

    // redirect cargo output into the void
    let cursor = std::io::Cursor::new(Vec::<u8>::new());
    let shell = cargo::core::Shell::from_write(Box::new(cursor));
    let Some(home_dir) = cargo::util::homedir(&dir) else {
        bail!("Could not find home directory");
    };

    let mut cargo_config = cargo::Config::new(shell, dir, home_dir);
    cargo_config.configure(0, false, None, false, false, false, &None, &[], &[])?;
    Ok(cargo_config)
}

pub fn cargo_ws(cargo_config: &cargo::Config) -> XtaskResult<Workspace> {
    let arg_matches = ArgMatches::default();
    let ws = arg_matches.workspace(cargo_config)?;
    Ok(ws)
}
