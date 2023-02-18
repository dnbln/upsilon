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

//! This module contains the [`AnalysisContext`] type, which is used to perform
//! the analysis of a difftest, or a test index.
//!
//! The [`AnalysisContext`] can be obtained by calling any of the following:
//! - [`HasExportedProfdata::start_analysis`](super::HasExportedProfdata::start_analysis)
//! - [`AnalysisContext::from_index`]
//! - [`AnalysisContext::with_index_from`]
//! - [`AnalysisContext::with_index_from_difftest`]
//!
//! The [`AnalysisContext`] can then be used to perform the analysis, by calling
//! [`AnalysisContext::run`].
//!
//! After [`AnalysisContext::run`] finished, the [`AnalysisContext::finish_analysis`]
//! method can be called to finish the analysis and get the result, dropping the
//! [`AnalysisContext`].
//!
//! # Examples
//!
//! ## Analyzing a difftest from coverage data
//!
//! ```no_run
//! # use std::path::{PathBuf, Path};
//! # use cargo_difftests::{
//! #     difftest::{Difftest, ExportProfdataConfig},
//! #     analysis::{AnalysisConfig, AnalysisResult, DirtyAlgorithm},
//! # };
//! let mut difftest = Difftest::discover_from(PathBuf::from("difftest"), None)?;
//! let has_profdata = difftest.merge_profraw_files_into_profdata(false)?;
//! let has_exported_profdata = has_profdata.export_profdata_file(ExportProfdataConfig {
//!     ignore_registry_files: true,
//!     force: false,
//!     test_desc: None,
//!     other_binaries: vec![],
//! })?;
//!
//! let mut analysis_context = has_exported_profdata.start_analysis()?;
//! analysis_context.run(&AnalysisConfig {
//!     dirty_algorithm: DirtyAlgorithm::FileSystemMtimes,
//! })?;
//!
//! let r = analysis_context.finish_analysis();
//!
//! match r {
//!     AnalysisResult::Dirty => {
//!         println!("difftest is dirty");
//!     }
//!     AnalysisResult::Clean => {
//!         println!("difftest is clean");
//!     }
//! }
//! # Ok::<_, cargo_difftests::DifftestsError>(())
//! ```
//!
//! ## Analyzing a difftest from a test index
//!
//! ```no_run
//! # use std::path::{PathBuf, Path};
//! # use cargo_difftests::{
//! #     difftest::{Difftest, ExportProfdataConfig},
//! #     index_data::{IndexDataCompilerConfig, TestIndex},
//! #     analysis::{AnalysisConfig, AnalysisContext, AnalysisResult, DirtyAlgorithm},
//! # };
//! // compile the test index first
//! let mut difftest = Difftest::discover_from(PathBuf::from("difftest"), None)?;
//! let has_profdata = difftest.merge_profraw_files_into_profdata(false)?;
//! let has_exported_profdata = has_profdata.export_profdata_file(ExportProfdataConfig {
//!     ignore_registry_files: true,
//!     force: false,
//!     test_desc: None,
//!     other_binaries: vec![],
//! })?;
//!
//! let test_index = has_exported_profdata.compile_test_index_data(IndexDataCompilerConfig {
//!     remove_bin_path: true,
//!     accept_file: Box::new(|_| true),
//!     index_filename_converter: Box::new(|p| p.to_path_buf()),
//! })?;
//!
//! // optionally save it
//! test_index.write_to_file(Path::new("test_index.json"))?;
//!
//! // or if we already have a test index saved
//! let test_index = TestIndex::read_from_file(Path::new("test_index.json"))?;
//!
//! let mut analysis_context = AnalysisContext::from_index(test_index);
//! analysis_context.run(&AnalysisConfig {
//!     dirty_algorithm: DirtyAlgorithm::FileSystemMtimes,
//! })?;
//!
//! let r = analysis_context.finish_analysis();
//!
//! match r {
//!     AnalysisResult::Dirty => {
//!         println!("difftest is dirty");
//!     }
//!     AnalysisResult::Clean => {
//!         println!("difftest is clean");
//!     }
//! }
//! # Ok::<_, cargo_difftests::DifftestsError>(())
//! ```
//!
//! # Explanations of the different [`DirtyAlgorithm`]s
//!
//! ## [`DirtyAlgorithm::FileSystemMtimes`]
//!
//! This algorithm compares the mtime of the files that were "touched" by the test
//! with the time when the test was last ran.
//!
//! A file is considered "touched" if it had any region with a non-zero coverage
//! execution count.
//!
//! It uses the mtime of one of the files generated by the `cargo_difftests_testclient::init`
//! function to determine when the test was last ran.
//!
//! ## [`DirtyAlgorithm::GitDiff`]
//!
//! For both [`GitDiffStrategy`]ies, this algorithm looks through the git diff
//! between the working tree and the commit HEAD points to, or the commit
//! commit in [`DirtyAlgorithm::GitDiff`] `commit` field if it is [`Some`].
//!
//! ### With [`GitDiffStrategy::FilesOnly`]
//!
//! This algorithm only looks at the files that were changed in the diff.
//!
//! If any of the files that were changed in the diff are files that were
//! "touched" by the test, then the test is considered dirty, and that is
//! the result of the analysis.
//!
//! ### With [`GitDiffStrategy::Hunks`]
//!
//! This algorithm looks at the hunks in the diff.
//!
//! A hunk is a continuous part of a file that was changed in the diff.
//!
//! It is identified by 4 numbers:
//! - An old start index
//! - An old line count
//! - A new start index
//! - A new line count
//!
//! This algorithm looks at the regions that were touched by the test,
//! and tries to intersect them with the hunks that were changed in the diff.
//!
//! If any of the regions that were touched by the test intersect with any of
//! the hunks that were changed in the diff, then the test is considered dirty,
//! and that is the result of the analysis.
//!
//! The way it achieves this is by looking at the ranges given by:
//! - `(hunk.old_start..hunk.old_start + hunk.old_line_count)` from the hunk
//! - `(region.l1..=region.l2)` from the region.
//!
//! If the intersection of these two ranges is not empty, then the region
//! intersects with the hunk.
//!
//! This is pretty error-prone, and in the [introductory blog post] there is
//! an example of how this algorithm can fail, but if used properly, it has
//! the potential to be the most accurate out of the three, as it can detect
//! changes in specific parts of the code, and in the case of big files that
//! can help a lot.
//!
//! [introductory blog post]: https://blog.dnbln.dev/posts/cargo-difftests/

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use git2::{DiffDelta, DiffHunk};
use log::{debug, info};

