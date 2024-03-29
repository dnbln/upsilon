/*
 *        Copyright (c) 2023 Dinu Blanovschi
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

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Debug;

use anyhow::format_err;

use crate::ast::Span;
use crate::{UkonfFnError, UkonfRunError};

#[derive(Copy, Clone, Debug)]
pub enum NumValue {
    Int(i64),
    Float(f64),
}

#[derive(Clone, Debug)]
pub enum UkonfValue {
    Null,
    Num(NumValue),
    Str(String),
    Bool(bool),
    Array(Vec<UkonfValue>),
    Object(UkonfObject),
}

impl UkonfValue {
    pub fn as_string(&self) -> Option<&String> {
        match self {
            UkonfValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_string_mut(&mut self) -> Option<&mut String> {
        match self {
            UkonfValue::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn into_string(self) -> Result<String, Self> {
        match self {
            UkonfValue::Str(s) => Ok(s),
            v => Err(v),
        }
    }

    pub fn expect_string(self) -> Result<String, UkonfFnError> {
        match self {
            UkonfValue::Str(s) => Ok(s),
            v => Err(format_err!("Expected string, got: {v:?}")),
        }
    }

    pub fn expect_object(self) -> Result<UkonfObject, UkonfFnError> {
        match self {
            UkonfValue::Object(o) => Ok(o),
            v => Err(format_err!("Expected object, got: {v:?}")),
        }
    }

    pub fn clone_to_string(&self) -> Result<String, UkonfFnError> {
        match self {
            UkonfValue::Str(s) => Ok(s.clone()),
            v => Err(format_err!("Expected string, got: {v:?}")),
        }
    }

    pub fn expect_bool(self) -> Result<bool, UkonfFnError> {
        match self {
            UkonfValue::Bool(b) => Ok(b),
            v => Err(format_err!("Expected bool, got: {v:?}")),
        }
    }

    pub fn expect_array(self) -> Result<Vec<UkonfValue>, UkonfFnError> {
        match self {
            UkonfValue::Array(arr) => Ok(arr),
            v => Err(format_err!("Expected array, got: {v:?}")),
        }
    }

    pub(crate) fn _expect_string(self, span: &Span) -> Result<String, UkonfRunError> {
        match self {
            UkonfValue::Str(s) => Ok(s),
            v => Err(UkonfRunError::ExpectedString(v, span.clone())),
        }
    }

    pub(crate) fn _expect_object(self, span: &Span) -> Result<UkonfObject, UkonfRunError> {
        match self {
            UkonfValue::Object(obj) => Ok(obj),
            v => Err(UkonfRunError::ExpectedObject(v, span.clone())),
        }
    }

    pub(crate) fn expect_mut_object(
        &mut self,
        span: &Span,
    ) -> Result<&mut UkonfObject, UkonfRunError> {
        match self {
            UkonfValue::Object(obj) => Ok(obj),
            v => Err(UkonfRunError::ExpectedObject(v.clone(), span.clone())),
        }
    }

    pub fn as_object(&self) -> Option<&UkonfObject> {
        match self {
            UkonfValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    pub fn as_mut_object(&mut self) -> Option<&mut UkonfObject> {
        match self {
            UkonfValue::Object(obj) => Some(obj),
            _ => None,
        }
    }

    #[track_caller]
    pub(crate) fn unwrap_object(self) -> UkonfObject {
        match self {
            UkonfValue::Object(obj) => obj,
            _ => panic!("Expected object, got {self:?}"),
        }
    }
}

impl From<String> for UkonfValue {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}

impl From<&str> for UkonfValue {
    fn from(s: &str) -> Self {
        Self::Str(s.to_owned())
    }
}

impl From<bool> for UkonfValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<Vec<UkonfValue>> for UkonfValue {
    fn from(v: Vec<UkonfValue>) -> Self {
        Self::Array(v)
    }
}

impl From<UkonfObject> for UkonfValue {
    fn from(o: UkonfObject) -> Self {
        Self::Object(o)
    }
}

#[derive(Clone, Default)]
pub struct UkonfObject {
    map: Vec<UkonfValue>,
    key_map: BTreeMap<String, usize>,
}

impl fmt::Debug for UkonfObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;

        let mut need_comma = false;

        for (k, v) in &self.key_map {
            if need_comma {
                write!(f, ", ")?;
            }
            write!(f, "{:?}: {:?}", k, self.map[*v])?;
            need_comma = true;
        }

        write!(f, "}}")?;

        Ok(())
    }
}

impl UkonfObject {
    pub fn new() -> Self {
        Self {
            map: Vec::new(),
            key_map: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: UkonfValue) {
        self.key_map.insert(key, self.map.len());
        self.map.push(value);
    }

    pub fn with(mut self, key: impl Into<String>, value: impl Into<UkonfValue>) -> Self {
        self.insert(key.into(), value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&UkonfValue> {
        let Some(k) = self.key_map.get(key) else {
            return None;
        };

        Some(&self.map[*k])
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut UkonfValue> {
        let Some(k) = self.key_map.get(key) else {
            return None;
        };

        Some(&mut self.map[*k])
    }

    #[track_caller]
    pub fn get_or_insert(&mut self, key: String, value: UkonfValue) -> &mut UkonfValue {
        match self.key_map.entry(key) {
            Entry::Occupied(e) => &mut self.map[*e.get()],
            Entry::Vacant(e) => {
                let index = self.map.len();
                self.map.push(value);
                e.insert(index);
                &mut self.map[index]
            }
        }
    }

    pub fn into_value(self) -> UkonfValue {
        UkonfValue::Object(self)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &UkonfValue)> {
        let mut keys = self
            .key_map
            .iter()
            .map(|(k, v)| (k, *v))
            .collect::<Vec<_>>();

        keys.sort_by_key(|(_, v)| *v);

        keys.into_iter().map(|(k, v)| (k, &self.map[v]))
    }

    pub fn into_iter(self) -> impl Iterator<Item = (String, UkonfValue)> {
        let mut keys = self
            .key_map
            .into_iter()
            .map(|(k, v)| (k, v))
            .collect::<Vec<_>>();

        keys.sort_by_key(|(_, v)| *v);

        let mut map = self.map.into_iter().map(Some).collect::<Vec<_>>();

        keys.into_iter()
            .map(move |(k, v)| (k, map[v].take().unwrap()))
    }
}

pub mod to_json;
pub mod to_yaml;
