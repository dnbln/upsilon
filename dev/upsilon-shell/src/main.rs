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

use clap::builder::ArgPredicate;
use clap::Parser;
use log::info;
use rustyline::error::ReadlineError;
use rustyline::{ColorMode, CompletionType, Config};
use serde::Deserialize;
use upsilon_shell::{
    parse_line, BuildUrlError, Client, GqlQueryResult, Helper, UshHostInfo, UshParsedCommand, UshRepoAccessProtocol
};

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
    #[clap(
        long,
        default_value = "http://localhost:8000/graphql",
        default_value_ifs([
            ("hostname", ArgPredicate::IsPresent, None),
            ("https", ArgPredicate::IsPresent, None),
            ("http-port", ArgPredicate::IsPresent, None),
        ])
    )]
    gql_endpoint: Option<String>,
    /// Do not reconfigure the shell, just connect to the server.
    #[clap(long = "no-reconfigure", default_value_t = true, action = clap::ArgAction::SetFalse)]
    reconfigure: bool,
}

impl App {
    fn into_parsed(self) -> ParsedApp {
        let App {
            ssh,
            ssh_port,
            git_protocol,
            git_protocol_port,
            git_http,
            http_port,
            https,
            hostname,
            gql_endpoint,
            reconfigure,
        } = self;

        let gql_endpoint = gql_endpoint.unwrap_or_else(|| {
            let (proto, default_port) = if https { ("https", 443) } else { ("http", 80) };
            let hostname = hostname.as_str();
            let port = http_port;

            if port == default_port {
                format!("{proto}://{hostname}/graphql")
            } else {
                format!("{proto}://{hostname}:{port}/graphql")
            }
        });

        ParsedApp {
            ssh,
            ssh_port,
            git_protocol,
            git_protocol_port,
            git_http,
            http_port,
            https,
            hostname,
            gql_endpoint,
            reconfigure,
        }
    }
}

#[derive(Clone)]
pub struct ParsedApp {
    ssh: bool,
    ssh_port: u16,
    git_protocol: bool,
    git_protocol_port: u16,
    git_http: bool,
    http_port: u16,
    https: bool,
    hostname: String,
    gql_endpoint: String,
    reconfigure: bool,
}

impl ParsedApp {
    fn to_ush_host_info(&self) -> UshHostInfo {
        UshHostInfo {
            git_ssh_enabled: self.ssh,
            ssh_port: self.ssh_port,
            git_protocol_enabled: self.git_protocol,
            git_port: self.git_protocol_port,
            git_http_enabled: self.git_http,
            http_port: self.http_port,
            https_enabled: self.https,
            hostname: self.hostname.clone(),
        }
    }
}

const HISTORY_FILE: &str = ".ush-history";

