/*
 *        Copyright (c) 2022 Dinu Blanovschi
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

#![feature(try_blocks)]

use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use zip::write::{FileOptions, ZipWriter};

use crate::cmd::cargo_cmd;
use crate::result::XtaskResult;
use crate::ws::ws_path;

mod cmd;
mod git_checks;
mod result;
mod ws;

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "fmt")]
    Fmt,
    #[clap(name = "fmt-check")]
    FmtCheck,
    #[clap(name = "git-checks")]
    GitChecks,
    #[clap(name = "run-dev")]
    RunDev,
    #[clap(name = "pack-release")]
    PackRelease,
}

fn main() -> XtaskResult<()> {
    let app: App = App::parse();

    match app {
        App::Fmt => {
            cargo_cmd!("fmt", "--all")?;
        }
        App::FmtCheck => {
            cargo_cmd!("fmt", "--all", "--check")?;
        }
        App::GitChecks => {
            let repo = git_checks::get_repo()?;

            git_checks::linear_history(&repo)?;
        }
        App::RunDev => {
            cargo_cmd!("build", "-p", "upsilon-web", @logging-error-and-returnok);
            cargo_cmd!(
                "run",
                "-p",
                "upsilon",
                "--",
                "web",
                @workdir = ws_path!("testenv"),
                @logging-error-and-returnok,
            );
        }
        App::PackRelease => {
            cargo_cmd!("build", "-p", "upsilon-web", "--bin", "upsilon-web", "--release", @logging-error-and-returnok);
            cargo_cmd!("build", "-p", "upsilon", "--bin", "upsilon", "--release", @logging-error-and-returnok);

            let release_zip_file = std::env::var("UPSILON_RELEASE_ZIP_PATH")
                .map_or_else(|_| ws_path!("releases" / "release.zip"), PathBuf::from);

            if let Some(parent) = release_zip_file.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let mut wr = ZipWriter::new(std::fs::File::create(release_zip_file)?);
            let options =
                FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

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

            wr.finish()?;
        }
    }

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

    let mut buf = [0u8; 65536];
    let mut f = std::fs::File::open(path)?;

    loop {
        let read = f.read(&mut buf)?;

        if read == 0 {
            break;
        }

        wr.write_all(&buf[..read])?;
    }

    Ok(())
}
