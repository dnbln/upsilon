use std::fmt;
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

use crate::diagnostics::{Diagnostic, DiagnosticSeverity, DiagnosticsHost, Label};
use crate::file_host::FileHost;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug, Hash)]
pub struct TextSize(usize);

impl TextSize {
    pub(crate) const ZERO: TextSize = TextSize(0);

    pub(crate) fn new(size: usize) -> Self {
        Self(size)
    }
}

#[derive(Clone)]
pub struct SpanData {
    start: TextSize,
    end: TextSize,
}

impl Debug for SpanData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start.0, self.end.0)
    }
}

impl Display for SpanData {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start.0, self.end.0)
    }
}

#[derive(Clone)]
pub struct Span {
    span_data: SpanData,

    pub(crate) span_hosts: Rc<SpanHosts>,
}

impl Span {
    pub(crate) fn new(start: TextSize, end: TextSize, span_hosts: Rc<SpanHosts>) -> Self {
        Self {
            span_data: SpanData { start, end },
            span_hosts,
        }
    }

    pub fn start(&self) -> TextSize {
        self.span_data.start
    }

    pub fn end(&self) -> TextSize {
        self.span_data.end
    }

    pub fn error(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.clone(), message, DiagnosticSeverity::Error)
    }

    pub fn warning(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.clone(), message, DiagnosticSeverity::Warning)
    }

    pub fn note(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.clone(), message, DiagnosticSeverity::Note)
    }

    pub fn help(&self, message: impl Into<String>) -> Diagnostic {
        Diagnostic::new(self.clone(), message, DiagnosticSeverity::Help)
    }

    pub(crate) fn to_range_usize(&self) -> std::ops::Range<usize> {
        self.span_data.start.0..self.span_data.end.0
    }

    pub fn label(&self, message: impl Into<String>) -> Label {
        Label::new(self.clone(), message)
    }

    pub(crate) fn join(&self, other: &Span) -> Span {
        let start = if self.start() < other.start() {
            self.start()
        } else {
            other.start()
        };

        let end = if self.end() > other.end() {
            self.end()
        } else {
            other.end()
        };

        Span::new(start, end, Rc::clone(&self.span_hosts))
    }
}

impl Debug for Span {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.span_data)
    }
}

impl Display for Span {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.span_data)
    }
}

pub(crate) struct SpanHosts {
    pub(crate) file_host: Rc<FileHost>,
    pub(crate) diagnostics_host: Rc<DiagnosticsHost>,
}

impl SpanHosts {
    pub(crate) fn new(file_host: Rc<FileHost>, diagnostics_host: Rc<DiagnosticsHost>) -> Self {
        Self {
            file_host,
            diagnostics_host,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Spanned<T>(T, Span);

impl<T> Spanned<T> {
    pub(crate) fn new(value: T, span: Span) -> Self {
        Self(value, span)
    }
}

impl<T> Spanned<T> {
    pub fn span(&self) -> &Span {
        &self.1
    }
}

impl<T> Deref for Spanned<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Spanned<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: PartialEq> PartialEq for Spanned<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: Eq> Eq for Spanned<T> {}

impl<T: PartialOrd> PartialOrd for Spanned<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(&other.0)
    }
}

impl<T: Ord> Ord for Spanned<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<T: Display> Display for Spanned<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
