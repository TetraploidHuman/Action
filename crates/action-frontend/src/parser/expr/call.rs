use super::super::*;

impl Parser {
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
            let lambda = self.coerce_trailing_lambda(lambda)?;
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
}
