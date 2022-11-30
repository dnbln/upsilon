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

macro_rules! models {
    ($($name:ident),* $(,)?) => {
        const MODELS: &[&str] = &[
            $(
                concat!("../../api-models/protobuf/", stringify!($name), ".proto"),
            )*
        ];
    };
}

models! {
    namespace,
    organizations,
    repos,
    users,
}

fn main() {
    println!("cargo:rerun-if-changed=../../api-models/protobuf");
    println!("cargo:rerun-if-changed=build.rs");

    prost_build::Config::new()
        .format(true)
        .compile_protos(MODELS, &["../../api-models/protobuf"])
        .expect("prost_build");
}
