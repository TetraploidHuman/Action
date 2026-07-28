use super::super::*;

impl Parser {
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_pratt(Precedence::Lowest)
    }

    /// Peek ahead to check if a `(` starts a when-arm pattern rather than a function call.
    /// Returns true if the parenthesized content parses as comma-separated patterns
    /// followed by `)` then `{` (arm body).
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
            ok = self.current_kind() == TokenKind::LBrace;
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
                    return Err(self.error_coded(
                        "?. safe call is not supported; use fallible access with or { }",
                        crate::error::DiagnosticCode::E012,
                    ));
                }
                TokenKind::Or => {
                    // Check for or-block (fallible fallback): expr or { ... }
                    // Only when followed by { — otherwise it's logical OR
                    if self.peek2() == TokenKind::LBrace {
                        self.advance(); // skip 'or'
                        let fallback = self.parse_block_expr()?;
                        left = ExprKind::OrBlock {
                            fallible: Box::new(left),
                            fallback: Box::new(fallback),
                        }
                        .into();
                        true
                    } else {
                        false
                    }
                }
                TokenKind::LBrace => {
                    // Named struct: `Point { x = 1 }` — only PascalCase idents so
                    // `for x { s = … }` / `if y { a = … }` stay statement blocks.
                    if let ExprKind::Ident(ref name) = left.kind {
                        if name
                            .chars()
                            .next()
                            .is_some_and(|c| c.is_uppercase())
                            && self.brace_starts_struct_literal()
                        {
                            let type_name = name.clone();
                            self.advance(); // skip '{'
                            left = self.parse_struct_literal(Some(type_name))?;
                            true
                        } else if name == "launch" || name == "coroutineScope" {
                            let lambda = self.parse_lambda_or_struct()?;
                            let lambda = self.coerce_trailing_lambda(lambda)?;
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
                    } else if matches!(&left.kind, ExprKind::FieldAccess(_, _)) {
                        let lambda = self.parse_lambda_or_struct()?;
                        let lambda = self.coerce_trailing_lambda(lambda)?;
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
                // Classic Pratt left-assoc: parse RHS at prec.next() so same-prec
                // chains fold into `left` on the next outer iteration
                // (`10 - 3 - 2` → `(10 - 3) - 2`). A former same-prec fold that
                // nested into `right` inverted this into right-assoc (M41).
                let rhs_min = if is_left_associative(&op) {
                    prec.next()
                } else {
                    prec
                };
                let right = self.parse_pratt(rhs_min)?;
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
