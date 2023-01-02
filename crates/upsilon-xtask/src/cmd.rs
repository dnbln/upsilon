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
use std::{fmt, io};

#[macro_export]
macro_rules! cmd_process {
    (@@if_cmd_arg $cmd:expr, $arg:expr, $arg_condition:expr) => {
        if $arg_condition {
            $cmd.arg($arg);
        }
    };
    (@@if_cmd_arg $cmd:expr, $arg:expr, ) => {
        $cmd.arg($arg);
    };
    (@@if_cmd_env $cmd:expr, $env_name:expr, $env_value:expr, $env_condition:expr) => {
        if $env_condition {
            $cmd.env($env_name, $env_value);
        }
    };
    (@@if_cmd_env $cmd:expr, $env_name:expr, $env_value:expr, ) => {
        $cmd.env($env_name, $env_value);
    };
    ($s:expr $(, $($args:expr $(=> @if $arg_condition:expr)?),+)? $(, @env $env_name:expr => $env_var_value:expr $(=> @if $env_condition:expr)?)* $(, @workdir = $wd:expr)? $(,)?) => {{
        #[allow(unused_mut)]
        let mut cmd = ::std::process::Command::new($s);

        $(
            $(
                $crate::cmd_process!(@@if_cmd_arg cmd, $args, $($arg_condition)?);
            )+
        )?
        $(
            $crate::cmd_process!(@@if_cmd_env cmd, $env_name, $env_var_value, $($env_condition)?);
        )*
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

#[macro_export]
macro_rules! cmd {
    ($($args:expr),+ $(, @env $env_name:expr => $env_var_value:expr)* $(, @workdir = $wd:expr)? $(,)?) => {
        $crate::cmd_process!($($args),+ $(@env $env_name => $env_var_value,)* $(, @workdir = $wd)?)
            .output()
            .expect("failed to execute process")
    };
}

#[macro_export]
macro_rules! cmd_call {
    ($($args:expr $(=> @if $arg_condition:expr)?),+ $(, @env $env_name:expr => $env_var_value:expr $(=> @if $env_condition:expr)?)* $(, @workdir = $wd:expr)? $(,)?) => {
        {
            (|| -> $crate::cmd::CmdResult {
                let exit_status = $crate::cmd_process!($($args $(=> @if $arg_condition)?),+ $(, @env $env_name => $env_var_value $(=> @if $env_condition)?)* $(, @workdir = $wd)?).spawn()?.wait()?;
                if !exit_status.success() {
                    Err::<(), _>($crate::cmd::CmdError::NotSuccess(exit_status))?
                }

                Ok(())
            })()
        }
    };

    ($($args:expr $(=> @if $arg_condition:expr)?),+ $(, @env $env_name:expr => $env_var_value:expr $(=> @if $env_condition:expr)?)* $(, @workdir = $wd:expr)?, @logging-error-and-returnok $(,)?) => {{
        if let Err(__err) = $crate::cmd_call!($($args $(=> @if $arg_condition)?),+ $(, @env $env_name => $env_var_value $(=> @if $env_condition)?)* $(, @workdir = $wd)?) {
            eprintln!("Error: {}", __err);

            return Ok(());
        }
    }};
}

#[macro_export]
macro_rules! cargo_cmd {
    ($($args:expr $(=> @if $arg_condition:expr)?),+ $(, @env $env_name:expr => $env_var_value:expr $(=> @if $env_condition:expr)?)* $(, @workdir = $workdir:expr)? $(,)?) => {
        {
            let __cargo_path = $crate::cmd::cargo_path();
            $crate::cmd_call!(__cargo_path, $($args $(=> @if $arg_condition)?,)+ $(@env $env_name => $env_var_value $(=> @if $env_condition)?,)* $(@workdir = $workdir,)?)
        }
    };

    ($($args:expr $(=> @if $arg_condition:expr)?),+ $(, @env $env_name:expr => $env_var_value:expr $(=> @if $env_condition:expr)?)* $(, @workdir = $workdir:expr)?, @ignoring-error $(,)?) => {
        {
            let _ = $crate::cargo_cmd!($($args $(=> @if $arg_condition)?,)+ $(@env $env_name => $env_var_value $(=> @if $env_condition)?,)* $(@workdir = $workdir,)?);
        }
    };

    ($($args:expr $(=> @if $arg_condition:expr)?),+ $(, @env $env_name:expr => $env_var_value:expr $(=> @if $env_condition:expr)?)* $(, @workdir = $workdir:expr)?, @logging-error $(,)?) => {
        {
            if let Err(__err) = $crate::cargo_cmd!($($args $(=> @if $arg_condition)?,)+ $(@env $env_name => $env_var_value $(=> @if $env_condition)?,)* $(@workdir = $workdir,)?) {
                eprintln!("Error while running cargo command: {}", __err);
            }
        }
    };

    ($($args:expr $(=> @if $arg_condition:expr)?),+ $(, @env $env_name:expr => $env_var_value:expr $(=> @if $env_condition:expr)?)* $(, @workdir = $workdir:expr)?, @logging-error-and-returnok $(,)?) => {
        {
            if let Err(__err) = $crate::cargo_cmd!($($args $(=> @if $arg_condition)?,)+ $(@env $env_name => $env_var_value $(=> @if $env_condition)?,)* $(@workdir = $workdir,)?) {
                eprintln!("Error while running cargo command: {}", __err);

                return Ok(());
            }
        }
    };
}

pub fn cargo_path() -> PathBuf {
    PathBuf::from(env!("CARGO"))
}

#[derive(Debug)]
pub enum CmdError {
    IoError(io::Error),

    NotSuccess(ExitStatus),
}

impl fmt::Display for CmdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(err) => write!(f, "io error: {err}"),
            Self::NotSuccess(status) => write!(f, "not success: {status}"),
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

pub type CmdResult<T = ()> = Result<T, CmdError>;
