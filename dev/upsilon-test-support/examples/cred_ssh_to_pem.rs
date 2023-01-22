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

use russh_keys::key::{KeyPair, SignatureHash};
use upsilon_test_support::TestCx;

fn main() {
    let kp = KeyPair::generate_rsa(2048, SignatureHash::SHA2_512).unwrap();
    let result = TestCx::cred_ssh_to_pem(&kp).unwrap();
    println!("{}", result);
}
