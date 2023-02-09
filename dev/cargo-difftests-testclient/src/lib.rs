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

#![cfg(cargo_difftests)]

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use cargo_difftests_core::CoreTestDesc;

pub struct TestDesc {
    pub pkg_name: String,
    pub crate_name: String,
    pub bin_name: Option<String>,
    pub bin_path: PathBuf,
    pub test_name: String,

    pub other_fields: HashMap<String, String>,
}

pub struct DifftestsEnv {
    llvm_profile_file_name: OsString,
    llvm_profile_file_value: OsString,
}

impl DifftestsEnv {
    pub fn env_for_children(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> {
        std::iter::once((
            self.llvm_profile_file_name.as_os_str(),
            self.llvm_profile_file_value.as_os_str(),
        ))
    }
}

extern "C" {
    fn __llvm_profile_set_filename(filename: *const std::ffi::c_char);
}

pub fn init(desc: TestDesc, tmpdir: &Path) -> std::io::Result<DifftestsEnv> {
    if tmpdir.exists() {
        std::fs::remove_dir_all(tmpdir)?;
    }
    std::fs::create_dir_all(tmpdir)?;

    let self_profile_file = tmpdir.join("self.profraw");

    let self_profile_file_str = self_profile_file.to_str().unwrap();

    let self_profile_file_str_c = std::ffi::CString::new(self_profile_file_str).unwrap();

    unsafe {
        __llvm_profile_set_filename(self_profile_file_str_c.as_ptr());
    }

    let self_info_path = tmpdir.join("self.json");

    let mut core_test_desc = CoreTestDesc {
        pkg_name: desc.pkg_name,
        crate_name: desc.crate_name,
        bin_name: desc.bin_name,
        bin_path: desc.bin_path,
        test_name: desc.test_name,
        other_fields: desc.other_fields,
    };

    core_test_desc.other_fields.insert(
        "CARGO_DIFFTESTS_VERSION".into(),
        env!("CARGO_PKG_VERSION").into(),
    );

    let self_info = serde_json::to_string(&core_test_desc).unwrap();

    std::fs::write(self_info_path, self_info)?;

    // and for children
    let profraw_path = tmpdir.join("%m_%p.profraw");
    Ok(DifftestsEnv {
        llvm_profile_file_name: "LLVM_PROFILE_FILE".into(),
        llvm_profile_file_value: profraw_path.into(),
    })
}
