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

use upsilon_procx::{dart_model_classes, DartModelClass};

dart_model_classes!();

#[derive(DartModelClass)]
pub struct F(String, Option<String>);

#[derive(DartModelClass)]
pub struct G {
    a: String,
    b: Option<String>,
    c: Vec<String>,
    d: F,
}

macro_rules! model_module {
    ($name:ident) => {
        pub mod $name {
            include!(concat!(
                env!("OUT_DIR"),
                "/models.",
                stringify!($name),
                ".rs"
            ));
        }
    };
}

pub mod models {
    model_module!(namespace);
    model_module!(organizations);
    model_module!(repos);
    model_module!(users);
}

fn x() {
    // let id = models::namespace::NamespaceId { kind: Some(models::namespace::Kind::User) };
}
