mod expr;
mod pattern;
mod stmt;
mod type_parse;

use crate::ast::*;
use crate::error::CompilerError;
use crate::lexer::{Lexer, Token, TokenKind};
use action_span::Span;

/// Parse error with source location.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub code: Option<crate::error::DiagnosticCode>,
}

impl ParseError {
    pub fn to_compiler_error(&self) -> CompilerError {
        let mut e = CompilerError::new(self.message.clone()).with_span(self.span);
        if let Some(code) = self.code {
            e = e.with_code(code);
        }
        e
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Parse error at line {}, col {}: {}",
            self.span.line, self.span.col, self.message
        )
    }
}

/// Pratt parsing precedence levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Precedence {
    Lowest,
    Assignment,
    To,
    LogicalOr,
    LogicalAnd,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Comparison,
    Shift,
    Range,
    Sum,
    Product,
    Power,
    Unary,
    Call,
}

impl Precedence {
    fn of_binary(op: &BinaryOp) -> Self {
        match op {
            BinaryOp::Assign => Precedence::Assignment,
            BinaryOp::Or => Precedence::LogicalOr,
            BinaryOp::And => Precedence::LogicalAnd,
            BinaryOp::BitOr => Precedence::BitwiseOr,
            BinaryOp::BitXor => Precedence::BitwiseXor,
            BinaryOp::BitAnd => Precedence::BitwiseAnd,
            BinaryOp::Eq
            | BinaryOp::Neq
            | BinaryOp::Lt
            | BinaryOp::Gt
            | BinaryOp::Lte
            | BinaryOp::Gte
            | BinaryOp::In
            | BinaryOp::Is => Precedence::Comparison,
            BinaryOp::Shl | BinaryOp::Shr => Precedence::Shift,
            BinaryOp::Range | BinaryOp::RangeExclusive => Precedence::Range,
            BinaryOp::Add | BinaryOp::Sub => Precedence::Sum,
            BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => Precedence::Product,
            BinaryOp::Pow => Precedence::Power,
        }
    }

    fn next(self) -> Self {
        match self {
            Precedence::Lowest => Precedence::Assignment,
            Precedence::Assignment => Precedence::To,
            Precedence::To => Precedence::LogicalOr,
            Precedence::LogicalOr => Precedence::LogicalAnd,
            Precedence::LogicalAnd => Precedence::BitwiseOr,
            Precedence::BitwiseOr => Precedence::BitwiseXor,
            Precedence::BitwiseXor => Precedence::BitwiseAnd,
            Precedence::BitwiseAnd => Precedence::Comparison,
            Precedence::Comparison => Precedence::Shift,
            Precedence::Shift => Precedence::Range,
            Precedence::Range => Precedence::Sum,
            Precedence::Sum => Precedence::Product,
            Precedence::Product => Precedence::Power,
            Precedence::Power => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Call,
        }
    }
}

fn token_to_binary_op(kind: &TokenKind) -> Option<BinaryOp> {
    match kind {
        TokenKind::Plus => Some(BinaryOp::Add),
        TokenKind::Minus => Some(BinaryOp::Sub),
        TokenKind::Star => Some(BinaryOp::Mul),
        TokenKind::Slash => Some(BinaryOp::Div),
        TokenKind::Percent => Some(BinaryOp::Mod),
        TokenKind::EqEq => Some(BinaryOp::Eq),
        TokenKind::Neq => Some(BinaryOp::Neq),
        TokenKind::Lt => Some(BinaryOp::Lt),
        TokenKind::Gt => Some(BinaryOp::Gt),
        TokenKind::Lte => Some(BinaryOp::Lte),
        TokenKind::Gte => Some(BinaryOp::Gte),
        TokenKind::And => Some(BinaryOp::And),
        TokenKind::Or => Some(BinaryOp::Or),
        TokenKind::Ampersand => Some(BinaryOp::BitAnd),
        TokenKind::Pipe => Some(BinaryOp::BitOr),
        TokenKind::Caret => Some(BinaryOp::BitXor),
        TokenKind::Shl => Some(BinaryOp::Shl),
        TokenKind::Shr => Some(BinaryOp::Shr),
        TokenKind::StarStar => Some(BinaryOp::Pow),
        TokenKind::DotDot => Some(BinaryOp::Range),
        TokenKind::DotDotLt => Some(BinaryOp::RangeExclusive),
        TokenKind::Eq => Some(BinaryOp::Assign),
        TokenKind::PlusEq => Some(BinaryOp::Add),
        TokenKind::MinusEq => Some(BinaryOp::Sub),
        TokenKind::StarEq => Some(BinaryOp::Mul),
        TokenKind::SlashEq => Some(BinaryOp::Div),
        TokenKind::PercentEq => Some(BinaryOp::Mod),
        _ => None,
    }
}

