#![feature(try_blocks)]

use std::path::PathBuf;

use clap::Parser;

use crate::cmd::cargo_cmd;
use crate::result::XtaskResult;

mod cmd;
mod gen_models;
mod result;
mod ws;
mod git_checks;

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "gen-models")]
    GenModels {
        #[arg(short, long)]
        #[clap(default_value_os_t = ws::ws_path!("client-app" / "upsilon_client"))]
        target: PathBuf,
    },
    #[clap(name = "fmt")]
    Fmt,
    #[clap(name = "fmt-check")]
    FmtCheck,
    #[clap(name = "git-checks")]
    GitChecks,
}

fn main() -> XtaskResult<()> {
    let app: App = App::parse();

    match app {
        App::GenModels { target } => {
            gen_models::gen_models(target)?;
        }
        App::Fmt => {
            cargo_cmd!("fmt", "--all")?;
        }
        App::FmtCheck => {
            cargo_cmd!("fmt", "--all", "--check")?;
        }
        App::GitChecks => {
            let repo = git_checks::get_repo()?;

            git_checks::linear_history(&repo)?;
        }
    }

    Ok(())
}
