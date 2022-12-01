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

#![feature(proc_macro_span)]
#![feature(proc_macro_diagnostic)]

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

#[proc_macro]
pub fn private_context(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime went before UNIX_EPOCH");

    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let pid = std::process::id();

    let hash = {
        let mut hasher = DefaultHasher::new();
        item.to_string().hash(&mut hasher);
        hasher.finish()
    };

    let name = format_ident!("__private_context_{secs}_{nanos}_{pid}_{hash}");
    let ts = TokenStream::from(item);

    proc_macro::TokenStream::from(quote! {
        mod #name {
            #ts
        }
    })
}

mod api_routes;

api_routes::version_proc_macro_wrappers! {}

// Only to be called from upsilon-api
#[proc_macro]
pub fn api_routes(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    api_routes::api_routes(item)
}

#[proc_macro_attribute]
pub fn api_configurator(
    attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    api_routes::api_configurator(attr, item)
}

