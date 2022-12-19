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

use std::fmt;

use serde_json::Value;

#[macro_export]
macro_rules! assert_json_eq {
    ($actual:expr, $expected:expr) => {{
        $crate::json_diff::_assert_same_json(&$actual, &$expected);
    }};
}

enum ChangeTag {
    Added,
    Removed,
    Equal,
}

struct JsonDiffResult {
    lines: Vec<(ChangeTag, String, Option<usize>, Option<usize>)>,
}

impl fmt::Display for JsonDiffResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (tag, line, old_index, new_index) in &self.lines {
            use colored::Colorize;

            let line_nums = match (old_index, new_index) {
                (Some(old), Some(new)) => format!("{old:>2} {new:>2}"),
                (Some(old), None) => format!("{old:>2}   "),
                (None, Some(new)) => format!("   {new:>2}"),
                (None, None) => unreachable!("Both indices are None"),
            };

            match tag {
                ChangeTag::Added => write!(f, "{}", format!("{line_nums} + {line}").green())?,
                ChangeTag::Removed => write!(f, "{}", format!("{line_nums} - {line}").red())?,
                ChangeTag::Equal => write!(f, "{line_nums}   {line}")?,
            }
        }
        Ok(())
    }
}

pub fn _assert_same_json(actual: &Value, expected: &Value) {
    if expected != actual {
        let expected_string = serde_json::to_string_pretty(expected).unwrap();
        let actual_string = serde_json::to_string_pretty(actual).unwrap();

        let mut diff = JsonDiffResult { lines: Vec::new() };
        let text_diff = similar::TextDiff::from_lines(&expected_string, &actual_string);

        for change in text_diff.iter_all_changes() {
            let tag = match change.tag() {
                similar::ChangeTag::Delete => ChangeTag::Removed,
                similar::ChangeTag::Insert => ChangeTag::Added,
                similar::ChangeTag::Equal => ChangeTag::Equal,
            };

            let old_index = change.old_index();
            let new_index = change.new_index();

            diff.lines
                .push((tag, change.to_string(), old_index, new_index));
        }

        eprint!("{}", diff);

        panic!("JSONs are not the same");
    }
}
