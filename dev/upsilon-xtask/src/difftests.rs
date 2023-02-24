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

use std::ffi::OsStr;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use cargo_difftests::{AnalysisVerdict, AnalyzeAllSingleTest};
use clap::{Parser, ValueEnum};
use upsilon_xtask::pkg::Profile;

use crate::ws_layout::WS_BIN_LAYOUT;
use crate::{ws_path, XtaskResult};

#[derive(Parser, Debug)]
pub enum DiffTestsCommand {
    #[clap(name = "print-tests-to-rerun")]
    PrintTestsToRerun {
        #[clap(long, default_value_t = Default::default())]
        algo: DirtyAlgo,
        #[clap(long)]
        commit: Option<git2::Oid>,
        #[clap(long, default_value_t = Profile::Difftests)]
        profile: Profile,
    },
}

macro_rules! difftests_cmd {
    ($($args:tt)*) => {
        $crate::cargo_cmd!(
            "run",
            "-p", "cargo-difftests",
            "--bin", "cargo-difftests",
            "--",
            "difftests",
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
            "difftests",
            $($args)*
        )
    };
}

#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum DirtyAlgo {
    #[default]
    #[clap(name = "fs-mtime")]
    FsMtime,
    #[clap(name = "git-diff-files")]
    GitDiffFiles,
    #[clap(name = "git-diff-hunks")]
    GitDiffHunks,
}

impl Display for DirtyAlgo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DirtyAlgo::FsMtime => write!(f, "fs-mtime"),
            DirtyAlgo::GitDiffFiles => write!(f, "git-diff-files"),
            DirtyAlgo::GitDiffHunks => write!(f, "git-diff-hunks"),
        }
    }
}

fn index_root() -> PathBuf {
    ws_path!("tests" / "difftests-index-root")
}

fn other_bins(profile: Profile) -> Vec<PathBuf> {
    vec![
        WS_BIN_LAYOUT.upsilon_web_main.path_in_profile(profile),
        WS_BIN_LAYOUT
            .upsilon_gracefully_shutdown_host_main
            .path_in_profile(profile),
    ]
}

fn analyze_all(
    algo: DirtyAlgo,
    commit: Option<git2::Oid>,
    profile: Profile,
) -> XtaskResult<Vec<AnalyzeAllSingleTest>> {
    let output = difftests_cmd_output!(
        "analyze-all",
        "--dir",
        ws_path!("target" / "tmp" / "upsilon-difftests"),
        ...other_bins(profile).iter().flat_map(|p| [OsStr::new("--bin"), p.as_os_str()]),
        "--index-root",
        index_root(),
        "--index-strategy",
        "always-and-clean",
        "--algo",
        algo.to_string(),
        ...["--commit", &commit.to_string()] => @if let Some(commit) = commit,
    )?;

    let tests = serde_json::from_str::<Vec<AnalyzeAllSingleTest>>(&output)?;

    Ok(tests)
}

fn analyze_all_from_index(
    algo: DirtyAlgo,
    commit: Option<git2::Oid>,
) -> XtaskResult<Vec<AnalyzeAllSingleTest>> {
    let output = difftests_cmd_output!(
        "analyze-all-from-index",
        "--index-root",
        index_root(),
        "--algo",
        algo.to_string(),
        ...["--commit", &commit.to_string()] => @if let Some(commit) = commit,
    )?;

    let tests = serde_json::from_str::<Vec<AnalyzeAllSingleTest>>(&output)?;

    Ok(tests)
}

pub fn tests_to_rerun(
    algo: DirtyAlgo,
    commit: Option<git2::Oid>,
    profile: Profile,
) -> XtaskResult<Vec<AnalyzeAllSingleTest>> {
    Ok(analyze_all(algo, commit, profile)?
        .into_iter()
        .filter(|it| it.verdict == AnalysisVerdict::Dirty)
        .collect())
}

pub fn tests_to_rerun_from_index(
    algo: DirtyAlgo,
    commit: Option<git2::Oid>,
) -> XtaskResult<Vec<AnalyzeAllSingleTest>> {
    Ok(analyze_all_from_index(algo, commit)?
        .into_iter()
        .filter(|it| it.verdict == AnalysisVerdict::Dirty)
        .collect())
}

fn print_tests_to_rerun(
    algo: DirtyAlgo,
    commit: Option<git2::Oid>,
    profile: Profile,
) -> XtaskResult<()> {
    let to_rerun = tests_to_rerun(algo, commit, profile)?
        .into_iter()
        .map(|it| it.test_desc)
        .collect::<Vec<_>>();

    let s = serde_json::to_string(&to_rerun)?;
    println!("{s}");

    Ok(())
}

pub fn run(command: DiffTestsCommand) -> XtaskResult<()> {
    match command {
        DiffTestsCommand::PrintTestsToRerun {
            algo,
            commit,
            profile,
        } => {
            print_tests_to_rerun(algo, commit, profile)?;
        }
    }

    Ok(())
}
