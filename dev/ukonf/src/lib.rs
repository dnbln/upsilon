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

use std::cell::{Ref, RefCell};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use lalrpop_util::lexer::Token;
use lalrpop_util::{lalrpop_mod, ParseError};

use crate::ast::{AstItem, AstVal, FileId, NumLit, Span, Spanned, VirtualSpan};
use crate::value::{UkonfObject, UkonfValue};

lalrpop_mod!(
    #[allow(clippy::all, clippy::uninlined_format_args)]
    ukonf_parser
);

pub mod ast;

struct ParsedFile {
    path: PathBuf,
    file: String,
    ast: Option<ast::AstFile>,
}

struct Files {
    files: Vec<ParsedFile>,
}

impl Files {
    fn new() -> Self {
        Self { files: Vec::new() }
    }

    fn add_file(
        &mut self,
        path: PathBuf,
        file: String,
        parse_result: Option<ast::AstFile>,
    ) -> FileId {
        self.files.push(ParsedFile {
            path,
            file,
            ast: parse_result,
        });
        FileId(self.files.len() - 1)
    }

    fn get_file_contents(&self, id: FileId) -> &str {
        &self.files[id.0].file
    }

    fn get_ast(&self, id: FileId) -> Option<&ast::AstFile> {
        self.files[id.0].ast.as_ref()
    }

    fn new_file_id(&self) -> FileId {
        FileId(self.files.len())
    }

    fn get_path(&self, id: FileId) -> &Path {
        &self.files[id.0].path
    }
}

struct IncludeDirs {
    dirs: Vec<PathBuf>,
}

#[derive(Clone)]
pub struct UkonfConfig {
    include_dirs: Rc<IncludeDirs>,
    files: Rc<RefCell<Files>>,
}

impl UkonfConfig {
    pub fn new(include_dirs: Vec<PathBuf>) -> Self {
        Self {
            include_dirs: Rc::new(IncludeDirs { dirs: include_dirs }),
            files: Rc::new(RefCell::new(Files::new())),
        }
    }
}

pub struct UkonfParser {
    config: UkonfConfig,
    current_file: UkonfSourceFile,
}

enum UkonfSourceFile {
    OnDisk(PathBuf),
    String(String),
}

impl UkonfSourceFile {
    fn read_contents(&self) -> std::io::Result<String> {
        match self {
            UkonfSourceFile::OnDisk(p) => std::fs::read_to_string(p),
            UkonfSourceFile::String(s) => Ok(s.clone()),
        }
    }

    fn path(&self) -> PathBuf {
        match self {
            UkonfSourceFile::OnDisk(p) => p.clone(),
            UkonfSourceFile::String(_) => PathBuf::from("<string>"),
        }
    }
}

impl UkonfParser {
    fn new(file: UkonfSourceFile, config: UkonfConfig) -> Self {
        Self {
            config,
            current_file: file,
        }
    }

    fn parse<Err, E>(&mut self, err_cb: E) -> Result<FileId, Err>
    where
        E: for<'s> FnOnce(&'s str, ParseError<usize, Token<'s>, &str>) -> Err,
    {
        let file = self.current_file.read_contents().unwrap();
        let file_id = self.config.files.borrow().new_file_id();
        let parser = ukonf_parser::AstFileParser::new();
        let r = parser.parse(file_id, &file);
        let (ast, r) = match r {
            Ok(ast) => (Some(ast), Ok(file_id)),
            Err(e) => (None, Err(err_cb(&file, e))),
        };
        let fid = self
            .config
            .files
            .borrow_mut()
            .add_file(self.current_file.path(), file, ast);

        assert_eq!(fid, file_id);
        r
    }
}

#[derive(Default)]
pub struct UkonfFunctions {
    functions: BTreeMap<
        String,
        fn(&Rc<RefCell<Scope>>, &[UkonfValue]) -> Result<UkonfValue, UkonfFnError>,
    >,
    compiler_functions: BTreeMap<String, UkonfContextualValueCompiler>,
}

pub type UkonfFnError = anyhow::Error;

