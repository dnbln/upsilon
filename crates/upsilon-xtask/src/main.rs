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

use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use clap::Parser;
use upsilon_xtask::{cargo_cmd, cmd_call, ws_path, ws_root, XtaskResult};
use zip::write::{FileOptions, ZipWriter};

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "fmt")]
    Fmt,
    #[clap(name = "fmt-check")]
    FmtCheck,
    #[clap(name = "git-checks")]
    GitChecks,
    #[clap(name = "run-dev")]
    #[clap(alias = "run")]
    #[clap(alias = "r")]
    RunDev {
        #[clap(short, long)]
        dgql: bool,
    },
    #[clap(name = "build-dev")]
    #[clap(alias = "build")]
    #[clap(alias = "b")]
    BuildDev {
        #[clap(short, long)]
        dgql: bool,
    },
    #[clap(name = "test")]
    #[clap(alias = "t")]
    Test {
        #[clap(short, long)]
        dgql: bool,
    },
    #[clap(name = "pack-release")]
    PackRelease,
    #[clap(name = "install-aliases")]
    InstallAliases,
    #[clap(name = "build-docs")]
    #[clap(alias = "bd")]
    BuildDocs,
    #[clap(name = "serve-docs")]
    #[clap(alias = "d")]
    ServeDocs,
    #[clap(name = "publish-docs")]
    #[clap(alias = "pd")]
    PublishDocs,
}

fn build_dev(dgql: bool) -> XtaskResult<()> {
    if dgql {
        cargo_cmd!(
                    "build",
                    "-p", "upsilon-debug-data-driver",
                    "--features", "dump_gql_response",
                    @workdir = ws_root!(),
                    @logging-error-and-returnok);
    } else {
        cargo_cmd!(
                    "build",
                    "-p", "upsilon-debug-data-driver",
                    @workdir = ws_root!(),
                    @logging-error-and-returnok);
    }
    cargo_cmd!(
                "build",
                "-p", "upsilon-git-hooks",
                "--bin", "upsilon-git-hooks",
                "--features=build-bin",
                @workdir = ws_root!(),
                @logging-error-and-returnok);
    cargo_cmd!(
                "build",
                "-p", "upsilon-git-protocol-accesshook",
                @workdir = ws_root!(),
                @logging-error-and-returnok);
    cargo_cmd!(
                "build",
                "-p", "upsilon-web",
                @workdir = ws_root!(),
                @logging-error-and-returnok);

    cargo_cmd!(
        "build",
        "-p",
        "upsilon",
        @logging-error-and-returnok,
    );

    Ok(())
}

fn main() -> XtaskResult<()> {
    let app: App = App::parse();

    match app {
        App::Fmt => {
            cargo_cmd!("fmt", "--all", @workdir = ws_root!())?;
        }
        App::FmtCheck => {
            cargo_cmd!("fmt", "--all", "--check", @workdir = ws_root!())?;
        }
        App::GitChecks => {
            let repo = upsilon_xtask::git_checks::get_repo(&ws_root!())?;

            upsilon_xtask::git_checks::linear_history(&repo)?;
        }
        App::BuildDev { dgql } => {
            build_dev(dgql)?;
        }
        App::RunDev { dgql } => {
            build_dev(dgql)?;

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
        App::Test { dgql } => {
            build_dev(dgql)?;

            cargo_cmd!(
                "nextest",
                "run",
                "--all",
                @env "CLICOLOR_FORCE" => "1",
                @env "UPSILON_TEST_GUARD" => "1",
                @workdir = ws_root!(),
                @logging-error-and-returnok,
            );
        }
        App::PackRelease => {
            cargo_cmd!(
                "build",
                "-p", "upsilon-web",
                "--bin", "upsilon-web",
                "--release",
                @workdir = ws_root!(),
                @logging-error-and-returnok);
            cargo_cmd!(
                "build",
                "-p", "upsilon",
                "--bin", "upsilon",
                "--release",
                @workdir = ws_root!(),
                @logging-error-and-returnok);
            cargo_cmd!(
                "build",
                "-p", "upsilon-git-protocol-accesshook",
                "--bin", "upsilon-git-protocol-accesshook",
                "--release",
                @workdir = ws_root!(),
                @logging-error-and-returnok);
            cargo_cmd!(
                "build",
                "-p", "upsilon-git-hooks",
                "--bin", "upsilon-git-hooks",
                "--features=build-bin",
                "--release",
                @workdir = ws_root!(),
                @logging-error-and-returnok);

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
        }
        App::InstallAliases => {
            cargo_cmd!(
                "install",
                "--bin", "uxrd",
                "--path", ws_path!("crates" / "upsilon-xtask"),
                @logging-error-and-returnok);
        }
        App::BuildDocs => {
            cmd_call!(
                "mdbook",
                "build",
                @workdir = ws_path!("docs"),
                @logging-error-and-returnok);
        }
        App::ServeDocs => {
            cmd_call!(
                "mdbook",
                "serve",
                @workdir = ws_path!("docs"),
                @logging-error-and-returnok);
        }

        App::PublishDocs => {
            cmd_call!(
                "mdbook",
                "build",
                @workdir = ws_path!("docs"),
                @logging-error-and-returnok);

            #[cfg(windows)]
            cmd_call!(
                "./publish.bat",
                @workdir = ws_path!("docs"),
                @logging-error-and-returnok);
            #[cfg(not(windows))]
            cmd_call!(
                "./publish",
                @workdir = ws_path!("docs"),
                @logging-error-and-returnok);
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
