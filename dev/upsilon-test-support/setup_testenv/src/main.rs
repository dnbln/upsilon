/*
 *        Copyright (c) 2022-2023 Dinu Blanovschi
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

use log::*;
use upsilon_test_support::helpers::upsilon_cloned_repo_path;

fn setup_local_clone() {
    let upsilon_repo = upsilon_cloned_repo_path();
    if !upsilon_repo.exists() {
        std::fs::create_dir_all(&upsilon_repo).expect("Failed to create upsilon repo directory");
    }

    info!(
        "Created upsilon repo directory, cloning to {}",
        upsilon_repo.display()
    );

    git2::Repository::clone("https://github.com/dnbln/upsilon", &upsilon_repo)
        .expect("Failed to clone Upsilon repository");

    info!("Cloned upsilon repository to {}", upsilon_repo.display());
}

fn env_present(name: &str) -> bool {
    std::env::var(name).is_ok()
}

fn main() {
    pretty_env_logger::init();

    if !env_present("UPSILON_TESTSUITE_OFFLINE") {
        setup_local_clone();
    }
}
