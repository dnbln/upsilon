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

//! Holds the [`TestIndex`] struct, and logic for indexing [`CoverageData`] into
//! a [`TestIndex`].

use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use cargo_difftests_core::CoreTestDesc;

use crate::analysis_data::CoverageData;
use crate::{Difftest, DifftestsResult};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
struct IndexRegionSerDe([usize; 6]);

/// A region in a [`TestIndex`].
#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Debug)]
#[serde(from = "IndexRegionSerDe", into = "IndexRegionSerDe")]
pub struct IndexRegion {
    /// The line number of the first line of the region.
    pub l1: usize,
    /// The column number of the first column of the region.
    pub c1: usize,
    /// The line number of the last line of the region.
    pub l2: usize,
    /// The column number of the last column of the region.
    pub c2: usize,
    /// The number of times the region was executed.
    pub count: usize,
    /// The index of the file in the [`TestIndex`].
    pub file_id: usize,
}

impl From<IndexRegionSerDe> for IndexRegion {
    fn from(IndexRegionSerDe([l1, c1, l2, c2, count, file_id]): IndexRegionSerDe) -> Self {
        Self {
            l1,
            c1,
            l2,
            c2,
            count,
            file_id,
        }
    }
}

impl From<IndexRegion> for IndexRegionSerDe {
    fn from(
        IndexRegion {
            l1,
            c1,
            l2,
            c2,
            count,
            file_id,
        }: IndexRegion,
    ) -> Self {
        Self([l1, c1, l2, c2, count, file_id])
    }
}

/// A test index, which is a more compact representation of [`CoverageData`],
/// and contains only the information needed for analysis.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct TestIndex {
    /// The regions in all the files.
    pub regions: Vec<IndexRegion>,
    /// The paths to all the files.
    pub files: Vec<PathBuf>,
    /// The time the test was run.
    pub test_run: chrono::DateTime<chrono::Utc>,
    /// The test description.
    pub test_desc: CoreTestDesc,
}

impl TestIndex {
    /// Indexes/compiles the [`CoverageData`] into a [`TestIndex`].
    pub fn index(
        difftest: &Difftest,
        profdata: CoverageData,
        mut index_data_compiler_config: IndexDataCompilerConfig,
    ) -> DifftestsResult<Self> {
        let mut index_data = Self {
            regions: vec![],
            files: vec![],
            test_run: difftest.self_json_mtime()?.into(),
            test_desc: difftest.load_test_desc()?,
        };

        if index_data_compiler_config.remove_bin_path {
            index_data.test_desc.bin_path = PathBuf::new();
        }

        let mut mapping_files = BTreeMap::<PathBuf, usize>::new();

        for mapping in &profdata.data {
            for f in &mapping.functions {
                for region in &f.regions {
                    if region.execution_count == 0 {
                        continue;
                    }

                    let filename = &f.filenames[region.file_id];

                    if !(index_data_compiler_config.accept_file)(filename) {
                        continue;
                    }

                    let file_id = *mapping_files.entry(filename.clone()).or_insert_with(|| {
                        let id = index_data.files.len();
                        index_data
                            .files
                            .push((index_data_compiler_config.index_filename_converter)(
                                filename,
                            ));
                        id
                    });

                    index_data.regions.push(IndexRegion {
                        l1: region.l1,
                        c1: region.c1,
                        l2: region.l2,
                        c2: region.c2,
                        count: region.execution_count,
                        file_id,
                    });
                }
            }
        }

        Ok(index_data)
    }

    /// Writes the [`TestIndex`] to a file.
    pub fn write_to_file(&self, path: &Path) -> DifftestsResult {
        let mut file = File::create(path)?;
        let mut writer = BufWriter::new(&mut file);
        serde_json::to_writer(&mut writer, self)?;
        Ok(())
    }

    /// Reads a [`TestIndex`] from a file.
    pub fn read_from_file(path: &Path) -> DifftestsResult<Self> {
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    }
}

/// Configuration for the [`TestIndex::index`] function.
pub struct IndexDataCompilerConfig {
    /// A conversion function for the file names in the index.
    /// This is useful for converting absolute paths to paths
    /// relative to the repository root for example.
    pub index_filename_converter: Box<dyn FnMut(&Path) -> PathBuf>,
    /// A function that determines whether a file should be indexed.
    /// This is useful for excluding files that are not part of the
    /// project, such as files in the cargo registry.
    pub accept_file: Box<dyn FnMut(&Path) -> bool>,
    /// Whether to remove the binary path from the test description.
    ///
    /// As it is usually an absolute path (given by [`std::env::current_exe`]), it is not
    /// really useful, and may even not exist anymore, so passing true for this field
    /// removes it from the [`TestIndex`].
    pub remove_bin_path: bool,
}
