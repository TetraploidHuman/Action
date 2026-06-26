use action_span::Span;
use ariadne::{Color, Label, Report, ReportKind, Source};
use serde::{Deserialize, Serialize};

/// Structured diagnostic codes (R7 fallibility).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticCode {
    E001,
    E002,
    E003,
    E004,
    E005,
    E006,
    E007,
    E008,
    E009,
    /// `null` literal is not supported.
    E010,
    /// Nullable types `T?` are not supported.
    E011,
    /// Safe call `?.` is not supported.
    E012,
}

impl DiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            DiagnosticCode::E001 => "E001",
            DiagnosticCode::E002 => "E002",
            DiagnosticCode::E003 => "E003",
            DiagnosticCode::E004 => "E004",
            DiagnosticCode::E005 => "E005",
            DiagnosticCode::E006 => "E006",
            DiagnosticCode::E007 => "E007",
            DiagnosticCode::E008 => "E008",
            DiagnosticCode::E009 => "E009",
            DiagnosticCode::E010 => "E010",
            DiagnosticCode::E011 => "E011",
            DiagnosticCode::E012 => "E012",
        }
    }
}

/// Structured compiler error with optional source location and help text
#[derive(Debug, Clone)]
pub struct CompilerError {
    pub message: String,
    pub span: Option<Span>,
    pub help: Option<String>,
    pub code: Option<DiagnosticCode>,
}

impl CompilerError {
    pub fn new(message: impl Into<String>) -> Self {
        CompilerError {
            message: message.into(),
            span: None,
            help: None,
            code: None,
        }
    }

    pub fn with_code(mut self, code: DiagnosticCode) -> Self {
        self.code = Some(code);
        self
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
    if let Some(code) = error.code {
        return Some(match code {
            DiagnosticCode::E001 => {
                "Fallible operations can fail at runtime. Wrap them in `or { default }` \
                 or add a function-level `or { default }` fallback."
                    .to_string()
            }
            DiagnosticCode::E002 => {
                "The fallback inside `or { }` must have the same type as the fallible or \
                 nullable expression on the left."
                    .to_string()
            }
            DiagnosticCode::E003 => {
                "A function-level `or { }` fallback must match the function's declared return type."
                    .to_string()
            }
            DiagnosticCode::E004 => {
                "A nullable value (`T?`) cannot be used where plain `T` is required. Use \
                 `or { default }` or narrow with a null check."
                    .to_string()
            }
            DiagnosticCode::E005 => {
                "Arithmetic and bitwise operators require non-nullable operands. Apply \
                 `or { default }` to nullable values first."
                    .to_string()
            }
            DiagnosticCode::E006 => "List indexing can fail when the index is out of bounds. Use \
                 `lst[i] or { default }`."
                .to_string(),
            DiagnosticCode::E007 => {
                "`or { }` is only needed for fallible calls (e.g. `parseInt`) or nullable values. \
                 Remove it from this expression."
                    .to_string()
            }
            DiagnosticCode::E008 => {
                "Map lookup can fail when the key is missing. Use `map[key] or { default }`."
                    .to_string()
            }
            DiagnosticCode::E009 => {
                "Set membership lookup can fail when the element is absent. Use \
                 `set[elem] or { default }`."
                    .to_string()
            }
            DiagnosticCode::E010 => {
                "`null` has been removed. Use fallible calls with `or { default }` or \
                 `or { return expr }`."
                    .to_string()
            }
            DiagnosticCode::E011 => {
                "Nullable types (`T?`) are not supported. Use fallible return types with `or { }`."
                    .to_string()
            }
            DiagnosticCode::E012 => {
                "Safe call (`?.`) is not supported. Use fallible access with `or { }`."
                    .to_string()
            }
        });
    }
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

pub fn e001_or_required(name: &str, span: Span) -> CompilerError {
    CompilerError::new(format!(
        "fallible call '{}' must be used with 'or {{ }}' to provide a default",
        name
    ))
    .with_span(span)
    .with_code(DiagnosticCode::E001)
    .with_help(
        "Wrap the call in `or { default }` or append `or { default }` after the function body",
    )
}

pub fn e002_or_type_mismatch(span: Span) -> CompilerError {
    CompilerError::new("or-block fallback type does not match fallible expression type")
        .with_span(span)
        .with_code(DiagnosticCode::E002)
        .with_help("Ensure the expression inside `or { }` has the same type as the left-hand side")
}

pub fn e003_fn_or_return(span: Span) -> CompilerError {
    CompilerError::new("function or-block fallback type does not match return type")
        .with_span(span)
        .with_code(DiagnosticCode::E003)
        .with_help("Change the fallback value or the function's return type so they match")
}

pub fn e004_nullable_termination(inferred: &str, declared: &str, span: Span) -> CompilerError {
    CompilerError::new(format!(
        "cannot use nullable '{}' where non-nullable '{}' is expected. Use 'or {{ }}' to provide a default, or check for null first",
        inferred, declared
    ))
    .with_span(span)
    .with_code(DiagnosticCode::E004)
    .with_help("Wrap the nullable expression in `or { default }` or narrow with a null check")
}

pub fn e005_nullable_arithmetic(op: &str, span: Span) -> CompilerError {
    CompilerError::new(format!(
        "Arithmetic/bitwise operation '{}' does not accept nullable operands. Use 'or {{ }}' to provide a default",
        op
    ))
    .with_span(span)
    .with_code(DiagnosticCode::E005)
}

pub fn e006_fallible_index_required(span: Span) -> CompilerError {
    CompilerError::new("fallible list index access must be used with 'or { }' to provide a default")
        .with_span(span)
        .with_code(DiagnosticCode::E006)
        .with_help(
            "Wrap the index expression in `or { default }` when the index may be out of range",
        )
}

pub fn e007_or_unnecessary(span: Span) -> CompilerError {
    CompilerError::new("'or { }' is not required: expression is neither fallible nor nullable")
        .with_span(span)
        .with_code(DiagnosticCode::E007)
        .with_help("Remove `or { }` or use it only on fallible calls or nullable values")
}

pub fn e008_map_index_required(span: Span) -> CompilerError {
    CompilerError::new("fallible map index access must be used with 'or { }' to provide a default")
        .with_span(span)
        .with_code(DiagnosticCode::E008)
        .with_help("Wrap the index expression in `or { default }` when the key may be missing")
}

pub fn e009_set_index_required(span: Span) -> CompilerError {
    CompilerError::new("fallible set index access must be used with 'or { }' to provide a default")
        .with_span(span)
        .with_code(DiagnosticCode::E009)
        .with_help("Wrap the index expression in `or { default }` when the element may be absent")
}

fn diagnostic_code_for(error: &CompilerError) -> &'static str {
    if let Some(code) = error.code {
        return code.as_str();
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explain_help_covers_all_e00n_codes() {
        for code in [
            DiagnosticCode::E001,
            DiagnosticCode::E002,
            DiagnosticCode::E003,
            DiagnosticCode::E004,
            DiagnosticCode::E005,
            DiagnosticCode::E006,
            DiagnosticCode::E007,
            DiagnosticCode::E008,
            DiagnosticCode::E009,
        ] {
            let err = CompilerError::new("test").with_code(code);
            let help = explain_help_for(&err).expect("E00N should have explain help");
            assert!(
                help.contains("or {") || help.contains("nullable"),
                "code {:?} help should mention recovery: {}",
                code,
                help
            );
        }
    }
}
