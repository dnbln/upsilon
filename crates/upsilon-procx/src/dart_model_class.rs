use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote, TokenStreamExt};
use std::fmt;
use std::fmt::Write;
use std::path::PathBuf;
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Field, Fields, GenericArgument, PathArguments, Type,
};

pub fn derive_dart_model_class(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = syn::parse_macro_input!(item as DeriveInput);

    let dart_model_impl = match &input.data {
        Data::Struct(s) => dart_model_struct(&input, s),
        Data::Enum(e) => dart_model_enum(&input, e),
        Data::Union(_) => {
            panic!("Unions not supported")
        }
    };

    proc_macro::TokenStream::from(dart_model_impl)
}

fn dart_model_struct(derive_input: &DeriveInput, s: &DataStruct) -> TokenStream {
    let struct_name = &derive_input.ident;

    let result_ts = match &s.fields {
        Fields::Named(named) => {
            let mut constructor = String::new();
            let mut fields = String::new();
            let mut from_json = String::new();
            let mut to_json = String::new();

            named.named.iter().for_each(|it: &Field| {
                let field_name = it.ident.as_ref().expect("Missing ident");
                let dart_name = field_name.to_string();

                constructor.push_str("this.");
                constructor.push_str(&dart_name.to_string());
                constructor.push_str(", ");

                write!(&mut fields, "{} {dart_name};\n", &DartTyFmt(&it.ty)).unwrap();

                write!(
                    &mut from_json,
                    "{},\n",
                    DartTyDecode {
                        ty: &it.ty,
                        expr: &format!("json['{field_name}']")
                    }
                )
                .unwrap();

                write!(
                    &mut to_json,
                    "'{field_name}': {},\n",
                    DartTyEncode {
                        ty: &it.ty,
                        expr: &format!("{dart_name}")
                    }
                )
                .unwrap();
            });

            let constructor = constructor.trim_end_matches(", ");

            let span = struct_name.span().unwrap().source();
            let file = span.source_file();
            debug_assert!(file.is_real());

            let path = file.path();
            let path = path.display();
            let start = span.start();
            let line = start.line;
            let column = start.column;

            let class = format!(
                "\
class {struct_name} {{
    // generated from struct {struct_name}, at {path} :{line}:{column}
    {struct_name}({constructor});

{fields}

    factory {struct_name}.fromJson(Map<String, dynamic> json) => {struct_name}(
{from_json}
    );

    Map<String, dynamic> toJson() => {{
{to_json}
    }};
}}
",
                fields = fields.indent(4),
                from_json = from_json.indent(8),
                to_json = to_json.indent(8),
            );

            let mut result = quote! {
                impl #struct_name {
                    pub fn get_dart_model_class() -> &'static str {
                        #class
                    }
                }
            };

            result.append_all(link(struct_name));

            result
        }
        Fields::Unnamed(tuple) => {
            const NAMES: &[&str] = &[
                "first",
                "second",
                // "third",
                // "fourth",
                // "fifth",
            ];

            // struct X(String, [String; 10]);

            // to

            // class X {
            //     X(this.first, this.second);
            //
            //     final String first;
            //     final List<String> second;
            //
            //     factory X.fromJson(Iterable json) => X(
            //         json.elementAt(0) as String,
            //         List<String>.from((json.elementAt(1) as Iterable).map((d) => d as String)),
            //     );
            //
            //     Iterable<dynamic> toJson() => [first, second];
            // }

            let mut constructor = String::new();
            let mut fields = String::new();
            let mut from_json = String::new();
            let mut to_json = String::new();

            tuple
                .unnamed
                .iter()
                .enumerate()
                .for_each(|(idx, it): (usize, &Field)| {
                    let dart_name = NAMES
                        .get(idx)
                        .expect("too many unnamed fields, just convert to a proper tuple! dart code becomes awkward otherwise");

                    constructor.push_str("this.");
                    constructor.push_str(dart_name);
                    constructor.push_str(", ");

                    write!(&mut fields, "{} {dart_name};\n", &DartTyFmt(&it.ty)).unwrap();

                    write!(&mut from_json, "{},\n", DartTyDecode {
                        ty: &it.ty,
                        expr: &format!("json.elementAt({idx})")
                    }).unwrap();

                    write!(&mut to_json, "{},\n", DartTyEncode {ty: &it.ty, expr: &format!("{dart_name}")}).unwrap();
                })
            ;

            let constructor = constructor.trim_end_matches(", ");

            let span = struct_name.span().unwrap().source();
            let file = span.source_file();
            debug_assert!(file.is_real());

            let path = file.path();
            let path = path.display();
            let start = span.start();
            let line = start.line;
            let column = start.column;


            let class = format!(
                "\
class {struct_name} {{
    // generated from struct {struct_name}, at {path} :{line}:{column}
    {struct_name}({constructor});

{fields}

    factory {struct_name}.fromJson(Iterable json) => {struct_name}(
{from_json}
    );

    Iterable toJson() => [
{to_json}
    ];
}}
",
                fields = fields.indent(4),
                from_json = from_json.indent(8),
                to_json = to_json.indent(8),
            );

            let mut result = quote! {
                impl #struct_name {
                    pub fn get_dart_model_class() -> &'static str {
                        #class
                    }
                }
            };

            result.append_all(link(struct_name));

            result
        }
        Fields::Unit => quote! {
            impl #struct_name {
                pub fn get_dart_model_class() -> &'static str {
                    ""
                }
            }
        },
    };

    result_ts
}

