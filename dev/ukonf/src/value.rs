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

#[derive(Clone, Debug, Default)]
pub struct UkonfObject {
    map: Vec<UkonfValue>,
    key_map: BTreeMap<String, usize>,
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
}

pub mod to_json;
pub mod to_yaml;
