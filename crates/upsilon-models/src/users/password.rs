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

use upsilon_stdx::Also;
crate::utils::str_newtype!(PlainPassword);
crate::utils::str_newtype!(HashedPassword);

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum PasswordHashAlgorithmDescriptor {
    Argon2 { passes: u32, mem_cost: u32 },
    Bcrypt { cost: u32 },
}

impl From<upsilon_core::config::PasswordHashAlgorithmDescriptor>
    for PasswordHashAlgorithmDescriptor
{
    fn from(d: upsilon_core::config::PasswordHashAlgorithmDescriptor) -> Self {
        match d {
            upsilon_core::config::PasswordHashAlgorithmDescriptor::Argon2 { passes, mem_cost } => {
                Self::Argon2 { passes, mem_cost }
            }
            upsilon_core::config::PasswordHashAlgorithmDescriptor::Bcrypt { cost } => {
                Self::Bcrypt { cost }
            }
        }
    }
}

fn bcrypt_salt(salt: &[u8]) -> [u8; 16] {
    let mut result = [0u8; 16];
    let actual_salt_size = usize::min(salt.len(), 16);

    result[..actual_salt_size].copy_from_slice(&salt[..actual_salt_size]);

    result
}

impl PasswordHashAlgorithmDescriptor {
    pub fn hash_password(self, password: &PlainPassword, salt: &[u8]) -> HashedPassword {
        let hashed = match self {
            PasswordHashAlgorithmDescriptor::Argon2 { passes, mem_cost } => {
                let argon_config = argon2::Config::default().also(|cfg| {
                    cfg.variant = argon2::Variant::Argon2id;
                    cfg.time_cost = passes;
                    cfg.mem_cost = mem_cost;
                });
                argon2::hash_encoded(password.0.as_bytes(), salt, &argon_config)
                    .expect("argon2 hash failed")
            }
            PasswordHashAlgorithmDescriptor::Bcrypt { cost } => {
                bcrypt::hash_with_salt(password.0.as_bytes(), cost, bcrypt_salt(salt))
                    .expect("bcrypt hash failed")
                    .format_for_version(bcrypt::Version::TwoB)
            }
        };

        HashedPassword::from(hashed)
    }

    pub fn verify_password(self, password: &PlainPassword, hash: &HashedPassword) -> bool {
        match self {
            PasswordHashAlgorithmDescriptor::Argon2 { .. } => {
                argon2::verify_encoded(&hash.0, password.0.as_bytes())
                    .expect("argon2 verify failed")
            }
            PasswordHashAlgorithmDescriptor::Bcrypt { .. } => {
                bcrypt::verify(&password.0, &hash.0).expect("bcrypt verify failed")
            }
        }
    }
}
