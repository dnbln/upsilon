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

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_difftests_core::CoreTestDesc;
use log::{debug, info, warn};

use crate::analysis::AnalysisContext;
use crate::index_data::{DifftestsSingleTestIndexData, IndexDataCompilerConfig};

pub mod analysis_data;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct DiscoveredDifftest {
    dir: PathBuf,
    self_profraw: PathBuf,
    other_profraws: Vec<PathBuf>,
    self_json: PathBuf,
    profdata_file: Option<PathBuf>,
    exported_profdata_file: Option<PathBuf>,
}

pub struct ExportProfdataConfig {
    pub ignore_registry_files: bool,
    pub other_binaries: Vec<PathBuf>,
    pub test_desc: Option<CoreTestDesc>,
    pub force: bool,
}

pub struct HasProfdata<'r> {
    difftest: &'r mut DiscoveredDifftest,
}

pub struct HasExportedProfdata<'r> {
    difftest: &'r mut DiscoveredDifftest,
}

impl<'r> HasExportedProfdata<'r> {
    pub fn get_exported_profdata(&self) -> &Path {
        match self.difftest.exported_profdata_file.as_ref() {
            Some(p) => p,
            None => unreachable!(),
        }
    }

    pub fn get_profdata(&self) -> &Path {
        match self.difftest.profdata_file.as_ref() {
            Some(p) => p,
            None => unreachable!(),
        }
    }

    fn read_exported_profdata(&self) -> DifftestsResult<analysis_data::CoverageData> {
        let p = self.get_exported_profdata();
        let s = fs::read_to_string(p)?;
        let r =
            serde_json::from_str(&s).map_err(|e| DifftestsError::Json(e, Some(p.to_path_buf())))?;
        Ok(r)
    }

    pub fn start_analysis(self) -> DifftestsResult<AnalysisContext<'r>> {
        info!("Starting analysis...");
        debug!(
            "Reading exported profdata file from {:?}...",
            self.get_exported_profdata()
        );
        let profdata = self.read_exported_profdata()?;
        debug!(
            "Done reading exported profdata file from {:?}.",
            self.get_exported_profdata()
        );

        Ok(AnalysisContext::new(self.difftest, profdata))
    }

    pub fn compile_test_index_data(
        &self,
        index_data_compiler_config: IndexDataCompilerConfig,
    ) -> DifftestsResult<DifftestsSingleTestIndexData> {
        info!("Compiling test index data...");

        let profdata = self.read_exported_profdata()?;
        let test_index_data = DifftestsSingleTestIndexData::index(
            self.difftest,
            profdata,
            index_data_compiler_config,
        )?;

        info!("Done compiling test index data.");
        Ok(test_index_data)
    }
}

pub mod analysis;
pub mod index_data;

const EXPORTED_PROFDATA_FILE_NAME: &str = "exported.json";

impl<'r> HasProfdata<'r> {
    pub fn get_profdata(&self) -> &Path {
        match self.difftest.profdata_file.as_ref() {
            Some(p) => p,
            None => unreachable!(),
        }
    }

    pub fn export_profdata_file(
        &mut self,
        config: ExportProfdataConfig,
    ) -> DifftestsResult<HasExportedProfdata<'_>> {
        if self.difftest.exported_profdata_file.as_ref().is_some() && !config.force {
            return Ok(HasExportedProfdata {
                difftest: self.difftest,
            });
        }

        let profdata = self.get_profdata();

        let p = self.difftest.dir.join(EXPORTED_PROFDATA_FILE_NAME);

        let ExportProfdataConfig {
            ignore_registry_files,
            mut other_binaries,
            test_desc,
            force: _,
        } = config;

        let test_desc = match test_desc {
            Some(d) => d,
            None => self.difftest.load_test_desc()?,
        };

        for other_binary in other_binaries.iter_mut() {
            if !other_binary.is_absolute() {
                use path_absolutize::Absolutize;
                *other_binary = other_binary.absolutize()?.into_owned();
            }
        }

        let mut cmd = Command::new("rust-cov");

        cmd.arg("export")
            .arg("-instr-profile")
            .arg(profdata)
            .arg(&test_desc.bin_path)
            .args(
                other_binaries
                    .iter()
                    .flat_map(|it| [OsStr::new("--object"), it.as_os_str()]),
            );

        #[cfg(not(windows))]
        const REGISTRY_FILES_REGEX: &str = r"/.cargo/registry";

        #[cfg(windows)]
        const REGISTRY_FILES_REGEX: &str = r#"[\].cargo[\]registry"#;

        if ignore_registry_files {
            cmd.arg("--ignore-filename-regex").arg(REGISTRY_FILES_REGEX);
        }

        cmd.stdout(fs::File::create(&p)?);

        let status = cmd.status()?;

        if !status.success() {
            return Err(DifftestsError::ProcessFailed {
                name: "rust-profdata",
            });
        }

        self.difftest.exported_profdata_file = Some(p);

        Ok(HasExportedProfdata {
            difftest: self.difftest,
        })
    }

    pub fn assert_has_exported_profdata(self) -> HasExportedProfdata<'r> {
        assert!(
            self.difftest.exported_profdata_file.is_some(),
            "exported profdata file missing (from {})",
            self.difftest.dir.display(),
        );

        HasExportedProfdata {
            difftest: self.difftest,
        }
    }
}

impl DiscoveredDifftest {
    pub fn load_test_desc(&self) -> DifftestsResult<CoreTestDesc> {
        let s = fs::read_to_string(&self.self_json)?;
        let desc = serde_json::from_str(&s)
            .map_err(|e| DifftestsError::Json(e, Some(self.self_json.clone())))?;
        Ok(desc)
    }

