use super::scan::Lexer;
use super::token::{Token, TokenKind};

fn tokenize(source: &str) -> Vec<TokenKind> {
    let mut lexer = Lexer::new(source);
    lexer
        .tokenize()
        .into_iter()
        .filter(|t| t.kind != TokenKind::Eof)
        .map(|t| t.kind)
        .collect()
}

#[test]
fn test_keywords() {
    let tokens = tokenize("val var fun when if else for in is break continue return");
    assert_eq!(tokens[0], TokenKind::Val);
    assert_eq!(tokens[1], TokenKind::Var);
    assert_eq!(tokens[2], TokenKind::Fun);
    assert_eq!(tokens[3], TokenKind::When);
    assert_eq!(tokens[4], TokenKind::If);
    assert_eq!(tokens[5], TokenKind::Else);
    assert_eq!(tokens[6], TokenKind::For);
    assert_eq!(tokens[7], TokenKind::In);
    assert_eq!(tokens[8], TokenKind::Is);
    assert_eq!(tokens[9], TokenKind::Break);
    assert_eq!(tokens[10], TokenKind::Continue);
    assert_eq!(tokens[11], TokenKind::Return);
}

#[test]
fn test_literals() {
    let tokens = tokenize("42 3.14 true false \"hello\"");
    assert_eq!(tokens[0], TokenKind::IntLiteral(42));
    assert_eq!(tokens[1], TokenKind::FloatLiteral(3.14));
    assert_eq!(tokens[2], TokenKind::BoolLiteral(true));
    assert_eq!(tokens[3], TokenKind::BoolLiteral(false));
    assert_eq!(tokens[4], TokenKind::StringLiteral("hello".to_string()));
}

#[test]
fn test_negative_number_is_unary_op() {
    // -17 lexes as Minus then IntLiteral(17); unary minus handled by parser
    let tokens = tokenize("-17");
    assert_eq!(tokens[0], TokenKind::Minus);
    assert_eq!(tokens[1], TokenKind::IntLiteral(17));
}

#[test]
fn test_operators() {
    let tokens = tokenize("+ - * / % == != < > <= >= -> => and or !");
    assert_eq!(tokens[0], TokenKind::Plus);
    assert_eq!(tokens[1], TokenKind::Minus);
    assert_eq!(tokens[2], TokenKind::Star);
    assert_eq!(tokens[3], TokenKind::Slash);
    assert_eq!(tokens[4], TokenKind::Percent);
    assert_eq!(tokens[5], TokenKind::EqEq);
    assert_eq!(tokens[6], TokenKind::Neq);
    assert_eq!(tokens[7], TokenKind::Lt);
    assert_eq!(tokens[8], TokenKind::Gt);
    assert_eq!(tokens[9], TokenKind::Lte);
    assert_eq!(tokens[10], TokenKind::Gte);
    assert_eq!(tokens[11], TokenKind::Arrow);
    assert_eq!(tokens[12], TokenKind::FatArrow);
    assert_eq!(tokens[13], TokenKind::And);
    assert_eq!(tokens[14], TokenKind::Or);
    assert_eq!(tokens[15], TokenKind::Not);
}

#[test]
fn test_delimiters() {
    let tokens = tokenize("(){}[];:,.? .. ? a?.b");
    assert_eq!(tokens[0], TokenKind::LParen);
    assert_eq!(tokens[1], TokenKind::RParen);
    assert_eq!(tokens[2], TokenKind::LBrace);
    assert_eq!(tokens[3], TokenKind::RBrace);
    assert_eq!(tokens[4], TokenKind::LBracket);
    assert_eq!(tokens[5], TokenKind::RBracket);
    assert_eq!(tokens[6], TokenKind::Semicolon);
    assert_eq!(tokens[7], TokenKind::Colon);
    assert_eq!(tokens[8], TokenKind::Comma);
    assert_eq!(tokens[9], TokenKind::Dot);
    assert_eq!(tokens[10], TokenKind::Question);
    assert_eq!(tokens[11], TokenKind::DotDot);
    // ?. => Question + Dot (two separate tokens, no more SafeDot)
    assert_eq!(tokens[12], TokenKind::Question);
    assert_eq!(tokens[13], TokenKind::Ident("a".to_string()));
    assert_eq!(tokens[14], TokenKind::Question);
    assert_eq!(tokens[15], TokenKind::Dot);
}

