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

use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::{bail, format_err};
use clap::{Args, Parser};
use path_absolutize::Absolutize;
use upsilon_xtask::cmd::cargo_build_profiles_dir;
use upsilon_xtask::pkg::{BinTarget, Pkg, Profile};
use upsilon_xtask::{
    cargo_cmd, cargo_cmd_output, cmd_args, npm_cmd, ws_glob, ws_path, ws_root, XtaskResult
};

use crate::difftests::DiffTestsCommand;
use crate::test::{
    run_test_quick_cmd, run_test_support_examples_cmd, run_tests_cmd, TestCmd, TestQuickCmd
};
use crate::utils::{copy, extend_filext_new, rm};
use crate::ws_layout::{DOCS, WS_BIN_LAYOUT};

mod cargo_toml_style;
mod difftests;
mod dist;
mod test;
mod ukonf_ci;
mod utils;
mod ws_layout;

#[derive(Args, Debug)]
struct BuildDevArgs {
    /// Whether to dump the GraphQL API responses
    /// from `upsilon-debug-data-driver`, if used.
    #[clap(long)]
    dgql: bool,
    /// Whether to enable verbose logging.
    #[clap(short, long)]
    verbose: bool,
    /// The profile to use.
    #[clap(long, default_value_t = Profile::Dev)]
    profile: Profile,
}

impl BuildDevArgs {
    fn new_default(verbose: bool) -> Self {
        Self {
            dgql: false,
            verbose,
            profile: Profile::Dev,
        }
    }

    fn with_profile(self, profile: impl Into<Profile>) -> Self {
        Self {
            profile: profile.into(),
            ..self
        }
    }
}

#[derive(Clone, Debug)]
struct AbsolutizePathBuf(PathBuf);

impl AbsolutizePathBuf {
    fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl FromStr for AbsolutizePathBuf {
    type Err = std::io::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let path = PathBuf::from(s);
        Ok(Self(path.absolutize()?.into_owned()))
    }
}

/// Cargo workflows
#[derive(Parser, Debug)]
enum App {
    /// Formats the codebase.
    #[clap(name = "fmt")]
    Fmt,
    /// Checks the codebase for formatting errors.
    #[clap(name = "fmt-check")]
    FmtCheck,
    /// Performs some git checks.
    #[clap(name = "git-checks")]
    #[clap(alias = "gchk")]
    GitChecks {
        #[clap(short, long)]
        checkout: bool,
    },
    /// Runs the dev backend server.
    #[clap(name = "run-dev")]
    #[clap(alias = "run")]
    #[clap(alias = "r")]
    RunDev {
        #[clap(flatten)]
        build_dev_args: BuildDevArgs,
    },
    /// Builds the dev backend server
    /// and all required helper executables.
    #[clap(name = "build-dev")]
    #[clap(alias = "build")]
    #[clap(alias = "b")]
    BuildDev {
        #[clap(flatten)]
        build_dev_args: BuildDevArgs,
    },
    /// Runs the dev frontend server.
    #[clap(name = "frontend-run-dev")]
    #[clap(alias = "frun")]
    #[clap(alias = "fr")]
    FrontendRunDev,
    /// Runs the tests.
    #[clap(name = "test")]
    #[clap(alias = "t")]
    Test {
        #[clap(flatten)]
        cmd: TestCmd,
    },
    /// Similar to `test`, but runs only the tests
    /// that are (likely) affected by the changes since the
    /// tests were last run.
    #[clap(name = "test-quick")]
    #[clap(alias = "tq")]
    #[clap(alias = "qt")]
    TestQuick {
        #[clap(flatten)]
        cmd: TestQuickCmd,
    },
    /// Runs the given `upsilon-test-support` examples.
    #[clap(name = "test-support-examples")]
    #[clap(alias = "tse")]
    TestSupportExamples {
        /// The examples to run.
        examples: Vec<String>,
        /// The profile to use.
        #[clap(long, default_value_t = Profile::Dev)]
        profile: Profile,
    },
    /// Packs for release.
    #[clap(name = "pack-release")]
    #[clap(alias = "dist")]
    PackRelease,
    /// Installs some aliases.
    #[clap(name = "install-aliases")]
    InstallAliases,
    /// Builds the docs.
    #[clap(name = "build-docs")]
    #[clap(alias = "bd")]
    BuildDocs,
    /// Serves the docs.
    #[clap(name = "serve-docs")]
    #[clap(alias = "d")]
    ServeDocs,
    /// Builds the GraphQL schema.
    #[clap(name = "graphql-schema")]
    #[clap(alias = "gqls")]
    GraphQLSchema,
    /// Checks whether the GraphQL schema is up-to-date.
    #[clap(name = "graphql-schema-check")]
    #[clap(alias = "gqlschk")]
    GraphQLSchemaCheck,
    /// Checks that the dependencies in `Cargo.toml` files are ordered.
    #[clap(name = "check-cargo-dep-order")]
    #[clap(alias = "ccdo")]
    CheckCargoTomlDepOrder,
    /// Checks that the dependencies in `Cargo.toml` workspace members
    /// are not redeclared from the `workspace.dependencies`.
    #[clap(name = "check-cargo-dep-from-workspace")]
    #[clap(alias = "ccdw")]
    CheckCargoDepFromWorkspace,
    #[clap(name = "install-cranky")]
    InstallCranky,
    #[clap(name = "clean")]
    Clean,
    /// Lints the workspace.
    #[clap(name = "lint")]
    #[clap(alias = "l")]
    #[clap(alias = "check")]
    #[clap(alias = "clippy")]
    Lint,
    /// Prints the lint arguments.
    #[clap(name = "lint-args")]
    LintArgs,
    /// Compiles the given ukonf file to YAML.
    #[clap(name = "ukonf-to-yaml")]
    UkonfToYaml { from: PathBuf, to: PathBuf },
    /// Generates the CI files.
    #[clap(name = "gen-ci-files")]
    GenCiFiles,
    /// Checks that the CI files are up-to-date.
    #[clap(name = "check-ci-files-up-to-date")]
    CheckCiFilesUpToDate,
    /// Cleans the profiling data obtained while building with a
    /// `-C instrument-coverage` profile.
    #[clap(name = "clean-instrumentation-files")]
    CleanInstrumentationFiles,

