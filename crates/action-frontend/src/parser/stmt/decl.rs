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

        // Optional type parameters: type Foo[A, B] = ...
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

        self.expect(TokenKind::Eq)?;
        let definition = self.parse_type()?;

        Ok(Stmt::TypeAlias {
            name,
            type_params,
            definition,
            span: start_span,
        })
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
