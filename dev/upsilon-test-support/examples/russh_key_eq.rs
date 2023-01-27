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

fn main() {
    let k = russh_keys::key::KeyPair::generate_rsa(2048, russh_keys::key::SignatureHash::SHA2_512)
        .unwrap();

    let kpub = k.clone_public_key().unwrap();

    let mut k2 = k.clone();

    if let russh_keys::key::KeyPair::RSA { ref mut hash, .. } = k2 {
        *hash = russh_keys::key::SignatureHash::SHA2_256;
    }

    let kpub2 = k2.clone_public_key().unwrap();

    assert_ne!(kpub, kpub2); // should be eq, but are not because of the hash
}
