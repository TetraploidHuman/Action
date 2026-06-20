//! Source formatting (token-aware indentation) shared by the LSP and `action fmt` CLI.

use std::collections::HashMap;

use crate::lexer::{Token, TokenKind};

/// Formatting options (LSP `FormattingOptions` equivalent).
#[derive(Debug, Clone, Copy)]
pub struct FormatOptions {
    pub tab_size: usize,
    pub insert_spaces: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        FormatOptions {
            tab_size: 4,
            insert_spaces: true,
        }
    }
}

/// Brace depth delta per source line derived from lexer tokens (string/comment safe).
fn line_brace_counts_from_tokens(source: &str, tokens: &[Token]) -> HashMap<usize, (u32, u32)> {
    let line_starts: Vec<usize> = std::iter::once(0)
        .chain(source.match_indices('\n').map(|(i, _)| i + 1))
        .collect();

    let line_of = |byte: usize| -> usize {
        line_starts
            .partition_point(|&start| start <= byte)
            .saturating_sub(1)
    };

    let mut line_braces: HashMap<usize, (u32, u32)> = HashMap::new();
    for tok in tokens {
        let (open, close) = match tok.kind {
            TokenKind::LBrace => (1u32, 0u32),
            TokenKind::RBrace => (0u32, 1u32),
            _ => continue,
        };
        let line = line_of(tok.span.start);
        let entry = line_braces.entry(line).or_insert((0, 0));
        entry.0 += open;
        entry.1 += close;
    }
    line_braces
}

fn line_brace_counts_fallback(source: &str) -> HashMap<usize, (u32, u32)> {
    let mut line_braces = HashMap::new();
    for (line_num, line) in source.split('\n').enumerate() {
        let open = line.chars().filter(|&c| c == '{').count() as u32;
        let close = line.chars().filter(|&c| c == '}').count() as u32;
        if open > 0 || close > 0 {
            line_braces.insert(line_num, (open, close));
        }
    }
    line_braces
}

/// Re-indent `source` using token-aware brace depth when `tokens` is non-empty.
pub fn format_source(source: &str, tokens: &[Token], options: &FormatOptions) -> String {
    let indent_str = if options.insert_spaces {
        " ".repeat(options.tab_size)
    } else {
        "\t".to_string()
    };

    let line_braces = if tokens.is_empty() {
        line_brace_counts_fallback(source)
    } else {
        line_brace_counts_from_tokens(source, tokens)
    };

    let mut expected_depth: i32 = 0;
    let mut out = String::with_capacity(source.len());
    let line_iter = source.split('\n').enumerate();
    let line_count = source.matches('\n').count() + if source.is_empty() { 0 } else { 1 };

    for (line_num, line) in line_iter {
        let trimmed = line.trim_start();
        if !trimmed.is_empty() {
            let (open_count, close_count) = line_braces.get(&line_num).copied().unwrap_or((0, 0));
            let current_depth = expected_depth.saturating_sub(close_count as i32);
            let expected_indent = indent_str.repeat(current_depth as usize);
            out.push_str(&expected_indent);
            out.push_str(trimmed);
            expected_depth = current_depth.saturating_add(open_count as i32);
        }
        if line_num + 1 < line_count {
            out.push('\n');
        }
    }

    if source.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn format_at(source: &str) -> String {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        format_source(source, &tokens, &FormatOptions::default())
    }

    #[test]
    fn test_format_indents_block() {
        let src = "fun f() {\nval x = 1\n}\n";
        let formatted = format_at(src);
        assert!(formatted.contains("    val x = 1"), "got: {:?}", formatted);
    }

    #[test]
    fn test_format_preserves_already_formatted() {
        let src = "val x = 1\n";
        let formatted = format_at(src);
        assert_eq!(formatted, src);
    }

    #[test]
    fn test_format_fixes_bad_indent() {
        let src = "fun f() {\nval x = 1\n}\n";
        let formatted = format_at(src);
        assert_ne!(src, formatted);
        let again = format_at(&formatted);
        assert_eq!(formatted, again);
    }

    #[test]
    fn test_format_ignores_braces_in_strings() {
        let src = "fun f() {\nval s = \"{ not a brace }\"\nval x = 1\n}\n";
        let formatted = format_at(src);
        assert!(
            formatted.contains("    val x = 1"),
            "string braces must not affect indent: {:?}",
            formatted
        );
    }
}
