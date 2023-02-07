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

use serde_json::Value::Null;

use crate::value::{NumValue, UkonfValue};

impl UkonfValue {
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            UkonfValue::Null => Null,
            UkonfValue::Num(NumValue::Int(i)) => {
                serde_json::Value::Number(serde_json::Number::from(*i))
            }
            UkonfValue::Num(NumValue::Float(f)) => {
                serde_json::Value::Number(serde_json::Number::from_f64(*f).unwrap())
            }
            UkonfValue::Str(s) => serde_json::Value::String(s.clone()),
            UkonfValue::Bool(b) => serde_json::Value::Bool(*b),
            UkonfValue::Array(arr) => {
                serde_json::Value::Array(arr.iter().map(|v| v.to_json()).collect())
            }
            UkonfValue::Object(obj) => serde_json::Value::Object(
                obj.iter().map(|(k, v)| (k.clone(), v.to_json())).collect(),
            ),
        }
    }
}
