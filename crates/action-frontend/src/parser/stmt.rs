use super::*;

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

    pub(crate) fn parse_fun_def(&mut self, is_test: bool) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'fun'

        // Parse optional generic type parameters: fun <T, U> name(...)
        let mut type_params = Vec::new();
        if self.skip(TokenKind::Lt) {
            loop {
                let tp_name = match &self.current_kind() {
                    TokenKind::Ident(s) => s.clone(),
                    _ => return Err(self.error("Expected type parameter name")),
                };
                self.advance();
                type_params.push(tp_name);
                if !self.skip(TokenKind::Comma) {
                    break;
                }
            }
            self.expect(TokenKind::Gt)?;
        }

        let name = match &self.current_kind() {
            TokenKind::Ident(s) => s.clone(),
            _ => return Err(self.error("Expected function name")),
        };
        self.advance();

        // Set current type params so parse_type emits TypeVar for T, U, etc.
        let saved_type_params = std::mem::take(&mut self.current_type_params);
        self.current_type_params = type_params.clone();

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

        // Restore previous type params
        self.current_type_params = saved_type_params;

        // Body
        // When = is followed by {, treat it as a block body (same as without =)
        // so that function parameters remain in scope. Without this, { } in
        // expression position becomes a zero-param lambda which opens a new scope.
        let (body, is_single_expr) = if self.skip(TokenKind::Eq) {
            if self.current_kind() == TokenKind::LBrace {
                (self.parse_block_expr()?, false)
            } else {
                (self.parse_expr()?, true)
            }
        } else {
            (self.parse_block_expr()?, false)
        };

        Ok(Stmt::Fun {
            name,
            params,
            return_type,
            body,
            type_params,
            is_single_expr,
            is_test,
            span: start_span,
        })
    }

    pub(crate) fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        let start_span = self.current_span();
        self.advance(); // skip 'return'

        // Check if there's an expression following
        if matches!(
            self.current_kind(),
            TokenKind::Semicolon | TokenKind::RBrace | TokenKind::Eof
        ) {
            Ok(Stmt::Return {
                value: None,
                span: start_span,
            })
        } else {
            let expr = self.parse_expr()?;
            Ok(Stmt::Return {
                value: Some(expr),
                span: start_span,
            })
        }
    }

    // ---- Type Parsing ----

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
