#!/usr/bin/env python3
"""Fix match/if-let on Expr to use .kind field."""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXPR_NAMES = {
    "expr", "e", "left", "right", "func", "body", "value", "target", "nullable",
    "fallback", "inner", "lhs", "rhs", "receiver", "arg", "lambda", "condition",
    "then_expr", "else_expr", "iterable", "start", "end", "obj", "idx", "w", "f",
    "func_expr", "recv_val", "nullable_expr", "fallback_expr", "module_expr",
    "inner_ast", "body_expr", "fn_expr", "fn_val", "recv", "elem", "item",
    "map_expr", "set_expr", "list_expr", "call_expr", "field_expr", "index_expr",
    "range_start", "range_end", "pat_expr", "guard", "arm_body", "when_expr",
    "for_expr", "block_expr", "struct_expr", "map_lit_expr", "set_lit_expr",
    "tuple_expr", "assign_target", "assign_value", "copy_expr", "unsafe_expr",
    "string_expr", "interp_expr", "ref_expr", "callee", "method_recv", "first",
}


def fix_match_kind(text: str) -> str:
    def repl_match(m: re.Match) -> str:
        var, chain = m.group(1), m.group(2) or ""
        if var not in EXPR_NAMES:
            return m.group(0)
        if chain == ".as_ref()":
            return f"match {var}.as_ref().kind {{"
        return f"match &{var}.kind {{"

    text = re.sub(
        r"match\s+([a-z_][a-z0-9_]*)(\.as_ref\(\))?\s*\{",
        repl_match,
        text,
    )

    def repl_if_let(m: re.Match) -> str:
        var, chain, rest = m.group(2), m.group(3) or "", m.group(1)
        if var not in EXPR_NAMES:
            return m.group(0)
        if chain == ".as_ref()":
            return f"if let {rest} = {var}.as_ref().kind {{"
        return f"if let {rest} = &{var}.kind {{"

    text = re.sub(
        r"if let (ExprKind::[^{=]+)\s*=\s*([a-z_][a-z0-9_]*)(\.as_ref\(\))?",
        repl_if_let,
        text,
    )

    def repl_matches(m: re.Match) -> str:
        var, chain, rest = m.group(1), m.group(2) or "", m.group(3)
        if var not in EXPR_NAMES:
            return m.group(0)
        if chain == ".as_ref()":
            return f"matches!({var}.as_ref().kind, {rest}"
        return f"matches!(&{var}.kind, {rest}"

    text = re.sub(
        r"matches!\(\s*([a-z_][a-z0-9_]*)(\.as_ref\(\))?,\s*(ExprKind::)",
        repl_matches,
        text,
    )
    return text


def process_file(path: Path) -> bool:
    if path.name == "ast.rs" and "action-frontend" in path.parts:
        return False
    text = path.read_text()
    new = fix_match_kind(text)
    if new != text:
        path.write_text(new)
        return True
    return False


def main() -> int:
    changed = []
    for path in ROOT.rglob("*.rs"):
        if "target" in path.parts:
            continue
        if process_file(path):
            changed.append(path)
    print(f"Fixed {len(changed)} files")
    for p in sorted(changed):
        print(f"  {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
