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
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use cargo_difftests::analysis::{
    file_is_from_cargo_registry, AnalysisConfig, AnalysisContext, AnalysisResult, GitDiffStrategy
};
use cargo_difftests::index_data::{DifftestsSingleTestIndexData, IndexDataCompilerConfig};
use cargo_difftests::{
    AnalyzeAllSingleTest, Difftest, DiscoverIndexPathResolver, ExportProfdataConfig, IndexCompareDifferences, TouchSameFilesDifference
};
use cargo_difftests_core::CoreTestDesc;
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
    #[clap(
        long = "no-ignore-cargo-registry",
        default_value_t = true,
        action(clap::ArgAction::SetFalse)
    )]
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

impl Default for CompileTestIndexFlags {
    fn default() -> Self {
        Self {
            ignore_cargo_registry: true,
            flatten_files_to: Some(FlattenFilesTarget::RepoRoot),
            #[cfg(windows)]
            path_slash_replace: true,
        }
    }
}

#[derive(ValueEnum, Debug, Copy, Clone, Default)]
pub enum AnalysisIndexStrategy {
    #[clap(name = "always")]
    Always,
    #[clap(name = "if-available")]
    IfAvailable,
    #[default]
    #[clap(name = "never")]
    Never,
}

impl Display for AnalysisIndexStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisIndexStrategy::Always => write!(f, "always"),
            AnalysisIndexStrategy::IfAvailable => write!(f, "if-available"),
            AnalysisIndexStrategy::Never => write!(f, "never"),
        }
    }
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
        #[clap(long)]
        commit: Option<git2::Oid>,
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
        #[clap(long)]
        commit: Option<git2::Oid>,
    },
    IndexesTouchSameFilesReport {
        index1: PathBuf,
        index2: PathBuf,
        #[clap(long, default_value_t = Default::default())]
        action: IndexesTouchSameFilesReportAction,
    },
}

#[derive(ValueEnum, Debug, Copy, Clone, Default)]
pub enum IndexesTouchSameFilesReportAction {
    #[default]
    #[clap(name = "print")]
    Print,
    #[clap(name = "assert")]
    Assert,
}

impl IndexesTouchSameFilesReportAction {
    fn do_for_report(
        &self,
        report: Result<(), IndexCompareDifferences<TouchSameFilesDifference>>,
    ) -> CargoDifftestsResult {
        match self {
            IndexesTouchSameFilesReportAction::Print => match report {
                Ok(()) => {
                    println!("[]");

                    Ok(())
                }
                Err(diffs) => {
                    let s = serde_json::to_string(diffs.differences())?;

                    println!("{s}");

                    Ok(())
                }
            },
            IndexesTouchSameFilesReportAction::Assert => match report {
                Ok(()) => Ok(()),
                Err(e) => {
                    let s = serde_json::to_string(e.differences())?;

                    eprintln!("{s}");

                    bail!("indexes do not touch the same files")
                }
            },
        }
    }
}

impl Display for IndexesTouchSameFilesReportAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexesTouchSameFilesReportAction::Print => write!(f, "print"),
            IndexesTouchSameFilesReportAction::Assert => write!(f, "assert"),
        }
    }
}

#[derive(ValueEnum, Debug, Copy, Clone, Default)]
pub enum DirtyAlgorithm {
    #[default]
    #[clap(name = "fs-mtime")]
    FsMtime,
    #[clap(name = "git-diff-files")]
    GitDiffFiles,
    #[clap(name = "git-diff-hunks")]
    GitDiffHunks,
}

impl DirtyAlgorithm {
    fn convert(self, commit: Option<git2::Oid>) -> cargo_difftests::analysis::DirtyAlgorithm {
        match self {
            DirtyAlgorithm::FsMtime => cargo_difftests::analysis::DirtyAlgorithm::FileSystemMtimes,
            DirtyAlgorithm::GitDiffFiles => cargo_difftests::analysis::DirtyAlgorithm::GitDiff {
                strategy: GitDiffStrategy::FilesOnly,
                commit,
            },
            DirtyAlgorithm::GitDiffHunks => cargo_difftests::analysis::DirtyAlgorithm::GitDiff {
                strategy: GitDiffStrategy::Hunks,
                commit,
            },
        }
    }
}

impl Display for DirtyAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            DirtyAlgorithm::FsMtime => write!(f, "fs-mtime"),
            DirtyAlgorithm::GitDiffFiles => write!(f, "git-diff-files"),
            DirtyAlgorithm::GitDiffHunks => write!(f, "git-diff-hunks"),
        }
    }
}