use crate::analysis_data::CoverageData;
use crate::index_data::TestIndex;
use crate::{Difftest, DifftestsError, DifftestsResult};

enum AnalysisContextInternal<'r> {
    DifftestWithCoverageData {
        difftest: &'r mut Difftest,
        profdata: CoverageData,
    },
    IndexData {
        index: TestIndex,
    },
}

/// An analysis context, which is used to perform analysis on a difftest, or
/// a test index.
///
/// To get a context, you can use the
/// [`HasExportedProfdata::start_analysis`](crate::difftest::HasExportedProfdata::start_analysis),
/// [`AnalysisContext::from_index`], [`AnalysisContext::with_index_from`],
/// or [`AnalysisContext::with_index_from_difftest`] associated functions.
pub struct AnalysisContext<'r> {
    internal: AnalysisContextInternal<'r>,
    result: AnalysisResult,
}

impl AnalysisContext<'static> {
    /// Create a new context from a test index.
    pub fn from_index(index: TestIndex) -> Self {
        Self {
            internal: AnalysisContextInternal::IndexData { index },
            result: AnalysisResult::Clean,
        }
    }

    /// Create a new context from a test index, read from the file at the given path.
    pub fn with_index_from(p: &Path) -> DifftestsResult<Self> {
        let index = TestIndex::read_from_file(p)?;

        Ok(Self::from_index(index))
    }

    /// Create a new context from a test index, read from the index from the [Difftest].
    pub fn with_index_from_difftest(difftest: &Difftest) -> DifftestsResult<Self> {
        let Some(index) = difftest.read_index_data()? else {
            panic!("Difftest does not have index data")
        };

        Ok(Self::from_index(index))
    }
}

impl<'r> AnalysisContext<'r> {
    pub(crate) fn new(difftest: &'r mut Difftest, profdata: CoverageData) -> Self {
        Self {
            internal: AnalysisContextInternal::DifftestWithCoverageData { difftest, profdata },
            result: AnalysisResult::Clean,
        }
    }

    /// Get the optional [`CoverageData`] that is used for analysis.
    pub fn get_profdata(&self) -> Option<&CoverageData> {
        match &self.internal {
            AnalysisContextInternal::DifftestWithCoverageData { profdata, .. } => Some(profdata),
            AnalysisContextInternal::IndexData { .. } => None,
        }
    }

