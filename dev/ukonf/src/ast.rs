/*
 *        Copyright (c) 2023 Dinu Blanovschi
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

use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct FileId(pub(crate) usize);

impl fmt::Debug for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FileId({})", self.0)
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}f", self.0)
    }
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct PhysicalSpan {
    start: usize,
    end: usize,
    file_id: FileId,
}

impl fmt::Debug for PhysicalSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "physical({}..{}, {})",
            self.start, self.end, self.file_id
        )
    }
}

impl fmt::Display for PhysicalSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}(@{})", self.start, self.end, self.file_id)
    }
}

#[derive(Clone)]
pub struct InternedString {
    id: usize,
    interner: Rc<RefCell<StringInterner>>,
}

impl PartialEq for InternedString {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for InternedString {}

impl InternedString {
    pub fn with_inner<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&String) -> T,
    {
        let s = &self.interner.borrow().strings[self.id];
        f(s)
    }
}

impl fmt::Debug for InternedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_inner(|it| fmt::Debug::fmt(it, f))
    }
}

impl fmt::Display for InternedString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_inner(|it| fmt::Display::fmt(it, f))
    }
}

pub struct StringInterner {
    strings: Vec<String>,
}

impl Default for StringInterner {
    fn default() -> Self {
        Self::new()
    }
}

impl StringInterner {
    pub fn new() -> Self {
        Self {
            strings: Vec::new(),
        }
    }

    pub fn intern(interner: &Rc<RefCell<Self>>, string: String) -> InternedString {
        let mut r = interner.borrow_mut();

        if let Some(pos) = r.strings.iter().position(|s| s == &string) {
            InternedString {
                id: pos,
                interner: Rc::clone(interner),
            }
        } else {
            r.strings.push(string);

            InternedString {
                id: r.strings.len() - 1,
                interner: Rc::clone(interner),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualSpan {
    creator_fun_file: Option<FileId>,
    creator_fun_name: Option<InternedString>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SpanInner {
    Physical(PhysicalSpan),
    Virtual(VirtualSpan),
}

#[derive(Clone, PartialEq, Eq)]
pub struct Span(SpanInner);

impl Span {
    pub fn new(start: usize, end: usize, file_id: FileId) -> Self {
        Self(SpanInner::Physical(PhysicalSpan {
            start,
            end,
            file_id,
        }))
    }

    pub fn new_virtual(virtual_span: VirtualSpan) -> Self {
        Self(SpanInner::Virtual(virtual_span))
    }

    pub fn as_physical(&self) -> Option<PhysicalSpan> {
        match &self.0 {
            SpanInner::Physical(span) => Some(*span),
            SpanInner::Virtual(_) => None,
        }
    }

    pub fn as_virtual(&self) -> Option<&VirtualSpan> {
        match &self.0 {
            SpanInner::Physical(_) => None,
            SpanInner::Virtual(virt) => Some(virt),
        }
    }

    pub fn join_with(&self, other: &Self) -> Self {
        match (&self.0, &other.0) {
            (
                SpanInner::Physical(PhysicalSpan {
                    file_id: file_id1,
                    start: start1,
                    end: end1,
                }),
                SpanInner::Physical(PhysicalSpan {
                    file_id: file_id2,
                    start: start2,
                    end: end2,
                }),
            ) if file_id1 == file_id2 => {
                let start = start1.min(start2);
                let end = end1.max(end2);

                Self::new(*start, *end, *file_id1)
            }
            _ => panic!("Cannot join spans"),
        }
    }

    pub fn spanned<T>(self, v: T) -> Spanned<T> {
        Spanned(v, self)
    }

    pub fn spanned_string<T: Into<String>>(self, v: T) -> Spanned<String> {
        self.spanned(v.into())
    }
}

impl fmt::Debug for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            SpanInner::Physical(span) => write!(f, "{span:?}"),
            SpanInner::Virtual(span) => write!(f, "{span:?}"),
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            SpanInner::Physical(span) => write!(f, "{span}"),
            SpanInner::Virtual(span) => write!(f, "{span:?}"),
        }
    }
}

pub struct AstFile {
    pub imports: Vec<AstImport>,
    pub items: Vec<AstItem>,
}

pub enum AstItem {
    DocPatch(KV),
    Decl(AstDecl),
}

pub struct AstDecl {
    pub let_kw: LetKw,
    pub cx_kw: Option<CxKw>,
    pub name: Ident,
    pub colon: Colon,
    pub value: AstVal,
}

pub trait Punctuation: From<Span> {
    const PUNCT: &'static str;

    fn new(span: Span) -> Self {
        Self::from(span)
    }
}

macro_rules! punct {
    ($name:ident, $punct:literal) => {
        pub struct $name(Span);

        impl Punctuation for $name {
            const PUNCT: &'static str = $punct;
        }

        impl From<Span> for $name {
            fn from(span: Span) -> Self {
                Self(span)
            }
        }
    };
}

punct!(Comma, ",");
punct!(Newline, "\n");
punct!(OpenParen, "(");
punct!(CloseParen, ")");
punct!(OpenBracket, "[");
punct!(CloseBracket, "]");
punct!(OpenBrace, "{");
punct!(CloseBrace, "}");
punct!(DollarBrace, "${");
punct!(Semicolon, ";");
punct!(Colon, ":");
punct!(Dot, ".");

pub trait Keyword: From<Span> {
    const KW: &'static str;
    fn new(span: Span) -> Self {
        Self::from(span)
    }
}

macro_rules! kw {
    ($name:ident, $kw:literal) => {
        pub struct $name(Span);

        impl Keyword for $name {
            const KW: &'static str = $kw;
        }

        impl From<Span> for $name {
            fn from(span: Span) -> Self {
                Self(span)
            }
        }
    };
}

kw!(Import, "import");
kw!(LetKw, "let");
kw!(CxKw, "cx");
kw!(Null, "null");

pub struct AstImport {
    pub import_kw: Import,
    pub path: AstVal,
    pub semicolon: Semicolon,
    resolved_file_id: Rc<RefCell<Option<FileId>>>,
}

impl AstImport {
    pub fn new(import_kw: Import, path: AstVal, semicolon: Semicolon) -> Self {
        Self {
            import_kw,
            path,
            semicolon,
            resolved_file_id: Rc::new(RefCell::new(None)),
        }
    }

    pub fn resolve_to(&self, file_id: FileId) {
        *self.resolved_file_id.borrow_mut() = Some(file_id);
    }
}

pub struct Punctuated<T, S> {
    pub values: Vec<(T, S)>,
    pub trailing_value: Option<Box<T>>,
}

impl<T, S> Punctuated<T, S> {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.values
            .iter()
            .map(|(v, _)| v)
            .chain(self.trailing_value.iter().map(|it| &**it))
    }
}

pub enum NumLit {
    Int(Spanned<i64>),
    Float(Spanned<f64>),
}

pub struct BoolLit {
    pub value: bool,
    pub span: Span,
}

pub enum AstVal {
    Null(Null),
    Ident(Ident),
    Str(StrLit),
    Num(NumLit),
    Bool(BoolLit),
    Arr(OpenBracket, Vec<AstVal>, CloseBracket),
    Obj(OpenBrace, Vec<AstItem>, CloseBrace),
    FunctionCall(AstFunctionCall),
    Dot(Ident, Dot, K),
}

impl AstVal {
    pub fn span(&self) -> Span {
        match self {
            Self::Null(it) => it.0.clone(),
            Self::Ident(it) => it.0 .1.clone(),
            Self::Str(it) => it.span().clone(),
            Self::Num(it) => match it {
                NumLit::Int(it) => it.1.clone(),
                NumLit::Float(it) => it.1.clone(),
            },
            Self::Bool(it) => it.span.clone(),
            Self::Arr(start, _, end) => start.0.join_with(&end.0),
            Self::Obj(start, _, end) => start.0.join_with(&end.0),
            Self::FunctionCall(it) => it.span(),
            Self::Dot(base, _, k) => base.0 .1.join_with(&k.span()),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Spanned<T>(pub(crate) T, pub(crate) Span);

impl<T: fmt::Debug> fmt::Debug for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} @ {:?}", self.0, self.1)
    }
}

impl<T: fmt::Display> fmt::Display for Spanned<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} @ {}", self.0, self.1)
    }
}

impl<T> Spanned<T> {
    fn new(t: T, span: Span) -> Self {
        Self(t, span)
    }

    fn into_inner(self) -> T {
        self.0
    }

    fn span(&self) -> &Span {
        &self.1
    }
}

pub enum StrLit {
    Apostrophe(Spanned<String>),
    Quote(Spanned<String>),
    TripleQuote(Spanned<String>),
}

impl StrLit {
    pub fn str_val(&self) -> Cow<str> {
        match self {
            StrLit::Apostrophe(s) => Cow::Borrowed(&s.0[1..s.0.len() - 1]),
            StrLit::Quote(s) => Cow::Owned(unquote(&s.0[1..s.0.len() - 1])),
            StrLit::TripleQuote(s) => Cow::Owned(patch_triple_quote_string(&s.0[3..s.0.len() - 3])),
        }
    }

    pub fn span(&self) -> &Span {
        match self {
            StrLit::Apostrophe(s) => &s.1,
            StrLit::Quote(s) => &s.1,
            StrLit::TripleQuote(s) => &s.1,
        }
    }
}

fn unquote(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('\'') => out.push('\''),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('u') => {
                    let mut code = String::new();
                    for _ in 0..4 {
                        code.push(chars.next().unwrap());
                    }
                    let code = u32::from_str_radix(&code, 16).unwrap();
                    out.push(std::char::from_u32(code).unwrap());
                }
                Some(c) => panic!("unknown escape sequence: \\{c}"),
                None => panic!("unexpected end of string"),
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(windows)]
fn patch_triple_quote_string(s: &str) -> String {
    s.replace('\r', "")
}

#[cfg(not(windows))]
fn patch_triple_quote_string(s: &str) -> String {
    s.to_string()
}

pub struct Ident(pub(crate) Spanned<String>);

pub enum K {
    Name(Ident),
    StrLit(StrLit),
    Ref(DollarBrace, Box<K>, CloseBrace),
}

impl K {
    pub fn span(&self) -> Span {
        match self {
            K::Name(Ident(s)) => s.span().clone(),
            K::StrLit(s) => s.span().clone(),
            K::Ref(db, _k, cb) => db.0.join_with(&cb.0),
        }
    }
}

pub enum ResolvedKey<'a> {
    KeyValue(Cow<'a, str>),
    Indirection(Box<ResolvedKey<'a>>),
}

impl<'a> ResolvedKey<'a> {
    pub(crate) fn lower(self) -> (Cow<'a, str>, usize) {
        match self {
            ResolvedKey::KeyValue(s) => (s, 0),
            ResolvedKey::Indirection(k) => {
                let (r, ind) = k.lower();
                (r, ind + 1)
            }
        }
    }
}

impl K {
    pub fn key(&self) -> ResolvedKey<'_> {
        match self {
            K::Name(Ident(s)) => ResolvedKey::KeyValue(Cow::Borrowed(&s.0)),
            K::StrLit(s) => ResolvedKey::KeyValue(s.str_val()),
            K::Ref(_, k, _) => ResolvedKey::Indirection(Box::new(k.key())),
        }
    }
}

pub struct KV {
    pub key: Vec<K>,
    pub colon: Option<Colon>,
    pub value: AstVal,
}

pub struct AstFunctionCall {
    pub name: Ident,
    pub open_paren: OpenParen,
    pub args: Punctuated<AstVal, Comma>,
    pub close_paren: CloseParen,
}

impl AstFunctionCall {
    pub fn span(&self) -> Span {
        self.name.0 .1.join_with(&self.close_paren.0)
    }
}
