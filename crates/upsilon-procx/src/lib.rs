#![feature(proc_macro_span)]

extern crate core;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::SystemTime;

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

mod dart_model_class;

#[proc_macro_derive(DartModelClass, attributes(dart, dart_json))]
pub fn derive_dart_model_class(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    dart_model_class::derive_dart_model_class(item)
}

#[proc_macro]
pub fn dart_model_classes(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    dart_model_class::dart_model_classes(item)
}

#[proc_macro]
pub fn dart_model_classes_iter(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    dart_model_class::dart_model_classes_iter(item)
}
