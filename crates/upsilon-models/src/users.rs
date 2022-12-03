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

pub mod emails;
pub mod password;

use crate::assets::ImageAssetId;
use crate::namespace::{PlainNamespaceFragment, PlainNamespaceFragmentRef};
use crate::users::emails::UserEmails;
use crate::users::password::HashedPassword;

upsilon_id::id_ty!(
    #[uuid]
    #[timestamped]
    pub struct UserId;
);

crate::utils::str_newtype!(Username, UsernameRef);
crate::utils::str_newtype! {
    @conversions #[all]
    Username, UsernameRef,
    PlainNamespaceFragment, PlainNamespaceFragmentRef
}
crate::utils::str_newtype!(UserDisplayName, UserDisplayNameRef);

#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: Username,
    pub password: HashedPassword,
    pub display_name: Option<UserDisplayName>,

    pub emails: UserEmails,
    pub avatar: Option<ImageAssetId>,
}
