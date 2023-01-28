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

// Those tests only make sense for the shell builtin commands,
// the others use the `ArgDeclList::parse_from` function which
// eats all the tokens and returns an error if there are any
// that it couldn't make sense of.
// The test cases for those are in the `parse_tests::unknown_args`.
macro_rules! extra_args_test {
    ($test_name:ident, $line:literal, $unexpected:literal $(,)?) => {
        #[test]
        fn $test_name() {
            let err = parse_line($line).unwrap_err();

            err.assert_expected_end_of_input($unexpected);
        }
    };
}

extra_args_test! {
    cd_with_one_extra_arg,
    "cd a b",
    "b",
}

extra_args_test! {
    cd_with_two_extra_args,
    "cd a b c",
    "b",
}

extra_args_test! {
    ls_with_one_extra_arg,
    "ls a b",
    "b",
}

extra_args_test! {
    ls_with_two_extra_args,
    "ls a b c",
    "b",
}

extra_args_test! {
    pwd_with_arg,
    "pwd a",
    "a",
}

extra_args_test! {
    pwd_with_two_args,
    "pwd a b",
    "a",
}

extra_args_test! {
    exit_with_two_args,
    "exit 1 b",
    "b",
}

extra_args_test! {
    exit_with_three_args,
    "exit 1 b c",
    "b",
}