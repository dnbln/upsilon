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

use std::path::PathBuf;
use std::process::Child;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use clap::Parser;

#[derive(Parser, Debug)]
struct Run {
    program: String,
    /// On the creation of this file, kill the program.
    #[clap(short, long = "murderer")]
    murderer_file: Option<PathBuf>,
    #[clap(short, long = "arg")]
    args: Vec<String>,
}

fn kill_child(child: &Child) {
    #[cfg(target_os = "linux")]
    linux::kill_child(child);

    #[cfg(windows)]
    win_impl::kill_child(child);

    std::thread::sleep(Duration::from_secs(1));
}

fn main() {
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
        murderer_file,
    } = run;

    let mut cmd = std::process::Command::new(program);
    cmd.args(args);
    let child = cmd.spawn().expect("Failed to spawn subprocess");
    {
        let mut lock = child_mutex.lock().unwrap();
        *lock = Some(child);
    }

    loop {
        let murderer_file_created = murderer_file.as_ref().map_or(false, |f| f.exists());

        {
            let mut lock = child_mutex.lock().unwrap();

            if let Some(mut child) = lock.take() {
                let status = child.try_wait().expect("Failed to wait for child process");

                if let Some(status) = status {
                    std::process::exit(status.code().unwrap_or(0));
                }

                if murderer_file_created {
                    kill_child(&child);
                    std::process::exit(0);
                }

                *lock = Some(child);
            }
        }

        std::thread::sleep(Duration::from_millis(100));
    }
}
