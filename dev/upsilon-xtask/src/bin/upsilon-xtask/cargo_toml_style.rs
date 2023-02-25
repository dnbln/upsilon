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

use anyhow::bail;
use path_slash::PathBufExt;
use toml_edit::{Item, Key, TableLike};
use upsilon_xtask::{ws_path, XtaskResult};

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

pub fn run_check_cargo_toml_dep_order_cmd() -> XtaskResult<()> {
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

    Ok(())
}

pub fn run_check_cargo_deps_from_workspace_cmd() -> XtaskResult<()> {
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
        eprintln!("The following dependencies are redeclared from the workspace dependencies:");
        for (file, in_ws_deps) in in_ws_deps {
            eprintln!("  {}", file.to_slash().unwrap());
            for dep in in_ws_deps {
                eprintln!("    {dep}");
            }
        }
        bail!("Dependencies are redeclared from the workspace dependencies");
    }

    Ok(())
}
