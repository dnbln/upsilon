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

use std::cell::RefCell;
use std::ffi::OsString;
use std::fs;
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{bail, format_err, Context};
use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches, Parser};
use log::info;
use path_slash::{PathBufExt, PathExt};
use toml_edit::{Item, Key, TableLike};
use ukonf::value::{UkonfObject, UkonfValue};
use ukonf::{Scope, UkonfFnError, UkonfFunctions};
use upsilon_xtask::cmd::cargo_build_profiles_dir;
use upsilon_xtask::difftests::{DiffTestsCommand, DirtyAlgo};
use upsilon_xtask::pkg::Pkg;
use upsilon_xtask::{
    cargo_cmd, cmd_args, difftests, npm_cmd, ws_bin_path, ws_glob, ws_path, ws_root, XtaskResult
};
use ws_layout::WS_BIN_LAYOUT;
use zip::write::{FileOptions, ZipWriter};

use crate::ws_layout::{WsPkgLayout, DOCS, WS_PKG_LAYOUT};

pub mod ws_layout;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TestSuiteBinGroup {
    name: &'static str,
    aliases: &'static [&'static str],

    bin: &'static str,
    nextest_filter: &'static str,

    needs_upsilon_clone: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum TestGroup {
    TestSuiteBin(TestSuiteBinGroup),
    Package {
        name: &'static str,
        aliases: &'static [&'static str],

        package: &'static str,
        needs_testenv: bool,
        nextest_filter: &'static str,
    },
}

impl TestGroup {
    fn name(&self) -> &'static str {
        match self {
            TestGroup::TestSuiteBin(group) => group.name,
            TestGroup::Package { name, .. } => name,
        }
    }

    fn aliases(&self) -> &'static [&'static str] {
        match self {
            TestGroup::TestSuiteBin(group) => group.aliases,
            TestGroup::Package { aliases, .. } => aliases,
        }
    }

    fn nextest_filter(&self) -> &'static str {
        match self {
            TestGroup::TestSuiteBin(group) => group.nextest_filter,
            TestGroup::Package { nextest_filter, .. } => nextest_filter,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TestenvConfig {
    needs_upsilon_clone: bool,
}

impl TestenvConfig {
    fn merge_with(&mut self, other: Option<TestenvConfig>) {
        let Some(other) = other else {return;};
        self.needs_upsilon_clone |= other.needs_upsilon_clone;
    }

    fn all() -> Self {
        Self {
            needs_upsilon_clone: true,
        }
    }

    fn compile(self) -> Option<TestenvConfig> {
        if let TestenvConfig {
            needs_upsilon_clone: false,
        } = &self
        {
            None
        } else {
            Some(self)
        }
    }
}

impl TestGroup {
    fn testenv_config(&self) -> Option<TestenvConfig> {
        match self {
            TestGroup::TestSuiteBin(TestSuiteBinGroup {
                needs_upsilon_clone,
                ..
            }) => TestenvConfig {
                needs_upsilon_clone: *needs_upsilon_clone,
            }
            .compile(),
            TestGroup::Package { needs_testenv, .. } => TestenvConfig {
                needs_upsilon_clone: *needs_testenv,
            }
            .compile(),
        }
    }
}

const fn test_suite_bin_group_needs_upsilon_clone_patch(
    mut test_group: TestSuiteBinGroup,
) -> TestSuiteBinGroup {
    test_group.needs_upsilon_clone = true;
    test_group
}

macro_rules! expand_known_test_group {
    ({@testsuitebin $name:literal: $bin:literal}) => {
        TestGroup::TestSuiteBin(TestSuiteBinGroup {
            name: $name,
            aliases: &[],
            bin: $bin,
            nextest_filter: concat!("package(=upsilon-testsuite) & binary(=", $bin, ")"),
            needs_upsilon_clone: false,
        })
    };
    ({@testsuitebin $name:literal: $bin:literal, patch: [$($patch:ident),* $(,)?]$(,)?}) => {{
        let group = TestSuiteBinGroup {
            name: $name,
            aliases: &[],
            bin: $bin,
            nextest_filter: concat!("package(=upsilon-testsuite) & binary(=", $bin, ")"),
            needs_upsilon_clone: false,
        };
        $(
            let group = $patch(group);
        )*
        TestGroup::TestSuiteBin(group)
    }};
    ({@package $name:literal}) => {
        TestGroup::Package {
            name: $name,
            aliases: &[],
            package: $name,
            needs_testenv: false,
            nextest_filter: concat!("package(=", $name, ")"),
        }
    };
    ({@package $name:literal, aliases: $aliases:tt}) => {
        TestGroup::Package {
            name: $name,
            aliases: &$aliases,
            package: $name,
            needs_testenv: false,
            nextest_filter: concat!("package(=", $name, ")"),
        }
    };
}

macro_rules! known_test_groups {
    ($($group:tt),* $(,)?) => {
        const KNOWN_TEST_GROUPS: &[TestGroup] = &[
            $(
                expand_known_test_group!($group),
            )*
        ];
    };
}

known_test_groups! {
    {@testsuitebin
        "git-clone": "git_clone",
        patch: [test_suite_bin_group_needs_upsilon_clone_patch],
    },
    {@testsuitebin
        "git-graphql": "git_graphql",
        patch: [test_suite_bin_group_needs_upsilon_clone_patch],
    },
    {@testsuitebin "github-mirror": "github_mirror"},
    {@testsuitebin
        "lookup-repo": "lookup_repo",
        patch: [test_suite_bin_group_needs_upsilon_clone_patch],
    },
    {@testsuitebin "viewer": "viewer"},
    {@package "ukonf"},
    {@package "upsilon-shell", aliases: ["ush"]},
}

#[derive(Debug, Clone)]
struct TestGroups {
    groups: Vec<&'static TestGroup>,
    testenv_config: Option<TestenvConfig>,
}

impl TestGroups {
    fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        for group in &self.groups {
            args.push("-E".to_string());
            args.push(group.nextest_filter().to_string());
        }
        args
    }

    fn testenv_config(&self) -> Option<TestenvConfig> {
        if self.groups.is_empty() {
            Some(TestenvConfig::all())
        } else {
            self.testenv_config
        }
    }
}

