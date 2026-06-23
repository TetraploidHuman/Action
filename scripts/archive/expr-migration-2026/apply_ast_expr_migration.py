#!/usr/bin/env python3
"""Apply Expr -> ExprKind + struct Expr migration to ast.rs (idempotent)."""

from pathlib import Path
import re
import sys

P = Path(__file__).resolve().parent.parent / "crates/action-frontend/src/ast.rs"


def main() -> int:
    t = P.read_text()
    if "pub struct Expr" in t and "pub enum ExprKind" in t:
        print("ast.rs already migrated")
        return 0

    t = t.replace("pub enum Expr {", "pub enum ExprKind {", 1)
    needle = "    Unsafe(Box<Expr>),\n}\n\n#[derive(Debug, Clone, PartialEq)]\npub enum StringPart"
    insert = """    Unsafe(Box<Expr>),
}

/// Expression AST node with source span for diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    pub kind: ExprKind,
    pub span: Span,
}

impl Expr {
    pub fn new(kind: ExprKind, span: Span) -> Self { Expr { kind, span } }
    pub fn span(&self) -> Span { self.span }
    pub fn with_span(mut self, span: Span) -> Self { self.span = span; self }
}

impl Default for Expr {
    fn default() -> Self { Expr { kind: ExprKind::Null, span: Span::default() } }
}

impl From<ExprKind> for Expr {
    fn from(kind: ExprKind) -> Self { Expr::new(kind, Span::default()) }
}

impl std::ops::Deref for Expr {
    type Target = ExprKind;
    fn deref(&self) -> &ExprKind { &self.kind }
}

#[derive(Debug, Clone, PartialEq)]
pub enum StringPart"""
    if needle not in t:
        print("ERROR: needle not found in ast.rs", file=sys.stderr)
        return 1
    t = t.replace(needle, insert, 1)
    t = t.replace(
        "impl fmt::Display for Expr {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        match self {",
        "impl fmt::Display for Expr {\n    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {\n        match &self.kind {",
        1,
    )
    methods = {
        "int", "float", "bool", "string", "ident", "call", "call_with_lambda",
        "lambda", "it_lambda", "binary", "unary", "new", "span", "with_span",
    }
    lines = []
    in_d = False
    for line in t.splitlines():
        if line.startswith("impl fmt::Display for Expr"):
            in_d = True
        if in_d and line.startswith("impl fmt::Display for When"):
            in_d = False
        if in_d and "Expr::" in line:
            m = re.search(r"Expr::([A-Za-z_]+)", line)
            if m and m.group(1) not in methods:
                line = line.replace("Expr::", "ExprKind::")
        lines.append(line)
    t = "\n".join(lines) + "\n"
    start = t.find("// ---- Useful constructors ----")
    if start < 0:
        print("ERROR: constructors section not found", file=sys.stderr)
        return 1
    t = t[:start] + """// ---- Useful constructors ----

impl Expr {
    pub fn int(n: i64) -> Self { ExprKind::Literal(Literal::Int(n)).into() }
    pub fn float(n: f64) -> Self { ExprKind::Literal(Literal::Float(n)).into() }
    pub fn bool(b: bool) -> Self { ExprKind::Literal(Literal::Bool(b)).into() }
    pub fn string(s: &str) -> Self { ExprKind::Literal(Literal::String(s.to_string())).into() }
    #[allow(dead_code)]
    pub fn ident(name: &str) -> Self { ExprKind::Ident(name.to_string()).into() }
    pub fn call(func: Expr, args: Vec<Expr>) -> Self {
        ExprKind::Call { func: Box::new(func), args, trailing_lambda: None }.into()
    }
    #[allow(dead_code)]
    pub fn call_with_lambda(func: Expr, args: Vec<Expr>, lambda: Expr) -> Self {
        ExprKind::Call { func: Box::new(func), args, trailing_lambda: Some(Box::new(lambda)) }.into()
    }
    #[allow(dead_code)]
    pub fn lambda(params: Vec<&str>, body: Expr) -> Self {
        ExprKind::Lambda { params: params.into_iter().map(|s| s.to_string()).collect(), body: Box::new(body), implicit_it: false }.into()
    }
    pub fn it_lambda(body: Expr) -> Self {
        ExprKind::Lambda { params: vec!["it".to_string()], body: Box::new(body), implicit_it: true }.into()
    }
    #[allow(dead_code)]
    pub fn binary(lhs: Expr, op: BinaryOp, rhs: Expr) -> Self {
        ExprKind::Binary(Box::new(lhs), op, Box::new(rhs)).into()
    }
    pub fn unary(op: UnaryOp, expr: Expr) -> Self { ExprKind::Unary(op, Box::new(expr)).into() }
}
"""
    P.write_text(t)
    print("ast.rs migrated")
    return 0


if __name__ == "__main__":
    sys.exit(main())
