use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use crate::ast::*;
use crate::compile::LowerPathHolder;
use crate::keywords::*;
use crate::punct::*;
use crate::span::{Span, SpanHosts, TextSize};

pub struct LowerFile {
    pub packages: Vec<Rc<LowerPackage>>,

    pub(crate) span_hosts: Rc<SpanHosts>,
    pub(crate) references: Rc<RefCell<References>>,
}

impl LowerFile {
    pub fn lower(file: AstFile) -> LowerFile {
        let references = Rc::new(RefCell::new(References::new()));

        LowerFile {
            packages: file
                .packages
                .into_iter()
                .map(|package| LowerPackage::lower(package, &references))
                .collect(),
            span_hosts: file.span_hosts,
            references,
        }
    }
}

impl Drop for LowerFile {
    fn drop(&mut self) {
        // clear all refs
        self.references.borrow_mut().refs.clear();

        for package in &self.packages {
            package.clear_refs();
        }

        // now all Rc's should point down in the AST,
        // and there are no cycles.

        // so dropping the file now should drop all the AST nodes.
    }
}

pub struct LowerPackage {
    pub package_kw: PackageKw,
    pub name: Ident,
    pub open_brace: OpenBracePunctToken,
    pub package_items: Vec<LowerPackageItem>,
    pub close_brace: CloseBracePunctToken,
}

impl LowerPackage {
    fn lower(package: Package, references: &Refs) -> Rc<LowerPackage> {
        Rc::new(LowerPackage {
            package_kw: package.package_kw,
            name: package.name,
            open_brace: package.open_brace,
            package_items: package
                .package_items
                .into_iter()
                .map(|item| LowerPackageItem::lower(item, references))
                .collect(),
            close_brace: package.close_brace,
        })
    }

    fn clear_refs(&self) {
        for item in &self.package_items {
            match item {
                LowerPackageItem::NewtypeStruct(newtype_struct) => {
                    newtype_struct.clear_refs();
                }
                LowerPackageItem::Struct(struct_) => {
                    struct_.clear_refs();
                }
                LowerPackageItem::Enum(enum_) => {
                    enum_.clear_refs();
                }
                LowerPackageItem::Package(package) => {
                    package.clear_refs();
                }
            }
        }
    }
}

pub enum LowerPackageItem {
    NewtypeStruct(Rc<LowerNewtypeStruct>),
    Struct(Rc<LowerStruct>),
    Enum(Rc<LowerEnum>),
    Package(Rc<LowerPackage>),
}

impl LowerPackageItem {
    fn lower(package_item: PackageItem, references: &Refs) -> LowerPackageItem {
        match package_item {
            PackageItem::NewtypeStruct(newtype_struct) => LowerPackageItem::NewtypeStruct(
                LowerNewtypeStruct::lower(newtype_struct, references),
            ),
            PackageItem::Struct(struct_) => {
                LowerPackageItem::Struct(LowerStruct::lower(struct_, references))
            }
            PackageItem::Enum(enum_) => LowerPackageItem::Enum(LowerEnum::lower(enum_, references)),
            PackageItem::Package(package) => {
                LowerPackageItem::Package(LowerPackage::lower(package, references))
            }
        }
    }
}

pub struct LowerNewtypeStruct {
    pub attrs: Attrs,
    pub newtype_kw: NewtypeKw,
    pub open_angle_bracket: OpenAngleBracketPunctToken,
    pub raw_ty: LowerTyRef,
    pub close_angle_bracket: CloseAngleBracketPunctToken,
    pub struct_kw: StructKw,
    pub name: Ident,
    pub semicolon: SemicolonPunctToken,
}

impl LowerNewtypeStruct {
    fn lower(newtype_struct: NewtypeStruct, references: &Refs) -> Rc<LowerNewtypeStruct> {
        Rc::new(LowerNewtypeStruct {
            attrs: newtype_struct.attrs,
            newtype_kw: newtype_struct.newtype_kw,
            open_angle_bracket: newtype_struct.open_angle_bracket,
            raw_ty: LowerTyRef::lower(newtype_struct.raw_ty, references),
            close_angle_bracket: newtype_struct.close_angle_bracket,
            struct_kw: newtype_struct.struct_kw,
            name: newtype_struct.name,
            semicolon: newtype_struct.semicolon,
        })
    }

    fn clear_refs(&self) {
        self.raw_ty.clear_refs();
    }
}

pub struct LowerStruct {
    pub attrs: Attrs,
    pub struct_kw: StructKw,
    pub name: Ident,
    pub open_brace: OpenBracePunctToken,
    pub fields: Vec<LowerStructField>,
    pub close_brace: CloseBracePunctToken,
}

impl LowerStruct {
    fn lower(struct_: Struct, references: &Refs) -> Rc<LowerStruct> {
        Rc::new(LowerStruct {
            attrs: struct_.attrs,
            struct_kw: struct_.struct_kw,
            name: struct_.name,
            open_brace: struct_.open_brace,
            fields: struct_
                .fields
                .into_iter()
                .map(|field| LowerStructField::lower(field, references))
                .collect(),
            close_brace: struct_.close_brace,
        })
    }

