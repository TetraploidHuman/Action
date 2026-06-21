#!/usr/bin/env python3
"""Fix parser borrow errors by extracting span before moving Expr values."""

from pathlib import Path

P = Path(__file__).resolve().parent.parent / "crates/action-frontend/src/parser/expr.rs"


def main() -> None:
    t = P.read_text()
    subs = [
        (
            "left = self.make_expr_from(&left, ExprKind::FieldAccess(Box::new(left), field));",
            "let span = left.span;\n                    left = Expr::new(ExprKind::FieldAccess(Box::new(left), field), span);",
        ),
        (
            "left = self.make_expr_from(&left, ExprKind::FunctionRef(format!(\"{}.{}\", type_name, method)));",
            "let span = left.span;\n                    left = Expr::new(ExprKind::FunctionRef(format!(\"{}.{}\", type_name, method)), span);",
        ),
        (
            "left = self.make_expr_merge(&left, &idx, ExprKind::Index(Box::new(left), Box::new(idx)));",
            "let span = Self::merge_expr_spans(&left, &idx);\n                    left = Expr::new(ExprKind::Index(Box::new(left), Box::new(idx)), span);",
        ),
        (
            "left = self.make_expr_merge(&left, &fallback, ExprKind::OrBlock {\n                            nullable: Box::new(left),\n                            fallback: Box::new(fallback),\n                        });",
            "let span = Self::merge_expr_spans(&left, &fallback);\n                        left = Expr::new(ExprKind::OrBlock {\n                            nullable: Box::new(left),\n                            fallback: Box::new(fallback),\n                        }, span);",
        ),
        (
            "left = self.make_expr_from(&left, ExprKind::Call {\n                                func: Box::new(left),\n                                args: vec![],\n                                trailing_lambda: Some(Box::new(lambda)),\n                            });",
            "let span = left.span;\n                            left = Expr::new(ExprKind::Call {\n                                func: Box::new(left),\n                                args: vec![],\n                                trailing_lambda: Some(Box::new(lambda)),\n                            }, span);",
        ),
        (
            "let bin = self.make_expr_merge(&lhs_clone, &right, ExprKind::Binary(Box::new(lhs_clone), base_op, Box::new(right)));\n                left = self.make_expr_merge(&lhs_clone, &right, ExprKind::Assign {\n                    target: Box::new(left),\n                    value: Box::new(bin),\n                });",
            "let bin = Expr::new(\n                    ExprKind::Binary(Box::new(lhs_clone.clone()), base_op, Box::new(right.clone())),\n                    Self::merge_expr_spans(&lhs_clone, &right),\n                );\n                left = Expr::new(\n                    ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(bin),\n                    },\n                    Self::merge_expr_spans(&lhs_clone, &right),\n                );",
        ),
        (
            "left = self.make_expr_merge(&left, &right, ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(right),\n                    });",
            "let span = Self::merge_expr_spans(&left, &right);\n                    left = Expr::new(ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(right),\n                    }, span);",
        ),
        (
            "left = self.make_expr_merge(&left, &right, ExprKind::Binary(Box::new(left), op, Box::new(right)));",
            "let span = Self::merge_expr_spans(&left, &right);\n                left = Expr::new(ExprKind::Binary(Box::new(left), op, Box::new(right)), span);",
        ),
        (
            "left = self.wrap_expr(&left, ExprKind::FieldAccess(Box::new(left), field));",
            "let span = left.span;\n                    left = Expr::new(ExprKind::FieldAccess(Box::new(left), field), span);",
        ),
        (
            "left = self.wrap_expr(&left, ExprKind::FunctionRef(format!(\"{}.{}\", type_name, method)));",
            "let span = left.span;\n                    left = Expr::new(ExprKind::FunctionRef(format!(\"{}.{}\", type_name, method)), span);",
        ),
        (
            "left = self.wrap_merge(&left, &idx, ExprKind::Index(Box::new(left), Box::new(idx)));",
            "let span = Self::merge_expr_spans(&left, &idx);\n                    left = Expr::new(ExprKind::Index(Box::new(left), Box::new(idx)), span);",
        ),
        (
            "left = self.wrap_merge(&left, &fallback, ExprKind::OrBlock {\n                            nullable: Box::new(left),\n                            fallback: Box::new(fallback),\n                        });",
            "let span = Self::merge_expr_spans(&left, &fallback);\n                        left = Expr::new(ExprKind::OrBlock {\n                            nullable: Box::new(left),\n                            fallback: Box::new(fallback),\n                        }, span);",
        ),
        (
            "left = self.wrap_expr(&left, ExprKind::Call {\n                                func: Box::new(left),\n                                args: vec![],\n                                trailing_lambda: Some(Box::new(lambda)),\n                            });",
            "let span = left.span;\n                            left = Expr::new(ExprKind::Call {\n                                func: Box::new(left),\n                                args: vec![],\n                                trailing_lambda: Some(Box::new(lambda)),\n                            }, span);",
        ),
        (
            "let bin = self.wrap_merge(&lhs_clone, &right, ExprKind::Binary(Box::new(lhs_clone.clone()), base_op, Box::new(right.clone())));\n                left = self.wrap_merge(&lhs_clone, &right, ExprKind::Assign {\n                    target: Box::new(left),\n                    value: Box::new(bin),\n                });",
            "let bin = Expr::new(\n                    ExprKind::Binary(Box::new(lhs_clone.clone()), base_op, Box::new(right.clone())),\n                    Self::merge_expr_spans(&lhs_clone, &right),\n                );\n                left = Expr::new(\n                    ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(bin),\n                    },\n                    Self::merge_expr_spans(&lhs_clone, &right),\n                );",
        ),
        (
            "left = self.wrap_merge(&left, &right, ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(right),\n                    });",
            "let span = Self::merge_expr_spans(&left, &right);\n                    left = Expr::new(ExprKind::Assign {\n                        target: Box::new(left),\n                        value: Box::new(right),\n                    }, span);",
        ),
        (
            "left = self.wrap_merge(&left, &right, ExprKind::Binary(Box::new(left), op, Box::new(right)));",
            "let span = Self::merge_expr_spans(&left, &right);\n                left = Expr::new(ExprKind::Binary(Box::new(left), op, Box::new(right)), span);",
        ),
    ]
    for old, new in subs:
        t = t.replace(old, new)
    # fallback: any remaining wrap_merge/wrap_expr one-liners
    t = t.replace("self.wrap_merge(", "/*wrap_merge*/ self.wrap_merge(")
    P.write_text(t)
    print("parser span fixes applied")


if __name__ == "__main__":
    main()
