use action_frontend::lexer::{Span, Token, TokenKind};
use lsp_types::{Position, Range};

/// Convert 1-indexed line/col to byte offset in source
pub fn line_col_to_offset(source: &str, line: usize, col: usize) -> usize {
    let mut current_line = 1;
    let mut current_col = 1;
    for (i, ch) in source.char_indices() {
        if current_line == line && current_col == col {
            return i;
        }
        if ch == '\n' {
            current_line += 1;
            current_col = 1;
        } else {
            current_col += 1;
        }
    }
    source.len()
}

/// Convert byte offset to (line, col) — both 1-indexed
pub fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Convert byte offset to LSP Position (0-indexed)
pub fn offset_to_lsp_position(source: &str, offset: usize) -> Position {
    let (line, col) = offset_to_line_col(source, offset);
    Position {
        line: (line as u32).saturating_sub(1),
        character: (col as u32).saturating_sub(1),
    }
}

/// Convert LSP Position (0-indexed) to byte offset
pub fn lsp_position_to_offset(source: &str, pos: &Position) -> usize {
    line_col_to_offset(source, pos.line as usize + 1, pos.character as usize + 1)
}

/// Convert a Span to an LSP Range
pub fn span_to_lsp_range(span: &Span, source: &str) -> Range {
    Range {
        start: offset_to_lsp_position(source, span.start),
        end: offset_to_lsp_position(source, span.end),
    }
}

/// Find token at a given byte offset. Uses binary search.
pub fn find_token_at(tokens: &[Token], offset: usize) -> Option<&Token> {
    let idx = tokens.partition_point(|t| t.span.start <= offset);
    if idx > 0 {
        let tok = &tokens[idx - 1];
        if offset <= tok.span.end {
            return Some(tok);
        }
    }
    None
}

/// Result of finding a node at a position
pub enum FoundNode {
    Ident(String),
    Keyword(String),
    Literal,
    Operator,
}

