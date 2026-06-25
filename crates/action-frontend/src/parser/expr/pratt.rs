use super::super::*;

impl Parser {
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_pratt(Precedence::Lowest)
    }

    /// Peek ahead to check if a `(` starts a when-arm pattern rather than a function call.
    /// Returns true if the parenthesized content parses as comma-separated patterns
    /// followed by `)` then `->`.
    pub(crate) fn peek_when_arm_pattern(&mut self) -> bool {
        let saved = self.pos;
        self.advance(); // (
        let mut ok = self.parse_pattern().is_ok();
        while ok && self.current_kind() == TokenKind::Comma {
            self.advance();
            ok = self.parse_pattern().is_ok();
        }
        ok = ok && self.current_kind() == TokenKind::RParen;
        if ok {
            self.advance(); // )
            ok = self.current_kind() == TokenKind::Arrow;
        }
        self.pos = saved;
        ok
    }

    pub(crate) fn parse_pratt(&mut self, min_prec: Precedence) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            // Postfix operators first — they bind tighter than any binary operator.
            // After each postfix, continue the outer loop so binary operators after
            // the postfix (e.g. r.x + 1) are correctly parsed.
            let postfix_applied = match self.current_kind() {
                TokenKind::LParen => {
                    if self.no_postfix_call && self.peek_when_arm_pattern() {
                        false
                    } else {
                        left = self.parse_call_suffix(left)?;
                        true
                    }
                }
                TokenKind::Dot => {
                    self.advance();
                    let field = match &self.current_kind() {
                        TokenKind::Ident(s) => {
                            let name = s.clone();
                            self.advance();
                            name
                        }
                        TokenKind::IntLiteral(n) => {
                            let name = n.to_string();
                            self.advance();
                            name
                        }
                        TokenKind::When | TokenKind::For => {
                            let kw = self.current_kind().to_string();
                            self.advance();
                            kw
                        }
                        _ => return Err(self.error("Expected field name after '.'")),
                    };
                    left = ExprKind::FieldAccess(Box::new(left), field).into();
                    true
                }
                TokenKind::ColonColon => {
                    self.advance();
                    let method = match &self.current_kind() {
                        TokenKind::Ident(s) => {
                            let name = s.clone();
                            self.advance();
                            name
                        }
                        _ => return Err(self.error("Expected method name after '::'")),
                    };
                    let type_name = match &left.kind {
                        ExprKind::Ident(name) => name.clone(),
                        _ => {
                            return Err(
                                self.error("Expected type name before '::' (e.g., Int::toString)")
                            )
                        }
                    };
                    left = ExprKind::FunctionRef(format!("{}.{}", type_name, method)).into();
                    true
                }
                TokenKind::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    left = ExprKind::Index(Box::new(left), Box::new(idx)).into();
                    true
                }
                TokenKind::Question => {
                    self.advance();
                    // ? is only valid after a type or as part of ?. (safe call sugar)
                    // Standalone ? after expression is not valid.
                    // Check for ?. safe call sugar
                    if self.current_kind() == TokenKind::Dot {
                        self.advance(); // skip '.'
                        let field = match &self.current_kind() {
                            TokenKind::Ident(s) => {
                                let name = s.clone();
                                self.advance();
                                name
                            }
                            _ => return Err(self.error("Expected field name after '?.'")),
                        };
                        left = ExprKind::FieldAccess(Box::new(left), field).into();
                    } else if self.current_kind() == TokenKind::LBracket {
                        self.advance(); // skip '['
                        let idx = self.parse_expr()?;
                        self.expect(TokenKind::RBracket)?;
                        left = ExprKind::Index(Box::new(left), Box::new(idx)).into();
                    } else if self.current_kind() == TokenKind::LParen {
                        left = self.parse_call_suffix(left)?;
                    } else {
                        return Err(self.error("Unexpected '?'. Use 'or { }' for nullable fallback, or '?.' for safe call"));
                    }
                    true
                }
                TokenKind::Or => {
                    // Check for or-block (nullable fallback): expr or { ... }
                    // Only when followed by { — otherwise it's logical OR
                    if self.peek2() == TokenKind::LBrace {
                        self.advance(); // skip 'or'
                        let fallback = self.parse_block_expr()?;
                        left = ExprKind::OrBlock {
                            nullable: Box::new(left),
                            fallback: Box::new(fallback),
                        }
                        .into();
                        true
                    } else {
                        false
                    }
                }
                TokenKind::LBrace => {
                    let is_callable = matches!(&left.kind, ExprKind::Ident(name)
                        if name == "launch" || name == "coroutineScope")
                        || matches!(&left.kind, ExprKind::FieldAccess(_, _));
                    if is_callable {
                        let lambda = self.parse_lambda_or_struct()?;
                        if matches!(&lambda.kind, ExprKind::Lambda { .. }) {
                            left = ExprKind::Call {
                                func: Box::new(left),
                                args: vec![],
                                trailing_lambda: Some(Box::new(lambda)),
                            }
                            .into();
                            true
                        } else {
                            return Err(self.error("Expected lambda after call"));
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if postfix_applied {
                continue;
            }

            // Binary / compound / special operators
            let tok_kind = self.current_kind();
            if is_compound_assign(&tok_kind) {
                let base_op = compound_to_binary(&tok_kind).unwrap();
                self.advance();
                let right = self.parse_pratt(Precedence::Assignment.next())?;
                let lhs_clone = left.clone();
                left = ExprKind::Assign {
                    target: Box::new(left),
                    value: Box::new(
                        ExprKind::Binary(Box::new(lhs_clone), base_op, Box::new(right)).into(),
                    ),
                }
                .into();
                continue;
            }

            if let Some(op) = token_to_binary_op(&tok_kind) {
                let prec = Precedence::of_binary(&op);
                if prec < min_prec {
                    break;
                }
                self.advance();
                let mut right = self.parse_pratt(prec.next())?;
                loop {
                    let next_kind = self.current_kind();
                    if let Some(op2) = token_to_binary_op(&next_kind) {
                        let prec2 = Precedence::of_binary(&op2);
                        if prec2 == prec && is_left_associative(&op2) {
                            self.advance();
                            let r2 = self.parse_pratt(prec.next())?;
                            right = ExprKind::Binary(Box::new(right), op2, Box::new(r2)).into();
                            continue;
                        }
                    }
                    break;
                }
                if op == BinaryOp::Assign {
                    left = ExprKind::Assign {
                        target: Box::new(left),
                        value: Box::new(right),
                    }
                    .into();
                } else {
                    left = ExprKind::Binary(Box::new(left), op, Box::new(right)).into();
                }
                continue;
            }

            if tok_kind == TokenKind::In || tok_kind == TokenKind::Is {
                let prec = Precedence::Comparison;
                if prec < min_prec {
                    break;
                }
                let op = if tok_kind == TokenKind::In {
                    BinaryOp::In
                } else {
                    BinaryOp::Is
                };
                self.advance();
                let right = self.parse_pratt(prec.next())?;
                left = ExprKind::Binary(Box::new(left), op, Box::new(right)).into();
                continue;
            }

            if let TokenKind::Ident(ref s) = tok_kind {
                if s == "to" {
                    let prec = Precedence::To;
                    if prec < min_prec {
                        break;
                    }
                    self.advance();
                    let right = self.parse_pratt(prec.next())?;
                    let mut elements = if let ExprKind::Tuple(elems) = &left.kind {
                        elems.clone()
                    } else {
                        vec![(None, left)]
                    };
                    match &right.kind {
                        ExprKind::Tuple(elems) => elements.extend(elems.clone()),
                        _ => elements.push((None, right)),
                    }
                    left = ExprKind::Tuple(elements).into();
                    continue;
                }
            }

            break;
        }

        Ok(left)
    }
}