#[derive(Args, Debug)]
pub struct AnalysisIndex {
    #[clap(
        long,
        required_if_eq_any = [
            ("index_strategy", "always"),
            ("index_strategy", "if-available"),
        ]
    )]
    index_root: Option<PathBuf>,
    #[clap(long, default_value_t = Default::default())]
    index_strategy: AnalysisIndexStrategy,
}

#[derive(thiserror::Error, Debug)]
pub enum IndexResolverError {
    #[error("--root was not provided, but was required by the --index-strategy")]
    RootIsNone,
}

impl AnalysisIndex {
    fn index_resolver(
        &self,
        root: Option<PathBuf>,
    ) -> Result<Option<DiscoverIndexPathResolver>, IndexResolverError> {
        match self.index_strategy {
            AnalysisIndexStrategy::Always => {
                let index_root = self.index_root.as_ref().unwrap(); // should be set by clap

                Ok(Some(DiscoverIndexPathResolver::Remap {
                    from: root.ok_or(IndexResolverError::RootIsNone)?,
                    to: index_root.clone(),
                }))
            }
            AnalysisIndexStrategy::IfAvailable => {
                let index_root = self.index_root.as_ref().unwrap(); // should be set by clap

                Ok(Some(DiscoverIndexPathResolver::Remap {
                    from: root.ok_or(IndexResolverError::RootIsNone)?,
                    to: index_root.clone(),
                }))
            }
            AnalysisIndexStrategy::Never => Ok(None),
        }
    }
}

#[derive(Parser, Debug)]
pub enum App {
    DiscoverDifftests {
        #[clap(long, default_value = "target/tmp/cargo-difftests")]
        dir: PathBuf,
        #[clap(long)]
        index_root: Option<PathBuf>,
        #[clap(long)]
        ignore_incompatible: bool,
    },
    Analyze {
        #[clap(long)]
        dir: PathBuf,
        #[clap(long)]
        force: bool,
        #[clap(long, default_value_t = Default::default())]
        algo: DirtyAlgorithm,
        #[clap(long)]
        commit: Option<git2::Oid>,
        #[clap(long = "bin")]
        other_binaries: Vec<PathBuf>,
        #[clap(flatten)]
        analysis_index: AnalysisIndex,
        #[clap(long)]
        root: Option<PathBuf>,
    },
    AnalyzeAll {
        #[clap(long, default_value = "target/tmp/cargo-difftests")]
        dir: PathBuf,
        #[clap(long)]
        force: bool,
        #[clap(long, default_value_t = Default::default())]
        algo: DirtyAlgorithm,
        #[clap(long)]
        commit: Option<git2::Oid>,
        #[clap(long = "bin")]
        other_binaries: Vec<PathBuf>,
        #[clap(flatten)]
        analysis_index: AnalysisIndex,
        #[clap(long)]
        ignore_incompatible: bool,
    },
    LowLevel {
        #[clap(subcommand)]
        cmd: LowLevelCommand,
    },
}

#[derive(Parser, Debug)]
#[command(name = "cargo")]
#[command(bin_name = "cargo")]
pub enum CargoApp {
    Difftests {
        #[clap(subcommand)]
        app: App,
    },
}

pub type CargoDifftestsResult<T = ()> = anyhow::Result<T>;

fn resolver_for_index_root(
    tmpdir_root: &Path,
    index_root: Option<PathBuf>,
) -> Option<DiscoverIndexPathResolver> {
    index_root.map(|index_root| DiscoverIndexPathResolver::Remap {
        from: tmpdir_root.to_path_buf(),
        to: index_root,
    })
}

fn discover_difftests(
    dir: PathBuf,
    index_root: Option<PathBuf>,
    ignore_incompatible: bool,
) -> CargoDifftestsResult<Vec<Difftest>> {
    if !dir.exists() || !dir.is_dir() {
        warn!("Directory {} does not exist", dir.display());
        return Ok(vec![]);
    }

    let resolver = resolver_for_index_root(&dir, index_root);

    let discovered =
        cargo_difftests::discover_difftests(&dir, ignore_incompatible, resolver.as_ref())?;

    Ok(discovered)
}

