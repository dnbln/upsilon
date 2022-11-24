use crate::cmd::{cargo_cmd, cmd, cmd_call};
use crate::result::XtaskResult;
use crate::ws::ws_path_join;
use std::path::PathBuf;

pub fn gen_models(target: PathBuf) -> XtaskResult<()> {
    let target_lib_dir = ws_path_join!(target / "lib");

    let models_dir = ws_path_join!(
        #[clone]
        target_lib_dir
            / "models"
    );
    let target_file = ws_path_join!(target_lib_dir / "models" / "models.g.dart");

    std::fs::create_dir_all(&models_dir)?;

    cargo_cmd!(
        "run",
        "-p",
        "upsilon-api-models",
        "--bin",
        "gen_models",
        "--",
        &target_file
    )?;

    cmd_call!("dart", "format", &target_file)?;

    Ok(())
}
