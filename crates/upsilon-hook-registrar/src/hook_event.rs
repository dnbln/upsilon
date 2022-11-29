use std::ops::Deref;

use upsilon_models::users::User;
use upsilon_models::repo::Repo;

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