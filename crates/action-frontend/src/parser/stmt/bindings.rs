use super::super::*;

impl Parser {
    pub(crate) fn parse_let(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();

        // Check for lazy keyword
        let lazy_init = if self.current_kind() == TokenKind::Lazy {
            self.advance();
            true
        } else {
            false
        };

        let mutable = match &self.current_kind() {
            TokenKind::Var => true,
            TokenKind::Val => false,
            _ => return Err(self.error("Expected 'val' or 'var'")),
        };
        self.advance();

        // Check for destructuring pattern: val (x, y) = ... or val [a, b] = ...
        if self.current_kind() == TokenKind::LParen {
            self.advance(); // skip '('
            let mut names = Vec::new();
            loop {
                match &self.current_kind() {
                    TokenKind::Ident(s) => {
                        names.push(s.clone());
                        self.advance();
                    }
                    _ => return Err(self.error("Expected identifier in destructuring pattern")),
                }
                if self.skip(TokenKind::Comma) {
                    if self.current_kind() == TokenKind::RParen {
                        break;
                    }
                    continue;
                }
                break;
            }
            self.expect(TokenKind::RParen)?;

            // Assignment
            self.expect(TokenKind::Eq)?;
            let value = self.parse_expr()?;

            return Ok(Stmt::Destructure {
                mutable,
                names,
                renames: vec![],
                rest: None,
                is_list: false,
                is_struct: false,
                value,
                span: start_span,
            });
        }

        // List destructuring: val [a, b, c] = list or val [head, ...tail] = list
        if self.current_kind() == TokenKind::LBracket {
            self.advance(); // skip '['
            let mut names = Vec::new();
            let mut rest = None;
            loop {
                match &self.current_kind() {
                    TokenKind::DotDotDot => {
                        self.advance(); // skip '...'
                                        // Optional variable name after ...
                        if let TokenKind::Ident(s) = &self.current_kind() {
                            rest = Some(s.clone());
                            self.advance();
                        }
                        break;
                    }
                    TokenKind::Ident(s) => {
                        names.push(s.clone());
                        self.advance();
                    }
                    TokenKind::Comma => {
                        self.advance();
                        // After comma, check for ... or another ident
                        if self.current_kind() == TokenKind::DotDotDot {
                            self.advance();
                            if let TokenKind::Ident(s) = &self.current_kind() {
                                rest = Some(s.clone());
                                self.advance();
                            }
                            break;
                        }
                        if self.current_kind() == TokenKind::RBracket {
                            break; // trailing comma
                        }
                        continue;
                    }
                    _ => {
                        return Err(self.error("Expected identifier or '...' in list destructuring"))
                    }
                }
            }
            self.expect(TokenKind::RBracket)?;

            // Assignment
            self.expect(TokenKind::Eq)?;
            let value = self.parse_expr()?;

            return Ok(Stmt::Destructure {
                mutable,
                names,
                renames: vec![],
                rest,
                is_list: true,
                is_struct: false,
                value,
                span: start_span,
            });
        }

        // Struct destructuring: val {x, y} = expr or val {x as px, y as py} = expr
        if self.current_kind() == TokenKind::LBrace {
            self.advance(); // skip '{'
            let mut names = Vec::new();
            let mut renames = Vec::new();
            loop {
                match &self.current_kind() {
                    TokenKind::Ident(s) => {
                        let field = s.clone();
                        self.advance();
                        // Check for rename: {x as px}
                        if self.current_kind() == TokenKind::As {
                            self.advance();
                            let local = match &self.current_kind() {
                                TokenKind::Ident(s) => s.clone(),
                                _ => return Err(self.error("Expected variable name after 'as'")),
                            };
                            self.advance();
                            names.push(field.clone());
                            renames.push((field, local));
                        } else {
                            names.push(field.clone());
                            renames.push((field.clone(), field));
                        }
                    }
                    TokenKind::Comma => {
                        self.advance();
                        if self.current_kind() == TokenKind::RBrace {
                            break; // trailing comma
                        }
                        continue;
                    }
                    _ => break,
                }
            }
            self.expect(TokenKind::RBrace)?;

            // Assignment
            self.expect(TokenKind::Eq)?;
            let value = self.parse_expr()?;

            return Ok(Stmt::Destructure {
                mutable,
                names,
                renames,
                rest: None,
                is_list: false,
                is_struct: true,
                value,
                span: start_span,
            });
        }

        // Variable name
        let name = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected variable name")),
        };
        self.advance();

        // Type annotation requires colon: val x: Int = 0
        let type_ann = if self.skip(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Assignment
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;

        Ok(Stmt::Let {
            mutable,
            lazy_init,
            name,
            type_ann,
            value,
            span: start_span,
        })
    }

    pub(crate) fn parse_const(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'const'

        let name = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected constant name")),
        };
        self.advance();

        // Type annotation requires colon: val x: Int = 0
        let type_ann = if self.skip(TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        // Assignment
        self.expect(TokenKind::Eq)?;
        let value = self.parse_expr()?;

        Ok(Stmt::Const {
            name,
            type_ann,
            value,
            span: start_span,
        })
    }
}
