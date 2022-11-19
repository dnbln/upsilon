#![feature(proc_macro_span)]

use proc_macro2::TokenStream;
use quote::{format_ident, quote, TokenStreamExt};
use std::time::SystemTime;

#[proc_macro]
pub fn private_context(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("SystemTime went before UNIX_EPOCH");

    let secs = duration.as_secs();
    let nanos = duration.subsec_nanos();
    let pid = std::process::id();

    let name = format_ident!("__private_context_{secs}_{nanos}_{pid}");
    let ts = TokenStream::from(item);

    proc_macro::TokenStream::from(quote! {
        mod #name {
            #ts
        }
    })
}

mod api_routes;

api_routes::api_version_macros! {(v1, 1)}

// Only to be called from upsilon-api
#[proc_macro]
pub fn api_routes(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ident = syn::parse_macro_input!(item as syn::Ident);

    let mut ts = TokenStream::new();

    append_versions(&mut ts, &ident);

    proc_macro::TokenStream::from(quote! {
        pub struct #ident;

        #ts
    })
}
