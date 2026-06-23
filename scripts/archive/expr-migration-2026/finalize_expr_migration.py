#!/usr/bin/env python3
"""Apply manual fixes after Expr/ExprKind bulk migration."""

from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TO_AST = ROOT / "crates/action-frontend/src/hir/to_ast.rs"
LOWER = ROOT / "crates/action-frontend/src/hir/lower.rs"
RESOLVE = ROOT / "crates/action-frontend/src/loader/resolve.rs"

TO_EXPR_FN = '''    fn to_expr(&self) -> Expr {
        let kind = match &self.kind {
            HirExprKind::Literal(l) => ExprKind::Literal(l.clone()),
            HirExprKind::Ident(n) => ExprKind::Ident(n.clone()),
            HirExprKind::Binary(lhs, op, rhs) => {
                ExprKind::Binary(Box::new(lhs.to_expr()), *op, Box::new(rhs.to_expr()))
            }
            HirExprKind::Unary(op, inner) => ExprKind::Unary(*op, Box::new(inner.to_expr())),
            HirExprKind::Call { func, args, trailing_lambda } => ExprKind::Call {
                func: Box::new(func.to_expr()),
                args: args.iter().map(HirExpr::to_expr).collect(),
                trailing_lambda: trailing_lambda.as_ref().map(|l| Box::new(l.to_expr())),
            },
            HirExprKind::Lambda { params, body, implicit_it } => ExprKind::Lambda {
                params: params.clone(),
                body: Box::new(body.to_expr()),
                implicit_it: *implicit_it,
            },
            HirExprKind::When(w) => ExprKind::When(Box::new(w.to_when())),
            HirExprKind::For(f) => ExprKind::For(Box::new(f.to_for())),
            HirExprKind::Block(stmts) => ExprKind::Block(stmts.iter().map(HirStmt::to_stmt).collect()),
            HirExprKind::StructLiteral(fields) => ExprKind::StructLiteral(
                fields.iter().map(|(n, e)| (n.clone(), e.to_expr())).collect(),
            ),
            HirExprKind::MapLiteral(entries) => ExprKind::MapLiteral(
                entries.iter().map(|(k, v)| (k.to_expr(), v.to_expr())).collect(),
            ),
            HirExprKind::SetLiteral(items) => ExprKind::SetLiteral(items.iter().map(HirExpr::to_expr).collect()),
            HirExprKind::FieldAccess(obj, field) => ExprKind::FieldAccess(Box::new(obj.to_expr()), field.clone()),
            HirExprKind::Index(obj, idx) => ExprKind::Index(Box::new(obj.to_expr()), Box::new(idx.to_expr())),
            HirExprKind::Range(start, end) => ExprKind::Range(Box::new(start.to_expr()), Box::new(end.to_expr())),
            HirExprKind::Tuple(items) => ExprKind::Tuple(
                items.iter().map(|(n, e)| (n.clone(), e.to_expr())).collect(),
            ),
            HirExprKind::Null => ExprKind::Null,
            HirExprKind::OrBlock { nullable, fallback } => ExprKind::OrBlock {
                nullable: Box::new(nullable.to_expr()),
                fallback: Box::new(fallback.to_expr()),
            },
            HirExprKind::Assign { target, value } => ExprKind::Assign {
                target: Box::new(target.to_expr()),
                value: Box::new(value.to_expr()),
            },
            HirExprKind::StringInterpolate(parts) => ExprKind::StringInterpolate(
                parts.iter().map(|p| match p {
                    HirStringPart::Literal(s) => StringPart::Literal(s.clone()),
                    HirStringPart::Expr(e) => StringPart::Expr(Box::new(e.to_expr())),
                }).collect(),
            ),
            HirExprKind::Continue => ExprKind::Continue,
            HirExprKind::Break => ExprKind::Break,
            HirExprKind::FunctionRef(n) => ExprKind::FunctionRef(n.clone()),
            HirExprKind::Copy(inner) => ExprKind::Copy(Box::new(inner.to_expr())),
            HirExprKind::Unsafe(inner) => ExprKind::Unsafe(Box::new(inner.to_expr())),
        };
        Expr::new(kind, self.span)
    }'''


def patch_to_ast() -> None:
    text = TO_AST.read_text()
    start = text.index("    fn to_expr(&self) -> Expr {")
    end = text.index("\n}\n\nimpl HirWhen", start)
    TO_AST.write_text(text[:start] + TO_EXPR_FN + text[end:])


def patch_lower() -> None:
    text = LOWER.read_text()
    text = text.replace("let kind = match expr {", "let kind = match &expr.kind {")
    text = text.replace("span: Span::default(),", "span: expr.span,")
    LOWER.write_text(text)


def patch_resolve() -> None:
    text = RESOLVE.read_text()
    if "if let ExprKind::FieldAccess(ref base, ref field) = expr.kind" in text:
        return
    old = """    fn transform_expr(expr: &mut Expr, prefixes: &HashSet<String>) {
        if let Expr::FieldAccess(ref base, ref field) = expr {
            if let Expr::Ident(ref ident) = **base {
                if prefixes.contains(ident) {
                    *expr = Expr::Ident(format!("{}_{}", ident, field));
                    return;
                }
            }
        }
        match expr {"""
    new = """    fn transform_expr(expr: &mut Expr, prefixes: &HashSet<String>) {
        if let ExprKind::FieldAccess(ref base, ref field) = expr.kind {
            if let ExprKind::Ident(ref ident) = base.kind {
                if prefixes.contains(ident) {
                    expr.kind = ExprKind::Ident(format!("{}_{}", ident, field));
                    return;
                }
            }
        }
        match &mut expr.kind {"""
    if old in text:
        text = text.replace(old, new)
        RESOLVE.write_text(text)


def main() -> None:
    patch_to_ast()
    patch_lower()
    patch_resolve()
    print("finalized hir + resolve")


if __name__ == "__main__":
    main()
