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

use std::collections::BTreeMap;

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
            _ => panic!("Expected object, got {:?}", self),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct UkonfObject {
    map: BTreeMap<String, UkonfValue>,
}

impl UkonfObject {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, key: String, value: UkonfValue) {
        self.map.insert(key, value);
    }

    pub fn get_mut(&mut self, key: &str) -> Option<&mut UkonfValue> {
        self.map.get_mut(key)
    }

    pub fn get_or_insert(&mut self, key: String, value: UkonfValue) -> &mut UkonfValue {
        self.map.entry(key).or_insert(value)
    }

    pub fn into_value(self) -> UkonfValue {
        UkonfValue::Object(self)
    }
}

impl std::ops::Deref for UkonfObject {
    type Target = BTreeMap<String, UkonfValue>;

    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

pub mod to_json;
pub mod to_yaml;