fn run_discover_difftests(
    dir: PathBuf,
    index_root: Option<PathBuf>,
    ignore_incompatible: bool,
) -> CargoDifftestsResult {
    let discovered = discover_difftests(dir, index_root, ignore_incompatible)?;
    let s = serde_json::to_string(&discovered)?;
    println!("{s}");

    Ok(())
}

fn run_merge_profdata(dir: PathBuf, force: bool) -> CargoDifftestsResult {
    // we do not need the index resolver here, because we are not going to use the index
    let mut discovered = Difftest::discover_from(dir, None)?;

    discovered.merge_profraw_files_into_profdata(force)?;

    Ok(())
}

fn run_export_profdata(dir: PathBuf, cmd: ExportProfdataCommand) -> CargoDifftestsResult {
    // we do not need the index resolver here, because we are not going to use the index
    let mut discovered = Difftest::discover_from(dir, None)?;

    let has_profdata = discovered.assert_has_profdata();
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

fn run_analysis(
    dir: PathBuf,
    algo: DirtyAlgorithm,
    commit: Option<git2::Oid>,
) -> CargoDifftestsResult {
    let mut discovered = Difftest::discover_from(dir, None)?;
    let mut analysis_cx = discovered.assert_has_exported_profdata().start_analysis()?;

    analysis_cx.run(&AnalysisConfig {
        dirty_algorithm: algo.convert(commit),
    })?;

    let r = analysis_cx.finish_analysis();

    display_analysis_result(r);

    Ok(())
}

fn run_analysis_with_test_index(
    index: PathBuf,
    dirty_algorithm: DirtyAlgorithm,
    commit: Option<git2::Oid>,
) -> CargoDifftestsResult {
    let mut analysis_cx = AnalysisContext::with_index_from(&index)?;

    analysis_cx.run(&AnalysisConfig {
        dirty_algorithm: dirty_algorithm.convert(commit),
    })?;

    let r = analysis_cx.finish_analysis();

    display_analysis_result(r);

    Ok(())
}

fn compile_test_index_config(
    compile_test_index_flags: CompileTestIndexFlags,
) -> CargoDifftestsResult<IndexDataCompilerConfig> {
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

    Ok(config)
}

fn run_compile_test_index(
    dir: PathBuf,
    output: PathBuf,
    compile_test_index_flags: CompileTestIndexFlags,
) -> CargoDifftestsResult {
    let mut discovered = Difftest::discover_from(dir, None)?;
    let exported_profdata = discovered.assert_has_exported_profdata();

    let config = compile_test_index_config(compile_test_index_flags)?;

    let result = exported_profdata.compile_test_index_data(config)?;

    result.write_to_file(&output)?;

    Ok(())
}

fn run_indexes_touch_same_files_report(
    index1: PathBuf,
    index2: PathBuf,
    action: IndexesTouchSameFilesReportAction,
) -> CargoDifftestsResult {
    let index1 = DifftestsSingleTestIndexData::read_from_file(&index1)?;
    let index2 = DifftestsSingleTestIndexData::read_from_file(&index2)?;

    let report = cargo_difftests::compare_indexes_touch_same_files(&index1, &index2);

    action.do_for_report(report)?;

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
        LowLevelCommand::RunAnalysis { dir, algo, commit } => {
            run_analysis(dir, algo, commit)?;
        }
        LowLevelCommand::CompileTestIndex {
            dir,
            output,
            compile_test_index_flags,
        } => {
            run_compile_test_index(dir, output, compile_test_index_flags)?;
        }
        LowLevelCommand::RunAnalysisWithTestIndex {
            index,
            algo,
            commit,
        } => {
            run_analysis_with_test_index(index, algo, commit)?;
        }
        LowLevelCommand::IndexesTouchSameFilesReport {
            index1,
            index2,
            action,
        } => {
            run_indexes_touch_same_files_report(index1, index2, action)?;
        }
    }

    Ok(())
}

