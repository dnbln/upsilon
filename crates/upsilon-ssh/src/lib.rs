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

pub extern crate async_trait;

use async_trait::async_trait;
use upsilon_models::users::UserId;

#[derive(thiserror::Error, Debug)]
pub enum CommonSSHError {
    #[error("Other: {0}")]
    Other(Box<dyn std::error::Error>),
}

pub trait SSHServerConfig {}

#[async_trait]
pub trait SSHServerInitializer {
    type Config: SSHServerConfig;
    type Error: Into<CommonSSHError> + Send + Sync;
    type Server: SSHServer;

    fn new(config: Self::Config) -> Self;
    async fn init(
        self,
        dcmh: upsilon_data::DataClientMasterHolder,
    ) -> Result<Self::Server, Self::Error>;
}

#[async_trait]
pub trait SSHServer {
    type Config: SSHServerConfig;
    type Error: Into<CommonSSHError> + Send + Sync;
    type Initializer: SSHServerInitializer<Server = Self, Config = Self::Config>;

    async fn stop(&self) -> Result<(), Self::Error>;

    fn into_wrapper(self) -> Box<dyn SSHServerWrapper + Send + Sync>;
}

pub struct SSHKey(String);

impl SSHKey {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[async_trait]
pub trait SSHServerWrapper {
    async fn stop(&self) -> Result<(), CommonSSHError>;
}

#[macro_export]
macro_rules! impl_wrapper {
    ($server:ident, $wrapper:ident) => {
        pub struct $wrapper {
            server: $server,
        }

        impl $wrapper {
            pub fn new(server: $server) -> Self {
                Self { server }
            }
        }

        #[async_trait]
        impl SSHServerWrapper for $wrapper {
            async fn stop(&self) -> Result<(), CommonSSHError> {
                self.server.stop().await.map_err(Into::into)
            }
        }
    };
}

pub struct SSHServerHolder(Box<dyn SSHServerWrapper + Send + Sync>);

impl SSHServerHolder {
    pub fn new(server: Box<dyn SSHServerWrapper + Send + Sync>) -> Self {
        Self(server)
    }

    pub async fn stop(&self) -> Result<(), CommonSSHError> {
        self.0.stop().await
    }
}
