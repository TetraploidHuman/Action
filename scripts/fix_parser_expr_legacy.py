#!/usr/bin/env python3
"""Fix parser/expr.rs postfix loop and primary expr sites after ExprKind migration."""

from pathlib import Path

P = Path(__file__).resolve().parent.parent / "crates/action-frontend/src/parser/expr.rs"


def main() -> None:
    t = P.read_text()

    subs = [
        ("left = ExprKind::FieldAccess(Box::new(left), field);",
         "left = self.make_expr_from(&left, ExprKind::FieldAccess(Box::new(left), field));"),
        ("let type_name = match &left {\n                        ExprKind::Ident(name)",
         "let type_name = match &left.kind {\n                        ExprKind::Ident(name)"),
        (
            'left = ExprKind::FunctionRef(format!("{}.{}"',
            'left = self.make_expr_from(&left, ExprKind::FunctionRef(format!("{}.{}"',
        ),
        (
            ', type_name, method));',
            ', type_name, method)));',
        ),
        ("left = ExprKind::Index(Box::new(left), Box::new(idx));",
         "left = self.make_expr_merge(&left, &idx, ExprKind::Index(Box::new(left), Box::new(idx)));"),
        ("left = ExprKind::OrBlock {\n                            nullable: Box::new(left),\n                            fallback: Box::new(fallback),\n                        };",
         "left = self.make_expr_merge(&left, &fallback, ExprKind::OrBlock {\n                            nullable: Box::new(left),\n                            fallback: Box::new(fallback),\n                        });"),
        ("matches!(&left, ExprKind::Ident(name)", "matches!(&left.kind, ExprKind::Ident(name)"),
        ("|| matches!(&left, ExprKind::FieldAccess(_, _))",
         "|| matches!(&left.kind, ExprKind::FieldAccess(_, _))"),
        ("if matches!(lambda, ExprKind::Lambda { .. })",
         "if matches!(&lambda.kind, ExprKind::Lambda { .. })"),
        ("left = ExprKind::Call {\n                                func: Box::new(left),\n                                args: vec![],\n                                trailing_lambda: Some(Box::new(lambda)),\n                            };",
         "left = self.make_expr_from(&left, ExprKind::Call {\n                                func: Box::new(left),\n                                args: vec![],\n                                trailing_lambda: Some(Box::new(lambda)),\n                            });"),
        ("left = ExprKind::Assign {\n                    target: Box::new(left),\n                    value: Box::new(ExprKind::Binary(Box::new(lhs_clone), base_op, Box::new(right))),\n                };",
         "let bin = self.make_expr_merge(&lhs_clone, &right, ExprKind::Binary(Box::new(lhs_clone), base_op, Box::new(right)));\n                left = self.make_expr_merge(&lhs_clone, &right, ExprKind::Assign {\n                    target: Box::new(left),\n                    value: Box::new(bin),\n                });"),
        ("left = ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(right),\n                    };",
         "left = self.make_expr_merge(&left, &right, ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(right),\n                    });"),
        ("left = ExprKind::Binary(Box::new(left), op, Box::new(right));",
         "left = self.make_expr_merge(&left, &right, ExprKind::Binary(Box::new(left), op, Box::new(right)));"),
        ("left = ExprKind::Tuple(elements);", "left = self.make_expr(ExprKind::Tuple(elements));"),
        ("if let ExprKind::Tuple(elems) = left {", "if let ExprKind::Tuple(elems) = left.kind.clone() {"),
        ("match right {\n                        ExprKind::Tuple(elems) => elements.extend(elems),",
         "match right.kind {\n                        ExprKind::Tuple(elems) => elements.extend(elems.iter().cloned()),"),
        ("matches!(&func, ExprKind::Ident(_)) || matches!(&func, ExprKind::FieldAccess(_, _))",
         "matches!(&func.kind, ExprKind::Ident(_)) || matches!(&func.kind, ExprKind::FieldAccess(_, _))"),
        ("if matches!(lambda, ExprKind::Lambda { .. }) {",
         "if matches!(&lambda.kind, ExprKind::Lambda { .. }) {"),
        ("return Ok(ExprKind::Call {", "return Ok(self.make_expr(ExprKind::Call {"),
        ("Ok(ExprKind::Call {", "Ok(self.make_expr(ExprKind::Call {"),
        ("Ok(ExprKind::Null)", "Ok(self.make_expr(ExprKind::Null))"),
        ("Ok(ExprKind::Literal(Literal::Char(c)))", "Ok(self.make_expr(ExprKind::Literal(Literal::Char(c))))"),
        ("Ok(ExprKind::Ident(name))", "Ok(self.make_expr(ExprKind::Ident(name)))"),
        ("Ok(ExprKind::FunctionRef(path))", "Ok(self.make_expr(ExprKind::FunctionRef(path)))"),
        ("Ok(ExprKind::Continue)", "Ok(self.make_expr(ExprKind::Continue))"),
        ("Ok(ExprKind::Break)", "Ok(self.make_expr(ExprKind::Break))"),
        ("Ok(ExprKind::Copy(Box::new(expr)))", "Ok(self.make_expr(ExprKind::Copy(Box::new(expr))))"),
        ("Ok(ExprKind::Unsafe(Box::new(body)))", "Ok(self.make_expr(ExprKind::Unsafe(Box::new(body))))"),
        ("Ok(ExprKind::Ident(\"_\".to_string()))", "Ok(self.make_expr(ExprKind::Ident(\"_\".to_string())))"),
        ("Ok(ExprKind::StringInterpolate(parts))", "Ok(self.make_expr(ExprKind::StringInterpolate(parts)))"),
        ("return Ok(ExprKind::Literal(Literal::Unit))", "return Ok(self.make_expr(ExprKind::Literal(Literal::Unit)))"),
        ("return Ok(ExprKind::Tuple(exprs))", "return Ok(self.make_expr(ExprKind::Tuple(exprs)))"),
        ("Ok(ExprKind::Tuple(exprs))", "Ok(self.make_expr(ExprKind::Tuple(exprs)))"),
        ("Ok(Expr::call(Expr::Ident(\"__list\".to_string()), items))",
         "Ok(Expr::call(Expr::ident(\"__list\"), items))"),
        ("Ok(ExprKind::SetLiteral(elements))", "Ok(self.make_expr(ExprKind::SetLiteral(elements)))"),
        ("Ok(ExprKind::MapLiteral(entries))", "Ok(self.make_expr(ExprKind::MapLiteral(entries)))"),
        ("return Ok(ExprKind::Tuple(vec![]))", "return Ok(self.make_expr(ExprKind::Tuple(vec![])))"),
        ("return Ok(ExprKind::it_lambda(body))", "return Ok(Expr::it_lambda(body))"),
        ("Ok(ExprKind::Lambda {", "Ok(self.make_expr(ExprKind::Lambda {"),
        ("return Ok(ExprKind::Lambda {", "return Ok(self.make_expr(ExprKind::Lambda {"),
        ("fields.push((name.clone(), ExprKind::Ident(name)))",
         "fields.push((name.clone(), self.make_expr(ExprKind::Ident(name))))"),
        ("Ok(ExprKind::StructLiteral(fields))", "Ok(self.make_expr(ExprKind::StructLiteral(fields)))"),
        ("return Ok(ExprKind::it_lambda(body))", "return Ok(Expr::it_lambda(body))"),
        ("Ok(ExprKind::Block(stmts))", "Ok(self.make_expr(ExprKind::Block(stmts)))"),
        ("self.parse_call_suffix(ExprKind::Ident(name.clone()))",
         "self.parse_call_suffix(self.make_expr(ExprKind::Ident(name.clone())))"),
        ("let named_first = if let ExprKind::Ident(ref name) = first {",
         "let named_first = if let ExprKind::Ident(ref name) = first.kind {"),
    ]
    for old, new in subs:
        t = t.replace(old, new)

    # Close extra paren for wrapped Ok(self.make_expr(ExprKind::Call/...
    import re
    t = re.sub(
        r"Ok\(self\.make_expr\(ExprKind::Call \{([^}]+)\}\)\)",
        lambda m: f"Ok(self.make_expr(ExprKind::Call {{{m.group(1)}}}))",
        t,
        flags=re.DOTALL,
    )

    P.write_text(t)
    print("expr.rs legacy fixes applied")


if __name__ == "__main__":
    main()
