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

//! Holds the [`Difftest`] struct and related functions.

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use cargo_difftests_core::CoreTestDesc;
use log::{debug, info, warn};

use crate::analysis::AnalysisContext;
use crate::index_data::{IndexDataCompilerConfig, TestIndex};
use crate::{analysis_data, DifftestsError, DifftestsResult};

/// A single difftest.
///
/// Points to the files that were generated by the call to
/// `cargo_difftests_testclient::init`.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct Difftest {
    dir: PathBuf,
    self_profraw: PathBuf,
    other_profraws: Vec<PathBuf>,
    self_json: PathBuf,
    profdata_file: Option<PathBuf>,
    exported_profdata_file: Option<PathBuf>,
    index_data: Option<PathBuf>,

    cleaned: bool,
}

impl Difftest {
    const CLEANED_FILE_NAME: &'static str = "cargo_difftests_cleaned";

    /// Get the directory to the difftest.
    ///
    /// Note that you should not modify the contents
    /// of this directory in any way.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Tests whether the difftest has a test index
    /// file attached.
    pub fn has_index(&self) -> bool {
        self.index_data.is_some()
    }

    /// Reads the [`TestIndex`] data associated with the [`Difftest`].
    ///
    /// Returns `Ok(None)` if the [`Difftest`] does not have an associated index file.
    pub fn read_index_data(&self) -> DifftestsResult<Option<TestIndex>> {
        let Some(index_data) = &self.index_data else {
            return Ok(None);
        };

        TestIndex::read_from_file(index_data).map(Some)
    }

    pub(crate) fn self_json_mtime(&self) -> DifftestsResult<std::time::SystemTime> {
        Ok(fs::metadata(&self.self_json)?.modified()?)
    }

    /// Loads the `self.json` file from the test directory, and parses it into
    /// a [`CoreTestDesc`] value.
    pub fn load_test_desc(&self) -> DifftestsResult<CoreTestDesc> {
        let s = fs::read_to_string(&self.self_json)?;
        let desc = serde_json::from_str(&s)
            .map_err(|e| DifftestsError::Json(e, Some(self.self_json.clone())))?;
        Ok(desc)
    }

    /// Loads the exported `.json` file from the directory, and parses it into
    /// a [`analysis_data::CoverageData`] value.
    ///
    /// Returns `Ok(None)` if the file does not exist.
    ///
    /// To export the `.profdata` file into a `.json` file, use
    /// [`HasProfdata::export_profdata_file`].
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

    /// Merges the `.profraw` files into a `.profdata` file, via `llvm-profdata merge`.
    /// This method gives access to a new type, [`HasProfdata`], which encodes the
    /// invariant that the `.profdata` file was created, and its output can be
    /// used.
    ///
    /// See [`HasProfdata`] for more details.
    pub fn merge_profraw_files_into_profdata(
        &mut self,
        force: bool,
    ) -> DifftestsResult<HasProfdata<'_>> {
        if self.cleaned {
            return Err(DifftestsError::DifftestCleaned);
        }

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

    /// Cleans up the profiling data from the [`Difftest`], reducing
    /// the size on-disk substantially, but all the profiling data
    /// gets lost in the process.
    ///
    /// This is used by the `--index-strategy=always-and-clean` flag, which keeps
    /// the test index data around, even if the profiling data is deleted.
    ///
    /// Given that the profiling data is deleted, the [`HasProfdata`] and
    /// [`HasExportedProfdata`] can never be created again from this [`Difftest`],
    /// and to get the profiling data the test has to be rerun.
    pub fn clean(&mut self) -> DifftestsResult<()> {
        fn clean_file(f: &mut Option<PathBuf>) -> DifftestsResult<()> {
            if let Some(f) = f {
                fs::remove_file(f)?;
            }

            *f = None;
            Ok(())
        }

        clean_file(&mut self.profdata_file)?;
        clean_file(&mut self.exported_profdata_file)?;

        fs::write(&self.self_profraw, b"")?;

        for profraw in self.other_profraws.drain(..) {
            fs::remove_file(profraw)?;
        }

        fs::write(self.dir.join(Self::CLEANED_FILE_NAME), b"")?;

        self.cleaned = true;

        Ok(())
    }

    /// Checks whether the [`Difftest`] has the `.profdata` file, and if so,
    /// returns Some([`HasProfdata`]), otherwise [`None`].
    pub fn has_profdata(&mut self) -> Option<HasProfdata<'_>> {
        if self.cleaned {
            return None;
        }

