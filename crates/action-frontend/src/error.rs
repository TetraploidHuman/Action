use action_span::Span;
use ariadne::{Color, Label, Report, ReportKind, Source};
use serde::{Deserialize, Serialize};

/// Structured diagnostic codes (R7 fallibility + call/index hygiene).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticCode {
    E001,
    E002,
    E003,
    /// Call to an undeclared function / builtin.
    E004,
    /// Tuple/struct index is out of range or not an integer literal.
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
    /// Named struct has no such field.
    E013,
    /// Unknown enum constructor in a pattern.
    E014,
    /// Struct literal missing required field(s) for an expected Named type.
    E015,
    /// Struct literal field value type mismatch under expected Named type.
    E016,
    /// `if` / OneLine condition is not `Bool`.
    E017,
    /// `if` / OneLine then and else branch types differ.
    E018,
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
            DiagnosticCode::E013 => "E013",
            DiagnosticCode::E014 => "E014",
            DiagnosticCode::E015 => "E015",
            DiagnosticCode::E016 => "E016",
            DiagnosticCode::E017 => "E017",
            DiagnosticCode::E018 => "E018",
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
                "The fallback inside `or { }` must have the same type as the fallible expression on the left."
                    .to_string()
            }
            DiagnosticCode::E003 => {
                "A function-level `or { }` fallback must match the function's declared return type."
                    .to_string()
            }
            DiagnosticCode::E004 => {
                "The called name is not a known function, builtin, or enum variant. \
                 Check spelling, imports, and that host hooks are defined in the compiling module."
                    .to_string()
            }
            DiagnosticCode::E005 => {
                "Tuple/struct indexing uses a compile-time integer field slot, not a fallible \
                 collection lookup. Use an in-range literal (`p[0]`, `p[1]`, …); do not wrap with `or { }`."
                    .to_string()
            }
            DiagnosticCode::E006 => "List indexing can fail when the index is out of bounds. Use \
                 `lst[i] or { default }`."
                .to_string(),
            DiagnosticCode::E007 => {
                "`or { }` is only needed for fallible calls (e.g. `parseInt`, `head`, `get`). \
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
            DiagnosticCode::E013 => {
                "This type has no field with that name. Check the struct definition, or use \
                 a method call `recv.method(...)` if you meant UFCS."
                    .to_string()
            }
            DiagnosticCode::E014 => {
                "Pattern constructors must be declared enum variants. Check spelling, or bind \
                 with a lowercase variable / `else` instead of an unknown uppercase name."
                    .to_string()
            }
            DiagnosticCode::E015 => {
                "A struct literal under an expected type must include every declared field. \
                 Add the missing field(s), or change the type annotation."
                    .to_string()
            }
            DiagnosticCode::E016 => {
                "Each field in a struct literal must match the declared field type. \
                 Fix the value, or change the struct definition."
                    .to_string()
            }
            DiagnosticCode::E017 => {
                "`if` / conditional branches require a Bool condition. Use a comparison \
                 (`x > 0`), a Bool variable, or `true`/`false`."
                    .to_string()
            }
            DiagnosticCode::E018 => {
                "Both branches of `if` must have the same type (the value of the expression). \
                 Make then/else return the same type, or use `()` / omit a value if you only \
                 need side effects."
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

pub fn e004_unknown_call(name: &str, span: Span) -> CompilerError {
    CompilerError::new(format!("unknown function or builtin '{name}'"))
        .with_span(span)
        .with_code(DiagnosticCode::E004)
        .with_help("Define the function, import it, or use a registered builtin name")
}

pub fn e005_struct_index_invalid(message: impl Into<String>, span: Span) -> CompilerError {
    CompilerError::new(message)
        .with_span(span)
        .with_code(DiagnosticCode::E005)
        .with_help("Use a non-negative integer literal within the tuple/struct field count")
}

pub fn e013_unknown_struct_field(type_name: &str, field: &str, span: Span) -> CompilerError {
    CompilerError::new(format!("type '{type_name}' has no field '{field}'"))
        .with_span(span)
        .with_code(DiagnosticCode::E013)
        .with_help("Check the struct / type-alias field names")
}

pub fn e014_unknown_enum_constructor(name: &str, span: Span) -> CompilerError {
    CompilerError::new(format!("unknown enum constructor '{name}' in pattern"))
        .with_span(span)
        .with_code(DiagnosticCode::E014)
        .with_help("Use a declared enum variant, a lowercase binding, or `else`")
}

pub fn e015_struct_literal_missing_field(
    type_name: &str,
    field: &str,
    span: Span,
) -> CompilerError {
    CompilerError::new(format!(
        "struct literal for '{type_name}' is missing field '{field}'"
    ))
    .with_span(span)
    .with_code(DiagnosticCode::E015)
    .with_help("Include every field declared on the type, or widen the type annotation")
}

pub fn e016_struct_field_type_mismatch(
    type_name: &str,
    field: &str,
    expected: &str,
    got: &str,
    span: Span,
) -> CompilerError {
    CompilerError::new(format!(
        "struct literal for '{type_name}': field '{field}' expects '{expected}' but got '{got}'"
    ))
    .with_span(span)
    .with_code(DiagnosticCode::E016)
    .with_help("Change the field value type, or update the struct definition")
}

pub fn e017_if_condition_not_bool(got: &str, span: Span) -> CompilerError {
    CompilerError::new(format!(
        "If condition requires Bool expression, got '{got}'"
    ))
    .with_span(span)
    .with_code(DiagnosticCode::E017)
    .with_help("Use a Bool expression such as `x > 0` or a Bool variable")
}

pub fn e018_if_branch_type_mismatch(then_ty: &str, else_ty: &str, span: Span) -> CompilerError {
    CompilerError::new(format!(
        "If branches have mismatched types: then is '{then_ty}', else is '{else_ty}'"
    ))
    .with_span(span)
    .with_code(DiagnosticCode::E018)
    .with_help("Give both branches the same type, or use a block whose last expression matches")
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
    CompilerError::new("'or { }' is not required: expression is not fallible")
        .with_span(span)
        .with_code(DiagnosticCode::E007)
        .with_help("Remove `or { }` or use it only on fallible calls")
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
    fn explain_help_covers_fallible_e00n_codes() {
        for code in [
            DiagnosticCode::E001,
            DiagnosticCode::E002,
            DiagnosticCode::E003,
            DiagnosticCode::E006,
            DiagnosticCode::E007,
            DiagnosticCode::E008,
            DiagnosticCode::E009,
        ] {
            let err = CompilerError::new("test").with_code(code);
            let help = explain_help_for(&err).expect("E00N should have explain help");
            assert!(
                help.contains("or {"),
                "code {:?} help should mention recovery: {}",
                code,
                help
            );
        }
    }

    #[test]
    fn explain_help_covers_e016_struct_field_type() {
        let err = CompilerError::new("test").with_code(DiagnosticCode::E016);
        let help = explain_help_for(&err).expect("E016 should have explain help");
        assert!(
            help.contains("field") || help.contains("type"),
            "E016 help: {}",
            help
        );
    }

    #[test]
    fn explain_help_covers_e017_if_condition() {
        let err = CompilerError::new("test").with_code(DiagnosticCode::E017);
        let help = explain_help_for(&err).expect("E017 should have explain help");
        assert!(
            help.contains("Bool") || help.contains("comparison"),
            "E017 help: {}",
            help
        );
    }

    #[test]
    fn explain_help_covers_e018_if_branches() {
        let err = CompilerError::new("test").with_code(DiagnosticCode::E018);
        let help = explain_help_for(&err).expect("E018 should have explain help");
        assert!(
            help.contains("branch") || help.contains("type") || help.contains("if"),
            "E018 help: {}",
            help
        );
    }

    #[test]
    fn explain_help_covers_e015_struct_literal_missing() {
        let err = CompilerError::new("test").with_code(DiagnosticCode::E015);
        let help = explain_help_for(&err).expect("E015 should have explain help");
        assert!(
            help.contains("field") || help.contains("literal"),
            "E015 help: {}",
            help
        );
    }

    #[test]
    fn explain_help_covers_e014_unknown_constructor() {
        let err = CompilerError::new("test").with_code(DiagnosticCode::E014);
        let help = explain_help_for(&err).expect("E014 should have explain help");
        assert!(
            help.contains("variant") || help.contains("enum") || help.contains("else"),
            "E014 help: {}",
            help
        );
    }

    #[test]
    fn explain_help_covers_e013_unknown_field() {
        let err = CompilerError::new("test").with_code(DiagnosticCode::E013);
        let help = explain_help_for(&err).expect("E013 should have explain help");
        assert!(
            help.contains("field") || help.contains("struct"),
            "E013 help: {}",
            help
        );
    }

    #[test]
    fn explain_help_covers_e005_struct_index() {
        let err = CompilerError::new("test").with_code(DiagnosticCode::E005);
        let help = explain_help_for(&err).expect("E005 should have explain help");
        assert!(
            help.contains("literal") || help.contains("tuple"),
            "E005 help: {}",
            help
        );
    }

    #[test]
    fn explain_help_covers_e004_unknown_call() {
        let err = CompilerError::new("test").with_code(DiagnosticCode::E004);
        let help = explain_help_for(&err).expect("E004 should have explain help");
        assert!(
            help.contains("builtin") || help.contains("function"),
            "E004 help: {}",
            help
        );
    }
}
