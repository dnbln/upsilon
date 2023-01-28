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

use super::super::test_prelude::*;

macro_rules! unknown_flag_test {
    (
        $test_name:ident,
        $line:expr,
        $unexpected:expr,
        [$($possible_values:literal),* $(,)?] $(,)?
    ) => {
        #[test]
        fn $test_name() {
            parse_line($line)
                .unwrap_err()
                .assert_unknown_flag($unexpected, [$($possible_values,)*]);
        }
    };
}

macro_rules! unknown_flag_partial_tests {
    (
        $command:literal,
        [$($test_name:ident: $flag:literal),* $(,)?],
        $possible_values:tt $(,)?
    ) => {
        $(
            unknown_flag_test! {
                $test_name,
                concat!($command, " ", $flag),
                $flag,
                $possible_values,
            }
        )*
    };
}

unknown_flag_partial_tests! {
    "clone",
    [
        clone_partial_0: "--",
        clone_git_partial_1: "--g",
        clone_git_partial_2: "--gi",
        clone_http_partial_1: "--h",
        clone_http_partial_2: "--ht",
        clone_http_partial_3: "--htt",
        clone_ssh_partial_1: "--s",
        clone_ssh_partial_2: "--ss",
        clone_unknown_flag_1: "--unknown",
        clone_unknown_flag_2: "--unknown-1",
    ],
    ["http", "git", "ssh"],
}

unknown_flag_partial_tests! {
    "url",
    [
        url_partial_0: "--",
        url_git_partial_1: "--g",
        url_git_partial_2: "--gi",
        url_http_partial_1: "--h",
        url_http_partial_2: "--ht",
        url_http_partial_3: "--htt",
        url_ssh_partial_1: "--s",
        url_ssh_partial_2: "--ss",
        url_unknown_flag_1: "--unknown",
        url_unknown_flag_2: "--unknown-1",
    ],
    ["http", "git", "ssh"],
}
