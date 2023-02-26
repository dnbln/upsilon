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

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches, Parser};
use log::info;
use upsilon_xtask::pkg::Profile;
use upsilon_xtask::{cargo_cmd, cmd_args, ws_path, ws_root, XtaskResult};

use crate::difftests::DirtyAlgo;
use crate::ws_layout::{WsPkgLayout, WS_BIN_LAYOUT, WS_PKG_LAYOUT};
use crate::{
    build_dev, clean_unneeded_instrumentation_files, copy_test_artifacts, difftests, AbsolutizePathBuf
};

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
        if (self
            == TestenvConfig {
                needs_upsilon_clone: false,
            })
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
            args.push("-E".to_owned());
            args.push(group.nextest_filter().to_owned());
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
                    .aliases(group.aliases().iter().map(|it| (*it).to_owned()))
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

#[derive(Debug, Parser)]
pub struct TestCmd {
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
    #[clap(long, default_value_t = Profile::Dev)]
    profile: Profile,

    #[clap(long)]
    no_build_dev: bool,
    #[clap(long)]
    custom_bin_dir: Option<AbsolutizePathBuf>,

    #[clap(flatten)]
    test_groups: TestGroups,

    test_filters: Vec<String>,
}

#[derive(Debug, Parser)]
pub struct TestQuickCmd {
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
    #[clap(long, default_value_t = Profile::Difftests)]
    profile: Profile,

    #[clap(long)]
    no_build_dev: bool,
    #[clap(long)]
    custom_bin_dir: Option<AbsolutizePathBuf>,
}

fn run_doctests(verbose: bool, no_fail_fast: bool, profile: Profile) -> XtaskResult<()> {
    cargo_cmd!(
        "test",
        "--doc",
        "--workspace",
        "--verbose" => @if verbose,
        "--no-fail-fast" => @if no_fail_fast,
        "--profile", profile.name(),
        @workdir = ws_root!(),
    )?;

    Ok(())
}

fn test_excludes() -> Vec<OsString> {
    cmd_args!(
        "--exclude",
        "upsilon-setup-testenv",
        "--exclude",
        "upsilon-xtask",
        "--exclude",
        "cargo-cranky",
        "--exclude",
        "cargo-guard",
    )
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
    profile: Profile,
    custom_bin_dir: Option<PathBuf>,
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

    let bin_dir = custom_bin_dir.unwrap_or_else(|| profile.target_dir());

    if let Some(testenv_config) = test_groups.testenv_config() {
        cargo_cmd!(
            "build" => @if no_run,
            "run" => @if !no_run,
            "-p",
            "upsilon-setup-testenv",
            "--bin",
            "upsilon-setup-testenv",
            "--verbose" => @if verbose,
            "--profile", profile.name(),
            @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
            @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
            @env "UPSILON_SETUP_TESTENV_UPSILON_CLONE" => "1" => @if testenv_config.needs_upsilon_clone,
            @env "RUST_LOG" => "info",
            @env "UPSILON_BIN_DIR" => &bin_dir,
            @workdir = ws_root!(),
        )?;
    }

    if clean_profiles_between_steps {
        clean_unneeded_instrumentation_files()?;
    }

    let upsilon_web_binary = WS_BIN_LAYOUT
        .upsilon_web_main
        .path_in_custom_dir(bin_dir.clone());

    let upsilon_gracefully_shutdown_host_binary = WS_BIN_LAYOUT
        .upsilon_gracefully_shutdown_host_main
        .path_in_custom_dir(bin_dir.clone());

    cargo_cmd!(
        "nextest",
        "run",
        "--all",
        ...test_excludes(),
        "--offline" => @if offline,
        "--verbose" => @if verbose,
        "--no-fail-fast" => @if no_fail_fast,
        "--no-run" => @if no_run,
        "--no-capture" => @if no_capture,
        "--cargo-profile", profile.name(),
        ...test_groups.to_args(),
        ...test_filters,
        @env "CLICOLOR_FORCE" => "1",
        @env "UPSILON_TEST_GUARD" => "1",
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
        @env "UPSILON_HOST_REPO_GIT" => ws_path!(".git"),
        @env "UPSILON_WEB_BIN" => upsilon_web_binary,
        @env "UPSILON_GRACEFULLY_SHUTDOWN_HOST_BIN" => upsilon_gracefully_shutdown_host_binary,
        @env "UPSILON_BIN_DIR" => &bin_dir,
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
    profile: Profile,
    test_filters: Vec<OsString>,
    custom_bin_dir: Option<PathBuf>,
) -> XtaskResult<()> {
    let bin_dir = custom_bin_dir.unwrap_or_else(|| profile.target_dir());

    cargo_cmd!(
        "run",
        ...WS_BIN_LAYOUT.upsilon_setup_testenv_main.run_args(),
        "--verbose" => @if verbose,
        "--profile", profile.name(),
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
        @env "UPSILON_SETUP_TESTENV_UPSILON_CLONE" => "1",
        @env "RUST_LOG" => "info",
        @env "UPSILON_BIN_DIR" => &bin_dir,
        @workdir = ws_root!(),
    )?;

    if clean_profiles_between_steps {
        clean_unneeded_instrumentation_files()?;
    }

    let upsilon_web_binary = WS_BIN_LAYOUT
        .upsilon_web_main
        .path_in_custom_dir(bin_dir.clone());

    let upsilon_gracefully_shutdown_host_binary = WS_BIN_LAYOUT
        .upsilon_gracefully_shutdown_host_main
        .path_in_custom_dir(bin_dir.clone());

    cargo_cmd!(
        "nextest",
        "run",
        "--all",
        ...test_excludes(),
        "--offline" => @if offline,
        "--verbose" => @if verbose,
        "--no-fail-fast" => @if no_fail_fast,
        "--no-capture" => @if no_capture,
        "--cargo-profile", profile.name(),
        ...test_filters,
        @env "CLICOLOR_FORCE" => "1",
        @env "UPSILON_TEST_GUARD" => "1",
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "UPSILON_TESTSUITE_OFFLINE" => "" => @if offline,
        @env "UPSILON_HOST_REPO_GIT" => ws_path!(".git"),
        @env "UPSILON_WEB_BIN" => upsilon_web_binary,
        @env "UPSILON_GRACEFULLY_SHUTDOWN_HOST_BIN" => upsilon_gracefully_shutdown_host_binary,
        @env "UPSILON_BIN_DIR" => &bin_dir,
        @env "UPSILON_TESTSUITE_LOG" => "info",
        @workdir = ws_root!(),
    )?;

    Ok(())
}

fn run_test_support_examples(
    setup_testenv: &Path,
    tmpdir: &Path,
    examples: &[String],
    profile: Profile,
) -> XtaskResult<()> {
    cargo_cmd!(
        "run",
        ...WS_BIN_LAYOUT.upsilon_setup_testenv_main.run_args(),
        "--verbose",
        "--profile", profile.name(),
        @env "UPSILON_SETUP_TESTENV" => &setup_testenv,
        @env "RUST_LOG" => "info",
        @env "UPSILON_BIN_DIR" => profile.target_dir(),
        @workdir = ws_root!(),
    )?;

    let upsilon_web_binary = WS_BIN_LAYOUT.upsilon_web_main.path_in_profile(profile);

    let upsilon_gracefully_shutdown_host_binary = WS_BIN_LAYOUT
        .upsilon_gracefully_shutdown_host_main
        .path_in_profile(profile);

    for example in examples {
        cargo_cmd!(
            "run",
            ...WS_PKG_LAYOUT.upsilon_test_support.run_args(),
            "--example",
            example,
            "--profile", profile.name(),
            @env "CLICOLOR_FORCE" => "1",
            @env "UPSILON_TEST_GUARD" => "1",
            @env "UPSILON_SETUP_TESTENV" => setup_testenv,
            @env "UPSILON_HOST_REPO_GIT" => ws_path!(".git"),
            @env "UPSILON_WEB_BIN" => &upsilon_web_binary,
            @env "UPSILON_GRACEFULLY_SHUTDOWN_HOST_BIN" => &upsilon_gracefully_shutdown_host_binary,
            @env "UPSILON_BIN_DIR" => profile.target_dir(),
            @env "UPSILON_TESTSUITE_LOG" => "info",
            @env "UPSILON_TMPDIR" => tmpdir,
            @workdir = ws_root!(),
        )?;
    }

    Ok(())
}

pub fn run_tests_cmd(cmd: TestCmd) -> XtaskResult<()> {
    let TestCmd {
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
        no_build_dev,
        custom_bin_dir,
    } = cmd;

    let custom_bin_dir = custom_bin_dir.map(AbsolutizePathBuf::into_inner);

    if !no_build_dev {
        build_dev(dgql, verbose, profile)?;
        if let Some(ref p) = custom_bin_dir {
            copy_test_artifacts(profile, p)?;
        }
    }

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
        custom_bin_dir,
    );

    fs::remove_dir_all(&testenv_tests)?;

    result?;

    Ok(())
}

