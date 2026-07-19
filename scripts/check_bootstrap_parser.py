#!/usr/bin/env python3
"""Verify compiler.ac uses `import parser` (namespace import, M25)."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPILER = ROOT / "bootstrap" / "compiler.ac"


def main() -> None:
    lines = [ln.strip() for ln in COMPILER.read_text().splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if "import parser" not in non_comment:
        raise SystemExit(f"{COMPILER}: expected `import parser`")
    if any(ln.startswith("import parser.{") for ln in non_comment):
        raise SystemExit(f"{COMPILER}: use `import parser` with parser.fn() calls")
    print("=== bootstrap parser check OK ===")


if __name__ == "__main__":
    main()
