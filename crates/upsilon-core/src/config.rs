use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct UsersRegisterConfig {
    pub enabled: bool,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Deserialize)]
#[serde(tag = "type")]
pub enum PasswordHashAlgorithmDescriptor {
    #[serde(rename = "argon2")]
    Argon2 {
        #[serde(default = "default_passes")]
        passes: u32,
        #[serde(rename = "mem-cost", default = "default_mem_cost")]
        mem_cost: u32,
    },
    #[serde(rename = "bcrypt")]
    Bcrypt {
        #[serde(default = "default_bcrypt_cost")]
        cost: u32,
    },
}

const fn default_passes() -> u32 {
    6
}

const fn default_mem_cost() -> u32 {
    4096
}

const fn default_bcrypt_cost() -> u32 {
    11
}

#[derive(Deserialize, Debug, Clone)]
pub struct UsersAuthConfig {
    pub password: PasswordHashAlgorithmDescriptor,
}

#[derive(Deserialize, Debug, Clone)]
pub struct UsersConfig {
    pub register: UsersRegisterConfig,
    pub auth: UsersAuthConfig,
}