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

use std::ops::Deref;

use upsilon_models::repo::Repo;
use upsilon_models::users::User;

#[derive(Copy, Clone)]
pub enum HookEvent<'a> {
    UserPreCreate(UserPreCreateEvent<'a>),
    UserPostCreate(UserPostCreateEvent<'a>),
    RepoPreCreate(RepoPreCreateEvent<'a>),
    RepoPostCreate(RepoPostCreateEvent<'a>),
    UserPreDelete(UserPreDeleteEvent<'a>),
    UserPostDelete(UserPostDeleteEvent<'a>),
}

macro_rules! single_arg_event {
    ($name:ident, $ty:ty) => {
        #[derive(Copy, Clone)]
        pub struct $name<'a>(&'a $ty);

        impl<'a> Deref for $name<'a> {
            type Target = $ty;

            fn deref(&self) -> &Self::Target {
                self.0
            }
        }
    };
}

single_arg_event!(UserPreCreateEvent, User);
single_arg_event!(UserPostCreateEvent, User);
single_arg_event!(RepoPreCreateEvent, Repo);
single_arg_event!(RepoPostCreateEvent, Repo);
single_arg_event!(UserPreDeleteEvent, User);
single_arg_event!(UserPostDeleteEvent, User);
