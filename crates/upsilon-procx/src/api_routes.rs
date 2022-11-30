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

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use proc_macro::Span;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, TokenStreamExt};

fn versioned_route_link(version: usize, f_name: &Ident) -> TokenStream {
    let routes_holder_name = versioned_api_routes_static_holder_name(version);
    let route_wrap_name = format_ident!("__route_wrap_v{version}_{f_name}");

    quote! {
        #[linkme::distributed_slice(#routes_holder_name)]
        fn #route_wrap_name () -> Vec<rocket::Route> {
            rocket::routes![#f_name]
        }
    }
}

fn file_hash() -> u64 {
    // linkme doesn't allow multiple statics with the same name, so
    // we also use the hash of the source file path
    let span = Span::call_site();
    let mut hasher = DefaultHasher::new();
    span.source_file().path().hash(&mut hasher);
    hasher.finish()
}

fn versioned_api_routes_static_holder_name(version: usize) -> Ident {
    let hash = file_hash();

    format_ident!("__{hash}_V{version}_ROUTES")
}

fn global_versioned_api_routes_static_holder_name(version: usize) -> Ident {
    format_ident!("__V{version}_HOLDERS_COLLECTOR")
}

fn holder_name(version: usize) -> Ident {
    format_ident!("__holder__V{version}")
}

fn version_holder_static(version: usize) -> TokenStream {
    let holder_name = global_versioned_api_routes_static_holder_name(version);

    quote! {
        #[linkme::distributed_slice]
        static #holder_name: [fn() -> Vec<rocket::Route>] = [..];
    }
}

fn versioned_api_routes(version: usize) -> TokenStream {
    let routes_holder_name = versioned_api_routes_static_holder_name(version);
    let all_holders = global_versioned_api_routes_static_holder_name(version);
    let all_holders_path = quote! { crate:: #all_holders };
    let hn = holder_name(version);

    quote! {
        #[linkme::distributed_slice]
        static #routes_holder_name: [fn() -> Vec<rocket::Route>] = [..];

        #[linkme::distributed_slice(#all_holders_path)]
        fn #hn () -> Vec<rocket::Route> {
            let mut routes = Vec::new();

            for route in #routes_holder_name.iter() {
                routes.extend(route());
            }

            routes
        }
    }
}

fn versioned_api_fairing_name(version: usize) -> Ident {
    format_ident!("__ApiFairing_V{}", version)
}

fn versioned_api_fairing_decl(version: usize) -> TokenStream {
    let fairing_name = versioned_api_fairing_name(version);
    let api_v_root = format!("/api/v{version}");
    let fairing_desc_str = format!("API v{version}");
    let all_holders = global_versioned_api_routes_static_holder_name(version);

    quote! {
        struct #fairing_name;

        #[rocket::async_trait]
        impl rocket::fairing::Fairing for #fairing_name {
            fn info(&self) -> rocket::fairing::Info {
                rocket::fairing::Info {
                    name: #fairing_desc_str,
                    kind: rocket::fairing::Kind::Ignite | rocket::fairing::Kind::Singleton,
                }
            }

            async fn on_ignite(&self, rocket: Rocket<Build>) -> rocket::fairing::Result {
                let mut joined = rocket::routes![];

                for holder in #all_holders .iter() {
                    joined.extend(holder());
                }

                Ok(rocket.mount(#api_v_root, joined))
            }
        }
    }
}

fn version_macro(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
    version: usize,
) -> proc_macro::TokenStream {
    let f = ::syn::parse_macro_input!(item as ::syn::ItemFn);
    let f_name = f.sig.ident.clone();
    let v_route_link = versioned_route_link(version, &f_name);

    proc_macro::TokenStream::from(quote! {
        #f

        #v_route_link
    })
}

macro_rules! version_macro {
    ($macro_name:ident, $version:literal) => {
        // Only to be called from upsilon-api
        pub fn $macro_name(
            attr: proc_macro::TokenStream,
            item: proc_macro::TokenStream,
        ) -> proc_macro::TokenStream {
            version_macro(attr, item, $version)
        }
    };
}

macro_rules! api_version_macros {
    (
        $(($macro_name:ident, $version:literal)),* $(,)?
    ) => {
        $(
            version_macro!{$macro_name, $version}
        )*

        fn append_versions(ts: &mut TokenStream) {
            $(
                ts.append_all(versioned_api_routes($version));
            )*
        }

        fn append_versions_holders(ts: &mut TokenStream) {
            $(
                ts.append_all(version_holder_static($version));
            )*
        }

        fn append_api_fairing_decls(ts: &mut TokenStream) {
            $(
                ts.append_all(versioned_api_fairing_decl($version));
            )*
        }

        fn append_api_fairing_attach_chain(ts: &mut TokenStream) {
            $(
                let fairing_name = versioned_api_fairing_name($version);
                ts.append_all(quote! {
                    .attach(#fairing_name)
                });
            )*
        }

        macro_rules! version_proc_macro_wrappers {
            () => {
                $(
                    #[proc_macro_attribute]
                    pub fn $macro_name(
                        attr: ::proc_macro::TokenStream,
                        item: ::proc_macro::TokenStream,
                    ) -> ::proc_macro::TokenStream {
                        api_routes::$macro_name(attr, item)
                    }
                )*
            };
        }
    };
}

api_version_macros! {(v1, 1)}
pub(crate) use version_proc_macro_wrappers;

pub fn api_routes(_item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let mut ts = TokenStream::new();

    append_versions(&mut ts);

    proc_macro::TokenStream::from(ts)
}

pub fn api_configurator(
    _attr: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let item = syn::parse_macro_input!(item as syn::ItemStruct);
    let name = &item.ident;

    let mut holders_ts = TokenStream::new();

    append_versions_holders(&mut holders_ts);

    let mut api_fairing_decls = TokenStream::new();
    append_api_fairing_decls(&mut api_fairing_decls);

    let mut api_fairing_attach_chain = TokenStream::new();
    append_api_fairing_attach_chain(&mut api_fairing_attach_chain);

    proc_macro::TokenStream::from(quote! {
        #item

        #holders_ts

        #api_fairing_decls

        #[rocket::async_trait]
        impl rocket::fairing::Fairing for #name {
            fn info(&self) -> rocket::fairing::Info {
                rocket::fairing::Info {
                    name: "API fairing configurator",
                    kind: rocket::fairing::Kind::Ignite | rocket::fairing::Kind::Singleton,
                }
            }

            async fn on_ignite(&self, rocket: rocket::Rocket<rocket::Build>) -> rocket::fairing::Result {
                Ok(rocket #api_fairing_attach_chain)
            }
        }
    })
}
