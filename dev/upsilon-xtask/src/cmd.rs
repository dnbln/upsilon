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

use std::path::PathBuf;
use std::process::ExitStatus;
use std::string::FromUtf8Error;
use std::{fmt, io};

use log::info;

#[macro_export]
macro_rules! cmd_args_process {
    (@@process_one $cmd_args:expr $(,)?) => {
    };

    (@@process_one $cmd_args:expr, ...$arg:expr => @if $arg_condition:expr, $($remaining:tt)*) => {
        if $arg_condition {
            for arg in $arg {
                $cmd_args.push(arg.into());
            }
        }

        $crate::cmd_args_process!(@@process_one $cmd_args, $($remaining)*);
    };
    (@@process_one $cmd_args:expr, ...$arg:expr => @if let $pat:pat = $pat_expr:expr, $($remaining:tt)*) => {
        if let $pat = $pat_expr {
            for arg in $arg {
                $cmd_args.push(arg.into());
            }
        }

        $crate::cmd_args_process!(@@process_one $cmd_args, $($remaining)*);
    };
    (@@process_one $cmd_args:expr, ...$arg:expr, $($remaining:tt)*) => {
        for arg in $arg {
            $cmd_args.push(arg.into());
        }

        $crate::cmd_args_process!(@@process_one $cmd_args, $($remaining)*);
    };

    (@@process_one $cmd_args:expr, ...$arg:expr => @if $arg_condition:expr $(,)?) => {
        if $arg_condition {
            for arg in $arg {
                $cmd_args.args(arg.into());
            }
        }
    };
    (@@process_one $cmd_args:expr, ...$arg:expr => @if let $pat:pat = $pat_expr:expr $(,)?) => {
        if let $pat = $pat_expr {
            for arg in $arg {
                $cmd_args.push(arg.into());
            }
        }
    };
    (@@process_one $cmd_args:expr, ...$arg:expr $(,)?) => {
        for arg in $arg {
            $cmd_args.push(arg.into());
        }
    };

    (@@process_one $cmd_args:expr, $arg:expr => @if $arg_condition:expr, $($remaining:tt)*) => {
        if $arg_condition {
            $cmd_args.push($arg.into());
        }

        $crate::cmd_args_process!(@@process_one $cmd_args, $($remaining)*);
    };
    (@@process_one $cmd_args:expr, $arg:expr => @if let $pat:pat = $pat_expr:expr, $($remaining:tt)*) => {
        if let $pat = $pat_expr {
            $cmd_args.push($arg.into());
        }

        $crate::cmd_args_process!(@@process_one $cmd_args, $($remaining)*);
    };
    (@@process_one $cmd_args:expr, $arg:expr, $($remaining:tt)*) => {
        $cmd_args.push($arg.into());

        $crate::cmd_args_process!(@@process_one $cmd_args, $($remaining)*);
    };

    (@@process_one $cmd_args:expr, $arg:expr => @if $arg_condition:expr $(,)?) => {
        if $arg_condition {
            $cmd_args.push($arg.into());
        }
    };
    (@@process_one $cmd_args:expr, $arg:expr => @if let $pat:pat = $pat_expr:expr $(,)?) => {
        if let $pat = $pat_expr {
            $cmd_args.push($arg.into());
        }
    };
    (@@process_one $cmd_args:expr, $arg:expr $(,)?) => {
        $cmd_args.push($arg.into());
    };
}

#[macro_export]
macro_rules! cmd_args {
    ($($args:tt)*) => {
        {
            let mut cmd_args = Vec::<std::ffi::OsString>::new();
            $crate::cmd_args_process!(@@process_one cmd_args, $($args)*);
            cmd_args
        }
    };
}

