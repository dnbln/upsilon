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

use std::path::PathBuf;
use std::process::ExitStatus;
use std::{fmt, io};
macro_rules! cmd_process {
    ($s:expr $(, $($args:expr),+)?$(, workdir = $wd:expr)? $(,)?) => {{
        let mut cmd = ::std::process::Command::new($s);

        $(
            $(cmd.arg($args);)+
        )?
        $(cmd.current_dir($wd);)?

        println!("Running command: {:?}", &cmd);

        cmd
    }};

    (#[sh] $s:expr) => {{
        let mut cmd: ::std::process::Command;
        if cfg!(target_os = "windows") {
            cmd = ::std::process::Command::new("cmd");
            cmd.args(&["/C", $s]);
        } else {
            cmd = ::std::process::Command::new("sh");
            cmd.arg("-c").arg($s);
        }

        println!("Running command: {:?}", &cmd);

        cmd
    }};

    (#[sh] $s:expr, workdir = $wd:expr) => {{
        let mut cmd: ::std::process::Command;
        if cfg!(target_os = "windows") {
            cmd = ::std::process::Command::new("cmd");
            cmd.args(&["/C", $s]);
        } else {
            cmd = ::std::process::Command::new("sh");
            cmd.arg("-c").arg($s);
        }

        cmd.current_dir($wd);

        println!("Running command: {:?}", &cmd)

        cmd
    }};
}

macro_rules! cmd {
    ($($args:expr),+ $(, workdir = $wd:expr)? $(,)?) => {
        $crate::cmd::cmd_process!($($args),+ $(, workdir = $wd)?)
            .output()
            .expect("failed to execute process")
    };
}

macro_rules! cmd_call {
    ($($args:expr),+ $(, workdir = $wd:expr)? $(,)?) => {
        {
            let v: $crate::cmd::CmdResult = try {
                let exit_status = $crate::cmd::cmd_process!($($args),+ $(, workdir = $wd)?).spawn()?.wait()?;
                if !exit_status.success() {
                    Err::<(), _>($crate::cmd::CmdError::NotSuccess(exit_status))?
                }

                ()
            };
            v
        }
    };
}

macro_rules! cargo_cmd {
    ($($args:expr),+ $(,)?) => {
        {
            let __cargo_path = $crate::cmd::cargo_path();
            $crate::cmd::cmd_call!(__cargo_path, $($args),+)
        }
    };
}

pub(crate) fn cargo_path() -> PathBuf {
    let p = PathBuf::from(env!("CARGO"));

    p
}

#[derive(Debug)]
pub enum CmdError {
    IoError(io::Error),

    NotSuccess(ExitStatus),
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "io error: {}", err),
            Self::NotSuccess(status) => write!(f, "not success: {}", status),
        }
    }
}

impl From<io::Error> for CmdError {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

impl std::error::Error for CmdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(err) => Some(err),
            Self::NotSuccess(_) => None,
        }
    }
}

pub(crate) type CmdResult<T = ()> = Result<T, CmdError>;
pub(crate) use {cargo_cmd, cmd, cmd_call, cmd_process};
