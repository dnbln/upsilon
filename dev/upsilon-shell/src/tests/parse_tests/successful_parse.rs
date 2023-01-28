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

#[test]
fn cd_empty() {
    let command = parse_line("cd").unwrap();

    command.assert_cd(None);
}

#[test]
fn cd_with_path() {
    let command = parse_line("cd aaa").unwrap();

    command.assert_cd(Some(&UshPath::new("aaa")));
}

#[test]
fn ls_empty() {
    let command = parse_line("ls").unwrap();

    command.assert_ls(None);
}

#[test]
fn ls_with_path() {
    let command = parse_line("ls aaa").unwrap();

    command.assert_ls(Some(&UshPath::new("aaa")));
}

#[test]
fn pwd() {
    let command = parse_line("pwd").unwrap();

    command.assert_pwd();
}

#[test]
fn echo_empty() {
    let command = parse_line("echo").unwrap();

    command.assert_echo(&[]);
}

#[test]
fn echo_with_args() {
    let command = parse_line("echo aaa bbb ccc").unwrap();

    command.assert_echo(&["aaa", "bbb", "ccc"]);
}

#[test]
fn exit_no_args() {
    let command = parse_line("exit").unwrap();

    command.assert_exit(None);
}

#[test]
fn exit_with_args() {
    let command = parse_line("exit 1").unwrap();

    command.assert_exit(Some(1));
}

#[test]
fn login() {
    let command = parse_line("login username:aaa password:bbb").unwrap();

    command.assert_login("aaa", "bbb");
}

#[test]
fn create_user() {
    let command = parse_line("create-user username:aaa password:bbb email:ccc").unwrap();

    command.assert_create_user("aaa", "bbb", "ccc");
}

#[test]
fn create_repo() {
    let command = parse_line("create-repo name:aaa").unwrap();

    command.assert_create_repo("aaa");
}

#[test]
fn clone_http() {
    let command = parse_line("clone remote-path:upsilon").unwrap();

    command.assert_clone(
        "upsilon",
        &UshPath::new("upsilon"),
        UshRepoAccessProtocol::Http,
    );
}

#[test]
fn clone_ssh() {
    let command = parse_line("clone remote-path:upsilon --ssh").unwrap();

    command.assert_clone(
        "upsilon",
        &UshPath::new("upsilon"),
        UshRepoAccessProtocol::Ssh,
    );
}

#[test]
fn clone_git() {
    let command = parse_line("clone remote-path:upsilon --git").unwrap();

    command.assert_clone(
        "upsilon",
        &UshPath::new("upsilon"),
        UshRepoAccessProtocol::Git,
    );
}

#[test]
fn clone_to() {
    let command = parse_line("clone remote-path:upsilon to:aaa").unwrap();

    command.assert_clone("upsilon", &UshPath::new("aaa"), UshRepoAccessProtocol::Http);
}

#[test]
fn clone_to_ssh() {
    let command = parse_line("clone remote-path:upsilon to:aaa --ssh").unwrap();

    command.assert_clone("upsilon", &UshPath::new("aaa"), UshRepoAccessProtocol::Ssh);
}

#[test]
fn http_url() {
    let command = parse_line("http-url remote-path:upsilon").unwrap();

    command.assert_http_url("upsilon");
}

#[test]
fn git_url() {
    let command = parse_line("git-url remote-path:upsilon").unwrap();

    command.assert_git_url("upsilon");
}

#[test]
fn ssh_url() {
    let command = parse_line("ssh-url remote-path:upsilon").unwrap();

    command.assert_ssh_url("upsilon");
}

#[test]
fn url_default() {
    let command = parse_line("url remote-path:upsilon").unwrap();

    command.assert_url("upsilon", UshRepoAccessProtocol::Http);
}

#[test]
fn url_http() {
    let command = parse_line("url remote-path:upsilon --http").unwrap();

    command.assert_url("upsilon", UshRepoAccessProtocol::Http);
}

#[test]
fn url_ssh() {
    let command = parse_line("url remote-path:upsilon --ssh").unwrap();

    command.assert_url("upsilon", UshRepoAccessProtocol::Ssh);
}

#[test]
fn url_git() {
    let command = parse_line("url remote-path:upsilon --git").unwrap();

    command.assert_url("upsilon", UshRepoAccessProtocol::Git);
}