impl UkonfFunctions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_fn(
        mut self,
        name: impl Into<String>,
        f: fn(&Rc<RefCell<Scope>>, &[UkonfValue]) -> Result<UkonfValue, UkonfFnError>,
    ) -> Self {
        self.functions.insert(name.into(), f);
        self
    }

    pub fn add_fn(
        &mut self,
        name: impl Into<String>,
        f: fn(&Rc<RefCell<Scope>>, &[UkonfValue]) -> Result<UkonfValue, UkonfFnError>,
    ) -> &mut Self {
        self.functions.insert(name.into(), f);
        self
    }

    pub fn add_compiler_fn(
        &mut self,
        name: impl Into<String>,
        f: fn(&Rc<RefCell<Scope>>, UkonfValue) -> Result<UkonfValue, UkonfFnError>,
    ) -> &mut Self {
        self.compiler_functions
            .insert(name.into(), UkonfContextualValueCompiler { f });
        self
    }
}

pub struct UkonfRunner {
    config: UkonfConfig,
    functions: UkonfFunctions,
}

enum TempValue {
    RealValue(UkonfValue),
    Spread(Span, UkonfValue, Span),
}

impl TempValue {
    pub fn spread_to_array(self) -> Result<Vec<UkonfValue>, UkonfRunError> {
        match self {
            TempValue::RealValue(v) => Ok(vec![v]),
            TempValue::Spread(_spread, v, span) => match v {
                UkonfValue::Array(a) => Ok(a),
                v => Err(UkonfRunError::SpreadToArrayNotArray(v, span)),
            },
        }
    }
}

impl UkonfRunner {
    pub fn new(config: UkonfConfig, functions: UkonfFunctions) -> Self {
        Self { config, functions }
    }

    fn find_file(&self, f: &Path, file: &str) -> Option<PathBuf> {
        for dir in self.config.include_dirs.dirs.iter() {
            let p = dir.join(file);
            if p.is_file() {
                return Some(p.canonicalize().unwrap());
            }
        }

        if let Some(p) = f.parent() {
            let p = p.join(file);

            if p.is_file() {
                return Some(p.canonicalize().unwrap());
            }
        }

        None
    }

    fn process_string_file(&self, file: String) -> Result<FileId, UkonfRunError> {
        let mut parser = UkonfParser::new(UkonfSourceFile::String(file), self.config.clone());
        parser
            .parse(|_file, e| e.to_string())
            .map_err(|e| UkonfRunError::ParseError(e))
    }

    fn recursively_process_imports(&self, file: PathBuf) -> Result<FileId, UkonfRunError> {
        let mut files: BTreeMap<PathBuf, FileId> = BTreeMap::new();
        let mut files_to_process = vec![file];
        let mut first_file_id = None;

        while !files_to_process.is_empty() {
            let file = files_to_process.pop().unwrap();

            if files.contains_key(&file) {
                continue;
            }

            let mut parser =
                UkonfParser::new(UkonfSourceFile::OnDisk(file.clone()), self.config.clone());
            let file_id = parser
                .parse(|_file, e| e.to_string())
                .map_err(|e| UkonfRunError::ParseError(e))?;
            files.insert(file.clone(), file_id);
            if first_file_id.is_none() {
                first_file_id = Some(file_id);
            }

            if let Some(e) = self
                .config
                .files
                .borrow()
                .get_ast(file_id)
                .unwrap()
                .imports
                .iter()
                .filter_map(|it| match &it.path {
                    AstVal::Str(s) => {
                        let p = s.str_val();
                        let p = match self.find_file(&file, p.as_ref()) {
                            Some(path) => path,
                            None => {
                                return Some(UkonfRunError::CannotFindImport(
                                    p.into_owned(),
                                    s.span().clone(),
                                ))
                            }
                        };
                        files_to_process.push(p);
                        None
                    }
                    _ => Some(UkonfRunError::InvalidImport(it.path.span().clone())),
                })
                .next()
            {
                return Err(e);
            }
        }

        for file in files.values() {
            let files_ref = self.config.files.borrow();

            let file_path = files_ref.get_path(*file);

            files_ref
                .get_ast(*file)
                .unwrap()
                .imports
                .iter()
                .for_each(|it| {
                    let path = match &it.path {
                        AstVal::Str(s) => s.str_val(),
                        _ => unreachable!(),
                    };
                    let file_id = files[&self.find_file(file_path, path.as_ref()).unwrap()];
                    it.resolve_to(file_id);
                });
        }

        Ok(first_file_id.unwrap())
    }

