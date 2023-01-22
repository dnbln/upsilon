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

use std::path::PathBuf;
use std::time::Duration;

use anyhow::format_err;
use upsilon_test_support::prelude::*;
use upsilon_test_support::CxConfigVars;

async fn example_impl(cx: &mut TestCx) -> TestResult {
    make_global_mirror_from_host_repo(cx).await?;

    let username = "test";
    cx.create_user(username, "test", "test").await?;

    let kp = create_ssh_key()?;
    cx.add_ssh_key_to_user(&kp, username).await?;

    let clone_fut = cx.clone("upsilon-clone", upsilon_global_ssh, Credentials::SshKey(kp));

    let _ = tokio::time::timeout(Duration::from_secs(10), clone_fut)
        .await
        .map_err(|e| format_err!("clone timed out: {e}"))??;

    Ok(())
}

async fn main_impl() -> TestResult {
    let cfg = TestCxConfig::new(&CxConfigVars {
        workdir: PathBuf::from(std::env::var("UPSILON_TMPDIR").expect("UPSILON_TMPDIR not set")),
        test_name: "clone_to_local_ssh_example",
        source_file_path_hash: 0,
        works_offline: true,
        config_init: upsilon_basic_config_with_ssh,
    });

    let mut cx = TestCx::init(cfg).await?;

    let result = example_impl(&mut cx).await;

    cx.finish(result).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = main_impl().await {
        panic!("{}", e)
    }
}
