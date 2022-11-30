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

use crate::email::Email;
use crate::namespace::{PlainNamespaceFragment, PlainNamespaceFragmentRef};
use crate::users::{UserId, Username, UsernameRef};
upsilon_id::id_ty! {
    #[uuid]
    #[timestamped]
    pub struct OrganizationId;
}

upsilon_id::id_ty! {
    #[uuid]
    #[timestamped]
    pub struct TeamId;
}

crate::utils::str_newtype!(OrganizationName, OrganizationNameRef);
crate::utils::str_newtype! {
    @conversions #[all]
    OrganizationName, OrganizationNameRef,
    PlainNamespaceFragment, PlainNamespaceFragmentRef
}
crate::utils::str_newtype! {
    @eq #[all]
    OrganizationName, OrganizationNameRef,
    Username, UsernameRef
}

crate::utils::str_newtype!(OrganizationDisplayName, OrganizationDisplayNameRef);
crate::utils::str_newtype!(TeamName, TeamNameRef);
crate::utils::str_newtype! {
    @conversions #[all]
    TeamName, TeamNameRef,
    PlainNamespaceFragment, PlainNamespaceFragmentRef
}
crate::utils::str_newtype!(TeamDisplayName, TeamDisplayNameRef);

#[derive(Debug, Clone)]
pub struct Organization {
    pub id: OrganizationId,
    pub owner: UserId,
    pub name: OrganizationName,
    pub display_name: Option<OrganizationDisplayName>,
    pub email: Option<Email>,
}

impl Organization {
    pub fn new(owner: UserId, name: OrganizationName) -> Organization {
        Organization {
            id: OrganizationId::new(),
            owner,
            name,
            display_name: None,
            email: None,
        }
    }

    pub fn set_display_name(&mut self, display_name: OrganizationDisplayName) {
        self.display_name = Some(display_name);
    }

    pub fn set_email(&mut self, email: Email) {
        self.email = Some(email);
    }
}

#[derive(Debug, Clone)]
pub struct Team {
    pub id: TeamId,
    pub organization_id: OrganizationId,
    pub name: TeamName,
    pub display_name: Option<TeamDisplayName>,
}

#[derive(Debug, Clone)]
pub struct OrganizationMember {
    pub organization_id: OrganizationId,
    pub user_id: UserId,
    pub teams: Vec<TeamId>,
}
