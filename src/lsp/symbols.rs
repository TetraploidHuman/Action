use lsp_types::{DocumentSymbol, SemanticToken, SemanticTokenType, SymbolKind};

use crate::ast::*;
use crate::lexer::{Token, TokenKind};

use super::position;

/// Legend for semantic tokens
pub fn get_semantic_tokens_legend() -> lsp_types::SemanticTokensLegend {
    lsp_types::SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::KEYWORD,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::TYPE,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::OPERATOR,
        ],
        token_modifiers: vec![
            lsp_types::SemanticTokenModifier::DECLARATION,
            lsp_types::SemanticTokenModifier::READONLY,
        ],
    }
}

const TYPE_KEYWORD: u32 = 0;
const TYPE_VARIABLE: u32 = 1;
const TYPE_FUNCTION: u32 = 2;
const TYPE_STRING: u32 = 3;
const TYPE_NUMBER: u32 = 4;
const TYPE_TYPE: u32 = 5;
const TYPE_ENUM_MEMBER: u32 = 6;
const TYPE_OPERATOR: u32 = 7;

const MOD_DECLARATION: u32 = 0;
const MOD_READONLY: u32 = 1;

fn classify_token(token: &Token, prev_kind: Option<&TokenKind>) -> Option<(u32, u32)> {
    match &token.kind {
        // Keywords
        TokenKind::Null
        | TokenKind::Val
        | TokenKind::Var
        | TokenKind::Fun
        | TokenKind::When
        | TokenKind::Else
        | TokenKind::For
        | TokenKind::In
        | TokenKind::Is
        | TokenKind::Break
        | TokenKind::Continue
        | TokenKind::Return
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
        | TokenKind::Task => Some((TYPE_KEYWORD, 0)),

        // Identifiers — classify by preceding keyword context
        TokenKind::Ident(_name) => match prev_kind {
            Some(TokenKind::Fun) => Some((TYPE_FUNCTION, MOD_DECLARATION)),
            Some(TokenKind::Val) => Some((TYPE_VARIABLE, MOD_DECLARATION | MOD_READONLY)),
            Some(TokenKind::Var) => Some((TYPE_VARIABLE, MOD_DECLARATION)),
            Some(TokenKind::Const) => Some((TYPE_VARIABLE, MOD_DECLARATION | MOD_READONLY)),
            Some(TokenKind::Enum) => Some((TYPE_ENUM_MEMBER, MOD_DECLARATION)),
            Some(TokenKind::Type) => Some((TYPE_TYPE, MOD_DECLARATION)),
            Some(TokenKind::Module) => Some((TYPE_VARIABLE, MOD_DECLARATION)),
            _ => Some((TYPE_VARIABLE, 0)),
        },

        // Literals
        TokenKind::IntLiteral(_) | TokenKind::FloatLiteral(_) => Some((TYPE_NUMBER, 0)),
        TokenKind::BoolLiteral(_) => Some((TYPE_KEYWORD, 0)),
        TokenKind::StringLiteral(_) | TokenKind::CharLiteral(_) => Some((TYPE_STRING, 0)),

        // Operators
        TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::PlusEq
        | TokenKind::MinusEq
        | TokenKind::StarEq
        | TokenKind::SlashEq
        | TokenKind::PercentEq
        | TokenKind::StarStar
        | TokenKind::Ampersand
        | TokenKind::Pipe
        | TokenKind::Caret
        | TokenKind::Tilde
        | TokenKind::Shl
        | TokenKind::Shr
        | TokenKind::Eq
        | TokenKind::EqEq
        | TokenKind::Neq
        | TokenKind::Lt
        | TokenKind::Gt
        | TokenKind::Lte
        | TokenKind::Gte
        | TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::Dot
        | TokenKind::DotDot
        | TokenKind::DotDotLt
        | TokenKind::DotDotDot
        | TokenKind::Colon
        | TokenKind::ColonColon
        | TokenKind::Question => Some((TYPE_OPERATOR, 0)),

        // Delimiters — not highlighted
        TokenKind::LParen
        | TokenKind::RParen
        | TokenKind::LBrace
        | TokenKind::RBrace
        | TokenKind::LBracket
        | TokenKind::RBracket
        | TokenKind::Comma
        | TokenKind::Semicolon
        | TokenKind::Underscore
        | TokenKind::Eof => None,
    }
}