    /// Get the optional [`Difftest`] that is used for analysis.
    pub fn get_difftest(&self) -> Option<&Difftest> {
        match &self.internal {
            AnalysisContextInternal::DifftestWithCoverageData { difftest, .. } => Some(difftest),
            AnalysisContextInternal::IndexData { .. } => None,
        }
    }

    /// Get the optional [`TestIndex`] that is used for analysis.
    pub fn get_index(&self) -> Option<&TestIndex> {
        match &self.internal {
            AnalysisContextInternal::DifftestWithCoverageData { .. } => None,
            AnalysisContextInternal::IndexData { index } => Some(index),
        }
    }

    /// Finish the analysis, and return the result.
    ///
    /// This function should be called after [`AnalysisContext::run`].
    pub fn finish_analysis(self) -> AnalysisResult {
        let r = self.result;

        info!("Analysis finished with result: {r:?}");

        r
    }

    /// Gets the time at which the test was run.
    pub fn test_run_at(&self) -> DifftestsResult<SystemTime> {
        match &self.internal {
            AnalysisContextInternal::DifftestWithCoverageData { difftest, .. } => {
                difftest.self_json_mtime()
            }
            AnalysisContextInternal::IndexData { index } => Ok(index.test_run.into()),
        }
    }

    /// Gets an iterator over the regions that are covered by the test.
    ///
    /// This iterator does not filter the regions that were not touched, so it
    /// may contain regions that were not covered by the test, but still were there
    /// in the [`CoverageData`].
    ///
    /// If using a [`TestIndex`] to run the analysis, then this iterator will only
    /// contain the regions that were touched by the test, as those are the only
    /// regions present in the [`TestIndex`].
    ///
    /// To clarify, we call the regions that had a non-zero execution count "touched".
    pub fn regions(&self) -> AnalysisRegions<'_> {
        AnalysisRegions {
            cx: self,
            regions_iter_state: match self.internal {
                AnalysisContextInternal::DifftestWithCoverageData { .. } => {
                    RegionsIterState::CoverageData {
                        mapping_idx: 0,
                        function_idx: 0,
                        region_idx: 0,
                    }
                }
                AnalysisContextInternal::IndexData { .. } => {
                    RegionsIterState::IndexData { region_idx: 0 }
                }
            },
        }
    }
}

/// An iterator over the regions that are present in the coverage data of the test.
///
/// See the documentation of [AnalysisContext::regions] for more information.
pub struct AnalysisRegions<'r> {
    cx: &'r AnalysisContext<'r>,
    regions_iter_state: RegionsIterState,
}

enum RegionsIterState {
    CoverageData {
        mapping_idx: usize,
        function_idx: usize,
        region_idx: usize,
    },
    IndexData {
        region_idx: usize,
    },
}

impl<'r> Iterator for AnalysisRegions<'r> {
    type Item = AnalysisRegion<'r>;

    fn next(&mut self) -> Option<Self::Item> {
        match &self.cx.internal {
            AnalysisContextInternal::DifftestWithCoverageData { profdata, .. } => {
                let RegionsIterState::CoverageData { region_idx, mapping_idx, function_idx} = &mut self.regions_iter_state else {
                    panic!("Invalid state");
                };

                if *mapping_idx >= profdata.data.len() {
                    return None;
                }

                let mapping = &profdata.data[*mapping_idx];

                if *function_idx >= mapping.functions.len() {
                    *mapping_idx += 1;
                    *function_idx = 0;
                    *region_idx = 0;
                    return self.next();
                }

                let function = &mapping.functions[*function_idx];

                if *region_idx >= function.regions.len() {
                    *function_idx += 1;
                    *region_idx = 0;
                    return self.next();
                }

                let region = &function.regions[*region_idx];

                let r = AnalysisRegion {
                    l1: region.l1,
                    c1: region.c1,
                    l2: region.l2,
                    c2: region.c2,
                    execution_count: region.execution_count,
                    file_ref: &function.filenames[region.file_id],
                };

                *region_idx += 1;
                Some(r)
            }
            AnalysisContextInternal::IndexData { index } => {
                let RegionsIterState::IndexData { region_idx } = &mut self.regions_iter_state else {
                    panic!("Invalid state");
                };

                if *region_idx >= index.regions.len() {
                    return None;
                }

                let region = &index.regions[*region_idx];

                let r = AnalysisRegion {
                    l1: region.l1,
                    c1: region.c1,
                    l2: region.l2,
                    c2: region.c2,
                    execution_count: region.count,
                    file_ref: &index.files[region.file_id],
                };

                *region_idx += 1;
                Some(r)
            }
        }
    }
}