fn dart_model_enum(derive_input: &DeriveInput, s: &DataEnum) -> TokenStream {
    s.enum_token
        .span
        .unwrap()
        .error("derive(DartModelClass): Not (yet) supported for enums")
        .emit();

    quote! {}
}

trait Indent {
    fn indent(&self, indentation: usize) -> String;
}

impl Indent for str {
    fn indent(&self, indentation: usize) -> String {
        let mut result = self
            .lines()
            .map(|line| format!("{:indentation$}{}", "", line))
            .join("\n");

        if self.ends_with("\n") {
            result.push_str("\n");
        }

        result
    }
}

fn dart_ty_eq(ty1: &Type, ty2: &Type) -> bool {
    match (ty1, ty2) {
        (Type::Array(a), Type::Array(b)) => dart_ty_eq(&a.elem, &b.elem),
        (Type::Paren(a), Type::Paren(b)) => dart_ty_eq(&a.elem, &b.elem),
        (Type::Paren(a), b) => dart_ty_eq(&a.elem, b),
        (a, Type::Paren(b)) => dart_ty_eq(a, &b.elem),
        (Type::Path(a), Type::Path(b)) => match (a.path.segments.last(), b.path.segments.last()) {
            (Some(x), Some(y)) => {
                if x.ident != y.ident {
                    return false;
                }

                if x.ident == "Option" {
                    let x_inner = opt_ty(&x.arguments);
                    let y_inner = opt_ty(&y.arguments);

                    return dart_ty_eq(x_inner, y_inner);
                } else if x.ident == "Vec" {
                    let x_inner = vec_ty(&x.arguments);
                    let y_inner = vec_ty(&y.arguments);

                    return dart_ty_eq(x_inner, y_inner);
                }

                true
            }
            _ => false,
        },
        (Type::Reference(a), Type::Reference(b)) => dart_ty_eq(&a.elem, &b.elem),
        (Type::Slice(a), Type::Slice(b)) => dart_ty_eq(&a.elem, &b.elem),
        (Type::Tuple(a), Type::Tuple(b)) => {
            a.elems.len() == b.elems.len()
                && a.elems
                    .iter()
                    .zip(b.elems.iter())
                    .all(|(x, y)| dart_ty_eq(x, y))
        }
        (Type::Verbatim(a), Type::Verbatim(b)) => a.to_string() == b.to_string(),
        _ => false,
    }
}

