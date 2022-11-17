#![feature(associated_type_defaults)]
#![feature(try_trait_v2)]

pub extern crate upsilon_models;

pub use async_trait::async_trait;

pub use upsilon_models::users::{User, UserId, Username};

pub trait CommonDataClientErrorExtractor {
    fn into_common_error(self) -> CommonDataClientError;
}

pub enum CommonDataClientError {
    Other(Box<dyn std::error::Error>),
}

#[async_trait]
pub trait DataClient {
    type InnerConfiguration: for<'d> serde::Deserialize<'d>;
    type Error: std::error::Error + CommonDataClientErrorExtractor;
    type Result<T>: std::ops::Try<
        Output = T,
        Residual = Result<std::convert::Infallible, Self::Error>,
    > = Result<T, Self::Error>;

    type QueryImpl<'a>: DataClientQueryImpl<'a, Error = Self::Error> where Self: 'a;

    async fn init_client(config: &Self::InnerConfiguration) -> Self::Result<Self>
    where
        Self: Sized;
    fn data_client_query_impl<'a>(&'a self) -> Self::QueryImpl<'a>;
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

            fn as_queryer<'q>(&'q self) -> Box<dyn $crate::DataClientQueryer + 'q>;
            fn as_query_master<'q>(&'q self) -> Box<dyn $crate::DataClientQueryMaster + 'q>;
        }

        #[async_trait]
        pub trait DataClientQueryer {
            $(
                #[allow(unused_parens)]
                async fn $name (&self, $($param_name: $param_ty,)*) -> Result<($($ret_ty)?), Box<dyn std::error::Error>>;
            )*
        }

        #[async_trait]
        pub trait DataClientQueryMaster {
            $(
                #[allow(unused_parens)]
                async fn $name (&self, $($param_name: $param_ty,)*) -> Result<($($ret_ty)?), CommonDataClientError>;
            )*
        }

        #[macro_export]
        macro_rules! queryer_and_master_impl_trait {
            ($queryer_name:ident, $query_master_name:ident, $query_impl:ident) => {
                pub struct $queryer_name<'a>(&'a $query_impl <'a>);

                #[async_trait]
                impl<'a> $crate::DataClientQueryer for $queryer_name <'a> {
                    $(
                        #[allow(unused_parens)]
                        async fn $name (&self, $($param_name: $crate:: $param_ty,)*) -> Result<($($ret_ty)?), Box<dyn std::error::Error>> {
                            self.0.$name($($param_name,)*).await
                                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)
                        }
                    )*
                }

                pub struct $query_master_name<'a>(&'a $query_impl <'a>);

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