impl FromArgMatches for TestGroups {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let mut groups = vec![];
        let mut testenv_config = TestenvConfig {
            needs_upsilon_clone: false,
        };
        for group in KNOWN_TEST_GROUPS {
            if matches.get_flag(group.name()) {
                groups.push(group);
                testenv_config.merge_with(group.testenv_config());
            }
        }

        Ok(Self {
            groups,
            testenv_config: testenv_config.compile(),
        })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        let mut groups = vec![];
        for group in KNOWN_TEST_GROUPS {
            if matches.get_flag(group.name()) {
                groups.push(group);
            }
        }

        self.groups = groups;

        Ok(())
    }
}

impl Args for TestGroups {
    fn augment_args(mut cmd: Command) -> Command {
        for group in KNOWN_TEST_GROUPS {
            cmd = cmd.arg(
                Arg::new(group.name())
                    .long(group.name())
                    .action(ArgAction::SetTrue)
                    .aliases(group.aliases().iter().map(|it| it.to_string()))
                    .help(format!(
                        "Filter tests from the {group} test group",
                        group = group.name()
                    )),
            );
        }

        cmd
    }

    fn augment_args_for_update(cmd: Command) -> Command {
        Self::augment_args(cmd)
    }
}

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
    #[clap(long)]
    profile: Option<String>,
}

impl BuildDevArgs {
    fn new_default(verbose: bool) -> Self {
        Self {
            dgql: false,
            verbose,
            profile: None,
        }
    }