fn ty_if_all_equal<'a, I>(mut iter: I) -> Option<&'a Type>
where
    I: Iterator<Item = &'a Type>,
{
    let Some(first) = iter.next() else {return None};

    for remaining in iter {
        if !dart_ty_eq(first, remaining) {
            return None;
        }
    }

    Some(first)
}

fn opt_ty(args: &PathArguments) -> &Type {
    match &args {
        PathArguments::AngleBracketed(a) => {
            match a.args.first().expect("Missing type argument for Option") {
                GenericArgument::Type(t) => t,
                _ => panic!("Unsupported generic argument for Option"),
            }
        }
        PathArguments::Parenthesized(_) | PathArguments::None => {
            panic!("unsupported for Option")
        }
    }
}

fn vec_ty(args: &PathArguments) -> &Type {
    match &args {
        PathArguments::AngleBracketed(a) => {
            match a.args.first().expect("Missing type argument for Vec") {
                GenericArgument::Type(t) => t,
                _ => panic!("Unsupported generic argument for Vec"),
            }
        }
        PathArguments::Parenthesized(_) | PathArguments::None => {
            panic!("unsupported for Vec")
        }
    }
}

struct DartTyFmt<'a>(&'a Type);

impl<'a> fmt::Display for DartTyFmt<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let ty = self.0;
        match ty {
            Type::Array(t) => {
                write!(f, "List<{}>", DartTyFmt(&t.elem))
            }
            Type::Paren(p) => DartTyFmt(&p.elem).fmt(f),
            Type::Path(p) => {
                let s = p.path.segments.last().expect("Missing segment in path");

                if s.ident == "Option" {
                    let inner_ty = opt_ty(&s.arguments);

                    write!(f, "{}?", DartTyFmt(inner_ty))
                } else if s.ident == "Vec" {
                    let inner_ty = vec_ty(&s.arguments);

                    write!(f, "List<{}>", DartTyFmt(inner_ty))
                } else {
                    write!(f, "{}", s.ident.to_string())
                }
            }
            Type::Reference(r) => DartTyFmt(&r.elem).fmt(f),
            Type::Slice(s) => {
                write!(f, "List<{}>", DartTyFmt(&s.elem))
            }
            Type::Tuple(t) => {
                let all_equal = ty_if_all_equal(t.elems.iter());
                match all_equal {
                    Some(t) => write!(f, "List<{}>", DartTyFmt(t)),
                    None => match t.elems.first() {
                        Some(_) => write!(f, "List<dynamic>"),
                        None => write!(f, "UnitStruct"),
                    },
                }
            }
            _ => {
                panic!("Unsupported ty: {:?}", quote! {#ty})
            }
        }
    }
}

struct DartIterableDecode<'a> {
    ty: &'a Type,
    iterable_expr: &'a str,
}

impl<'a> fmt::Display for DartIterableDecode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { ty, iterable_expr } = *self;

        write!(
            f,
            "List<{t}>.from(({iterable_expr} as Iterable<dynamic>).map((v) => {elem_decode}))",
            t = DartTyFmt(ty),
            elem_decode = DartTyDecode { ty, expr: "v" }
        )
    }
}

struct DartTyDecode<'a> {
    ty: &'a Type,
    expr: &'a str,
}

