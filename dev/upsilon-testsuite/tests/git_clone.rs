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

macro_rules! clone_test {
    (@can_clone: $(#[$attr:meta])* $name:ident: $remote_path_builder_fn:ident) => {
        #[upsilon_test]
        $(#[$attr])*
        async fn $name(cx: &mut TestCx) -> TestResult {
            make_global_mirror_from_host_repo(cx).await?;

            let _ = cx.clone("upsilon-clone", $remote_path_builder_fn, None).await?;

            Ok(())
        }
    };

    (@can_clone_git_bin: $(#[$attr:meta])* $name:ident: $remote_path_builder_fn:ident) => {
        #[upsilon_test]
        $(#[$attr])*
        async fn $name(cx: &mut TestCx) -> TestResult {
            make_global_mirror_from_host_repo(cx).await?;

            let _ = cx.clone_git_binary("upsilon-clone", $remote_path_builder_fn, Duration::from_secs(10)).await?;

            Ok(())
        }
    };

    (@can_clone_ssh: $(#[$attr:meta])* $name:ident: $remote_path_builder_fn:ident) => {
        #[upsilon_test]
        $(#[$attr])*
        #[git_ssh]
        async fn $name(cx: &mut TestCx) -> TestResult {
            make_global_mirror_from_host_repo(cx).await?;

            let username = "test";

            cx.create_user(username, "test", "test").await?;

            let kp = create_ssh_key()?;
            cx.add_ssh_key_to_user(&kp, username).await?;

            cx.clone("upsilon-clone", $remote_path_builder_fn, Credentials::SshKey(kp))
                .await?;

            Ok(())
        }
    };

    (@clone_twice_same_result: $(#[$attr:meta])* $name:ident: $remote_path_builder_fn:ident) => {
        #[upsilon_test]
        $(#[$attr])*
        async fn $name(cx: &mut TestCx) -> TestResult {
            make_global_mirror_from_host_repo(cx).await?;

            let (clone1, clone2) = cx
                .clone_repo_twice("upsilon-clone-1", "upsilon-clone-2", $remote_path_builder_fn, None)
                .await?;

            assert_same_trunk(&clone1, &clone2)?;

            Ok(())
        }
    };
}

clone_test! {@can_clone: can_clone_to_local_http: upsilon_global}
clone_test! {@can_clone: #[git_daemon] can_clone_to_local_git: upsilon_global_git_protocol}
clone_test! {@can_clone_ssh: can_clone_to_local_ssh: upsilon_global_ssh}
clone_test! {@can_clone_git_bin: can_clone_to_local_git_binary_http: upsilon_global}
clone_test! {@can_clone_git_bin: #[git_daemon] can_clone_to_local_git_binary_git: upsilon_global_git_protocol}
clone_test! {@clone_twice_same_result: clone_twice_same_result_http: upsilon_global}
clone_test! {@clone_twice_same_result: #[git_daemon] clone_twice_same_result_git: upsilon_global_git_protocol}
