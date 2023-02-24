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

use crate::{Scope, UkonfConfig, UkonfFunctions, UkonfRunner};

fn fns() -> UkonfFunctions {
    let mut fns = UkonfFunctions::new();

    fns.add_fn("concat", |_scope, args| {
        let mut s = String::new();
        for arg in args {
            s.push_str(&arg.clone_to_string()?);
        }
        Ok(s.into())
    });

    fns.add_fn("concat2", |scope, args| {
        let mut s = String::new();

        if let Some(vars) = Scope::resolve_cx(scope, "concat_prefix").transpose()? {
            for arg in vars.expect_array()? {
                s.push_str(&arg.clone_to_string()?);
            }
        }

        for arg in args {
            s.push_str(&arg.clone_to_string()?);
        }

        if let Some(vars) = Scope::resolve_cx(scope, "concat_suffix").transpose()? {
            for arg in vars.expect_array()? {
                s.push_str(&arg.clone_to_string()?);
            }
        }

        Ok(s.into())
    });

    fns.add_compiler_fn("compiler1", |_scope, val| {
        let mut s = String::new();
        for arg in val.expect_array()? {
            s.push_str(&arg.clone_to_string()?);
        }
        Ok(s.into())
    });

    fns.add_compiler_fn("compiler2", |scope, val| {
        let mut s = String::new();
        if let Some(vars) = Scope::resolve_cx(scope, "compiler_prefix").transpose()? {
            for arg in vars.expect_array()? {
                s.push_str(&arg.clone_to_string()?);
            }
        }
        for arg in val.expect_array()? {
            s.push_str(&arg.clone_to_string()?);
        }
        if let Some(vars) = Scope::resolve_cx(scope, "compiler_suffix").transpose()? {
            for arg in vars.expect_array()? {
                s.push_str(&arg.clone_to_string()?);
            }
        }
        Ok(s.into())
    });

    fns
}

fn runner() -> UkonfRunner {
    UkonfRunner::new(UkonfConfig::new(vec![]), fns())
}

macro_rules! test_case_eq_json {
    ($test_name:ident, $ukonf:expr, $json:tt $(,)?) => {
        #[test]
        fn $test_name() {
            let r = runner();
            let v = r.run_str($ukonf).unwrap();
            let json = v.into_value().to_json();
            assert_json_eq!(json, $json);
        }
    };
}

// test to check that simple json works
test_case_eq_json! {simple, "a: 1", {"a": 1}}

// test to check that simple json works
test_case_eq_json! {simple2, "a: 2 c: 3", {"a": 2, "c": 3}}

// test to check that vars work
test_case_eq_json! {
    simple_with_var,
    r#"
    let x: 1
    a: x
    "#,
    {"a": 1},
}

// test to check that vars work in other vars
test_case_eq_json! {
    simple_with_var_in_var,
    r#"
    let x: 1
    let y: x
    a: y
    "#,
    {"a": 1},
}

// test to check that mutliple keys (`a b c: 1`) works
test_case_eq_json! {
    multiple_keys,
    r#"
    a b c d: 1
    "#,
    {"a": {"b": {"c": {"d": 1}}}},
}

// test to check that `${v}` works
test_case_eq_json! {
    ref_key,
    r#"
    let x: 'b'
    a ${x}: 1
    "#,
    {"a": {"b": 1}},
}

// test to check that `${${v}}` works
test_case_eq_json! {
    ref_ref_key,
    r#"
    let x: 'y'
    let y: 'b'
    a ${${x}}: 1
    "#,
    {"a": {"b": 1}},
}

// test to check that `${${v}}` does not enter the inner scope
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

// test to check `v: {v}` works
test_case_eq_json! {
    kv_object_with_colon,
    r#"
    a b: {c: 1}
    "#,
    {"a": {"b": {"c": 1}}},
}

// test to check `v {v}` works
test_case_eq_json! {
    kv_object_without_colon,
    r#"
    a b {c: 1}
    "#,
    {"a": {"b": {"c": 1}}},
}

// test to check `v: [v]` works
test_case_eq_json! {
    kv_array_with_colon,
    r#"
    a b: [1]
    "#,
    {"a": {"b": [1]}},
}

// test to check `v [v]` works
test_case_eq_json! {
    kv_array_without_colon,
    r#"
    a b [1]
    "#,
    {"a": {"b": [1]}},
}

// test to check `v.v` works
test_case_eq_json! {
    get_key,
    r#"
    let x: {a: 1 b: 2}
    a: x.a
    "#,
    {"a": 1},
}

// test to check `v.${v}` works
test_case_eq_json! {
    get_key_with_ref,
    r#"
    let x: {a: 1 b: 2}
    let y: 'a'
    a: x.${y}
    "#,
    {"a": 1},
}

// test to check `v.${${v}}` works
test_case_eq_json! {
    get_key_with_ref_ref,
    r#"
    let x: {a: 1 b: 2}
    let y: 'a'
    let z: 'y'
    a: x.${${z}}
    "#,
    {"a": 1},
}

// test to check \r is not present in output (from """ string)
test_case_eq_json! {
    no_carriage_return,
    concat!(r#"a: """ "#, "\r\n", r#" """ "#),
    {"a": " \n "},
}

// test to check that concat works
test_case_eq_json! {
    concat,
    r#"
    let x: 'a'
    let y: 'b'
    a: concat(x, y)
    "#,
    {"a": "ab"},
}

// test to check that context vars work
test_case_eq_json! {
    context_vars,
    r#"
    let x: 'a'
    let y: 'c'
    let cx concat_prefix: [x]
    let cx concat_suffix: [y]
    a: concat2('b')
    b: concat2('b', ' ', 'c')
    "#,
    {"a": "abc", "b": "ab cc"},
}

// test to check that value compilers work
test_case_eq_json! {
    value_compiler,
    r#"
    let x compiler compiler1: ['a' 'b']

    a: x
    "#,
    {"a": "ab"},
}

// test to check that value compilers
// can access context variables
test_case_eq_json! {
    value_compiler_with_context,
    r#"
    let x compiler compiler2: ['a' 'b']

    a: {
        let cx compiler_prefix: ['c']
        x: x
    }

    b: {
        let cx compiler_prefix: ['d']
        let cx compiler_suffix: ['e']
        x: x
    }
    "#,
    {"a": {"x": "cab"}, "b": {"x": "dabe"}},
}

mod internals;
