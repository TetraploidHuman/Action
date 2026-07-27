use super::super::*;

impl Parser {
    pub(crate) fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind() {
            TokenKind::IntLiteral(n) => {
                self.advance();
                Ok(Expr::int(n))
            }
            TokenKind::FloatLiteral(n) => {
                self.advance();
                Ok(Expr::float(n))
            }
            TokenKind::BoolLiteral(b) => {
                self.advance();
                Ok(Expr::bool(b))
            }
            TokenKind::Null => {
                return Err(self.error_coded(
                    "null is not supported; use fallible operations with or { }",
                    crate::error::DiagnosticCode::E010,
                ));
            }
            TokenKind::CharLiteral(c) => {
                self.advance();
                Ok(ExprKind::Literal(Literal::Char(c)).into())
            }
            TokenKind::StringLiteral(ref s) => {
                let s = s.clone();
                let str_span = self.current_span();
                self.advance();
                // Check for string interpolation: if string contains $ or ${
                if s.contains('$') {
                    self.parse_interpolated_string(&s, str_span)
                } else {
                    Ok(Expr::string(&s))
                }
            }
            TokenKind::Ident(ref name) => {
                let name = name.clone();
                self.advance();

                // Collection literals: List[...], Set[...], Map[...]
                if (name == "List" || name == "Set" || name == "Map")
                    && self.current_kind() == TokenKind::LBracket
                {
                    return self.parse_collection_literal(&name);
                }
                // Check for function call (identifier followed by paren)
                if self.current_kind() == TokenKind::LParen {
                    self.parse_call_suffix(ExprKind::Ident(name.clone()).into())
                } else {
                    Ok(ExprKind::Ident(name).into())
                }
            }
            TokenKind::ColonColon => {
                self.advance(); // skip ::
                                // Parse function reference path
                let mut path = String::new();
                match &self.current_kind() {
                    TokenKind::Ident(s) => {
                        path.push_str(s);
                        self.advance();
                    }
                    _ => return Err(self.error("Expected function name after ::").into()),
                }
                // Parse rest of path: ::method_name or .field_name
                while self.current_kind() == TokenKind::ColonColon
                    || self.current_kind() == TokenKind::Dot
                {
                    if self.current_kind() == TokenKind::ColonColon {
                        path.push_str("::");
                    } else {
                        path.push('.');
                    }
                    self.advance();
                    match &self.current_kind() {
                        TokenKind::Ident(s) => {
                            path.push_str(s);
                            self.advance();
                        }
                        _ => {
                            return Err(self
                                .error("Expected identifier in function reference path")
                                .into())
                        }
                    }
                }
                Ok(ExprKind::FunctionRef(path).into())
            }
            TokenKind::Plus => {
                self.advance();
                let expr = self.parse_pratt(Precedence::Unary)?;
                // M108: keep Unary Pos in AST so Bool/String operands can be rejected.
                Ok(Expr::unary(UnaryOp::Pos, expr))
            }
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_pratt(Precedence::Unary)?;
                Ok(Expr::unary(UnaryOp::Neg, expr))
            }
            TokenKind::Not => {
                self.advance();
                let expr = self.parse_pratt(Precedence::Unary)?;
                Ok(Expr::unary(UnaryOp::Not, expr))
            }
            TokenKind::Tilde => {
                self.advance();
                let expr = self.parse_pratt(Precedence::Unary)?;
                Ok(Expr::unary(UnaryOp::BitNot, expr))
            }
            TokenKind::Continue => {
                self.advance();
                Ok(ExprKind::Continue.into())
            }
            TokenKind::Break => {
                self.advance();
                Ok(ExprKind::Break.into())
            }
            TokenKind::When => self.parse_when(),
            TokenKind::If => self.parse_if(),
            TokenKind::For => self.parse_for(),
            TokenKind::Copy => {
                self.advance();
                let expr = self.parse_prefix()?;
                Ok(ExprKind::Copy(Box::new(expr)).into())
            }
            TokenKind::Unsafe => {
                self.advance();
                self.expect(TokenKind::LBrace)?;
                self.block_parse_stack.push(BlockFrame::new(
                    BlockFrameKind::PlainBlock,
                    !self.block_parse_stack.is_empty(),
                ));
                let body = self.run_block_parse_loop()?;
                Ok(ExprKind::Unsafe(Box::new(body)).into())
            }
            TokenKind::Lambda => self.parse_lambda_keyword(),
            TokenKind::LBrace => self.parse_immediate_block(),
            TokenKind::LParen => self.parse_paren_or_tuple(),
            // [ alone is no longer a list literal — use List[...] instead
            TokenKind::LBracket => Err(self.error(
                "Unexpected '[' — use List[...] for list literals, or variable[index] for indexing",
            )),
            TokenKind::Underscore => {
                self.advance();
                // Wildcard pattern — typically used in patterns, return as Ident for now
                Ok(ExprKind::Ident("_".to_string()).into())
            }
            _ => Err(self.error(&format!("Unexpected token: {}", self.current_kind()))),
        }
    }

    pub(crate) fn parse_interpolated_string(
        &self,
        s: &str,
        str_span: Span,
    ) -> Result<Expr, ParseError> {
        // Handle ${expr} interpolation only (per v6 spec)
        let mut parts = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                if !current.is_empty() {
                    parts.push(StringPart::Literal(current.clone()));
                    current.clear();
                }
                // ${expr}
                let mut expr_str = String::new();
                let mut depth = 1;
                i += 2;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    expr_str.push(chars[i]);
                    i += 1;
                }
                // Parse the embedded expression
                let mut sub_lexer = Lexer::new(&expr_str);
                let sub_tokens = sub_lexer.tokenize();
                // Propagate lexer errors from interpolated expressions
                let sub_errors = sub_lexer.take_errors();
                if !sub_errors.is_empty() {
                    let msgs: Vec<String> = sub_errors.iter().map(|e| e.to_string()).collect();
                    return Err(ParseError {
                        message: msgs.join("\n"),
                        span: str_span,
                        code: None,
                    });
                }
                let mut sub_parser = Parser::new(sub_tokens);
                // Propagate parse errors from the interpolated expression to the user.
                // Previously errors were silently swallowed and the raw ${...} text
                // was emitted as a literal — this made interpolation typos invisible.
                let expr = sub_parser.parse_expr().map_err(|e| ParseError {
                    message: format!("In string interpolation: {}", e.message),
                    span: str_span,
                    code: e.code,
                })?;
                parts.push(StringPart::Expr(Box::new(expr)));
            } else {
                current.push(chars[i]);
            }
            i += 1;
        }
        if !current.is_empty() {
            parts.push(StringPart::Literal(current));
        }
        Ok(ExprKind::StringInterpolate(parts).into())
    }

    pub(crate) fn parse_paren_or_tuple(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // skip '('

        if self.skip(TokenKind::RParen) {
            return Ok(ExprKind::Literal(Literal::Unit).into());
        }

        let first = self.parse_expr()?;

        // Check for named tuple: (name: value, ...)
        // If first expr is an Ident followed by ':', treat as named field

        // Check for named first element: (name: value, ...)
        let mut exprs: Vec<(Option<String>, Expr)> = Vec::new();

        // Check if first expr is named: ident followed by ':'
        let named_first = if let ExprKind::Ident(ref name) = first.kind {
            if self.current_kind() == TokenKind::Colon {
                let field_name = name.clone();
                self.advance(); // skip ':'
                let value = self.parse_expr()?;
                Some((field_name, value))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((name, val)) = named_first {
            exprs.push((Some(name), val));
        } else {
            exprs.push((None, first));
        }

        if self.skip(TokenKind::RParen) {
            // Single expression in parens — but now unwrap from tuple wrapper
            if exprs.len() == 1 && exprs[0].0.is_none() {
                return Ok(exprs.remove(0).1);
            }
            return Ok(ExprKind::Tuple(exprs).into());
        }

        // Tuple
        self.expect(TokenKind::Comma)?;
        while self.current_kind() != TokenKind::RParen {
            // Check for named field: identifier : expression
            if let TokenKind::Ident(_) = &self.current_kind() {
                if self.peek2() == TokenKind::Colon {
                    let name = match &self.current_kind() {
                        TokenKind::Ident(s) => s.clone(),
                        _ => unreachable!(),
                    };
                    self.advance(); // skip name
                    self.advance(); // skip ':'
                    let value = self.parse_expr()?;
                    exprs.push((Some(name), value));
                } else {
                    exprs.push((None, self.parse_expr()?));
                }
            } else {
                exprs.push((None, self.parse_expr()?));
            }
            if self.current_kind() != TokenKind::RParen {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(ExprKind::Tuple(exprs).into())
    }

    /// Parse collection literal after `List`, `Set`, or `Map` keyword: List[...], Set[...], Map[...]
    pub(crate) fn parse_collection_literal(&mut self, kind: &str) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBracket)?; // consume '['

        match kind {
            "List" => {
                let mut items = Vec::new();
                while self.current_kind() != TokenKind::RBracket {
                    if !items.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    if self.current_kind() == TokenKind::RBracket {
                        break; // trailing comma
                    }
                    items.push(self.parse_expr()?);
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::call(
                    ExprKind::Ident("__list".to_string()).into(),
                    items,
                ))
            }
            "Set" => {
                let mut elements = Vec::new();
                while self.current_kind() != TokenKind::RBracket {
                    if !elements.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    if self.current_kind() == TokenKind::RBracket {
                        break; // trailing comma
                    }
                    let elem = self.parse_expr()?;
                    elements.push(elem);
                }
                self.expect(TokenKind::RBracket)?;
                Ok(ExprKind::SetLiteral(elements).into())
            }
            "Map" => {
                let mut entries = Vec::new();
                while self.current_kind() != TokenKind::RBracket {
                    if !entries.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    if self.current_kind() == TokenKind::RBracket {
                        break; // trailing comma
                    }
                    let key = self.parse_expr()?;
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_expr()?;
                    entries.push((key, value));
                }
                self.expect(TokenKind::RBracket)?;
                Ok(ExprKind::MapLiteral(entries).into())
            }
            _ => unreachable!(),
        }
    }
}
