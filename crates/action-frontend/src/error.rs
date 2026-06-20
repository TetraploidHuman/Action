use action_span::Span;
use ariadne::{Color, Label, Report, ReportKind, Source};
use serde::{Deserialize, Serialize};

/// Structured compiler error with optional source location and help text
#[derive(Debug, Clone)]
pub struct CompilerError {
    pub message: String,
    pub span: Option<Span>,
    pub help: Option<String>,
}

impl CompilerError {
    pub fn new(message: impl Into<String>) -> Self {
        CompilerError {
            message: message.into(),
            span: None,
            help: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(span) = &self.span {
            write!(
                f,
                "Error at line {}, col {}: {}",
                span.line, span.col, self.message
            )?;
        } else {
            write!(f, "Error: {}", self.message)?;
        }
        if let Some(help) = &self.help {
            write!(f, "\n  help: {}", help)?;
        }
        Ok(())
    }
}

impl From<String> for CompilerError {
    fn from(s: String) -> Self {
        CompilerError::new(s)
    }
}

impl From<&str> for CompilerError {
    fn from(s: &str) -> Self {
        CompilerError::new(s.to_string())
    }
}

/// Suggest help text for common type-check errors (`action check --explain`).
pub fn explain_help_for(error: &CompilerError) -> Option<String> {
    let msg = &error.message;
    if msg.contains("Undefined variable") {
        Some(
            "Check that the variable is defined in the current scope. Variable names are case-sensitive."
                .to_string(),
        )
    } else if msg.contains("type") && msg.contains("expected") {
        Some(
            "Type annotations and inferred types must match. Consider adding an explicit type annotation."
                .to_string(),
        )
    } else if msg.contains("Undefined function") {
        Some(
            "Functions must be defined before they are called. Check for typos in the function name."
                .to_string(),
        )
    } else if msg.contains("not exhaustive") {
        Some(
            "When expressions must cover all possible cases. Add an 'else' arm or cover all enum variants."
                .to_string(),
        )
    } else {
        None
    }
}

/// Attach explain-mode help while preserving span and message.
pub fn enrich_with_explain(mut error: CompilerError) -> CompilerError {
    if error.help.is_none() {
        if let Some(help) = explain_help_for(&error) {
            error.help = Some(help);
        }
    }
    error
}

/// Machine-readable diagnostic for `--format json` / LSP adapters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub severity: String,
    pub message: String,
    pub file: String,
    pub line: Option<usize>,
    pub col: Option<usize>,
    pub highlight_len: Option<usize>,
    pub help: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticEnvelope {
    pub version: u32,
    pub diagnostics: Vec<Diagnostic>,
}

fn diagnostic_code_for(error: &CompilerError) -> &'static str {
    let msg = error.message.as_str();
    if msg.contains("Lexer error") || msg.contains("Unexpected") {
        "lex-error"
    } else if msg.contains("Parse error") || msg.contains("Expected") {
        "parse-error"
    } else if msg.contains("Circular import") || msg.contains("Module '") {
        "import-error"
    } else if msg.contains("Cannot infer type arguments") {
        "generic-error"
    } else if msg.contains("No matching overload") {
        "overload-error"
    } else {
        "type-error"
    }
}

pub fn compiler_error_to_diagnostic(error: &CompilerError, file: &str) -> Diagnostic {
    Diagnostic {
        severity: "error".to_string(),
        message: error.message.clone(),
        file: file.to_string(),
        line: error.span.as_ref().map(|s| s.line),
        col: error.span.as_ref().map(|s| s.col),
        highlight_len: error.span.as_ref().map(|s| s.highlight_len()),
        help: error.help.clone(),
        code: Some(diagnostic_code_for(error).to_string()),
    }
}

pub fn diagnostics_to_json_pretty(
    errors: &[CompilerError],
    file: &str,
    explain: bool,
) -> Result<String, serde_json::Error> {
    let diagnostics: Vec<Diagnostic> = errors
        .iter()
        .map(|e| {
            let err = if explain {
                enrich_with_explain(e.clone())
            } else {
                e.clone()
            };
            compiler_error_to_diagnostic(&err, file)
        })
        .collect();
    serde_json::to_string_pretty(&DiagnosticEnvelope {
        version: 1,
        diagnostics,
    })
}

/// Convert line (1-indexed) and col (1-indexed) to byte offset in source.
pub fn line_col_to_offset(source: &str, line: usize, col: usize) -> usize {
    let mut cur_line = 1;
    let mut cur_col = 1;
    for (i, ch) in source.char_indices() {
        if cur_line == line && cur_col == col {
            return i;
        }
        if ch == '\n' {
            cur_line += 1;
            cur_col = 1;
        } else {
            cur_col += 1;
        }
    }
    source.len()
}

/// Report compiler errors using structured spans (preferred over string re-parse).
pub fn report_compiler_errors(source: &str, path: &str, errors: &[CompilerError]) {
    for error in errors {
        report_one_compiler_error(source, path, error);
    }
}

fn report_one_compiler_error(source: &str, path: &str, error: &CompilerError) {
    if let Some(span) = &error.span {
        let offset = line_col_to_offset(source, span.line, span.col);
        let highlight_len = span.highlight_len();
        let end = (offset + highlight_len).min(source.len());
        let mut report = Report::build(ReportKind::Error, path, offset)
            .with_message(&error.message)
            .with_label(
                Label::new((path, offset..end))
                    .with_message("here")
                    .with_color(Color::Red),
            );
        if let Some(ref help) = error.help {
            report = report.with_help(help.clone());
        }
        report
            .finish()
            .eprint((path, Source::from(source)))
            .unwrap_or_else(|_| eprintln!("Error: {}", error.message));
    } else {
        eprintln!("\x1b[1;31merror:\x1b[0m {}", error.message);
        if let Some(ref help) = error.help {
            eprintln!("  \x1b[1;36mhelp:\x1b[0m {}", help);
        }
    }
}

/// Report a plain error message (no span available).
pub fn report_error_message(source: &str, path: &str, message: &str) {
    eprintln!("\x1b[1;31merror:\x1b[0m {}", message);
    let _ = (source, path);
}
