#![feature(associated_type_defaults)]
#![feature(try_trait_v2)]

pub extern crate upsilon_models;

pub use async_trait::async_trait;

pub use upsilon_models::users::{User, UserId, Username};

pub trait CommonDataClientErrorExtractor {
    fn into_common_error(self) -> CommonDataClientError;
}

#[derive(Debug, thiserror::Error)]
pub enum CommonDataClientError {
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error>),
}

#[async_trait]
pub trait DataClient {
    type InnerConfiguration;
    type Error: std::error::Error + CommonDataClientErrorExtractor;
    type Result<T>: std::ops::Try<
        Output = T,
        Residual = Result<std::convert::Infallible, Self::Error>,
    > = Result<T, Self::Error>;

    type QueryImpl<'a>: DataClientQueryImpl<'a, Error = Self::Error>
    where
        Self: 'a;

    async fn init_client(config: Self::InnerConfiguration) -> Self::Result<Self>
    where
        Self: Sized;
    fn data_client_query_impl<'a>(&'a self) -> Self::QueryImpl<'a>;
}

#[async_trait]
pub trait DataClientMaster: Send + Sync {
    fn query_master<'a>(&'a self) -> Box<dyn DataClientQueryMaster + 'a>;

    async fn on_shutdown(&self) -> Result<(), Box<dyn std::error::Error>>;
}

macro_rules! query_impl_trait {
    ($($(async)? fn $name:ident ($($param_name:ident: $param_ty:ty),* $(,)?) $(-> $ret_ty:ty)? $(;)?)*) => {
        #[async_trait]
        pub trait DataClientQueryImpl<'a> {
            type Error: std::error::Error + CommonDataClientErrorExtractor;

            $(
                #[allow(unused_parens)]
                async fn $name (&self, $($param_name: $param_ty,)*) -> Result<($($ret_ty)?), Self::Error>;
            )*

            fn as_query_master(self) -> Box<dyn $crate::DataClientQueryMaster + 'a>;
        }

        #[async_trait]
        pub trait DataClientQueryMaster: Send + Sync {
            $(
                #[allow(unused_parens)]
                async fn $name (&self, $($param_name: $param_ty,)*) -> Result<($($ret_ty)?), CommonDataClientError>;
            )*
        }

        #[macro_export]
        macro_rules! query_master_impl_trait {
            ($query_master_name:ident, $query_impl:ident) => {
                pub struct $query_master_name<'a>($query_impl <'a>);

                #[async_trait]
                impl<'a> $crate::DataClientQueryMaster for $query_master_name <'a> {
                    $(
                        #[allow(unused_parens)]
                        async fn $name (&self, $($param_name: $crate:: $param_ty,)*) -> Result<($($ret_ty)?), $crate::CommonDataClientError> {
                            self.0.$name($($param_name,)*).await.map_err(|e| e.into_common_error())
                        }
                    )*
                }
            };
        }
    };
}

query_impl_trait!(
    async fn create_user(user: User);
    async fn query_user(user_id: UserId) -> User;
    async fn set_user_name(user_id: UserId, user_name: Username);
);

pub struct DataClientMasterHolder(Box<dyn DataClientMaster>);

impl DataClientMasterHolder {
    pub fn new<T: DataClient + DataClientMaster + 'static>(client: T) -> Self {
        Self(Box::new(client))
    }

    pub fn query_master<'a>(&'a self) -> Box<dyn DataClientQueryMaster + 'a> {
        self.0.query_master()
    }

    pub async fn on_shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.0.on_shutdown().await
    }
}
