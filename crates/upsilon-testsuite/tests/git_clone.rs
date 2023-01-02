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

use std::time::Duration;

use upsilon_test_support::prelude::*;

#[upsilon_test]
async fn http_can_clone_to_local(cx: &mut TestCx) -> TestResult {
    make_global_mirror_from_host_repo(cx).await?;

    let _ = cx.clone("clone-upsilon", "upsilon").await?;

    Ok(())
}

#[upsilon_test]
async fn clone_twice_same_result(cx: &mut TestCx) -> TestResult {
    make_global_mirror_from_host_repo(cx).await?;

    let (_, clone1) = cx.clone("clone-upsilon-1", "upsilon").await?;
    let (_, clone2) = cx.clone("clone-upsilon-2", "upsilon").await?;

    assert_same_trunk(&clone1, &clone2)?;

    Ok(())
}

#[upsilon_test]
async fn clone_over_git_protocol(
    #[cfg_setup(upsilon_basic_config_with_git_daemon)] cx: &mut TestCx,
) -> TestResult {
    make_global_mirror_from_host_repo(cx).await?;

    let _ = cx.clone_over_git_protocol("upsilon", "upsilon").await?;

    Ok(())
}

#[upsilon_test]
async fn clone_twice_same_result_git_protocol(
    #[cfg_setup(upsilon_basic_config_with_git_daemon)] cx: &mut TestCx,
) -> TestResult {
    make_global_mirror_from_host_repo(cx).await?;

    let (_, clone1) = cx
        .clone_over_git_protocol("clone-upsilon-1", "upsilon")
        .await?;
    let (_, clone2) = cx
        .clone_over_git_protocol("clone-upsilon-2", "upsilon")
        .await?;

    assert_same_trunk(&clone1, &clone2)?;

    Ok(())
}

#[upsilon_test]
async fn clone_with_git_binary(cx: &mut TestCx) -> TestResult {
    make_global_mirror_from_host_repo(cx).await?;

    let tmp_dir = cx.tempdir("upsilon-clone").await?;
    let path = cx.http_repo_url("upsilon");

    let result = cx
        .run_command(
            "git",
            |cmd| cmd.arg("clone").arg(path).arg(&tmp_dir),
            Duration::from_secs(10),
        )
        .await?;

    if result.success() {
        Ok(())
    } else {
        bail!(
            "git clone failed with exit code {}",
            result.code().unwrap_or(-1)
        )
    }
}

#[upsilon_test]
async fn clone_with_git_binary_over_git_protocol(
    #[cfg_setup(upsilon_basic_config_with_git_daemon)] cx: &mut TestCx,
) -> TestResult {
    make_global_mirror_from_host_repo(cx).await?;

    let tmp_dir = cx.tempdir("upsilon-clone").await?;
    let path = cx.git_repo_url("upsilon");

    let result = cx
        .run_command(
            "git",
            |cmd| cmd.arg("clone").arg(path).arg(&tmp_dir),
            Duration::from_secs(10),
        )
        .await?;

    if result.success() {
        Ok(())
    } else {
        bail!(
            "git clone failed with exit code {}",
            result.code().unwrap_or(-1)
        )
    }
}
