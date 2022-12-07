/*
 *        Copyright (c) 2022 Dinu Blanovschi
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

pub extern crate upsilon_models;
pub extern crate upsilon_procx;

use std::sync::Arc;
pub use async_trait::async_trait;

pub trait CommonDataClientErrorExtractor {
    fn into_common_error(self) -> CommonDataClientError;
}

#[derive(Debug, thiserror::Error)]
pub enum CommonDataClientError {
    #[error("User not found")]
    UserNotFound,
    #[error("User already exists")]
    UserAlreadyExists,
    #[error("Repo already exists")]
    RepoAlreadyExists,
    #[error("Name conflict")]
    NameConflict,
    #[error("{0}")]
    Other(#[from] Box<dyn std::error::Error + Send + Sync>),
}

#[async_trait]
pub trait DataClient {
    type InnerConfiguration;
    type Error: std::error::Error + CommonDataClientErrorExtractor;

    type QueryImpl<'a>: DataClientQueryImpl<'a, Error = Self::Error>
    where
        Self: 'a;

    async fn init_client(config: Self::InnerConfiguration) -> Result<Self, Self::Error>
    where
        Self: Sized;
    fn data_client_query_impl<'a>(&'a self) -> Self::QueryImpl<'a>;
}

#[async_trait]
pub trait DataClientMaster: Send + Sync {
    fn query_master<'a>(&'a self) -> Box<dyn DataClientQueryMaster + 'a>;

    async fn on_shutdown(&self) -> Result<(), Box<dyn std::error::Error>>;
}

#[derive(Clone)]
pub struct DataClientMasterHolder(Arc<Box<dyn DataClientMaster>>);

impl DataClientMasterHolder {
    pub fn new<T: DataClient + DataClientMaster + 'static>(client: T) -> Self {
        Self(Arc::new(Box::new(client)))
    }

    pub fn query_master(&self) -> DataQueryMaster {
        DataQueryMaster(self.0.query_master())
    }

    pub async fn on_shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.0.on_shutdown().await
    }
}

pub struct DataQueryMaster<'a>(Box<dyn DataClientQueryMaster + 'a>);

#[macro_export]
macro_rules! expand_ret_ty_or_unit {
    ($ret_ty:ty) => {
        $ret_ty
    };
    () => {
        ()
    };
}

#[macro_export]
macro_rules! query_impl_self_ty {
    ('self_ref $($rest:tt)*) => {&'self_ref Self};
    ($($rest:tt)*) => {&'_ Self};
}

macro_rules! query_impl_trait {
    (@data_query_master_param_ty {} $ty:ty) => {$ty};
    // (@data_query_master_param_ty {into} $ty:ty) => {impl Into<$ty>};
    (@data_query_master_param_ty {into $($att:ident)*} $ty:ty) => {
        impl Into<query_impl_trait!(@data_query_master_param_ty {$($att)*} $ty)>
    };

    (@data_query_master_param_processor {} $value:expr) => {$value};
    // (@data_query_master_param_processor {into} $value:expr) => {$value.into()};
    (@data_query_master_param_processor {into $($att:ident)*} $value:expr) => {
        query_impl_trait!(@data_query_master_param_processor {$($att)*} $value)
            .into()
    };

    (
        $(
            $(async)? fn $name:ident $(<$($generics:tt),* $(,)?>)? (
                $(
                    $({$($param_att:ident)*})? $param_name:ident: $param_ty:ty
                ),*
                $(,)?
            ) $(-> $ret_ty:ty)?;
        )*
    ) => {
        #[async_trait]
        pub trait DataClientQueryImpl<'a> {
            type Error: std::error::Error + CommonDataClientErrorExtractor;

            $(
                async fn $name <$($($generics,)*)?> (
                    self: query_impl_self_ty!($($($generics,)*)?),
                    $($param_name: $param_ty,)*
                ) -> Result<
                    $crate::expand_ret_ty_or_unit!($($ret_ty)?),
                    Self::Error
                >;
            )*

            fn into_query_master(self) -> Box<dyn $crate::DataClientQueryMaster + 'a>;
        }

        #[async_trait]
        pub trait DataClientQueryMaster: Send + Sync {
            $(
                async fn $name $(<$($generics,)*>)? (
                    self: query_impl_self_ty!($($($generics,)*)?),
                    $($param_name: $param_ty,)*
                ) -> Result<
                    $crate::expand_ret_ty_or_unit!($($ret_ty)?),
                    CommonDataClientError
                >;
            )*
        }

        #[macro_export]
        macro_rules! query_master_impl_trait {
            ($query_master_name:ident, $query_impl:ident) => {
                pub struct $query_master_name<'a>($query_impl <'a>);

                $crate::upsilon_procx::private_context! {
                    use super::$query_master_name;
                    use $crate::upsilon_models;
                    use $crate::DataClientQueryImpl;
                    use $crate::async_trait;
                    use $crate::CommonDataClientErrorExtractor;

                    #[async_trait]
                    impl<'a> $crate::DataClientQueryMaster for $query_master_name <'a> {
                        $(
                            async fn $name $(<$($generics,)*>)? (
                                self: $crate::query_impl_self_ty!($($($generics,)*)?),
                                $($param_name: $param_ty,)*
                            ) -> Result<
                                $crate::expand_ret_ty_or_unit!($($ret_ty)?),
                                $crate::CommonDataClientError
                            > {
                                self.0.$name($($param_name,)*).await.map_err(|e| e.into_common_error())
                            }
                        )*
                    }
                }
            };
        }

        impl<'a> DataQueryMaster<'a> {
            $(
                pub async fn $name $(<$($generics,)*>)? (
                    self: query_impl_self_ty!($($($generics,)*)?),
                    $($param_name: query_impl_trait!(@data_query_master_param_ty {$($($param_att)*)?} $param_ty),)*
                ) -> Result<
                    $crate::expand_ret_ty_or_unit!($($ret_ty)?),
                    $crate::CommonDataClientError
                > {
                    self.0.$name($(
                        query_impl_trait!(@data_query_master_param_processor {$($($param_att)*)?} $param_name),
                    )*).await
                }
            )*
        }
    };
}

query_impl_trait! {
    // ===========================
    // ========= Users ===========
    // ===========================
    async fn create_user<'self_ref>(user: upsilon_models::users::User);
    async fn query_user<'self_ref>(
        {into} user_id: upsilon_models::users::UserId
    ) -> upsilon_models::users::User;
    async fn query_user_by_username_email<'self_ref>(
        username_email: &str,
    ) -> Option<upsilon_models::users::User>;
    async fn query_user_by_username<'self_ref>(
        {into} username: upsilon_models::users::UsernameRef<'self_ref>,
    ) -> Option<upsilon_models::users::User>;
    async fn set_user_name<'self_ref>(
        {into} user_id: upsilon_models::users::UserId,
        {into} user_name: upsilon_models::users::Username,
    );

    // ===========================
    // ======== Repos ============
    // ===========================
    async fn create_repo<'self_ref>(repo: upsilon_models::repo::Repo);
    async fn query_repo<'self_ref>(
        {into} repo_id: upsilon_models::repo::RepoId
    ) -> upsilon_models::repo::Repo;
    async fn query_repo_by_name<'self_ref>(
        {into} repo_name: upsilon_models::repo::RepoNameRef<'self_ref>,
        {into} repo_namespace: &upsilon_models::repo::RepoNamespace,
    ) -> Option<upsilon_models::repo::Repo>;
    async fn set_repo_name<'self_ref>(
        {into} repo_id: upsilon_models::repo::RepoId,
        {into} repo_name: upsilon_models::repo::RepoName,
    );
    async fn query_repo_user_perms<'self_ref>(
        {into} repo_id: upsilon_models::repo::RepoId,
        {into} user_id: upsilon_models::users::UserId,
    ) -> Option<upsilon_models::repo::RepoPermissions>;

    // ================================
    // ======== Organizations =========
    // ================================
    async fn create_organization<'self_ref>(
        org: upsilon_models::organization::Organization
    );
    async fn query_organization<'self_ref>(
        {into} org_id: upsilon_models::organization::OrganizationId,
    ) -> upsilon_models::organization::Organization;
    async fn query_organization_by_name<'self_ref>(
        {into} org_name: upsilon_models::organization::OrganizationNameRef<'self_ref>,
    ) -> Option<upsilon_models::organization::Organization>;
    async fn set_organization_name<'self_ref>(
        {into} org_id: upsilon_models::organization::OrganizationId,
        {into} org_name: upsilon_models::organization::OrganizationName,
    );
    async fn set_organization_display_name<'self_ref>(
        {into} org_id: upsilon_models::organization::OrganizationId,
        {into} org_display_name: Option<upsilon_models::organization::OrganizationDisplayName>,
    );

    async fn query_organization_member<'self_ref>(
        {into} org_id: upsilon_models::organization::OrganizationId,
        {into} user_id: upsilon_models::users::UserId,
    ) -> Option<upsilon_models::organization::OrganizationMember>;

    async fn query_organization_members<'self_ref>(
        {into} org_id: upsilon_models::organization::OrganizationId,
    ) -> Vec<upsilon_models::organization::OrganizationMember>;

    async fn query_user_organizations<'self_ref>(
        {into} user_id: upsilon_models::users::UserId,
    ) -> Vec<upsilon_models::organization::OrganizationMember>;

    // ===========================
    // ======== Teams ============
    // ===========================
    async fn create_team<'self_ref>(
        team: upsilon_models::organization::Team
    );
    async fn query_team<'self_ref>(
        {into} team_id: upsilon_models::organization::TeamId,
    ) -> upsilon_models::organization::Team;
    async fn query_organization_teams<'self_ref>(
        {into} org_id: upsilon_models::organization::OrganizationId,
    ) -> Vec<upsilon_models::organization::Team>;
    async fn query_team_by_name<'self_ref>(
        {into} org_id: upsilon_models::organization::OrganizationId,
        {into} team_name: upsilon_models::organization::TeamNameRef<'self_ref>,
    ) -> Option<upsilon_models::organization::Team>;
    async fn set_team_name<'self_ref>(
        {into} team_id: upsilon_models::organization::TeamId,
        {into} team_name: upsilon_models::organization::TeamName,
    );
    async fn set_team_display_name<'self_ref>(
        {into} team_id: upsilon_models::organization::TeamId,
        {into} team_display_name: Option<upsilon_models::organization::TeamDisplayName>,
    );

    async fn query_organization_and_team<'self_ref>(
        {into} team_id: upsilon_models::organization::TeamId,
    ) -> (
        upsilon_models::organization::Organization,
        upsilon_models::organization::Team,
    );

    async fn query_team_members<'self_ref>(
        {into} organization_id: upsilon_models::organization::OrganizationId,
        {into} team_id: upsilon_models::organization::TeamId,
    ) -> Vec<upsilon_models::organization::OrganizationMember>;
}
