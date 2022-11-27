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

pub fn __internal_new_with_ts() -> __InternalUUID {
    __InternalUUID(uuid::Uuid::now_v7())
}
pub fn __internal_new_without_ts() -> __InternalUUID {
    __InternalUUID(uuid::Uuid::new_v4())
}

impl __InternalUUID {
    pub fn ts(&self) -> SystemTime {
        let (s, ns) = self.0.get_timestamp().expect("Missing timestamp").to_unix();
        SystemTime::UNIX_EPOCH + Duration::from_secs(s) + Duration::from_nanos(ns as u64)
    }

    pub fn chrono_ts(&self) -> chrono::NaiveDateTime {
        let (s, ns) = self.0.get_timestamp().expect("Missing timestamp").to_unix();
        chrono::NaiveDateTime::from_timestamp_opt(s as i64, ns).expect("out of range")
    }
}

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