fn report_gql_result<T>(result: &GqlQueryResult<T>) {
    match result {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error: {e}");
        }
    }
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init_custom_env("UPSILON_SHELL_LOG");

    let mut app = App::parse().into_parsed();

    {
        info!("Connecting to {}", app.gql_endpoint);

        let temp_client = Client::new(app.gql_endpoint.clone());

        #[derive(Deserialize)]
        struct InitialRequestResponse {
            #[serde(rename = "apiVersion")]
            api_version: String,
            #[serde(rename = "ushCliArgs")]
            ush_cli_args: Vec<String>,
        }

        let response = temp_client
            .gql_query::<InitialRequestResponse>(
                //language=GraphQL
                "
query {
  apiVersion
  ushCliArgs
}
",
                None,
            )
            .await
            .unwrap();

        let InitialRequestResponse {
            api_version,
            ush_cli_args,
        } = response;

        info!(
            "Connected to {}, api version is {api_version}",
            app.gql_endpoint
        );
        info!(
            "{} ush cli args: {:?}",
            if app.reconfigure {
                "Reconfiguring with received"
            } else {
                "Received"
            },
            ush_cli_args
        );

        if app.reconfigure {
            app.clone_from(
                &App::parse_from(
                    std::env::args_os()
                        .next()
                        .into_iter()
                        .chain(ush_cli_args.into_iter().map(std::ffi::OsString::from)),
                )
                .into_parsed(),
            );
        }
    }

    let app = app; // remove mutability
    let client = Client::new(app.gql_endpoint.clone());

    let mut editor = rustyline::Editor::<Helper, rustyline::history::DefaultHistory>::with_config(
        Config::builder()
            .auto_add_history(true)
            .completion_type(CompletionType::List)
            .color_mode(ColorMode::Enabled)
            .build(),
    )
    .unwrap();

    if editor.load_history(HISTORY_FILE).is_err() {
        info!("No previous history.");
    }

    let cwd = Rc::new(RefCell::new(std::env::current_dir().unwrap()));

    editor.set_helper(Some(Helper {
        cwd: Rc::clone(&cwd),
        usermap: Rc::clone(client.usermap()),
    }));

    let exit_code = loop {
        let cwd_str = cwd.borrow().display().to_string();

        #[cfg(windows)]
        let cwd_str = {
            // quirk on windows
            // \\?\C:\Users => C:\Users
            cwd_str.replace("\\\\?\\", "")
        };

        let line = match editor.readline(&format!("ush: [{cwd_str}] >>> ")) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                eprintln!("^C");
                continue;
            }
            Err(ReadlineError::Eof) => {
                eprintln!("^D");
                break 0;
            }
            Err(err) => {
                eprintln!("Error: {err:?}");
                break 1;
            }
        };

        let parsed = match parse_line(&line) {
            Ok(parsed) => parsed,
            Err(err) => {
                eprintln!("Error: {err}");
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
                    eprintln!("cd: {}: No such directory", new_path.display());
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
                let r = client
                    .login(&login.username.value, &login.password.value)
                    .await;

                report_gql_result(&r);
            }
            UshParsedCommand::CreateUser(create_user) => {
                let r = client
                    .create_user(
                        &create_user.username.value,
                        &create_user.email.value,
                        &create_user.password.value,
                    )
                    .await;

                report_gql_result(&r);
            }
            UshParsedCommand::CreateRepo(create_repo) => {
                println!("create repo {}", create_repo.repo_name.value);
            }
            UshParsedCommand::Clone(clone) => {
                println!("clone {} to {}", clone.repo_path.value, clone.to.0);
            }
            UshParsedCommand::HttpUrl(http_url) => {
                match UshRepoAccessProtocol::Http
                    .build_url(&http_url.repo_path.value, &app.to_ush_host_info())
                {
                    Ok(url) => println!("{url}"),
                    Err(BuildUrlError::ProtocolDisabled) => {
                        eprintln!("Seems like the http protocol was not enabled")
                    }
                }
            }
            UshParsedCommand::GitUrl(git_url) => {
                match UshRepoAccessProtocol::Git
                    .build_url(&git_url.repo_path.value, &app.to_ush_host_info())
                {
                    Ok(url) => println!("{url}"),
                    Err(BuildUrlError::ProtocolDisabled) => {
                        eprintln!("Seems like the git protocol was not enabled")
                    }
                }
            }
            UshParsedCommand::SshUrl(ssh_url) => {
                match UshRepoAccessProtocol::Ssh
                    .build_url(&ssh_url.repo_path.value, &app.to_ush_host_info())
                {
                    Ok(url) => println!("{url}"),
                    Err(BuildUrlError::ProtocolDisabled) => {
                        eprintln!("Seems like the ssh protocol was not enabled")
                    }
                }
            }
            UshParsedCommand::Url(url) => {
                match url
                    .protocol
                    .build_url(&url.repo_path.value, &app.to_ush_host_info())
                {
                    Ok(url) => println!("{url}"),
                    Err(BuildUrlError::ProtocolDisabled) => {
                        eprintln!("Seems like the {:?} protocol was not enabled", url.protocol)
                    }
                }
            }
            UshParsedCommand::UploadSshKey(upload_ssh_key) => {
                println!("upload ssh key {}", upload_ssh_key.key.value.0);
            }
            UshParsedCommand::ListUsers(list_users) => {
                client
                    .usermap()
                    .borrow()
                    .for_each_user(|username, _tokens| {
                        println!("{username}");
                    });
            }
        }
    };

    editor.save_history(HISTORY_FILE).unwrap();

    std::process::exit(exit_code);
}
