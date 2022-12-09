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

use std::path::PathBuf;

struct Hook {
    name: &'static str,
    rust_name: &'static str,
    args: &'static [HookArg],
}

struct HookArg {
    name: &'static str,
    rust_type: &'static str,
}

const HOOKS: &[Hook] = &[
    Hook {
        name: "pre-receive",
        rust_name: "PreReceive",
        args: &[],
    },
    Hook {
        name: "update",
        rust_name: "Update",
        args: &[
            HookArg {
                name: "ref_name",
                rust_type: "String",
            },
            HookArg {
                name: "old_oid",
                rust_type: "String",
            },
            HookArg {
                name: "new_oid",
                rust_type: "String",
            },
        ],
    },
    Hook {
        name: "post-receive",
        rust_name: "PostReceive",
        args: &[],
    },
];

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let mut hooks_to_register = String::new();
    let mut app_variants = String::new();

    for hook in HOOKS {
        hooks_to_register.push_str(&format!("{:?},", hook.name));

        let mut args = String::new();

        if !hook.args.is_empty() {
            args.push_str(" {");
        }

        for arg in hook.args {
            args.push_str(&format!("{}: {}, ", arg.name, arg.rust_type));
        }

        if !hook.args.is_empty() {
            args.push('}');
        }

        app_variants.push_str(&format!(
            "
            #[clap(name = {:?})]
            {}{},",
            hook.name, hook.rust_name, args,
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
            "
            #[derive(clap::Parser, Debug)]
            pub enum App {{
                {app_variants}
            }}
            "
        ),
    )
    .expect("Failed to write app.rs");
}
