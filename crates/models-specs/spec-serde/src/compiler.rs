use std::path::Path;
use std::rc::Rc;

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens, TokenStreamExt};
use spec::ast::Ident;
use spec::diagnostics::DiagnosticsHost;
use spec::lower::*;
use spec::Compiler;

pub struct SerdeCompiler;

impl Default for SerdeCompiler {
    fn default() -> Self {
        Self
    }
}

impl Compiler for SerdeCompiler {
    fn compile_file(
        &self,
        cx: spec::CompileCx,
        file: &Rc<LowerFile>,
        diagnostics: &DiagnosticsHost,
        to: &Path,
    ) {
        compile_file(&CompileCx::new_from(&cx, diagnostics), file, to)
    }
}

struct CompileCx<'a> {
    spec_cx: &'a spec::CompileCx<'a>,
    diagnostics: &'a DiagnosticsHost,
}

impl<'a> CompileCx<'a> {
    fn new_from(spec_cx: &'a spec::CompileCx<'a>, diagnostics: &'a DiagnosticsHost) -> Self {
        Self {
            spec_cx,
            diagnostics,
        }
    }
}

fn compile_file(cx: &CompileCx, file: &Rc<LowerFile>, to: &Path) {
    let mut modules = TokenStream::new();

    let mut root_module_imports = vec![];

    for package in file.packages.iter() {
        root_module_imports.push(package.name.clone());
    }

    for package in file.packages.iter() {
        modules.append_all(compile_package(cx, package, &root_module_imports, 0));
    }

    let s = format!("{}", modules);
    let result = std::fs::write(to, &s);

    if let Err(e) = result {
        eprintln!("Error while writing to file {}: {}", to.display(), e);
        cx.spec_cx.compilation_failed();
    }
}

fn compile_package(
    cx: &CompileCx,
    package: &Rc<LowerPackage>,
    root_module_imports: &[Ident],
    current_depth: usize,
) -> TokenStream {
    let mut module_contents = TokenStream::new();

    let mut parents = TokenStream::new();

    for _ in 0..(current_depth + 1) {
        parents.append_all(quote! {super::});
    }

    for import in root_module_imports {
        let import_ident = format_ident!("{}", import.as_str());

        module_contents.append_all(quote! {
            pub use #parents #import_ident;
        });
    }

    for package_item in package.package_items.iter() {
        match package_item {
            LowerPackageItem::NewtypeStruct(newtype_struct) => {
                module_contents.append_all(compile_newtype_struct(cx, newtype_struct));
            }
            LowerPackageItem::Struct(struct_) => {
                module_contents.append_all(compile_struct(cx, struct_));
            }
            LowerPackageItem::Enum(enum_) => {
                module_contents.append_all(compile_enum(cx, enum_));
            }
            LowerPackageItem::Package(package) => {
                module_contents.append_all(compile_package(
                    cx,
                    package,
                    root_module_imports,
                    current_depth + 1,
                ));
            }
        }
    }

    let mod_name = format_ident!("{}", package.name.as_str());

    quote! {
        pub mod #mod_name {
            #module_contents
        }
    }
}

