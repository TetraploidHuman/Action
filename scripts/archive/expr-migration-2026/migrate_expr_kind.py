#!/usr/bin/env python3
"""Bulk-migrate Expr enum variant references to ExprKind."""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
METHODS = {
    "int", "float", "bool", "string", "ident", "call", "call_with_lambda",
    "lambda", "it_lambda", "binary", "unary", "new", "span", "with_span",
    "default", "from",
}
SKIP_FILES = {"migrate_expr_kind.py", "ast.rs"}


def should_replace(line: str, pos: int) -> bool:
    after = line[pos + len("Expr::") :]
    m = re.match(r"([A-Za-z_][A-Za-z0-9_]*)", after)
    if not m:
        return True
    return m.group(1) not in METHODS


def migrate_line(line: str) -> str:
    out, i = [], 0
    while True:
        j = line.find("Expr::", i)
        if j < 0:
            out.append(line[i:])
            break
        out.append(line[i:j])
        out.append("ExprKind::" if should_replace(line, j) else "Expr::")
        i = j + len("Expr::")
    return "".join(out)


def migrate_file(path: Path) -> bool:
    text = path.read_text()
    if "Expr::" not in text:
        return False
    new_text = "\n".join(migrate_line(line) for line in text.splitlines()) + "\n"
    if new_text != text:
        path.write_text(new_text)
        return True
    return False


def main() -> int:
    changed = []
    for path in ROOT.rglob("*.rs"):
        if path.name in SKIP_FILES or "target" in path.parts:
            continue
        if migrate_file(path):
            changed.append(path)
    print(f"Updated {len(changed)} files")
    for p in sorted(changed):
        print(f"  {p.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
