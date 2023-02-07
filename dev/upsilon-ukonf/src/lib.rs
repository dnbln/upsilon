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
use std::path::PathBuf;
use std::rc::Rc;

use lalrpop_util::lexer::Token;
use lalrpop_util::{lalrpop_mod, ParseError};

use crate::ast::{AstItem, AstVal, FileId, NumLit, Spanned};
use crate::value::{UkonfObject, UkonfValue};

lalrpop_mod!(ukonf_parser);

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
    functions: BTreeMap<String, fn(&[UkonfValue]) -> Result<UkonfValue, String>>,
}

impl UkonfFunctions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_fn(
        mut self,
        name: impl Into<String>,
        f: fn(&[UkonfValue]) -> Result<UkonfValue, String>,
    ) -> Self {
        self.functions.insert(name.into(), f);
        self
    }

    pub fn add_fn(
        &mut self,
        name: impl Into<String>,
        f: fn(&[UkonfValue]) -> Result<UkonfValue, String>,
    ) -> &mut Self {
        self.functions.insert(name.into(), f);
        self
    }
}

pub struct UkonfRunner {
    config: UkonfConfig,
    functions: UkonfFunctions,
}

impl UkonfRunner {
    pub fn new(config: UkonfConfig, functions: UkonfFunctions) -> Self {
        Self { config, functions }
    }

    fn find_file(&self, file: &str) -> Option<PathBuf> {
        for dir in self.config.include_dirs.dirs.iter() {
            let p = dir.join(file);
            if p.is_file() {
                return Some(p.canonicalize().unwrap());
            }
        }
        None
    }

    fn process_string_file(&self, file: String) -> Result<FileId, String> {
        let mut parser = UkonfParser::new(UkonfSourceFile::String(file), self.config.clone());
        parser.parse(|_file, e| e.to_string())
    }

    fn recursively_process_imports(&self, file: PathBuf) -> Result<FileId, String> {
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
            let file_id = parser.parse(|_file, e| e.to_string())?;
            files.insert(file, file_id);
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
                        let p = match self.find_file(p.as_ref()) {
                            Some(path) => path,
                            None => return Some(format!("Could not find file {p}")),
                        };
                        files_to_process.push(p);
                        None
                    }
                    _ => Some("Invalid import path".to_string()),
                })
                .next()
            {
                return Err(e);
            }
        }

        for file in files.values() {
            self.config
                .files
                .borrow()
                .get_ast(*file)
                .unwrap()
                .imports
                .iter()
                .for_each(|it| {
                    let path = match &it.path {
                        AstVal::Str(s) => s.str_val(),
                        _ => unreachable!(),
                    };
                    let file_id = files[&self.find_file(path.as_ref()).unwrap()];
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
    ) -> Result<UkonfValue, String> {
        let v = match val {
            AstVal::Null(_) => UkonfValue::Null,
            AstVal::Ident(ident) => {
                let name = &ident.0 .0;
                scope.borrow().resolve(name).unwrap()
            }
            AstVal::Str(s) => UkonfValue::Str(s.str_val().into_owned()),
            AstVal::Num(NumLit::Int(Spanned(v, _))) => UkonfValue::Num(value::NumValue::Int(*v)),
            AstVal::Num(NumLit::Float(Spanned(v, _))) => {
                UkonfValue::Num(value::NumValue::Float(*v))
            }
            AstVal::Bool(b) => UkonfValue::Bool(b.value),
            AstVal::Arr(_, values, _) => {
                let arr = values
                    .iter()
                    .map(|it| self.run_value(files, file_id, scope, it))
                    .collect::<Result<Vec<_>, _>>()?;

                UkonfValue::Array(arr)
            }
            AstVal::Obj(_, items, _) => {
                let new_scope = Rc::new(RefCell::new(Scope {
                    parent: Some(Rc::clone(scope)),
                    vars: BTreeMap::new(),
                }));

                let obj = self.run_scope(files, file_id, &new_scope, items)?;
                UkonfValue::Object(obj)
            }
            AstVal::FunctionCall(call) => {
                let name = &call.name.0 .0;

                let f = self.functions.functions.get(name).unwrap();

                let args = call
                    .args
                    .iter()
                    .map(|it| self.run_value(files, file_id, scope, it))
                    .collect::<Result<Vec<_>, _>>()?;

                f(&args)?
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
    ) -> Result<UkonfObject, String> {
        let mut result = UkonfValue::Object(UkonfObject::new());

        for item in scope_items {
            match item {
                AstItem::Decl(decl) => {
                    let value = self.run_value(files, file_id, scope, &decl.value)?;

                    scope
                        .borrow_mut()
                        .vars
                        .insert(decl.name.0 .0.clone(), value);
                }
                AstItem::DocPatch(patch) => {
                    let mut t = &mut result;

                    for k in &patch.key[..patch.key.len() - 1] {
                        let (name, indirections) = k.key().lower();
                        let mut name = name.to_string();

                        if indirections > 0 {
                            name = scope
                                .borrow()
                                .resolve_with_indirections(&name, indirections - 1)
                                .unwrap()
                                .as_string()
                                .unwrap()
                                .clone();
                        }

                        t = t
                            .as_mut_object()
                            .unwrap()
                            .get_or_insert(name, UkonfValue::Object(UkonfObject::new()));
                    }

                    let k = patch.key.last().unwrap();
                    let (name, indirections) = k.key().lower();
                    let mut name = name.to_string();

                    if indirections > 0 {
                        name = scope
                            .borrow()
                            .resolve_with_indirections(&name, indirections - 1)
                            .unwrap()
                            .as_string()
                            .unwrap()
                            .clone();
                    }

                    t.as_mut_object().unwrap().insert(
                        name,
                        self.run_value(
                            files,
                            file_id,
                            &Rc::new(RefCell::new(Scope {
                                parent: Some(Rc::clone(scope)),
                                vars: BTreeMap::new(),
                            })),
                            &patch.value,
                        )?,
                    );
                }
            }
        }

        Ok(result.unwrap_object())
    }

    fn run_file(&self, file_id: FileId) -> Result<UkonfObject, String> {
        let files = self.config.files.borrow();
        let ast = files.get_ast(file_id).unwrap();

        self.run_scope(
            &files,
            file_id,
            &Rc::new(RefCell::new(Scope {
                parent: None,
                vars: BTreeMap::new(),
            })),
            &ast.items,
        )
    }

    pub fn run(&self, file: PathBuf) -> Result<UkonfObject, String> {
        let file_id = self.recursively_process_imports(file)?;

        self.run_file(file_id)
    }

    pub fn run_str(&self, s: &str) -> Result<UkonfObject, String> {
        let file_id = self.process_string_file(s.to_string())?;

        self.run_file(file_id)
    }
}

pub struct Scope {
    parent: Option<Rc<RefCell<Scope>>>,
    vars: BTreeMap<String, UkonfValue>,
}

impl Scope {
    fn resolve(&self, name: &String) -> Option<UkonfValue> {
        self.vars
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref().and_then(|p| p.borrow().resolve(name)))
    }

    fn resolve_with_indirections(&self, name: &String, indirections: usize) -> Option<UkonfValue> {
        if indirections == 0 {
            return self.resolve(name);
        }

        let val = self.vars.get(name);

        if let Some(val) = val {
            if indirections == 0 {
                return Some(val.clone());
            }

            let name = val.as_string().unwrap();
            self.resolve_with_indirections(name, indirections - 1)
        } else {
            self.parent
                .as_ref()
                .and_then(|p| p.borrow().resolve_with_indirections(name, indirections))
        }
    }
}

pub mod value;

#[cfg(test)]
mod tests;
