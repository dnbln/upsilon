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

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use git2::{DiffDelta, DiffHunk};
use log::{debug, info};

use crate::analysis_data::{CoverageBranch, CoverageData};
use crate::{DifftestsError, DifftestsResult, DiscoveredDifftest};

pub struct AnalysisContext<'r> {
    difftest: &'r mut DiscoveredDifftest,
    profdata: CoverageData,
    result: AnalysisResult,
}

impl<'r> AnalysisContext<'r> {
    pub(crate) fn new(difftest: &'r mut DiscoveredDifftest, profdata: CoverageData) -> Self {
        Self {
            difftest,
            profdata,
            result: AnalysisResult::Clean,
        }
    }

    pub fn get_profdata(&self) -> &CoverageData {
        &self.profdata
    }

    pub fn get_difftest(&self) -> &DiscoveredDifftest {
        self.difftest
    }

    pub fn finish_analysis(self) -> AnalysisResult {
        let r = self.result;

        info!("Analysis finished with result: {r:?}");

        r
    }
}

#[derive(Debug, Clone)]
pub enum DirtyAlgorithm {
    FileSystemMtimes,
    GitDiff,
}

#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    pub dirty_algorithm: DirtyAlgorithm,
}

impl<'r> AnalysisContext<'r> {
    pub fn run(&mut self, config: &AnalysisConfig) -> DifftestsResult {
        let AnalysisConfig { dirty_algorithm } = config;

        let r = match dirty_algorithm {
            DirtyAlgorithm::FileSystemMtimes => file_system_mtime_analysis(self)?,
            DirtyAlgorithm::GitDiff => git_diff_analysis(self)?,
        };

        self.result = r;

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnalysisResult {
    Clean,
    Dirty,
}

pub fn file_is_from_cargo_registry(f: &Path) -> bool {
    f.starts_with(concat!(env!("CARGO_HOME"), "/registry"))
}

pub fn test_touched_files(cx: &AnalysisContext, include_registry_files: bool) -> BTreeSet<PathBuf> {
    let pd = cx.get_profdata();

    fn any_branch_executed(b: &[CoverageBranch]) -> bool {
        b.iter().any(|it| it.execution_count > 0)
    }

    let mut test_touched_files = BTreeSet::new();

    for mapping in &pd.data {
        for function in &mapping.functions {
            let function_executed = function.count > 0;

            if !function_executed {
                continue;
            }

            for f in &function.filenames {
                if file_is_from_cargo_registry(f) && !include_registry_files {
                    continue;
                }

                test_touched_files.insert(f.clone());
            }
        }

        for file in &mapping.files {
            let file_touched_branches = any_branch_executed(&file.branches);
            let file_touched_expansions = file
                .expansions
                .iter()
                .any(|it| any_branch_executed(&it.branches));
            let file_touched_segments = file.segments.iter().any(|it| it.count > 0);

            if !file_touched_branches && !file_touched_expansions && !file_touched_segments {
                continue;
            }

            if file_is_from_cargo_registry(&file.filename) && !include_registry_files {
                continue;
            }

            test_touched_files.insert(file.filename.clone());
        }
    }

    test_touched_files
}

pub fn file_system_mtime_analysis(cx: &AnalysisContext) -> DifftestsResult<AnalysisResult> {
    let pd = &cx.profdata;
    let test_run_time = cx.difftest.self_json.metadata()?.modified()?;

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

pub trait LineRangeConstraint {
    fn validate(start: usize, end: usize) -> bool;
}

pub struct LineRange<C>
where
    C: LineRangeConstraint,
{
    pub start: usize,
    pub end: usize,
    _constraint: PhantomData<C>,
}

impl<C> fmt::Debug for LineRange<C>
where
    C: LineRangeConstraint,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "L{}..=L{}", self.start, self.end)
    }
}

