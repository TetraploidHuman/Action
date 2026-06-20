use lsp_types::{DocumentSymbol, SemanticToken, SemanticTokenType, SymbolKind};
use std::collections::HashMap;
use std::sync::OnceLock;

use action_frontend::ast::*;
use action_frontend::lexer::{Span, Token, TokenKind};

use super::position;

fn blank_document_symbol() -> DocumentSymbol {
    static BLANK: OnceLock<DocumentSymbol> = OnceLock::new();
    BLANK
        .get_or_init(|| {
            serde_json::from_value(serde_json::json!({
                "name": "",
                "kind": 3,
                "range": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 0}
                },
                "selectionRange": {
                    "start": {"line": 0, "character": 0},
                    "end": {"line": 0, "character": 0}
                },
            }))
            .expect("blank DocumentSymbol")
        })
        .clone()
}

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

fn classify_ident_by_type_env(
    name: &str,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
    definition_map: &HashMap<String, Span>,
) -> Option<(u32, u32)> {
    if let Some(ty) = type_env.get(name).or_else(|| stdlib_type_env.get(name)) {
        return match ty {
            Type::Function(..) => Some((TYPE_FUNCTION, 0)),
            Type::Named(n) if n.chars().next().is_some_and(|c| c.is_uppercase()) => {
                Some((TYPE_TYPE, MOD_DECLARATION))
            }
            _ => Some((TYPE_VARIABLE, 0)),
        };
    }
    if definition_map.contains_key(name) && name.chars().next().is_some_and(|c| c.is_uppercase()) {
        return Some((TYPE_TYPE, MOD_DECLARATION));
    }
    None
}

fn classify_token(
    token: &Token,
    prev_kind: Option<&TokenKind>,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
    definition_map: &HashMap<String, Span>,
) -> Option<(u32, u32)> {
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

        // Identifiers — type_env first, then keyword context
        TokenKind::Ident(name) => {
            classify_ident_by_type_env(name, type_env, stdlib_type_env, definition_map).or_else(
                || match prev_kind {
                    Some(TokenKind::Fun) => Some((TYPE_FUNCTION, MOD_DECLARATION)),
                    Some(TokenKind::Val) => Some((TYPE_VARIABLE, MOD_DECLARATION | MOD_READONLY)),
                    Some(TokenKind::Var) => Some((TYPE_VARIABLE, MOD_DECLARATION)),
                    Some(TokenKind::Const) => Some((TYPE_VARIABLE, MOD_DECLARATION | MOD_READONLY)),
                    Some(TokenKind::Enum) => Some((TYPE_ENUM_MEMBER, MOD_DECLARATION)),
                    Some(TokenKind::Type) => Some((TYPE_TYPE, MOD_DECLARATION)),
                    Some(TokenKind::Module) => Some((TYPE_VARIABLE, MOD_DECLARATION)),
                    _ => Some((TYPE_VARIABLE, 0)),
                },
            )
        }

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
        | TokenKind::At
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

/// Compute semantic tokens from a token list using delta encoding and type_env hints.
pub fn compute_semantic_tokens(
    tokens: &[Token],
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
    definition_map: &HashMap<String, Span>,
) -> Vec<SemanticToken> {
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

        if let Some((token_type, modifiers)) =
            classify_token(token, prev_kind, type_env, stdlib_type_env, definition_map)
        {
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
                ..blank_document_symbol()
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
                ..blank_document_symbol()
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
                ..blank_document_symbol()
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
                        ..blank_document_symbol()
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
                ..blank_document_symbol()
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
            ..blank_document_symbol()
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
                ..blank_document_symbol()
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
            ..blank_document_symbol()
        }),
        _ => None,
    }
}

fn extract_body_symbols(body: &Expr, source: &str) -> Vec<DocumentSymbol> {
    match &body.kind {
        ExprKind::Block(stmts) => extract_document_symbols(stmts, source),
        _ => Vec::new(),
    }
}

