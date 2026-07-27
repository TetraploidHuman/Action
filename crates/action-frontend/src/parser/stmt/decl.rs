use super::super::*;

impl Parser {
    pub(crate) fn parse_type_alias(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'type'

        let name = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected type name")),
        };
        self.advance();

        // Optional type parameters: type Foo[A, B] …
        let type_params = if self.skip(TokenKind::LBracket) {
            let mut params = Vec::new();
            while self.current_kind() != TokenKind::RBracket {
                if !params.is_empty() {
                    self.expect(TokenKind::Comma)?;
                }
                match &self.current_kind() {
                    TokenKind::Ident(s) => params.push(s.clone()),
                    _ => return Err(self.error("Expected type parameter name")),
                }
                self.advance();
            }
            self.expect(TokenKind::RBracket)?;
            params
        } else {
            vec![]
        };

        // Phase 2/3: `type Point { fields…; fun … }`
        // Pure alias: `type UserId = Int` (record form `type Name = { … }` abolished)
        let (definition, methods) = if self.current_kind() == TokenKind::LBrace {
            self.parse_type_body(&name, &type_params)?
        } else {
            self.expect(TokenKind::Eq)?;
            if self.current_kind() == TokenKind::LBrace {
                return Err(self.error(
                    "Use `type Name { fields }` for record types; `type Name = { … }` is no longer valid",
                ));
            }
            (self.parse_type()?, vec![])
        };

        Ok(Stmt::TypeAlias {
            name,
            type_params,
            definition,
            methods,
            span: start_span,
        })
    }

    /// Parse `{ field: Ty, …; fun … }` for a named record type.
    fn parse_type_body(
        &mut self,
        type_name: &str,
        type_params: &[String],
    ) -> Result<(Type, Vec<Stmt>), ParseError> {
        self.expect(TokenKind::LBrace)?;
        let mut fields: Vec<(String, Type)> = Vec::new();
        let mut methods: Vec<Stmt> = Vec::new();

        while self.current_kind() != TokenKind::RBrace {
            if self.current_kind() == TokenKind::Fun {
                let mut method = self.parse_fun_def(false)?;
                self.annotate_type_method_self(&mut method, type_name, type_params);
                methods.push(method);
                self.skip(TokenKind::Semicolon);
                continue;
            }

            if !fields.is_empty() {
                // Separators are optional when the next item is clearly a field/`fun`/`}`.
                let _ = self.skip(TokenKind::Comma) || self.skip(TokenKind::Semicolon);
                if self.current_kind() == TokenKind::RBrace
                    || self.current_kind() == TokenKind::Fun
                {
                    continue;
                }
                if matches!(self.current_kind(), TokenKind::Ident(_))
                    && self.peek2() == TokenKind::Colon
                {
                    // next field — newline-separated OK
                } else if matches!(self.current_kind(), TokenKind::Ident(_))
                    || self.current_kind() == TokenKind::Fun
                {
                    // fall through to field/fun parse
                } else {
                    return Err(self.error("Expected field, 'fun', or '}' in type body"));
                }
            }

            let field_name = match &self.current_kind() {
                TokenKind::Ident(s) => s.clone(),
                _ => {
                    return Err(self.error(
                        "Expected field name or 'fun' in type body",
                    ))
                }
            };
            self.advance();
            self.expect(TokenKind::Colon)?;
            let ty = self.parse_type()?;
            fields.push((field_name, ty));
        }
        self.expect(TokenKind::RBrace)?;
        Ok((Type::Struct(fields), methods))
    }

    /// Inject `self: TypeName` when omitted; keep explicit annotations.
    fn annotate_type_method_self(
        &self,
        method: &mut Stmt,
        type_name: &str,
        type_params: &[String],
    ) {
        let Stmt::Fun { params, .. } = method else {
            return;
        };
        if let Some(first) = params.first_mut() {
            if first.name == "self" && first.ty.is_none() {
                first.ty = Some(if type_params.is_empty() {
                    Type::Named(type_name.to_string())
                } else {
                    Type::Generic(
                        Box::new(Type::Named(type_name.to_string())),
                        type_params
                            .iter()
                            .map(|p| Type::TypeVar(p.clone()))
                            .collect(),
                    )
                });
            }
        }
    }

    pub(crate) fn parse_enum_def(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'enum'

        let name = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected enum name")),
        };
        self.advance();

        let type_params = if self.skip(TokenKind::LBracket) {
            let mut params = Vec::new();
            while self.current_kind() != TokenKind::RBracket {
                if !params.is_empty() {
                    self.expect(TokenKind::Comma)?;
                }
                match &self.current_kind() {
                    TokenKind::Ident(s) => params.push(s.clone()),
                    _ => return Err(self.error("Expected type parameter name")),
                }
                self.advance();
            }
            self.expect(TokenKind::RBracket)?;
            params
        } else {
            vec![]
        };

        self.expect(TokenKind::LBrace)?;
        let mut variants = Vec::new();

        while self.current_kind() != TokenKind::RBrace {
            if !variants.is_empty() {
                self.skip(TokenKind::Comma);
            }

            let variant_name = match &self.current_kind() {
                TokenKind::Ident(s) => s.clone(),
                _ => return Err(self.error("Expected variant name")),
            };
            self.advance();

            let params = if self.skip(TokenKind::LParen) {
                let mut variant_params = Vec::new();
                while self.current_kind() != TokenKind::RParen {
                    if !variant_params.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    // Check for named param: name: Type
                    if let TokenKind::Ident(ref pname) = self.current_kind() {
                        let pname = pname.clone();
                        if self.peek2() == TokenKind::Colon {
                            self.advance(); // param name
                            self.advance(); // ':'
                            let ty = self.parse_type()?;
                            variant_params.push(EnumVariantParam::Named { name: pname, ty });
                            continue;
                        }
                    }
                    let ty = self.parse_type()?;
                    variant_params.push(EnumVariantParam::Positional(ty));
                }
                self.expect(TokenKind::RParen)?;
                variant_params
            } else {
                vec![]
            };

            variants.push(EnumVariant {
                name: variant_name,
                params,
            });
        }
        self.expect(TokenKind::RBrace)?;

        Ok(Stmt::Enum {
            name,
            type_params,
            variants,
            span: start_span,
        })
    }
}
