use super::super::*;

impl Parser {
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
                    // Phase 4: nested `{}` is an empty block (Unit), not a Tuple/lambda.
                    let unit: Expr = ExprKind::Block(vec![]).into();
                    let span = unit.span;
                    self.block_parse_stack
                        .last_mut()
                        .expect("block stack non-empty")
                        .stmts
                        .push(Stmt::Expr { expr: unit, span });
                    continue;
                }
                self.block_parse_stack
                    .push(BlockFrame::new(BlockFrameKind::PlainBlock, false));
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
