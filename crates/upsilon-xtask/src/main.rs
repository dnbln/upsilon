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

#![feature(try_blocks)]

use std::path::PathBuf;

use clap::Parser;

use crate::cmd::cargo_cmd;
use crate::result::XtaskResult;

mod cmd;
mod gen_models;
mod result;
mod ws;
mod git_checks;

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "gen-models")]
    GenModels {
        #[arg(short, long)]
        #[clap(default_value_os_t = ws::ws_path!("client-app" / "upsilon_client"))]
        target: PathBuf,
    },
    #[clap(name = "fmt")]
    Fmt,
    #[clap(name = "fmt-check")]
    FmtCheck,
    #[clap(name = "git-checks")]
    GitChecks,
}

fn main() -> XtaskResult<()> {
    let app: App = App::parse();

    match app {
        App::GenModels { target } => {
            gen_models::gen_models(target)?;
        }
        App::Fmt => {
            cargo_cmd!("fmt", "--all")?;
        }
        App::FmtCheck => {
            cargo_cmd!("fmt", "--all", "--check")?;
        }
        App::GitChecks => {
            let repo = git_checks::get_repo()?;

            git_checks::linear_history(&repo)?;
        }
    }

    Ok(())
}
