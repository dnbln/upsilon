use crate::defs::Defs;
use crate::diagnostics::DiagnosticsHost;
use crate::file_host::FileHost;
use crate::lower::LowerFile;
use crate::span::SpanHosts;
use ast::AstFile;
use std::path::PathBuf;
use std::rc::Rc;

pub mod ast;
pub mod defs;
mod compile;
pub mod diagnostics;
mod file_host;
mod keywords;
mod lower;
mod punct;
mod span;

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

pub fn compile(file: AstFile, diagnostics: &DiagnosticsHost) -> Option<Defs> {
    let lower_file = LowerFile::lower(file);

    compile::compile(lower_file, diagnostics)
}
