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

use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use anyhow::Context;
use cargo_difftests::analysis::{file_is_from_cargo_registry, AnalysisConfig, AnalysisResult};
use cargo_difftests::index_data::IndexDataCompilerConfig;
use cargo_difftests::{DiscoveredDifftest, ExportProfdataConfig};
use clap::{Args, Parser, ValueEnum};
use log::warn;
use path_slash::PathExt;

#[derive(Args, Debug)]
pub struct ExportProfdataCommand {
    #[clap(
        long = "no-ignore-registry-files",
        default_value_t = true,
        action(clap::ArgAction::SetFalse)
    )]
    ignore_registry_files: bool,
    #[clap(long = "bin")]
    other_binaries: Vec<PathBuf>,
    #[clap(long)]
    force: bool,
}

#[derive(ValueEnum, Debug, Copy, Clone)]
pub enum FlattenFilesTarget {
    #[clap(name = "repo-root")]
    RepoRoot,
}

impl Display for FlattenFilesTarget {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FlattenFilesTarget::RepoRoot => write!(f, "repo-root"),
        }
    }
}

#[derive(Args, Debug, Copy, Clone)]
pub struct CompileTestIndexFlags {
    #[clap(long)]
    ignore_cargo_registry: bool,
    #[clap(long)]
    flatten_files_to: Option<FlattenFilesTarget>,
    #[cfg(windows)]
    #[clap(
        long = "no-path-slash-replace",
        default_value_t = true,
        action(clap::ArgAction::SetFalse)
    )]
    path_slash_replace: bool,
}

#[derive(Parser, Debug)]
pub enum LowLevelCommand {
    MergeProfdata {
        #[clap(long)]
        dir: PathBuf,
        #[clap(long)]
        force: bool,
    },
    ExportProfdata {
        #[clap(long)]
        dir: PathBuf,
        #[clap(flatten)]
        cmd: ExportProfdataCommand,
    },
    RunAnalysis {
        #[clap(long)]
        dir: PathBuf,
        #[clap(long, default_value_t = Default::default())]
        algo: DirtyAlgorithm,
    },
    CompileTestIndex {
        #[clap(long)]
        dir: PathBuf,
        #[clap(short, long)]
        output: PathBuf,
        #[clap(flatten)]
        compile_test_index_flags: CompileTestIndexFlags,
    },
    RunAnalysisWithTestIndex {
        #[clap(long)]
        index: PathBuf,
        #[clap(long, default_value_t = Default::default())]
        algo: DirtyAlgorithm,
    },
}

#[derive(ValueEnum, Debug, Copy, Clone, Default)]
pub enum DirtyAlgorithm {
    #[default]
    #[clap(name = "fs-mtime")]
    FsMtime,
    #[clap(name = "git-diff")]
    GitDiff,
}

impl From<DirtyAlgorithm> for cargo_difftests::analysis::DirtyAlgorithm {
    fn from(algo: DirtyAlgorithm) -> Self {
        match algo {
            DirtyAlgorithm::FsMtime => Self::FileSystemMtimes,
            DirtyAlgorithm::GitDiff => Self::GitDiff,
        }
    }
}

impl Display for DirtyAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DirtyAlgorithm::FsMtime => write!(f, "fs-mtime"),
            DirtyAlgorithm::GitDiff => write!(f, "git-diff"),
        }
    }
}

#[derive(Parser, Debug)]
pub enum App {
    DiscoverDifftests {
        #[clap(long, default_value = "target/tmp/cargo-difftests")]
        dir: PathBuf,
    },
    Analyze {
        #[clap(long)]
        dir: PathBuf,
        #[clap(long)]
        force: bool,
        #[clap(long, default_value_t = Default::default())]
        algo: DirtyAlgorithm,
        #[clap(long = "bin")]
        other_binaries: Vec<PathBuf>,
    },
    AnalyzeAll {
        #[clap(long, default_value = "target/tmp/cargo-difftests")]
        dir: PathBuf,
        #[clap(long)]
        force: bool,
        #[clap(long, default_value_t = Default::default())]
        algo: DirtyAlgorithm,
        #[clap(long = "bin")]
        other_binaries: Vec<PathBuf>,
    },
    LowLevel {
        #[clap(subcommand)]
        cmd: LowLevelCommand,
    },
}

