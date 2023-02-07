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

#[cfg(all(not(windows), not(target_os = "linux")))]
compile_error!("Unsupported OS");

#[cfg(windows)]
mod win_impl;

#[cfg(target_os = "linux")]
mod linux_impl;

fn prepare_kill(child: &Child) -> PrepResult {
    #[cfg(target_os = "linux")]
    return linux_impl::prepare(child);

    #[cfg(windows)]
    return win_impl::prepare(child);
}

use std::path::PathBuf;
use std::process::{Child, ExitCode};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use clap::Parser;

#[derive(Parser, Debug)]
struct Run {
    program: String,
    /// On the creation of this file, kill the program.
    #[clap(long = "kfile")]
    kfile: Option<PathBuf>,
    #[clap(long = "grace", default_value = "60s", value_parser = parse_duration)]
    grace: Duration,
    #[clap(long = "arg")]
    args: Vec<String>,
}

fn parse_duration(s: &str) -> Result<Duration, String> {
    let mut s = s.to_string();
    if s.ends_with('s') {
        s.pop();
    }
    s.parse::<u64>()
        .map(Duration::from_secs)
        .map_err(|e| format!("Cannot parse duration: {}", e))
}

fn kill_child(child: &Child) {
    #[cfg(target_os = "linux")]
    linux_impl::kill_child(child);

    #[cfg(windows)]
    win_impl::kill_child(child);

    std::thread::sleep(Duration::from_secs(1));
}

#[cfg(target_os = "linux")]
use linux_impl::PrepResult;
#[cfg(windows)]
use win_impl::PrepResult;

fn kill_child_with_prep_result(child: &Child, prep_result: PrepResult) {
    #[cfg(target_os = "linux")]
    linux_impl::kill_child_with_prep_result(child, prep_result);

    #[cfg(windows)]
    win_impl::kill_child_with_prep_result(child, prep_result);

    std::thread::sleep(Duration::from_secs(1));
}

fn main() -> ExitCode {
    println!(
        "gracefully shutdown host LLVM_PROFILE_FILE: {:?}",
        std::env::var("LLVM_PROFILE_FILE")
    );
    let child_mutex = Arc::new(Mutex::new(None));

    {
        let child_mutex = Arc::clone(&child_mutex);
        ctrlc::set_handler(move || {
            let child = child_mutex.lock().unwrap().take();

            if let Some(mut child) = child {
                kill_child(&child);

                let result = child
                    .try_wait()
                    .expect("Child should have exited by now")
                    .expect("Child should have exited by now")
                    .code()
                    .expect("code");

                std::process::exit(result);
            }
        })
        .expect("Failed to set CtrlC handler");
    }

    let run = Run::parse();

    let Run {
        program,
        args,
        kfile,
        grace,
    } = run;

    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    let child = cmd.spawn().expect("Failed to spawn subprocess");
    {
        let mut lock = child_mutex.lock().unwrap();
        *lock = Some(child);
    }

    loop {
        let kfile_created = kfile.as_ref().map_or(false, |f| f.exists());

        {
            let mut lock = child_mutex.lock().unwrap();

            if let Some(mut child) = lock.take() {
                let status = child.try_wait().expect("Failed to wait for child process");

                if let Some(status) = status {
                    return ExitCode::from(status.code().unwrap_or(1).try_into().unwrap_or(1));
                }

                if kfile_created {
                    let prep_result = prepare_kill(&child);

                    std::fs::write(kfile.as_ref().unwrap(), "a").expect("Failed to write to kfile");

                    let grace_time = Instant::now() + grace;

                    loop {
                        if Instant::now() > grace_time {
                            break;
                        }

                        let status = child.try_wait().expect("Failed to wait for child process");
                        if status.is_some() {
                            break;
                        }

                        std::thread::sleep(Duration::from_millis(100));
                    }

                    kill_child_with_prep_result(&child, prep_result);

                    let code = dbg!(child
                        .try_wait()
                        .expect("Failed to wait for child process")
                        .expect("Child should have exited by now")
                        .code())
                    .unwrap_or(1);

                    return ExitCode::from(code.try_into().unwrap_or(1));
                }

                *lock = Some(child);
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
