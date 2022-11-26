use std::fmt;
use crate::keywords::{EnumKw, NewtypeKw, PackageKw, StructKw};
use crate::punct::{
    CloseAngleBracketPunctToken, CloseBracePunctToken, CloseBracketPunctToken,
    CloseParenPunctToken, ColonPunctToken, DotPunctToken, EqPunctToken, HashPunctToken,
    OpenAngleBracketPunctToken, OpenBracePunctToken, OpenBracketPunctToken, OpenParenPunctToken,
    QMarkPunctToken, SemicolonPunctToken,
};
use crate::span::{Span, SpanHosts, Spanned};
use std::rc::Rc;

#[derive(Debug, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct Ident(Spanned<String>);

impl Ident {
    pub fn span(&self) -> &Span {
        self.0.span()
    }

    pub(crate) fn new(v: String, span: Span) -> Self {
        Self(Spanned::new(v, span))
    }
}

impl fmt::Display for Ident {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[non_exhaustive]
pub struct AstFile {
    pub packages: Vec<Package>,

    pub(crate) span_hosts: Rc<SpanHosts>,
}

impl AstFile {
    pub(crate) fn new(packages: Vec<Package>, span_hosts: Rc<SpanHosts>) -> Self {
        Self {
            packages,
            span_hosts,
        }
    }
}

#[non_exhaustive]
pub enum Path {
    Ident(Ident),
    Path(Box<Path>, DotPunctToken, Ident),
}

impl Path {
    pub(crate) fn new_ident(ident: Ident) -> Self {
        Self::Ident(ident)
    }

    pub(crate) fn new_path(path: Path, dot: DotPunctToken, ident: Ident) -> Self {
        Self::Path(Box::new(path), dot, ident)
    }
}

#[non_exhaustive]
pub struct TyRef {
    pub path: Path,
    pub generics: Option<TyGenerics>,
}

impl TyRef {
    pub(crate) fn new(path: Path, generics: Option<TyGenerics>) -> Self {
        Self { path, generics }
    }
}

#[non_exhaustive]
pub struct TyGenerics {
    pub open_angle_bracket: OpenAngleBracketPunctToken,

    pub ty_params: Vec<TyRef>,

