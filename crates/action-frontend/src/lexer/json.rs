use super::token::{Token, TokenKind};
use super::Span;

#[derive(serde::Serialize)]
struct TokenJson {
    kind: String,
    span: Span,
}

/// Stable JSON for lexer golden tests (kinds via Display, spans included).
pub fn tokens_to_json(tokens: &[Token]) -> String {
    let items: Vec<TokenJson> = tokens
        .iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .map(|t| TokenJson {
            kind: t.kind.to_string(),
            span: t.span,
        })
        .collect();
    serde_json::to_string_pretty(&items).expect("token json")
}
