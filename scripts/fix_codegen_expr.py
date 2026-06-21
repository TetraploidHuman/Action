#!/usr/bin/env python3
"""Fix action-codegen for Expr { kind, span } without over-matching."""

from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent / "crates/action-codegen/src"

EXPR_MATCH_VARS = {"expr", "func", "lhs", "rhs", "receiver", "target", "nullable", "fallback", "inner", "body", "module_expr"}


def fix_file(path: Path) -> bool:
    lines = path.read_text().splitlines()
    out = []
    changed = False
    for line in lines:
        new = line
        for var in EXPR_MATCH_VARS:
            old = f"match {var} {{"
            if old in new and f"match &{var}.kind {{" not in new:
                new = new.replace(old, f"match &{var}.kind {{")
                changed = True
        reps = [
            ("if let ExprKind::Ident(name) = func.as_ref()", "if let ExprKind::Ident(name) = &func.kind"),
            ("if let ExprKind::Ident(fn_name) = func.as_ref()", "if let ExprKind::Ident(fn_name) = &func.kind"),
            ("if let ExprKind::FieldAccess(receiver, method) = func.as_ref()", "if let ExprKind::FieldAccess(receiver, method) = &func.kind"),
            ("if let ExprKind::Ident(module_name) = module_expr.as_ref()", "if let ExprKind::Ident(module_name) = &module_expr.kind"),
            ("if let ExprKind::Ident(name) = target.as_ref()", "if let ExprKind::Ident(name) = &target.kind"),
            ("} else if let ExprKind::FieldAccess(receiver, method) = func.as_ref()", "} else if let ExprKind::FieldAccess(receiver, method) = &func.kind"),
            ("matches!(&args[i], ExprKind::Lambda", "matches!(&args[i].kind, ExprKind::Lambda"),
            ("matches!(arg, ExprKind::Lambda", "matches!(&arg.kind, ExprKind::Lambda"),
            ("= ExprKind::Ident(", "= Expr::ident("),
        ]
        for a, b in reps:
            if a in new:
                new = new.replace(a, b)
                changed = True
        out.append(new)
    if changed:
        path.write_text("\n".join(out) + "\n")
    return changed


def main() -> None:
    n = 0
    for path in ROOT.rglob("*.rs"):
        if fix_file(path):
            n += 1
            print(path.relative_to(ROOT))
    print(f"fixed {n} files")


if __name__ == "__main__":
    main()