/// An analysis region.
pub struct AnalysisRegion<'r> {
    /// The first line of the region.
    pub l1: usize,
    /// The first column of the region.
    pub c1: usize,
    /// The last line of the region.
    pub l2: usize,
    /// The last column of the region.
    pub c2: usize,
    /// The execution count of the region.
    pub execution_count: usize,
    /// The file that the region is in.
    pub file_ref: &'r Path,
}

/// The algorithm to use for the analysis.
#[derive(Debug, Clone)]
pub enum DirtyAlgorithm {
    /// Use file system mtimes.
    FileSystemMtimes,
    /// Use git diff, with the given strategy,
    /// and the base commit to diff with.
    GitDiff {
        /// The diff strategy to use.
        ///
        /// See [`GitDiffStrategy`] for more.
        strategy: GitDiffStrategy,
        /// The commit to diff with.
        commit: Option<git2::Oid>,
    },
}

/// The configuration for the analysis.
#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    /// The algorithm to use for the analysis.
    pub dirty_algorithm: DirtyAlgorithm,
}

impl<'r> AnalysisContext<'r> {
    /// Runs the analysis, with the given [`AnalysisConfig`].
    ///
    /// This should only be called once.
    /// If called multiple times, the output of
    /// the analysis will correspond to the last [`AnalysisContext::run`] call.
    pub fn run(&mut self, config: &AnalysisConfig) -> DifftestsResult {
        let AnalysisConfig { dirty_algorithm } = config;

        let r = match dirty_algorithm {
            DirtyAlgorithm::FileSystemMtimes => file_system_mtime_analysis(self)?,
            DirtyAlgorithm::GitDiff {
                strategy,
                commit: Some(commit),
            } => git_diff_analysis_from_commit(self, *strategy, *commit)?,
            DirtyAlgorithm::GitDiff {
                strategy,
                commit: None,
            } => git_diff_analysis(self, *strategy)?,
        };

        self.result = r;

        Ok(())
    }
}

/// The result of an analysis of a single [`Difftest`] or [`TestIndex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalysisResult {
    /// The analysis found no modifications to the source files used by the test,
    /// and it should probably not be rerun (unless something else changed, like
    /// non-source code file system inputs).
    Clean,
    /// The analysis found some modifications to the source files used by the test,
    /// and the it should be rerun.
    Dirty,
}

/// Checks whether the file is under the cargo registry.
///
/// # Examples
///
/// ```
/// # use std::path::Path;
/// # use cargo_difftests::analysis::file_is_from_cargo_registry;
///
/// assert!(file_is_from_cargo_registry(Path::new(concat!(env!("CARGO_HOME"), "/registry/src/github.com-1ecc6299db9ec823/serde-1.0.130/src/ser/impls.rs"))));
/// assert!(!file_is_from_cargo_registry(Path::new("src/main.rs")));
/// ```
pub fn file_is_from_cargo_registry(f: &Path) -> bool {
    f.starts_with(concat!(env!("CARGO_HOME"), "/registry"))
}

/// Returns a [`BTreeSet`] of the files that were touched by the test (have an execution_count > 0),
/// and ignoring files from the cargo registry if `include_registry_files` = false.
pub fn test_touched_files(cx: &AnalysisContext, include_registry_files: bool) -> BTreeSet<PathBuf> {
    cx.regions()
        .filter(|it| it.execution_count > 0)
        .map(|it| it.file_ref.to_path_buf())
        .filter(|it| include_registry_files || !file_is_from_cargo_registry(it))
        .collect::<BTreeSet<PathBuf>>()
}

/// Performs an analysis of the [`Difftest`], using file system mtimes.
///
/// For a comparison of the different algorithms,
/// see the [module-level documentation](crate::analysis).
pub fn file_system_mtime_analysis(cx: &AnalysisContext) -> DifftestsResult<AnalysisResult> {
    let test_run_time = cx.test_run_at()?;

    let test_touched_files = test_touched_files(cx, false);

    for f in &test_touched_files {
        debug!("Touched file: {}", f.display());
        let mtime = std::fs::metadata(f)?.modified()?;
        if mtime > test_run_time {
            debug!("File {} was modified after test run", f.display());
            return Ok(AnalysisResult::Dirty);
        }
    }

    Ok(AnalysisResult::Clean)
}

