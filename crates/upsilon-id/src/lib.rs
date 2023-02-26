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

pub extern crate chrono;

use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[serde(transparent)]
pub struct __InternalUUID(uuid::Uuid);

impl std::fmt::Debug for __InternalUUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.as_hyphenated().fmt(f)
    }
}

impl std::fmt::Display for __InternalUUID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.as_hyphenated().fmt(f)
    }
}

impl std::str::FromStr for __InternalUUID {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse().map(Self)
    }
}

pub fn __internal_new_with_ts() -> __InternalUUID {
    __InternalUUID(uuid::Uuid::now_v7())
}
pub fn __internal_new_without_ts() -> __InternalUUID {
    __InternalUUID(uuid::Uuid::new_v4())
}

impl __InternalUUID {
    pub fn ts(&self) -> SystemTime {
        let (s, ns) = self.0.get_timestamp().expect("Missing timestamp").to_unix();
        SystemTime::UNIX_EPOCH + Duration::from_secs(s) + Duration::from_nanos(u64::from(ns))
    }

    pub fn chrono_ts(&self) -> chrono::NaiveDateTime {
        let (s, ns) = self.0.get_timestamp().expect("Missing timestamp").to_unix();
        chrono::NaiveDateTime::from_timestamp_opt(i64::try_from(s).unwrap(), ns)
            .expect("out of range")
    }
}

pub type FromStrError = uuid::Error;

#[macro_export]
macro_rules! id_ty {
    (
        #[uuid]
        @decl_and_commons
        $(#[$att:meta])*
        $vis:vis struct $name:ident;
    ) => {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
        $(#[$att])*
        #[serde(transparent)]
        $vis struct $name($crate::__InternalUUID);


        impl From<$crate::__InternalUUID> for $name {
            fn from(id: $crate::__InternalUUID) -> Self {
                Self(id)
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::str::FromStr for $name {
            type Err = <$crate::__InternalUUID as std::str::FromStr>::Err;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                s.parse().map(Self)
            }
        }

        #[juniper::graphql_scalar]
        impl<S> GraphQLScalar for $name
            where
                S: juniper::ScalarValue,
        {
            fn resolve(&self) -> Value {
                juniper::Value::scalar(self.0.to_string())
            }

            fn from_input_value(value: &juniper::InputValue) -> Option<Self> {
                value
                    .as_string_value()
                    .and_then(|s| s.parse().ok())
                    .map(Self)
            }

            fn from_str(value: juniper::ScalarToken) -> juniper::ParseScalarResult<S> {
                <String as juniper::ParseScalarValue<S>>::from_str(value)
            }
        }
    };

    (
        #[uuid]
        #[timestamped]
        $(#[$att:meta])*
        $vis:vis struct $name:ident;
    ) => {
        $crate::id_ty!(
            #[uuid]
            @decl_and_commons
            $(#[$att])*
            $vis struct $name;
        );

        impl $name {
            pub fn new() -> Self {
                Self($crate::__internal_new_with_ts())
            }
        }

        impl $name {
            pub fn ts(&self) -> std::time::SystemTime {
                self.0.ts()
            }

            pub fn chrono_ts(&self) -> $crate::chrono::NaiveDateTime {
                self.0.chrono_ts()
            }
        }
    };

    (
        #[uuid]
        $(#[$att:meta])*
        $vis:vis struct $name:ident;
    ) => {
        $crate::id_ty!(
            #[uuid]
            @decl_and_commons
            $(#[$att])*
            $vis struct $name;
        );

        impl $name {
            pub fn new() -> Self {
                Self($crate::__internal_new_without_ts())
            }
        }
    };

    (
        #[seq]
        @decl_and_commons
        $(#[$att:meta])*
        $vis:vis struct $name:ident;
    ) => {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
        $(#[$att])*
        #[serde(transparent)]
        $vis struct $name(usize);

        impl From<usize> for $name {
            fn from(id: usize) -> Self {
                Self(id)
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };

    (
        #[seq]
        $(#[$att:meta])*
        $vis:vis struct $name:ident;
    ) => {
        $crate::id_ty!(
            #[seq]
            @decl_and_commons
            $(#[$att])*
            $vis struct $name;
        );
    }
}
