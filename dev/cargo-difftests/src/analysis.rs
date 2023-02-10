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
use std::fmt;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::SystemTime;

use git2::{DiffDelta, DiffHunk};
use log::{debug, info};

use crate::analysis_data::CoverageData;
use crate::index_data::DifftestsSingleTestIndexData;
use crate::{Difftest, DifftestsError, DifftestsResult};

enum AnalysisContextInternal<'r> {
    DifftestWithCoverageData {
        difftest: &'r mut Difftest,
        profdata: CoverageData,
    },
    IndexData {
        index: DifftestsSingleTestIndexData,
    },
}

pub struct AnalysisContext<'r> {
    internal: AnalysisContextInternal<'r>,
    result: AnalysisResult,
}

impl AnalysisContext<'static> {
    pub fn from_index(index: DifftestsSingleTestIndexData) -> Self {
        Self {
            internal: AnalysisContextInternal::IndexData { index },
            result: AnalysisResult::Clean,
        }
    }

    pub fn with_index_from(p: &Path) -> DifftestsResult<Self> {
        let index = DifftestsSingleTestIndexData::read_from_file(p)?;

        Ok(Self::from_index(index))
    }

    pub fn with_index_from_difftest(difftest: &Difftest) -> DifftestsResult<Self> {
        let Some(index_data) = difftest.index_data.as_ref() else {
            panic!("Difftest does not have index data")
        };

        let index = DifftestsSingleTestIndexData::read_from_file(index_data)?;

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

    pub fn get_profdata(&self) -> Option<&CoverageData> {
        match &self.internal {
            AnalysisContextInternal::DifftestWithCoverageData { profdata, .. } => Some(profdata),
            AnalysisContextInternal::IndexData { .. } => None,
        }
    }

    pub fn get_difftest(&self) -> Option<&Difftest> {
        match &self.internal {
            AnalysisContextInternal::DifftestWithCoverageData { difftest, .. } => Some(difftest),
            AnalysisContextInternal::IndexData { .. } => None,
        }
    }

    pub fn finish_analysis(self) -> AnalysisResult {
        let r = self.result;

        info!("Analysis finished with result: {r:?}");

        r
    }

    fn test_run_at(&self) -> DifftestsResult<SystemTime> {
        match &self.internal {
            AnalysisContextInternal::DifftestWithCoverageData { difftest, .. } => {
                Ok(difftest.self_json.metadata()?.modified()?)
            }
            AnalysisContextInternal::IndexData { index } => Ok(index.test_run.into()),
        }
    }

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

pub struct AnalysisRegion<'r> {
    pub l1: usize,
    pub c1: usize,
    pub l2: usize,
    pub c2: usize,
    pub execution_count: usize,
    pub file_ref: &'r Path,
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
    cx.regions()
        .filter(|it| it.execution_count > 0)
        .map(|it| it.file_ref.to_path_buf())
        .filter(|it| include_registry_files || !file_is_from_cargo_registry(it))
        .collect::<BTreeSet<PathBuf>>()
}

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
            Some(Self {
                start,
                end,
                _constraint: PhantomData,
            })
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
    let repo = git2::Repository::open_from_env()?;
    let head = repo.head()?.peel_to_tree()?;

    let mut diff_options = git2::DiffOptions::new();

    let diff = repo.diff_tree_to_workdir(Some(&head), Some(&mut diff_options))?;

    let analysis_result = Rc::new(RefCell::new(AnalysisResult::Clean));

    let mut file_cb = {
        // let analysis_result = Rc::clone(&analysis_result);

        |_delta: DiffDelta, _progress: f32| true
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

            for region in cx
                .regions()
                .filter(|region| path.ends_with(region.file_ref) || region.file_ref.ends_with(path))
            {
                if LineRange::<LineRangeValidConstraint>::new(region.l1, region.l2)
                    .intersects(&intersection_target)
                {
                    *analysis_result.borrow_mut() = AnalysisResult::Dirty;
                    return false;
                }
            }

            true
        }
    };

    let git_r = diff.foreach(&mut file_cb, None, Some(&mut hunk_cb), None);

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