    fn run_value(
        &self,
        files: &Ref<Files>,
        file_id: FileId,
        scope: &Rc<RefCell<Scope>>,
        val: &AstVal,
    ) -> Result<TempValue, UkonfRunError> {
        let v = match val {
            AstVal::Null(_) => TempValue::RealValue(UkonfValue::Null),
            AstVal::Ident(ident) => {
                let name = &ident.0 .0;
                TempValue::RealValue(
                    Scope::resolve(scope, name)
                        .ok_or_else(|| {
                            UkonfRunError::UndefinedVariable(name.clone(), ident.0 .1.clone())
                        })?
                        .fn_error(&ident.0 .1)?,
                )
            }
            AstVal::Str(s) => TempValue::RealValue(UkonfValue::Str(s.str_val().into_owned())),
            AstVal::Num(NumLit::Int(Spanned(v, _))) => {
                TempValue::RealValue(UkonfValue::Num(value::NumValue::Int(*v)))
            }
            AstVal::Num(NumLit::Float(Spanned(v, _))) => {
                TempValue::RealValue(UkonfValue::Num(value::NumValue::Float(*v)))
            }
            AstVal::Bool(b) => TempValue::RealValue(UkonfValue::Bool(b.value)),
            AstVal::Arr(_, values, _) => {
                let arr = values
                    .iter()
                    .map(|it| {
                        self.run_value(files, file_id, scope, it)
                            .and_then(|v| v.spread_to_array())
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                TempValue::RealValue(UkonfValue::Array(arr))
            }
            AstVal::Obj(_, items, _) => {
                let new_scope = Rc::new(RefCell::new(Scope {
                    parent: Some(Rc::clone(scope)),
                    vars: BTreeMap::new(),
                }));

                let obj = self.run_scope(files, file_id, &new_scope, items)?;
                TempValue::RealValue(UkonfValue::Object(obj))
            }
            AstVal::FunctionCall(call) => {
                let name = &call.name.0 .0;

                let f = self.functions.functions.get(name).ok_or_else(|| {
                    UkonfRunError::UnknownFunction(name.clone(), call.name.0 .1.clone())
                })?;

                let args = call
                    .args
                    .iter()
                    .map(|it| {
                        self.run_value(files, file_id, scope, it)
                            .and_then(|v| v.spread_to_array())
                    })
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();

                TempValue::RealValue(f(scope, &args).fn_error(&call.span())?)
            }
            AstVal::Dot(base, _, k) => {
                let base_value = Scope::resolve(scope, &base.0 .0)
                    .ok_or_else(|| UkonfRunError::UndefinedVariable(base.0 .0.clone(), k.span()))?
                    .fn_error(&base.0 .1)?;
                let (name, indirections) = k.key().lower();
                let mut name = name.to_string();
                let span = k.span();

                if indirections > 0 {
                    name = Scope::resolve_with_indirections(scope, &name, &span, indirections)?
                        ._expect_string(&span)?;
                }

                TempValue::RealValue(base_value.expect_object(&span)?.get(&name).unwrap().clone())
            }
            AstVal::Spread(spread, name) => {
                let value = Scope::resolve(scope, &name.0 .0)
                    .ok_or_else(|| {
                        UkonfRunError::UndefinedVariable(name.0 .0.clone(), name.0 .1.clone())
                    })?
                    .fn_error(&name.0 .1)?;

                TempValue::Spread(spread.0.clone(), value, name.0 .1.clone())
            }
        };

        Ok(v)
    }

    fn run_scope(
        &self,
        files: &Ref<Files>,
        file_id: FileId,
        scope: &Rc<RefCell<Scope>>,
        scope_items: &[AstItem],
    ) -> Result<UkonfObject, UkonfRunError> {
        let mut result = UkonfValue::Object(UkonfObject::new());

        for item in scope_items {
            match item {
                AstItem::Decl(decl) => {
                    let value = match self.run_value(files, file_id, scope, &decl.value)? {
                        TempValue::RealValue(v) => v,
                        TempValue::Spread(_, _, _) => {
                            return Err(UkonfRunError::SpreadNotAllowedHere(
                                decl.value.span(),
                                decl.name.0 .1.clone(),
                            ))
                        }
                    };

                    let cx_kind = match &decl.cx_kw {
                        Some(_cx_kw) => CxKind::Cx,
                        None => CxKind::Local,
                    };

                    let compiler = match &decl.compiler {
                        Some((_, compiler)) => {
                            let compiler_fn = self.functions.compiler_functions.get(&compiler.0 .0);

                            match compiler_fn {
                                Some(c) => Some(*c),
                                None => {
                                    return Err(UkonfRunError::UnknownFunction(
                                        compiler.0 .0.clone(),
                                        compiler.0 .1.clone(),
                                    ))
                                }
                            }
                        }
                        None => None,
                    };

                    scope
                        .borrow_mut()
                        .vars
                        .insert(decl.name.0 .0.clone(), (cx_kind, value, compiler));
                }
                AstItem::DocPatch(patch) => {
                    let v = match self.run_value(
                        files,
                        file_id,
                        &Rc::new(RefCell::new(Scope {
                            parent: Some(Rc::clone(scope)),
                            vars: BTreeMap::new(),
                        })),
                        &patch.value,
                    )? {
                        TempValue::RealValue(v) => v,
                        TempValue::Spread(_, _, _) => {
                            return Err(UkonfRunError::SpreadNotAllowedHere(
                                patch.value.span(),
                                patch.key[0].span(),
                            ))
                        }
                    };

                    let mut t = &mut result;

                    for k in &patch.key[..patch.key.len() - 1] {
                        let (name, indirections) = k.key().lower();
                        let mut name = name.to_string();
                        let span = k.span();

                        if indirections > 0 {
                            name = Scope::resolve_with_indirections(
                                scope,
                                &name,
                                &span,
                                indirections,
                            )?
                            ._expect_string(&span)?;
                        }

                        t = t
                            .expect_mut_object(&span)?
                            .get_or_insert(name, UkonfValue::Object(UkonfObject::new()));
                    }

                    let k = patch.key.last().unwrap();
                    let (name, indirections) = k.key().lower();
                    let mut name = name.to_string();
                    let span = k.span();

                    if indirections > 0 {
                        name = Scope::resolve_with_indirections(scope, &name, &span, indirections)?
                            ._expect_string(&span)?;
                    }

                    t.expect_mut_object(&span)?.insert(name, v);
                }
                AstItem::DocPatchSpread(_spread, name) => {
                    let value = Scope::resolve(scope, &name.0 .0)
                        .ok_or_else(|| {
                            UkonfRunError::UndefinedVariable(name.0 .0.clone(), name.0 .1.clone())
                        })?
                        .fn_error(&name.0 .1)?;

                    let value = match value {
                        UkonfValue::Object(o) => o,
                        v => {
                            return Err(UkonfRunError::SpreadToObjectNotObject(
                                v,
                                name.0 .1.clone(),
                            ))
                        }
                    };

                    let t = result.expect_mut_object(&name.0 .1)?;

                    for (k, v) in value.into_iter() {
                        t.insert(k, v);
                    }
                }
            }
        }

        Ok(result.unwrap_object())
    }

    fn run_file(&self, file_id: FileId) -> Result<UkonfObject, UkonfRunError> {
        let files = self.config.files.borrow();
        let ast = files.get_ast(file_id).unwrap();

        let scope = Rc::new(RefCell::new(Scope {
            parent: None,
            vars: BTreeMap::new(),
        }));

        let mut import_id = 0;
        for import in &ast.imports {
            let file_id = import.resolved().unwrap();

            let obj = self.run_file(file_id)?;

            let id = import.as_name.as_ref().map_or_else(
                || format!("import_{import_id}"),
                |(_, name)| name.0 .0.clone(),
            );
            import_id += 1;

            scope
                .borrow_mut()
                .vars
                .insert(id, (CxKind::Local, UkonfValue::Object(obj), None));
        }

        self.run_scope(&files, file_id, &scope, &ast.items)
    }

    pub fn run(&self, file: PathBuf) -> Result<UkonfObject, UkonfRunError> {
        let file_id = self.recursively_process_imports(file)?;

        self.run_file(file_id)
    }

    pub fn run_str(&self, s: &str) -> Result<UkonfObject, UkonfRunError> {
        let file_id = self.process_string_file(s.to_string())?;

        self.run_file(file_id)
    }
}

pub struct Scope {
    parent: Option<Rc<RefCell<Scope>>>,
    vars: BTreeMap<String, (CxKind, UkonfValue, Option<UkonfContextualValueCompiler>)>,
}

#[derive(Copy, Clone)]
pub struct UkonfContextualValueCompiler {
    f: fn(&Rc<RefCell<Scope>>, UkonfValue) -> Result<UkonfValue, UkonfFnError>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
enum CxKind {
    Cx,
    Local,
}

impl Scope {
    fn resolve(scope: &Rc<RefCell<Self>>, name: &str) -> Option<Result<UkonfValue, UkonfFnError>> {
        Scope::resolve_one(Rc::clone(scope), name).map(|((_, v, c), _)| match c {
            Some(c) => (c.f)(scope, v),
            None => Ok(v),
        })
    }

    pub fn resolve_cx(
        scope: &Rc<RefCell<Self>>,
        name: &str,
    ) -> Option<Result<UkonfValue, UkonfFnError>> {
        Scope::resolve_one(Rc::clone(scope), name).and_then(|((k, v, c), _)| {
            if let CxKind::Cx = k {
                Some(match c {
                    Some(compiler) => (compiler.f)(scope, v),
                    None => Ok(v),
                })
            } else {
                None
            }
        })
    }

    fn resolve_one<'a>(
        scope: Rc<RefCell<Self>>,
        name: &str,
    ) -> Option<(
        (CxKind, UkonfValue, Option<UkonfContextualValueCompiler>),
        Rc<RefCell<Self>>,
    )> {
        let v = scope.borrow().vars.get(name).cloned();
        match v {
            Some(val) => Some((val, scope)),
            None => scope
                .borrow()
                .parent
                .as_ref()
                .and_then(|p| Scope::resolve_one(Rc::clone(p), name)),
        }
    }

    fn resolve_with_indirections(
        scope: &Rc<RefCell<Self>>,
        name: &str,
        span: &Span,
        indirections: usize,
    ) -> Result<UkonfValue, UkonfRunError> {
        if indirections == 1 {
            return Scope::resolve(scope, name)
                .ok_or_else(|| UkonfRunError::UndefinedVariable(name.to_string(), span.clone()))?
                .fn_error(span);
        }

        let mut name = name.to_string();

        enum R<'a> {
            Ref(&'a Rc<RefCell<Scope>>),
            Owned(Rc<RefCell<Scope>>),
        }

        impl<'a> R<'a> {
            fn as_ref(&self) -> &Rc<RefCell<Scope>> {
                match self {
                    R::Ref(r) => r,
                    R::Owned(r) => r,
                }
            }
        }

        let mut resolver_scope = R::Ref(scope);

        for _ in 1..indirections {
            let ((_, v, _), scope) = Scope::resolve_one(Rc::clone(resolver_scope.as_ref()), &name)
                .ok_or_else(|| UkonfRunError::UndefinedVariable(name, span.clone()))?;
            name = v._expect_string(span)?;
            resolver_scope = R::Owned(scope);
        }

        Scope::resolve(resolver_scope.as_ref(), &name)
            .ok_or_else(|| UkonfRunError::UndefinedVariable(name, span.clone()))?
            .fn_error(span)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UkonfRunError {
    #[error("Undefined variable: {0}@{1}")]
    UndefinedVariable(String, Span),
    #[error("Expected string, got {0:?}@{1}")]
    ExpectedString(UkonfValue, Span),
    #[error("Expected object, got {0:?}@{1}")]
    ExpectedObject(UkonfValue, Span),
    #[error("Unknown function: {0}@{1}")]
    UnknownFunction(String, Span),
    #[error("Cannot find import: {0}@{1}")]
    CannotFindImport(String, Span),
    #[error("Invalid import: {0}")]
    InvalidImport(Span),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Spread to array, but is not an array: {0:?}@{1}")]
    SpreadToArrayNotArray(UkonfValue, Span),
    #[error("Spread to object, but is not an object: {0:?}@{1}")]
    SpreadToObjectNotObject(UkonfValue, Span),
    #[error("Spread not allowed in this context: {0:?}@{1}")]
    SpreadNotAllowedHere(Span, Span),
    #[error("Fn error: {0} (@{1})")]
    FnError(UkonfFnError, Span),
}

pub mod value;

#[cfg(test)]
mod tests;

trait FnError<T> {
    fn fn_error(self, span: &Span) -> Result<T, UkonfRunError>;
}

impl<T> FnError<T> for Result<T, UkonfFnError> {
    fn fn_error(self, span: &Span) -> Result<T, UkonfRunError> {
        self.map_err(|e| UkonfRunError::FnError(e, span.clone()))
    }
}
