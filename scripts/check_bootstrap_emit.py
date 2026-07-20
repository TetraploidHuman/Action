#!/usr/bin/env python3
"""Verify compiler.ac uses `import emit` (namespace import, M26)."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPILER = ROOT / "bootstrap" / "compiler.ac"
EMIT = ROOT / "bootstrap" / "emit.ac"
FIXTURE_EMIT = ROOT / "tests" / "fixtures" / "bootstrap" / "emit.ac"


def sync_fixture_emit() -> None:
    body = EMIT.read_text()
    if FIXTURE_EMIT.is_file() and FIXTURE_EMIT.read_text() == body:
        return
    FIXTURE_EMIT.write_text(body)


def main() -> None:
    sync_fixture_emit()
    lines = [ln.strip() for ln in COMPILER.read_text().splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if "import emit" not in non_comment:
        raise SystemExit(f"{COMPILER}: expected `import emit`")
    if any(ln.startswith("import emit.{") for ln in non_comment):
        raise SystemExit(f"{COMPILER}: use `import emit` with emit.fn() calls")
    if "fun jOut(" in COMPILER.read_text():
        raise SystemExit(f"{COMPILER}: jOut should live in emit.ac, not compiler.ac")
    emit = EMIT.read_text()
    if "fun stmtsAsBlock(" not in emit:
        raise SystemExit(f"{EMIT}: expected stmtsAsBlock (M125)")
    if "fun slotBlockStmts(" not in emit:
        raise SystemExit(f"{EMIT}: expected slotBlockStmts (M125)")
    if "fun letAsStmt(" not in emit:
        raise SystemExit(f"{EMIT}: expected letAsStmt (M127)")
    if "fun returnAsStmt(" not in emit:
        raise SystemExit(f"{EMIT}: expected returnAsStmt (M129)")
    print("=== bootstrap emit check OK ===")


if __name__ == "__main__":
    main()