        if self.profdata_file.is_some() {
            Some(HasProfdata { difftest: self })
        } else {
            None
        }
    }

    /// Asserts that the [`Difftest`] has the `.profdata` file, and if so,
    /// returns [`HasProfdata`], otherwise panics.
    ///
    /// # Panics
    ///
    /// Panics if the [`Difftest`] has been cleaned, or if the `.profdata` file
    /// is missing.
    #[track_caller]
    pub fn assert_has_profdata(&mut self) -> HasProfdata<'_> {
        assert!(
            !self.cleaned,
            "difftest has been cleaned (from {})",
            self.dir.display(),
        );

        assert!(
            self.profdata_file.is_some(),
            "profdata file missing (from {})",
            self.dir.display(),
        );

        HasProfdata { difftest: self }
    }

    /// Checks whether the [`Difftest`] has the exported `.json` file, and if so,
    /// returns Some([`HasExportedProfdata`]), otherwise [`None`].
    ///
    /// Equivalent to [`Difftest::assert_has_profdata`] followed by
    /// [`HasProfdata::assert_has_exported_profdata`].
    #[track_caller]
    pub fn assert_has_exported_profdata(&mut self) -> HasExportedProfdata<'_> {
        self.assert_has_profdata().assert_has_exported_profdata()
    }

    /// Tries to discover a [`Difftest`] from the given directory, with the
    /// given [`DiscoverIndexPathResolver`] to resolve the index path.
    pub fn discover_from(
        dir: PathBuf,
        index_resolver: Option<&DiscoverIndexPathResolver>,
    ) -> DifftestsResult<Self> {
        let self_json = dir.join(cargo_difftests_core::CARGO_DIFFTESTS_SELF_JSON_FILENAME);

        if !self_json.exists() || !self_json.is_file() {
            return Err(DifftestsError::SelfJsonDoesNotExist(self_json));
        }

        discover_difftest_from_tempdir(dir, self_json, index_resolver)
    }
}

/// The configuration to use when exporting a `.profdata` file
/// into a `.json` file.
pub struct ExportProfdataConfig {
    /// Whether to ignore files from the cargo registry.
    pub ignore_registry_files: bool,
    /// Other binaries to include in the export.
    ///
    /// By default, only the test binary is included (via the [`CoreTestDesc`]'s
    /// `bin_path` field), but if the test has spawned some other child process
    /// (as often happens when testing binaries), and they
    /// were profiled, the paths to those binaries should
    /// be passed here.
    pub other_binaries: Vec<PathBuf>,
    /// The [`CoreTestDesc`] to use, if cached.
    ///
    /// If this is `None`, the test description will be read from the
    /// `self_json` file of the [`Difftest`].
    pub test_desc: Option<CoreTestDesc>,
    /// Whether to force the export.
    ///
    /// If `false`, the export will only be performed if the
    /// `.profdata` file does not exist.
    pub force: bool,
}

/// A [`Difftest`] with a `.profdata` file, invariant encoded
/// as a separate type, which allows the export of the `.profdata`
/// file into a `.json` file via [`HasProfdata::export_profdata_file`],
/// or to assert it's presence via the [`HasProfdata::assert_has_exported_profdata`].
///
/// To get this type, use [`Difftest::merge_profraw_files_into_profdata`]
/// or [`Difftest::assert_has_profdata`], or [`Difftest::has_profdata`].
pub struct HasProfdata<'r> {
    difftest: &'r mut Difftest,
}

const EXPORTED_PROFDATA_FILE_NAME: &str = "exported.json";

impl<'r> HasProfdata<'r> {
    /// Get the path to the `.profdata` file.
    pub fn get_profdata(&self) -> &Path {
        match self.difftest.profdata_file.as_ref() {
            Some(p) => p,
            None => unreachable!(),
        }
    }

    /// Exports the `.profdata` file into a `.json` file, via `llvm-cov export`.
    ///
    /// This method gives access to a new type, [`HasExportedProfdata`], which
    /// encodes the invariant that the `.profdata` file was exported, and its
    /// output can be used.
    ///
    /// See [`HasExportedProfdata`] for more details.
    pub fn export_profdata_file(
        mut self,
        config: ExportProfdataConfig,
    ) -> DifftestsResult<HasExportedProfdata<'r>> {
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
            return Err(DifftestsError::ProcessFailed { name: "rust-cov" });
        }

        self.difftest.exported_profdata_file = Some(p);

        Ok(HasExportedProfdata {
            difftest: self.difftest,
        })
    }

    /// Asserts that the `.profdata` file was exported into a `.json`
    /// with coverage data via [`HasProfdata::export_profdata_file`] previously,
    /// and returns a [`HasExportedProfdata`].
    ///
    /// # Panics
    ///
    /// Panics if the `.profdata` file was not previously exported via
    /// [`HasProfdata::export_profdata_file`].
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

