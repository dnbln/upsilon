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

use clap::Parser;

use crate::cmd::cargo_cmd;
use crate::result::XtaskResult;

mod cmd;
mod git_checks;
mod result;
mod ws;

#[derive(Parser, Debug)]
enum App {
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
