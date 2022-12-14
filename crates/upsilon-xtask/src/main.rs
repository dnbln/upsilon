/*
 *        Copyright (c) 2022-2023 Dinu Blanovschi
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
    #[clap(alias = "gchk")]
    GitChecks,
    #[clap(name = "run-dev")]
    #[clap(alias = "run")]
    #[clap(alias = "r")]
    RunDev {
        #[clap(short, long)]
        dgql: bool,
        #[clap(short, long)]
        verbose: bool,
    },
    #[clap(name = "build-dev")]
    #[clap(alias = "build")]
    #[clap(alias = "b")]
    BuildDev {
        #[clap(short, long)]
        dgql: bool,
        #[clap(short, long)]
        verbose: bool,
    },
    #[clap(name = "test")]
    #[clap(alias = "t")]
    Test {
        #[clap(short, long)]
        dgql: bool,
        #[clap(short, long)]
        offline: bool,
        #[clap(short, long)]
        verbose: bool,
        #[clap(long)]
        no_fail_fast: bool,
        #[clap(long)]
        no_run: bool,
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
    #[clap(name = "graphql-schema")]
    #[clap(alias = "gqls")]
    GraphQLSchema,
}

fn build_dev(dgql: bool, verbose: bool) -> XtaskResult<()> {
    cargo_cmd!(
        "build",
        "-p", "upsilon-debug-data-driver",
        "--bin", "upsilon-debug-data-driver",
        "--features=dump_gql_response" => @if dgql,
        "--verbose" => @if verbose,
        @workdir = ws_root!(),
    )?;
    cargo_cmd!(
        "build",
        "-p", "upsilon-git-hooks",
        "--bin", "upsilon-git-hooks",
        "--features=build-bin",
        "--verbose" => @if verbose,
        @workdir = ws_root!(),
    )?;
    cargo_cmd!(
        "build",
        "-p", "upsilon-git-protocol-accesshook",
        "--bin", "upsilon-git-protocol-accesshook",
        "--verbose" => @if verbose,
        @workdir = ws_root!(),
    )?;
    cargo_cmd!(
        "build",
        "-p", "upsilon-web",
        "--verbose" => @if verbose,
        @workdir = ws_root!(),
    )?;

    cargo_cmd!(
        "build",
        "-p", "upsilon-gracefully-shutdown-host",
        "--bin", "upsilon-gracefully-shutdown-host",
        "--verbose" => @if verbose,
        @workdir = ws_root!(),
    )?;

    cargo_cmd!("build", "-p", "upsilon", "--bin", "upsilon", "--verbose" => @if verbose)?;

    Ok(())
}

fn run_tests(
    setup_testenv: &Path,
    offline: bool,
    verbose: bool,
    no_fail_fast: bool,
    no_run: bool,
) -> XtaskResult<()> {
    cargo_cmd!(
        "build" => @if no_run,
        "run" => @if !no_run,
        "-p",
        "setup_testenv",
        "--bin",
        "setup_testenv",
        "--verbose" => @if verbose,
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
        @env "RUST_LOG" => "info",
        @env "UPSILON_BIN_DIR" => ws_path!("target/debug"),
        @workdir = ws_root!(),
    )?;

    let mut upsilon_web_binary = ws_path!("target" / "debug" / "upsilon-web");
    upsilon_web_binary.set_extension(std::env::consts::EXE_EXTENSION);

    cargo_cmd!(
        "nextest",
        "run",
        "--all",
        "--offline" => @if offline,
        "--verbose" => @if verbose,
        "--no-fail-fast" => @if no_fail_fast,
        "--no-run" => @if no_run,
        @env "CLICOLOR_FORCE" => "1",
        @env "UPSILON_TEST_GUARD" => "1",
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
        @env "UPSILON_HOST_REPO_GIT" => ws_path!(".git"),
        @env "UPSILON_WEB_BIN" => upsilon_web_binary,
        @env "UPSILON_BIN_DIR" => ws_path!("target/debug"),
        @env "UPSILON_TESTSUITE_LOG" => "info",
        @workdir = ws_root!(),
    )?;

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

const ALIASES: &[&str] = &["uxrd"];

fn main_impl() -> XtaskResult<()> {
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
        App::BuildDev { dgql, verbose } => {
            build_dev(dgql, verbose)?;
        }
        App::RunDev { dgql, verbose } => {
            build_dev(dgql, verbose)?;

            cargo_cmd!(
                "run",
                "-p",
                "upsilon",
                "--",
                "web",
                @workdir = ws_path!("testenv"),
            )?;
        }
        App::Test {
            dgql,
            offline,
            verbose,
            no_fail_fast,
            no_run,
        } => {
            build_dev(dgql, verbose)?;

            let testenv_tests = ws_path!("testenv_tests");

            let setup_testenv = testenv_tests.join(std::process::id().to_string());

            if setup_testenv.exists() {
                std::fs::remove_dir_all(&setup_testenv)?;
            }

            std::fs::create_dir_all(&setup_testenv)?;

            let result = run_tests(&setup_testenv, offline, verbose, no_fail_fast, no_run);

            std::fs::remove_dir_all(&testenv_tests)?;

            result?;
        }
        App::PackRelease => {
            cargo_cmd!(
                "build",
                "-p", "upsilon-web",
                "--bin", "upsilon-web",
                "--release",
                @workdir = ws_root!(),
            )?;
            cargo_cmd!(
                "build",
                "-p", "upsilon",
                "--bin", "upsilon",
                "--release",
                @workdir = ws_root!(),
            )?;
            cargo_cmd!(
                "build",
                "-p", "upsilon-git-protocol-accesshook",
                "--bin", "upsilon-git-protocol-accesshook",
                "--release",
                @workdir = ws_root!(),
            )?;
            cargo_cmd!(
                "build",
                "-p", "upsilon-git-hooks",
                "--bin", "upsilon-git-hooks",
                "--features=build-bin",
                "--release",
                @workdir = ws_root!(),
            )?;

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
            for alias in ALIASES {
                cargo_cmd!(
                    "install",
                    "--bin",
                    alias,
                    "--path",
                    ws_path!("crates" / "upsilon-xtask"),
                )?;
            }
        }
        App::BuildDocs => {
            cmd_call!(
                "mdbook",
                "build",
                @workdir = ws_path!("docs"),
            )?;
        }
        App::ServeDocs => {
            cmd_call!(
                "mdbook",
                "serve",
                @workdir = ws_path!("docs"),
            )?;
        }

        App::PublishDocs => {
            cmd_call!(
                "mdbook",
                "build",
                @workdir = ws_path!("docs"),
            )?;

            #[cfg(windows)]
            cmd_call!(
                "./publish.bat",
                @workdir = ws_path!("docs"),
            )?;
            #[cfg(not(windows))]
            cmd_call!(
                "./publish",
                @workdir = ws_path!("docs"),
            )?;
        }
        App::GraphQLSchema => {
            cargo_cmd!(
                "run",
                "-p", "upsilon-api",
                "--bin", "dump_graphql_schema",
                "--", ws_path!("schemas" / "graphql" / "schema.graphql"),
                @workdir = ws_root!(),
            )?;
        }
    }

    Ok(())
}

fn main() {
    if let Err(err) = main_impl() {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}