    fn clear_refs(&self) {
        for field in &self.fields {
            field.clear_refs();
        }
    }
}

pub struct LowerStructField {
    pub attrs: Attrs,
    pub name: Ident,
    pub qmark: Option<QMarkPunctToken>,
    pub colon: ColonPunctToken,
    pub ty: LowerTyRef,
}

impl LowerStructField {
    fn lower(struct_field: StructField, references: &Refs) -> LowerStructField {
        LowerStructField {
            attrs: struct_field.attrs,
            name: struct_field.name,
            qmark: struct_field.qmark,
            colon: struct_field.colon,
            ty: LowerTyRef::lower(struct_field.ty, references),
        }
    }

    fn clear_refs(&self) {
        self.ty.clear_refs();
    }
}

pub struct LowerEnum {
    pub attrs: Attrs,
    pub enum_kw: EnumKw,
    pub name: Ident,
    pub open_brace: OpenBracePunctToken,
    pub variants: Vec<LowerEnumVariant>,
    pub close_brace: CloseBracePunctToken,
}

impl LowerEnum {
    fn lower(enum_: Enum, references: &Refs) -> Rc<LowerEnum> {
        Rc::new(LowerEnum {
            attrs: enum_.attrs,
            enum_kw: enum_.enum_kw,
            name: enum_.name,
            open_brace: enum_.open_brace,
            variants: enum_
                .variants
                .into_iter()
                .map(|variant| LowerEnumVariant::lower(variant, references))
                .collect(),
            close_brace: enum_.close_brace,
        })
    }

    fn clear_refs(&self) {
        for variant in &self.variants {
            variant.clear_refs();
        }
    }
}

pub struct LowerEnumVariant {
    pub(crate) attrs: Attrs,
    pub(crate) name: Ident,
    pub(crate) colon: ColonPunctToken,
    pub(crate) ty: LowerTyRef,
}

impl LowerEnumVariant {
    fn lower(enum_variant: EnumVariant, references: &Refs) -> LowerEnumVariant {
        LowerEnumVariant {
            attrs: enum_variant.attrs,
            name: enum_variant.name,
            colon: enum_variant.colon,
            ty: LowerTyRef::lower(enum_variant.ty, references),
        }
    }

    fn clear_refs(&self) {
        self.ty.clear_refs();
    }
}

pub struct LowerTyRef {
    pub path: Rc<LowerPath>,
    pub generics: Option<LowerTyGenerics>,

    pub(crate) path_ref: Ref,
}

impl LowerTyRef {
    fn lower(ty: TyRef, references: &Refs) -> LowerTyRef {
        let path = LowerPath::lower(ty.path, references);

        LowerTyRef {
            path_ref: Ref::new_path(Rc::clone(&path), references),
            path,
            generics: ty
                .generics
                .map(|generics| LowerTyGenerics::lower(generics, references)),
        }
    }

    fn clear_refs(&self) {
        *self.path_ref.resolved_to.borrow_mut() = None;

        if let Some(generics) = &self.generics {
            generics.clear_refs();
        }
    }

    pub fn path_resolved_to(&self) -> RefTargetHost {
        self.path_ref
            .resolved_to
            .borrow()
            .as_ref()
            .cloned()
            .expect("path should have been resolved by now")
    }
}

pub struct LowerTyGenerics {
    pub open_angle_bracket: OpenAngleBracketPunctToken,
    pub ty_params: Vec<LowerTyRef>,
    pub close_angle_bracket: CloseAngleBracketPunctToken,
}

impl LowerTyGenerics {
    fn lower(ty_generics: TyGenerics, references: &Refs) -> LowerTyGenerics {
        LowerTyGenerics {
            open_angle_bracket: ty_generics.open_angle_bracket,
            ty_params: ty_generics
                .ty_params
                .into_iter()
                .map(|ty| LowerTyRef::lower(ty, references))
                .collect(),
            close_angle_bracket: ty_generics.close_angle_bracket,
        }
    }

    fn clear_refs(&self) {
        for ty in &self.ty_params {
            ty.clear_refs();
        }
    }
}

