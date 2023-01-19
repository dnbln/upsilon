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

use std::fmt;
use anyhow::bail;

use serde_json::Value;
use crate::TestResult;

#[macro_export]
macro_rules! expanded_json {
    ($it:ident) => {
        &$it
    };
    ($it:tt) => {
        $crate::serde_json::json!($it)
    };
}

#[macro_export]
macro_rules! assert_json_eq {
    ($actual:tt, $expected:tt) => {{
        let actual = $crate::expanded_json!($actual);
        let expected = $crate::expanded_json!($expected);

        $crate::json_diff::_assert_same_json(&actual, &expected)?;
    }};
}

pub fn _assert_same_json(actual: &Value, expected: &Value) -> TestResult {
    if expected != actual {
        let expected_string = serde_json::to_string_pretty(expected).unwrap();
        let actual_string = serde_json::to_string_pretty(actual).unwrap();

        let diff = upsilon_diff_util::build_diff(&expected_string, &actual_string);

        let sep = "=".repeat(20);
        eprintln!("JSONs are not equal:\n{sep}\n{diff}\n{sep}\n\n");

        bail!("JSONs are not the same: \n{diff}");
    }

    Ok(())
}
