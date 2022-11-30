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

macro_rules! str_newtype {
    (#[no_as_str] $name:ident $(@derives [$($derive:path),* $(,)?])?) => {
        #[derive(
            serde::Serialize, serde::Deserialize, Clone, Eq, PartialEq, Hash, $($($derive,)*)?
        )]
        #[serde(transparent)]
        pub struct $name(pub(crate) String);

        impl From<String> for $name {
            fn from(s: String) -> Self {
                Self(s)
            }
        }

        impl<'a> From<&'a str> for $name {
            fn from(s: &'a str) -> Self {
                Self(s.to_owned())
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

        impl std::cmp::PartialEq<str> for $name {
            fn eq(&self, other: &str) -> bool {
                self.0.as_str() == other
            }
        }

        impl std::cmp::PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.0.as_str() == *other
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
                    .map(|s| s.to_string())
                    .map(Self)
            }

            fn from_str(value: juniper::ScalarToken) -> juniper::ParseScalarResult<S> {
                <String as juniper::ParseScalarValue<S>>::from_str(value)
            }
        }
    };
    ($name:ident $(@derives [$($derive:path),* $(,)?])?) => {
        crate::utils::str_newtype!(#[no_as_str] $name $(@derives [$($derive),*])?);

        impl $name {
            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }
    };

    (@ref #[no_as_str] $name:ident $(@derives [$($derive:path),* $(,)?])?) => {
        #[derive(
            serde::Serialize, serde::Deserialize, Copy, Clone, Eq, PartialEq, Hash, $($($derive,)*)?
        )]
        #[serde(transparent)]
        pub struct $name <'a>(pub(crate) &'a str);

        impl<'a> From<&'a str> for $name <'a> {
            fn from(s: &'a str) -> Self {
                Self(s)
            }
        }

        impl<'a> std::fmt::Debug for $name <'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl<'a> std::fmt::Display for $name <'a> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        impl<'a> std::cmp::PartialEq<str> for $name <'a> {
            fn eq(&self, other: &str) -> bool {
                self.0 == other
            }
        }

        impl<'a> std::cmp::PartialEq<&str> for $name <'a> {
            fn eq(&self, other: &&str) -> bool {
                self.0 == *other
            }
        }
    };

    (@ref $name:ident $(@derives [$($derive:path),* $(,)?])?) => {
        crate::utils::str_newtype!(@ref #[no_as_str] $name $(@derives [$($derive),*])?);

        impl<'a> $name <'a> {
            pub fn as_str(&'a self) -> &'a str {
                self.0
            }
        }
    };

    ($name:ident, $name_ref:ident $(@derives [$($derive:path),* $(,)?])?) => {
        crate::utils::str_newtype!($name $(@derives [$($derive,)*])?);
        crate::utils::str_newtype!(@ref $name_ref $(@derives [$($derive,)*])?);

        impl $name {
            pub fn as_ref(&self) -> $name_ref {
                $name_ref::from(self)
            }
        }

        impl<'a> From<&'a $name> for $name_ref<'a> {
            fn from(s: &'a $name) -> Self {
                Self(&s.0)
            }
        }

        impl<'a> From<$name_ref<'a>> for $name {
            fn from(s: $name_ref<'a>) -> Self {
                Self(s.0.to_string())
            }
        }

        impl<'a> std::cmp::PartialEq<$name_ref<'a>> for $name {
            fn eq(&self, other: &$name_ref<'a>) -> bool {
                self.0.as_str() == other.0
            }
        }

        impl<'a> std::cmp::PartialEq<$name> for $name_ref<'a> {
            fn eq(&self, other: &$name) -> bool {
                self.0 == other.0.as_str()
            }
        }
    };

    (#[no_as_str] $name:ident, $name_ref:ident $(@derives [$($derive:path),* $(,)?])?) => {
        crate::utils::str_newtype!(#[no_as_str] $name $(@derives [$($derive,)*])?);
        crate::utils::str_newtype!(@ref #[no_as_str] $name_ref $(@derives [$($derive,)*])?);

        impl $name {
            pub fn as_ref(&self) -> $name_ref {
                $name_ref::from(self)
            }
        }

        impl<'a> From<&'a $name> for $name_ref<'a> {
            fn from(s: &'a $name) -> Self {
                Self(&s.0)
            }
        }

        impl<'a> From<$name_ref<'a>> for $name {
            fn from(s: $name_ref<'a>) -> Self {
                Self(s.0.to_string())
            }
        }

        impl<'a> std::cmp::PartialEq<$name_ref<'a>> for $name {
            fn eq(&self, other: &$name_ref<'a>) -> bool {
                self.0.as_str() == other.0
            }
        }

        impl<'a> std::cmp::PartialEq<$name> for $name_ref<'a> {
            fn eq(&self, other: &$name) -> bool {
                self.0 == other.0.as_str()
            }
        }
    };

    (@conversions #[owned to owned] $name1:ident, $name2:ident) => {
        impl From<$name1> for $name2 {
            fn from(s: $name1) -> Self {
                Self::from(s.0)
            }
        }

        impl From<$name2> for $name1 {
            fn from(s: $name2) -> Self {
                Self::from(s.0)
            }
        }
    };

    (@conversions #[ref to ref] $name1:ident, $name2:ident) => {
        impl<'a> From<$name1<'a>> for $name2<'a> {
            fn from(s: $name1<'a>) -> Self {
                Self(&s.0)
            }
        }

        impl<'a> From<$name2<'a>> for $name1<'a> {
            fn from(s: $name2<'a>) -> Self {
                Self(&s.0)
            }
        }
    };

    (@conversions #[owned to ref] $name1:ident, $name2:ident) => {
        impl<'a> From<&'a $name1> for $name2<'a> {
            fn from(s: &'a $name1) -> Self {
                Self(&s.0)
            }
        }

        impl<'a> From<$name2<'a>> for $name1 {
            fn from(s: $name2<'a>) -> Self {
                Self(s.0.to_string())
            }
        }
    };

    (@conversions #[all] $name1:ident, $name1_ref:ident, $name2:ident, $name2_ref:ident) => {
        crate::utils::str_newtype!(@conversions #[owned to owned] $name1, $name2);
        crate::utils::str_newtype!(@conversions #[ref to ref] $name1_ref, $name2_ref);
        crate::utils::str_newtype!(@conversions #[owned to ref] $name1, $name2_ref);
        crate::utils::str_newtype!(@conversions #[owned to ref] $name2, $name1_ref);
    };

    (@eq #[owned to owned] $name1:ident, $name2:ident) => {
        impl std::cmp::PartialEq<$name2> for $name1 {
            fn eq(&self, other: &$name2) -> bool {
                self.0 == other.0
            }
        }

        impl std::cmp::PartialEq<$name1> for $name2 {
            fn eq(&self, other: &$name1) -> bool {
                self.0 == other.0
            }
        }
    };

    (@eq #[owned to ref] $name1:ident, $name2:ident) => {
        impl<'a> std::cmp::PartialEq<$name2<'a>> for $name1 {
            fn eq(&self, other: &$name2<'a>) -> bool {
                self.0 == other.0
            }
        }

        impl<'a> std::cmp::PartialEq<$name1> for $name2<'a> {
            fn eq(&self, other: &$name1) -> bool {
                self.0 == other.0
            }
        }
    };

    (@eq #[ref to ref] $name1:ident, $name2:ident) => {
        impl<'a> std::cmp::PartialEq<$name2<'a>> for $name1<'a> {
            fn eq(&self, other: &$name2<'a>) -> bool {
                self.0 == other.0
            }
        }

        impl<'a> std::cmp::PartialEq<$name1<'a>> for $name2<'a> {
            fn eq(&self, other: &$name1<'a>) -> bool {
                self.0 == other.0
            }
        }
    };

    (@eq #[all] $name1:ident, $name1_ref:ident, $name2:ident, $name2_ref:ident) => {
        crate::utils::str_newtype!(@eq #[owned to owned] $name1, $name2);
        crate::utils::str_newtype!(@eq #[owned to ref] $name1, $name2_ref);
        crate::utils::str_newtype!(@eq #[owned to ref] $name2, $name1_ref);
        crate::utils::str_newtype!(@eq #[ref to ref] $name1_ref, $name2_ref);
    };
}

macro_rules! qerror {
    ($vis:vis $name:ident, $($variant:ident : $error:literal),* $(,)?) => {
        #[derive(thiserror::Error, Debug, Clone, Copy, PartialEq, Eq, Hash)]
        $vis enum $name {
            $(
                #[error($error)]
                $variant,
            )*
        }
    };
}

pub(crate) use {qerror, str_newtype};
