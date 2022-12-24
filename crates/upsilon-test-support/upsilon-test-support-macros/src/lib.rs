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
#![feature(drain_filter)]

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::token::Async;
use syn::{Attribute, FnArg, ReturnType, Stmt, Type};

#[proc_macro_attribute]
pub fn upsilon_test(
        attr: proc_macro::TokenStream,
        item: proc_macro::TokenStream,
        ) -> proc_macro::TokenStream {
    let fun = syn::parse_macro_input!(item as syn::ItemFn);

    expand_upsilon_test(attr.into(), fun)
    .unwrap_or_else(syn::Error::into_compile_error)
    .into()
}

fn expand_upsilon_test(attr: TokenStream, mut fun: syn::ItemFn) -> syn::Result<TokenStream> {
    let guard = quote! {
        if !::std::env::var("UPSILON_TEST_GUARD").is_ok() {
            panic!("UPSILON_TEST_GUARD not set; did you use `cargo xtask test` to run the tests?");
        }
    };

    let rt = &mut fun.sig.output;

    match rt {
        ReturnType::Default => {
            return Err(syn::Error::new(
                fun.sig.span(),
                "need return type to be TestResult",
            ));
        }
        ReturnType::Type(_, _) => {}
    }

    if let None | Some(Stmt::Item(_) | Stmt::Semi(_, _) | Stmt::Local(_)) = fun.block.stmts.last() {
        fun.block.stmts.push(Stmt::Expr(
            syn::parse_quote! {Ok::<_, upsilon_test_support::TestError>(())},
        ));
    }

    let inner_fun_name = format_ident!("__upsilon_test_impl");
    let name = std::mem::replace(&mut fun.sig.ident, inner_fun_name.clone());
    let asyncness = fun.sig.asyncness.clone();
    let vis = std::mem::replace(&mut fun.vis, syn::Visibility::Inherited);
    let inputs = fun
        .sig
        .inputs
        .iter_mut()
        .filter_map(|it| match it {
            FnArg::Typed(pat) => Some(pat),
            _ => None,
        })
        .map(|it| {
            let (test_attrs, other_attrs) = it.attrs.drain(..).partition(|it| {
                it.path.is_ident("cfg_setup")
                    || it.path.is_ident("setup")
                    || it.path.is_ident("teardown")
            });

            it.attrs = other_attrs;

            (test_attrs, it.ty.clone())
        })
        .collect::<Vec<_>>();

    let mut test_attrs = TokenStream::new();
    let mut works_offline_opt = None;

    for attr in fun.attrs.drain_filter(|attr| attr.path.is_ident("offline")) {
        if attr.path.is_ident("offline") {
            let works_offline = if attr.tokens.is_empty() {
                true
            } else {
                let ident = attr.parse_args::<Ident>()?;

                if ident == "ignore" {
                    false
                } else if ident == "run" {
                    true
                } else {
                    return Err(syn::Error::new(ident.span(), "Should be `ignore` or `run`"));
                }
            };

            if works_offline_opt.is_some() {
                return Err(syn::Error::new(
                    attr.span(),
                    "Multiple #[offline] attributes",
                ));
            }

            works_offline_opt = Some(works_offline);
        }
    }

    if matches!(works_offline_opt, Some(false) | None) {
        test_attrs.append_all(quote! {#[cfg_attr(offline, ignore)]})
    }

    let mut body = quote! { #fun };

    let inner_fn_call = InnerFnCall {
        test_name: name.clone(),
        inner_fun_name,
        asyncness,
        inputs,
    };

    body.append_all(quote! { #inner_fn_call });

    let ts = quote! {
        #[tokio::test]
        #test_attrs
        #vis async fn #name () {
            #guard

            #body
        }
    };

    eprintln!("{ts}");

    Ok(ts)
}

struct InnerFnCall {
    test_name: Ident,
    inner_fun_name: Ident,
    asyncness: Option<Async>,
    inputs: Vec<(Vec<Attribute>, Box<Type>)>,
}

impl ToTokens for InnerFnCall {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let params = self.inputs.iter();

        let mut setup = TokenStream::new();
        let mut parameters = Vec::new();
        let mut teardown = TokenStream::new();
        let mut finish = TokenStream::new();
        let test_name = self.test_name.to_string();

        let file_path_hash = {
            use std::hash::{Hash, Hasher};
            let span = Spanned::span(&test_name);
            let source_file = span.unwrap().source_file();
            let path = source_file.path();
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            path.hash(&mut hasher);
            hasher.finish()
        };

        let vars_name = format_ident!("__upsilon_test_vars");
        let vars_setup = quote! {
            let #vars_name = ::upsilon_test_support::CxConfigVars {
                workdir: ::std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")),
                test_name: #test_name,
                source_file_path_hash: #file_path_hash
            };
        };

        for (i, (attrs, ty)) in params.enumerate() {
            let ty = match **ty {
                Type::Reference(ref r) => &r.elem,
                _ => panic!("Unsupported type, need reference"),
            };

            let param_name = format_ident!("__param_{}", i);
            let param_name_config = format_ident!("__param_{}_config", i);

            setup.append_all(quote! {
                let mut #param_name_config = <#ty>::Config::new(&#vars_name);
            });

            for setup_fn in
                find_attrs(attrs, "cfg_setup").expect("cannot parse cfg_setup attribute")
            {
                setup.append_all(quote! {
                    #setup_fn(&mut #param_name_config);
                });
            }

            setup.append_all(quote! {
                let mut #param_name = <#ty>::init(#param_name_config).await;
            });

            for setup_fn in find_attrs(attrs, "setup").expect("cannot parse setup attribute") {
                setup.append_all(quote! {
                    #setup_fn(&mut #param_name).await;
                });
            }

            for teardown_fn in
                find_attrs(attrs, "teardown").expect("cannot parse teardown attribute")
            {
                teardown.append_all(quote! {
                    #teardown_fn(&mut #param_name).await;
                });
            }

            finish.append_all(quote! {
                #param_name.finish().await;
            });

            parameters.push(quote! {
                &mut #param_name
            });
        }

        let inner_fun_name = &self.inner_fun_name;
        let await_token = self.asyncness.as_ref().map(|_| quote! { .await });

        tokens.append_all(quote! {
            #vars_setup

            #setup

            let result = #inner_fun_name(#(#parameters),*) #await_token;

            #teardown

            if let Err(e) = result {
                panic!("Error: {}", e);
            }

            #finish
        });
    }
}

fn find_attrs(attrs: &[Attribute], name: &str) -> syn::Result<Vec<syn::Path>> {
    type CommaPath = syn::punctuated::Punctuated<syn::Path, syn::token::Comma>;

    struct CommaPathParse(CommaPath);

    impl syn::parse::Parse for CommaPathParse {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            CommaPath::parse_terminated(input).map(Self)
        }
    }

    Ok(attrs
        .iter()
        .filter(|it| it.path.is_ident(name))
        .map(|it| it.parse_args::<CommaPathParse>())
        .collect::<syn::Result<Vec<CommaPathParse>>>()?
        .into_iter()
        .map(|it| it.0.into_iter())
        .flatten()
        .collect::<Vec<_>>())
}
