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

fn main() {
    println!(
        r#"
Hello!

If you meant to run the dev version of upsilon,
you should use `cargo xtask run-dev` instead
(or `cargo x r` for short).

Or if you meant to pack for release, use
`cargo xtask pack-release`.
"#
    );

    std::process::exit(1);
}
