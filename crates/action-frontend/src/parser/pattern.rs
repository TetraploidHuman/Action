use super::*;

impl Parser {
    pub(crate) fn parse_when(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // skip 'when'

        // Check for: when { cond -> body; ... }
        if self.current_kind() == TokenKind::LBrace {
            self.advance(); // skip '{'
            let mut arms = Vec::new();
            while self.current_kind() != TokenKind::RBrace {
                if !arms.is_empty() {
                    self.skip(TokenKind::Semicolon);
                }
                // Handle `else -> body` as wildcard (always matches)
                let pattern = if self.current_kind() == TokenKind::Else {
                    self.advance(); // skip 'else'
                    Pattern::Wildcard
                } else {
                    // Try parsing as expression first — condition chains use expressions
                    // like `x < 0 -> "negative"`. If it's a simple identifier or pattern,
                    // the expression will parse correctly too.
                    let expr = self.parse_expr()?;
                    Pattern::Expr(Box::new(expr))
                };
                let guard = if self.current_kind() == TokenKind::And {
                    self.advance(); // skip 'and'
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                self.expect(TokenKind::Arrow)?;
                self.no_postfix_call = true;
                let body = self.parse_expr()?;
                self.no_postfix_call = false;
                arms.push(WhenArm {
                    pattern,
                    guard,
                    body: Box::new(body),
                });
                self.skip(TokenKind::Semicolon);
            }
            self.expect(TokenKind::RBrace)?;
            return Ok(ExprKind::When(Box::new(When {
                kind: WhenKind::ConditionChain { arms },
            }))
            .into());
        }

        // Parse the condition or value
        self.no_trailing_lambda = true;
        let first = self.parse_expr()?;
        self.no_trailing_lambda = false;

        // when first { ... }
        if self.current_kind() == TokenKind::LBrace {
            self.advance(); // skip '{'

            // Distinguish: binary { A else B } vs value-match { Pat -> expr, ... }
            // Try parsing first item as a pattern. If followed by ->, it's value-match.
            // Also handle optional `and guard` between pattern and ->.
            let saved_pos = self.pos;
            let is_value_match = self.parse_pattern().ok().map_or(false, |_| {
                // Skip comma-separated or-patterns: 0, 1, 2 -> ...
                while self.current_kind() == TokenKind::Comma {
                    self.advance();
                    if self.parse_pattern().is_err() {
                        return false;
                    }
                }
                // Skip optional and guard
                if self.current_kind() == TokenKind::And {
                    self.advance();
                    let _ = self.parse_expr();
                }
                self.current_kind() == TokenKind::Arrow
            });
            self.pos = saved_pos;

            if is_value_match {
                // when value { Pat -> expr, ... }
                let mut arms = Vec::new();
                while self.current_kind() != TokenKind::RBrace {
                    if !arms.is_empty() {
                        self.skip(TokenKind::Semicolon);
                    }
                    let first_pat = self.parse_pattern()?;
                    let pattern = if self.current_kind() == TokenKind::Comma {
                        let mut patterns = vec![first_pat];
                        while self.current_kind() == TokenKind::Comma {
                            self.advance();
                            patterns.push(self.parse_pattern()?);
                        }
                        Pattern::Or(patterns)
                    } else {
                        first_pat
                    };
                    let guard = if self.current_kind() == TokenKind::And {
                        self.advance();
                        Some(Box::new(self.parse_expr()?))
                    } else {
                        None
                    };
                    self.expect(TokenKind::Arrow)?;
                    self.no_postfix_call = true;
                    let body = self.parse_pratt(Precedence::Shift)?;
                    self.no_postfix_call = false;
                    arms.push(WhenArm {
                        pattern,
                        guard,
                        body: Box::new(body),
                    });
                    self.skip(TokenKind::Semicolon);
                }
                self.expect(TokenKind::RBrace)?;
                return Ok(ExprKind::When(Box::new(When {
                    kind: WhenKind::ValueMatch {
                        value: Box::new(first),
                        arms,
                    },
                }))
                .into());
            } else {
                // Binary conditional: when cond { true_expr else false_expr }
                // else clause is optional; if omitted, defaults to ()
                // Support struct-literal arms: when cond { {fields} else {fields} }
                // The outer { of the when-block was already consumed above;
                // if an arm starts with {, it's a struct literal or block expression
                // which needs its own {} pair.
                // The when-block's closing } must still appear after the last arm.
                let true_expr = self.parse_when_arm_expr()?;
                let mut false_expr = if self.current_kind() == TokenKind::Else {
                    self.advance();
                    self.parse_when_arm_expr()?
                } else {
                    ExprKind::Literal(Literal::Unit).into()
                };
                self.expect(TokenKind::RBrace)?;
                // Multiline: when cond { then } else { alt }
                if matches!(false_expr.kind, ExprKind::Literal(Literal::Unit))
                    && self.current_kind() == TokenKind::Else
                {
                    self.advance();
                    false_expr = self.parse_when_arm_expr()?;
                }
                return Ok(ExprKind::When(Box::new(When {
                    kind: WhenKind::OneLine {
                        condition: Box::new(first),
                        then_expr: Box::new(true_expr),
                        else_expr: Box::new(false_expr),
                    },
                }))
                .into());
            }
        }

        Err(self.error("Invalid when expression"))
    }

    /// Parse an expression that appears as a when arm (inside the when-block { }).
    /// After the outer { of the when block is consumed, the arm may itself be a
    /// struct literal or block starting with {, so we route to the right parser.
    pub(crate) fn parse_when_arm_expr(&mut self) -> Result<Expr, ParseError> {
        if self.current_kind() == TokenKind::LBrace {
            self.parse_lambda_or_struct()
        } else {
            self.parse_expr()
        }
    }

    pub(crate) fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        self.parse_single_pattern()
    }

