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

//! Profiling data, deserialized from `llvm-cov export` JSON.

use std::collections::HashMap;
use std::path::PathBuf;

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct CoverageData {
    pub data: Vec<CoverageMapping>,
    #[serde(rename = "type")]
    pub kind: String,
    pub version: String,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct CoverageMapping {
    pub functions: Vec<CoverageFunction>,
    pub files: Vec<CoverageFile>,
    pub totals: BinarySummary,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct CoverageFile {
    pub filename: PathBuf,
    pub branches: Vec<CoverageBranch>,
    pub segments: Vec<CoverageFileSegment>,
    pub expansions: Vec<Expansion>,
    pub summary: FileSummary,
}

#[derive(serde::Deserialize, Debug)]
#[serde(transparent)]
pub struct CoverageBranchDe([usize; 9]);

#[derive(serde::Deserialize, Debug)]
#[serde(from = "CoverageBranchDe")]
pub struct CoverageBranch {
    pub l1: usize,
    pub l2: usize,
    pub c1: usize,
    pub c2: usize,

    pub execution_count: usize,
    pub false_execution_count: usize,
    pub file_id: usize,
    pub expanded_file_id: usize,
    pub region_kind: usize,
}

impl From<CoverageBranchDe> for CoverageBranch {
    fn from(
        CoverageBranchDe(
            [l1, l2, c1, c2, execution_count, false_execution_count, file_id, expanded_file_id, region_kind],
        ): CoverageBranchDe,
    ) -> Self {
        Self {
            l1,
            l2,
            c1,
            c2,
            execution_count,
            false_execution_count,
            file_id,
            expanded_file_id,
            region_kind,
        }
    }
}

#[derive(serde::Deserialize, Debug)]
struct CoverageFileSegmentDe(usize, usize, usize, bool, bool, bool);

#[derive(serde::Deserialize, Debug)]
#[serde(from = "CoverageFileSegmentDe")]
pub struct CoverageFileSegment {
    line: usize,
    col: usize,
    count: usize,
    has_count: bool,
    is_region_entry: bool,
    is_gap_region: bool,
}

impl From<CoverageFileSegmentDe> for CoverageFileSegment {
    fn from(
        CoverageFileSegmentDe(line, col, count, has_count, is_region_entry, is_gap_region): CoverageFileSegmentDe,
    ) -> Self {
        Self {
            line,
            col,
            count,
            has_count,
            is_region_entry,
            is_gap_region,
        }
    }
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct Expansion {
    pub branches: Vec<CoverageBranch>,
    pub filenames: Vec<PathBuf>,
    pub source_region: Region,
    pub target_regions: Vec<Region>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(transparent)]
pub struct TargetRegion {
    pub region: HashMap<String, serde_json::Value>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct FileSummary {
    pub lines: GenericSummary,
    pub functions: GenericSummary,
    pub instantiations: GenericSummary,
    pub regions: RegionsSummary,
    pub branches: BranchesSummary,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct BranchesSummary {
    #[serde(flatten)]
    pub generic: GenericSummary,
    pub notcovered: usize,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct RegionsSummary {
    #[serde(flatten)]
    pub generic: GenericSummary,
    pub notcovered: usize,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct GenericSummary {
    pub count: usize,
    pub covered: usize,
    pub percent: f64,
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct CoverageFunction {
    pub branches: Vec<serde_json::Value>,
    pub filenames: Vec<PathBuf>,
    #[serde(deserialize_with = "deserialize_function_name")]
    pub name: String,
    pub count: usize,
    pub regions: Vec<Region>,
}

#[derive(serde::Deserialize, Debug)]
#[serde(transparent)]
struct CoverageFunctionRegionDe([usize; 8]);

#[derive(serde::Deserialize, Debug)]
#[serde(from = "CoverageFunctionRegionDe")]
pub struct Region {
    pub l1: usize,
    pub c1: usize,
    pub l2: usize,
    pub c2: usize,

    pub execution_count: usize,
    pub file_id: usize,
    pub expanded_file_id: usize,
    pub region_kind: usize,
}

impl From<CoverageFunctionRegionDe> for Region {
    fn from(
        CoverageFunctionRegionDe(
            [l1, c1, l2, c2, execution_count, file_id, expanded_file_id, region_kind],
        ): CoverageFunctionRegionDe,
    ) -> Self {
        Self {
            l1,
            c1,
            l2,
            c2,
            execution_count,
            file_id,
            expanded_file_id,
            region_kind,
        }
    }
}

fn deserialize_function_name<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = <String as serde::Deserialize>::deserialize(deserializer)?;
    Ok(rustc_demangle::demangle(&s).to_string())
}

#[derive(serde::Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct BinarySummary {
    pub lines: GenericSummary,
    pub functions: GenericSummary,
    pub instantiations: GenericSummary,
    pub regions: RegionsSummary,
    pub branches: BranchesSummary,
}