pub type CargoDifftestsResult<T = ()> = anyhow::Result<T>;

fn discover_difftests(dir: PathBuf) -> CargoDifftestsResult<Vec<DiscoveredDifftest>> {
    if !dir.exists() || !dir.is_dir() {
        warn!("Directory {} does not exist", dir.display());
        return Ok(vec![]);
    }

    let discovered = cargo_difftests::discover_difftests(&dir)?;

    Ok(discovered)
}

fn run_discover_difftests(dir: PathBuf) -> CargoDifftestsResult {
    let discovered = discover_difftests(dir)?;
    let s = serde_json::to_string(&discovered)?;
    println!("{s}");

    Ok(())
}

fn run_merge_profdata(dir: PathBuf, force: bool) -> CargoDifftestsResult {
    let mut discovered = DiscoveredDifftest::discover_from(dir)?;

    discovered.merge_profraw_files_into_profdata(force)?;

    Ok(())
}

fn run_export_profdata(dir: PathBuf, cmd: ExportProfdataCommand) -> CargoDifftestsResult {
    let mut discovered = DiscoveredDifftest::discover_from(dir)?;

    let mut has_profdata = discovered.assert_has_profdata();
    has_profdata.export_profdata_file(ExportProfdataConfig {
        force: cmd.force,
        ignore_registry_files: cmd.ignore_registry_files,
        other_binaries: cmd.other_binaries,
        test_desc: None, // will read from `self.json`
    })?;

    Ok(())
}

fn display_analysis_result(r: AnalysisResult) {
    let res = match r {
        AnalysisResult::Clean => "clean",
        AnalysisResult::Dirty => "dirty",
    };

    println!("{res}");
}

fn run_analysis(dir: PathBuf, algo: DirtyAlgorithm) -> CargoDifftestsResult {
    let mut discovered = DiscoveredDifftest::discover_from(dir)?;
    let mut analysis_cx = discovered.assert_has_exported_profdata().start_analysis()?;

    analysis_cx.run(&AnalysisConfig {
        dirty_algorithm: algo.into(),
    })?;

    let r = analysis_cx.finish_analysis();

    display_analysis_result(r);

    Ok(())
}

fn run_analysis_with_test_index(
    index: PathBuf,
    dirty_algorithm: DirtyAlgorithm,
) -> CargoDifftestsResult {
    let mut analysis_cx = cargo_difftests::analysis::AnalysisContext::with_index_from(&index)?;

    analysis_cx.run(&AnalysisConfig {
        dirty_algorithm: dirty_algorithm.into(),
    })?;

    let r = analysis_cx.finish_analysis();

    display_analysis_result(r);

    Ok(())
}

fn run_compile_test_index(
    dir: PathBuf,
    output: PathBuf,
    compile_test_index_flags: CompileTestIndexFlags,
) -> CargoDifftestsResult {
    let mut discovered = DiscoveredDifftest::discover_from(dir)?;
    let exported_profdata = discovered.assert_has_exported_profdata();

    let flatten_root = match compile_test_index_flags.flatten_files_to {
        Some(FlattenFilesTarget::RepoRoot) => {
            let repo = git2::Repository::open_from_env()?;
            let root = repo.workdir().context("repo has no workdir")?;
            Some(root.to_path_buf())
        }
        None => None,
    };

    let config = IndexDataCompilerConfig {
        index_filename_converter: Box::new(move |path| {
            let p = match &flatten_root {
                Some(root) => path.strip_prefix(root).unwrap_or(path),
                None => path,
            };

            #[cfg(windows)]
            let p = if compile_test_index_flags.path_slash_replace {
                PathBuf::from(p.to_slash().unwrap().into_owned())
            } else {
                p.to_path_buf()
            };

            #[cfg(not(windows))]
            let p = p.to_path_buf();

            p
        }),
        accept_file: Box::new(move |path| {
            if compile_test_index_flags.ignore_cargo_registry && file_is_from_cargo_registry(path) {
                return false;
            }

            true
        }),
    };

    let result = exported_profdata.compile_test_index_data(config)?;

    result.write_to_file(&output)?;

    Ok(())
}