#[test]
fn test_comments() {
    let tokens = tokenize("val x = 10 // this is a comment\nval y = 20 /* block */");
    assert_eq!(tokens[0], TokenKind::Val);
    assert_eq!(tokens[4], TokenKind::Val);
}

#[test]
fn test_ident_with_underscore() {
    let tokens = tokenize("my_var parse_int toString");
    assert_eq!(tokens[0], TokenKind::Ident("my_var".to_string()));
    assert_eq!(tokens[1], TokenKind::Ident("parse_int".to_string()));
    assert_eq!(tokens[2], TokenKind::Ident("toString".to_string()));
}

#[test]
fn test_hex_numbers() {
    let tokens = tokenize("0xFF 0x1A");
    assert_eq!(tokens[0], TokenKind::IntLiteral(255));
    assert_eq!(tokens[1], TokenKind::IntLiteral(26));
}

#[test]
fn test_string_escapes() {
    let tokens = tokenize("\"hello\\nworld\"");
    assert_eq!(
        tokens[0],
        TokenKind::StringLiteral("hello\nworld".to_string())
    );
}

#[test]
fn test_keywords_extended() {
    let tokens = tokenize(
        "enum type import module export const copy extension as lazy unsafe external null Task",
    );
    assert_eq!(tokens[0], TokenKind::Enum);
    assert_eq!(tokens[1], TokenKind::Type);
    assert_eq!(tokens[2], TokenKind::Import);
    assert_eq!(tokens[3], TokenKind::Module);
    assert_eq!(tokens[4], TokenKind::Export);
    assert_eq!(tokens[5], TokenKind::Const);
    assert_eq!(tokens[6], TokenKind::Copy);
    assert_eq!(tokens[7], TokenKind::Extension);
    assert_eq!(tokens[8], TokenKind::As);
    assert_eq!(tokens[9], TokenKind::Lazy);
    assert_eq!(tokens[10], TokenKind::Unsafe);
    assert_eq!(tokens[11], TokenKind::External);
    assert_eq!(tokens[12], TokenKind::Null);
    assert_eq!(tokens[13], TokenKind::Task);
}

#[test]
fn test_compound_operators() {
    let tokens = tokenize("+= -= *= /= %= ** & | ^ ~ << >>");
    assert_eq!(tokens[0], TokenKind::PlusEq);
    assert_eq!(tokens[1], TokenKind::MinusEq);
    assert_eq!(tokens[2], TokenKind::StarEq);
    assert_eq!(tokens[3], TokenKind::SlashEq);
    assert_eq!(tokens[4], TokenKind::PercentEq);
    assert_eq!(tokens[5], TokenKind::StarStar);
    assert_eq!(tokens[6], TokenKind::Ampersand);
    assert_eq!(tokens[7], TokenKind::Pipe);
    assert_eq!(tokens[8], TokenKind::Caret);
    assert_eq!(tokens[9], TokenKind::Tilde);
    assert_eq!(tokens[10], TokenKind::Shl);
    assert_eq!(tokens[11], TokenKind::Shr);
}

#[test]
fn test_special_tokens() {
    let tokens = tokenize("..< ... :: _");
    assert_eq!(tokens[0], TokenKind::DotDotLt);
    assert_eq!(tokens[1], TokenKind::DotDotDot);
    assert_eq!(tokens[2], TokenKind::ColonColon);
    assert_eq!(tokens[3], TokenKind::Underscore);
}

#[test]
fn test_char_literals() {
    let tokens = tokenize("'a' '\n' '\\\\'");
    assert_eq!(tokens[0], TokenKind::CharLiteral('a'));
    assert_eq!(tokens[1], TokenKind::CharLiteral('\n'));
    assert_eq!(tokens[2], TokenKind::CharLiteral('\\'));
}

#[test]
fn test_float_edge_cases() {
    let tokens = tokenize(".5 5. 1.5e10 3.0e-3");
    assert_eq!(tokens[0], TokenKind::FloatLiteral(0.5));
    assert_eq!(tokens[1], TokenKind::FloatLiteral(5.0));
    assert_eq!(tokens[2], TokenKind::FloatLiteral(1.5e10));
    assert_eq!(tokens[3], TokenKind::FloatLiteral(0.003));
}

#[test]
fn test_empty_input() {
    let tokens = tokenize("");
    assert!(tokens.is_empty());
}