    pub(crate) fn parse_single_pattern(&mut self) -> Result<Pattern, ParseError> {
        match self.current_kind() {
            TokenKind::Else => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            TokenKind::IntLiteral(n) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Int(n)))
            }
            TokenKind::BoolLiteral(b) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Bool(b)))
            }
            TokenKind::StringLiteral(ref s) => {
                let s = s.clone();
                self.advance();
                Ok(Pattern::Literal(Literal::String(s)))
            }
            TokenKind::CharLiteral(c) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Char(c)))
            }
            TokenKind::FloatLiteral(f) => {
                self.advance();
                Ok(Pattern::Literal(Literal::Float(f)))
            }
            TokenKind::Null => {
                return Err(self.error_coded(
                    "null pattern is not supported",
                    crate::error::DiagnosticCode::E010,
                ));
            }
            TokenKind::Underscore => {
                self.advance();
                Ok(Pattern::Wildcard)
            }
            TokenKind::Ident(ref name) => {
                let name = name.clone();
                self.advance();

                // Check if constructor with args: Some(x) or Circle(r: Float)
                if self.current_kind() == TokenKind::LParen {
                    self.advance();
                    let mut args = Vec::new();
                    let mut named_fields = Vec::new();

                    while self.current_kind() != TokenKind::RParen {
                        if !args.is_empty() || !named_fields.is_empty() {
                            self.expect(TokenKind::Comma)?;
                        }

                        // Check for named field: name: pattern
                        if let TokenKind::Ident(ref field_name) = self.current_kind() {
                            let field_name = field_name.clone();
                            if self.peek2() == TokenKind::Colon {
                                self.advance(); // field name
                                self.advance(); // ':'
                                let pat = self.parse_pattern()?;
                                named_fields.push((field_name, pat));
                                continue;
                            }
                        }
                        args.push(self.parse_pattern()?);
                    }
                    self.expect(TokenKind::RParen)?;
                    Ok(Pattern::Constructor {
                        name,
                        args,
                        named_fields,
                    })
                } else if name.chars().next().map_or(false, |c| c.is_uppercase()) {
                    // Uppercase identifier without args -> nullary constructor (e.g. Red, None)
                    Ok(Pattern::Constructor {
                        name,
                        args: vec![],
                        named_fields: vec![],
                    })
                } else {
                    // Lowercase identifier -> variable pattern
                    Ok(Pattern::Variable(name))
                }
            }
            TokenKind::In => {
                self.advance();
                // Range pattern: in start..end
                let start = self.parse_expr()?;
                self.expect(TokenKind::DotDot)?;
                let end = self.parse_expr()?;
                Ok(Pattern::Range(Box::new(start), Box::new(end)))
            }
            TokenKind::Is => {
                self.advance();
                let type_name = match &self.current_kind() {
                    TokenKind::Ident(s) => s.clone(),
                    _ => return Err(self.error("Expected type name after 'is'")),
                };
                self.advance();
                Ok(Pattern::IsType(type_name))
            }
            TokenKind::LParen => {
                self.advance();
                let mut patterns = Vec::new();
                while self.current_kind() != TokenKind::RParen {
                    if !patterns.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    patterns.push(self.parse_pattern()?);
                }
                self.expect(TokenKind::RParen)?;
                match patterns.len() {
                    0 => Err(self.error("Empty tuple pattern is not allowed")),
                    1 => Ok(patterns.into_iter().next().unwrap()),
                    _ => Ok(Pattern::Tuple(patterns)),
                }
            }
            _ => Err(self.error("Expected pattern")),
        }
    }

    pub(crate) fn parse_for(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // skip 'for'

        // Check for: for { body } (infinite loop)
        if self.current_kind() == TokenKind::LBrace {
            let body = self.parse_block_expr()?;
            return Ok(ExprKind::For(Box::new(For {
                kind: ForKind::Infinite {
                    body: Box::new(body),
                },
            }))
            .into());
        }

        // Check for shorthand: for List[...] / Set[...] / Map[...] { body } uses implicit "it"
        if let TokenKind::Ident(ref name) = self.current_kind() {
            if (name == "List" || name == "Set" || name == "Map")
                && self.peek2() == TokenKind::LBracket
            {
                let collection_kind = name.clone();
                self.advance(); // skip List/Set/Map
                let iterable = self.parse_collection_literal(&collection_kind)?;
                let body = self.parse_block_expr()?;
                return Ok(ExprKind::For(Box::new(For {
                    kind: ForKind::Iterate {
                        var: "it".to_string(),
                        iterable: Box::new(iterable),
                        body: Box::new(body),
                        collect: true,
                    },
                }))
                .into());
            }
        }

        // Check for: for index, item in iterable { body }
        if let TokenKind::Ident(ref first_var) = self.current_kind() {
            if self.peek2() == TokenKind::Comma {
                let second_kind = self
                    .tokens
                    .get(self.pos + 2)
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof);
                let after_second = self
                    .tokens
                    .get(self.pos + 3)
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof);
                if matches!(second_kind, TokenKind::Ident(_)) && after_second == TokenKind::In {
                    let mut vars = vec![first_var.clone()];
                    self.advance(); // first var
                    self.expect(TokenKind::Comma)?;
                    let second = match &self.current_kind() {
                        TokenKind::Ident(s) => s.clone(),
                        _ => {
                            return Err(self
                                .error("Expected variable name after ',' in for-with-index loop"));
                        }
                    };
                    vars.push(second);
                    self.advance();
                    self.expect(TokenKind::In)?;
                    let iterable = self.parse_expr()?;
                    let body = self.parse_block_expr()?;
                    return Ok(ExprKind::For(Box::new(For {
                        kind: ForKind::IterateWithIndex {
                            vars,
                            iterable: Box::new(iterable),
                            body: Box::new(body),
                        },
                    }))
                    .into());
                }
            }
        }

        // Check for: for var in iterable ... (var is an identifier followed by 'in')
        if let TokenKind::Ident(ref var_name) = self.current_kind() {
            if self.peek2() == TokenKind::In {
                let var = var_name.clone();
                self.advance(); // skip var name
                self.advance(); // skip 'in'

                let first_iterable = self.parse_expr()?;
                let mut bindings = vec![(var.clone(), first_iterable)];

                // Parse additional bindings: for x in xs, y in ys, ...
                while self.skip(TokenKind::Comma) {
                    let v = match &self.current_kind() {
                        TokenKind::Ident(s) => s.clone(),
                        _ => return Err(self.error("Expected variable name after ',' in for loop")),
                    };
                    self.advance();
                    self.expect(TokenKind::In)?;
                    let iter = self.parse_expr()?;
                    bindings.push((v, iter));
                }

                // Multiple bindings → nested iterate (for expression, collects results)
                if bindings.len() > 1 {
                    let body = self.parse_block_expr()?;
                    return Ok(ExprKind::For(Box::new(For {
                        kind: ForKind::NestedIterate {
                            bindings,
                            body: Box::new(body),
                            collect: true,
                        },
                    }))
                    .into());
                }

                // Single binding
                let (var_single, iterable_single) = bindings.into_iter().next().unwrap();

                // for var in iterable { body } → for expression (collects results)
                let body = self.parse_block_expr()?;
                return Ok(ExprKind::For(Box::new(For {
                    kind: ForKind::Iterate {
                        var: var_single,
                        iterable: Box::new(iterable_single),
                        body: Box::new(body),
                        collect: true,
                    },
                }))
                .into());
            }
        }

        // Parse the first expression (for condition loops)
        let first = self.parse_expr()?;

        // for condition { body }
        if self.current_kind() == TokenKind::LBrace {
            let body = self.parse_block_expr()?;
            return Ok(ExprKind::For(Box::new(For {
                kind: ForKind::Condition {
                    condition: Box::new(first),
                    body: Box::new(body),
                },
            }))
            .into());
        }

        Err(self.error("Invalid for expression"))
    }

    // ---- Module / Import / Export / Type / Enum ----
}
