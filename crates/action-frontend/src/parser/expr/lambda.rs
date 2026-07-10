use super::super::*;

impl Parser {
    /// Scan ahead from current position to see if there's an `->` before `}` (at depth 0).
    /// Used to distinguish lambda params from struct shorthand fields.
    /// Peek past `{` to check if content looks like a lambda/struct, not a block.
    /// Returns false if the brace starts with statement keywords (var, val, for, ...).
    pub(crate) fn brace_is_lambda_like(&self) -> bool {
        // Current token is LBrace; peek at the next token
        if self.pos + 1 >= self.tokens.len() {
            return false;
        }
        match &self.tokens[self.pos + 1].kind {
            TokenKind::Var
            | TokenKind::Val
            | TokenKind::For
            | TokenKind::When
            | TokenKind::Return
            | TokenKind::Const
            | TokenKind::Fun
            | TokenKind::Import
            | TokenKind::Export
            | TokenKind::Type
            | TokenKind::Enum
            | TokenKind::External
            | TokenKind::Module
            | TokenKind::RBrace => false,
            _ => true,
        }
    }

    fn scan_ahead_for_arrow(&self) -> bool {
        let saved = self.tokens.iter().skip(self.pos);
        let mut brace_depth = 0;
        for token in saved {
            match &token.kind {
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => {
                    if brace_depth == 0 {
                        return false; // found } before ->
                    }
                    brace_depth -= 1;
                }
                TokenKind::Arrow => {
                    if brace_depth == 0 {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn parse_lambda_or_struct(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // skip '{'

        // {} → empty block returning unit ()
        if self.skip(TokenKind::RBrace) {
            return Ok(ExprKind::Tuple(vec![]).into()); // unit value
        }

        // {:} is an error — use Map[] instead
        if self.skip(TokenKind::Colon) {
            self.expect(TokenKind::RBrace)?;
            return Err(self.error("Use Map[] for empty map literal, not {:}"));
        }

        // `{ val ...; return x }` in expression position → plain block (supports `{ } or { }`).
        if self.brace_starts_statement_block() {
            let return_on_close = !self.block_parse_stack.is_empty();
            self.block_parse_stack
                .push(BlockFrame::new(BlockFrameKind::PlainBlock, return_on_close));
            return self.run_block_parse_loop();
        }

        // To distinguish struct literal from lambda:
        // - {x = expr, ...} or {x: expr, ...} → struct (Ident + '=' or ':')
        // - {x, y} → struct if no '->' before '}' (shorthand fields)
        // - {x -> body} or {x, y -> body} → lambda (has '->')
        let is_struct = if matches!(self.current_kind(), TokenKind::Ident(_)) {
            match self.peek2() {
                TokenKind::Eq | TokenKind::Colon => true,
                TokenKind::Comma => !self.scan_ahead_for_arrow(),
                TokenKind::Arrow => false, // {x -> body} is lambda
                _ => false,                // {expr} is lambda (block)
            }
        } else {
            false
        };

        if is_struct {
            return self.parse_struct_literal();
        }

        // Everything else in expression position with {} is a lambda
        // Check for explicit params: { x, y -> body } or { x -> body }
        let mut has_explicit_params = false;
        let mut implicit_it = false;

        // Look for identifiers before ->
        if let TokenKind::Ident(ref first_id) = self.current_kind() {
            let first_id = first_id.clone();
            // Peek ahead to see if we have -> after identifiers
            let mut peek_pos = self.pos;
            let mut found_arrow = false;
            loop {
                match self.tokens.get(peek_pos).map(|t| &t.kind) {
                    Some(TokenKind::Arrow) => {
                        found_arrow = true;
                        break;
                    }
                    Some(TokenKind::Comma) => {
                        peek_pos += 1;
                        match self.tokens.get(peek_pos).map(|t| &t.kind) {
                            Some(TokenKind::Ident(_)) => {
                                peek_pos += 1;
                            }
                            _ => break,
                        }
                    }
                    Some(TokenKind::Ident(_)) => {
                        peek_pos += 1;
                    }
                    _ => break,
                }
            }
            has_explicit_params = found_arrow;

            // If first ident is 'it' and not followed by ->, it's an implicit-it lambda
            if !has_explicit_params && first_id == "it" {
                implicit_it = true;
            }
        }

        if has_explicit_params {
            return self.parse_lambda_body(false);
        }

        if implicit_it {
            // { it ... } — body contains `it` reference
            let body = self.parse_expr()?;
            self.expect(TokenKind::RBrace)?;
            return Ok(Expr::it_lambda(body));
        }

        // { stmts } — no-param lambda with block body (handles both single expr and multi-stmt)
        let return_on_close = !self.block_parse_stack.is_empty();
        self.block_parse_stack
            .push(BlockFrame::new(BlockFrameKind::LambdaBody, return_on_close));
        self.run_block_parse_loop()
    }

    pub(crate) fn parse_struct_literal(&mut self) -> Result<Expr, ParseError> {
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

            // Check for shorthand: {x, y} — field name used as variable
            if self.current_kind() == TokenKind::Eq {
                self.advance();
                let value = self.parse_expr()?;
                fields.push((name, value));
            } else {
                // Shorthand: {x} becomes {x: x}
                fields.push((name.clone(), ExprKind::Ident(name).into()));
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(ExprKind::StructLiteral(fields).into())
    }

    /// Check if the token after the current one is a colon (for Map detection)
    pub(crate) fn parse_lambda_body(&mut self, implicit_it: bool) -> Result<Expr, ParseError> {
        if implicit_it {
            // { expr } — single expression with implicit `it`
            let body = self.parse_expr()?;
            self.expect(TokenKind::RBrace)?;
            return Ok(Expr::it_lambda(body));
        }

        let mut params = Vec::new();

        // Parse parameters
        loop {
            match &self.current_kind() {
                TokenKind::Ident(name) => {
                    params.push(name.clone());
                    self.advance();
                    if self.current_kind() == TokenKind::Comma {
                        self.advance();
                        continue;
                    }
                    break;
                }
                _ => {
                    // No explicit params — treat as no-param lambda { expr }
                    let body = self.parse_expr()?;
                    self.expect(TokenKind::RBrace)?;
                    return Ok(ExprKind::Lambda {
                        params: vec![],
                        body: Box::new(body),
                        implicit_it: false,
                    }
                    .into());
                }
            }
        }

        // Expect ->
        self.expect(TokenKind::Arrow)?;

        let body = self.parse_expr()?;
        self.expect(TokenKind::RBrace)?;

        Ok(ExprKind::Lambda {
            params,
            body: Box::new(body),
            implicit_it: false,
        }
        .into())
    }

    /// After `{` was consumed: `{ val/for/return ... }` is a statement block, not a lambda.
    pub(crate) fn brace_starts_statement_block(&self) -> bool {
        matches!(
            self.current_kind(),
            TokenKind::Var
                | TokenKind::Val
                | TokenKind::Lazy
                | TokenKind::For
                | TokenKind::When
                | TokenKind::Return
                | TokenKind::Const
                | TokenKind::Fun
                | TokenKind::Import
                | TokenKind::Export
                | TokenKind::Type
                | TokenKind::Enum
                | TokenKind::External
                | TokenKind::Module
        )
    }

    pub(crate) fn brace_starts_block_body(&self) -> bool {
        if self.current_kind() != TokenKind::LBrace {
            return false;
        }
        let inner_pos = self.pos + 1;
        if inner_pos >= self.tokens.len() {
            return false;
        }
        match &self.tokens[inner_pos].kind {
            TokenKind::RBrace | TokenKind::Colon => false,
            TokenKind::Var
            | TokenKind::Val
            | TokenKind::Lazy
            | TokenKind::For
            | TokenKind::When
            | TokenKind::Return
            | TokenKind::Const
            | TokenKind::Fun
            | TokenKind::Import
            | TokenKind::Export
            | TokenKind::Type
            | TokenKind::Enum
            | TokenKind::External
            | TokenKind::Module => false,
            TokenKind::Ident(name) => match self.tokens.get(inner_pos + 1).map(|t| &t.kind) {
                Some(TokenKind::Eq) | Some(TokenKind::Colon) => false,
                Some(TokenKind::Comma) => !self.scan_ahead_for_arrow_from(inner_pos),
                Some(TokenKind::Arrow) => false,
                _ => name != "it",
            },
            _ => true,
        }
    }

    fn scan_ahead_for_arrow_from(&self, start_pos: usize) -> bool {
        let mut brace_depth = 0;
        for token in self.tokens.iter().skip(start_pos) {
            match &token.kind {
                TokenKind::LBrace => brace_depth += 1,
                TokenKind::RBrace => {
                    if brace_depth == 0 {
                        return false;
                    }
                    brace_depth -= 1;
                }
                TokenKind::Arrow => {
                    if brace_depth == 0 {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub(crate) fn frame_into_expr(frame: BlockFrame) -> Expr {
        let block = ExprKind::Block(frame.stmts).into();
        match frame.kind {
            BlockFrameKind::PlainBlock => block,
            BlockFrameKind::LambdaBody => ExprKind::Lambda {
                params: vec![],
                body: Box::new(block),
                implicit_it: false,
            }
            .into(),
        }
    }
}