    /// Installs the `cargo-binutils` package.
    /// This is required for `cargo-difftests` (and `xtask test-quick`) to work.
    #[clap(name = "install-binutils")]
    InstallBinutils,

    /// Helpers for `cargo-difftests`.
    #[clap(name = "difftests")]
    Difftests {
        #[clap(subcommand)]
        command: DiffTestsCommand,
    },

    /// Publishes the `cargo-difftests*` crates to crates.io.
    #[clap(name = "publish-difftests-crates")]
    PublishDifftestsCrates,
}

fn copy_test_artifact(bin: &BinTarget, profile: Profile, custom_bin_dir: &Path) -> XtaskResult<()> {
    let src = bin.path_in_profile(profile);
    let dst = custom_bin_dir.join(src.file_name().unwrap());
    copy(&src, &dst)?;
    Ok(())
}

fn copy_test_artifacts(profile: Profile, custom_bin_dir: &Path) -> XtaskResult<()> {
    let bins_to_copy = &[
        &WS_BIN_LAYOUT.upsilon_web_main,
        &WS_BIN_LAYOUT.upsilon_gracefully_shutdown_host_main,
        &WS_BIN_LAYOUT.upsilon_git_hooks_main,
        &WS_BIN_LAYOUT.upsilon_git_protocol_accesshook_main,
        &WS_BIN_LAYOUT.upsilon_main,
    ];

    for bin in bins_to_copy {
        copy_test_artifact(bin, profile, custom_bin_dir)?;
    }

    Ok(())
}

fn build_dev(dgql: bool, verbose: bool, profile: Profile) -> XtaskResult<()> {
    WS_BIN_LAYOUT
        .upsilon_debug_data_driver_main
        .build(cmd_args!(
            "--features=dump_gql_response" => @if dgql,
            "--verbose" => @if verbose,
            "--profile", profile.name(),
        ))?;
    WS_BIN_LAYOUT.upsilon_git_hooks_main.build(cmd_args!(
        "--features=build-bin",
        "--verbose" => @if verbose,
        "--profile", profile.name(),
    ))?;
    WS_BIN_LAYOUT
        .upsilon_git_protocol_accesshook_main
        .build(cmd_args!(
            "--verbose" => @if verbose,
            "--profile", profile.name(),
        ))?;
    WS_BIN_LAYOUT.upsilon_web_main.build(cmd_args!(
        "--verbose" => @if verbose,
        "--profile", profile.name(),
    ))?;

    WS_BIN_LAYOUT
        .upsilon_gracefully_shutdown_host_main
        .build(cmd_args!(
            "--verbose" => @if verbose,
            "--profile", profile.name(),
        ))?;

    WS_BIN_LAYOUT.upsilon_main.build(cmd_args!(
        "--verbose" => @if verbose,
        "--profile", profile.name(),
    ))?;

    Ok(())
}

fn gqls_path() -> PathBuf {
    ws_path!("schemas" / "graphql" / "schema.graphql")
}

fn lint_args() -> XtaskResult<Vec<String>> {
    let cranky_config = cargo_cranky::config::CrankyConfig::get_config()?;
    let clippy_flags = cranky_config.extra_right_args();

    Ok(clippy_flags)
}

const ALIASES: &[&str] = &["uxrd"];

fn clean_unneeded_instrumentation_files() -> XtaskResult<()> {
    let paths = ws_glob!("**" / "default_*.profraw")?;
    for path in paths {
        rm(&path)?;
    }

    let p = cargo_build_profiles_dir();
    rm(&p)?;

    Ok(())
}

