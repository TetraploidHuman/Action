use super::super::*;

impl Parser {
    pub(crate) fn parse_fun_def(&mut self, is_test: bool) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'fun'

        // Parse optional generic type parameters: fun <T, U> name(...)
        let mut type_params = Vec::new();
        if self.skip(TokenKind::Lt) {
            loop {
                let tp_name = match &self.current_kind() {
                    TokenKind::Ident(s) => s.clone(),
                    _ => return Err(self.error("Expected type parameter name")),
                };
                self.advance();
                type_params.push(tp_name);
                if !self.skip(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt)?;
        }

        let name = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected function name")),
        };
        self.advance();

        // Set current type params so parse_type emits TypeVar for T, U, etc.
        let saved_type_params = std::mem::take(&mut self.current_type_params);
        self.current_type_params = type_params.clone();

        // Parameters
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        while self.current_kind() != TokenKind::RParen {
            if !params.is_empty() {
                self.expect(TokenKind::Comma)?;
            }
            let param_name = match &self.current_kind() {
                TokenKind::Ident(s) => s.clone(),
                _ => return Err(self.error("Expected parameter name")),
            };
            self.advance();

            let ty = if self.skip(TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };
            params.push(Param {
                name: param_name,
                ty,
            });
        }
        self.expect(TokenKind::RParen)?;

        // Optional return type (-> syntax)
        let return_type = if self.skip(TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Restore previous type params
        self.current_type_params = saved_type_params;

        // Body: `fun f() { ... }` or `fun f() = expr`
        let (body, is_single_expr) = if self.skip(TokenKind::Eq) {
            if self.current_kind() == TokenKind::LBrace {
                (self.parse_block_expr()?, false)
            } else {
                (self.parse_expr()?, true)
            }
        } else {
            (self.parse_block_expr()?, false)
        };

        // Function-level or-block: `fun f() { body } or { fallback }`
        let fn_or_fallback = if !is_single_expr && self.skip(TokenKind::Or) {
            Some(self.parse_fn_or_fallback()?)
        } else {
            None
        };

        Ok(Stmt::Fun {
            name,
            params,
            return_type,
            body,
            type_params,
            is_single_expr,
            is_test,
            fn_or_fallback,
            span: start_span,
        })
    }

    /// Parse `or { fallback }` after a function block body.
    fn parse_fn_or_fallback(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBrace)?;
        if self.current_kind() == TokenKind::LBrace {
            let inner = self.parse_block_expr()?;
            self.expect(TokenKind::RBrace)?;
            Ok(inner)
        } else {
            let e = self.parse_expr()?;
            self.expect(TokenKind::RBrace)?;
            Ok(e)
        }
    }

    pub(crate) fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'return'

        // Check if there's an expression following
        if matches!(
            self.current_kind(),
            TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof
        ) {
            Ok(Stmt::Return {
                value: None,
                span: start_span,
            })
        } else {
            let expr = self.parse_expr()?;
            Ok(Stmt::Return {
                value: Some(expr),
                span: start_span,
            })
        }
    }

    // ---- Type Parsing ----
}
