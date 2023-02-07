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

use ukonf::UkonfConfig;

fn main() {
    let value = ukonf::UkonfRunner::new(UkonfConfig::new(vec![]))
        .run_str(
            r#"
    a b: {
        c d: {
            let x: {
                a: 1
                b: 2
            }
            e f: {
                g: x
            }
            h i: {
                j: x
            }
        }
    }
    "#,
        )
        .unwrap()
        .into_value();

    let json = serde_json::to_string_pretty(&value.to_json()).unwrap();
    println!("{json}");

    let yaml = serde_yaml::to_string(&value.to_yaml()).unwrap();
    println!("{yaml}");
}
