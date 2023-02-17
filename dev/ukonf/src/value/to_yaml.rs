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

use crate::value::{NumValue, UkonfValue};

impl UkonfValue {
    pub fn to_yaml(&self) -> serde_yaml::Value {
        match self {
            UkonfValue::Null => serde_yaml::Value::Null,
            UkonfValue::Num(NumValue::Int(i)) => {
                serde_yaml::Value::Number(serde_yaml::Number::from(*i))
            }
            UkonfValue::Num(NumValue::Float(f)) => {
                serde_yaml::Value::Number(serde_yaml::Number::from(*f))
            }
            UkonfValue::Str(s) => serde_yaml::Value::String(s.clone()),
            UkonfValue::Bool(b) => serde_yaml::Value::Bool(*b),
            UkonfValue::Array(arr) => {
                serde_yaml::Value::Sequence(arr.iter().map(|v| v.to_yaml()).collect())
            }
            UkonfValue::Object(obj) => serde_yaml::Value::Mapping(
                obj.iter()
                    .map(|(k, v)| (serde_yaml::Value::String(k.clone()), v.to_yaml()))
                    .collect(),
            ),
        }
    }
}
