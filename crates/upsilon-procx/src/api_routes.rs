use proc_macro::Span;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};

pub fn versioned_route_link(version: usize, f_name: &Ident) -> TokenStream {
    let routes_holder_name = versioned_api_routes_static_holder_name(version);
    let route_wrap_name = format_ident!("__route_wrap_v{}_{}", version, f_name);

    quote! {
        #[linkme::distributed_slice(#routes_holder_name)]
        fn #route_wrap_name () -> Vec<rocket::Route> {
            rocket::routes![#f_name]
        }
    }
}

pub fn versioned_api_routes_static_holder_name(version: usize) -> Ident {
    // linkme doesn't allow multiple "holders" with the same name, so we create a
    // new one for each module we use `api_routes!` in, the name also being dependent
    // on the module path (by hashing here).
    let hash = {
        let span = Span::call_site();
        let mut hasher = DefaultHasher::new();
        span.source_file().path().hash(&mut hasher);
        hasher.finish()
    };

    format_ident!("__{hash}_V{version}_ROUTES")
}

pub fn versioned_api_routes(ident: &Ident, version: usize) -> TokenStream {
    let routes_holder_name = versioned_api_routes_static_holder_name(version);

    quote! {
        #[linkme::distributed_slice]
        static #routes_holder_name: [fn() -> Vec<rocket::Route>] = [..];

        impl crate::ApiRoutes<#version> for #ident {
            fn get_routes() -> Vec<rocket::Route> {
                let mut routes = Vec::new();

                for route in #routes_holder_name.iter() {
                    routes.extend(route());
                }

                routes
            }
        }
    }
}

macro_rules! version_macro {
    ($macro_name:ident, $version:literal) => {
        // Only to be called from upsilon-api
        #[proc_macro_attribute]
        pub fn $macro_name(
            _attr: ::proc_macro::TokenStream,
            item: ::proc_macro::TokenStream,
        ) -> proc_macro::TokenStream {
            let f = ::syn::parse_macro_input!(item as ::syn::ItemFn);
            let f_name = f.sig.ident.clone();
            let v_route_link = $crate::api_routes::versioned_route_link($version, &f_name);

            proc_macro::TokenStream::from(::quote::quote! {
                #f

                #v_route_link
            })
        }
    };
}

macro_rules! api_version_macros {
    (
        $(($macro_name:ident, $version:literal)),* $(,)?
    ) => {
        $(
            $crate::api_routes::version_macro!{$macro_name, $version}
        )*

        fn append_versions(ts: &mut ::proc_macro2::TokenStream, ident: &::syn::Ident) {
            $(
                ts.append_all($crate::api_routes::versioned_api_routes(&ident.clone(), $version));
            )*
        }
    };
}

pub(crate) use version_macro;
pub(crate) use api_version_macros;