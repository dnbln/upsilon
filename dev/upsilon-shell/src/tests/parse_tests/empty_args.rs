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

macro_rules! test_empty_arg {
    ($test_name:ident, $line:expr, $empty_arg:literal $(,)?) => {
        test_empty_arg!($test_name, $line, $empty_arg, None);
    };

    ($test_name:ident, $line:expr, $empty_arg:literal, $arg_hint:ident $(,)?) => {
        #[test]
        fn $test_name() {
            parse_line($line)
                .unwrap_err()
                .assert_empty_arg($empty_arg, ArgHint::$arg_hint);
        }
    };
}

macro_rules! test_empty_arg_append {
    ($test_name:ident, $start:literal, $empty_arg:literal $(, $hint:ident)? $(,)?) => {
        test_empty_arg! {
            $test_name,
            concat!($start, " ", $empty_arg),
            $empty_arg,
            $($hint,)?
        }
    };
}

test_empty_arg_append! {
    login_username_empty,
    "login", "username:",
    Username,
}

test_empty_arg_append! {
    login_password_empty,
    "login", "password:",
}

test_empty_arg_append! {
    create_user_username_empty,
    "create-user", "username:",
}

test_empty_arg_append! {
    create_user_password_empty,
    "create-user", "password:",
}

test_empty_arg_append! {
    create_user_email_empty,
    "create-user", "email:",
}

test_empty_arg_append! {
    create_repo_name_empty,
    "create-repo", "name:",
}

test_empty_arg_append! {
    clone_remote_path_empty,
    "clone", "remote-path:",
}

test_empty_arg_append! {
    clone_to_empty,
    "clone", "to:",
}

test_empty_arg_append! {
    git_url_remote_path_empty,
    "git-url", "remote-path:",
}

test_empty_arg_append! {
    http_url_remote_path_empty,
    "http-url", "remote-path:",
}

test_empty_arg_append! {
    ssh_url_remote_path_empty,
    "ssh-url", "remote-path:",
}

test_empty_arg_append! {
    url_remote_path_empty,
    "url", "remote-path:",
}