/// A [`Difftest`] with a `.profdata` file, which also was exported.
/// Similarly to [`HasProfdata`], this type merely encodes the invariant that the
/// `.profdata` file was exported, and allows the analysis of the
/// exported data via [`HasExportedProfdata::start_analysis`], or to compile
/// a test index via [`HasExportedProfdata::compile_test_index_data`].
///
/// To get this type, use [`HasProfdata::export_profdata_file`],
/// [`HasProfdata::assert_has_exported_profdata`], or
/// [`Difftest::assert_has_exported_profdata`].
pub struct HasExportedProfdata<'r> {
    difftest: &'r mut Difftest,
}

impl<'r> HasExportedProfdata<'r> {
    /// Get the path to the exported `.json` profiling data file.
    pub fn get_exported_profdata(&self) -> &Path {
        match self.difftest.exported_profdata_file.as_ref() {
            Some(p) => p,
            None => unreachable!(),
        }
    }

    /// Get the path to the `.profdata` file.
    pub fn get_profdata(&self) -> &Path {
        match self.difftest.profdata_file.as_ref() {
            Some(p) => p,
            None => unreachable!(),
        }
    }

    /// Reads the exported `.json` profiling data file, to be able to use
    /// it for analysis.
    pub fn read_exported_profdata(&self) -> DifftestsResult<analysis_data::CoverageData> {
        let p = self.get_exported_profdata();
        let s = fs::read_to_string(p)?;
        let r =
            serde_json::from_str(&s).map_err(|e| DifftestsError::Json(e, Some(p.to_path_buf())))?;
        Ok(r)
    }

    /// Starts the analysis of the exported `.json` profiling data file.
    ///
    /// See the [`AnalysisContext`] type and the [`analysis`](crate::analysis)
    /// module for how to perform the analysis.
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

    /// Compiles the exported `.json` profiling data file into a
    /// [`TestIndex`] object, but does not save it on-disk.
    ///
    /// To save it on-disk, use [`TestIndex::write_to_file`].
    pub fn compile_test_index_data(
        &self,
        index_data_compiler_config: IndexDataCompilerConfig,
    ) -> DifftestsResult<TestIndex> {
        info!("Compiling test index data...");

        let profdata = self.read_exported_profdata()?;
        let test_index_data =
            TestIndex::index(self.difftest, profdata, index_data_compiler_config)?;

        info!("Done compiling test index data.");
        Ok(test_index_data)
    }
}

/// A resolver for test index data file paths.
///
/// # Examples
///
/// ```
/// # use std::path::{Path, PathBuf};
/// # use cargo_difftests::difftest::DiscoverIndexPathResolver;
///
/// let resolver = DiscoverIndexPathResolver::Remap {from: "foo".into(), to: "bar".into()};
///
/// assert_eq!(resolver.resolve(Path::new("foo/bar/baz")), Some(PathBuf::from("bar/bar/baz")));
/// assert_eq!(resolver.resolve(Path::new("bar/baz")), None);
/// ```
pub enum DiscoverIndexPathResolver {
    /// Remaps the index path from the given `from` path to the given `to` path.
    Remap {
        /// The path to strip from the index path.
        from: PathBuf,
        /// The path to append to the stripped index path.
        to: PathBuf,
    },
    /// A custom remapping function.
    Custom {
        /// The remapping function.
        f: Box<dyn Fn(&Path) -> Option<PathBuf>>,
    },
}

impl DiscoverIndexPathResolver {
    /// Resolves the index path from the given [`Difftest`] directory.
    pub fn resolve(&self, p: &Path) -> Option<PathBuf> {
        match self {
            DiscoverIndexPathResolver::Remap { from, to } => {
                let p = p.strip_prefix(from).ok()?;
                Some(to.join(p))
            }
            DiscoverIndexPathResolver::Custom { f } => f(p),
        }
    }
}