/// Compute semantic tokens from a token list using delta encoding
pub fn compute_semantic_tokens(tokens: &[Token]) -> Vec<SemanticToken> {
    let mut result = Vec::new();
    let mut prev_line: u32 = 0;
    let mut prev_start: u32 = 0;

    for i in 0..tokens.len() {
        let token = &tokens[i];
        let prev_kind = if i > 0 {
            Some(&tokens[i - 1].kind)
        } else {
            None
        };

        if let Some((token_type, modifiers)) = classify_token(token, prev_kind) {
            // span positions are 1-indexed; LSP is 0-indexed
            let line = (token.span.line as u32).saturating_sub(1);
            let start = (token.span.col as u32).saturating_sub(1);
            let len = (token.span.end - token.span.start) as u32;

            let delta_line = line.saturating_sub(prev_line);
            let delta_start = if delta_line == 0 {
                start.saturating_sub(prev_start)
            } else {
                start
            };

            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: len,
                token_type,
                token_modifiers_bitset: modifiers,
            });

            prev_line = line;
            prev_start = start;
        }
    }

    result
}

/// Extract document symbols from AST statements
pub fn extract_document_symbols(stmts: &[Stmt], source: &str) -> Vec<DocumentSymbol> {
    stmts
        .iter()
        .filter_map(|s| stmt_to_document_symbol(s, source))
        .collect()
}

