use super::*;

impl Parser {
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_pratt(Precedence::Lowest)
    }

    /// Peek ahead to check if a `(` starts a when-arm pattern rather than a function call.
    /// Returns true if the parenthesized content parses as comma-separated patterns
    /// followed by `)` then `->`.
    pub(crate) fn peek_when_arm_pattern(&mut self) -> bool {
        let saved = self.pos;
        self.advance(); // (
        let mut ok = self.parse_pattern().is_ok();
        while ok && self.current_kind() == TokenKind::Comma {
            self.advance();
            ok = self.parse_pattern().is_ok();
        }
        ok = ok && self.current_kind() == TokenKind::RParen;
        if ok {
            self.advance(); // )
            ok = self.current_kind() == TokenKind::Arrow;
        }
        self.pos = saved;
        ok
    }

    pub(crate) fn parse_pratt(&mut self, min_prec: Precedence) -> Result<Expr, ParseError> {
        let mut left = self.parse_prefix()?;

        loop {
            // Postfix operators first — they bind tighter than any binary operator.
            // After each postfix, continue the outer loop so binary operators after
            // the postfix (e.g. r.x + 1) are correctly parsed.
            let postfix_applied = match self.current_kind() {
                TokenKind::LParen => {
                    if self.no_postfix_call && self.peek_when_arm_pattern() {
                        false
                    } else {
                        left = self.parse_call_suffix(left)?;
                        true
                    }
                }
                TokenKind::Dot => {
                    self.advance();
                    let field = match &self.current_kind() {
                        TokenKind::Ident(s) => {
                            let name = s.clone();
                            self.advance();
                            name
                        }
                        TokenKind::IntLiteral(n) => {
                            let name = n.to_string();
                            self.advance();
                            name
                        }
                        TokenKind::When | TokenKind::For => {
                            let kw = self.current_kind().to_string();
                            self.advance();
                            kw
                        }
                        _ => return Err(self.error("Expected field name after '.'")),
                    };
                    left = ExprKind::FieldAccess(Box::new(left), field).into();
                    true
                }
                TokenKind::ColonColon => {
                    self.advance();
                    let method = match &self.current_kind() {
                        TokenKind::Ident(s) => {
                            let name = s.clone();
                            self.advance();
                            name
                        }
                        _ => return Err(self.error("Expected method name after '::'")),
                    };
                    let type_name = match &left.kind {
                        ExprKind::Ident(name) => name.clone(),
                        _ => {
                            return Err(
                                self.error("Expected type name before '::' (e.g., Int::toString)")
                            )
                        }
                    };
                    left = ExprKind::FunctionRef(format!("{}.{}", type_name, method)).into();
                    true
                }
                TokenKind::LBracket => {
                    self.advance();
                    let idx = self.parse_expr()?;
                    self.expect(TokenKind::RBracket)?;
                    left = ExprKind::Index(Box::new(left), Box::new(idx)).into();
                    true
                }
                TokenKind::Question => {
                    self.advance();
                    // ? is only valid after a type or as part of ?. (safe call sugar)
                    // Standalone ? after expression is not valid.
                    // Check for ?. safe call sugar
                    if self.current_kind() == TokenKind::Dot {
                        self.advance(); // skip '.'
                        let field = match &self.current_kind() {
                            TokenKind::Ident(s) => {
                                let name = s.clone();
                                self.advance();
                                name
                            }
                            _ => return Err(self.error("Expected field name after '?.'")),
                        };
                        left = ExprKind::FieldAccess(Box::new(left), field).into();
                    } else if self.current_kind() == TokenKind::LBracket {
                        self.advance(); // skip '['
                        let idx = self.parse_expr()?;
                        self.expect(TokenKind::RBracket)?;
                        left = ExprKind::Index(Box::new(left), Box::new(idx)).into();
                    } else if self.current_kind() == TokenKind::LParen {
                        left = self.parse_call_suffix(left)?;
                    } else {
                        return Err(self.error("Unexpected '?'. Use 'or { }' for nullable fallback, or '?.' for safe call"));
                    }
                    true
                }
                TokenKind::Or => {
                    // Check for or-block (nullable fallback): expr or { ... }
                    // Only when followed by { — otherwise it's logical OR
                    if self.peek2() == TokenKind::LBrace {
                        self.advance(); // skip 'or'
                        let fallback = self.parse_block_expr()?;
                        left = ExprKind::OrBlock {
                            nullable: Box::new(left),
                            fallback: Box::new(fallback),
                        }
                        .into();
                        true
                    } else {
                        false
                    }
                }
                TokenKind::LBrace => {
                    let is_callable = matches!(&left.kind, ExprKind::Ident(name)
                        if name == "launch" || name == "coroutineScope")
                        || matches!(&left.kind, ExprKind::FieldAccess(_, _));
                    if is_callable {
                        let lambda = self.parse_lambda_or_struct()?;
                        if matches!(&lambda.kind, ExprKind::Lambda { .. }) {
                            left = ExprKind::Call {
                                func: Box::new(left),
                                args: vec![],
                                trailing_lambda: Some(Box::new(lambda)),
                            }
                            .into();
                            true
                        } else {
                            return Err(self.error("Expected lambda after call"));
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            };
            if postfix_applied {
                continue;
            }

            // Binary / compound / special operators
            let tok_kind = self.current_kind();
            if is_compound_assign(&tok_kind) {
                let base_op = compound_to_binary(&tok_kind).unwrap();
                self.advance();
                let right = self.parse_pratt(Precedence::Assignment.next())?;
                let lhs_clone = left.clone();
                left = ExprKind::Assign {
                    target: Box::new(left),
                    value: Box::new(
                        ExprKind::Binary(Box::new(lhs_clone), base_op, Box::new(right)).into(),
                    ),
                }
                .into();
                continue;
            }

            if let Some(op) = token_to_binary_op(&tok_kind) {
                let prec = Precedence::of_binary(&op);
                if prec < min_prec {
                    break;
                }
                self.advance();
                let mut right = self.parse_pratt(prec.next())?;
                loop {
                    let next_kind = self.current_kind();
                    if let Some(op2) = token_to_binary_op(&next_kind) {
                        let prec2 = Precedence::of_binary(&op2);
                        if prec2 == prec && is_left_associative(&op2) {
                            self.advance();
                            let r2 = self.parse_pratt(prec.next())?;
                            right = ExprKind::Binary(Box::new(right), op2, Box::new(r2)).into();
                            continue;
                        }
                    }
                    break;
                }
                if op == BinaryOp::Assign {
                    left = ExprKind::Assign {
                        target: Box::new(left),
                        value: Box::new(right),
                    }
                    .into();
                } else {
                    left = ExprKind::Binary(Box::new(left), op, Box::new(right)).into();
                }
                continue;
            }

            if tok_kind == TokenKind::In || tok_kind == TokenKind::Is {
                let prec = Precedence::Comparison;
                if prec < min_prec {
                    break;
                }
                let op = if tok_kind == TokenKind::In {
                    BinaryOp::In
                } else {
                    BinaryOp::Is
                };
                self.advance();
                let right = self.parse_pratt(prec.next())?;
                left = ExprKind::Binary(Box::new(left), op, Box::new(right)).into();
                continue;
            }

            if let TokenKind::Ident(ref s) = tok_kind {
                if s == "to" {
                    let prec = Precedence::To;
                    if prec < min_prec {
                        break;
                    }
                    self.advance();
                    let right = self.parse_pratt(prec.next())?;
                    let mut elements = if let ExprKind::Tuple(elems) = &left.kind {
                        elems.clone()
                    } else {
                        vec![(None, left)]
                    };
                    match &right.kind {
                        ExprKind::Tuple(elems) => elements.extend(elems.clone()),
                        _ => elements.push((None, right)),
                    }
                    left = ExprKind::Tuple(elements).into();
                    continue;
                }
            }

            break;
        }

        Ok(left)
    }

    pub(crate) fn parse_call_suffix(&mut self, func: Expr) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LParen)?;
        let mut args = Vec::new();
        while self.current_kind() != TokenKind::RParen {
            if !args.is_empty() {
                self.expect(TokenKind::Comma)?;
            }
            // Check for trailing lambda (syntax sugar)
            // If we see a { at the end, treat it as a lambda
            if self.current_kind() == TokenKind::LBrace && self.peek2() != TokenKind::Eq {
                // Could be a lambda { ... } or a struct literal {x = ...}
                // We distinguish by looking ahead: { ident -> ... } is lambda
                // { ident = ... } is struct literal
                // For simplicity: look ahead a few tokens
                // Let's just treat it as expression
                args.push(self.parse_expr()?);
            } else {
                args.push(self.parse_expr()?);
            }
        }
        self.expect(TokenKind::RParen)?;

        // Check for trailing lambda (outside parentheses).
        // Only consume { as trailing lambda if the content looks like a lambda
        // (params -> body or expression), not a statement block (var/val/for/when/...).
        let is_simple_target = matches!(&func.kind, ExprKind::Ident(_))
            || matches!(&func.kind, ExprKind::FieldAccess(_, _));
        if is_simple_target
            && !self.no_trailing_lambda
            && self.current_kind() == TokenKind::LBrace
            && self.brace_is_lambda_like()
        {
            let lambda = self.parse_lambda_or_struct()?;
            if matches!(&lambda.kind, ExprKind::Lambda { .. }) {
                return Ok(ExprKind::Call {
                    func: Box::new(func),
                    args,
                    trailing_lambda: Some(Box::new(lambda)),
                }
                .into());
            } else {
                return Err(self.error("Expected lambda after call"));
            }
        }

        Ok(ExprKind::Call {
            func: Box::new(func),
            args,
            trailing_lambda: None,
        }
        .into())
    }

    pub(crate) fn parse_prefix(&mut self) -> Result<Expr, ParseError> {
        match self.current_kind() {
            TokenKind::IntLiteral(n) => {
                self.advance();
                Ok(Expr::int(n))
            }
            TokenKind::FloatLiteral(n) => {
                self.advance();
                Ok(Expr::float(n))
            }
            TokenKind::BoolLiteral(b) => {
                self.advance();
                Ok(Expr::bool(b))
            }
            TokenKind::Null => {
                self.advance();
                Ok(ExprKind::Null.into())
            }
            TokenKind::CharLiteral(c) => {
                self.advance();
                Ok(ExprKind::Literal(Literal::Char(c)).into())
            }
            TokenKind::StringLiteral(ref s) => {
                let s = s.clone();
                let str_span = self.current_span();
                self.advance();
                // Check for string interpolation: if string contains $ or ${
                if s.contains('$') {
                    self.parse_interpolated_string(&s, str_span)
                } else {
                    Ok(Expr::string(&s))
                }
            }
            TokenKind::Ident(ref name) => {
                let name = name.clone();
                self.advance();

                // Collection literals: List[...], Set[...], Map[...]
                if (name == "List" || name == "Set" || name == "Map")
                    && self.current_kind() == TokenKind::LBracket
                {
                    return self.parse_collection_literal(&name);
                }
                // Check for function call (identifier followed by paren)
                if self.current_kind() == TokenKind::LParen {
                    self.parse_call_suffix(ExprKind::Ident(name.clone()).into())
                } else {
                    Ok(ExprKind::Ident(name).into())
                }
            }
            TokenKind::ColonColon => {
                self.advance(); // skip ::
                                // Parse function reference path
                let mut path = String::new();
                match &self.current_kind() {
                    TokenKind::Ident(s) => {
                        path.push_str(s);
                        self.advance();
                    }
                    _ => return Err(self.error("Expected function name after ::").into()),
                }
                // Parse rest of path: ::method_name or .field_name
                while self.current_kind() == TokenKind::ColonColon
                    || self.current_kind() == TokenKind::Dot
                {
                    if self.current_kind() == TokenKind::ColonColon {
                        path.push_str("::");
                    } else {
                        path.push('.');
                    }
                    self.advance();
                    match &self.current_kind() {
                        TokenKind::Ident(s) => {
                            path.push_str(s);
                            self.advance();
                        }
                        _ => {
                            return Err(self
                                .error("Expected identifier in function reference path")
                                .into())
                        }
                    }
                }
                Ok(ExprKind::FunctionRef(path).into())
            }
            TokenKind::Minus => {
                self.advance();
                let expr = self.parse_pratt(Precedence::Unary)?;
                Ok(Expr::unary(UnaryOp::Neg, expr))
            }
            TokenKind::Not => {
                self.advance();
                let expr = self.parse_pratt(Precedence::Unary)?;
                Ok(Expr::unary(UnaryOp::Not, expr))
            }
            TokenKind::Tilde => {
                self.advance();
                let expr = self.parse_pratt(Precedence::Unary)?;
                Ok(Expr::unary(UnaryOp::BitNot, expr))
            }
            TokenKind::Continue => {
                self.advance();
                Ok(ExprKind::Continue.into())
            }
            TokenKind::Break => {
                self.advance();
                Ok(ExprKind::Break.into())
            }
            TokenKind::When => self.parse_when(),
            TokenKind::For => self.parse_for(),
            TokenKind::Copy => {
                self.advance();
                let expr = self.parse_prefix()?;
                Ok(ExprKind::Copy(Box::new(expr)).into())
            }
            TokenKind::Unsafe => {
                self.advance();
                self.expect(TokenKind::LBrace)?;
                self.block_parse_stack.push(BlockFrame::new(
                    BlockFrameKind::PlainBlock,
                    !self.block_parse_stack.is_empty(),
                ));
                let body = self.run_block_parse_loop()?;
                Ok(ExprKind::Unsafe(Box::new(body)).into())
            }
            TokenKind::LBrace => self.parse_lambda_or_struct(),
            TokenKind::LParen => self.parse_paren_or_tuple(),
            // [ alone is no longer a list literal — use List[...] instead
            TokenKind::LBracket => Err(self.error(
                "Unexpected '[' — use List[...] for list literals, or variable[index] for indexing",
            )),
            TokenKind::Underscore => {
                self.advance();
                // Wildcard pattern — typically used in patterns, return as Ident for now
                Ok(ExprKind::Ident("_".to_string()).into())
            }
            _ => Err(self.error(&format!("Unexpected token: {}", self.current_kind()))),
        }
    }

    pub(crate) fn parse_interpolated_string(
        &self,
        s: &str,
        str_span: Span,
    ) -> Result<Expr, ParseError> {
        // Handle ${expr} interpolation only (per v6 spec)
        let mut parts = Vec::new();
        let mut current = String::new();
        let chars: Vec<char> = s.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            if chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                if !current.is_empty() {
                    parts.push(StringPart::Literal(current.clone()));
                    current.clear();
                }
                // ${expr}
                let mut expr_str = String::new();
                let mut depth = 1;
                i += 2;
                while i < chars.len() && depth > 0 {
                    if chars[i] == '{' {
                        depth += 1;
                    } else if chars[i] == '}' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    expr_str.push(chars[i]);
                    i += 1;
                }
                // Parse the embedded expression
                let mut sub_lexer = Lexer::new(&expr_str);
                let sub_tokens = sub_lexer.tokenize();
                // Propagate lexer errors from interpolated expressions
                let sub_errors = sub_lexer.take_errors();
                if !sub_errors.is_empty() {
                    let msgs: Vec<String> = sub_errors.iter().map(|e| e.to_string()).collect();
                    return Err(ParseError {
                        message: msgs.join("\n"),
                        span: str_span,
                    });
                }
                let mut sub_parser = Parser::new(sub_tokens);
                // Propagate parse errors from the interpolated expression to the user.
                // Previously errors were silently swallowed and the raw ${...} text
                // was emitted as a literal — this made interpolation typos invisible.
                let expr = sub_parser.parse_expr().map_err(|e| ParseError {
                    message: format!("In string interpolation: {}", e.message),
                    span: str_span,
                })?;
                parts.push(StringPart::Expr(Box::new(expr)));
            } else {
                current.push(chars[i]);
            }
            i += 1;
        }
        if !current.is_empty() {
            parts.push(StringPart::Literal(current));
        }
        Ok(ExprKind::StringInterpolate(parts).into())
    }

    pub(crate) fn parse_paren_or_tuple(&mut self) -> Result<Expr, ParseError> {
        self.advance(); // skip '('

        if self.skip(TokenKind::RParen) {
            return Ok(ExprKind::Literal(Literal::Unit).into());
        }

        let first = self.parse_expr()?;

        // Check for named tuple: (name: value, ...)
        // If first expr is an Ident followed by ':', treat as named field

        // Check for named first element: (name: value, ...)
        let mut exprs: Vec<(Option<String>, Expr)> = Vec::new();

        // Check if first expr is named: ident followed by ':'
        let named_first = if let ExprKind::Ident(ref name) = first.kind {
            if self.current_kind() == TokenKind::Colon {
                let field_name = name.clone();
                self.advance(); // skip ':'
                let value = self.parse_expr()?;
                Some((field_name, value))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((name, val)) = named_first {
            exprs.push((Some(name), val));
        } else {
            exprs.push((None, first));
        }

        if self.skip(TokenKind::RParen) {
            // Single expression in parens — but now unwrap from tuple wrapper
            if exprs.len() == 1 && exprs[0].0.is_none() {
                return Ok(exprs.remove(0).1);
            }
            return Ok(ExprKind::Tuple(exprs).into());
        }

        // Tuple
        self.expect(TokenKind::Comma)?;
        while self.current_kind() != TokenKind::RParen {
            // Check for named field: identifier : expression
            if let TokenKind::Ident(_) = &self.current_kind() {
                if self.peek2() == TokenKind::Colon {
                    let name = match &self.current_kind() {
                        TokenKind::Ident(s) => s.clone(),
                        _ => unreachable!(),
                    };
                    self.advance(); // skip name
                    self.advance(); // skip ':'
                    let value = self.parse_expr()?;
                    exprs.push((Some(name), value));
                } else {
                    exprs.push((None, self.parse_expr()?));
                }
            } else {
                exprs.push((None, self.parse_expr()?));
            }
            if self.current_kind() != TokenKind::RParen {
                self.expect(TokenKind::Comma)?;
            }
        }
        self.expect(TokenKind::RParen)?;
        Ok(ExprKind::Tuple(exprs).into())
    }

    /// Parse collection literal after `List`, `Set`, or `Map` keyword: List[...], Set[...], Map[...]
    pub(crate) fn parse_collection_literal(&mut self, kind: &str) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBracket)?; // consume '['

        match kind {
            "List" => {
                let mut items = Vec::new();
                while self.current_kind() != TokenKind::RBracket {
                    if !items.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    if self.current_kind() == TokenKind::RBracket {
                        break; // trailing comma
                    }
                    items.push(self.parse_expr()?);
                }
                self.expect(TokenKind::RBracket)?;
                Ok(Expr::call(
                    ExprKind::Ident("__list".to_string()).into(),
                    items,
                ))
            }
            "Set" => {
                let mut elements = Vec::new();
                while self.current_kind() != TokenKind::RBracket {
                    if !elements.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    if self.current_kind() == TokenKind::RBracket {
                        break; // trailing comma
                    }
                    let elem = self.parse_expr()?;
                    elements.push(elem);
                }
                self.expect(TokenKind::RBracket)?;
                Ok(ExprKind::SetLiteral(elements).into())
            }
            "Map" => {
                let mut entries = Vec::new();
                while self.current_kind() != TokenKind::RBracket {
                    if !entries.is_empty() {
                        self.expect(TokenKind::Comma)?;
                    }
                    if self.current_kind() == TokenKind::RBracket {
                        break; // trailing comma
                    }
                    let key = self.parse_expr()?;
                    self.expect(TokenKind::Colon)?;
                    let value = self.parse_expr()?;
                    entries.push((key, value));
                }
                self.expect(TokenKind::RBracket)?;
                Ok(ExprKind::MapLiteral(entries).into())
            }
            _ => unreachable!(),
        }
    }

    /// Scan ahead from current position to see if there's an `->` before `}` (at depth 0).
    /// Used to distinguish lambda params from struct shorthand fields.
    /// Peek past `{` to check if content looks like a lambda/struct, not a block.
    /// Returns false if the brace starts with statement keywords (var, val, for, ...).
    fn brace_is_lambda_like(&self) -> bool {
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

    fn brace_starts_block_body(&self) -> bool {
        if self.current_kind() != TokenKind::LBrace {
            return false;
        }
        let inner_pos = self.pos + 1;
        if inner_pos >= self.tokens.len() {
            return false;
        }
        match &self.tokens[inner_pos].kind {
            TokenKind::RBrace | TokenKind::Colon => false,
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

    fn frame_into_expr(frame: BlockFrame) -> Expr {
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

    pub(crate) fn run_block_parse_loop(&mut self) -> Result<Expr, ParseError> {
        loop {
            if self.current_kind() == TokenKind::RBrace {
                self.advance();
                let frame = self
                    .block_parse_stack
                    .pop()
                    .ok_or_else(|| self.error("Unmatched '}'"))?;
                let return_on_close = frame.return_on_close;
                let expr = Self::frame_into_expr(frame);
                if self.block_parse_stack.is_empty() || return_on_close {
                    return Ok(expr);
                }
                let span = expr.span;
                self.block_parse_stack
                    .last_mut()
                    .expect("block stack non-empty")
                    .stmts
                    .push(Stmt::Expr { expr, span });
                continue;
            }

            if self.current_kind() == TokenKind::LBrace && self.brace_starts_block_body() {
                self.advance();
                if self.skip(TokenKind::RBrace) {
                    let unit: Expr = ExprKind::Tuple(vec![]).into();
                    let span = unit.span;
                    self.block_parse_stack
                        .last_mut()
                        .expect("block stack non-empty")
                        .stmts
                        .push(Stmt::Expr { expr: unit, span });
                    continue;
                }
                self.block_parse_stack
                    .push(BlockFrame::new(BlockFrameKind::LambdaBody, false));
                continue;
            }

            let stmt = self.parse_statement()?;
            self.block_parse_stack
                .last_mut()
                .expect("block stack non-empty")
                .stmts
                .push(stmt);
            self.skip(TokenKind::Semicolon);
        }
    }

    pub(crate) fn parse_block_expr(&mut self) -> Result<Expr, ParseError> {
        self.expect(TokenKind::LBrace)?;
        let return_on_close = !self.block_parse_stack.is_empty();
        self.block_parse_stack
            .push(BlockFrame::new(BlockFrameKind::PlainBlock, return_on_close));
        self.run_block_parse_loop()
    }
}
