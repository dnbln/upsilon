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

use std::path::PathBuf;

use anyhow::bail;
use ra_ap_project_model::{ProjectWorkspace, TargetKind};
use upsilon_difftests::DifftestsResult;

fn main_impl() -> DifftestsResult {
    let cargo_config = upsilon_difftests::cargo_config();

    let ws = upsilon_difftests::load_project_workspace(
        PathBuf::from("C:/Users/Dinu/proj/upsilon"),
        &cargo_config,
        &|progress| {
            eprintln!("Progress: {progress}");
        },
    )?;

    let ws = match ws {
        ProjectWorkspace::Cargo { cargo, .. } => cargo,
        ProjectWorkspace::Json { .. } => {
            bail!("Json workspace not supported")
        }
        ProjectWorkspace::DetachedFiles { .. } => {
            bail!("Detached files workspace not supported")
        }
    };

    let packages = ["upsilon-testsuite", "upsilon-shell"];

    for p in ws.packages()
        .map(|it| &ws[it])
        .filter(|it| packages.iter().any(|p| p == &it.name)) {
        println!("P: {}", p.name);
        dbg!(&p);
        for target in &p.targets {
            let t = &ws[*target];
            dbg!(&t);
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = main_impl() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