fn main_impl() -> XtaskResult<()> {
    pretty_env_logger::init_custom_env("UXTASK_LOG");

    let app: App = App::parse();

    match app {
        App::Fmt => {
            cargo_cmd!("fmt", "--all", @workdir = ws_root!())?;
        }
        App::FmtCheck => {
            cargo_cmd!("fmt", "--all", "--check", @workdir = ws_root!())?;
        }
        App::GitChecks { checkout } => {
            let repo = upsilon_xtask::git_checks::get_repo(&ws_root!())?;

            if !checkout {
                upsilon_xtask::git_checks::linear_history(&repo)?;
            }
        }
        App::BuildDev {
            build_dev_args:
                BuildDevArgs {
                    dgql,
                    verbose,
                    profile,
                },
        } => {
            build_dev(dgql, verbose, profile)?;
        }
        App::RunDev {
            build_dev_args:
                BuildDevArgs {
                    dgql,
                    verbose,
                    profile,
                },
        } => {
            build_dev(dgql, verbose, profile)?;

            cargo_cmd!(
                "run",
                ...WS_BIN_LAYOUT.upsilon_web_main.run_args(),
                @workdir = ws_path!("testenv"),
            )?;
        }
        App::FrontendRunDev => {
            DOCS.build()?;

            copy(
                ws_path!("docs" / "book"),
                ws_path!("client" / "static" / "docs"),
            )?;

            npm_cmd!(
                "run",
                "dev",
                @workdir = ws_path!("client")
            )?;
        }
        App::Test { cmd } => {
            run_tests_cmd(cmd)?;
        }
        App::TestQuick { cmd } => {
            run_test_quick_cmd(cmd)?;
        }
        App::TestSupportExamples { examples, profile } => {
            run_test_support_examples_cmd(examples, profile)?;
        }
        App::PackRelease => {
            dist::dist()?;
        }
        App::InstallAliases => {
            for alias in ALIASES {
                cargo_cmd!(
                    "install",
                    "--bin",
                    alias,
                    "--path",
                    ws_path!("dev" / "upsilon-xtask"),
                )?;
            }
        }
        App::BuildDocs => {
            DOCS.build()?;
        }
        App::ServeDocs => {
            DOCS.serve()?;
        }

        App::GraphQLSchema => {
            cargo_cmd!(
                "run",
                ...WS_BIN_LAYOUT.upsilon_dump_gql_schema_main.run_args(),
                "--", gqls_path(),
                @workdir = ws_root!(),
            )?;
        }
        App::GraphQLSchemaCheck => {
            let p = gqls_path();
            let temp_p = extend_filext_new(&p);

            cargo_cmd!(
                "run",
                ...WS_BIN_LAYOUT.upsilon_dump_gql_schema_main.run_args(),
                "--", &temp_p,
                @workdir = ws_root!(),
            )?;

            let contents = fs::read_to_string(&p)?;
            let new_contents = fs::read_to_string(&temp_p)?;

            let up_to_date = contents == new_contents;

            if !up_to_date {
                let diff = upsilon_diff_util::build_diff(&contents, &new_contents);

                eprintln!("GraphQL schema is out of date. Run `cargo xtask gqls` to update it.");
                eprintln!("Diff:");
                eprintln!("=====================");
                eprintln!("{diff}");
                eprintln!("=====================");
            }

            fs::remove_file(&temp_p)?;

            if !up_to_date {
                bail!("GraphQL schema is out of date");
            }
        }
        App::CheckCargoTomlDepOrder => {
            cargo_toml_style::run_check_cargo_toml_dep_order_cmd()?;
        }
        App::CheckCargoDepFromWorkspace => {
            cargo_toml_style::run_check_cargo_deps_from_workspace_cmd()?;
        }
        App::InstallCranky => {
            WS_BIN_LAYOUT.cargo_cranky_main.install()?;
        }
        App::Clean => {
            cargo_cmd!("clean", @workdir = ws_root!())?;
        }
        App::Lint => {
            let clippy_flags = lint_args()?;

            cargo_cmd!(
                "clippy",
                "--all",
                "--",
                ...clippy_flags,
                @workdir = ws_root!(),
            )?;
        }
        App::LintArgs => {
            let clippy_flags = lint_args()?;

            println!("{}", clippy_flags.join(" "));
        }
        App::UkonfToYaml { from, to } => {
            ukonf_ci::run_ukonf_to_yaml_cmd(from, &to)?;
        }
        App::GenCiFiles => {
            ukonf_ci::run_gen_ci_files_cmd()?;
        }
        App::CheckCiFilesUpToDate => {
            ukonf_ci::run_check_ci_files_up_to_date_cmd()?;
        }
        App::CleanInstrumentationFiles => {
            clean_unneeded_instrumentation_files()?;
        }
        App::InstallBinutils => {
            Pkg::crates_io("cargo-binutils").install()?;
        }
        App::Difftests { command } => {
            difftests::run(command)?;
        }
        App::PublishDifftestsCrates => {
            Pkg::dev_pkg("cargo-difftests-core").publish()?;
            Pkg::dev_pkg("cargo-difftests-testclient").publish()?;
            Pkg::dev_pkg("cargo-difftests").publish()?;
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