fn infer_value_type(value: &Expr) -> String {
    match &value.kind {
        ExprKind::Literal(Literal::Int(_)) => "Int".to_string(),
        ExprKind::Literal(Literal::Float(_)) => "Float".to_string(),
        ExprKind::Literal(Literal::Bool(_)) => "Bool".to_string(),
        ExprKind::Literal(Literal::String(_)) => "String".to_string(),
        ExprKind::Literal(Literal::Char(_)) => "Char".to_string(),
        ExprKind::Literal(Literal::Unit) => "()".to_string(),
        ExprKind::MapLiteral(_) => "map".to_string(),
        ExprKind::SetLiteral(_) => "set".to_string(),
        ExprKind::Lambda { .. } => "Function".to_string(),
        ExprKind::StructLiteral(_) => "Struct".to_string(),
        ExprKind::Ident(name) => name.clone(),
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
        Type::TypeVar(name) => name.clone(),
        Type::Unit => "()".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use action_frontend::lexer::{Lexer, TokenKind};
    use std::collections::HashMap;

    fn empty_envs() -> (
        HashMap<String, Type>,
        HashMap<String, Type>,
        HashMap<String, Span>,
    ) {
        (HashMap::new(), HashMap::new(), HashMap::new())
    }

    fn classify(token: &Token, prev: Option<&TokenKind>) -> Option<(u32, u32)> {
        let (te, ste, dm) = empty_envs();
        classify_token(token, prev, &te, &ste, &dm)
    }

    fn tokenize(source: &str) -> Vec<Token> {
        let mut lexer = Lexer::new(source);
        lexer.tokenize()
    }

    #[test]
    fn test_classify_keyword() {
        let tokens = tokenize("val x = 42");
        // Token "val" is a keyword
        let token = &tokens[0];
        let result = classify(token, None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, TYPE_KEYWORD);
    }

    #[test]
    fn test_classify_ident_as_declaration() {
        let tokens = tokenize("fun hello() {}");
        // Token "hello" follows "fun"
        let fun_token = &tokens[0];
        let hello_token = &tokens[1];
        let result = classify(hello_token, Some(&fun_token.kind));
        assert!(result.is_some());
        let (ttype, mods) = result.unwrap();
        assert_eq!(ttype, TYPE_FUNCTION);
        assert_eq!(mods & MOD_DECLARATION, MOD_DECLARATION);
    }

    #[test]
    fn test_classify_ident_as_val() {
        let tokens = tokenize("val x = 42");
        let val_token = &tokens[0];
        let x_token = &tokens[1];
        let result = classify(x_token, Some(&val_token.kind));
        assert!(result.is_some());
        let (ttype, mods) = result.unwrap();
        assert_eq!(ttype, TYPE_VARIABLE);
        assert_eq!(mods & MOD_DECLARATION, MOD_DECLARATION);
        assert_eq!(mods & MOD_READONLY, MOD_READONLY);
    }

    #[test]
    fn test_classify_number() {
        let tokens = tokenize("42");
        let result = classify(&tokens[0], None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, TYPE_NUMBER);
    }

    #[test]
    fn test_classify_string() {
        let tokens = tokenize("\"hello\"");
        let result = classify(&tokens[0], None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, TYPE_STRING);
    }

    #[test]
    fn test_classify_operator() {
        let tokens = tokenize("a + b");
        let result = classify(&tokens[1], None);
        assert!(result.is_some());
        assert_eq!(result.unwrap().0, TYPE_OPERATOR);
    }

    #[test]
    fn test_classify_delimiter_none() {
        let tokens = tokenize("(x)");
        // LParen and RParen should return None
        for t in &tokens {
            if matches!(t.kind, TokenKind::LParen | TokenKind::RParen) {
                assert!(classify(t, None).is_none());
            }
        }
    }

    #[test]
    fn test_compute_semantic_tokens_empty() {
        let (te, ste, dm) = empty_envs();
        let result = compute_semantic_tokens(&[], &te, &ste, &dm);
        assert!(result.is_empty());
    }

    #[test]
    fn test_compute_semantic_tokens_basic() {
        let tokens = tokenize("val x = 42");
        let (te, ste, dm) = empty_envs();
        let result = compute_semantic_tokens(&tokens, &te, &ste, &dm);
        assert!(!result.is_empty(), "should produce semantic tokens");
        // First token "val" should be a keyword
        assert_eq!(result[0].token_type, TYPE_KEYWORD);
    }

    #[test]
    fn test_type_to_detail_named() {
        assert_eq!(type_to_detail(&Type::Named("Int".into())), "Int");
    }

    #[test]
    fn test_type_to_detail_generic() {
        let ty = Type::Generic(
            Box::new(Type::Named("List".into())),
            vec![Type::Named("Int".into())],
        );
        assert_eq!(type_to_detail(&ty), "List[Int]");
    }

    #[test]
    fn test_type_to_detail_nullable() {
        let ty = Type::Nullable(Box::new(Type::Named("String".into())));
        assert_eq!(type_to_detail(&ty), "String?");
    }

    #[test]
    fn test_type_to_detail_function() {
        let ty = Type::Function(
            vec![Type::Named("Int".into()), Type::Named("String".into())],
            Box::new(Type::Named("Bool".into())),
        );
        assert_eq!(type_to_detail(&ty), "(Int, String) -> Bool");
    }

    #[test]
    fn test_infer_value_type_int() {
        let expr: Expr = ExprKind::Literal(Literal::Int(42)).into();
        assert_eq!(infer_value_type(&expr), "Int");
    }

    #[test]
    fn test_infer_value_type_float() {
        let expr: Expr = ExprKind::Literal(Literal::Float(3.14)).into();
        assert_eq!(infer_value_type(&expr), "Float");
    }

    #[test]
    fn test_infer_value_type_string() {
        let expr: Expr = ExprKind::Literal(Literal::String("hi".into())).into();
        assert_eq!(infer_value_type(&expr), "String");
    }

    #[test]
    fn test_extract_document_symbols_empty() {
        let result = extract_document_symbols(&[], "");
        assert!(result.is_empty());
    }

    #[test]
    fn test_extract_document_symbols_fun() {
        let source = "fun hello() {}";
        let session = action_frontend::session::FrontendSession::with_context(
            Vec::new(),
            action_frontend::typecheck::TypeRegistry::new(),
            HashMap::new(),
        )
        .unwrap();
        let result = session.compile_recover_buffer(source);
        let symbols = extract_document_symbols(&result.stmts, source);
        assert!(!symbols.is_empty(), "should extract function symbol");
        assert_eq!(symbols[0].name, "hello");
        assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
    }

    #[test]
    fn test_extract_document_symbols_val() {
        let source = "val x = 42";
        let session = action_frontend::session::FrontendSession::with_context(
            Vec::new(),
            action_frontend::typecheck::TypeRegistry::new(),
            HashMap::new(),
        )
        .unwrap();
        let result = session.compile_recover_buffer(source);
        let symbols = extract_document_symbols(&result.stmts, source);
        assert!(!symbols.is_empty(), "should extract val symbol");
        assert_eq!(symbols[0].name, "x");
        assert_eq!(symbols[0].kind, SymbolKind::VARIABLE);
    }
}
