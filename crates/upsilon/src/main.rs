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

use std::path::Path;
use std::process::Child;
use std::sync::{Arc, Mutex};

use clap::Parser;

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "web")]
    Web,
}

fn proc(exe: &Path, subproc: &Mutex<Option<Child>>) {
    let child = std::process::Command::new(exe)
        .spawn()
        .expect("Failed to spawn subprocess");

    let mut subproc = subproc.lock().unwrap();
    *subproc = Some(child);
}

const WAIT_DURATION: std::time::Duration = std::time::Duration::from_millis(500);

fn wait_loop(subproc: &Mutex<Option<Child>>) {
    loop {
        let mut subproc = subproc.lock().unwrap();

        if let Some(child) = subproc.as_mut() {
            if let Ok(Some(status)) = child.try_wait() {
                println!("Subprocess exited with status: {status:?}");
                *subproc = None;

                return;
            }
        }

        drop(subproc);

        std::thread::sleep(WAIT_DURATION);
    }
}

fn main() {
    let app: App = App::parse();
    let subprocess = Arc::new(Mutex::new(None::<Child>));

    {
        ctrlc::set_handler(move || {
            println!("Ctrl-C pressed, exiting");
        })
        .expect("Failed to set Ctrl-C handler");
    }

    match app {
        App::Web => {
            proc(&upsilon_core::alt_exe("upsilon-web"), &subprocess);
        }
    }

    wait_loop(&subprocess);
}