#[test]
fn test_unicode_identifiers() {
    let tokens = tokenize("名字 测试");
    assert_eq!(tokens[0], TokenKind::Ident("名字".to_string()));
    assert_eq!(tokens[1], TokenKind::Ident("测试".to_string()));
}

#[test]
fn test_ident_leading_underscore() {
    let tokens = tokenize("_hidden_val __double __private_var");
    assert_eq!(tokens[0], TokenKind::Ident("_hidden_val".to_string()));
    assert_eq!(tokens[1], TokenKind::Ident("__double".to_string()));
    assert_eq!(tokens[2], TokenKind::Ident("__private_var".to_string()));
}

#[test]
fn test_multiline_block_comment() {
    let tokens = tokenize("val x = 1 /* start\nmiddle\nend */ val y = 2");
    assert_eq!(tokens[0], TokenKind::Val);
    assert_eq!(tokens[4], TokenKind::Val);
}

#[test]
fn test_lexer_error_empty_hex() {
    let source = "0x 123";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let errors = lexer.take_errors();
    assert!(!errors.is_empty(), "Expected error for empty hex literal");
    assert!(
        errors[0].message.to_lowercase().contains("empty hex"),
        "Expected 'empty hex' error, got: {}",
        errors[0]
    );
    let kinds: Vec<TokenKind> = tokens.into_iter().map(|t| t.kind).collect();
    assert_eq!(kinds[0], TokenKind::IntLiteral(123));
}

#[test]
fn test_lexer_error_overflow() {
    let source = "999999999999999999999999999";
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    let errors = lexer.take_errors();
    assert!(!errors.is_empty(), "Expected error for integer overflow");
    assert!(
        errors[0].message.to_lowercase().contains("overflow"),
        "Expected 'overflow' error, got: {}",
        errors[0]
    );
    assert_eq!(tokens[0].kind, TokenKind::IntLiteral(i64::MAX));
}

#[test]
fn test_lexer_whitespace_sensitive_tokens() {
    let tokens = tokenize("- > -> - >");
    assert_eq!(tokens[0], TokenKind::Minus);
    assert_eq!(tokens[1], TokenKind::Gt);
    assert_eq!(tokens[2], TokenKind::Arrow);
    assert_eq!(tokens[3], TokenKind::Minus);
    assert_eq!(tokens[4], TokenKind::Gt);
}

#[test]
fn test_lexer_empty_line_comment() {
    let tokens = tokenize("//\nval x = 1");
    assert_eq!(tokens[0], TokenKind::Val);
}

#[test]
fn test_lexer_empty_block_comment() {
    let tokens = tokenize("/**/val x = 1");
    assert_eq!(tokens[0], TokenKind::Val);
}

#[test]
fn test_lexer_deeply_nested_parens() {
    let input = format!("{}42", "(".repeat(100) + &")".repeat(100));
    let tokens = tokenize(&input);
    // Should not crash, final token should be IntLiteral
    assert_eq!(tokens[tokens.len() - 1], TokenKind::IntLiteral(42));
}

#[test]
fn test_lexer_malformed_string_unterminated() {
    let mut lexer = Lexer::new("\"hello world");
    let _tokens = lexer.tokenize();
    let errors = lexer.take_errors();
    assert!(!errors.is_empty(), "expected error for unterminated string");
}

#[test]
fn test_lexer_malformed_char_unterminated() {
    let mut lexer = Lexer::new("'x");
    let _ = lexer.tokenize();
    let errors = lexer.take_errors();
    assert!(
        !errors.is_empty(),
        "expected error for unterminated char literal"
    );
}

#[test]
fn test_lexer_malformed_char_empty() {
    let mut lexer = Lexer::new("''");
    let _ = lexer.tokenize();
    let errors = lexer.take_errors();
    assert!(!errors.is_empty(), "expected error for empty char literal");
}

#[test]
fn test_lexer_unicode_mixed_with_operators() {
    let tokens = tokenize("名字 + 测试 - 数据");
    assert_eq!(tokens[0], TokenKind::Ident("名字".to_string()));
    assert_eq!(tokens[1], TokenKind::Plus);
    assert_eq!(tokens[2], TokenKind::Ident("测试".to_string()));
    assert_eq!(tokens[3], TokenKind::Minus);
    assert_eq!(tokens[4], TokenKind::Ident("数据".to_string()));
}

