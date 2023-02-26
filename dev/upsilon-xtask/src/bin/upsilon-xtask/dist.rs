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
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use upsilon_xtask::{cmd_args, ws_path, XtaskResult};
use zip::write::FileOptions;
use zip::ZipWriter;

use crate::ws_layout::WS_BIN_LAYOUT;

pub fn dist() -> XtaskResult<()> {
    WS_BIN_LAYOUT
        .upsilon_web_main
        .build(cmd_args!("--release"))?;
    WS_BIN_LAYOUT.upsilon_main.build(cmd_args!("--release"))?;
    WS_BIN_LAYOUT
        .upsilon_git_protocol_accesshook_main
        .build(cmd_args!("--release"))?;
    WS_BIN_LAYOUT
        .upsilon_git_hooks_main
        .build(cmd_args!("--release", "--features=build-bin"))?;

    let release_zip_file = std::env::var("UPSILON_RELEASE_ZIP_PATH")
        .map_or_else(|_| ws_path!("releases" / "release.zip"), PathBuf::from);

    if let Some(parent) = release_zip_file.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut wr = ZipWriter::new(fs::File::create(release_zip_file)?);
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    write_bin_file_to_zip(
        &mut wr,
        "bin/upsilon",
        ws_path!("target" / "release" / "upsilon"),
        options,
    )?;
    write_bin_file_to_zip(
        &mut wr,
        "bin/upsilon-web",
        ws_path!("target" / "release" / "upsilon-web"),
        options,
    )?;
    write_bin_file_to_zip(
        &mut wr,
        "bin/upsilon-git-protocol-accesshook",
        ws_path!("target" / "release" / "upsilon-git-protocol-accesshook"),
        options,
    )?;
    write_bin_file_to_zip(
        &mut wr,
        "bin/upsilon-git-hooks",
        ws_path!("target" / "release" / "upsilon-git-hooks"),
        options,
    )?;

    wr.finish()?;

    Ok(())
}

fn write_bin_file_to_zip<W: Write + Seek>(
    wr: &mut ZipWriter<W>,
    zip_path: impl AsRef<Path>,
    path: impl AsRef<Path>,
    options: FileOptions,
) -> XtaskResult<()> {
    wr.start_file(
        zip_path
            .as_ref()
            .with_extension(std::env::consts::EXE_EXTENSION)
            .to_str()
            .expect("Cannot convert to string"),
        options,
    )?;

    let path = path
        .as_ref()
        .with_extension(std::env::consts::EXE_EXTENSION);

    let mut buf = [0u8; 0x0001_0000];
    let mut f = fs::File::open(path)?;

    loop {
        let read = f.read(&mut buf)?;

        if read == 0 {
            break;
        }

        wr.write_all(&buf[..read])?;
    }

    Ok(())
}
