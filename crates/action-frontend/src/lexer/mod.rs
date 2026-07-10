//! Lexer for the Action language.

mod json;
mod scan;
mod token;

#[cfg(test)]
mod tests;

pub use action_span::Span;
pub use json::tokens_to_json;
pub use scan::Lexer;
pub use token::{Token, TokenKind};
