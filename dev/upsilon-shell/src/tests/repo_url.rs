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

use super::test_prelude::*;

fn default_hostinfo() -> UshHostInfo {
    UshHostInfo {
        git_ssh_enabled: false,
        ssh_port: 22,
        git_protocol_enabled: false,
        git_port: 9418,
        git_http_enabled: true,
        http_port: 80,
        https_enabled: false,
        hostname: "localhost".to_owned(),
    }
}

macro_rules! test_case {
    ($test_name:ident, $protocol:ident $(, patches: [$($patch_name:ident: $patch_value:expr),* $(,)?])?, $repo_path:literal, $result:literal $(,)?) => {
        #[test]
        fn $test_name() {
            let mut __hostinfo = default_hostinfo();
            $(
                $(
                    __hostinfo.$patch_name = $patch_value;
                )*
            )?
            assert_eq!(
                UshRepoAccessProtocol::$protocol
                    .build_url($repo_path, &__hostinfo),
                Ok(String::from($result))
            );
        }
    };
}

test_case! {
    http_default_port,
    Http,
    "upsilon",
    "http://localhost/upsilon",
}
test_case! {
    http_custom_port,
    Http,
    patches: [http_port: 8000],
    "upsilon",
    "http://localhost:8000/upsilon",
}
test_case! {
    ssh_default_port,
    Ssh,
    patches: [git_ssh_enabled: true],
    "upsilon",
    "git@localhost:upsilon",
}
test_case! {
    ssh_custom_port,
    Ssh,
    patches: [git_ssh_enabled: true, ssh_port: 8000],
    "upsilon",
    "ssh://git@localhost:8000/upsilon",
}
test_case! {
    https_default_port,
    Http,
    patches: [https_enabled: true, http_port: 443],
    "upsilon",
    "https://localhost/upsilon",
}
test_case! {
    https_custom_port,
    Http,
    patches: [https_enabled: true, http_port: 8000],
    "upsilon",
    "https://localhost:8000/upsilon",
}
test_case! {
    git_default_port,
    Git,
    patches: [git_protocol_enabled: true],
    "upsilon",
    "git://localhost/upsilon",
}
test_case! {
    git_custom_port,
    Git,
    patches: [git_protocol_enabled: true, git_port: 8000],
    "upsilon",
    "git://localhost:8000/upsilon",
}
test_case! {
    http_remote_host,
    Http,
    patches: [hostname: "upsilon.dnbln.dev".to_owned(), http_port: 443, https_enabled: true],
    "upsilon",
    "https://upsilon.dnbln.dev/upsilon",
}
test_case! {
    ssh_remote_host,
    Ssh,
    patches: [git_ssh_enabled: true, hostname: "upsilon.dnbln.dev".to_owned(), ssh_port: 22],
    "upsilon",
    "git@upsilon.dnbln.dev:upsilon",
}
test_case! {
    ssh_remote_host_with_port,
    Ssh,
    patches: [git_ssh_enabled: true, hostname: "upsilon.dnbln.dev".to_owned(), ssh_port: 1234],
    "upsilon",
    "ssh://git@upsilon.dnbln.dev:1234/upsilon",
}
