#!/usr/bin/env python3
"""Verify compiler.ac uses `import typeenv` (namespace import, M27)."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPILER = ROOT / "bootstrap" / "compiler.ac"
TYPEENV = ROOT / "bootstrap" / "typeenv.ac"
FIXTURE = ROOT / "tests" / "fixtures" / "bootstrap" / "typeenv.ac"


def sync_fixture() -> None:
    body = TYPEENV.read_text()
    if FIXTURE.is_file() and FIXTURE.read_text() == body:
        return
    FIXTURE.write_text(body)


def main() -> None:
    sync_fixture()
    text = COMPILER.read_text()
    lines = [ln.strip() for ln in text.splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if "import typeenv" not in non_comment and "import typeenv.{" not in non_comment:
        raise SystemExit(f"{COMPILER}: expected `import typeenv` or selective import")
    if any(ln.startswith("import typeenv.{") for ln in non_comment):
        raise SystemExit(f"{COMPILER}: use `import typeenv` with typeenv.fn() calls")
    for fn in ("lookupTag", "tyAnnTag", "envClear", "typeErrorMark"):
        if f"fun {fn}(" in text:
            raise SystemExit(f"{COMPILER}: {fn} should live in typeenv.ac")
    print("=== bootstrap typeenv check OK ===")


if __name__ == "__main__":
    main()