#[macro_export]
macro_rules! cmd_process {
    (@@process_one $cmd:expr $(,)?) => {
    };

    (@@process_one $cmd:expr, ...$arg:expr => @if $arg_condition:expr, $($remaining:tt)*) => {
        if $arg_condition {
            $cmd.args($arg);
        }

        $crate::cmd_process!(@@process_one $cmd, $($remaining)*);
    };
    (@@process_one $cmd:expr, ...$arg:expr => @if let $pat:pat = $pat_expr:expr, $($remaining:tt)*) => {
        if let $pat = $pat_expr {
            $cmd.args($arg);
        }

        $crate::cmd_process!(@@process_one $cmd, $($remaining)*);
    };
    (@@process_one $cmd:expr, ...$arg:expr, $($remaining:tt)*) => {
        $cmd.args($arg);

        $crate::cmd_process!(@@process_one $cmd, $($remaining)*);
    };

    (@@process_one $cmd:expr, ...$arg:expr => @if $arg_condition:expr $(,)?) => {
        if $arg_condition {
            $cmd.args($arg);
        }
    };
    (@@process_one $cmd:expr, ...$arg:expr => @if let $pat:pat = $pat_expr:expr $(,)?) => {
        if let $pat = $pat_expr {
            $cmd.args($arg);
        }
    };
    (@@process_one $cmd:expr, ...$arg:expr $(,)?) => {
        $cmd.args($arg);
    };

    (@@process_one $cmd:expr, $arg:expr => @if $arg_condition:expr, $($remaining:tt)*) => {
        if $arg_condition {
            $cmd.arg($arg);
        }

        $crate::cmd_process!(@@process_one $cmd, $($remaining)*);
    };
    (@@process_one $cmd:expr, $arg:expr => @if let $pat:pat = $pat_expr:expr, $($remaining:tt)*) => {
        if let $pat = $pat_expr {
            $cmd.arg($arg);
        }

        $crate::cmd_process!(@@process_one $cmd, $($remaining)*);
    };
    (@@process_one $cmd:expr, $arg:expr, $($remaining:tt)*) => {
        $cmd.arg($arg);

        $crate::cmd_process!(@@process_one $cmd, $($remaining)*);
    };

    (@@process_one $cmd:expr, $arg:expr => @if $arg_condition:expr $(,)?) => {
        if $arg_condition {
            $cmd.arg($arg);
        }
    };
    (@@process_one $cmd:expr, $arg:expr => @if let $pat:pat = $pat_expr:expr $(,)?) => {
        if let $pat = $pat_expr {
            $cmd.arg($arg);
        }
    };
    (@@process_one $cmd:expr, $arg:expr $(,)?) => {
        $cmd.arg($arg);
    };


    (@@process_one $cmd:expr, @env $env_name:expr => $env_value:expr => @if $env_condition:expr, $($remaining:tt)*) => {
        if $env_condition {
            $cmd.env($env_name, $env_value);
        }

        $crate::cmd_process!(@@process_one $cmd, $($remaining)*);
    };
    (@@process_one $cmd:expr, @env $env_name:expr => $env_value:expr, $($remaining:tt)*) => {
        $cmd.env($env_name, $env_value);

        $crate::cmd_process!(@@process_one $cmd, $($remaining)*);
    };

    (@@process_one $cmd:expr, @env $env_name:expr => $env_value:expr => @if $env_condition:expr $(,)?) => {
        if $env_condition {
            $cmd.env($env_name, $env_value);
        }
    };
    (@@process_one $cmd:expr, @env $env_name:expr => $env_value:expr $(,)?) => {
        $cmd.env($env_name, $env_value);
    };


    (@@process_one $cmd:expr, @workdir = $wd:expr, $($remaining:tt)*) => {
        $cmd.current_dir($wd);

        $crate::cmd_process!(@@process_one $cmd, $($remaining)*);
    };

    (@@process_one $cmd:expr, @workdir = $wd:expr $(,)?) => {
        $cmd.current_dir($wd);
    };


    ($exe:expr) => {{
        let mut cmd = ::std::process::Command::new($exe);
        println!("Running command: {:?}", &cmd);
        cmd
    }};

    ($exe:expr, $($rest:tt)*) => {{
        #[allow(unused_mut)]
        let mut cmd = ::std::process::Command::new($exe);

        $crate::cmd_process!(@@process_one cmd, $($rest)*);

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

#[macro_export]
macro_rules! cmd_output {
    ($($args:tt)+) => {
        {
            (|| -> $crate::cmd::CmdResult<Vec<u8>> {
                let mut child = $crate::cmd_process!($($args)*)
                    .stdout(::std::process::Stdio::piped())
                    .spawn()?;

                let exit_status = child.wait()?;
                if !exit_status.success() {
                    return Err($crate::cmd::CmdError::NotSuccess(exit_status))
                }

                let mut result = Vec::new();

                std::io::Read::read_to_end(child.stdout.as_mut().unwrap(), &mut result)?;

                Ok(result)
            })()
        }
    };
}

#[macro_export]
macro_rules! cmd_output_string {
    ($($args:tt)*) => {
        (|| -> $crate::cmd::CmdResult<String> {
            let output = $crate::cmd_output!($($args)*)?;
            let s = String::from_utf8(output)?;

            Ok(s)
        })()
    };
}

#[macro_export]
macro_rules! cmd_output_pipe_to_file {
    (@ $path:expr, $($args:tt)*) => {
        {
            (|| -> $crate::cmd::CmdResult<()> {
                let mut child = $crate::cmd_process!($($args)*)
                    .stdout(::std::process::Stdio::from(std::fs::File::create($path)?))
                    .spawn()?;

                let exit_status = child.wait()?;
                if !exit_status.success() {
                    return Err($crate::cmd::CmdError::NotSuccess(exit_status))
                }

                Ok(())
            })()
        }
    };
}

#[macro_export]
macro_rules! cmd_call {
    ($($args:tt)+) => {
        {
            (|| -> $crate::cmd::CmdResult {
                let exit_status = $crate::cmd_process!($($args)*).spawn()?.wait()?;
                if !exit_status.success() {
                    Err::<(), _>($crate::cmd::CmdError::NotSuccess(exit_status))?
                }

                Ok(())
            })()
        }
    };
}

#[macro_export]
macro_rules! cargo_cmd {
    ($($args:tt)*) => {
        {
            let __cargo_path = $crate::cmd::cargo_path();
            let __cargo_build_profile_file_name = $crate::cmd::cargo_build_profile_file_name();
            $crate::cmd_call!(
                __cargo_path,
                @env "LLVM_PROFILE_FILE" => __cargo_build_profile_file_name,
                $($args)*
            )
        }
    };
}

#[macro_export]
macro_rules! cargo_cmd_output {
    ($($args:tt)*) => {
        {
            let __cargo_path = $crate::cmd::cargo_path();
            let __cargo_build_profile_file_name = $crate::cmd::cargo_build_profile_file_name();
            $crate::cmd_output_string!(
                __cargo_path,
                @env "LLVM_PROFILE_FILE" => __cargo_build_profile_file_name,
                $($args)*
            )
        }
    };
}

pub fn cargo_build_profiles_dir() -> PathBuf {
    crate::ws_path!("target" / "profiles")
}

pub fn cargo_build_profile_file_name() -> PathBuf {
    let p = cargo_build_profiles_dir();
    crate::ws_path_join!(p / "prof_%m_%p.profraw")
}

#[macro_export]
macro_rules! npm_cmd {
    ($($args:tt)*) => {
        {
            let __npm_path = $crate::cmd::npm_path();
            $crate::cmd_call!(__npm_path, $($args)*)
        }
    };
}

pub fn cargo_path() -> PathBuf {
    match option_env!("UXTASK_USE_GLOBAL_CARGO") {
        Some(_) => {
            info!("UXTASK_USE_GLOBAL_CARGO is set, using global cargo");
            PathBuf::from("cargo")
        }
        None => PathBuf::from(env!("CARGO")),
    }
}

#[cfg(not(windows))]
const NPM_NAME: &str = "npm";
#[cfg(windows)]
const NPM_NAME: &str = "npm.cmd";

pub fn npm_path() -> PathBuf {
    PathBuf::from(NPM_NAME)
}

#[derive(Debug)]
pub enum CmdError {
    IoError(io::Error),

    NotSuccess(ExitStatus),

    Utf8Error(FromUtf8Error),
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "io error: {err}"),
            Self::NotSuccess(status) => write!(f, "not success: {status}"),
            Self::Utf8Error(err) => write!(f, "utf8 error: {err}"),
        }
    }
}

impl From<io::Error> for CmdError {
    fn from(err: io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<FromUtf8Error> for CmdError {
    fn from(err: FromUtf8Error) -> Self {
        Self::Utf8Error(err)
    }
}

impl std::error::Error for CmdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(err) => Some(err),
            Self::NotSuccess(_) => None,
            Self::Utf8Error(err) => Some(err),
        }
    }
}

pub type CmdResult<T = ()> = Result<T, CmdError>;