    pub fn load_exported_profdata_file(
        &self,
    ) -> DifftestsResult<Option<analysis_data::CoverageData>> {
        let Some(p) = self.exported_profdata_file.as_ref() else {
            return Ok(None);
        };

        let data = serde_json::from_reader(fs::File::open(p)?)
            .map_err(|e| DifftestsError::Json(e, Some(p.clone())))?;

        Ok(Some(data))
    }

    pub fn merge_profraw_files_into_profdata(
        &mut self,
        force: bool,
    ) -> DifftestsResult<HasProfdata<'_>> {
        if self.profdata_file.is_some() && !force {
            return Ok(HasProfdata { difftest: self });
        }

        const OUT_FILE_NAME: &str = "merged.profdata";

        let p = self.dir.join(OUT_FILE_NAME);

        let mut cmd = Command::new("rust-profdata");

        cmd.arg("merge")
            .arg("-sparse")
            .arg(&self.self_profraw)
            .args(&self.other_profraws)
            .arg("-o")
            .arg(&p);

        let status = cmd.status()?;

        if !status.success() {
            return Err(DifftestsError::ProcessFailed {
                name: "rust-profdata",
            });
        }

        self.profdata_file = Some(p);

        Ok(HasProfdata { difftest: self })
    }

    pub fn has_profdata(&mut self) -> Option<HasProfdata<'_>> {
        if self.profdata_file.is_some() {
            Some(HasProfdata { difftest: self })
        } else {
            None
        }
    }

    pub fn assert_has_profdata(&mut self) -> HasProfdata<'_> {
        assert!(
            self.profdata_file.is_some(),
            "profdata file missing (from {})",
            self.dir.display(),
        );

        HasProfdata { difftest: self }
    }

    pub fn assert_has_exported_profdata(&mut self) -> HasExportedProfdata<'_> {
        self.assert_has_profdata().assert_has_exported_profdata()
    }

    pub fn discover_from(dir: PathBuf) -> DifftestsResult<Self> {
        let self_json = dir.join("self.json");

        if !self_json.exists() || !self_json.is_file() {
            return Err(DifftestsError::SelfJsonDoesNotExist(self_json));
        }

        discover_difftest_from_tempdir(dir, self_json)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum DifftestsError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error(
        "JSON error: {0}{}",
        match &.1 {
            Some(it) => format!(" (in {it:?})"),
            None => "".to_string(),
        }
    )]
    Json(#[source] serde_json::Error, Option<PathBuf>),
    #[error("Self json does not exist: {0:?}")]
    SelfJsonDoesNotExist(PathBuf),
    #[error("Self profraw does not exist: {0:?}")]
    SelfProfrawDoesNotExist(PathBuf),
    #[error("process failed: {name}")]
    ProcessFailed { name: &'static str },
    #[error("profdata file missing")]
    ProfdataFileMissing,
    #[error("git error: {0}")]
    Git(#[from] git2::Error),
}

impl From<serde_json::Error> for DifftestsError {
    fn from(e: serde_json::Error) -> Self {
        DifftestsError::Json(e, None)
    }
}

pub type DifftestsResult<T = ()> = Result<T, DifftestsError>;

fn discover_difftest_from_tempdir(
    dir: PathBuf,
    self_json: PathBuf,
) -> DifftestsResult<DiscoveredDifftest> {
    let self_profraw = dir.join("self.profraw");

    if !self_profraw.exists() {
        return Err(DifftestsError::SelfProfrawDoesNotExist(self_profraw));
    }

    let mut other_profraws = Vec::new();

    let mut profdata_file = None;

    for e in dir.read_dir()? {
        let e = e?;
        let p = e.path();

        if !p.is_file() {
            continue;
        }

        let file_name = p.file_name();
        let ext = p.extension();

        if ext == Some(OsStr::new("profraw")) && file_name != Some(OsStr::new("self.profraw")) {
            other_profraws.push(p);
            continue;
        }

        if ext == Some(OsStr::new("profdata")) {
            if profdata_file.is_none() {
                profdata_file = Some(p);
            } else {
                warn!(
                    "multiple profdata files found in difftest directory: {:?}",
                    dir
                );
                warn!("ignoring: {:?}", p);
            }
            continue;
        }
    }

    let exported_profdata_path = dir.join(EXPORTED_PROFDATA_FILE_NAME);

    let mut exported_profdata_file = None;
    if exported_profdata_path.exists() && exported_profdata_path.is_file() {
        exported_profdata_file = Some(exported_profdata_path);
    }

    Ok(DiscoveredDifftest {
        dir,
        self_profraw,
        other_profraws,
        self_json,
        profdata_file,
        exported_profdata_file,
    })
}

fn discover_difftests_to_vec(
    dir: &Path,
    discovered: &mut Vec<DiscoveredDifftest>,
) -> DifftestsResult {
    let self_json = dir.join("self.json");
    if self_json.exists() && self_json.is_file() {
        discovered.push(discover_difftest_from_tempdir(
            dir.to_path_buf(),
            self_json,
        )?);
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            discover_difftests_to_vec(&path, discovered)?;
        }
    }

    Ok(())
}

pub fn discover_difftests(dir: &Path) -> DifftestsResult<Vec<DiscoveredDifftest>> {
    let mut discovered = Vec::new();

    discover_difftests_to_vec(dir, &mut discovered)?;

    Ok(discovered)
}