/// Find what kind of node is at the given LSP position
pub fn find_node_at(tokens: &[Token], source: &str, pos: &Position) -> Option<FoundNode> {
    let offset = lsp_position_to_offset(source, pos);
    let token = find_token_at(tokens, offset)?;

    match &token.kind {
        TokenKind::Ident(name) => Some(FoundNode::Ident(name.clone())),
        TokenKind::IntLiteral(_) => Some(FoundNode::Literal),
        TokenKind::FloatLiteral(_) => Some(FoundNode::Literal),
        TokenKind::StringLiteral(_) => Some(FoundNode::Literal),
        TokenKind::CharLiteral(_) => Some(FoundNode::Literal),
        TokenKind::BoolLiteral(b) => Some(FoundNode::Keyword(
            if *b { "true" } else { "false" }.to_string(),
        )),
        TokenKind::Val
        | TokenKind::Var
        | TokenKind::Fun
        | TokenKind::Return
        | TokenKind::When
        | TokenKind::If
        | TokenKind::Else
        | TokenKind::For
        | TokenKind::In
        | TokenKind::Is
        | TokenKind::Break
        | TokenKind::Continue
        | TokenKind::Enum
        | TokenKind::Type
        | TokenKind::Import
        | TokenKind::Module
        | TokenKind::Export
        | TokenKind::Const
        | TokenKind::Copy
        | TokenKind::Lazy
        | TokenKind::Unsafe
        | TokenKind::External
        | TokenKind::Extension
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Not
        | TokenKind::As
        | TokenKind::Task
        | TokenKind::Lambda => Some(FoundNode::Keyword(format!("{:?}", token.kind))),
        TokenKind::LBrace
        | TokenKind::RBrace
        | TokenKind::LParen
        | TokenKind::RParen
        | TokenKind::LBracket
        | TokenKind::RBracket
        | TokenKind::Semicolon
        | TokenKind::Comma
        | TokenKind::Colon
        | TokenKind::Dot
        | TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::DotDot
        | TokenKind::DotDotDot
        | TokenKind::DotDotLt
        | TokenKind::ColonColon
        | TokenKind::Question => Some(FoundNode::Operator),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use action_frontend::lexer::{Lexer, TokenKind};

    fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        lexer.tokenize()
    }

    #[test]
    fn test_line_col_to_offset_empty() {
        assert_eq!(line_col_to_offset("", 1, 1), 0);
    }

    #[test]
    fn test_line_col_to_offset_basic() {
        let src = "hello\nworld";
        assert_eq!(line_col_to_offset(src, 1, 1), 0);
        assert_eq!(line_col_to_offset(src, 1, 3), 2);
        assert_eq!(line_col_to_offset(src, 2, 1), 6);
        assert_eq!(line_col_to_offset(src, 2, 5), 10);
    }

    #[test]
    fn test_line_col_to_offset_beyond_end() {
        assert_eq!(line_col_to_offset("hi", 99, 1), 2);
    }

    #[test]
    fn test_offset_to_line_col_simple() {
        let src = "abc";
        assert_eq!(offset_to_line_col(src, 0), (1, 1));
        assert_eq!(offset_to_line_col(src, 2), (1, 3));
        assert_eq!(offset_to_line_col(src, 3), (1, 4));
    }

    #[test]
    fn test_offset_to_line_col_newlines() {
        let src = "ab\ncd\nef";
        assert_eq!(offset_to_line_col(src, 0), (1, 1));
        assert_eq!(offset_to_line_col(src, 2), (1, 3));
        assert_eq!(offset_to_line_col(src, 3), (2, 1));
        assert_eq!(offset_to_line_col(src, 5), (2, 3));
        assert_eq!(offset_to_line_col(src, 6), (3, 1));
    }

    #[test]
    fn test_offset_to_lsp_position() {
        let src = "val x = 1\nval y = 2";
        let pos = offset_to_lsp_position(src, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
        let pos2 = offset_to_lsp_position(src, 10);
        assert_eq!(pos2.line, 1);
        assert_eq!(pos2.character, 0);
    }

    #[test]
    fn test_lsp_position_to_offset_roundtrip() {
        let src = "hello world\nfoo bar\nbaz";
        let offsets = [0, 5, 11, 14];
        for &offset in &offsets {
            let pos = offset_to_lsp_position(src, offset);
            let got = lsp_position_to_offset(src, &pos);
            assert_eq!(got, offset, "roundtrip failed at offset {}", offset);
        }
    }

    #[test]
    fn test_span_to_lsp_range() {
        let src = "val x = 42";
        let span = Span::new(0, 1, 1).with_end(3);
        let range = span_to_lsp_range(&span, src);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.start.character, 0);
        assert_eq!(range.end.line, 0);
        assert_eq!(range.end.character, 3);
    }

    #[test]
    fn test_find_token_at_exact() {
        let tokens = tokenize("val x = 42");
        let tok = find_token_at(&tokens, 0).unwrap();
        assert_eq!(tok.kind, TokenKind::Val);
    }

    #[test]
    fn test_find_token_at_middle() {
        let tokens = tokenize("val x = 42");
        let tok = find_token_at(&tokens, 6).unwrap();
        assert_eq!(tok.kind, TokenKind::Eq);
    }

    #[test]
    fn test_find_token_at_out_of_range() {
        let tokens = tokenize("val x");
        assert!(find_token_at(&tokens, 999).is_none());
    }

    #[test]
    fn test_find_token_at_empty() {
        let tokens: Vec<Token> = vec![];
        assert!(find_token_at(&tokens, 0).is_none());
    }

    #[test]
    fn test_find_node_at_keyword() {
        let tokens = tokenize("val x = 1");
        let pos = Position {
            line: 0,
            character: 0,
        };
        let node = find_node_at(&tokens, "val x = 1", &pos);
        assert!(node.is_some());
        match node.unwrap() {
            FoundNode::Keyword(_) => {}
            _ => panic!("expected Keyword, got something else"),
        }
    }

    #[test]
    fn test_find_node_at_ident() {
        let tokens = tokenize("val x = 1");
        let pos = Position {
            line: 0,
            character: 4,
        };
        let node = find_node_at(&tokens, "val x = 1", &pos);
        assert!(node.is_some());
        match node.unwrap() {
            FoundNode::Ident(name) => assert_eq!(name, "x"),
            _ => panic!("expected Ident, got something else"),
        }
    }
}
