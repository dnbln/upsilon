extern crate ra_ap_hir;
extern crate ra_ap_hir_def;
extern crate ra_ap_hir_expand;
extern crate ra_ap_hir_ty;
extern crate ra_ap_syntax;

use std::path::PathBuf;

use ra_ap_ide_db::FxHashMap;
use ra_ap_paths::AbsPathBuf;
use ra_ap_project_model::{
    CargoConfig, CargoFeatures, InvocationLocation, InvocationStrategy, ProjectManifest, ProjectWorkspace, UnsetTestCrates
};
use ra_ap_rust_analyzer::cli::load_cargo::LoadCargoConfig;

pub type DifftestsResult<T = ()> = anyhow::Result<T>;

pub fn cargo_config() -> CargoConfig {
    let features = CargoFeatures::All;
    let target = None;
    let sysroot = None;
    let rustc_source = None;
    let unset_test_crates = UnsetTestCrates::None;
    let wrap_rustc_in_build_scripts = true;
    let run_build_script_command = None;
    let extra_env = FxHashMap::default();
    let invocation_strategy = InvocationStrategy::PerWorkspace;
    let invocation_location = InvocationLocation::Workspace;

    CargoConfig {
        features,
        target,
        sysroot,
        rustc_source,
        unset_test_crates,
        wrap_rustc_in_build_scripts,
        run_build_script_command,
        extra_env,
        invocation_strategy,
        invocation_location,
    }
}

pub fn load_cargo_config() -> LoadCargoConfig {
    LoadCargoConfig {
        load_out_dirs_from_check: true,
        with_proc_macro: false,
        prefill_caches: false,
    }
}

pub fn load_project_workspace(
    root: PathBuf,
    cargo_config: &CargoConfig,
    progress: &dyn Fn(String),
) -> DifftestsResult<ProjectWorkspace> {
    let root = AbsPathBuf::assert(root);
    let root = ProjectManifest::discover_single(&root)?;

    ProjectWorkspace::load(root, cargo_config, progress)
}
