#!/usr/bin/env python3
"""Verify compiler.ac uses `import whenty` (namespace import, M27)."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPILER = ROOT / "bootstrap" / "compiler.ac"
WHENTY = ROOT / "bootstrap" / "whenty.ac"
FIXTURE = ROOT / "tests" / "fixtures" / "bootstrap" / "whenty.ac"


def sync_fixture() -> None:
    body = WHENTY.read_text()
    if FIXTURE.is_file() and FIXTURE.read_text() == body:
        return
    FIXTURE.write_text(body)


def main() -> None:
    sync_fixture()
    text = COMPILER.read_text()
    lines = [ln.strip() for ln in text.splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if "import whenty" not in non_comment and "import whenty.{" not in non_comment:
        raise SystemExit(f"{COMPILER}: expected `import whenty` or selective import")
    if "import when" in non_comment:
        raise SystemExit(f"{COMPILER}: `when` is a keyword; use `import whenty`")
    for fn in ("whenChainTagSnap", "patJson", "whenTagUnify"):
        if f"fun {fn}(" in text:
            raise SystemExit(f"{COMPILER}: {fn} should live in whenty.ac")
    whenty_text = WHENTY.read_text()
    if "import typeenv" not in whenty_text:
        raise SystemExit(f"{WHENTY}: expected `import typeenv`")
    print("=== bootstrap whenty check OK ===")


if __name__ == "__main__":
    main()