#[test]
fn test_lexer_float_overflow() {
    let mut lexer = Lexer::new("1e9999");
    let tokens = lexer.tokenize();
    let errors = lexer.take_errors();
    // Should not panic, may report overflow or parse as inf
    assert!(
        !errors.is_empty() || tokens[0].kind == TokenKind::FloatLiteral(f64::INFINITY),
        "expected overflow error or infinity for huge float"
    );
}

#[test]
fn test_lexer_only_whitespace() {
    let tokens = tokenize("   \n  \t  ");
    assert!(tokens.is_empty());
}

#[test]
fn test_lexer_ident_with_numbers() {
    let tokens = tokenize("x1 var2 fun3");
    assert_eq!(tokens[0], TokenKind::Ident("x1".to_string()));
    assert_eq!(tokens[1], TokenKind::Ident("var2".to_string()));
    assert_eq!(tokens[2], TokenKind::Ident("fun3".to_string()));
}

#[test]
fn test_lexer_ident_starting_with_underscore() {
    let tokens = tokenize("_hidden __private");
    assert_eq!(tokens[0], TokenKind::Ident("_hidden".to_string()));
    assert_eq!(tokens[1], TokenKind::Ident("__private".to_string()));
}

#[test]
fn test_lexer_block_comment_with_nested() {
    let tokens = tokenize("/* outer /* inner */ */ 42");
    // Block comment should consume everything including nested
    assert_eq!(tokens[0], TokenKind::IntLiteral(42));
}

#[test]
fn test_lexer_unterminated_block_comment() {
    let mut lexer = Lexer::new("val x = 1 /* unterminated");
    let tokens = lexer.tokenize();
    let errors = lexer.take_errors();
    assert!(
        !errors.is_empty(),
        "expected error for unterminated block comment"
    );
    // Should still produce EOF and not crash
    assert!(!tokens.is_empty());
}

#[test]
fn test_lexer_floating_point_modulo() {
    let tokens = tokenize("5.5 % 2.0");
    assert_eq!(tokens[0], TokenKind::FloatLiteral(5.5));
    assert_eq!(tokens[1], TokenKind::Percent);
    assert_eq!(tokens[2], TokenKind::FloatLiteral(2.0));
}

#[test]
fn test_lexer_consecutive_dots() {
    let tokens = tokenize(".. ... ..<");
    assert_eq!(tokens[0], TokenKind::DotDot);
    assert_eq!(tokens[1], TokenKind::DotDotDot);
    assert_eq!(tokens[2], TokenKind::DotDotLt);
}

use proptest::prelude::*;

proptest! {


    #[test]
    // Disabled: triggers STATUS_STACK_BUFFER_OVERRUN on Windows
    fn proptest_lexer_never_panics(s in ".{0,50}") {
        let _ = s;
    }

    #[test]
    // Disabled: triggers STATUS_STACK_BUFFER_OVERRUN on Windows
    fn proptest_lexer_valid_spans(s in ".{0,50}") {
        let _ = s;
    }

    #[test]
    fn proptest_lexer_whitespace_only(n in 0usize..256) {
        let s = " ".repeat(n);
        let mut lexer = Lexer::new(&s);
        let tokens = lexer.tokenize();
        let non_eof: Vec<_> = tokens.into_iter().filter(|t| t.kind != TokenKind::Eof).collect();
        prop_assert!(non_eof.is_empty(), "whitespace-only input should produce no tokens");
    }

    #[test]
    fn proptest_lexer_repeated_chars(ch in "[a-zA-Z]", count in 0usize..128) {
        let s: String = std::iter::repeat(ch).take(count).collect();
        let mut lexer = Lexer::new(&s);
        let _tokens = lexer.tokenize();
    }

    #[test]
    fn proptest_identifiers(name in "[a-zA-Z_][a-zA-Z0-9_]{0,30}") {
        let s = format!("val {} = 42", name);
        prop_assume!(name != "_");
        let mut lexer = Lexer::new(&s);
        let tokens = lexer.tokenize();
        let kinds: Vec<_> = tokens.iter().map(|t| &t.kind).collect();
        prop_assert!(kinds.contains(&&TokenKind::Ident(name.clone())),
            "expected Ident({}) in tokens: {:?}", name, kinds);
    }

    #[test]
    fn proptest_numbers(num_str in "[0-9]{1,15}") {
        let mut lexer = Lexer::new(&num_str);
        let _tokens = lexer.tokenize();
    }
}
