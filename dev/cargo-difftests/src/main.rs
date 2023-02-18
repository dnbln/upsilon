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
use cargo_difftests::difftest::{Difftest, DiscoverIndexPathResolver, ExportProfdataConfig};
use cargo_difftests::index_data::{IndexDataCompilerConfig, TestIndex};
use cargo_difftests::{AnalyzeAllSingleTest, IndexCompareDifferences, TouchSameFilesDifference};
use clap::{Args, Parser, ValueEnum};
use log::warn;

#[derive(Args, Debug)]
pub struct ExportProfdataCommand {
    /// Whether to ignore files from the cargo registry.
    ///
    /// This is enabled by default, as files in the cargo registry are not
    /// expected to be modified by the user.
    ///
    /// If you want to include files from the cargo registry, use the
    /// `--no-ignore-cargo-registry` flag.
    #[clap(
        long = "no-ignore-registry-files",
        default_value_t = true,
        action(clap::ArgAction::SetFalse)
    )]
    ignore_registry_files: bool,
    #[clap(flatten)]
    other_binaries: OtherBinaries,
    /// Whether to force the stages of the analysis.
    ///
    /// Without this flag, we will try to use intermediary cached results
    /// from previous runs of `cargo-difftests` if possible, to speed up
    /// the analysis.
    ///
    /// This flag will force the analysis to be run from scratch.
    #[clap(long)]
    force: bool,
}

#[derive(ValueEnum, Debug, Copy, Clone)]
pub enum FlattenFilesTarget {
    /// Flatten all files to the root of the repository.
    ///
    /// Files outside of the repository will be kept as-is.
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
    /// Whether to ignore files from the cargo registry.
    ///
    /// This is enabled by default, as files in the cargo registry are not
    /// expected to be modified by the user.
    ///
    /// If you want to include files from the cargo registry, use the
    /// `--no-ignore-cargo-registry` flag.
    #[clap(
        long = "no-ignore-cargo-registry",
        default_value_t = true,
        action(clap::ArgAction::SetFalse)
    )]
    ignore_cargo_registry: bool,
    /// Whether to flatten all files to a directory.
    #[clap(long)]
    flatten_files_to: Option<FlattenFilesTarget>,
    /// Whether to remove the binary path from the difftest info
    /// in the index.
    ///
    /// This is enabled by default, as it is expected to be an absolute
    /// path.
    #[clap(
        long = "no-remove-bin-path",
        default_value_t = true,
        action(clap::ArgAction::SetFalse)
    )]
    remove_bin_path: bool,
    /// Windows-only: Whether to replace all backslashes in paths with
    /// normal forward slashes.
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
            remove_bin_path: true,
            #[cfg(windows)]
            path_slash_replace: true,
        }
    }
}

#[derive(ValueEnum, Debug, Copy, Clone, Default)]
pub enum AnalysisIndexStrategy {
    /// Will always use indexes.
    ///
    /// If the indexes are not available, or they are outdated,
    /// they will be re-generated, and then the analysis will use
    /// the indexes.
    #[clap(name = "always")]
    Always,
    /// Will use indexes if they are available,
    /// but if they are not available, it will not generate them,
    /// and instead use a slightly slower algorithm to work with data
    /// straight from `llvm-cov export` instead.
    #[clap(name = "if-available")]
    IfAvailable,
    /// Will never use indexes.
    #[default]
    #[clap(name = "never")]
    Never,
    /// Will always use indexes, and will also clean up the difftest
    /// directory of all the profiling data, which should in theory
    /// not be needed anymore, as the analysis can run on index data alone,
    /// unless using the `never` strategy in subsequent calls of `cargo-difftests`.
    #[clap(name = "always-and-clean")]
    AlwaysAndClean,
}

impl Display for AnalysisIndexStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AnalysisIndexStrategy::Always => write!(f, "always"),
            AnalysisIndexStrategy::IfAvailable => write!(f, "if-available"),
            AnalysisIndexStrategy::Never => write!(f, "never"),
            AnalysisIndexStrategy::AlwaysAndClean => write!(f, "always-and-clean"),
        }
    }
}

