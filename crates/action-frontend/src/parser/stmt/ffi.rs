use super::super::*;

impl Parser {
    pub(crate) fn parse_extension(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'extension'

        let type_name = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected type name after 'extension'")),
        };
        self.advance();

        self.expect(TokenKind::LBrace)?;
        let mut methods = Vec::new();
        while self.current_kind() != TokenKind::RBrace {
            if self.current_kind() == TokenKind::Eof {
                return Err(self.error("Unterminated extension block"));
            }
            let stmt = self.parse_statement()?;
            methods.push(stmt);
            self.skip(TokenKind::Semicolon);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(Stmt::Extension {
            type_name,
            methods,
            span: start_span,
        })
    }

    pub(crate) fn parse_external_fun(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'external'

        // external type Name
        if self.skip(TokenKind::Type) {
            let name = match &self.current_kind() {
                TokenKind::Ident(s) => s.clone(),
                _ => return Err(self.error("Expected type name after 'external type'")),
            };
            self.advance();
            return Ok(Stmt::ExternalType {
                name,
                span: start_span,
            });
        }

        if !self.skip(TokenKind::Fun) {
            return Err(self.error("Expected 'fun' or 'type' after 'external'"));
        }

        let name = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected function name after 'external fun'")),
        };
        self.advance();

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

        Ok(Stmt::External {
            name,
            params,
            return_type,
            span: start_span,
        })
    }
}