pub fn run_test_quick_cmd(cmd: TestQuickCmd) -> XtaskResult<()> {
    let TestQuickCmd {
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
        no_build_dev,
        custom_bin_dir,
    } = cmd;

    if profile != Profile::Difftests {
        bail!("Only difftests profile is supported for quick tests");
    }

    let custom_bin_dir = custom_bin_dir.map(AbsolutizePathBuf::into_inner);

    let tests = match from_index {
        false => difftests::tests_to_rerun(algo, commit, profile)?,
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
                WsPkgLayout::package_from_str(&t.test_desc.pkg_name).context("Unknown package")?,
                t,
            ))
        })
        .collect::<XtaskResult<Vec<_>>>()?;

    let test_filters = tests
        .iter()
        .flat_map(|(pkg, t)| pkg.nextest_test_filter(&t.test_desc.test_name))
        .collect::<Vec<_>>();

    if !no_build_dev {
        build_dev(dgql, verbose, profile)?;

        if let Some(ref p) = custom_bin_dir {
            copy_test_artifacts(profile, p)?;
        }
    }

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
        custom_bin_dir,
    );

    fs::remove_dir_all(&testenv_tests)?;

    result?;

    Ok(())
}

pub fn run_test_support_examples_cmd(examples: Vec<String>, profile: Profile) -> XtaskResult<()> {
    build_dev(false, false, profile)?;

    let testenv_tests = ws_path!("testenv_tests");

    let tmpdir_root = testenv_tests.join(std::process::id().to_string());

    if tmpdir_root.exists() {
        fs::remove_dir_all(&tmpdir_root)?;
    }

    fs::create_dir_all(&tmpdir_root)?;

    let setup_testenv = tmpdir_root.join("../../../../../testenv");

    fs::create_dir_all(&setup_testenv)?;

    let tmpdir = tmpdir_root.join("tmpdir");

    fs::create_dir_all(&tmpdir)?;

    let result = run_test_support_examples(&setup_testenv, &tmpdir, &examples, profile);

    fs::remove_dir_all(&testenv_tests)?;

    result?;

    Ok(())
}