trait LineRangeConstraint {
    fn validate(start: usize, end: usize) -> bool;
}

struct LineRange<C>
where
    C: LineRangeConstraint,
{
    start: usize,
    end: usize,
    _constraint: PhantomData<C>,
}

impl<C> Clone for LineRange<C>
where
    C: LineRangeConstraint,
{
    fn clone(&self) -> Self {
        Self {
            start: self.start,
            end: self.end,
            _constraint: PhantomData,
        }
    }
}

impl<C> Copy for LineRange<C> where C: LineRangeConstraint {}

impl<C> fmt::Debug for LineRange<C>
where
    C: LineRangeConstraint,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}..L{}", self.start, self.end)
    }
}

impl<C> LineRange<C>
where
    C: LineRangeConstraint,
{
    #[track_caller]
    fn new(start: usize, end: usize) -> Self {
        Self::try_new(start, end).unwrap()
    }

    fn try_new(start: usize, end: usize) -> Option<Self> {
        if C::validate(start, end) {
            Some(Self {
                start,
                end,
                _constraint: PhantomData,
            })
        } else {
            None
        }
    }

    fn map_constraint<C2: LineRangeConstraint>(self) -> Result<LineRange<C2>, Self> {
        LineRange::try_new(self.start, self.end).ok_or(self)
    }

    fn map_constraint_assert<C2: LineRangeConstraint>(self) -> LineRange<C2> {
        self.map_constraint::<C2>().unwrap()
    }

    #[track_caller]
    fn new_u32(start: u32, end: u32) -> Self {
        Self::new(start as usize, end as usize)
    }

    fn intersects<C2: LineRangeConstraint>(&self, other: &LineRange<C2>) -> bool {
        self.start <= other.start && self.end > other.start
            || self.start < other.end && self.end >= other.end
    }
}

struct LineRangeEmptyConstraint;

impl LineRangeConstraint for LineRangeEmptyConstraint {
    fn validate(start: usize, end: usize) -> bool {
        start == end
    }
}

struct LineRangeNotEmptyConstraint;

impl LineRangeConstraint for LineRangeNotEmptyConstraint {
    fn validate(start: usize, end: usize) -> bool {
        start < end
    }
}

struct LineRangeValidConstraint;

impl LineRangeConstraint for LineRangeValidConstraint {
    fn validate(start: usize, end: usize) -> bool {
        start <= end
    }
}

#[derive(Debug, Clone, Copy)]
enum Diff {
    Added(
        LineRange<LineRangeEmptyConstraint>,
        LineRange<LineRangeNotEmptyConstraint>,
    ),
    Removed(
        LineRange<LineRangeNotEmptyConstraint>,
        LineRange<LineRangeEmptyConstraint>,
    ),
    Modified(
        LineRange<LineRangeNotEmptyConstraint>,
        LineRange<LineRangeNotEmptyConstraint>,
    ),
}

impl Diff {
    fn from_hunk(hunk: &DiffHunk) -> Self {
        let start = hunk.new_start();
        let end = start + hunk.new_lines();
        let new_range = LineRange::<LineRangeValidConstraint>::new_u32(start, end);

        let start = hunk.old_start();
        let end = start + hunk.old_lines();
        let old_range = LineRange::<LineRangeValidConstraint>::new_u32(start, end);

        if hunk.old_lines() == 0 {
            Self::Added(
                old_range.map_constraint_assert(),
                new_range.map_constraint_assert(),
            )
        } else if hunk.new_lines() == 0 {
            Self::Removed(
                old_range.map_constraint_assert(),
                new_range.map_constraint_assert(),
            )
        } else {
            Self::Modified(
                old_range.map_constraint_assert(),
                new_range.map_constraint_assert(),
            )
        }
    }
}

/// The git-diff strategy to use for the analysis.
///
/// More information in the [module-level documentation](crate::analysis).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GitDiffStrategy {
    /// Use files only.
    #[default]
    FilesOnly,
    /// Use hunks.
    Hunks,
}