#[derive(Args, Debug)]
pub struct DifftestDir {
    /// The path to the difftest directory.
    ///
    /// This should be the directory that was passed
    /// to `cargo_difftests_testclient::init`.
    #[clap(long)]
    pub dir: PathBuf,
}

#[derive(Parser, Debug)]
pub enum LowLevelCommand {
    /// Run the `llvm-profdata merge` command, to merge all
    /// the `.profraw` files from a difftest directory into
    /// a single `.profdata` file.
    MergeProfdata {
        #[clap(flatten)]
        dir: DifftestDir,
        /// Whether to force the merge.
        ///
        /// If this flag is not passed, and the `.profdata` file
        /// already exists, the merge will not be run.
        #[clap(long)]
        force: bool,
    },
    /// Run the `llvm-cov export` command, to export the
    /// `.profdata` file into a `.json` file that can be later
    /// used for analysis.
    ExportProfdata {
        #[clap(flatten)]
        dir: DifftestDir,
        #[clap(flatten)]
        cmd: ExportProfdataCommand,
    },
    /// Run the analysis for a single difftest directory.
    RunAnalysis {
        #[clap(flatten)]
        dir: DifftestDir,
        #[clap(flatten)]
        algo: AlgoArgs,
    },
    /// Compile a test index for a single difftest directory.
    CompileTestIndex {
        #[clap(flatten)]
        dir: DifftestDir,
        /// The output file to write the index to.
        #[clap(short, long)]
        output: PathBuf,
        #[clap(flatten)]
        compile_test_index_flags: CompileTestIndexFlags,
    },
    /// Runs the analysis for a single test index.
    RunAnalysisWithTestIndex {
        /// The path to the test index.
        #[clap(long)]
        index: PathBuf,
        #[clap(flatten)]
        algo: AlgoArgs,
    },
    /// Compare two test indexes, by the files that they "touch"
    /// (have regions that have an execution count > 0).
    IndexesTouchSameFilesReport {
        /// The first index to compare.
        index1: PathBuf,
        /// The second index to compare.
        index2: PathBuf,
        /// The action to take for the report.
        #[clap(long, default_value_t = Default::default())]
        action: IndexesTouchSameFilesReportAction,
    },
}

