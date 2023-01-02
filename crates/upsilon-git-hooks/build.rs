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

use std::path::PathBuf;

struct Hook {
    name: &'static str,
    rust_name: &'static str,
}

const HOOKS: &[Hook] = &[
    Hook {
        name: "pre-receive",
        rust_name: "PreReceive",
    },
    Hook {
        name: "update",
        rust_name: "Update",
    },
    Hook {
        name: "post-receive",
        rust_name: "PostReceive",
    },
];

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let mut hooks_to_register = String::new();
    let mut app_variants = String::new();
    let mut run_hook_match_arms = String::new();

    for hook in HOOKS {
        hooks_to_register.push_str(&format!("{:?},", hook.name));

        app_variants.push_str(&format!(
            "
            #[clap(name = {git_hook_name:?})]
            {name}({name}),",
            git_hook_name = hook.name,
            name = hook.rust_name,
        ));

        run_hook_match_arms.push_str(&format!(
            "
            App::{name}(hook) => hook.run(),",
            name = hook.rust_name,
        ));
    }

    std::fs::write(
        out_dir.join("hooks.rs"),
        format!("pub const HOOKS_TO_REGISTER: &[&str] = &[{hooks_to_register}];"),
    )
    .expect("Failed to write hooks.rs");

    std::fs::write(
        out_dir.join("app.rs"),
        format!(
            r#"
            #[derive(clap::Parser, Debug)]
            pub enum App {{
                {app_variants}
            }}


            pub fn run_hook(app: App) -> GitHookResult<()> {{
                match app {{
                    {run_hook_match_arms}
                }}
            }}
            "#
        ),
    )
    .expect("Failed to write app.rs");
}
