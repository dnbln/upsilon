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

use std::fmt::Formatter;

use serde::de::{Error, MapAccess, SeqAccess};
use serde::{de, Deserialize, Deserializer};

pub(crate) struct Any;

impl<'de> Deserialize<'de> for Any {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct AnyVisitor;

        impl<'des> de::Visitor<'des> for AnyVisitor {
            type Value = Any;

            fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
                write!(formatter, "any value")
            }

            fn visit_bool<E>(self, _v: bool) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Any)
            }

            fn visit_i64<E>(self, _v: i64) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Any)
            }

            fn visit_str<E>(self, _v: &str) -> Result<Self::Value, E>
            where
                E: Error,
            {
                Ok(Any)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'des>,
            {
                while let Some(_elem) = seq.next_element::<Any>()? {}
                Ok(Any)
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'des>,
            {
                while let Some(_elem) = map.next_entry::<&str, Any>()? {}
                Ok(Any)
            }
        }

        deserializer.deserialize_any(AnyVisitor)
    }
}