impl<'a> fmt::Display for DartTyDecode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { ty, expr } = *self;

        match ty {
            Type::Array(t) => {
                write!(
                    f,
                    "{}",
                    DartIterableDecode {
                        ty: &t.elem,
                        iterable_expr: expr,
                    },
                )
            }
            Type::Paren(p) => DartTyDecode { ty: &p.elem, expr }.fmt(f),
            Type::Path(p) => {
                let s = p.path.segments.last().expect("Missing segment in path");

                if s.ident == "Option" {
                    let inner_ty = opt_ty(&s.arguments);
                    write!(
                        f,
                        "\
(_invokeWith({expr}, ((v) {{
    {inner_ty}? result;
    if (v != null) {{
        result = {inner_val};
    }} else {{
        result = null;
    }}
    return result;
}})))",
                        inner_ty = DartTyFmt(inner_ty),
                        inner_val = DartTyDecode {
                            ty: inner_ty,
                            expr: "v"
                        }
                    )
                } else if s.ident == "Vec" {
                    let inner_ty = vec_ty(&s.arguments);

                    write!(
                        f,
                        "{}",
                        DartIterableDecode {
                            ty: inner_ty,
                            iterable_expr: expr
                        }
                    )
                } else if let Some(standard_ty) = standard(&s.ident) {
                    standard_ty.decode(expr).fmt(f)
                } else {
                    write!(f, "{name}.fromJson({expr})", name = s.ident.to_string())
                }
            }
            Type::Reference(r) => {
                write!(f, "{}", DartTyDecode { expr, ty: &r.elem })
            }
            Type::Slice(s) => {
                write!(
                    f,
                    "{}",
                    DartIterableDecode {
                        ty: &s.elem,
                        iterable_expr: expr
                    }
                )
            }
            Type::Tuple(t) => {
                let all_equal = ty_if_all_equal(t.elems.iter());
                match all_equal {
                    Some(t) => {
                        write!(
                            f,
                            "{}",
                            DartIterableDecode {
                                ty: t,
                                iterable_expr: expr,
                            }
                        )
                    }
                    None => {
                        write!(f, "(((Iterable<dynamic> iterable) => <dynamic>[")?;

                        for (index, t) in t.elems.iter().enumerate() {
                            write!(
                                f,
                                "_invokeWith(iterable.elementAt({index}), ((v) => {elem_decode})), ",
                                elem_decode = DartTyDecode { ty: t, expr: "v" }
                            )?;
                        }

                        write!(f, "])({expr}))")
                    }
                }
            }
            _ => {
                panic!("Unsupported ty: {:?}", quote! {#ty})
            }
        }
    }
}

struct DartIterableEncode<'a> {
    ty: &'a Type,
    expr: &'a str,
}

impl<'a> fmt::Display for DartIterableEncode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { ty, expr } = *self;

        write!(
            f,
            "({expr}).map((v) => {ty_encode})",
            ty_encode = DartTyEncode { ty, expr: "v" }
        )
    }
}

struct DartTyEncode<'a> {
    ty: &'a Type,
    expr: &'a str,
}

impl<'a> fmt::Display for DartTyEncode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { ty, expr } = *self;

        match ty {
            Type::Array(t) => {
                write!(f, "{}", DartIterableEncode { ty: &t.elem, expr })
            }
            Type::Paren(p) => DartTyEncode { ty: &p.elem, expr }.fmt(f),
            Type::Path(p) => {
                let s = p.path.segments.last().expect("Missing segment in path");

                if s.ident == "Option" {
                    let inner_ty = opt_ty(&s.arguments);
                    write!(
                        f,
                        "\
(_invokeWith({expr}, ((v) {{
    dynamic result;
    if (v != null) {{
        result = {inner_val_encode};
    }} else {{
        result = null;
    }}
    return result;
}})))",
                        inner_val_encode = DartTyEncode {
                            ty: inner_ty,
                            expr: "v"
                        }
                    )
                } else if s.ident == "Vec" {
                    let inner_ty = vec_ty(&s.arguments);

                    write!(f, "{}", DartIterableEncode { ty: inner_ty, expr })
                } else if let Some(standard_ty) = standard(&s.ident) {
                    standard_ty.encode(expr).fmt(f)
                } else {
                    write!(f, "({expr}).toJson()")
                }
            }
            Type::Reference(r) => {
                write!(f, "{}", DartTyEncode { expr, ty: &r.elem })
            }
            Type::Slice(s) => {
                write!(f, "{}", DartIterableEncode { ty: &s.elem, expr })
            }
            Type::Tuple(t) => {
                let all_equal = ty_if_all_equal(t.elems.iter());
                match all_equal {
                    Some(t) => {
                        write!(f, "{}", DartIterableEncode { ty: t, expr })
                    }
                    None => {
                        write!(f, "(((Iterable<dynamic> iterable) => <dynamic>[")?;

                        for (index, t) in t.elems.iter().enumerate() {
                            write!(
                                f,
                                "_invokeWith(iterable.elementAt({index}), ((v) => {elem_decode})), ",
                                elem_decode = DartTyDecode { ty: t, expr: "v" }
                            )?;
                        }

                        write!(f, "])({expr}))")
                    }
                }
            }
            _ => {
                panic!("Unsupported ty: {:?}", quote! {#ty})
            }
        }
    }
}