fn run_low_level_cmd(cmd: LowLevelCommand) -> CargoDifftestsResult {
    match cmd {
        LowLevelCommand::MergeProfdata { dir, force } => {
            run_merge_profdata(dir, force)?;
        }
        LowLevelCommand::ExportProfdata { dir, cmd } => {
            run_export_profdata(dir, cmd)?;
        }
        LowLevelCommand::RunAnalysis { dir, algo } => {
            run_analysis(dir, algo)?;
        }
        LowLevelCommand::CompileTestIndex {
            dir,
            output,
            compile_test_index_flags,
        } => {
            run_compile_test_index(dir, output, compile_test_index_flags)?;
        }
        LowLevelCommand::RunAnalysisWithTestIndex { index, algo } => {
            run_analysis_with_test_index(index, algo)?;
        }
    }

    Ok(())
}

fn run_analyze(
    dir: PathBuf,
    force: bool,
    algo: DirtyAlgorithm,
    bins: Vec<PathBuf>,
) -> CargoDifftestsResult {
    let mut discovered = DiscoveredDifftest::discover_from(dir)?;

    let mut has_profdata = discovered.merge_profraw_files_into_profdata(force)?;
    let has_exported_profdata = has_profdata.export_profdata_file(ExportProfdataConfig {
        force,
        ignore_registry_files: true,
        other_binaries: bins,
        test_desc: None, // will read from `self.json`
    })?;

    let mut analysis_cx = has_exported_profdata.start_analysis()?;

    analysis_cx.run(&AnalysisConfig {
        dirty_algorithm: algo.into(),
    })?;

    let r = analysis_cx.finish_analysis();

    display_analysis_result(r);

    Ok(())
}

#[derive(serde::Serialize)]
pub struct AnalyzeAllSingleTest {
    #[serde(flatten)]
    discovered: DiscoveredDifftest,
    verdict: AnalysisVerdict,
}

#[derive(serde::Serialize, Copy, Clone)]
pub enum AnalysisVerdict {
    #[serde(rename = "clean")]
    Clean,
    #[serde(rename = "dirty")]
    Dirty,
}

impl From<AnalysisResult> for AnalysisVerdict {
    fn from(r: AnalysisResult) -> Self {
        match r {
            AnalysisResult::Clean => AnalysisVerdict::Clean,
            AnalysisResult::Dirty => AnalysisVerdict::Dirty,
        }
    }
}

pub fn run_analyze_all(
    dir: PathBuf,
    force: bool,
    algo: DirtyAlgorithm,
    bins: Vec<PathBuf>,
) -> CargoDifftestsResult {
    let discovered = discover_difftests(dir)?;

    let mut results = vec![];

    for mut discovered in discovered {
        let mut has_profdata = discovered.merge_profraw_files_into_profdata(force)?;
        let has_exported_profdata = has_profdata.export_profdata_file(ExportProfdataConfig {
            force,
            ignore_registry_files: true,
            other_binaries: bins.clone(),
            test_desc: None, // will read from `self.json`
        })?;

        let mut analysis_cx = has_exported_profdata.start_analysis()?;

        analysis_cx.run(&AnalysisConfig {
            dirty_algorithm: algo.into(),
        })?;

        let r = analysis_cx.finish_analysis();

        let result = AnalyzeAllSingleTest {
            discovered,
            verdict: r.into(),
        };

        results.push(result);
    }

    let out_json = serde_json::to_string(&results)?;
    println!("{out_json}");

    Ok(())
}

fn main_impl() -> CargoDifftestsResult {
    pretty_env_logger::init_custom_env("CARGO_DIFFTESTS_LOG");
    let app = App::parse();

    match app {
        App::DiscoverDifftests { dir } => {
            run_discover_difftests(dir)?;
        }
        App::Analyze {
            dir,
            force,
            algo,
            other_binaries,
        } => {
            run_analyze(dir, force, algo, other_binaries)?;
        }
        App::AnalyzeAll {
            dir,
            force,
            algo,
            other_binaries,
        } => {
            run_analyze_all(dir, force, algo, other_binaries)?;
        }
        App::LowLevel { cmd } => {
            run_low_level_cmd(cmd)?;
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = main_impl() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
