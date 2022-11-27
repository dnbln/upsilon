use std::path::PathBuf;

use spec_serde::Config;

fn main() {
    let res = Config::new().with_file(
        PathBuf::from("api-models/spec/models.modelspec"),
        PathBuf::from("test.rs"),
    ).run();

    if !*res {
        panic!("Failed to generate models");
    }
}