fn standard(name: &Ident) -> Option<StandardTy> {
    let ty = match name.to_string().as_str() {
        "i8" | "u8" | "i16" | "u16" | "i32" | "i64" | "i128" | "u128" | "isize" | "usize" => {
            StandardTy::Int
        }
        "f32" | "f64" => StandardTy::Float,
        "String" | "str" => StandardTy::String,
        _ => return None,
    };

    Some(ty)
}

#[derive(Copy, Clone)]
enum StandardTy {
    Int,
    Float,
    String,
}

impl StandardTy {
    fn decode(self, expr: &str) -> StandardTyDecode {
        StandardTyDecode { ty: self, expr }
    }

    fn encode(self, expr: &str) -> StandardTyEncode {
        StandardTyEncode { ty: self, expr }
    }
}

struct StandardTyDecode<'a> {
    ty: StandardTy,
    expr: &'a str,
}

impl<'a> fmt::Display for StandardTyDecode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { ty, expr } = *self;

        match ty {
            StandardTy::Int => {
                write!(f, "(({expr}) as int)")
            }
            StandardTy::Float => {
                write!(f, "(({expr}) as double)")
            }
            StandardTy::String => {
                write!(f, "(({expr}) as String)")
            }
        }
    }
}

struct StandardTyEncode<'a> {
    ty: StandardTy,
    expr: &'a str,
}

impl<'a> fmt::Display for StandardTyEncode<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let Self { ty: _, expr } = *self; // type doesn't matter, always cast to `dynamic`

        write!(f, "(({expr}) as dynamic)")
    }
}

fn classes_holder_name() -> Ident {
    format_ident!("____DART_MODEL_CLASSES_HOLDER")
}

fn link(name: &Ident) -> TokenStream {
    let name_str = name.to_string();
    let fn_name = format_ident!("__dart_model_class_{}", name);
    let classes_holder = classes_holder_name();

    quote! {
        #[::linkme::distributed_slice(crate:: #classes_holder)]
        #[allow(non_snake_case)]
        fn #fn_name() -> (&'static str, &'static str) {
            (#name_str, #name::get_dart_model_class())
        }
    }
}

pub fn dart_model_classes(_item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let classes_holder = classes_holder_name();

    let utils = r#"
T _invokeWith<T, V>(V v, T Function(V) f) => f(v);


"#;

    proc_macro::TokenStream::from(quote! {
        #[::linkme::distributed_slice]
        pub static #classes_holder: [fn() -> (&'static str, &'static str)] = [..];

        mod ____dart_model_class_holder_root_check {
            use crate::#classes_holder;
        }

        #[::linkme::distributed_slice(#classes_holder)]
        fn ____dart_model_class_holder_utils() -> (&'static str, &'static str) {
            ("____DART_MODEL_CLASS_HOLDER_UTILS", #utils)
        }
    })
}

pub fn dart_model_classes_iter(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let path = syn::parse_macro_input!(item as syn::Path);

    let classes_holder = classes_holder_name();

    proc_macro::TokenStream::from(quote! {(#path :: #classes_holder).iter()})
}
