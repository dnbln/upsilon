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

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use anyhow::{bail, format_err, Context};
use ukonf::value::{UkonfObject, UkonfValue};
use ukonf::{Scope, UkonfFnError, UkonfFunctions};
use upsilon_xtask::{ws_path, XtaskResult};

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
        let path = {
            #[cfg(not(windows))]
            {
                use path_slash::PathBufExt;
                if path.contains('\\') {
                    PathBuf::from_backslash(path)
                } else {
                    PathBuf::from(path)
                }
            }
            #[cfg(windows)]
            {
                PathBuf::from(path)
            }
        };
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
        path.replace('/', "\\")
    }

    #[cfg(windows)]
    {
        use path_slash::PathBufExt;
        PathBuf::from_slash(path).to_str().unwrap().to_string()
    }
}

enum UkonfXtaskPath {
    Artifact(String),
    CargoRun,
}

impl UkonfXtaskPath {
    fn run(&self) -> &str {
        match self {
            UkonfXtaskPath::Artifact(path) => path,
            UkonfXtaskPath::CargoRun => "cargo xtask",
        }
    }
}

fn ukonf_xtask_path(scope: &Rc<RefCell<Scope>>) -> Result<UkonfXtaskPath, UkonfFnError> {
    let xtask_cargo_run = Scope::resolve_cx(scope, "xtask_cargo_run")
        .transpose()?
        .map_or(Ok(false), |v| v.expect_bool())?;

    if xtask_cargo_run {
        return Ok(UkonfXtaskPath::CargoRun);
    }

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

    Ok(UkonfXtaskPath::Artifact(xtask_artifact_path))
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

        let xtask_path = ukonf_xtask_path(scope)?;

        *run = UkonfValue::Str(format!(
            "{} {}",
            xtask_path.run(),
            r.get("xtask")
                .context("xtask: expected xtask property")?
                .as_string()
                .context("xtask: expected string")?
        ));

        Ok(xtask_v)
    });
}

const CI_UKONF_FUNCTIONS: &[fn(&mut UkonfFunctions)] = &[ukonf_add_xtask];

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

fn gen_ci_file(from: PathBuf, to: &Path) -> XtaskResult<()> {
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

pub fn run_ukonf_to_yaml_cmd(from: PathBuf, to: &Path) -> XtaskResult<()> {
    ukonf_to_yaml(from, to, ukonf_normal_functions)?;

    Ok(())
}

pub fn run_gen_ci_files_cmd() -> XtaskResult<()> {
    for (from, to) in list_ci_files() {
        gen_ci_file(from, &to)?;
    }

    Ok(())
}

pub fn run_check_ci_files_up_to_date_cmd() -> XtaskResult<()> {
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

    Ok(())
}
