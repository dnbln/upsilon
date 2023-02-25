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

use std::fs;
use std::path::{Path, PathBuf};

use log::info;
use upsilon_xtask::XtaskResult;

pub fn extend_filext_new(p: impl AsRef<Path>) -> PathBuf {
    p.as_ref().with_file_name(format!(
        "{}.new",
        p.as_ref().file_name().unwrap().to_string_lossy()
    ))
}

pub fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> XtaskResult<()> {
    let from = from.as_ref();
    let to = to.as_ref();

    if from.is_file() {
        if let Some(p) = to.parent() {
            if !p.exists() {
                fs::create_dir_all(p)?;
            }
        }

        fs::copy(from, to)?;
        return Ok(());
    }

    if !to.exists() {
        fs::create_dir_all(to)?;
    }

    fs_extra::dir::copy(
        from,
        to,
        &fs_extra::dir::CopyOptions {
            overwrite: true,
            copy_inside: false,
            ..Default::default()
        },
    )?;

    Ok(())
}

pub fn rm(p: &Path) -> XtaskResult<()> {
    if !p.exists() {
        return Ok(());
    }

    info!("Removing {}", p.display());

    if p.is_file() {
        fs::remove_file(p)?;
    } else {
        fs::remove_dir_all(p)?;
    }

    Ok(())
}
