#![feature(try_blocks)]

use std::path::PathBuf;

use clap::Parser;

use crate::cmd::cargo_cmd;
use crate::result::XtaskResult;

mod cmd;
mod gen_models;
mod result;
mod ws;

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "gen-dart-models")]
    GenDartModels {
        #[arg(short, long)]
        #[clap(default_value_os_t = ws::ws_path!("client-app" / "upsilon_client"))]
        target: PathBuf,
    },
    #[clap(name = "fmt")]
    Fmt,
    #[clap(name = "fmt-check")]
    FmtCheck,
}

fn main() -> XtaskResult<()> {
    let app: App = App::parse();

    match app {
        App::GenDartModels { target } => {
            gen_models::gen_models(target)?;
        }
        App::Fmt => {
            cargo_cmd!("fmt", "--all")?;
        }
        App::FmtCheck => {
            cargo_cmd!("fmt", "--all", "--check")?;
        }
    }

    Ok(())
}
