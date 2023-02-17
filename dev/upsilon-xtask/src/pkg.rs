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

use std::ffi::{OsStr, OsString};

use crate::{cargo_cmd, cmd_args, ws_path, ws_root, XtaskResult};

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum PkgKind {
    LocalCrates,
    LocalDev,
    CratesIo,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Pkg {
    name: String,
    kind: PkgKind,
}

impl Pkg {
    pub fn dev_pkg(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: PkgKind::LocalDev,
        }
    }

    pub fn local_crates(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: PkgKind::LocalCrates,
        }
    }

    pub fn crates_io(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: PkgKind::CratesIo,
        }
    }

    pub fn bin_target(&self, bin_name: impl Into<BinName>) -> BinTarget {
        BinTarget::new(self, bin_name)
    }

    pub fn main_target(&self) -> BinTarget {
        Self::bin_target(self, self.name.as_str())
    }

    pub fn publish(&self) -> XtaskResult<()> {
        cargo_cmd!("publish", "-p", self.name.as_str())?;

        Ok(())
    }

    #[track_caller]
    pub fn build_args(&self) -> Vec<OsString> {
        match self.kind {
            PkgKind::LocalCrates | PkgKind::LocalDev => vec!["-p".into(), self.name.clone().into()],
            PkgKind::CratesIo => {
                panic!("Cannot build crates.io package: {}", self.name.as_str())
            }
        }
    }

    #[track_caller]
    pub fn run_args(&self) -> Vec<OsString> {
        match self.kind {
            PkgKind::LocalCrates | PkgKind::LocalDev => vec!["-p".into(), self.name.clone().into()],
            PkgKind::CratesIo => {
                panic!("Cannot run crates.io package: {}", self.name.as_str())
            }
        }
    }

    pub fn install_args(&self) -> Vec<OsString> {
        match self.kind {
            PkgKind::LocalCrates => vec![
                "--path".into(),
                ws_path!("crates" / (self.name.as_str())).into(),
            ],
            PkgKind::LocalDev => vec![
                "--path".into(),
                ws_path!("dev" / (self.name.as_str())).into(),
            ],
            PkgKind::CratesIo => vec![self.name.clone().into()],
        }
    }

    pub fn install(&self) -> XtaskResult<()> {
        cargo_cmd!("install", ...self.install_args(), @workdir = ws_root!())?;

        Ok(())
    }

    pub fn nextest_test_filter(&self, test_name: &str) -> Vec<OsString> {
        cmd_args!(
            "-E",
            format!("package(={}) & test(={test_name})", self.name.as_str()),
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BinName {
    name: String,
}

impl<T> From<T> for BinName
where
    String: From<T>,
{
    fn from(name: T) -> Self {
        Self { name: name.into() }
    }
}

impl BinName {
    pub fn bin_arg(&self) -> Vec<OsString> {
        vec!["--bin".into(), self.name.clone().into()]
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct BinTarget<'pkg> {
    pkg: &'pkg Pkg,
    bin_name: BinName,
}

impl<'pkg> BinTarget<'pkg> {
    pub fn new(pkg: &'pkg Pkg, bin_name: impl Into<BinName>) -> Self {
        Self {
            pkg,
            bin_name: bin_name.into(),
        }
    }

    #[track_caller]
    pub fn build_args(&self) -> Vec<OsString> {
        let mut args = self.pkg.build_args();
        args.extend(self.bin_name.bin_arg());
        args
    }

    #[track_caller]
    pub fn build<T: AsRef<OsStr>, I: IntoIterator<Item = T>>(
        &self,
        extra_args: I,
    ) -> XtaskResult<()> {
        cargo_cmd!("build", ...self.build_args(), ...extra_args, @workdir = ws_root!())?;

        Ok(())
    }

    #[track_caller]
    pub fn run_args(&self) -> Vec<OsString> {
        let mut args = self.pkg.run_args();
        args.extend(self.bin_name.bin_arg());
        args
    }

    pub fn install_args(&self) -> Vec<OsString> {
        let mut args = self.pkg.install_args();
        args.extend(self.bin_name.bin_arg());
        args
    }

    pub fn install(&self) -> XtaskResult<()> {
        cargo_cmd!("install", ...self.install_args(), @workdir = ws_root!())?;

        Ok(())
    }
}
