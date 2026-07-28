use super::*;

impl Parser {
    /// Type starts after a binding/param name (`val a Int`, `fun f(x Int)`).
    /// Includes anonymous struct types `{ x Int }`.
    pub(crate) fn looks_like_type_start(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Ident(_) | TokenKind::LParen | TokenKind::Task | TokenKind::LBrace
        )
    }

    /// Return type after `)` — `{` is always the function body, never a struct return type.
    pub(crate) fn looks_like_return_type_start(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Ident(_) | TokenKind::LParen | TokenKind::Task
        )
    }

    /// Optional type annotation without colon: `name Type` (rejects legacy `name: Type`).
    pub(crate) fn parse_optional_type_ann(&mut self) -> Result<Option<Type>, ParseError> {
        if self.current_kind() == TokenKind::Colon {
            return Err(self.error(
                "Use `name Type` without colon; `name: Type` is no longer valid",
            ));
        }
        if self.looks_like_type_start() {
            Ok(Some(self.parse_type()?))
        } else {
            Ok(None)
        }
    }

    /// Optional return type without arrow: `fun f() Int` (rejects legacy `->`).
    pub(crate) fn parse_optional_return_type(&mut self) -> Result<Option<Type>, ParseError> {
        if self.current_kind() == TokenKind::Arrow {
            return Err(self.error(
                "Use `fun name() RetTy` without `->`; `fun name() -> RetTy` is no longer valid",
            ));
        }
        if self.looks_like_return_type_start() {
            Ok(Some(self.parse_type()?))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn parse_type(&mut self) -> Result<Type, ParseError> {
        let ty = self.parse_type_primary()?;

        // Function type arrow
        // Nullable type: T? (allow chained ? for T?? error)
        if self.skip(TokenKind::Question) {
            return Err(self.error_coded(
                "nullable types (T?) are not supported; use fallible return types with or { }",
                crate::error::DiagnosticCode::E011,
            ));
        }

        if self.skip(TokenKind::Arrow) {
            let params = match ty {
                Type::Unit => vec![],
                _ => vec![ty],
            };
            let ret = self.parse_type()?;
            return Ok(Type::Function(params, Box::new(ret)));
        }

        Ok(ty)
    }

    pub(crate) fn parse_type_primary(&mut self) -> Result<Type, ParseError> {
        match self.current_kind() {
            TokenKind::Ident(ref name) => {
                let original_name = name.clone();
                self.advance();

                // Check for generic instantiation: List[Int]
                if self.skip(TokenKind::LBracket) {
                    let mut args = Vec::new();
                    while self.current_kind() != TokenKind::RBracket {
                        if !args.is_empty() {
                            self.expect(TokenKind::Comma)?;
                        }
                        args.push(self.parse_type()?);
                    }
                    self.expect(TokenKind::RBracket)?;
                    Ok(Type::Generic(
                        Box::new(Type::Named(original_name.clone())),
                        args,
                    ))
                } else if self.current_type_params.contains(&original_name) {
                    Ok(Type::TypeVar(original_name))
                } else {
                    Ok(Type::Named(original_name))
                }
            }
            TokenKind::Task => {
                self.advance();
                if self.skip(TokenKind::LBracket) {
                    let inner = self.parse_type()?;
                    self.expect(TokenKind::RBracket)?;
                    Ok(Type::Task(Box::new(inner)))
                } else {
                    Ok(Type::Named("Task".into()))
                }
            }
            TokenKind::LParen => {
                self.advance();
                // Could be unit type () or tuple/function params
                if self.skip(TokenKind::RParen) {
                    return Ok(Type::Unit);
                }
                let mut params = Vec::new();
                while self.current_kind() != TokenKind::RParen {
                    if !params.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    params.push(self.parse_type()?);
                }
                self.expect(TokenKind::RParen)?;

                if self.skip(TokenKind::Arrow) {
                    let ret = self.parse_type()?;
                    Ok(Type::Function(params, Box::new(ret)))
                } else if params.len() == 1 {
                    // Just a parenthesized type
                    // Safe: we just checked params.len() == 1 above
                    Ok(params.remove(0))
                } else {
                    Err(self.error(
                        "Expected '->' for function type after parenthesized parameter list",
                    ))
                }
            }
            TokenKind::LBrace => {
                // Struct type: {x: Int, y: Int} — fields separated by `,` or `;`
                self.advance();
                let mut fields = Vec::new();
                while self.current_kind() != TokenKind::RBrace {
                    if !fields.is_empty() {
                        if !self.skip(TokenKind::Comma) && !self.skip(TokenKind::Semicolon) {
                            return Err(self.error("Expected ',' or ';' between struct fields"));
                        }
                        // Allow trailing separator before `}`
                        if self.current_kind() == TokenKind::RBrace {
                            break;
                        }
                    }
                    let name = match &self.current_kind() {
                        TokenKind::Ident(s) => s.clone(),
                        _ => return Err(self.error("Expected field name")),
                    };
                    self.advance();
                    if self.current_kind() == TokenKind::Colon {
                        return Err(self.error(
                            "Use `field Type` without colon; `field: Type` is no longer valid",
                        ));
                    }
                    let ty = self.parse_type()?;
                    fields.push((name, ty));
                }
                self.expect(TokenKind::RBrace)?;
                Ok(Type::Struct(fields))
            }
            _ => Err(self.error("Expected type")),
        }
    }

    // ---- Expression Parsing (Pratt) ----
}