fn resolve_to_path(path: &LowerPath, result: &mut TokenStream) {
    match path {
        LowerPath::Ident(ident) => {
            let id = format_ident!("{}", ident.as_str());
            result.append_all(quote! { #id });
        }
        LowerPath::Path(path, _, id) => {
            resolve_to_path(path, result);
            let id = format_ident!("{}", id.as_str());
            result.append_all(quote! { ::#id });
        }
    }
}

fn resolve_ty_path(cx: &CompileCx, path: &Rc<LowerPath>, target: RefTargetHost) -> TokenStream {
    match target {
        RefTargetHost::Struct(struct_) => {
            let mut ts = TokenStream::new();
            resolve_to_path(&struct_.get_self_path(), &mut ts);
            ts
        }
        RefTargetHost::NewtypeStruct(newtype_struct) => {
            let mut ts = TokenStream::new();
            resolve_to_path(&newtype_struct.get_self_path(), &mut ts);
            ts
        }
        RefTargetHost::Enum(enum_) => {
            let mut ts = TokenStream::new();
            resolve_to_path(&enum_.get_self_path(), &mut ts);
            ts
        }
        RefTargetHost::Package(_) => {
            panic!("Cannot resolve to a package!");
        }
        RefTargetHost::BuiltinType(builtin) => resolve_builtin_ty(cx, builtin),
    }
}

fn resolve_builtin_ty(cx: &CompileCx, builtin: LowerBuiltinTy) -> TokenStream {
    match builtin {
        LowerBuiltinTy::Bool => {
            quote! {bool}
        }
        LowerBuiltinTy::I8 => {
            quote! {i8}
        }
        LowerBuiltinTy::I16 => {
            quote! {i16}
        }
        LowerBuiltinTy::I32 => {
            quote! {i32}
        }
        LowerBuiltinTy::I64 => {
            quote! {i64}
        }
        LowerBuiltinTy::U8 => {
            quote! {u8}
        }
        LowerBuiltinTy::U16 => {
            quote! {u16}
        }
        LowerBuiltinTy::U32 => {
            quote! {u32}
        }
        LowerBuiltinTy::U64 => {
            quote! {u64}
        }
        LowerBuiltinTy::F32 => {
            quote! {f32}
        }
        LowerBuiltinTy::F64 => {
            quote! {f64}
        }
        LowerBuiltinTy::Char => {
            quote! {char}
        }
        LowerBuiltinTy::Str => {
            quote! {std::string::String}
        }
        LowerBuiltinTy::UUID => {
            quote! {uuid::Uuid}
        }
        LowerBuiltinTy::Bytes => {
            quote! {std::vec::Vec<u8>}
        }
    }
}

fn compile_newtype_struct(cx: &CompileCx, newtype_struct: &Rc<LowerNewtypeStruct>) -> TokenStream {
    let target = newtype_struct.raw_ty.path_resolved_to();

    let name = format_ident!("{}", newtype_struct.name.as_str());
    let ty = resolve_ty_path(cx, newtype_struct.get_self_path().unwrap_parent(), target);

    quote! {
        #[derive(serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct #name(pub #ty);
    }
}

fn compile_struct(cx: &CompileCx, struct_: &Rc<LowerStruct>) -> TokenStream {
    struct StructField<'a>(&'a CompileCx<'a>, &'a LowerStruct, &'a LowerStructField);

    impl<'a> ToTokens for StructField<'a> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let name = format_ident!("{}", self.2.name.as_str());
            let ty = resolve_ty_path(
                self.0,
                &self.1.get_self_path().unwrap_parent(),
                self.2.ty.path_resolved_to(),
            );

            tokens.append_all(quote! {
                pub #name: #ty
            });
        }
    }

    let name = format_ident!("{}", struct_.name.as_str());
    let fields = struct_
        .fields
        .iter()
        .map(|field| StructField(cx, struct_, field));

    quote! {
        #[derive(serde::Serialize, serde::Deserialize)]
        pub struct #name {
            #(#fields,)*
        }
    }
}

fn compile_enum(cx: &CompileCx, enum_: &Rc<LowerEnum>) -> TokenStream {
    struct EnumVariant<'a>(&'a CompileCx<'a>, &'a LowerEnum, &'a LowerEnumVariant);

    impl<'a> ToTokens for EnumVariant<'a> {
        fn to_tokens(&self, tokens: &mut TokenStream) {
            let name = format_ident!("{}", self.2.name.as_str());
            let ty = resolve_ty_path(
                self.0,
                &self.1.get_self_path().unwrap_parent(),
                self.2.ty.path_resolved_to(),
            );

            tokens.append_all(quote! {
                #name(#ty)
            });
        }
    }

    let name = format_ident!("{}", enum_.name.as_str());
    let variants = enum_
        .variants
        .iter()
        .map(|variant| EnumVariant(cx, enum_, variant));

    quote! {
        #[derive(serde::Serialize, serde::Deserialize)]
        pub enum #name {
            #(#variants,)*
        }
    }
}
