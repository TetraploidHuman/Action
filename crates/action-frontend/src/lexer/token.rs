use std::fmt;

use super::Span;

/// All token types in the Atomic language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Val,
    Var,
    Fun,
    When,
    Else,
    For,
    In,
    Is,
    Break,
    Continue,
    Return,
    Enum,
    Type,
    Import,
    Module,
    Export,
    Const,
    Copy,
    Extension,
    And,
    Or,
    Not,
    As,
    Lazy,
    Unsafe,
    External,
    Null,
    Task,

    // Literals
    IntLiteral(i64),
    FloatLiteral(f64),
    BoolLiteral(bool),
    StringLiteral(String),
    CharLiteral(char),

    // Identifiers
    Ident(String),

    // Operators
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    PlusEq,     // +=
    MinusEq,    // -=
    StarEq,     // *=
    SlashEq,    // /=
    PercentEq,  // %=
    StarStar,   // **
    Ampersand,  // &
    Pipe,       // |
    Caret,      // ^
    Tilde,      // ~
    Shl,        // <<
    Shr,        // >>
    Eq,         // =
    EqEq,       // ==
    Neq,        // !=
    Lt,         // <
    Gt,         // >
    Lte,        // <=
    Gte,        // >=
    Arrow,      // ->
    FatArrow,   // =>
    Dot,        // .
    DotDot,     // ..
    DotDotLt,   // ..<
    DotDotDot,  // ...
    Colon,      // :
    ColonColon, // ::
    Comma,      // ,
    Semicolon,  // ;
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    Underscore, // _
    Question,   // ?

    // Special
    Eof,
    At, // @
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Val => write!(f, "val"),
            TokenKind::Var => write!(f, "var"),
            TokenKind::Fun => write!(f, "fun"),
            TokenKind::When => write!(f, "when"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::For => write!(f, "for"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Is => write!(f, "is"),
            TokenKind::Break => write!(f, "break"),
            TokenKind::Continue => write!(f, "continue"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Module => write!(f, "module"),
            TokenKind::Export => write!(f, "export"),
            TokenKind::Const => write!(f, "const"),
            TokenKind::Copy => write!(f, "copy"),
            TokenKind::Extension => write!(f, "extension"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "!"),
            TokenKind::As => write!(f, "as"),
            TokenKind::Lazy => write!(f, "lazy"),
            TokenKind::Unsafe => write!(f, "unsafe"),
            TokenKind::External => write!(f, "external"),
            TokenKind::Null => write!(f, "null"),
            TokenKind::Task => write!(f, "Task"),
            TokenKind::IntLiteral(n) => write!(f, "{}", n),
            TokenKind::FloatLiteral(n) => write!(f, "{}", n),
            TokenKind::BoolLiteral(b) => write!(f, "{}", b),
            TokenKind::StringLiteral(s) => write!(f, "\"{}\"", s),
            TokenKind::CharLiteral(c) => write!(f, "'{}'", c),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::PlusEq => write!(f, "+="),
            TokenKind::MinusEq => write!(f, "-="),
            TokenKind::StarEq => write!(f, "*="),
            TokenKind::SlashEq => write!(f, "/="),
            TokenKind::PercentEq => write!(f, "%="),
            TokenKind::StarStar => write!(f, "**"),
            TokenKind::Ampersand => write!(f, "&"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Caret => write!(f, "^"),
            TokenKind::Tilde => write!(f, "~"),
            TokenKind::Shl => write!(f, "<<"),
            TokenKind::Shr => write!(f, ">>"),
            TokenKind::Eq => write!(f, "="),
            TokenKind::EqEq => write!(f, "=="),
            TokenKind::Neq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::Lte => write!(f, "<="),
            TokenKind::Gte => write!(f, ">="),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::Dot => write!(f, "."),
            TokenKind::DotDot => write!(f, ".."),
            TokenKind::DotDotLt => write!(f, "..<"),
            TokenKind::DotDotDot => write!(f, "..."),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::ColonColon => write!(f, "::"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::Underscore => write!(f, "_"),
            TokenKind::At => write!(f, "@"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::Eof => write!(f, "<eof>"),
        }
    }
}

/// A token with its source location
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Token { kind, span }
    }
}