#[derive(Clone)]
pub enum RefTargetHost {
    Struct(Rc<LowerStruct>),
    NewtypeStruct(Rc<LowerNewtypeStruct>),
    Enum(Rc<LowerEnum>),
    BuiltinType(LowerBuiltinTy),
    Package(Rc<LowerPackage>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum LowerBuiltinTy {
    Bool,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Char,
    Str,
    UUID,
    Bytes,
}

impl LowerBuiltinTy {
    pub fn name(&self) -> &'static str {
        match self {
            LowerBuiltinTy::Bool => "bool",
            LowerBuiltinTy::I8 => "i8",
            LowerBuiltinTy::I16 => "i16",
            LowerBuiltinTy::I32 => "i32",
            LowerBuiltinTy::I64 => "i64",
            LowerBuiltinTy::U8 => "u8",
            LowerBuiltinTy::U16 => "u16",
            LowerBuiltinTy::U32 => "u32",
            LowerBuiltinTy::U64 => "u64",
            LowerBuiltinTy::F32 => "f32",
            LowerBuiltinTy::F64 => "f64",
            LowerBuiltinTy::Char => "char",
            LowerBuiltinTy::Str => "str",
            LowerBuiltinTy::UUID => "uuid",
            LowerBuiltinTy::Bytes => "bytes",
        }
    }

    pub fn for_name(s: &str) -> Option<Self> {
        let builtin = match s {
            "bool" | "boolean" => LowerBuiltinTy::Bool,
            "i8" => LowerBuiltinTy::I8,
            "i16" => LowerBuiltinTy::I16,
            "i32" | "int" => LowerBuiltinTy::I32,
            "i64" => LowerBuiltinTy::I64,
            "u8" => LowerBuiltinTy::U8,
            "u16" => LowerBuiltinTy::U16,
            "u32" | "uint" => LowerBuiltinTy::U32,
            "u64" => LowerBuiltinTy::U64,
            "f32" | "float" => LowerBuiltinTy::F32,
            "f64" | "double" => LowerBuiltinTy::F64,
            "char" => LowerBuiltinTy::Char,
            "str" | "string" | "String" => LowerBuiltinTy::Str,
            "uuid" | "UUID" => LowerBuiltinTy::UUID,
            "bytes" | "Bytes" => LowerBuiltinTy::Bytes,
            _ => return None,
        };

        Some(builtin)
    }
}

impl RefTargetHost {
    pub(crate) fn name_span(&self) -> &Span {
        match self {
            RefTargetHost::Struct(struct_) => struct_.name.span(),
            RefTargetHost::NewtypeStruct(newtype_struct) => newtype_struct.name.span(),
            RefTargetHost::Enum(enum_) => enum_.name.span(),
            RefTargetHost::Package(package) => package.name.span(),
            RefTargetHost::BuiltinType(_) => {
                panic!("Builtin types don't have physical declarations")
            }
        }
    }
}

enum RefKind {
    TyPath(Rc<LowerPath>),
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq)]
pub enum LowerPath {
    Ident(Ident),
    Path(Rc<LowerPath>, DotPunctToken, Ident),
}

impl fmt::Display for LowerPath {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LowerPath::Ident(ident) => write!(f, "{}", ident),
            LowerPath::Path(path, _, ident) => write!(f, "{}.{}", path, ident),
        }
    }
}

impl LowerPath {
    pub(crate) fn span(&self) -> Span {
        match self {
            LowerPath::Ident(ident) => ident.span().clone(),
            LowerPath::Path(path, _, ident) => path.span().join(ident.span()),
        }
    }

    pub(crate) fn join(self: &Rc<Self>, other: &Rc<LowerPath>) -> LowerPath {
        fn dot(span_hosts: &Rc<SpanHosts>) -> DotPunctToken {
            Span::new(TextSize::ZERO, TextSize::ZERO, Rc::clone(span_hosts)).into()
        }

        match other.get() {
            LowerPath::Ident(ident) => LowerPath::Path(
                Rc::clone(self),
                dot(&ident.span().span_hosts),
                ident.clone(),
            ),
            LowerPath::Path(path, _, ident) => LowerPath::Path(
                Rc::new(self.join(path)),
                dot(&ident.span().span_hosts),
                ident.clone(),
            ),
        }
    }

    pub(crate) fn unwrap_parent(&self) -> &Rc<LowerPath> {
        match self {
            LowerPath::Ident(_) => panic!("unwrap_parent called on Ident"),
            LowerPath::Path(path, _, _) => path,
        }
    }

    fn lower(ty_path: Path, references: &Refs) -> Rc<LowerPath> {
        Rc::new(match ty_path {
            Path::Ident(ident) => LowerPath::Ident(ident),
            Path::Path(path, dot, ident) => {
                LowerPath::Path(LowerPath::lower(*path, references), dot, ident)
            }
        })
    }
}

pub(crate) struct Ref {
    ref_kind: RefKind,
    references: Refs,

    resolved_to: RefCell<Option<RefTargetHost>>,
}

impl Ref {
    fn new_path(path: Rc<LowerPath>, references: &Refs) -> Ref {
        Ref {
            ref_kind: RefKind::TyPath(path),
            references: Refs::clone(references),

            resolved_to: RefCell::new(None),
        }
    }

    pub(crate) fn resolved_to(&self, resolved_to: RefTargetHost) {
        *self.resolved_to.borrow_mut() = Some(resolved_to);
    }
}

type Refs = Rc<RefCell<References>>;

impl Ref {
    pub fn get(&self) -> Option<RefTargetHost> {
        match &self.ref_kind {
            RefKind::TyPath(r) => self.references.borrow().refs.get(&**r).cloned(),
        }
    }
}

pub(crate) struct References {
    refs: BTreeMap<LowerPath, RefTargetHost>,
}

impl References {
    fn new() -> References {
        References {
            refs: BTreeMap::new(),
        }
    }

    pub(crate) fn push_new_ref(&mut self, path: LowerPath, target: RefTargetHost) {
        self.refs.insert(path, target);
    }

    pub(crate) fn get(&self, path: &LowerPath) -> Option<&RefTargetHost> {
        self.refs.get(path)
    }
}