    fn with_profile(self, profile: impl Into<String>) -> Self {
        Self {
            profile: Some(profile.into()),
            ..self
        }
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
        #[clap(long)]
        doc: bool,
        #[clap(long)]
        no_capture: bool,
        #[clap(long)]
        clean_profiles_between_steps: bool,
        #[clap(long)]
        profile: Option<String>,

        #[clap(flatten)]
        test_groups: TestGroups,

        test_filters: Vec<String>,
    },
    /// Similar to `test`, but runs only the tests
    /// that are (likely) affected by the changes since the
    /// tests were last run.
    #[clap(name = "test-quick")]
    #[clap(alias = "tq")]
    #[clap(alias = "qt")]
    TestQuick {
        #[clap(short, long)]
        dgql: bool,
        #[clap(short, long)]
        offline: bool,
        #[clap(short, long)]
        verbose: bool,
        #[clap(long)]
        no_fail_fast: bool,
        #[clap(long)]
        no_capture: bool,
        #[clap(long)]
        clean_profiles_between_steps: bool,
        #[clap(long)]
        from_index: bool,
        #[clap(long, default_value_t = Default::default())]
        algo: DirtyAlgo,
        #[clap(long)]
        commit: Option<git2::Oid>,
        #[clap(long, default_value = "difftests")]
        profile: String,
    },
    /// Runs the given `upsilon-test-support` examples.
    #[clap(name = "test-support-examples")]
    #[clap(alias = "tse")]
    TestSupportExamples {
        /// The examples to run.
        examples: Vec<String>,
        /// The profile to use.
        profile: Option<String>,
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

fn build_dev(dgql: bool, verbose: bool, profile: Option<&str>) -> XtaskResult<()> {
    WS_BIN_LAYOUT
        .upsilon_debug_data_driver_main
        .build(cmd_args!(
            "--features=dump_gql_response" => @if dgql,
            "--verbose" => @if verbose,
            ...["--profile", profile] => @if let Some(profile) = profile,
        ))?;
    WS_BIN_LAYOUT.upsilon_git_hooks_main.build(cmd_args!(
        "--features=build-bin",
        "--verbose" => @if verbose,
        ...["--profile", profile] => @if let Some(profile) = profile,
    ))?;
    WS_BIN_LAYOUT
        .upsilon_git_protocol_accesshook_main
        .build(cmd_args!(
            "--verbose" => @if verbose,
            ...["--profile", profile] => @if let Some(profile) = profile,
        ))?;
    WS_BIN_LAYOUT.upsilon_web_main.build(cmd_args!(
        "--verbose" => @if verbose,
        ...["--profile", profile] => @if let Some(profile) = profile,
    ))?;

    WS_BIN_LAYOUT
        .upsilon_gracefully_shutdown_host_main
        .build(cmd_args!(
            "--verbose" => @if verbose,
            ...["--profile", profile] => @if let Some(profile) = profile,
        ))?;

    WS_BIN_LAYOUT.upsilon_main.build(cmd_args!(
        "--verbose" => @if verbose,
        ...["--profile", profile] => @if let Some(profile) = profile,
    ))?;

    Ok(())
}

fn run_doctests(verbose: bool, no_fail_fast: bool, profile: Option<&str>) -> XtaskResult<()> {
    cargo_cmd!(
        "test",
        "--doc",
        "--workspace",
        "--verbose" => @if verbose,
        "--no-fail-fast" => @if no_fail_fast,
        ...["--profile", profile] => @if let Some(profile) = profile,
        @workdir = ws_root!(),
    )?;

    Ok(())
}

fn run_tests(
    setup_testenv: &Path,
    offline: bool,
    verbose: bool,
    no_fail_fast: bool,
    no_run: bool,
    no_capture: bool,
    clean_profiles_between_steps: bool,
    test_filters: &[String],
    test_groups: &TestGroups,
    doc: bool,
    profile: Option<&str>,
) -> XtaskResult<()> {
    if doc {
        macro_rules! redundant_arg {
            ($arg:expr, $value:expr, $flag:expr) => {
                if $value {
                    log::error!(
                        "The --{} flag is redundant when using the --{} flag",
                        $arg,
                        $flag
                    );
                }
            };
        }

        redundant_arg!("no-run", no_run, "doc");
        redundant_arg!("offline", offline, "doc");

        return run_doctests(verbose, no_fail_fast, profile);
    }

    let prof = profile.unwrap_or("debug");

    if let Some(testenv_config) = test_groups.testenv_config() {
        cargo_cmd!(
            "build" => @if no_run,
            "run" => @if !no_run,
            "-p",
            "upsilon-setup-testenv",
            "--bin",
            "upsilon-setup-testenv",
            "--verbose" => @if verbose,
            ...["--profile", profile] => @if let Some(profile) = profile,
            @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
            @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
            @env "UPSILON_SETUP_TESTENV_UPSILON_CLONE" => "1" => @if testenv_config.needs_upsilon_clone,
            @env "RUST_LOG" => "info",
            @env "UPSILON_BIN_DIR" => ws_path!("target" / prof),
            @workdir = ws_root!(),
        )?;
    }

    if clean_profiles_between_steps {
        clean_unneeded_instrumentation_files()?;
    }

    let upsilon_web_binary = ws_bin_path!(profile = prof, name = "upsilon-web");

    let upsilon_gracefully_shutdown_host_binary =
        ws_bin_path!(profile = prof, name = "upsilon-gracefully-shutdown-host");

    cargo_cmd!(
        "nextest",
        "run",
        "--all",
        "--offline" => @if offline,
        "--verbose" => @if verbose,
        "--no-fail-fast" => @if no_fail_fast,
        "--no-run" => @if no_run,
        "--no-capture" => @if no_capture,
        ...["--cargo-profile", profile] => @if let Some(profile) = profile,
        ...test_groups.to_args(),
        ...test_filters,
        @env "CLICOLOR_FORCE" => "1",
        @env "UPSILON_TEST_GUARD" => "1",
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
        @env "UPSILON_HOST_REPO_GIT" => ws_path!(".git"),
        @env "UPSILON_WEB_BIN" => upsilon_web_binary,
        @env "UPSILON_GRACEFULLY_SHUTDOWN_HOST_BIN" => upsilon_gracefully_shutdown_host_binary,
        @env "UPSILON_BIN_DIR" => ws_path!("target" / prof),
        @env "UPSILON_TESTSUITE_LOG" => "info",
        @workdir = ws_root!(),
    )?;

    Ok(())
}

fn run_tests_quick(
    setup_testenv: &Path,
    offline: bool,
    verbose: bool,
    no_fail_fast: bool,
    no_capture: bool,
    clean_profiles_between_steps: bool,
    profile: &str,
    test_filters: Vec<OsString>,
) -> XtaskResult<()> {
    let prof = profile;

    cargo_cmd!(
        "run",
        ...WS_BIN_LAYOUT.upsilon_setup_testenv_main.run_args(),
        "--verbose" => @if verbose,
        "--profile",
        profile,
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
        @env "UPSILON_SETUP_TESTENV_UPSILON_CLONE" => "1",
        @env "RUST_LOG" => "info",
        @env "UPSILON_BIN_DIR" => ws_path!("target" / prof),
        @workdir = ws_root!(),
    )?;

    if clean_profiles_between_steps {
        clean_unneeded_instrumentation_files()?;
    }

    let upsilon_web_binary = ws_bin_path!(profile = prof, name = "upsilon-web");

    let upsilon_gracefully_shutdown_host_binary =
        ws_bin_path!(profile = prof, name = "upsilon-gracefully-shutdown-host");

    cargo_cmd!(
        "nextest",
        "run",
        "--all",
        "--offline" => @if offline,
        "--verbose" => @if verbose,
        "--no-fail-fast" => @if no_fail_fast,
        "--no-capture" => @if no_capture,
        "--cargo-profile", profile,
        ...test_filters,
        @env "CLICOLOR_FORCE" => "1",
        @env "UPSILON_TEST_GUARD" => "1",
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
        @env "UPSILON_HOST_REPO_GIT" => ws_path!(".git"),
        @env "UPSILON_WEB_BIN" => upsilon_web_binary,
        @env "UPSILON_GRACEFULLY_SHUTDOWN_HOST_BIN" => upsilon_gracefully_shutdown_host_binary,
        @env "UPSILON_BIN_DIR" => ws_path!("target" / prof),
        @env "UPSILON_TESTSUITE_LOG" => "info",
        @workdir = ws_root!(),
    )?;

    Ok(())
}

fn run_test_support_examples(
    setup_testenv: &Path,
    tmpdir: &Path,
    examples: &[String],
    profile: Option<&str>,
) -> XtaskResult<()> {
    cargo_cmd!(
        "run",
        ...WS_BIN_LAYOUT.upsilon_setup_testenv_main.run_args(),
        "--verbose",
        ...["--profile", profile] => @if let Some(profile) = profile,
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "RUST_LOG" => "info",
        @env "UPSILON_BIN_DIR" => ws_path!("target/debug"),
        @workdir = ws_root!(),
    )?;

    let prof = profile.unwrap_or("debug");

    let upsilon_web_binary = ws_bin_path!(profile = prof, name = "upsilon-web");
    let upsilon_gracefully_shutdown_host_binary =
        ws_bin_path!(profile = prof, name = "upsilon-gracefully-shutdown-host");

    for example in examples {
        cargo_cmd!(
            "run",
            ...WS_PKG_LAYOUT.upsilon_test_support.run_args(),
            "--example",
            example,
            @env "CLICOLOR_FORCE" => "1",
            @env "UPSILON_TEST_GUARD" => "1",
            @env "UPSILON_SETUP_TESTENV" => setup_testenv,
            @env "UPSILON_HOST_REPO_GIT" => ws_path!(".git"),
            @env "UPSILON_WEB_BIN" => &upsilon_web_binary,
            @env "UPSILON_GRACEFULLY_SHUTDOWN_HOST_BIN" => &upsilon_gracefully_shutdown_host_binary,
            @env "UPSILON_BIN_DIR" => ws_path!("target" / prof),
            @env "UPSILON_TESTSUITE_LOG" => "info",
            @env "UPSILON_TMPDIR" => tmpdir,
            @workdir = ws_root!(),
        )?;
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

fn doc_extract_key_path_table<'a>(
    doc: &'a toml_edit::Document,
    kpath: &[&str],
) -> Option<(&'a Key, &'a Item)> {
    let mut deps_table = None;
    for path in kpath {
        if deps_table.is_none() {
            deps_table = doc.get_key_value(path);
        } else {
            deps_table = deps_table
                .unwrap()
                .1
                .as_table()
                .unwrap()
                .get_key_value(path);
        }

        if deps_table.is_none() {
            break;
        }
    }

    deps_table
}

fn check_dep_order_for_table(
    out_of_order_for_file: &mut Vec<(String, String)>,
    toml_doc: &toml_edit::Document,
    dep_table_path: &[&str],
) {
    assert!(!dep_table_path.is_empty());

    let Some(deps) = doc_extract_key_path_table(toml_doc, dep_table_path) else {return};

    let deps = deps.1.as_table().unwrap().iter().collect::<Vec<_>>();

    struct Last<'a>(&'a str);

    enum State<'a> {
        NonUpsilon(Option<Last<'a>>),
        Upsilon(Option<Last<'a>>),
    }

    let mut iter = deps.iter();
    let mut state = State::NonUpsilon(None);

    fn add_out_of_order_if_necessary(
        out_of_order_for_file: &mut Vec<(String, String)>,
        last: &Option<Last>,
        k: &str,
    ) {
        if let Some(last) = last {
            if *k < *last.0 {
                out_of_order_for_file
                    .push((k.to_string(), format!("{k} should be before {}", last.0)));
            }
        }
    }

    for (k, _item) in iter.by_ref() {
        match &state {
            State::NonUpsilon(last) => {
                if k.starts_with("upsilon-") {
                    state = State::Upsilon(Some(Last(k)));
                } else {
                    add_out_of_order_if_necessary(out_of_order_for_file, last, k);

                    state = State::NonUpsilon(Some(Last(k)));
                }
            }
            State::Upsilon(last) => {
                if k.starts_with("upsilon-") {
                    add_out_of_order_if_necessary(out_of_order_for_file, last, k);
                } else {
                    out_of_order_for_file
                        .push((k.to_string(), "upsilon deps should be last".to_string()));
                }
            }
        }
    }
}

fn load_cargo_manifest(path: impl AsRef<Path>) -> XtaskResult<toml_edit::Document> {
    fs::read_to_string(path)?.parse().map_err(Into::into)
}

fn check_dep_order(
    file: PathBuf,
    out_of_order: &mut Vec<(PathBuf, Vec<(String, String)>)>,
    deps_tables_to_check: &[&[&str]],
) -> XtaskResult<()> {
    let toml_doc = load_cargo_manifest(&file)?;

    let mut out_of_order_for_file = vec![];

    for dep_table_path in deps_tables_to_check {
        check_dep_order_for_table(&mut out_of_order_for_file, &toml_doc, dep_table_path);
    }

    if !out_of_order_for_file.is_empty() {
        out_of_order.push((file, out_of_order_for_file));
    }

    Ok(())
}

fn all_cargo_manifests_except_ws_root() -> XtaskResult<Vec<PathBuf>> {
    fn collect_from_folder(folder: PathBuf, to: &mut Vec<PathBuf>) -> XtaskResult<()> {
        let folder = folder.to_slash().unwrap();
        let cargo_toml_files_pattern = format!("{folder}/**/Cargo.toml");
        let cargo_toml_files =
            glob::glob(&cargo_toml_files_pattern)?.collect::<Result<Vec<_>, _>>()?;

        to.extend(cargo_toml_files);
        Ok(())
    }

    let mut cargo_toml_files = vec![];

    collect_from_folder(ws_path!("crates"), &mut cargo_toml_files)?;
    collect_from_folder(ws_path!("dev"), &mut cargo_toml_files)?;

    Ok(cargo_toml_files)
}

fn check_if_any_deps_in_ws_deps(
    file: PathBuf,
    ws_deps: &[String],
    in_ws_deps: &mut Vec<(PathBuf, Vec<String>)>,
) -> XtaskResult<()> {
    let toml_doc = load_cargo_manifest(&file)?;

    let mut in_ws_deps_for_file = vec![];

    let Some(deps) = doc_extract_key_path_table(&toml_doc, &["dependencies"])
        .map(|(_, deps)| deps.as_table().unwrap().iter().collect::<Vec<_>>()) else {
        return Ok(())
    };

    for (k, item) in deps {
        if !ws_deps.iter().any(|it| it == k) {
            continue;
        }

        if item.is_str() {
            in_ws_deps_for_file.push(k.to_string());
            continue;
        }

        let table_like = item
            .as_table_like()
            .expect("dependencies should be table like");

        if !table_like.contains_key("workspace") {
            in_ws_deps_for_file.push(k.to_string());
            continue;
        }

        fn dep_ws_value(dep_table_like: &dyn TableLike) -> bool {
            dep_table_like
                .get_key_value("workspace")
                .expect("workspace key missing") // we checked above
                .1
                .as_value()
                .expect("dependencies.<name>.workspace should be a boolean value")
                .as_bool()
                .expect("dependencies.<name>.workspace should be a boolean value")
        }

        if !dep_ws_value(table_like) {
            in_ws_deps_for_file.push(k.to_string());
            continue;
        }
    }

    if !in_ws_deps_for_file.is_empty() {
        in_ws_deps.push((file, in_ws_deps_for_file));
    }

    Ok(())
}

fn gqls_path() -> PathBuf {
    ws_path!("schemas" / "graphql" / "schema.graphql")
}

fn extend_filext_new(p: impl AsRef<Path>) -> PathBuf {
    p.as_ref().with_file_name(format!(
        "{}.new",
        p.as_ref().file_name().unwrap().to_string_lossy()
    ))
}

fn copy(from: impl AsRef<Path>, to: impl AsRef<Path>) -> XtaskResult<()> {
    let from = from.as_ref();
    let to = to.as_ref();

    if from.is_file() {
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

fn ukonf_to_yaml_string(from: PathBuf, fns: fn() -> UkonfFunctions) -> XtaskResult<String> {
    let result = ukonf::UkonfRunner::new(ukonf::UkonfConfig::new(vec![]), fns())
        .run(from)
        .map_err(|err| format_err!("Failed to run ukonf: {err}"))?;
    let yaml = result.into_value().to_yaml();
    let yaml_string = serde_yaml::to_string(&yaml)?;

    Ok(yaml_string)
}

fn ukonf_to_yaml(from: PathBuf, to: &Path, fns: fn() -> UkonfFunctions) -> XtaskResult<()> {
    let s = ukonf_to_yaml_string(from, fns)?;
    fs::write(to, s)?;

    Ok(())
}

fn ukonf_concat_strings(strings: &[UkonfValue]) -> Result<UkonfValue, UkonfFnError> {
    let mut result = String::new();
    for arg in strings {
        result.push_str(arg.as_string().context("concat: expected string")?);
    }
    Ok(UkonfValue::Str(result))
}

pub fn add_concat(fns: &mut UkonfFunctions) {
    fns.add_fn("concat", |_scope, args| ukonf_concat_strings(args));
}

pub fn add_parent_dir(fns: &mut UkonfFunctions) {
    fns.add_fn("parent_dir", |_scope, args| {
        if args.len() != 1 {
            bail!("parent_dir: expected exactly one argument");
        }

        let path = args[0].as_string().context("parent_dir: expected string")?;
        let path = Path::new(path);
        let parent = path.parent().context("parent_dir: no parent")?;
        Ok(UkonfValue::Str(
            parent
                .to_str()
                .context("parent_dir: invalid utf-8")?
                .to_string(),
        ))
    });
}

const NORMAL_UKONF_FUNCTIONS: &[fn(&mut UkonfFunctions)] = &[add_concat, add_parent_dir];

pub fn ukonf_normal_functions() -> UkonfFunctions {
    let mut fns = UkonfFunctions::new();
    for f in NORMAL_UKONF_FUNCTIONS {
        f(&mut fns);
    }
    fns
}

pub fn convert_path_to_win(path: &str) -> String {
    #[cfg(not(windows))]
    {
        let mut result = String::new();
        for c in path.to_str().unwrap().chars() {
            if c == '/' {
                result.push('\\');
            } else {
                result.push(c);
            }
        }
        result
    }

    #[cfg(windows)]
    {
        PathBuf::from_slash(path).to_str().unwrap().to_string()
    }
}

pub fn ukonf_add_win_path(fns: &mut UkonfFunctions) {
    fns.add_fn("win_path", |_scope, args| {
        if args.len() != 1 {
            bail!("win_path: expected exactly one argument");
        }

        let path = args[0].clone_to_string()?;

        let path = convert_path_to_win(&path);

        Ok(UkonfValue::Str(path))
    });
}

pub fn ukonf_xtask_path(scope: &Rc<RefCell<Scope>>) -> Result<String, UkonfFnError> {
    let mut xtask_artifact_path = Scope::resolve_cx(scope, "xtask_artifact_path")
        .context("xtask_path: xtask_artifact_path not found")??
        .expect_string()?;

    let xtask_is_win = Scope::resolve_cx(scope, "xtask_is_win")
        .transpose()?
        .map_or(Ok(false), |v| v.expect_bool())?;

    if xtask_is_win {
        let mut p = PathBuf::from(xtask_artifact_path);
        p.set_extension("exe");

        xtask_artifact_path = convert_path_to_win(
            p.to_str()
                .context("xtask_path: invalid utf-8 for xtask_artifact_path")?,
        );
    }

    Ok(xtask_artifact_path)
}

pub fn ukonf_add_xtask(fns: &mut UkonfFunctions) {
    fns.add_fn("xtask", |scope, args| {
        if args.len() != 1 {
            bail!("xtask: expected exactly one argument");
        }

        let xtask = args[0].as_string().context("xtask: expected string")?;

        Ok(UkonfValue::Object(
            UkonfObject::new().with("xtask", &**xtask),
        ))
    })
    .add_compiler_fn("xtask", |scope, mut xtask_v| {
        let xtask_obj = xtask_v.as_mut_object().context("xtask: expected object")?;
        let run = xtask_obj.get_mut("run").context("xtask: expected run")?;

        let r = run.as_mut_object().context("xtask: expected object")?;

        let xtask_artifact_path = ukonf_xtask_path(scope)?;

        *run = UkonfValue::Str(format!(
            "{xtask_artifact_path} {}",
            r.get("xtask")
                .context("xtask: expected xtask property")?
                .as_string()
                .context("xtask: expected string")?
        ));

        Ok(xtask_v)
    });
}

const CI_UKONF_FUNCTIONS: &[fn(&mut UkonfFunctions)] = &[ukonf_add_xtask, ukonf_add_win_path];

pub fn ukonf_ci_functions() -> UkonfFunctions {
    let mut fns = UkonfFunctions::new();
    for f in NORMAL_UKONF_FUNCTIONS {
        f(&mut fns);
    }
    for f in CI_UKONF_FUNCTIONS {
        f(&mut fns);
    }
    fns
}

fn gen_ci_file(from: PathBuf, to: &PathBuf) -> XtaskResult<()> {
    ukonf_to_yaml(from, to, ukonf_ci_functions)
}

pub struct OutdatedReport {
    from: PathBuf,
    to: PathBuf,
    diff: upsilon_diff_util::DiffResult,
}

fn check_ci_file(from: PathBuf, to: PathBuf, reports: &mut Vec<OutdatedReport>) -> XtaskResult<()> {
    let new = ukonf_to_yaml_string(from.clone(), ukonf_ci_functions)?;

    let old = fs::read_to_string(&to)?;

    if old == new {
        return Ok(());
    }

    let diff = upsilon_diff_util::build_diff(&old, &new);
    reports.push(OutdatedReport { from, to, diff });

    Ok(())
}

fn list_ci_files() -> Vec<(PathBuf, PathBuf)> {
    vec![
        (
            ws_path!(".ci" / "github-workflows" / "publish-docs.ukonf"),
            ws_path!(".github" / "workflows" / "publish-docs.yaml"),
        ),
        (
            ws_path!(".ci" / "github-workflows" / "test.ukonf"),
            ws_path!(".github" / "workflows" / "test.yaml"),
        ),
    ]
}

fn rm(p: &Path) -> XtaskResult<()> {
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
            let profile = profile.as_deref();
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
            let profile = profile.as_deref();
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
        App::Test {
            dgql,
            offline,
            verbose,
            no_fail_fast,
            no_run,
            no_capture,
            doc,
            clean_profiles_between_steps,
            profile,
            test_filters,
            test_groups,
        } => {
            let profile = profile.as_deref();
            build_dev(dgql, verbose, profile)?;

            if clean_profiles_between_steps {
                clean_unneeded_instrumentation_files()?;
            }

            let testenv_tests = ws_path!("testenv_tests");

            let setup_testenv = testenv_tests.join(std::process::id().to_string());

            if setup_testenv.exists() {
                fs::remove_dir_all(&setup_testenv)?;
            }

            fs::create_dir_all(&setup_testenv)?;

            let result = run_tests(
                &setup_testenv,
                offline,
                verbose,
                no_fail_fast,
                no_run,
                no_capture,
                clean_profiles_between_steps,
                &test_filters,
                &test_groups,
                doc,
                profile,
            );

            fs::remove_dir_all(&testenv_tests)?;

            result?;
        }
        App::TestQuick {
            dgql,
            offline,
            verbose,
            no_fail_fast,
            no_capture,
            clean_profiles_between_steps,
            from_index,
            algo,
            commit,
            profile,
        } => {
            let profile = profile.as_str();
            if profile != "difftests" && !profile.starts_with("difftests-") {
                bail!("Only difftests profile is supported for quick tests");
            }

            let tests = match from_index {
                false => difftests::tests_to_rerun(algo, commit)?,
                true => difftests::tests_to_rerun_from_index(algo, commit)?,
            };

            if tests.is_empty() {
                info!("No tests to rerun");
                return Ok(());
            }

            let tests = tests
                .iter()
                .map(|t| {
                    Ok((
                        WsPkgLayout::package_from_str(&t.test_desc.pkg_name)
                            .context("Unknown package")?,
                        t,
                    ))
                })
                .collect::<XtaskResult<Vec<_>>>()?;

            let test_filters = tests
                .iter()
                .flat_map(|(pkg, t)| pkg.nextest_test_filter(&t.test_desc.test_name))
                .collect::<Vec<_>>();

            build_dev(dgql, verbose, Some(profile))?;

            if clean_profiles_between_steps {
                clean_unneeded_instrumentation_files()?;
            }

            let testenv_tests = ws_path!("testenv_tests");

            let setup_testenv = testenv_tests.join(std::process::id().to_string());

            if setup_testenv.exists() {
                fs::remove_dir_all(&setup_testenv)?;
            }

            fs::create_dir_all(&setup_testenv)?;

            let result = run_tests_quick(
                &setup_testenv,
                offline,
                verbose,
                no_fail_fast,
                no_capture,
                clean_profiles_between_steps,
                profile,
                test_filters,
            );

            fs::remove_dir_all(&testenv_tests)?;

            result?;
        }
        App::TestSupportExamples { examples, profile } => {
            let profile = profile.as_deref();
            build_dev(false, false, profile)?;

            let testenv_tests = ws_path!("testenv_tests");

            let tmpdir_root = testenv_tests.join(std::process::id().to_string());

            if tmpdir_root.exists() {
                fs::remove_dir_all(&tmpdir_root)?;
            }

            fs::create_dir_all(&tmpdir_root)?;

            let setup_testenv = tmpdir_root.join("testenv");

            fs::create_dir_all(&setup_testenv)?;

            let tmpdir = tmpdir_root.join("tmpdir");

            fs::create_dir_all(&tmpdir)?;

            let result = run_test_support_examples(&setup_testenv, &tmpdir, &examples, profile);

            fs::remove_dir_all(&testenv_tests)?;

            result?;
        }
        App::PackRelease => {
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
            let mut out_of_order = vec![];
            check_dep_order(
                ws_path!("Cargo.toml"),
                &mut out_of_order,
                &[&["workspace", "dependencies"]],
            )?;

            let cargo_toml_files = all_cargo_manifests_except_ws_root()?;

            for cargo_toml_file in cargo_toml_files {
                check_dep_order(cargo_toml_file, &mut out_of_order, &[&["dependencies"]])?;
            }

            if !out_of_order.is_empty() {
                eprintln!("The following dependencies are out of order:");
                for (file, out_of_order) in out_of_order {
                    eprintln!("  {}", file.to_slash().unwrap());
                    for (dep, reason) in out_of_order {
                        eprintln!("    {dep}: {reason}");
                    }
                }
                bail!("Dependencies out of order");
            }
        }
        App::CheckCargoDepFromWorkspace => {
            let ws_manifest = load_cargo_manifest(ws_path!("Cargo.toml"))?;
            let ws_deps = doc_extract_key_path_table(&ws_manifest, &["workspace", "dependencies"])
                .expect("Missing workspace dependencies")
                .1
                .as_table()
                .expect("workspace.dependencies is not a table");

            let ws_deps_names = ws_deps
                .iter()
                .map(|(k, _)| k.to_string())
                .collect::<Vec<_>>();

            let cargo_toml_files = all_cargo_manifests_except_ws_root()?;

            let mut in_ws_deps = vec![];

            for cargo_toml in cargo_toml_files {
                check_if_any_deps_in_ws_deps(cargo_toml, &ws_deps_names, &mut in_ws_deps)?;
            }

            if !in_ws_deps.is_empty() {
                eprintln!(
                    "The following dependencies are redeclared from the workspace dependencies:"
                );
                for (file, in_ws_deps) in in_ws_deps {
                    eprintln!("  {}", file.to_slash().unwrap());
                    for dep in in_ws_deps {
                        eprintln!("    {dep}");
                    }
                }
                bail!("Dependencies are redeclared from the workspace dependencies");
            }
        }
        App::Lint => {
            let cranky_config = cargo_cranky::config::CrankyConfig::get_config()?;
            let clippy_flags = cranky_config.extra_right_args();

            cargo_cmd!(
                "clippy",
                "--all",
                "--",
                ...clippy_flags,
                @workdir = ws_root!(),
            )?;
        }
        App::LintArgs => {
            let cranky_config = cargo_cranky::config::CrankyConfig::get_config()?;
            let clippy_flags = cranky_config.extra_right_args();

            println!("{}", clippy_flags.join(" "));
        }
        App::UkonfToYaml { from, to } => {
            ukonf_to_yaml(from, &to, ukonf_normal_functions)?;
        }
        App::GenCiFiles => {
            for (from, to) in list_ci_files() {
                gen_ci_file(from, &to)?;
            }
        }
        App::CheckCiFilesUpToDate => {
            let mut reports = vec![];

            for (from, to) in list_ci_files() {
                check_ci_file(from, to, &mut reports)?;
            }

            if !reports.is_empty() {
                eprintln!("The following CI files are out of date:");

                for report in reports {
                    let OutdatedReport { from, to, diff } = report;
                    eprintln!("  {from} -> {to}", from = from.display(), to = to.display());
                    eprintln!("====");
                    eprintln!("{diff}");
                    eprintln!("====");
                }

                eprintln!("Run `cargo xtask gen-ci-files` to update them.");

                bail!("CI files are out of date");
            }
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