fn analyze_single_test(
    difftest: &mut Difftest,
    force: bool,
    algo: DirtyAlgorithm,
    commit: Option<git2::Oid>,
    bins: Vec<PathBuf>,
    analysis_index: &AnalysisIndex,
    resolver: Option<&DiscoverIndexPathResolver>,
) -> CargoDifftestsResult<AnalysisResult> {
    let mut analysis_cx = match analysis_index.index_strategy {
        AnalysisIndexStrategy::Never => {
            let has_profdata = difftest.merge_profraw_files_into_profdata(force)?;
            let has_exported_profdata =
                has_profdata.export_profdata_file(ExportProfdataConfig {
                    force,
                    ignore_registry_files: true,
                    other_binaries: bins,
                    test_desc: None, // will read from `self.json`
                })?;

            has_exported_profdata.start_analysis()?
        }
        AnalysisIndexStrategy::Always => {
            'l: {
                if difftest.has_index() {
                    // if we already have the index built, use it
                    break 'l AnalysisContext::with_index_from_difftest(difftest)?;
                }

                let has_profdata = difftest.merge_profraw_files_into_profdata(force)?;
                let has_exported_profdata =
                    has_profdata.export_profdata_file(ExportProfdataConfig {
                        force,
                        ignore_registry_files: true,
                        other_binaries: bins,
                        test_desc: None, // will read from `self.json`
                    })?;

                let config = compile_test_index_config(CompileTestIndexFlags::default())?;

                let test_index_data = has_exported_profdata.compile_test_index_data(config)?;

                if let Some(p) = resolver.and_then(|r| r.resolve(difftest.dir())) {
                    let parent = p.parent().unwrap();
                    if !parent.exists() {
                        fs::create_dir_all(parent)?;
                    }
                    test_index_data.write_to_file(&p)?;
                }

                AnalysisContext::from_index(test_index_data)
            }
        }
        AnalysisIndexStrategy::IfAvailable => {
            'l: {
                if difftest.has_index() {
                    // if we already have the index built, use it
                    break 'l AnalysisContext::with_index_from_difftest(difftest)?;
                }

                let has_profdata = difftest.merge_profraw_files_into_profdata(force)?;
                let has_exported_profdata =
                    has_profdata.export_profdata_file(ExportProfdataConfig {
                        force,
                        ignore_registry_files: true,
                        other_binaries: bins,
                        test_desc: None, // will read from `self.json`
                    })?;

                has_exported_profdata.start_analysis()?
            }
        }
    };

    analysis_cx.run(&AnalysisConfig {
        dirty_algorithm: algo.convert(commit),
    })?;

    let r = analysis_cx.finish_analysis();

    Ok(r)
}

fn run_analyze(
    dir: PathBuf,
    force: bool,
    algo: DirtyAlgorithm,
    commit: Option<git2::Oid>,
    bins: Vec<PathBuf>,
    root: Option<PathBuf>,
    analysis_index: AnalysisIndex,
) -> CargoDifftestsResult {
    let resolver = analysis_index.index_resolver(root)?;

    let mut difftest = Difftest::discover_from(dir, resolver.as_ref())?;

    let r = analyze_single_test(
        &mut difftest,
        force,
        algo,
        commit,
        bins,
        &analysis_index,
        resolver.as_ref(),
    )?;

    display_analysis_result(r);

    Ok(())
}

pub fn run_analyze_all(
    dir: PathBuf,
    force: bool,
    algo: DirtyAlgorithm,
    commit: Option<git2::Oid>,
    bins: Vec<PathBuf>,
    analysis_index: AnalysisIndex,
    ignore_incompatible: bool,
) -> CargoDifftestsResult {
    let resolver = analysis_index.index_resolver(Some(dir.clone()))?;
    let discovered =
        discover_difftests(dir, analysis_index.index_root.clone(), ignore_incompatible)?;

    let mut results = vec![];

    for mut difftest in discovered {
        let r = analyze_single_test(
            &mut difftest,
            force,
            algo,
            commit,
            bins.clone(),
            &analysis_index,
            resolver.as_ref(),
        )?;

        let result = AnalyzeAllSingleTest {
            test_desc: difftest.load_test_desc()?,
            difftest,
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
    let CargoApp::Difftests { app } = CargoApp::parse();

    match app {
        App::DiscoverDifftests {
            dir,
            index_root,
            ignore_incompatible,
        } => {
            run_discover_difftests(dir, index_root, ignore_incompatible)?;
        }
        App::Analyze {
            dir,
            root,
            force,
            algo,
            commit,
            other_binaries,
            analysis_index,
        } => {
            run_analyze(
                dir,
                force,
                algo,
                commit,
                other_binaries,
                root,
                analysis_index,
            )?;
        }
        App::AnalyzeAll {
            dir,
            force,
            algo,
            commit,
            other_binaries,
            analysis_index,
            ignore_incompatible,
        } => {
            run_analyze_all(
                dir,
                force,
                algo,
                commit,
                other_binaries,
                analysis_index,
                ignore_incompatible,
            )?;
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
