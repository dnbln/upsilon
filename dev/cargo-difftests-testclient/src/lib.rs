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

#![cfg(any(cargo_difftests, docsrs))]

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use cargo_difftests_core::CoreTestDesc;

/// A description of a test.
///
/// This is used to identify the test, and the binary from which it came from.
/// `cargo difftests` only uses the `bin_path`, all the other fields can
/// have any values you'd like to give them.
pub struct TestDesc {
    /// The package name.
    pub pkg_name: String,
    /// The crate name.
    pub crate_name: String,
    /// The binary name.
    pub bin_name: Option<String>,
    /// The binary path.
    pub bin_path: PathBuf,
    /// The test name.
    pub test_name: String,

    /// Any other fields to help identify the test.
    pub other_fields: HashMap<String, String>,
}

/// The difftests environment.
pub struct DifftestsEnv {
    llvm_profile_file_name: OsString,
    llvm_profile_file_value: OsString,
}

impl DifftestsEnv {
    /// Returns an iterator over the environment variables that should be set
    /// for child processes.
    pub fn env_for_children(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        std::iter::once((
            self.llvm_profile_file_name.as_os_str(),
            self.llvm_profile_file_value.as_os_str(),
        ))
    }
}

#[cfg(cargo_difftests)]
extern "C" {
    fn __llvm_profile_set_filename(filename: *const std::ffi::c_char);
}

// put a dummy for docs.rs
#[cfg(all(not(cargo_difftests), docsrs))]
unsafe fn __llvm_profile_set_filename(_: *const std::ffi::c_char) {}

/// Initializes the difftests environment.
pub fn init(desc: TestDesc, tmpdir: &Path) -> std::io::Result<DifftestsEnv> {
    if tmpdir.exists() {
        std::fs::remove_dir_all(tmpdir)?;
    }
    std::fs::create_dir_all(tmpdir)?;

    let self_profile_file =
        tmpdir.join(cargo_difftests_core::CARGO_DIFFTESTS_SELF_PROFILE_FILENAME);

    let self_profile_file_str = self_profile_file.to_str().unwrap();

    let self_profile_file_str_c = std::ffi::CString::new(self_profile_file_str).unwrap();

    unsafe {
        __llvm_profile_set_filename(self_profile_file_str_c.as_ptr());
    }

    let self_info_path = tmpdir.join(cargo_difftests_core::CARGO_DIFFTESTS_SELF_JSON_FILENAME);

    let core_test_desc = CoreTestDesc {
        pkg_name: desc.pkg_name,
        crate_name: desc.crate_name,
        bin_name: desc.bin_name,
        bin_path: desc.bin_path,
        test_name: desc.test_name,
        other_fields: desc.other_fields,
    };

    let self_info = serde_json::to_string(&core_test_desc).unwrap();

    std::fs::write(self_info_path, self_info)?;

    std::fs::write(
        tmpdir.join(cargo_difftests_core::CARGO_DIFFTESTS_VERSION_FILENAME),
        env!("CARGO_PKG_VERSION"),
    )?;

    // and for children
    let profraw_path =
        tmpdir.join(cargo_difftests_core::CARGO_DIFFTESTS_OTHER_PROFILE_FILENAME_TEMPLATE);
    Ok(DifftestsEnv {
        llvm_profile_file_name: "LLVM_PROFILE_FILE".into(),
        llvm_profile_file_value: profraw_path.into(),
    })
}
