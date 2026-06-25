use super::super::*;

impl Parser {
    pub(crate) fn parse_module(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'module'

        let name = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected module name")),
        };
        self.advance();

        self.expect(TokenKind::LBrace)?;
        let mut exports = Vec::new();
        let mut body = Vec::new();

        while self.current_kind() != TokenKind::RBrace {
            if self.skip(TokenKind::Export) {
                if self.skip(TokenKind::LBrace) {
                    // export { fun f1 ... fun f2 ... } block
                    while self.current_kind() != TokenKind::RBrace {
                        let stmt = self.parse_statement()?;
                        match &stmt {
                            Stmt::Fun { name, .. } => {
                                exports.push(ExportItem::Function(name.clone()));
                            }
                            Stmt::Const { name, .. } => {
                                exports.push(ExportItem::Constant(name.clone()));
                            }
                            _ => {}
                        }
                        body.push(stmt);
                        self.skip(TokenKind::Semicolon);
                    }
                    self.expect(TokenKind::RBrace)?;
                } else {
                    // Parse the exported statement properly
                    let stmt = self.parse_statement()?;
                    match &stmt {
                        Stmt::Fun { name, .. } => {
                            exports.push(ExportItem::Function(name.clone()));
                        }
                        Stmt::Const { name, .. } => {
                            exports.push(ExportItem::Constant(name.clone()));
                        }
                        Stmt::TypeAlias { name, .. } => {
                            exports.push(ExportItem::Type(name.clone()));
                        }
                        _ => {}
                    }
                    body.push(stmt);
                }
            } else {
                body.push(self.parse_statement()?);
            }
            self.skip(TokenKind::Semicolon);
        }
        self.expect(TokenKind::RBrace)?;

        Ok(Stmt::Module {
            name,
            exports,
            body,
            span: start_span,
        })
    }

    pub(crate) fn parse_export(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'export'
        let stmt = self.parse_statement()?;
        Ok(Stmt::Export {
            stmt: Box::new(stmt),
            span: start_span,
        })
    }

    pub(crate) fn parse_import(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'import'

        let module = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected module name")),
        };
        self.advance();

        // import math.{add, PI}
        let items = if self.skip(TokenKind::Dot) {
            self.expect(TokenKind::LBrace)?;
            let mut its = Vec::new();
            while self.current_kind() != TokenKind::RBrace {
                if !its.is_empty() {
                    self.expect(TokenKind::Comma)?;
                }
                match &self.current_kind() {
                    TokenKind::Ident(s) => its.push(s.clone()),
                    _ => return Err(self.error("Expected import item name")),
                }
                self.advance();
            }
            self.expect(TokenKind::RBrace)?;
            Some(its)
        } else {
            None
        };

        // import math as m
        let alias = if self.skip(TokenKind::As) {
            match &self.current_kind() {
                TokenKind::Ident(s) => Some(s.clone()),
                _ => return Err(self.error("Expected alias name")),
            }
        } else {
            None
        };

        Ok(Stmt::Import {
            module,
            items,
            alias,
            span: start_span,
        })
    }
}
