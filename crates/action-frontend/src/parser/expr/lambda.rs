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
            | TokenKind::If
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

    pub(crate) fn coerce_trailing_lambda(&self, expr: Expr) -> Result<Expr, ParseError> {
        match &expr.kind {
            ExprKind::Lambda { .. } => Ok(expr),
            ExprKind::Block(_) => Ok(ExprKind::Lambda {
                params: vec![],
                body: Box::new(expr),
                implicit_it: false,
            }
            .into()),
            _ => Err(self.error("Expected lambda after call")),
        }
    }

    /// Phase 4: `lambda a, b { … }` / `lambda { }` / `lambda { it … }`.
    pub(crate) fn parse_lambda_keyword(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // skip `lambda`

        let mut params = Vec::new();
        while matches!(self.current_kind(), TokenKind::Ident(_))
            && (self.peek2() == TokenKind::Comma || self.peek2() == TokenKind::LBrace)
        {
            let TokenKind::Ident(name) = self.current_kind() else {
                unreachable!();
            };
            params.push(name.clone());
            self.advance();
            if self.current_kind() == TokenKind::Comma {
                self.advance();
                continue;
            }
            break;
        }

        self.expect(TokenKind::LBrace)?;

        // `lambda { }` — empty body
        if self.skip(TokenKind::RBrace) {
            return Ok(ExprKind::Lambda {
                params,
                body: Box::new(ExprKind::Block(vec![]).into()),
                implicit_it: false,
            }
            .into());
        }

        // `lambda { it … }` with no explicit params → implicit-it
        if params.is_empty() {
            if let TokenKind::Ident(ref id) = self.current_kind() {
                if id == "it" && self.peek2() != TokenKind::Arrow {
                    let body = self.parse_expr()?;
                    self.expect(TokenKind::RBrace)?;
                    return Ok(Expr::it_lambda(body));
                }
            }
        }

        // Body: statement block or single expression
        let return_on_close = !self.block_parse_stack.is_empty();
        self.block_parse_stack
            .push(BlockFrame::new(BlockFrameKind::PlainBlock, return_on_close));
        let body = self.run_block_parse_loop()?;
        Ok(ExprKind::Lambda {
            params,
            body: Box::new(body),
            implicit_it: false,
        }
        .into())
    }

    /// Phase 4+: expression-position `{ … }` is an immediately-executed block.
    /// Closures use `lambda … { }`. Struct construction requires `TypeName { … }`.
    pub(crate) fn parse_immediate_block(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // skip '{'

        // `{}` → empty block, Unit
        if self.skip(TokenKind::RBrace) {
            return Ok(ExprKind::Block(vec![]).into());
        }

        if self.skip(TokenKind::Colon) {
            self.expect(TokenKind::RBrace)?;
            return Err(self.error("Use Map[] for empty map literal, not {:}"));
        }

        if self.brace_starts_statement_block() {
            let return_on_close = !self.block_parse_stack.is_empty();
            self.block_parse_stack
                .push(BlockFrame::new(BlockFrameKind::PlainBlock, return_on_close));
            return self.run_block_parse_loop();
        }

        // `{ x -> … }` / `{ a, b -> … }` no longer create closures in expression position.
        if self.scan_ahead_for_arrow() {
            return Err(self.error(
                "Use `lambda params { body }` for closures; `{ x -> … }` is no longer valid in expression position",
            ));
        }

        // Anonymous `{ x = … }` / `{ x, y }` construction abolished — require `TypeName { … }`.
        let looks_like_anon_struct = if matches!(self.current_kind(), TokenKind::Ident(_)) {
            match self.peek2() {
                TokenKind::Eq | TokenKind::Colon => true,
                TokenKind::Comma => true,
                _ => false,
            }
        } else {
            false
        };
        if looks_like_anon_struct {
            return Err(self.error(
                "Use `TypeName { field = … }` for struct construction; anonymous `{ x = … }` / `{ x, y }` is no longer valid",
            ));
        }

        // Immediate block (single expr or multi-stmt)
        let return_on_close = !self.block_parse_stack.is_empty();
        self.block_parse_stack
            .push(BlockFrame::new(BlockFrameKind::PlainBlock, return_on_close));
        self.run_block_parse_loop()
    }

    /// Trailing lambda after a call (Phase 5).
    ///
    /// - `{ it … }` — implicit-it
    /// - `{ body }` — no-param / body-only
    /// - `{ a, b` + newline + `body }` or `{ a, b; body }` — explicit param line
    /// - `{ a, b -> body }` — **rejected** (migrate to param-line form)
    pub(crate) fn parse_trailing_lambda_brace(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // skip '{'

        if self.skip(TokenKind::RBrace) {
            return Ok(ExprKind::Lambda {
                params: vec![],
                body: Box::new(ExprKind::Block(vec![]).into()),
                implicit_it: false,
            }
            .into());
        }

        if self.skip(TokenKind::Colon) {
            self.expect(TokenKind::RBrace)?;
            return Err(self.error("Use Map[] for empty map literal, not {:}"));
        }

        if self.brace_starts_statement_block() {
            let return_on_close = !self.block_parse_stack.is_empty();
            self.block_parse_stack
                .push(BlockFrame::new(BlockFrameKind::LambdaBody, return_on_close));
            return self.run_block_parse_loop();
        }

        let is_struct = if matches!(self.current_kind(), TokenKind::Ident(_)) {
            match self.peek2() {
                TokenKind::Eq | TokenKind::Colon => true,
                TokenKind::Comma => false, // may be param list; not struct in trailing
                _ => false,
            }
        } else {
            false
        };
        if is_struct {
            return Err(self.error("Expected lambda after call, got struct literal"));
        }

        // Phase 5: abolish `{ a, b -> body }` in trailing position.
        if self.scan_ahead_for_arrow() {
            return Err(self.error(
                "Use trailing param line `{ a, b` + newline + `body }` or `{ a, b; body }`; `{ a, b -> body }` is no longer valid",
            ));
        }

        if let Some(params) = self.try_trailing_param_line() {
            return self.parse_trailing_param_line_body(params);
        }

        // Multi-param start without separator → sticky parse, reject early.
        if matches!(self.current_kind(), TokenKind::Ident(_)) && self.peek2() == TokenKind::Comma {
            return Err(self.error(
                "Trailing lambda multi-params need a newline or `;` after the param line (e.g. `{ a, b; body }`)",
            ));
        }

        if let TokenKind::Ident(ref first_id) = self.current_kind() {
            if first_id == "it" {
                let body = self.parse_expr()?;
                self.expect(TokenKind::RBrace)?;
                return Ok(Expr::it_lambda(body));
            }
        }

        let return_on_close = !self.block_parse_stack.is_empty();
        self.block_parse_stack
            .push(BlockFrame::new(BlockFrameKind::LambdaBody, return_on_close));
        self.run_block_parse_loop()
    }

    /// Peek whether `{` content (already past `{`) starts a Phase-5 param line.
    /// Param line = `Ident (, Ident)*` followed by `;` or a token on the next source line.
    fn try_trailing_param_line(&self) -> Option<Vec<String>> {
        let mut peek = self.pos;
        let mut params = Vec::new();
        loop {
            let Some(tok) = self.tokens.get(peek) else {
                return None;
            };
            let TokenKind::Ident(name) = &tok.kind else {
                return None;
            };
            params.push(name.clone());
            let param_line = tok.span.line;
            peek += 1;
            let Some(next) = self.tokens.get(peek) else {
                return None;
            };
            match &next.kind {
                TokenKind::Comma => {
                    peek += 1;
                    continue;
                }
                TokenKind::Semicolon => return Some(params),
                TokenKind::RBrace => {
                    // `{ a, b }` with no body — treat as param line + empty body only if multi.
                    return if params.len() > 1 { Some(params) } else { None };
                }
                _ => {
                    if next.span.line > param_line {
                        return Some(params);
                    }
                    // Same line continuation → expression body / `it`, not a param line.
                    return None;
                }
            }
        }
    }

    fn parse_trailing_param_line_body(&mut self, params: Vec<String>) -> Result<Expr, ParseError> {
        // Consume `Ident (, Ident)*`
        for (i, expected) in params.iter().enumerate() {
            match &self.current_kind() {
                TokenKind::Ident(name) if name == expected => self.advance(),
                _ => return Err(self.error("Internal error: trailing param line mismatch")),
            }
            if i + 1 < params.len() {
                self.expect(TokenKind::Comma)?;
            }
        }
        // Separator: `;` or already on next line (no token to consume).
        if self.current_kind() == TokenKind::Semicolon {
            self.advance();
        }

        if self.skip(TokenKind::RBrace) {
            return Ok(ExprKind::Lambda {
                params,
                body: Box::new(ExprKind::Block(vec![]).into()),
                implicit_it: false,
            }
            .into());
        }

        let return_on_close = !self.block_parse_stack.is_empty();
        self.block_parse_stack
            .push(BlockFrame::new(BlockFrameKind::PlainBlock, return_on_close));
        let body = self.run_block_parse_loop()?;
        Ok(ExprKind::Lambda {
            params,
            body: Box::new(body),
            implicit_it: false,
        }
        .into())
    }

    /// Back-compat name used by call/pratt trailing paths.
    pub(crate) fn parse_lambda_or_struct(&mut self) -> Result<Expr, ParseError> {
        self.parse_trailing_lambda_brace()
    }

    /// Parse struct fields; `{` must already have been consumed.
    pub(crate) fn parse_struct_literal(
        &mut self,
        type_name: Option<String>,
    ) -> Result<Expr, ParseError> {
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
            } else if self.current_kind() == TokenKind::Colon {
                // `{ x: expr }` field init (legacy colon form)
                self.advance();
                let value = self.parse_expr()?;
                fields.push((name, value));
            } else {
                // Shorthand: {x} becomes {x = x}
                fields.push((name.clone(), ExprKind::Ident(name).into()));
            }
        }
        self.expect(TokenKind::RBrace)?;
        Ok(ExprKind::StructLiteral { type_name, fields }.into())
    }

    /// Peek: current token is `{` and body looks like a struct literal (not lambda/block).
    /// Note: `{ x }` alone is a block/lambda, NOT a struct — use `Point { x = x }` for
    /// single-field shorthand so `if a < b { x }` is not parsed as `b { x }`.
    pub(crate) fn brace_starts_struct_literal(&self) -> bool {
        let Some(after_brace) = self.tokens.get(self.pos + 1) else {
            return false;
        };
        match &after_brace.kind {
            TokenKind::Ident(_) => match self.tokens.get(self.pos + 2).map(|t| &t.kind) {
                Some(TokenKind::Eq | TokenKind::Colon) => true,
                Some(TokenKind::Comma) => !self.scan_ahead_for_arrow_from(self.pos + 1),
                _ => false,
            },
            _ => false,
        }
    }

    /// Check if the token after the current one is a colon (for Map detection)
    /// Legacy `{ params -> body }` parser (kept for potential diagnostics; unused after Phase 5).
    #[allow(dead_code)]
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
                | TokenKind::If
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
            | TokenKind::If
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
