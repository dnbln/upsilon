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

use upsilon_test_support::prelude::*;

fn main_impl() -> TestResult {
    let k = create_ssh_key()?;
    let pubk = k.clone_public_key()?;

    let encoded = TestCx::encode_ssh_key(&pubk)?;

    println!("Encoded:");
    println!("{encoded}");

    let decode_pubk = russh_keys::parse_public_key_base64(&encoded)?;

    assert_eq!(pubk, decode_pubk);

    Ok(())
}

fn main() {
    if let Err(e) = main_impl() {
        panic!("{}", e);
    }
}