    pub close_angle_bracket: CloseAngleBracketPunctToken,
}

impl TyGenerics {
    pub(crate) fn new(
        open_angle_bracket: OpenAngleBracketPunctToken,
        ty_params: Vec<TyRef>,
        close_angle_bracket: CloseAngleBracketPunctToken,
    ) -> Self {
        Self {
            open_angle_bracket,
            ty_params,
            close_angle_bracket,
        }
    }
}

#[non_exhaustive]
pub struct StructField {
    pub attrs: Attrs,
    pub name: Ident,
    pub qmark: Option<QMarkPunctToken>,
    pub colon: ColonPunctToken,
    pub ty: TyRef,
}

impl StructField {
    pub(crate) fn new(
        attrs: Attrs,
        name: Ident,
        qmark: Option<QMarkPunctToken>,
        colon: ColonPunctToken,
        ty: TyRef,
    ) -> Self {
        Self {
            attrs,
            name,
            qmark,
            colon,
            ty,
        }
    }
}

pub struct Attrs(Vec<Attr>);

impl Attrs {
    pub(crate) fn new(attrs: Vec<Attr>) -> Self {
        Self(attrs)
    }
}

#[non_exhaustive]
pub struct Struct {
    pub attrs: Attrs,
    pub struct_kw: StructKw,
    pub name: Ident,
    pub open_brace: OpenBracePunctToken,
    pub fields: Vec<StructField>,
    pub close_brace: CloseBracePunctToken,
}

impl Struct {
    pub(crate) fn new(
        attrs: Attrs,
        struct_kw: StructKw,
        name: Ident,
        open_brace: OpenBracePunctToken,
        fields: Vec<StructField>,
        close_brace: CloseBracePunctToken,
    ) -> Self {
        Self {
            attrs,
            struct_kw,
            name,
            open_brace,
            fields,
            close_brace,
        }
    }
}

#[non_exhaustive]
pub struct EnumVariant {
    pub attrs: Attrs,
    pub name: Ident,
    pub colon: ColonPunctToken,
    pub ty: TyRef,
}

impl EnumVariant {
    pub(crate) fn new(attrs: Attrs, name: Ident, colon: ColonPunctToken, ty: TyRef) -> Self {
        Self {
            attrs,
            name,
            colon,
            ty,
        }
    }
}

#[non_exhaustive]
pub struct Enum {
    pub attrs: Attrs,
    pub enum_kw: EnumKw,
    pub name: Ident,
    pub open_brace: OpenBracePunctToken,
    pub variants: Vec<EnumVariant>,
    pub close_brace: CloseBracePunctToken,
}

impl Enum {
    pub(crate) fn new(
        attrs: Attrs,
        enum_kw: EnumKw,
        name: Ident,
        open_brace: OpenBracePunctToken,
        variants: Vec<EnumVariant>,
        close_brace: CloseBracePunctToken,
    ) -> Self {
        Self {
            attrs,
            enum_kw,
            name,
            open_brace,
            variants,
            close_brace,
        }
    }
}

#[non_exhaustive]
pub struct NewtypeStruct {
    pub attrs: Attrs,
    pub newtype_kw: NewtypeKw,
    pub open_angle_bracket: OpenAngleBracketPunctToken,
    pub raw_ty: TyRef,
    pub close_angle_bracket: CloseAngleBracketPunctToken,
    pub struct_kw: StructKw,
    pub name: Ident,
    pub semicolon: SemicolonPunctToken,
}

impl NewtypeStruct {
    pub(crate) fn new(
        attrs: Attrs,
        newtype_kw: NewtypeKw,
        open_angle_bracket: OpenAngleBracketPunctToken,
        raw_ty: TyRef,
        close_angle_bracket: CloseAngleBracketPunctToken,
        struct_kw: StructKw,
        name: Ident,
        semicolon: SemicolonPunctToken,
    ) -> Self {
        Self {
            attrs,
            newtype_kw,
            open_angle_bracket,
            raw_ty,
            close_angle_bracket,
            struct_kw,
            name,
            semicolon,
        }
    }
}

pub enum LiteralValue {
    String(String),
    Integer(i64),
}

#[non_exhaustive]
pub struct Literal {
    pub value: LiteralValue,
    span: Span,
}

impl Literal {
    pub fn span(&self) -> &Span {
        &self.span
    }

    pub fn value(&self) -> &LiteralValue {
        &self.value
    }

    pub(crate) fn new(value: LiteralValue, span: Span) -> Self {
        Self { value, span }
    }
}

#[non_exhaustive]
pub enum AttrMeta {
    Parenthesized(
        Ident,
        OpenParenPunctToken,
        Vec<AttrMeta>,
        CloseParenPunctToken,
    ),
    NameValue(Ident, EqPunctToken, Literal),
    Ident(Ident),
}

#[non_exhaustive]
pub struct Attr {
    pub hash: HashPunctToken,
    pub open_bracket: OpenBracketPunctToken,
    pub meta: AttrMeta,
    pub close_bracket: CloseBracketPunctToken,
}

impl Attr {
    pub(crate) fn new(
        hash: HashPunctToken,
        open_bracket: OpenBracketPunctToken,
        meta: AttrMeta,
        close_bracket: CloseBracketPunctToken,
    ) -> Self {
        Self {
            hash,
            open_bracket,
            meta,
            close_bracket,
        }
    }
}

pub enum PackageItem {
    Struct(Struct),
    NewtypeStruct(NewtypeStruct),
    Enum(Enum),
    Package(Package),
}

#[non_exhaustive]
pub struct Package {
    pub package_kw: PackageKw,
    pub name: Ident,
    pub open_brace: OpenBracePunctToken,
    pub package_items: Vec<PackageItem>,
    pub close_brace: CloseBracePunctToken,
}

impl Package {
    pub(crate) fn new(
        package_kw: PackageKw,
        name: Ident,
        open_brace: OpenBracePunctToken,
        package_items: Vec<PackageItem>,
        close_brace: CloseBracePunctToken,
    ) -> Self {
        Self {
            package_kw,
            name,
            open_brace,
            package_items,
            close_brace,
        }
    }
}
