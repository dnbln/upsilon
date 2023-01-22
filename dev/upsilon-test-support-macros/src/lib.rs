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

#![feature(proc_macro_span)]
#![feature(drain_filter)]

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, TokenStreamExt};
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

    let result = expand_upsilon_test(attr.into(), fun)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into();

    result
}

fn expand_upsilon_test(_attr: TokenStream, mut fun: syn::ItemFn) -> syn::Result<TokenStream> {
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
    let asyncness = fun.sig.asyncness;
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

    let mut config_path = quote! { ::upsilon_test_support::helpers::upsilon_basic_config };

    for attr in fun.attrs.drain_filter(|attr| {
        ["offline", "git_daemon", "git_ssh", "test_attr"]
            .into_iter()
            .any(|it| attr.path.is_ident(it))
    }) {
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
        } else if attr.path.is_ident("git_daemon") {
            config_path =
                quote! {::upsilon_test_support::helpers::upsilon_basic_config_with_git_daemon};
            test_attrs.append_all(quote! {
                #[cfg_attr(all(windows, ci), ignore = "git-daemon behaves in weird ways on Windows, and may crash for no reason, so it's disabled on CI")]
            });
        } else if attr.path.is_ident("git_ssh") {
            config_path = quote! {::upsilon_test_support::helpers::upsilon_basic_config_with_ssh};
            test_attrs.append_all(quote! {
                #[cfg_attr(windows, ignore = "git-shell is not available in git-for-windows, so ssh tests should be ignored")]
            });
        } else if attr.path.is_ident("test_attr") {
            let test_attr = attr.parse_args::<syn::Meta>()?;

            test_attrs.append_all(quote! { #[#test_attr] });
        } else {
            unreachable!()
        }
    }

    let works_offline = matches!(works_offline_opt, Some(true) | None);

    if !works_offline {
        test_attrs.append_all(quote! {#[cfg_attr(offline, ignore = "Test doesn't work offline")]})
    }

    let mut body = quote! { #fun };

    let inner_fn_call = InnerFnCall {
        test_name: name.clone(),
        inner_fun_name,
        asyncness,
        inputs,
        works_offline,
        config_path,
    };

    let inner_fn_call_ts = inner_fn_call.build()?;

    body.append_all(quote! { #inner_fn_call_ts });

    let ts = quote! {
        #[tokio::test]
        #test_attrs
        #vis async fn #name () {
            #guard

            #body
        }
    };

    Ok(ts)
}

struct InnerFnCall {
    test_name: Ident,
    inner_fun_name: Ident,
    asyncness: Option<Async>,
    inputs: Vec<(Vec<Attribute>, Box<Type>)>,
    works_offline: bool,
    config_path: TokenStream,
}

impl InnerFnCall {
    fn build(&self) -> syn::Result<TokenStream> {
        let mut tokens = TokenStream::new();

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
        let test_result_name = format_ident!("__upsilon_test_result");
        let works_offline = self.works_offline;
        let config_path = &self.config_path;
        let vars_setup = quote! {
            let #vars_name = ::upsilon_test_support::CxConfigVars {
                workdir: ::std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")),
                test_name: #test_name,
                source_file_path_hash: #file_path_hash,
                works_offline: #works_offline,
                config_init: #config_path,
            };
        };

        fn param_name_for_index(index: usize) -> Ident {
            format_ident!("__upsilon_test_param_{}", index)
        }

        for (i, (attrs, ty)) in params.enumerate() {
            let ty = match **ty {
                Type::Reference(ref r) => {
                    if r.mutability.is_none() {
                        return Err(syn::Error::new(
                            r.span(),
                            "Expected a mutable reference type",
                        ));
                    }

                    &r.elem
                }
                _ => return Err(syn::Error::new(ty.span(), "Expected a reference type")),
            };

            let param_name = param_name_for_index(i);
            let param_name_config = format_ident!("__param_{}_config", i);

            setup.append_all(quote! {
                let mut #param_name_config = <#ty>::Config::new(&#vars_name);
            });

            for setup_fn in find_attrs(attrs, "cfg_setup")? {
                setup.append_all(quote! {
                    #setup_fn(&mut #param_name_config);
                });
            }

            setup.append_all(quote! {
                let mut #param_name = <#ty>::init(#param_name_config).await?;
            });

            for setup_fn in find_attrs(attrs, "setup")? {
                setup.append_all(quote! {
                    #setup_fn(&mut #param_name).await?;
                });
            }

            for teardown_fn in find_attrs(attrs, "teardown")? {
                teardown.append_all(quote! {
                    #teardown_fn(&mut #param_name).await?;
                });
            }

            finish.append_all(quote! {
                if let Err(e) = #param_name.finish(#test_result_name).await {
                    ::upsilon_test_support::log::error!("Error finishing test parameter {}: {}", stringify!(#ty), e);

                    return Err(e);
                }
            });

            if i != 0 {
                parameters.push(quote! {
                    &mut #param_name
                });
            }
        }

        let test_wrapper_fn_name = format_ident!("__upsilon_test_wrapper");
        let inner_fun_name = &self.inner_fun_name;

        let param_name = param_name_for_index(0);

        tokens.append_all(quote! {
            async fn #test_wrapper_fn_name() -> TestResult<()> {
                #vars_setup

                #setup

                let #test_result_name = match {
                    use ::upsilon_test_support::futures::future::FutureExt;
                    let fut = ::core::panic::AssertUnwindSafe(#inner_fun_name(&mut #param_name, #(#parameters),*)).catch_unwind();
                    fut.await
                } {
                    Ok(Ok(v)) => Ok(v),
                    Ok(Err(e)) => Err(e),
                    Err(e) => {
                        #param_name.set_panic_info(e);
                        Ok(())
                    },
                };

                #teardown

                #finish

                Ok(())
            }

            if let Err(e) = #test_wrapper_fn_name().await {
                panic!("Test result is Err: {}", e);
            }
        });

        Ok(tokens)
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
        .flat_map(|it| it.0.into_iter())
        .collect::<Vec<_>>())
}
