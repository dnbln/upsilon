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

use upsilon_json_diff::assert_json_eq;

use crate::{UkonfConfig, UkonfFunctions, UkonfRunner};

fn runner() -> UkonfRunner {
    UkonfRunner::new(UkonfConfig::new(vec![]), UkonfFunctions::new())
}

macro_rules! test_case_eq_json {
    ($test_name:ident, $ukonf:literal, $json:tt $(,)?) => {
        #[test]
        fn $test_name() {
            let r = runner();
            let v = r.run_str($ukonf).unwrap();
            let json = v.into_value().to_json();
            assert_json_eq!(json, $json);
        }
    };
}

test_case_eq_json! {simple, "a: 1", {"a": 1}}
test_case_eq_json! {simple2, "a: 2 c: 3", {"a": 2, "c": 3}}
test_case_eq_json! {
    simple_with_var,
    r#"
    let x: 1
    a: x
    "#,
    {"a": 1},
}
test_case_eq_json! {
    simple_with_var_in_var,
    r#"
    let x: 1
    let y: x
    a: y
    "#,
    {"a": 1},
}
test_case_eq_json! {
    multiple_keys,
    r#"
    a b c d: 1
    "#,
    {"a": {"b": {"c": {"d": 1}}}},
}
test_case_eq_json! {
    ref_key,
    r#"
    let x: 'b'
    a ${x}: 1
    "#,
    {"a": {"b": 1}},
}
test_case_eq_json! {
    ref_ref_key,
    r#"
    let x: 'y'
    let y: 'b'
    a ${${x}}: 1
    "#,
    {"a": {"b": 1}},
}
test_case_eq_json! {
    ref_ref_key_does_not_enter_inner_scope,
    r#"
    let x: 'y'
    let y: 'b'
    v {
        let y: 'c'
        a ${${x}}: 1
    }
    "#,
    {"v": {"a": {"b": 1}}},
}
test_case_eq_json! {
    kv_object_with_colon,
    r#"
    a b: {c: 1}
    "#,
    {"a": {"b": {"c": 1}}},
}
test_case_eq_json! {
    kv_object_without_colon,
    r#"
    a b {c: 1}
    "#,
    {"a": {"b": {"c": 1}}},
}

test_case_eq_json! {
    kv_array_with_colon,
    r#"
    a b: [1]
    "#,
    {"a": {"b": [1]}},
}
test_case_eq_json! {
    kv_array_without_colon,
    r#"
    a b [1]
    "#,
    {"a": {"b": [1]}},
}
