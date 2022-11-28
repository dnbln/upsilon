use std::borrow::Borrow;
use std::cell::RefCell;
use std::rc::Rc;

use codespan_reporting::files::SimpleFiles;
use codespan_reporting::term::termcolor::{ColorChoice, StandardStream};
use codespan_reporting::term::DisplayStyle;

use crate::file_host::FileHost;
use crate::span::Span;

pub struct DiagnosticsHost {
    diagnostics: RefCell<Vec<Diagnostic>>,

    file_host: Rc<FileHost>,
}

impl Drop for DiagnosticsHost {
    fn drop(&mut self) {
        if !self.diagnostics.borrow().is_empty() {
            eprintln!(
                "\
Missed DiagnosticsHost.emit() call! Performing from Drop; \
if you wanted to clear the diagnostics, call .clear() instead.\
"
            );

            self.emit();
        }
    }
}

impl DiagnosticsHost {
    pub fn new(file_host: Rc<FileHost>) -> Self {
        Self {
            diagnostics: RefCell::new(vec![]),
            file_host,
        }
    }

    pub(crate) fn collect(&self, diagnostic: Diagnostic) {
        self.diagnostics.borrow_mut().push(diagnostic);
    }

    pub fn clear(&mut self) {
        self.diagnostics.get_mut().clear();
    }

    pub fn emit(&self) {
        let mut diagnostics = self.diagnostics.borrow_mut();

        let writer = StandardStream::stderr(ColorChoice::Auto);
        let config = codespan_reporting::term::Config {
            display_style: DisplayStyle::Rich,
            ..Default::default()
        };
        let (files, file_id) = {
            let mut files = SimpleFiles::<String, &String>::new();

            let fh: &FileHost = self.file_host.borrow();

            let file_id = files.add(
                fh.path
                    .as_ref()
                    .map_or_else(|| String::from(""), |p| p.display().to_string()),
                &fh.contents,
            );

            (files, file_id)
        };

        for diagnostic in diagnostics.drain(..) {
            let mut diag_codespan = codespan_reporting::diagnostic::Diagnostic::<usize>::new(
                match diagnostic.severity {
                    DiagnosticSeverity::Error => codespan_reporting::diagnostic::Severity::Error,
                    DiagnosticSeverity::Warning => {
                        codespan_reporting::diagnostic::Severity::Warning
                    }
                    DiagnosticSeverity::Note => codespan_reporting::diagnostic::Severity::Note,
                    DiagnosticSeverity::Help => codespan_reporting::diagnostic::Severity::Help,
                },
            );

            if let Some(message) = diagnostic.message {
                diag_codespan = diag_codespan.with_message(message);
            }

            diag_codespan = diag_codespan.with_labels(
                std::iter::once(
                    codespan_reporting::diagnostic::Label::new(
                        codespan_reporting::diagnostic::LabelStyle::Primary,
                        file_id,
                        diagnostic.span.to_range_usize(),
                    )
                    .with_message(diagnostic.span_message),
                )
                .chain(diagnostic.additional_labels.into_iter().map(|it: Label| {
                    codespan_reporting::diagnostic::Label::new(
                        codespan_reporting::diagnostic::LabelStyle::Secondary,
                        file_id,
                        it.span.to_range_usize(),
                    )
                    .with_message(it.message)
                }))
                .collect(),
            );

            codespan_reporting::term::emit(&mut writer.lock(), &config, &files, &diag_codespan)
                .expect("codespan");
        }
    }
}

pub struct Label {
    span: Span,
    message: String,
}

impl Label {
    pub(crate) fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

#[must_use]
pub struct Diagnostic {
    span: Span,
    span_message: String,
    additional_labels: Vec<Label>,
    severity: DiagnosticSeverity,
    message: Option<String>,
}

pub(crate) enum DiagnosticSeverity {
    Error,
    Warning,
    Note,
    Help,
}

impl Diagnostic {
    pub(crate) fn new(
        span: Span,
        message: impl Into<String>,
        severity: DiagnosticSeverity,
    ) -> Self {
        Self {
            span,
            span_message: message.into(),
            additional_labels: vec![],
            severity,
            message: None,
        }
    }

    pub fn add_additional_label(&mut self, label: Label) {
        self.additional_labels.push(label)
    }

    pub fn with_additional_label(mut self, label: Label) -> Self {
        self.additional_labels.push(label);

        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());

        self
    }

    pub fn emit(self) {
        // cannot use borrowed diagnostics_host because we are moving self
        let diagnostics_host = Rc::clone(&self.span.span_hosts.diagnostics_host);
        diagnostics_host.collect(self);
    }
}
