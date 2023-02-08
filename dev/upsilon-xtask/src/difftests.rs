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
use std::path::{Path, PathBuf};

use clap::Parser;
use log::info;

use crate::{cmd_call, cmd_output_pipe_to_file, cmd_output_string, cmd_process, XtaskResult};

#[derive(Parser, Debug)]
pub enum DiffTestsCommand {
    #[clap(name = "compile-profdata")]
    CompileProfdata {
        /// The directory containing the .profraw files,
        /// passed as tempdir to `upsilon_difftests_testclient::init`.
        tempdir: PathBuf,
    },
    #[clap(name = "export-coverage")]
    ExportCoverage {
        /// The directory containing the .profraw files,
        /// passed as tempdir to `upsilon_difftests_testclient::init`.
        tempdir: PathBuf,
        /// The binary to export coverage for.
        binary: PathBuf,
        /// The name of the coverage file to generate.
        #[clap(default_value = "coverage.json")]
        coverage_file_name: String,
    },
}

pub fn compile_profdata(tempdir: &Path) -> XtaskResult<()> {
    let mut profraw_files = vec![];

    for p in tempdir.read_dir()? {
        let p = p?;
        if p.path().extension() == Some(OsStr::new("profraw")) {
            profraw_files.push(p.path());
        }
    }

    info!(
        "Merging {} .profraw files into coverage.profdata",
        profraw_files.len(),
    );

    let target_file = tempdir.join("coverage.profdata");

    cmd_call!("rust-profdata", "merge", "-sparse", ...profraw_files, "-o", target_file)?;

    Ok(())
}

pub fn export_coverage(
    tempdir: &Path,
    binary: &Path,
    coverage_file_name: String,
) -> XtaskResult<()> {
    let target_file = tempdir.join("coverage.profdata");

    #[cfg(not(windows))]
    const REGISTRY_FILES_REGEX: &str = r"/.cargo/registry";

    #[cfg(windows)]
    const REGISTRY_FILES_REGEX: &str = r#"\\.cargo\\registry"#;

    let coverage_file = tempdir.join(coverage_file_name);
    cmd_output_pipe_to_file!(
        @coverage_file,
        "rust-cov",
        "export",
        "-instr-profile",
        target_file,
        binary,
        "--ignore-filename-regex",
        REGISTRY_FILES_REGEX,
    )?;

    Ok(())
}

pub fn run(command: DiffTestsCommand) -> XtaskResult<()> {
    match command {
        DiffTestsCommand::CompileProfdata { tempdir } => compile_profdata(&tempdir),
        DiffTestsCommand::ExportCoverage {
            tempdir,
            binary,
            coverage_file_name,
        } => export_coverage(&tempdir, &binary, coverage_file_name),
    }
}
