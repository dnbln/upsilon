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

use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgMatches, Args, Command, FromArgMatches, Parser, ValueHint};
use path_slash::PathExt;
use toml_edit::{Item, Key, TableLike};
use upsilon_xtask::{cargo_cmd, cmd_call, ws_path, ws_root, XtaskResult};
use zip::write::{FileOptions, ZipWriter};

#[derive(Debug, Clone)]
struct TestGroups {
    groups: Vec<String>,
}

impl TestGroups {
    fn to_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        for group in &self.groups {
            args.push("-E".to_string());
            args.push(format!("binary({})", group.to_string()));
        }
        args
    }
}

fn test_binaries() -> std::io::Result<Vec<String>> {
    let path = ws_path!("crates" / "upsilon-testsuite" / "tests");

    let mut test_binaries = vec![];

    let dir = std::fs::read_dir(path)?;
    for file in dir {
        let file = file?;

        let path = file.path();

        let name = path
            .file_name()
            .expect("Missing file name")
            .to_str()
            .expect("Invalid file name")
            .to_string();

        if name.ends_with(".rs") {
            let name = name.replace(".rs", "");
            test_binaries.push(name);
        }
    }

    Ok(test_binaries)
}

impl FromArgMatches for TestGroups {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, clap::Error> {
        let test_binaries = test_binaries()?;

        let mut groups = vec![];
        for test_binary in test_binaries {
            if matches.get_flag(&test_binary) {
                groups.push(test_binary);
            }
        }

        Ok(Self { groups })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        let test_binaries = test_binaries()?;

        let mut groups = vec![];
        for test_binary in test_binaries {
            if matches.get_flag(&test_binary) {
                groups.push(test_binary);
            }
        }

        self.groups = groups;

        Ok(())
    }
}

impl Args for TestGroups {
    fn augment_args(mut cmd: Command) -> Command {
        let Ok(test_binaries) = test_binaries() else { return cmd; };

        for test_bin in test_binaries {
            cmd = cmd.arg(
                Arg::new(test_bin.clone())
                    .long(test_bin.clone())
                    .action(ArgAction::SetTrue)
                    .help(format!("Filter tests from the {test_bin} binary")),
            );
        }

        cmd
    }

    fn augment_args_for_update(cmd: Command) -> Command {
        Self::augment_args(cmd)
    }
}

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "fmt")]
    Fmt,
    #[clap(name = "fmt-check")]
    FmtCheck,
    #[clap(name = "git-checks")]
    #[clap(alias = "gchk")]
    GitChecks {
        #[clap(short, long)]
        checkout: bool,
    },
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

        #[clap(flatten)]
        test_groups: TestGroups,

        tests_filters: Vec<String>,
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
    #[clap(name = "graphql-schema-check")]
    #[clap(alias = "gqlschk")]
    GraphQLSchemaCheck,
    #[clap(name = "check-cargo-dep-order")]
    #[clap(alias = "ccdo")]
    CheckCargoTomlDepOrder,
    #[clap(name = "check-cargo-dep-from-workspace")]
    #[clap(alias = "ccdw")]
    CheckCargoDepFromWorkspace,
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
    test_filters: &[String],
    test_groups: &TestGroups,
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
        ...test_groups.to_args(),
        ...test_filters,
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
    std::fs::read_to_string(path)?.parse().map_err(Into::into)
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
    let crates_folder = ws_path!("crates");
    let crates_folder = crates_folder.to_slash().unwrap();
    let cargo_toml_files_pattern = format!("{crates_folder}/**/Cargo.toml");
    let cargo_toml_files = glob::glob(&cargo_toml_files_pattern)?.collect::<Result<Vec<_>, _>>()?;

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

        let table_like = item.as_table_like().expect("deps should be table like");

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
                .expect("deps.<name>.workspace should be a boolean value")
                .as_bool()
                .expect("deps.<name>.workspace should be a boolean value")
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
        App::GitChecks { checkout } => {
            let repo = upsilon_xtask::git_checks::get_repo(&ws_root!())?;

            if !checkout {
                upsilon_xtask::git_checks::linear_history(&repo)?;
            }
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
            tests_filters,
            test_groups,
        } => {
            build_dev(dgql, verbose)?;

            let testenv_tests = ws_path!("testenv_tests");

            let setup_testenv = testenv_tests.join(std::process::id().to_string());

            if setup_testenv.exists() {
                std::fs::remove_dir_all(&setup_testenv)?;
            }

            std::fs::create_dir_all(&setup_testenv)?;

            let result = run_tests(
                &setup_testenv,
                offline,
                verbose,
                no_fail_fast,
                no_run,
                &tests_filters,
                &test_groups,
            );

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
                "--", gqls_path(),
                @workdir = ws_root!(),
            )?;
        }
        App::GraphQLSchemaCheck => {
            let p = gqls_path();
            let temp_p = extend_filext_new(&p);

            cargo_cmd!(
                "run",
                "-p", "upsilon-api",
                "--bin", "dump_graphql_schema",
                "--", &temp_p,
                @workdir = ws_root!(),
            )?;

            let contents = std::fs::read_to_string(&p)?;
            let new_contents = std::fs::read_to_string(&temp_p)?;

            let up_to_date = contents == new_contents;

            if !up_to_date {
                let diff = upsilon_diff_util::build_diff(&contents, &new_contents);

                eprintln!("GraphQL schema is out of date. Run `cargo xtask gqls` to update it.");
                eprintln!("Diff:");
                eprintln!("=====================");
                eprintln!("{}", diff);
                eprintln!("=====================");
            }

            std::fs::remove_file(&temp_p)?;

            if !up_to_date {
                std::process::exit(1);
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
                std::process::exit(1);
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
                std::process::exit(1);
            }
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