impl GitDiffStrategy {
    fn callbacks<'a>(
        &self,
        cx: &'a AnalysisContext,
        analysis_result: Rc<RefCell<AnalysisResult>>,
    ) -> (
        Box<dyn FnMut(DiffDelta, f32) -> bool + 'a>,
        Box<dyn FnMut(DiffDelta, DiffHunk) -> bool + 'a>,
    ) {
        match self {
            Self::FilesOnly => {
                let file_cb = {
                    let analysis_result = Rc::clone(&analysis_result);

                    let test_touched_files = test_touched_files(cx, false);

                    move |delta: DiffDelta, _progress: f32| {
                        let Some(path) = delta.new_file().path().or_else(|| delta.old_file().path()) else {
                            return true;
                        };

                        let test_modified = test_touched_files.iter().any(|it| it.ends_with(path));

                        if test_modified {
                            *analysis_result.borrow_mut() = AnalysisResult::Dirty;
                            return false;
                        }

                        true
                    }
                };

                let hunk_cb = { |_delta: DiffDelta, _hunk: DiffHunk| true };

                (Box::new(file_cb), Box::new(hunk_cb))
            }
            Self::Hunks => {
                let file_cb = { |_delta: DiffDelta, _progress: f32| true };

                let hunk_cb = {
                    let analysis_result = Rc::clone(&analysis_result);

                    move |delta: DiffDelta, hunk: DiffHunk| {
                        let diff = Diff::from_hunk(&hunk);

                        let intersection_target = match diff {
                            Diff::Added(old, _new) => {
                                old.map_constraint_assert::<LineRangeValidConstraint>()
                            }
                            Diff::Removed(old, _new) => {
                                old.map_constraint_assert::<LineRangeValidConstraint>()
                            }
                            Diff::Modified(old, _new) => {
                                old.map_constraint_assert::<LineRangeValidConstraint>()
                            }
                        };

                        let Some(path) = delta.old_file().path().or_else(|| delta.new_file().path()) else {
                            return true;
                        };

                        for region in
                            cx.regions()
                                .filter(|r| r.execution_count > 0)
                                .filter(|region| {
                                    path.ends_with(region.file_ref)
                                        || region.file_ref.ends_with(path)
                                })
                        {
                            let region_range = LineRange::<LineRangeValidConstraint>::new(
                                region.l1,
                                region.l2 + 1, // l2 is inclusive
                            );
                            if region_range.intersects(&intersection_target) {
                                *analysis_result.borrow_mut() = AnalysisResult::Dirty;
                                return false;
                            }
                        }

                        true
                    }
                };

                (Box::new(file_cb), Box::new(hunk_cb))
            }
        }
    }
}

/// Performs a git diff analysis on the diff between the given tree
/// and the working tree.
///
/// The analysis is performed using the given strategy.
pub fn git_diff_analysis_from_tree(
    cx: &AnalysisContext,
    strategy: GitDiffStrategy,
    repo: &git2::Repository,
    tree: &git2::Tree,
) -> DifftestsResult<AnalysisResult> {
    let mut diff_options = git2::DiffOptions::new();

    diff_options.context_lines(0);

    let diff = repo.diff_tree_to_workdir(Some(&tree), Some(&mut diff_options))?;

    let analysis_result = Rc::new(RefCell::new(AnalysisResult::Clean));

    let (mut file_cb, mut hunk_cb) = strategy.callbacks(cx, Rc::clone(&analysis_result));

    let git_r = diff.foreach(&mut *file_cb, None, Some(&mut *hunk_cb), None);

    if let Err(e) = git_r {
        if e.code() != git2::ErrorCode::User {
            return Err(DifftestsError::Git(e));
        } else {
            debug_assert_eq!(*analysis_result.borrow(), AnalysisResult::Dirty);
        }
    }

    let r = *analysis_result.borrow();

    Ok(r)
}

/// Performs a git diff analysis on the diff between the current HEAD
/// and the working tree.
pub fn git_diff_analysis(
    cx: &AnalysisContext,
    strategy: GitDiffStrategy,
) -> DifftestsResult<AnalysisResult> {
    let repo = git2::Repository::open_from_env()?;
    let head = repo.head()?.peel_to_tree()?;

    git_diff_analysis_from_tree(cx, strategy, &repo, &head)
}

/// Performs a git diff analysis on the diff between tree of the
/// given commit and the working tree.
pub fn git_diff_analysis_from_commit(
    cx: &AnalysisContext,
    strategy: GitDiffStrategy,
    commit: git2::Oid,
) -> DifftestsResult<AnalysisResult> {
    let repo = git2::Repository::open_from_env()?;
    let tree = repo.find_commit(commit)?.tree()?;

    git_diff_analysis_from_tree(cx, strategy, &repo, &tree)
}
