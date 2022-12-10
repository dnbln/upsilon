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
                println!("Subprocess exited with status: {:?}", status);
                *subproc = None;

                return;
            }
        }

        std::thread::sleep(WAIT_DURATION);
    }
}

fn main() {
    let app: App = App::parse();
    let subprocess = Arc::new(Mutex::new(None::<Child>));

    {
        let subprocess = Arc::clone(&subprocess);

        ctrlc::set_handler(move || {
            println!("Ctrl-C pressed, exiting");

            let mut subprocess = subprocess.lock().unwrap();
            if let Some(mut subprocess) = subprocess.take() {
                subprocess.kill().expect("Failed to kill subprocess");
            }
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
