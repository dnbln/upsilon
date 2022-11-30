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

use std::path::PathBuf;

use crate::cmd::{cargo_cmd, cmd, cmd_call};
use crate::result::XtaskResult;
use crate::ws::ws_path_join;

pub fn gen_models(target: PathBuf) -> XtaskResult<()> {
    let target_lib_dir = ws_path_join!(target / "lib");

    let models_dir = ws_path_join!(
        #[clone]
        target_lib_dir
            / "models"
    );
    let target_file = ws_path_join!(target_lib_dir / "models" / "models.g.dart");

    std::fs::create_dir_all(&models_dir)?;

    cargo_cmd!(
        "run",
        "-p",
        "upsilon-api-models-protobuf",
        "--bin",
        "gen_models",
        "--",
        &target_file
    )?;

    cmd_call!("dart", "format", &target_file)?;

    Ok(())
}
