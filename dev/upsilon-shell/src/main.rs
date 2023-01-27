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

use std::cell::RefCell;
use std::rc::Rc;

use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::{ColorMode, CompletionType, Config};
use upsilon_shell::{parse_line, Helper, UserMap, UshParsedCommand};

#[derive(Parser, Debug)]
pub struct App {
    #[clap(long)]
    ssh: bool,
    #[clap(long, default_value_t = 22)]
    ssh_port: u16,
    #[clap(long)]
    git_protocol: bool,
    #[clap(long, default_value_t = 9418)]
    git_protocol_port: u16,
    #[clap(long)]
    git_http: bool,
    #[clap(long, default_value_t = 8000)]
    http_port: u16,
    #[clap(long)]
    https: bool,
    #[clap(long, default_value = "localhost")]
    hostname: String,
}

const HISTORY_FILE: &str = ".ush-history";

fn main() {
    let app = App::parse();

    let mut editor = rustyline::Editor::<Helper>::with_config(
        Config::builder()
            .auto_add_history(true)
            .completion_type(CompletionType::List)
            .color_mode(ColorMode::Enabled)
            .build(),
    )
    .unwrap();

    if editor.load_history(HISTORY_FILE).is_err() {
        println!("No previous history.");
    }

    let cwd = Rc::new(RefCell::new(std::env::current_dir().unwrap()));
    let usermap = Rc::new(RefCell::new(UserMap::default()));

    editor.set_helper(Some(Helper {
        cwd: cwd.clone(),
        usermap: usermap.clone(),
    }));

    let exit_code = loop {
        let mut cwd_str = cwd.borrow().display().to_string();
        #[cfg(windows)]
        {
            // quirk on windows
            // \\?\C:\Users => C:\Users
            cwd_str = cwd_str.replace("\\\\?\\", "");
        }

        let line = match editor.readline(&format!("ush: [{cwd_str}] >>> ")) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("^D");
                break 0;
            }
            Err(err) => {
                println!("Error: {err:?}");
                break 1;
            }
        };

        let parsed = match parse_line(&line) {
            Ok(parsed) => parsed,
            Err(err) => {
                println!("Error: {err}");
                continue;
            }
        };

        match parsed {
            UshParsedCommand::Cd(cd) => {
                let new_path = match cd.path {
                    Some(path) => cwd
                        .borrow()
                        .join(path.value.deref_path())
                        .canonicalize()
                        .unwrap(),
                    None => home::home_dir().unwrap(),
                };

                if new_path.is_dir() {
                    *cwd.borrow_mut() = new_path;
                } else {
                    println!("cd: {}: No such file or directory", new_path.display());
                }
            }
            UshParsedCommand::Ls(ls) => {
                if let Some(path) = ls.path {
                    println!("ls {}", path.value.deref_path().display());
                } else {
                    println!("ls");
                }
            }
            UshParsedCommand::Pwd(pwd) => {
                println!("pwd: {}", cwd.borrow().display());
            }
            UshParsedCommand::Echo(echo) => {
                for arg in &echo.args {
                    print!("{} ", arg.value);
                }
                println!();
            }
            UshParsedCommand::Exit(exit) => {
                if let Some(e) = exit.exit_code {
                    break e.value;
                } else {
                    break 0;
                }
            }
            UshParsedCommand::Login(login) => {
                println!(
                    "login {} with pass {}",
                    login.username.value.0, login.password.value
                );
            }
            UshParsedCommand::CreateUser(create_user) => {
                println!(
                    "create user {} with email {} and pass {}",
                    create_user.username.value.0,
                    create_user.email.value,
                    create_user.password.value
                );
            }
            UshParsedCommand::CreateRepo(create_repo) => {
                println!("create repo {}", create_repo.repo_name.value);
            }
            UshParsedCommand::Clone(clone) => {
                println!("clone {} to {}", clone.repo_path.value, clone.to.0);
            }
            UshParsedCommand::HttpUrl(http_url) => {
                if app.git_http {
                    let default_port = if app.https { 443 } else { 80 };
                    let proto = if app.https { "https" } else { "http" };

                    match app.http_port {
                        port if port == default_port => {
                            println!("{proto}://{}/{}", app.hostname, http_url.repo_path.value)
                        }
                        port => println!(
                            "{proto}://{}:{port}/{}",
                            app.hostname, http_url.repo_path.value
                        ),
                    }
                } else {
                    eprintln!("Seems like git over http was not enabled");
                }
            }
            UshParsedCommand::GitUrl(git_url) => {
                if app.git_protocol {
                    match app.git_protocol_port {
                        9418 => println!("git://{}/{}", app.hostname, git_url.repo_path.value),
                        port => {
                            println!("git://{}:{port}/{}", app.hostname, git_url.repo_path.value)
                        }
                    };
                } else {
                    eprintln!("Seems like the git protocol was not enabled");
                }
            }
            UshParsedCommand::SshUrl(ssh_url) => {
                if app.ssh {
                    const SSH_USER: &str = "git";
                    match app.ssh_port {
                        22 => println!("{SSH_USER}@{}:{}", app.hostname, ssh_url.repo_path.value),
                        port => println!(
                            "ssh://{SSH_USER}@{}:{port}/{}",
                            app.hostname, ssh_url.repo_path.value
                        ),
                    };
                } else {
                    eprintln!("Seems like git over ssh was not enabled");
                }
            }
        }
    };

    editor.save_history(HISTORY_FILE).unwrap();

    std::process::exit(exit_code);
}
