use super::*;

impl Parser {
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
                // Struct type: {x: Int, y: Int}
                self.advance();
                let mut fields = Vec::new();
                while self.current_kind() != TokenKind::RBrace {
                    if !fields.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    let name = match &self.current_kind() {
                        TokenKind::Ident(s) => s.clone(),
                        _ => return Err(self.error("Expected field name")),
                    };
                    self.advance();
                    self.expect(TokenKind::Colon)?;
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