#[derive(ValueEnum, Debug, Copy, Clone, Default)]
pub enum IndexesTouchSameFilesReportAction {
    /// Print the report to stdout.
    #[default]
    #[clap(name = "print")]
    Print,
    /// Assert that the indexes touch the same files.
    ///
    /// If they do not, the program will exit with a non-zero exit code.
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

/// The algorithm to use for the analysis.
#[derive(ValueEnum, Debug, Copy, Clone, Default)]
pub enum DirtyAlgorithm {
    /// Use file system mtimes to find the files that have changed.
    ///
    /// This is the fastest algorithm, but it is not very accurate.
    #[default]
    #[clap(name = "fs-mtime")]
    FsMtime,
    /// Use the list of files from `git diff`.
    ///
    /// This is a bit slower than `fs-mtime`.
    ///
    /// Warning: not very accurate if not used well.
    /// See the introductory blog post for more details.
    #[clap(name = "git-diff-files")]
    GitDiffFiles,
    /// Use the list of diff hunks from `git diff` to compute the changed files.
    ///
    /// This is a bit slower than `fs-mtime`.
    ///
    /// Warning: like `git-diff-files`, it is not very accurate if not used well.
    /// See the introductory blog post for more details.
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
    /// The root directory where all index files will be stored.
    ///
    /// Only used if `--index-strategy` is set to `always`, `always-and-clean`
    /// or `if-available`, otherwise ignored.
    #[clap(
        long,
        required_if_eq_any = [
            ("index_strategy", "always"),
            ("index_strategy", "always-and-clean"),
            ("index_strategy", "if-available"),
        ]
    )]
    index_root: Option<PathBuf>,
    /// The strategy to use for the analysis index.
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
            AnalysisIndexStrategy::Always | AnalysisIndexStrategy::AlwaysAndClean => {
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

#[derive(Args, Debug)]
pub struct AlgoArgs {
    /// The algorithm to use to find the "dirty" files.
    #[clap(long, default_value_t = Default::default())]
    algo: DirtyAlgorithm,
    /// Optionally, if the algorithm is `git-diff-files` or `git-diff-hunks`,
    /// through this option we can specify another commit to use as the base
    /// for the diff.
    ///
    /// By default, the commit `HEAD` points to will be used.
    #[clap(long)]
    commit: Option<git2::Oid>,
}

#[derive(Args, Debug)]
pub struct OtherBinaries {
    /// Any other binaries to use for the analysis.
    ///
    /// By default, the only binary that `cargo-difftests` uses will
    /// be the `bin_path` from the test description (passed to
    /// `cargo_difftests_testclient::init`), but if the test spawned other
    /// children subprocesses that were profiled, and should be used in the
    /// analysis, then the paths to those binaries should be passed here.
    #[clap(long = "bin")]
    other_binaries: Vec<PathBuf>,
}

#[derive(Parser, Debug)]
pub enum App {
    /// Discover the difftests from a given directory.
    DiscoverDifftests {
        /// The root directory where all the difftests were stored.
        ///
        /// This should be some common ancestor directory of all
        /// the paths passed to `cargo_difftests_testclient::init`.
        #[clap(long, default_value = "target/tmp/cargo-difftests")]
        dir: PathBuf,
        /// The directory where the index files were stored, if any.
        #[clap(long)]
        index_root: Option<PathBuf>,
        /// With this flag, `cargo-difftests` will ignore any incompatible difftest and continue.
        ///
        /// Without this flag, when `cargo-difftests` finds an
        /// incompatible difftest on-disk, it will fail.
        #[clap(long)]
        ignore_incompatible: bool,
    },
    /// Analyze a single difftest.
    Analyze {
        #[clap(flatten)]
        dir: DifftestDir,
        /// Whether to force the generation of intermediary files.
        ///
        /// Without this flag, if the intermediary files are already present,
        /// they will be used instead of being regenerated.
        #[clap(long)]
        force: bool,
        #[clap(flatten)]
        algo: AlgoArgs,
        #[clap(flatten)]
        other_binaries: OtherBinaries,
        #[clap(flatten)]
        analysis_index: AnalysisIndex,
        /// The root directory where all the difftests were stored.
        ///
        /// Needs to be known to be able to properly remap the paths
        /// to the index files, and is therefore only required if the
        /// `--index-strategy` is `always`, `always-and-clean`, or
        /// `if-available`.
        #[clap(long, default_value = "target/tmp/cargo-difftests")]
        root: Option<PathBuf>,
    },
    /// Analyze all the difftests in a given directory.
    ///
    /// This is somewhat equivalent to running `cargo difftests discover-difftests`,
    /// and then `cargo difftests analyze` on each of the discovered difftests.
    AnalyzeAll {
        /// The root directory where all the difftests were stored.
        ///
        /// This should be some common ancestor directory of all
        /// the paths passed to `cargo_difftests_testclient::init`.
        #[clap(long, default_value = "target/tmp/cargo-difftests")]
        dir: PathBuf,
        /// Whether to force the generation of intermediary files.
        ///
        /// Without this flag, if the intermediary files are already present,
        /// they will be used instead of being regenerated.
        #[clap(long)]
        force: bool,
        #[clap(flatten)]
        algo: AlgoArgs,
        #[clap(flatten)]
        other_binaries: OtherBinaries,
        #[clap(flatten)]
        analysis_index: AnalysisIndex,
        /// With this flag, `cargo-difftests` will ignore any incompatible
        /// difftest and continue.
        ///
        /// Without this flag, when `cargo-difftests` finds an
        /// incompatible difftest on-disk, it will fail.
        #[clap(long)]
        ignore_incompatible: bool,
    },
    /// Analyze all the difftests in a given directory, using their index files.
    ///
    /// Note that this does not require the outputs of the difftests to be
    /// present on-disk, and can be used to analyze difftests that were
    /// run on a different machine (given correct flags when
    /// compiling the index).
    AnalyzeAllFromIndex {
        /// The root directory where all the index files are stored.
        #[clap(long)]
        index_root: PathBuf,
        #[clap(flatten)]
        algo: AlgoArgs,
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
        other_binaries: cmd.other_binaries.other_binaries,
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
                use path_slash::PathExt;

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
        remove_bin_path: compile_test_index_flags.remove_bin_path,
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
    let index1 = TestIndex::read_from_file(&index1)?;
    let index2 = TestIndex::read_from_file(&index2)?;

    let report = cargo_difftests::compare_indexes_touch_same_files(&index1, &index2);

    action.do_for_report(report)?;

    Ok(())
}

fn run_low_level_cmd(cmd: LowLevelCommand) -> CargoDifftestsResult {
    match cmd {
        LowLevelCommand::MergeProfdata { dir, force } => {
            run_merge_profdata(dir.dir, force)?;
        }
        LowLevelCommand::ExportProfdata { dir, cmd } => {
            run_export_profdata(dir.dir, cmd)?;
        }
        LowLevelCommand::RunAnalysis {
            dir,
            algo: AlgoArgs { algo, commit },
        } => {
            run_analysis(dir.dir, algo, commit)?;
        }
        LowLevelCommand::CompileTestIndex {
            dir,
            output,
            compile_test_index_flags,
        } => {
            run_compile_test_index(dir.dir, output, compile_test_index_flags)?;
        }
        LowLevelCommand::RunAnalysisWithTestIndex {
            index,
            algo: AlgoArgs { algo, commit },
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
        AnalysisIndexStrategy::AlwaysAndClean => {
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

                    difftest.clean()?;
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
            difftest: Some(difftest),
            verdict: r.into(),
        };

        results.push(result);
    }

    let out_json = serde_json::to_string(&results)?;
    println!("{out_json}");

    Ok(())
}

fn discover_indexes_to_vec(
    index_root: &Path,
    indexes: &mut Vec<TestIndex>,
) -> CargoDifftestsResult {
    for entry in fs::read_dir(index_root)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            discover_indexes_to_vec(&path, indexes)?;
        } else {
            let index = TestIndex::read_from_file(&path)?;
            indexes.push(index);
        }
    }

    Ok(())
}

pub fn run_analyze_all_from_index(
    index_root: PathBuf,
    algo: DirtyAlgorithm,
    commit: Option<git2::Oid>,
) -> CargoDifftestsResult {
    let indexes = {
        let mut indexes = vec![];
        discover_indexes_to_vec(&index_root, &mut indexes)?;
        indexes
    };

    let mut results = vec![];

    for index in indexes {
        let test_desc = index.test_desc.clone();

        let r = {
            let mut analysis_cx = AnalysisContext::from_index(index);
            analysis_cx.run(&AnalysisConfig {
                dirty_algorithm: algo.convert(commit),
            })?;
            analysis_cx.finish_analysis()
        };

        let result = AnalyzeAllSingleTest {
            test_desc,
            difftest: None,
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
            algo: AlgoArgs { algo, commit },
            other_binaries,
            analysis_index,
        } => {
            run_analyze(
                dir.dir,
                force,
                algo,
                commit,
                other_binaries.other_binaries,
                root,
                analysis_index,
            )?;
        }
        App::AnalyzeAll {
            dir,
            force,
            algo: AlgoArgs { algo, commit },
            other_binaries,
            analysis_index,
            ignore_incompatible,
        } => {
            run_analyze_all(
                dir,
                force,
                algo,
                commit,
                other_binaries.other_binaries,
                analysis_index,
                ignore_incompatible,
            )?;
        }
        App::AnalyzeAllFromIndex {
            index_root,
            algo: AlgoArgs { algo, commit },
        } => {
            run_analyze_all_from_index(index_root, algo, commit)?;
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
