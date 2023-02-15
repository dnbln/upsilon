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

use std::collections::BTreeMap;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};

use crate::analysis_data::CoverageData;
use crate::{Difftest, DifftestsResult};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct IndexRegionSerDe([usize; 6]);

#[derive(serde::Serialize, serde::Deserialize, Copy, Clone, Debug)]
#[serde(from = "IndexRegionSerDe", into = "IndexRegionSerDe")]
pub struct IndexRegion {
    pub l1: usize,
    pub c1: usize,
    pub l2: usize,
    pub c2: usize,
    pub count: usize,
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DifftestsSingleTestIndexData {
    pub regions: Vec<IndexRegion>,
    pub files: Vec<PathBuf>,
    pub test_run: chrono::DateTime<chrono::Utc>,
}

impl DifftestsSingleTestIndexData {
    pub fn index(
        difftest: &Difftest,
        profdata: CoverageData,
        mut index_data_compiler_config: IndexDataCompilerConfig,
    ) -> DifftestsResult<Self> {
        let mut index_data = Self {
            regions: vec![],
            files: vec![],
            test_run: difftest.self_json.metadata()?.modified()?.into(),
        };

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

    pub fn write_to_file(&self, path: &Path) -> DifftestsResult {
        let mut file = File::create(path)?;
        let mut writer = BufWriter::new(&mut file);
        serde_json::to_writer(&mut writer, self)?;
        Ok(())
    }

    pub fn read_from_file(path: &Path) -> DifftestsResult<Self> {
        Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
    }
}

pub struct IndexDataCompilerConfig {
    pub index_filename_converter: Box<dyn FnMut(&Path) -> PathBuf>,
    pub accept_file: Box<dyn FnMut(&Path) -> bool>,
}