impl<C> LineRange<C>
where
    C: LineRangeConstraint,
{
    #[track_caller]
    pub fn new(start: usize, end: usize) -> Self {
        Self::try_new(start, end).unwrap()
    }

    pub fn try_new(start: usize, end: usize) -> Option<Self> {
        if C::validate(start, end) {
            Some(Self::new(start, end))
        } else {
            None
        }
    }

    pub fn map_constraint<C2: LineRangeConstraint>(self) -> Result<LineRange<C2>, Self> {
        LineRange::try_new(self.start, self.end).ok_or(self)
    }

    pub fn map_constraint_assert<C2: LineRangeConstraint>(self) -> LineRange<C2> {
        self.map_constraint::<C2>().unwrap()
    }

    #[track_caller]
    pub fn new_u32(start: u32, end: u32) -> Self {
        Self::new(start as usize, end as usize)
    }

    pub fn intersects<C2: LineRangeConstraint>(&self, other: &LineRange<C2>) -> bool {
        self.start <= other.start && self.end >= other.start
            || self.start <= other.end && self.end >= other.end
    }
}

pub struct LineRangeEmptyConstraint;

impl LineRangeConstraint for LineRangeEmptyConstraint {
    fn validate(start: usize, end: usize) -> bool {
        start == end
    }
}

pub struct LineRangeNotEmptyConstraint;

impl LineRangeConstraint for LineRangeNotEmptyConstraint {
    fn validate(start: usize, end: usize) -> bool {
        start < end
    }
}

pub struct LineRangeValidConstraint;

impl LineRangeConstraint for LineRangeValidConstraint {
    fn validate(start: usize, end: usize) -> bool {
        start <= end
    }
}

pub enum Diff {
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
    pub fn from_hunk(hunk: &DiffHunk) -> Self {
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

pub fn git_diff_analysis(cx: &AnalysisContext) -> DifftestsResult<AnalysisResult> {
    let pd = &cx.profdata;

    let repo = git2::Repository::open_from_env()?;
    let head = repo.head()?.peel_to_tree()?;

    let mut diff_options = git2::DiffOptions::new();
    diff_options.include_untracked(true);
    diff_options.include_ignored(true);
    diff_options.include_unmodified(false);
    diff_options.include_typechange_trees(false);
    diff_options.include_typechange(true);
    diff_options.recurse_untracked_dirs(true);
    diff_options.recurse_ignored_dirs(true);

    let diff = repo.diff_tree_to_workdir(Some(&head), Some(&mut diff_options))?;

    let analysis_result = Rc::new(RefCell::new(AnalysisResult::Clean));

    let mut file_cb = {
        let analysis_result = Rc::clone(&analysis_result);

        move |delta: DiffDelta, _progress: f32| {
            let Some(path) = delta.old_file().path().or_else(|| delta.new_file().path()) else {
                return true;
            };

            for mapping in &pd.data {
                for file in &mapping.files {
                    if file.filename.ends_with(path) {
                        *analysis_result.borrow_mut() = AnalysisResult::Dirty;
                        return false;
                    }
                }
            }

            true
        }
    };

    let mut hunk_cb = {
        let analysis_result = Rc::clone(&analysis_result);

        move |delta: DiffDelta, hunk: DiffHunk| {
            let diff = Diff::from_hunk(&hunk);

            let intersection_target = match diff {
                Diff::Added(old, _new) => old.map_constraint_assert::<LineRangeValidConstraint>(),
                Diff::Removed(old, _new) => old.map_constraint_assert::<LineRangeValidConstraint>(),
                Diff::Modified(old, _new) => {
                    old.map_constraint_assert::<LineRangeValidConstraint>()
                }
            };

            let Some(path) = delta.old_file().path().or_else(|| delta.new_file().path()) else {
                return true;
            };

            for mapping in &pd.data {
                for file in &mapping.files {
                    if !file.filename.ends_with(path) {
                        continue;
                    }

                    for branch in &file.branches {
                        if LineRange::<LineRangeValidConstraint>::new(branch.l1, branch.l2)
                            .intersects(&intersection_target)
                        {
                            *analysis_result.borrow_mut() = AnalysisResult::Dirty;
                            return false;
                        }
                    }
                }
            }

            true
        }
    };

    let git_r = diff.foreach(&mut file_cb, None, Some(&mut hunk_cb), None);

    if let Err(e) = git_r {
        if e.code() != git2::ErrorCode::User {
            return Err(DifftestsError::Git(e));
        }
    }

    let r = *analysis_result.borrow();

    Ok(r)
}