fn discover_difftest_from_tempdir(
    dir: PathBuf,
    self_json: PathBuf,
    index_resolver: Option<&DiscoverIndexPathResolver>,
) -> DifftestsResult<Difftest> {
    let self_profraw = dir.join(cargo_difftests_core::CARGO_DIFFTESTS_SELF_PROFILE_FILENAME);

    if !self_profraw.exists() {
        return Err(DifftestsError::SelfProfrawDoesNotExist(self_profraw));
    }

    let cargo_difftests_version = dir.join(cargo_difftests_core::CARGO_DIFFTESTS_VERSION_FILENAME);

    if !cargo_difftests_version.exists() {
        return Err(DifftestsError::CargoDifftestsVersionDoesNotExist(
            cargo_difftests_version,
        ));
    }

    let version = fs::read_to_string(&cargo_difftests_version)?;

    if version != env!("CARGO_PKG_VERSION") {
        return Err(DifftestsError::CargoDifftestsVersionMismatch(
            version,
            env!("CARGO_PKG_VERSION").to_string(),
        ));
    }

    let mut other_profraws = Vec::new();

    let mut profdata_file = None;

    let mut cleaned = false;

    for e in dir.read_dir()? {
        let e = e?;
        let p = e.path();

        if !p.is_file() {
            continue;
        }

        let file_name = p.file_name();
        let ext = p.extension();

        if ext == Some(OsStr::new("profraw"))
            && file_name
                != Some(OsStr::new(
                    cargo_difftests_core::CARGO_DIFFTESTS_SELF_PROFILE_FILENAME,
                ))
        {
            other_profraws.push(p);
            continue;
        }

        if ext == Some(OsStr::new("profdata")) {
            if profdata_file.is_none() {
                profdata_file = Some(p);
            } else {
                warn!(
                    "multiple profdata files found in difftest directory: {}",
                    dir.display()
                );
                warn!("ignoring: {}", p.display());
            }
            continue;
        }

        if file_name == Some(OsStr::new(Difftest::CLEANED_FILE_NAME)) {
            cleaned = true;
        }
    }

    let exported_profdata_path = dir.join(EXPORTED_PROFDATA_FILE_NAME);

    let mut exported_profdata_file = None;
    if exported_profdata_path.exists() && exported_profdata_path.is_file() {
        exported_profdata_file = Some(exported_profdata_path);
    }

    let index_data = 'index_data: {
        let index_data = index_resolver.and_then(|resolver| resolver.resolve(&dir));

        if let Some(ind) = &index_data {
            if !ind.exists() {
                debug!("index data file does not exist: {}", ind.display());
                break 'index_data None;
            } else if !ind.is_file() {
                debug!("index data file is not a file: {}", ind.display());
                break 'index_data None;
            }

            if ind.metadata()?.modified()? < self_json.metadata()?.modified()? {
                warn!(
                    "index data file is older than {}: {} older than {}",
                    cargo_difftests_core::CARGO_DIFFTESTS_SELF_JSON_FILENAME,
                    ind.display(),
                    self_json.display()
                );
                break 'index_data None;
            }
        }

        index_data
    };

    Ok(Difftest {
        dir,
        self_profraw,
        other_profraws,
        self_json,
        profdata_file,
        exported_profdata_file,
        index_data,
        cleaned,
    })
}

fn discover_difftests_to_vec(
    dir: &Path,
    discovered: &mut Vec<Difftest>,
    ignore_incompatible: bool,
    index_resolver: Option<&DiscoverIndexPathResolver>,
) -> DifftestsResult {
    let self_json = dir.join(cargo_difftests_core::CARGO_DIFFTESTS_SELF_JSON_FILENAME);
    if self_json.exists() && self_json.is_file() {
        let r = discover_difftest_from_tempdir(dir.to_path_buf(), self_json, index_resolver);

        if let Err(DifftestsError::CargoDifftestsVersionMismatch(_, _)) = r {
            if ignore_incompatible {
                return Ok(());
            }
        }

        discovered.push(r?);
        return Ok(());
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            discover_difftests_to_vec(&path, discovered, ignore_incompatible, index_resolver)?;
        }
    }

    Ok(())
}

/// Discovers all the [`Difftest`]s in the given directory,
/// optionally using the `index_resolver` to resolve the index paths,
/// and ignoring incompatible [`Difftest`] directories if `ignore_incompatible`
/// is true.
pub fn discover_difftests(
    dir: &Path,
    ignore_incompatible: bool,
    index_resolver: Option<&DiscoverIndexPathResolver>,
) -> DifftestsResult<Vec<Difftest>> {
    let mut discovered = Vec::new();

    discover_difftests_to_vec(dir, &mut discovered, ignore_incompatible, index_resolver)?;

    Ok(discovered)
}
