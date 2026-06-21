#!/usr/bin/env python3
"""Wrap ExprKind in parser/expr.rs with span-aware Expr."""

from pathlib import Path

P = Path(__file__).resolve().parent.parent / "crates/action-frontend/src/parser/expr.rs"


def main() -> None:
    text = P.read_text()
    subs = [
        ("matches!(&left, ExprKind::", "matches!(&left.kind, ExprKind::"),
        ("matches!(&func, ExprKind::", "matches!(&func.kind, ExprKind::"),
        ("if matches!(lambda, ExprKind::", "if matches!(&lambda.kind, ExprKind::"),
        ("match &left {\n                        ExprKind::Ident",
         "match &left.kind {\n                        ExprKind::Ident"),
        ("if let ExprKind::Tuple(elems) = left {", "if let ExprKind::Tuple(elems) = left.kind.clone() {"),
        ("if let ExprKind::Ident(ref name) = first {", "if let ExprKind::Ident(ref name) = first.kind {"),
        ("Expr::call(ExprKind::Ident(", "Expr::call(Expr::ident("),
        ("parse_call_suffix(ExprKind::Ident(name.clone()))",
         "parse_call_suffix(self.make_expr(ExprKind::Ident(name.clone())))"),
        ("parse_call_suffix(self.make_expr(ExprKind::Ident(name.clone()))))",
         "parse_call_suffix(self.make_expr(ExprKind::Ident(name.clone())))"),
        ("fields.push((name.clone(), ExprKind::Ident(name)))",
         "fields.push((name.clone(), self.make_expr(ExprKind::Ident(name))))"),
        ("left = ExprKind::FieldAccess(Box::new(left), field);",
         "left = self.make_expr_from(&left, ExprKind::FieldAccess(Box::new(left), field));"),
        ("left = ExprKind::Index(Box::new(left), Box::new(idx));",
         "left = self.make_expr_merge(&left, &idx, ExprKind::Index(Box::new(left), Box::new(idx)));"),
        ("left = ExprKind::FunctionRef(format!(\"{}.{{}}\", type_name, method));",
         "left = self.make_expr_from(&left, ExprKind::FunctionRef(format!(\"{}.{{}}\", type_name, method)));"),
        ("left = ExprKind::Binary(Box::new(left), op, Box::new(right));",
         "left = self.make_expr_merge(&left, &right, ExprKind::Binary(Box::new(left), op, Box::new(right)));"),
        ("left = ExprKind::Tuple(elements);", "left = self.make_expr(ExprKind::Tuple(elements));"),
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
        ("Ok(ExprKind::Literal(Literal::Unit))", "Ok(self.make_expr(ExprKind::Literal(Literal::Unit)))"),
        ("Ok(ExprKind::Tuple(exprs))", "Ok(self.make_expr(ExprKind::Tuple(exprs)))"),
        ("Ok(ExprKind::SetLiteral(elements))", "Ok(self.make_expr(ExprKind::SetLiteral(elements)))"),
        ("Ok(ExprKind::MapLiteral(entries))", "Ok(self.make_expr(ExprKind::MapLiteral(entries)))"),
        ("Ok(ExprKind::StructLiteral(fields))", "Ok(self.make_expr(ExprKind::StructLiteral(fields)))"),
        ("Ok(ExprKind::Block(stmts))", "Ok(self.make_expr(ExprKind::Block(stmts)))"),
        ("return Ok(ExprKind::Tuple(vec![]))", "return Ok(self.make_expr(ExprKind::Tuple(vec![])))"),
        ("ExprKind::Tuple(elems) => elements.extend(elems)", "ExprKind::Tuple(elems) => elements.extend(elems.iter().cloned())"),
        ("Ok(Expr::call(Expr::Ident(\"__list\".to_string()), items))", "Ok(Expr::call(Expr::ident(\"__list\"), items))"),
    ]
    for old, new in subs:
        text = text.replace(old, new)
    multiline = [
        ("left = ExprKind::OrBlock {\n                            nullable: Box::new(left),\n                            fallback: Box::new(fallback),\n                        };",
         "left = self.make_expr_merge(&left, &fallback, ExprKind::OrBlock {\n                            nullable: Box::new(left),\n                            fallback: Box::new(fallback),\n                        });"),
        ("left = ExprKind::Call {\n                                func: Box::new(left),\n                                args: vec![],\n                                trailing_lambda: Some(Box::new(lambda)),\n                            };",
         "left = self.make_expr_from(&left, ExprKind::Call {\n                                func: Box::new(left),\n                                args: vec![],\n                                trailing_lambda: Some(Box::new(lambda)),\n                            });"),
        ("left = ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(right),\n                    };",
         "left = self.make_expr_merge(&left, &right, ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(right),\n                    });"),
        ("left = ExprKind::Assign {\n                    target: Box::new(left),\n                    value: Box::new(ExprKind::Binary(Box::new(lhs_clone), base_op, Box::new(right))),\n                };",
         "let bin = self.make_expr_merge(&lhs_clone, &right, ExprKind::Binary(Box::new(lhs_clone), base_op, Box::new(right)));\n                left = self.make_expr_merge(&lhs_clone, &right, ExprKind::Assign {\n                    target: Box::new(left),\n                    value: Box::new(bin),\n                });"),
        ("return Ok(ExprKind::Call {\n                    func: Box::new(func),\n                    args,\n                    trailing_lambda: Some(Box::new(lambda)),\n                });",
         "return Ok(self.make_expr(ExprKind::Call {\n                    func: Box::new(func),\n                    args,\n                    trailing_lambda: Some(Box::new(lambda)),\n                }));"),
        ("Ok(ExprKind::Call {\n            func: Box::new(func),\n            args,\n            trailing_lambda: None,\n        })",
         "Ok(self.make_expr(ExprKind::Call {\n            func: Box::new(func),\n            args,\n            trailing_lambda: None,\n        }))"),
        ("Ok(ExprKind::Lambda {\n            params: vec![],\n            body: Box::new(body),\n            implicit_it: false,\n        })",
         "Ok(self.make_expr(ExprKind::Lambda {\n            params: vec![],\n            body: Box::new(body),\n            implicit_it: false,\n        }))"),
        ("return Ok(ExprKind::Lambda {\n                        params: vec![],\n                        body: Box::new(body),\n                        implicit_it: false,\n                    });",
         "return Ok(self.make_expr(ExprKind::Lambda {\n                        params: vec![],\n                        body: Box::new(body),\n                        implicit_it: false,\n                    }));"),
        ("Ok(ExprKind::Lambda {\n            params,\n            body: Box::new(body),\n            implicit_it: false,\n        })",
         "Ok(self.make_expr(ExprKind::Lambda {\n            params,\n            body: Box::new(body),\n            implicit_it: false,\n        }))"),
    ]
    for old, new in multiline:
        text = text.replace(old, new)
    P.write_text(text)
    print("expr.rs wrapped")


if __name__ == "__main__":
    main()