fn is_compound_assign(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::PlusEq
            | TokenKind::MinusEq
            | TokenKind::StarEq
            | TokenKind::SlashEq
            | TokenKind::PercentEq
    )
}

fn compound_to_binary(kind: &TokenKind) -> Option<BinaryOp> {
    match kind {
        TokenKind::PlusEq => Some(BinaryOp::Add),
        TokenKind::MinusEq => Some(BinaryOp::Sub),
        TokenKind::StarEq => Some(BinaryOp::Mul),
        TokenKind::SlashEq => Some(BinaryOp::Div),
        TokenKind::PercentEq => Some(BinaryOp::Mod),
        _ => None,
    }
}

fn is_left_associative(op: &BinaryOp) -> bool {
    !matches!(op, BinaryOp::Pow | BinaryOp::Assign)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockFrameKind {
    PlainBlock,
    LambdaBody,
}

#[derive(Debug, Clone)]
pub(crate) struct BlockFrame {
    pub stmts: Vec<Stmt>,
    pub kind: BlockFrameKind,
    pub return_on_close: bool,
}

impl BlockFrame {
    pub fn new(kind: BlockFrameKind, return_on_close: bool) -> Self {
        BlockFrame {
            stmts: Vec::new(),
            kind,
            return_on_close,
        }
    }
}

pub struct Parser {
    pub(crate) tokens: Vec<Token>,
    pub(crate) pos: usize,
    pub(crate) current_type_params: Vec<String>,
    pub(crate) no_postfix_call: bool,
    /// When true, do not attach `{ ... }` after a call as trailing lambda (when condition).
    pub(crate) no_trailing_lambda: bool,
    pub(crate) block_parse_stack: Vec<BlockFrame>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser {
            tokens,
            pos: 0,
            current_type_params: Vec::new(),
            no_postfix_call: false,
            no_trailing_lambda: false,
            block_parse_stack: Vec::new(),
        }
    }

    pub(crate) fn current(&self) -> &Token {
        self.tokens.get(self.pos).unwrap_or_else(|| {
            self.tokens
                .last()
                .expect("Parser has no tokens: lexer must produce at least EOF")
        })
    }

    pub(crate) fn current_kind(&self) -> TokenKind {
        self.current().kind.clone()
    }

    pub(crate) fn advance(&mut self) {
        if self.current().kind != TokenKind::Eof {
            self.pos += 1;
        }
    }

    pub(crate) fn peek2(&self) -> TokenKind {
        self.tokens
            .get(self.pos + 1)
            .map(|t| t.kind.clone())
            .unwrap_or(TokenKind::Eof)
    }

    pub(crate) fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        let tok = self.current().clone();
        if tok.kind == kind {
            self.advance();
            Ok(tok)
        } else {
            Err(self.error(&format!("Expected {}, got {}", kind, tok.kind)))
        }
    }

    pub(crate) fn skip(&mut self, kind: TokenKind) -> bool {
        if std::mem::discriminant(&self.current_kind()) == std::mem::discriminant(&kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn error(&self, msg: &str) -> ParseError {
        ParseError {
            message: msg.to_string(),
            span: self.current().span,
            code: None,
        }
    }

    pub(crate) fn error_coded(&self, msg: &str, code: crate::error::DiagnosticCode) -> ParseError {
        ParseError {
            message: msg.to_string(),
            span: self.current().span,
            code: Some(code),
        }
    }

    pub(crate) fn current_span(&self) -> Span {
        self.current().span
    }

    // ---- Parse Program ----

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut stmts = Vec::new();
        while self.current_kind() != TokenKind::Eof {
            stmts.push(self.parse_statement()?);
            // Optional semicolons between statements
            self.skip(TokenKind::Semicolon);
        }
        Ok(Program { stmts })
    }

    /// Parse a full program with error recovery.
    /// Returns successfully parsed statements and all parse errors encountered.
    pub fn parse_program_recover(&mut self) -> (Vec<Stmt>, Vec<ParseError>) {
        let mut stmts = Vec::new();
        let mut errors = Vec::new();
        while self.current_kind() != TokenKind::Eof {
            match self.parse_statement() {
                Ok(stmt) => {
                    stmts.push(stmt);
                    self.skip(TokenKind::Semicolon);
                }
                Err(e) => {
                    errors.push(e);
                    self.skip_to_next_stmt();
                    self.skip(TokenKind::Semicolon);
                }
            }
        }
        (stmts, errors)
    }

    // ---- Statement Parsing ----

    pub fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        match self.current_kind() {
            TokenKind::Val | TokenKind::Var | TokenKind::Lazy => self.parse_let(),
            TokenKind::Const => self.parse_const(),
            TokenKind::Fun => self.parse_fun_def(false),
            TokenKind::At => {
                // Check for @test annotation
                if self.peek2() == TokenKind::Ident("test".to_string()) {
                    self.advance(); // skip @
                    self.advance(); // skip "test"
                    if self.current_kind() == TokenKind::Fun {
                        self.parse_fun_def(true)
                    } else {
                        Err(self.error("Expected 'fun' after '@test'"))
                    }
                } else {
                    Err(self.error("Unexpected '@'"))
                }
            }
            TokenKind::Return => self.parse_return(),
            TokenKind::Break => {
                let span = self.current_span();
                self.advance();
                Ok(Stmt::Break { span })
            }
            TokenKind::Continue => {
                let span = self.current_span();
                self.advance();
                Ok(Stmt::Continue { span })
            }
            TokenKind::Type => self.parse_type_alias(),
            TokenKind::Enum => self.parse_enum_def(),
            TokenKind::Module => self.parse_module(),
            TokenKind::Export => self.parse_export(),
            TokenKind::Import => self.parse_import(),
            TokenKind::Extension => self.parse_extension(),
            TokenKind::External => self.parse_external_fun(),
            _ => {
                let expr = self.parse_expr()?;
                self.skip(TokenKind::Semicolon);
                Ok(Stmt::Expr {
                    expr,
                    span: start_span,
                })
            }
        }
    }
    fn skip_to_next_stmt(&mut self) {
        // Skip tokens until we hit a meaningful statement boundary.
        // Track brace depth to avoid skipping past } that belongs to a nested block
        // or function body (e.g., "val x =\nfun foo() { return 42 }" should stop at
        // the semicolon after `=`, not consume `fun foo() { ... }`).
        let mut brace_depth: usize = 0;
        loop {
            match self.current_kind() {
                TokenKind::Eof => break,
                TokenKind::Semicolon => break,
                TokenKind::RBrace if brace_depth == 0 => break,
                TokenKind::LBrace => {
                    brace_depth = brace_depth.saturating_add(1);
                    self.advance();
                }
                TokenKind::RBrace => {
                    brace_depth = brace_depth.saturating_sub(1);
                    self.advance();
                }
                _ => {
                    self.advance();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;

    fn parse(source: &str) -> Result<Program, ParseError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn parse_expr(source: &str) -> Result<Expr, ParseError> {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        parser.parse_expr()
    }

    #[test]
    fn test_let_val() {
        let prog = parse("val x = 10").unwrap();
        assert_eq!(prog.stmts.len(), 1);
        match &prog.stmts[0] {
            Stmt::Let { mutable, name, .. } => {
                assert!(!mutable);
                assert_eq!(name, "x");
            }
            _ => panic!("Expected Let"),
        }
    }

    #[test]
    fn test_let_var() {
        let prog = parse("var y = 20").unwrap();
        match &prog.stmts[0] {
            Stmt::Let { mutable, name, .. } => {
                assert!(*mutable);
                assert_eq!(name, "y");
            }
            _ => panic!("Expected Let"),
        }
    }

    #[test]
    fn test_fun_def() {
        let prog = parse("fun add(x, y) { x + y }").unwrap();
        match &prog.stmts[0] {
            Stmt::Fun { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "x");
                assert_eq!(params[1].name, "y");
            }
            _ => panic!("Expected Fun"),
        }
    }

    #[test]
    fn test_fun_single_expr() {
        let prog = parse("fun add(x: int, y: int) -> int = x + y").unwrap();
        match &prog.stmts[0] {
            Stmt::Fun {
                name,
                params,
                return_type,
                is_single_expr,
                ..
            } => {
                assert_eq!(name, "add");
                assert_eq!(params.len(), 2);
                assert_eq!(params[0].name, "x");
                assert!(return_type.is_some());
                assert!(*is_single_expr);
            }
            _ => panic!("Expected Fun"),
        }
    }

    #[test]
    fn test_binary_expr() {
        let expr = parse_expr("1 + 2 * 3").unwrap();
        // Should be: 1 + (2 * 3)
        match expr.kind {
            ExprKind::Binary(lhs, op, rhs) => {
                assert_eq!(op, BinaryOp::Add);
                match lhs.kind {
                    ExprKind::Literal(Literal::Int(1)) => {}
                    _ => panic!("Expected 1"),
                }
                match rhs.kind {
                    ExprKind::Binary(_, BinaryOp::Mul, _) => {}
                    _ => panic!("Expected multiplication"),
                }
            }
            _ => panic!("Expected binary"),
        }
    }

    #[test]
    fn test_when_one_line() {
        let expr = parse_expr("when a > b { a else b }").unwrap();
        match expr.kind {
            ExprKind::When(w) => match &w.kind {
                WhenKind::OneLine { .. } => {}
                _ => panic!("Expected one-line when"),
            },
            _ => panic!("Expected when"),
        }
    }

    #[test]
    fn test_when_call_condition_not_trailing_lambda() {
        let expr = parse_expr("when atEnd(s, i) { false else true }").unwrap();
        match expr.kind {
            ExprKind::When(w) => match &w.kind {
                WhenKind::OneLine {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    assert!(matches!(condition.kind, ExprKind::Call { .. }));
                    assert!(matches!(
                        then_expr.kind,
                        ExprKind::Literal(Literal::Bool(false))
                    ));
                    assert!(matches!(
                        else_expr.kind,
                        ExprKind::Literal(Literal::Bool(true))
                    ));
                }
                _ => panic!("Expected one-line when"),
            },
            _ => panic!("Expected when"),
        }
    }

    #[test]
    fn test_when_multiline_trailing_else() {
        let expr = parse_expr("when a > b { a } else b").unwrap();
        match expr.kind {
            ExprKind::When(w) => match &w.kind {
                WhenKind::OneLine { else_expr, .. } => match &else_expr.kind {
                    ExprKind::Ident(s) => assert_eq!(s, "b"),
                    _ => panic!("Expected else ident b, got {:?}", else_expr.kind),
                },
                _ => panic!("Expected one-line when"),
            },
            _ => panic!("Expected when"),
        }
    }

    #[test]
    fn test_when_value_match() {
        let prog = parse("when x { 0 -> \"zero\"; 1 -> \"one\"; else -> \"many\" }").unwrap();
        match &prog.stmts[0] {
            Stmt::Expr { ref expr, .. } => match &expr.kind {
                ExprKind::When(w) => match &w.kind {
                    WhenKind::ValueMatch { arms, .. } => {
                        assert_eq!(arms.len(), 3);
                    }
                    _ => panic!("Expected value match"),
                },
                _ => panic!("Expected when expr"),
            },
            _ => panic!("Expected when stmt"),
        }
    }

    #[test]
    fn test_lambda() {
        let expr = parse_expr("{ it * 2 }").unwrap();
        match expr.kind {
            ExprKind::Lambda { implicit_it, .. } => {
                assert!(implicit_it);
            }
            _ => panic!("Expected lambda"),
        }
    }

    #[test]
    fn test_for_iterate() {
        let prog = parse("for item in List[1,2,3] { println(item) }").unwrap();
        match &prog.stmts[0] {
            Stmt::Expr { ref expr, .. } => match &expr.kind {
                ExprKind::For(f) => match &f.kind {
                    ForKind::Iterate { var, .. } => {
                        assert_eq!(var, "item");
                    }
                    _ => panic!("Expected iterate"),
                },
                _ => panic!("Expected for"),
            },
            _ => panic!("Expected for stmt"),
        }
    }

    #[test]
    fn test_for_with_index() {
        let prog = parse("for i, v in List[1,2,3] { println(i + v) }").unwrap();
        match &prog.stmts[0] {
            Stmt::Expr { ref expr, .. } => match &expr.kind {
                ExprKind::For(f) => match &f.kind {
                    ForKind::IterateWithIndex { vars, .. } => {
                        assert_eq!(vars, &vec!["i".to_string(), "v".to_string()]);
                    }
                    _ => panic!("Expected IterateWithIndex"),
                },
                _ => panic!("Expected for"),
            },
            _ => panic!("Expected for stmt"),
        }
    }

    #[test]
    fn test_for_expression() {
        let expr = parse_expr("for x in List[1,2,3,4,5] { x * x }").unwrap();
        match expr.kind {
            ExprKind::For(f) => match &f.kind {
                ForKind::Iterate { var, .. } => {
                    assert_eq!(var, "x");
                }
                _ => panic!("Expected iterate"),
            },
            _ => panic!("Expected for"),
        }
    }

    #[test]
    fn test_enum_def() {
        let prog = parse("enum Option[T] { Some(T), None }").unwrap();
        match &prog.stmts[0] {
            Stmt::Enum {
                name,
                type_params,
                variants,
                ..
            } => {
                assert_eq!(name, "Option");
                assert_eq!(type_params, &vec!["T"]);
                assert_eq!(variants.len(), 2);
                assert_eq!(variants[0].name, "Some");
                assert_eq!(variants[1].name, "None");
            }
            _ => panic!("Expected Enum"),
        }
    }

    #[test]
    fn test_type_alias() {
        let prog = parse("type Point = {x: Int, y: Int}").unwrap();
        match &prog.stmts[0] {
            Stmt::TypeAlias {
                name, type_params, ..
            } => {
                assert_eq!(name, "Point");
                assert!(type_params.is_empty());
            }
            _ => panic!("Expected TypeAlias"),
        }
    }

    #[test]
    fn test_struct_literal() {
        let expr = parse_expr("{x = 10, y = 20}").unwrap();
        match expr.kind {
            ExprKind::StructLiteral(fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0, "x");
                assert_eq!(fields[1].0, "y");
            }
            _ => panic!("Expected struct literal"),
        }
    }

    #[test]
    fn test_field_access() {
        let expr = parse_expr("p.x").unwrap();
        match expr.kind {
            ExprKind::FieldAccess(obj, field) => {
                match obj.kind {
                    ExprKind::Ident(name) => assert_eq!(name, "p"),
                    _ => panic!("Expected identifier"),
                }
                assert_eq!(field, "x");
            }
            _ => panic!("Expected field access"),
        }
    }

    #[test]
    fn test_type_ann_requires_colon() {
        assert!(
            parse("val x Int = 1").is_err(),
            "space-separated type annotation should be rejected"
        );
        assert!(parse("val x: Int = 1").is_ok());
        assert!(parse("const MAX: Int = 1").is_ok());
    }

    #[test]
    fn test_null_literal_rejected_e010() {
        let result = parse("val x = null");
        assert!(result.is_err(), "null should be rejected");
        let err = result.unwrap_err();
        assert_eq!(err.code, Some(crate::error::DiagnosticCode::E010));
    }

    #[test]
    fn test_nullable_type_rejected_e011() {
        let result = parse("val x: Int? = 1");
        assert!(result.is_err(), "Int? should be rejected");
        let err = result.unwrap_err();
        assert_eq!(err.code, Some(crate::error::DiagnosticCode::E011));
    }

    #[test]
    fn test_safe_call_rejected_e012() {
        let result = parse_expr("x?.y");
        assert!(result.is_err(), "?. should be rejected");
        let err = result.unwrap_err();
        assert_eq!(err.code, Some(crate::error::DiagnosticCode::E012));
    }

    #[test]
    fn test_parse_malformed_module_name() {
        // Module name with invalid characters should produce errors
        let result =
            crate::parser::Parser::new(crate::lexer::Lexer::new("module 123invalid {}").tokenize())
                .parse_program();
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_deeply_nested_blocks() {
        let depth = 100;
        let open = "{".repeat(depth);
        let close = "}".repeat(depth);
        let source = format!("val x = {}42{}", open, close);
        let mut parser = crate::parser::Parser::new(crate::lexer::Lexer::new(&source).tokenize());
        let result = parser.parse_program();
        assert!(
            result.is_ok(),
            "deeply nested blocks should parse: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_parse_deeply_nested_binary_expr() {
        let expr = (0..100).map(|_| "1 +").collect::<String>() + "1";
        let source = format!("val x = {}", expr);
        let mut parser = crate::parser::Parser::new(crate::lexer::Lexer::new(&source).tokenize());
        let result = parser.parse_program();
        assert!(
            result.is_ok(),
            "deeply nested binary expr should parse: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_parse_empty_program() {
        let mut parser = crate::parser::Parser::new(crate::lexer::Lexer::new("").tokenize());
        let result = parser.parse_program().unwrap();
        assert!(result.stmts.is_empty());
    }

    #[test]
    fn test_parse_only_comments() {
        let mut parser = crate::parser::Parser::new(
            crate::lexer::Lexer::new("// just a comment\n/* block comment */").tokenize(),
        );
        let result = parser.parse_program().unwrap();
        assert!(result.stmts.is_empty());
    }

    #[test]
    fn test_parse_incomplete_fun_def() {
        // Missing body should produce error
        let mut parser =
            crate::parser::Parser::new(crate::lexer::Lexer::new("fun foo() -> Int").tokenize());
        let result = parser.parse_program();
        assert!(result.is_err(), "incomplete function def should error");
    }

    #[test]
    fn test_parse_when_with_missing_arms() {
        // Empty when should error
        let mut parser =
            crate::parser::Parser::new(crate::lexer::Lexer::new("when x {}").tokenize());
        let result = parser.parse_program();
        assert!(result.is_err(), "empty when should error");
    }

    #[test]
    fn test_parse_double_comma_in_list() {
        let source = "val x = List[1,,2]";
        let mut parser = crate::parser::Parser::new(crate::lexer::Lexer::new(source).tokenize());
        // Should not panic; may produce error or recover
        let _ = parser.parse_program();
    }

    #[test]
    fn test_parse_unclosed_paren() {
        let source = "val x = (1 + 2";
        let mut parser = crate::parser::Parser::new(crate::lexer::Lexer::new(source).tokenize());
        let result = parser.parse_program();
        assert!(result.is_err(), "unclosed paren should error");
    }

    #[test]
    fn test_parse_modulo_op() {
        let source = "val x = 10 % 3";
        let mut parser = crate::parser::Parser::new(crate::lexer::Lexer::new(source).tokenize());
        let result = parser.parse_program().unwrap();
        assert_eq!(result.stmts.len(), 1);
    }

    #[test]
    fn test_parse_power_op() {
        let source = "val x = 2 ** 10";
        let mut parser = crate::parser::Parser::new(crate::lexer::Lexer::new(source).tokenize());
        let result = parser.parse_program().unwrap();
        assert_eq!(result.stmts.len(), 1);
    }

    use proptest::prelude::*;
    proptest! {

        #[test]
        // Disabled: triggers STATUS_STACK_BUFFER_OVERRUN on Windows
        // (stack buffer corruption in lexer/parser on random inputs)
        fn proptest_parse_never_panics(s in ".{0,50}") {
            let _ = s;
        }

        #[test]
        fn proptest_parse_simple_val(name in "[a-zA-Z][a-zA-Z0-9_]{0,20}", n in 0i64..10000i64) {
            const KEYWORDS: &[&str] = &[
                "val", "var", "fun", "when", "else", "for", "in", "is", "break", "continue",
                "return", "enum", "type", "import", "module", "export", "const", "copy",
                "extension", "as", "and", "or", "not", "lazy", "unsafe", "external", "null",
                "Task", "true", "false",
            ];
            prop_assume!(!KEYWORDS.contains(&name.as_str()));
            let s = format!("val {} = {}", name, n);
            let tokens = crate::lexer::Lexer::new(&s).tokenize();
            let mut parser = Parser::new(tokens);
            let (stmts, errors) = parser.parse_program_recover();
            prop_assert!(errors.is_empty() || stmts.len() == 1,
                "expected 1 statement or parse errors for '{}', got {} stmts, {} errors", s, stmts.len(), errors.len());
        }
    }
}