#[allow(deprecated)]
fn stmt_to_document_symbol(stmt: &Stmt, source: &str) -> Option<DocumentSymbol> {
    match stmt {
        Stmt::Fun {
            name,
            params,
            return_type,
            body,
            span,
            ..
        } => {
            let detail = if let Some(rt) = return_type {
                format!("fun {}{}", name, type_to_detail(rt))
            } else {
                format!(
                    "fun {}({})",
                    name,
                    params
                        .iter()
                        .map(|p| {
                            if let Some(ty) = &p.ty {
                                format!("{}: {}", p.name, type_to_detail(ty))
                            } else {
                                p.name.clone()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            let children = extract_body_symbols(body, source);
            Some(DocumentSymbol {
                name: name.clone(),
                detail: Some(detail),
                kind: SymbolKind::FUNCTION,
                range: position::span_to_lsp_range(span, source),
                selection_range: position::span_to_lsp_range(span, source),
                children: Some(children),
                tags: None,
                deprecated: None,
            })
        }
        Stmt::Let {
            name, value, span, ..
        } => {
            let ty_str = infer_value_type(value);
            Some(DocumentSymbol {
                name: name.clone(),
                detail: Some(format!("let {}: {}", name, ty_str)),
                kind: SymbolKind::VARIABLE,
                range: position::span_to_lsp_range(span, source),
                selection_range: position::span_to_lsp_range(span, source),
                children: None,
                tags: None,
                deprecated: None,
            })
        }
        Stmt::Const {
            name, value, span, ..
        } => {
            let ty_str = infer_value_type(value);
            Some(DocumentSymbol {
                name: name.clone(),
                detail: Some(format!("const {}: {}", name, ty_str)),
                kind: SymbolKind::CONSTANT,
                range: position::span_to_lsp_range(span, source),
                selection_range: position::span_to_lsp_range(span, source),
                children: None,
                tags: None,
                deprecated: None,
            })
        }
        Stmt::Enum {
            name,
            variants,
            span,
            ..
        } => {
            let children: Vec<DocumentSymbol> = variants
                .iter()
                .map(|v| {
                    let detail = if v.params.is_empty() {
                        String::new()
                    } else {
                        format!(
                            "({})",
                            v.params
                                .iter()
                                .map(|p| format!("{:?}", p))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    };
                    DocumentSymbol {
                        name: v.name.clone(),
                        detail: Some(format!("{}{}", v.name, detail)),
                        kind: SymbolKind::ENUM_MEMBER,
                        range: position::span_to_lsp_range(span, source),
                        selection_range: position::span_to_lsp_range(span, source),
                        children: None,
                        tags: None,
                        deprecated: None,
                    }
                })
                .collect();
            Some(DocumentSymbol {
                name: name.clone(),
                detail: Some(format!("enum {}", name)),
                kind: SymbolKind::ENUM,
                range: position::span_to_lsp_range(span, source),
                selection_range: position::span_to_lsp_range(span, source),
                children: Some(children),
                tags: None,
                deprecated: None,
            })
        }
        Stmt::TypeAlias { name, span, .. } => Some(DocumentSymbol {
            name: name.clone(),
            detail: Some(format!("type {}", name)),
            kind: SymbolKind::TYPE_PARAMETER,
            range: position::span_to_lsp_range(span, source),
            selection_range: position::span_to_lsp_range(span, source),
            children: None,
            tags: None,
            deprecated: None,
        }),
        Stmt::Module {
            name, body, span, ..
        } => {
            let children = extract_document_symbols(body, source);
            Some(DocumentSymbol {
                name: name.clone(),
                detail: Some(format!("module {}", name)),
                kind: SymbolKind::MODULE,
                range: position::span_to_lsp_range(span, source),
                selection_range: position::span_to_lsp_range(span, source),
                children: Some(children),
                tags: None,
                deprecated: None,
            })
        }
        Stmt::Destructure { names, span, .. } => Some(DocumentSymbol {
            name: names.join(", "),
            detail: Some(format!("destructure {}", names.join(", "))),
            kind: SymbolKind::VARIABLE,
            range: position::span_to_lsp_range(span, source),
            selection_range: position::span_to_lsp_range(span, source),
            children: None,
            tags: None,
            deprecated: None,
        }),
        _ => None,
    }
}

fn extract_body_symbols(body: &Expr, source: &str) -> Vec<DocumentSymbol> {
    match body {
        Expr::Block(stmts) => extract_document_symbols(stmts, source),
        _ => Vec::new(),
    }
}

fn infer_value_type(value: &Expr) -> String {
    match value {
        Expr::Literal(Literal::Int(_)) => "Int".to_string(),
        Expr::Literal(Literal::Float(_)) => "Float".to_string(),
        Expr::Literal(Literal::Bool(_)) => "Bool".to_string(),
        Expr::Literal(Literal::String(_)) => "String".to_string(),
        Expr::Literal(Literal::Char(_)) => "Char".to_string(),
        Expr::Literal(Literal::Unit) => "()".to_string(),
        Expr::MapLiteral(_) => "Map".to_string(),
        Expr::SetLiteral(_) => "Set".to_string(),
        Expr::Lambda { .. } => "Function".to_string(),
        Expr::StructLiteral(_) => "Struct".to_string(),
        Expr::Ident(name) => name.clone(),
        _ => "?".to_string(),
    }
}

fn type_to_detail(ty: &Type) -> String {
    match ty {
        Type::Named(n) => n.clone(),
        Type::Generic(base, args) => {
            let args_str: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
            format!("{}[{}]", base, args_str.join(", "))
        }
        Type::Function(params, ret) => {
            let params_str: Vec<String> = params.iter().map(type_to_detail).collect();
            format!("({}) -> {}", params_str.join(", "), type_to_detail(ret))
        }
        Type::Struct(fields) => {
            let fs: Vec<String> = fields
                .iter()
                .map(|(n, t)| format!("{}: {}", n, type_to_detail(t)))
                .collect();
            format!("{{{}}}", fs.join(", "))
        }
        Type::Map(k, v) => format!("Map<{}, {}>", type_to_detail(k), type_to_detail(v)),
        Type::Set(t) => format!("Set<{}>", type_to_detail(t)),
        Type::Task(t) => format!("Task<{}>", type_to_detail(t)),
        Type::Stream(t) => format!("Stream<{}>", type_to_detail(t)),
        Type::LazyList(t) => format!("LazyList<{}>", type_to_detail(t)),
        Type::CString => "CString".to_string(),
        Type::Ptr(t) => format!("Ptr<{}>", type_to_detail(t)),
        Type::FileHandle => "FileHandle".to_string(),
        Type::Nullable(t) => format!("{}?", type_to_detail(t)),
        Type::Unit => "()".to_string(),
    }
}
