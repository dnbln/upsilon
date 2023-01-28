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

macro_rules! test_unknown_arg {
    ($test_name:ident, $line:literal, $arg_name:literal $(,)?) => {
        #[test]
        fn $test_name() {
            parse_line($line).unwrap_err().assert_unknown_arg($arg_name);
        }
    };
}

macro_rules! test_expected_arg {
    ($test_name:ident, $line:literal, $unexpected:literal, $($possible_values:literal),* $(,)?) => {
        #[test]
        fn $test_name() {
            parse_line($line).unwrap_err().assert_expected_arg($unexpected, &[$($possible_values),*]);
        }
    };
}

test_unknown_arg! {
    login_unknown_arg,
    "login usernaame:aaa",
    "usernaame",
}

test_unknown_arg! {
    login_unknown_arg_2,
    "login username:aaa passwoord:bbb",
    "passwoord",
}

test_unknown_arg! {
    create_user,
    "create-user usernaame:aaa password:bbb",
    "usernaame",
}

test_unknown_arg! {
    clone,
    "clone repo:aaa",
    "repo",
}

test_unknown_arg! {
    clone_2,
    "clone remote-path:aaa branch:bbb",
    "branch",
}

test_expected_arg! {
    clone_expected,
    "clone remot",
    "remot",
    "remote-path",
    "to",
}

test_expected_arg! {
    clone_expected_2,
    "clone remote-pat",
    "remote-pat",
    "remote-path",
    "to",
}

test_expected_arg! {
    clone_expected_3,
    "clone remote-path",
    "remote-path",
    "remote-path",
    "to",
}
