use std::ops::{Deref, Not};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use crate::ast::AstFile;
use crate::diagnostics::DiagnosticsHost;
use crate::file_host::FileHost;
use crate::lower::LowerFile;
use crate::span::SpanHosts;

pub mod ast;
mod compile;
pub mod config;
pub mod diagnostics;
mod file_host;
mod keywords;
pub mod lower;
mod punct;
mod span;

pub use compile::{CompileCx, Compiler};
pub use config::Config;

#[rustfmt::skip]
mod parser {
    include!("parser.rs");
}

pub fn parse<'input>(
    path: Option<PathBuf>,
    file: &'input str,
) -> Result<
    (AstFile, Rc<DiagnosticsHost>),
    lalrpop_util::ParseError<usize, lalrpop_util::lexer::Token<'input>, &'static str>,
> {
    let file_host = Rc::new(FileHost::new(path, file.to_string()));
    let diagnostics_host = Rc::new(DiagnosticsHost::new(Rc::clone(&file_host)));
    let span_hosts = Rc::new(SpanHosts::new(file_host, Rc::clone(&diagnostics_host)));

    let file = parser::FileParser::new().parse(&span_hosts, file)?;

    Ok((file, diagnostics_host))
}

pub fn resolve_refs(file: AstFile, diagnostics: &DiagnosticsHost) -> (LowerFile, Successful) {
    let lower_file = Rc::new(LowerFile::lower(file));

    let success = compile::resolve_refs(&lower_file, diagnostics);

    let file = match Rc::try_unwrap(lower_file) {
        Ok(f) => f,
        Err(_) => {
            panic!(
                "Rc::try_unwrap failed. Do not store references to the file in the file AST nodes!"
            );
        }
    };

    (file, success)
}

pub fn compile<C>(
    file: AstFile,
    diagnostics: &DiagnosticsHost,
    compiler: &C,
    to_file: &Path,
) -> Successful
where
    C: Compiler,
{
    let lower_file = Rc::new(LowerFile::lower(file));

    compile::compile(lower_file, diagnostics, compiler, to_file)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[must_use]
pub enum Successful {
    Yes,
    No,
}

impl Not for Successful {
    type Output = Successful;

    fn not(self) -> Self::Output {
        match self {
            Successful::Yes => Successful::No,
            Successful::No => Successful::Yes,
        }
    }
}

impl Deref for Successful {
    type Target = bool;

    fn deref(&self) -> &Self::Target {
        match self {
            Successful::Yes => &true,
            Successful::No => &false,
        }
    }
}
