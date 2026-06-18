use action_span::Span;
use ariadne::{Color, Label, Report, ReportKind, Source};

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

/// Report compiler errors with span-aware highlighting.
///
/// Accepts errors as string slices — each error string may be in
/// "Error at line X, col Y: message" format (produced by `CompilerError`'s
/// `Display` impl) or a plain error message.
pub fn report_compiler_errors(source: &str, path: &str, errors: &[impl AsRef<str>]) {
    for err_str in errors {
        let err_str = err_str.as_ref();
        let lines: Vec<&str> = err_str.lines().collect();
        let main_line = lines.first().copied().unwrap_or("");
        let help = if lines.len() > 1 && lines[1].trim().starts_with("help: ") {
            Some(
                lines[1]
                    .trim()
                    .strip_prefix("help: ")
                    .unwrap_or("")
                    .to_string(),
            )
        } else {
            None
        };

        if let Some((line, col, msg)) = parse_error_line(main_line) {
            let offset = line_col_to_offset(source, line, col);
            let highlight_len = 1usize;
            let mut report = Report::build(ReportKind::Error, path, offset)
                .with_message(&msg)
                .with_label(
                    Label::new((path, offset..offset + highlight_len))
                        .with_message("here")
                        .with_color(Color::Red),
                );
            if let Some(ref help) = help {
                report = report.with_help(help.clone());
            }
            report
                .finish()
                .eprint((path, Source::from(source)))
                .unwrap_or_else(|_| eprintln!("Error: {}", main_line));
        } else {
            eprintln!("\x1b[1;31merror:\x1b[0m {}", main_line);
            if let Some(ref help) = help {
                eprintln!("  \x1b[1;36mhelp:\x1b[0m {}", help);
            }
        }
    }
}

/// Extract (line, col, message) from an error string formatted as
/// "Error at line X, col Y: message" or "Parse error at line X, col Y: message".
fn parse_error_line(line: &str) -> Option<(usize, usize, String)> {
    if let Some(rest) = line.strip_prefix("Error at line ") {
        let parts: Vec<&str> = rest.splitn(2, ", col ").collect();
        if parts.len() == 2 {
            let line_num: usize = parts[0].parse().ok()?;
            let col_parts: Vec<&str> = parts[1].splitn(2, ": ").collect();
            let col: usize = col_parts[0].parse().ok()?;
            let msg = col_parts.get(1).unwrap_or(&"error").to_string();
            return Some((line_num, col, msg));
        }
    }
    if let Some(rest) = line.strip_prefix("Parse error at line ") {
        let parts: Vec<&str> = rest.splitn(2, ", col ").collect();
        if parts.len() == 2 {
            let line_num: usize = parts[0].parse().ok()?;
            let col_parts: Vec<&str> = parts[1].splitn(2, ": ").collect();
            let col: usize = col_parts[0].parse().ok()?;
            let msg = col_parts.get(1).unwrap_or(&"parse error").to_string();
            return Some((line_num, col, msg));
        }
    }
    None
}
