use crate::lexer::{Span, Token, TokenKind};
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
        | TokenKind::Task => Some(FoundNode::Keyword(format!("{:?}", token.kind))),
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
