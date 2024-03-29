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

use std::fmt::Write as _;
use std::io::Write;
use std::path::Path;

use path_slash::PathExt;
use upsilon_xtask::cargo_ws::{cargo_config, cargo_ws};
use upsilon_xtask::pkg::PkgKind;
use upsilon_xtask::{ws_path, ws_root, XtaskResult};

fn one_of(v: &[bool]) -> bool {
    let Some(pos) = v.iter().position(|it| *it) else {
        return false;
    };

    v[pos + 1..].iter().all(|it| !*it)
}

pub fn gen_ws_layout(to: &Path) -> XtaskResult<()> {
    let cargo_config = cargo_config()?;
    let ws = cargo_ws(&cargo_config)?;

    let mut ws_layout_members = Vec::new();

    for member in ws.members() {
        let pkg_name = member.name().as_str();
        let path = member.root();
        let rustic_name = pkg_name.replace('-', "_");

        let path_in_ws = path.strip_prefix(ws_root!()).unwrap();
        let is_dev = path_in_ws.starts_with("dev");
        let is_crates = path_in_ws.starts_with("crates");
        let is_plugins = path_in_ws.starts_with("plugins");
        let is_tools = path_in_ws.starts_with("tools");

        assert!(one_of(&[is_dev, is_crates, is_plugins, is_tools]));

        let kind = match (is_dev, is_crates, is_plugins, is_tools) {
            (true, false, false, false) => PkgKind::LocalDev,
            (false, true, false, false) => PkgKind::LocalCrates,
            (false, false, true, false) => PkgKind::LocalPlugins,
            (false, false, false, true) => PkgKind::LocalTools {
                path_in_ws: path_in_ws.to_path_buf(),
            },
            _ => unreachable!(),
        };

        let mut bins = Vec::new();

        for bin in member.targets().iter().filter(|it| it.is_bin()) {
            let name = bin.name();
            let bin_rustic_whole_name = if name == pkg_name {
                format!("{rustic_name}_main")
            } else {
                format!("{rustic_name}_{name}")
            };

            bins.push((bin_rustic_whole_name, name));
        }

        ws_layout_members.push((pkg_name, rustic_name, path, kind, bins));
    }

    let mut ws_pkg_layout_decl = String::new();
    ws_pkg_layout_decl.push_str("pub struct WsPkgLayout {\n");
    let mut ws_pkg_layout = String::new();
    ws_pkg_layout.push_str(
        r#"
lazy_static::lazy_static! {
    pub static ref WS_PKG_LAYOUT: WsPkgLayout = WsPkgLayout {
"#,
    );

    let mut package_from_str = String::new();
    package_from_str.push_str("match name {\n");

    let mut ws_bin_layout_decl = String::new();
    ws_bin_layout_decl.push_str("pub struct WsBinLayout {\n");
    let mut ws_bin_layout = String::new();
    ws_bin_layout.push_str(
        r#"
lazy_static::lazy_static! {
    pub static ref WS_BIN_LAYOUT: WsBinLayout = WsBinLayout {
"#,
    );

    for (name, rustic_name, _path, kind, bins) in &ws_layout_members {
        writeln!(
            ws_pkg_layout_decl,
            "    pub {rustic_name}: upsilon_xtask::pkg::Pkg,"
        )
        .unwrap();

        let (pkg_initializer, extra) = match kind {
            PkgKind::LocalDev => ("dev_pkg", None),
            PkgKind::LocalCrates => ("local_crates", None),
            PkgKind::LocalPlugins => ("plugin_pkg", None),
            PkgKind::LocalTools { path_in_ws } => (
                "tool_pkg",
                Some(format!("\"{}\"", path_in_ws.to_slash().unwrap())),
            ),
            PkgKind::CratesIo => unreachable!(),
        };

        writeln!(
            ws_pkg_layout,
            r#"        {rustic_name}: upsilon_xtask::pkg::Pkg::{pkg_initializer}("{name}"{extra}),"#,
            extra = if let Some(extra) = extra {
                format!(", {extra}")
            } else {
                "".to_owned()
            }
        )
        .unwrap();

        writeln!(
            package_from_str,
            r#"            "{name}" => Some(&WS_PKG_LAYOUT.{rustic_name}),"#,
        )
        .unwrap();

        for (bin_rustic_name, name) in bins {
            writeln!(
                ws_bin_layout_decl,
                "    pub {bin_rustic_name}: upsilon_xtask::pkg::BinTarget<'static>,"
            )
            .unwrap();

            writeln!(ws_bin_layout,
                r#"        {bin_rustic_name}: upsilon_xtask::pkg::BinTarget::new(&WS_PKG_LAYOUT.{rustic_name}, "{name}"),"#,
            ).unwrap();
        }
    }

    ws_pkg_layout_decl.push_str("}\n\n");

    ws_pkg_layout.push_str("    };\n}\n\n");

    package_from_str.push_str("            _ => None,\n        }");

    ws_bin_layout_decl.push_str("}\n\n");

    ws_bin_layout.push_str("    };\n}\n\n");

    let mut ws_layout_file = std::fs::File::create(to)?;

    ws_layout_file.write_all(
        br#"// This file is auto-generated by the gen_ws_layout binary. Do not edit it manually.
// Path: dev/upsilon-xtask/src/bin/gen_ws_layout.rs
// Run cargo xgen-ws-layout to re-generate it.

"#,
    )?;

    ws_layout_file.write_all(ws_pkg_layout_decl.as_bytes())?;
    ws_layout_file.write_all(ws_pkg_layout.as_bytes())?;
    ws_layout_file.write_all(
        br#"
impl WsPkgLayout {
    pub fn package_from_str(name: &str) -> Option<&'static upsilon_xtask::pkg::Pkg> {
        "#,
    )?;
    ws_layout_file.write_all(package_from_str.as_bytes())?;
    ws_layout_file.write_all(
        br#"
    }
}
"#,
    )?;
    ws_layout_file.write_all(ws_bin_layout_decl.as_bytes())?;
    ws_layout_file.write_all(ws_bin_layout.as_bytes())?;

    ws_layout_file.write_all(
        br#"

lazy_static::lazy_static! {
    pub static ref DOCS: upsilon_xtask::docusaurus::Docusaurus = upsilon_xtask::docusaurus::Docusaurus::new(upsilon_xtask::ws_path!("docs"));
}
"#,
    )?;

    Ok(())
}

fn main() -> XtaskResult<()> {
    gen_ws_layout(&ws_path!(
        "dev" / "upsilon-xtask" / "src" / "bin" / "upsilon-xtask" / "ws_layout.rs"
    ))
}
