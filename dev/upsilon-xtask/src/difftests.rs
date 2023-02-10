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

use cargo_difftests::{AnalysisVerdict, AnalyzeAllSingleTest};
use clap::Parser;

use crate::{
    cmd_call, cmd_output_pipe_to_file, cmd_output_string, cmd_process, ws_bin_path, ws_path, XtaskResult
};

#[derive(Parser, Debug)]
pub enum DiffTestsCommand {
    #[clap(name = "print-tests-to-rerun")]
    PrintTestsToRerun,
}

macro_rules! difftests_cmd {
    ($($args:tt)*) => {
        $crate::cargo_cmd!(
            "run",
            "-p", "cargo-difftests",
            "--bin", "cargo-difftests",
            "--",
            $($args)*
        )
    };
}

macro_rules! difftests_cmd_output {
    ($($args:tt)*) => {
        $crate::cargo_cmd_output!(
            "run",
            "-p", "cargo-difftests",
            "--bin", "cargo-difftests",
            "--",
            $($args)*
        )
    };
}

fn analyze_all() -> XtaskResult<Vec<AnalyzeAllSingleTest>> {
    let output = difftests_cmd_output!(
        "analyze-all",
        "--dir",
        ws_path!("target" / "tmp" / "upsilon-difftests"),
        "--bin",
        ws_bin_path!(profile = "difftests", name = "upsilon-web"),
        "--bin",
        ws_bin_path!(
            profile = "difftests",
            name = "upsilon-gracefully-shutdown-host"
        ),
        "--index-root",
        ws_path!("tests" / "difftests-index-root"),
        "--index-strategy",
        "always",
    )?;

    let tests = serde_json::from_str::<Vec<AnalyzeAllSingleTest>>(&output)?;

    Ok(tests)
}

fn tests_to_rerun() -> XtaskResult<Vec<AnalyzeAllSingleTest>> {
    Ok(analyze_all()?
        .into_iter()
        .filter(|it| it.verdict == AnalysisVerdict::Dirty)
        .collect())
}

fn print_tests_to_rerun() -> XtaskResult<()> {
    let to_rerun = tests_to_rerun()?
        .into_iter()
        .map(|it| it.test_desc)
        .collect::<Vec<_>>();

    let s = serde_json::to_string(&to_rerun)?;
    println!("{s}");

    Ok(())
}

pub fn run(command: DiffTestsCommand) -> XtaskResult<()> {
    match command {
        DiffTestsCommand::PrintTestsToRerun => {
            print_tests_to_rerun()?;
        }
    }

    Ok(())
}
