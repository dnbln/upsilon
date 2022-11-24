#![feature(try_blocks)]

use std::path::PathBuf;
use clap::Parser;
use crate::result::XtaskResult;

mod result;
mod gen_models;
mod cmd;
mod ws;

#[derive(Parser, Debug)]
enum App {
    #[clap(name = "gen-dart-models")]
    GenDartModels {
        #[arg(short, long)]
        #[clap(default_value_os_t = ws::ws_path!("client-app/upsilon_client"))]
        target: PathBuf,
    }
}

fn main() -> XtaskResult<()> {
    let app: App = App::parse();

    match app {
        App::GenDartModels { target } => {
            gen_models::gen_models(target)?;
        },
    }

    Ok(())
}