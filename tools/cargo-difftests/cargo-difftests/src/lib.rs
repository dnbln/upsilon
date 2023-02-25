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

use std::collections::BTreeSet;
use std::path::PathBuf;

use cargo_difftests_core::CoreTestDesc;

use crate::analysis::AnalysisResult;
use crate::difftest::Difftest;
use crate::index_data::TestIndex;

pub mod analysis;
pub mod analysis_data;
pub mod difftest;
pub mod index_data;

/// Errors that can occur when running `cargo difftests`.
#[derive(thiserror::Error, Debug)]
pub enum DifftestsError {
    /// IO error.
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    /// JSON error (during the deserialization of the
    /// file at the given [PathBuf], if any).
    #[error(
        "JSON error: {0}{}",
        match &.1 {
            Some(it) => format!(" (in {it:?})"),
            None => "".to_string(),
        }
    )]
    Json(#[source] serde_json::Error, Option<PathBuf>),
    /// The `self.json` file does not exist.
    #[error("Self json does not exist: {0:?}")]
    SelfJsonDoesNotExist(PathBuf),
    /// The `self.profraw` file does not exist.
    #[error("Self profraw does not exist: {0:?}")]
    SelfProfrawDoesNotExist(PathBuf),
    /// The `cargo_difftests_version` file does not exist.
    #[error("cargo_difftests_version file does not exist: {0:?}")]
    CargoDifftestsVersionDoesNotExist(PathBuf),
    /// The content of the `cargo_difftests_version` file indicates
    /// a mismatch between the version of `cargo_difftests_testclient`
    /// that generated the difftest and the version of `cargo_difftests`.
    #[error("cargo difftests version mismatch: {0} (file) != {1} (cargo difftests)")]
    CargoDifftestsVersionMismatch(String, String),
    /// The process failed.
    #[error("process failed: {name}")]
    ProcessFailed { name: &'static str },
    /// A [git2::Error] occurred.
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
    /// The difftest has been cleaned.
    #[error("difftest has been cleaned")]
    DifftestCleaned,
}

impl From<serde_json::Error> for DifftestsError {
    fn from(e: serde_json::Error) -> Self {
        DifftestsError::Json(e, None)
    }
}

pub type DifftestsResult<T = ()> = Result<T, DifftestsError>;

/// Compares two indexes, returning an error consisting of their deltas
/// if they are different, or [Ok] if they are the same.
///
/// This only looks at the files that are touched by the indexes,
/// and not at individual code regions.
pub fn compare_indexes_touch_same_files(
    index_a: &TestIndex,
    index_b: &TestIndex,
) -> Result<(), IndexCompareDifferences<TouchSameFilesDifference>> {
    let mut diffs = IndexCompareDifferences {
        differences: vec![],
    };

    let a_files = index_a.files.iter().collect::<BTreeSet<_>>();
    let b_files = index_b.files.iter().collect::<BTreeSet<_>>();

    diffs.differences.extend(
        a_files
            .difference(&b_files)
            .map(|f| TouchSameFilesDifference::TouchedByFirstOnly((*f).clone())),
    );

    diffs.differences.extend(
        b_files
            .difference(&a_files)
            .map(|f| TouchSameFilesDifference::TouchedBySecondOnly((*f).clone())),
    );

    if diffs.differences.is_empty() {
        Ok(())
    } else {
        Err(diffs)
    }
}

/// A list of differences between two indexes.
#[derive(Clone, Debug)]
pub struct IndexCompareDifferences<D> {
    differences: Vec<D>,
}

impl<D> IndexCompareDifferences<D> {
    pub fn differences(&self) -> &[D] {
        &self.differences
    }
}

/// A difference between two indexes, given by comparing the list
/// of files that the [Difftest]s they come from touched.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum TouchSameFilesDifference {
    /// A file that was touched by the first index, but not by the second.
    #[serde(rename = "first_only")]
    TouchedByFirstOnly(PathBuf),
    /// A file that was touched by the second index, but not by the first.
    #[serde(rename = "second_only")]
    TouchedBySecondOnly(PathBuf),
}

/// When using `analyze-all`, the output is a JSON stream of type
/// [Vec]<[AnalyzeAllSingleTest]>.
///
/// This is in the library so that it can be used by consumers of the
/// `cargo difftests` binary.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct AnalyzeAllSingleTest {
    /// The [Difftest] that was analyzed, or [None] if
    /// the analysis was performed on the index data alone,
    /// with no [Difftest] associated.
    pub difftest: Option<Difftest>,
    /// The description of the test that was analyzed.
    pub test_desc: CoreTestDesc,
    /// The result of the analysis.
    pub verdict: AnalysisVerdict,
}

/// An analysis verdict.
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, PartialEq, Eq, Debug)]
pub enum AnalysisVerdict {
    /// The analysis found no modifications to the source files used by the test,
    /// and it should probably not be rerun (unless something else changed, like
    /// non-source code file system inputs).
    #[serde(rename = "clean")]
    Clean,
    /// The analysis found some modifications to the source files used by the test,
    /// and the it should be rerun.
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
