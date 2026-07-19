#!/usr/bin/env python3
"""Verify bootstrap modules use namespace imports (import prelude / import parser)."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PRELUDE = ROOT / "bootstrap" / "prelude.ac"
FIXTURE_PRELUDE = ROOT / "tests" / "fixtures" / "bootstrap" / "prelude.ac"
LEXER = ROOT / "bootstrap" / "lexer.ac"
COMPILER = ROOT / "bootstrap" / "compiler.ac"
PARSER = ROOT / "bootstrap" / "parser.ac"
FIXTURE_PARSER = ROOT / "tests" / "fixtures" / "bootstrap" / "parser.ac"


def prelude_is_standalone() -> None:
    text = PRELUDE.read_text()
    if "import " in text:
        raise SystemExit("bootstrap/prelude.ac must not import other modules")
    if "fun keywordKindOpsTail" in text:
        raise SystemExit("bootstrap/prelude.ac must not define keywordKindOpsTail (host hook)")


def sync_fixture_prelude() -> None:
    body = PRELUDE.read_text()
    if FIXTURE_PRELUDE.is_file() and FIXTURE_PRELUDE.read_text() == body:
        return
    FIXTURE_PRELUDE.write_text(body)


def sync_fixture_parser() -> None:
    body = PARSER.read_text()
    if FIXTURE_PARSER.is_file() and FIXTURE_PARSER.read_text() == body:
        return
    FIXTURE_PARSER.write_text(body)


def check_import_line(path: Path, want: str) -> None:
    lines = [ln.strip() for ln in path.read_text().splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if want not in non_comment:
        raise SystemExit(f"{path}: expected `{want}`")
    if any(ln.startswith("import prelude.{") or ln.startswith("import parser.{") for ln in non_comment):
        raise SystemExit(f"{path}: selective import list is deprecated; use `{want}` + namespace calls")


def main() -> None:
    prelude_is_standalone()
    sync_fixture_prelude()
    sync_fixture_parser()
    check_import_line(LEXER, "import prelude")
    check_import_line(COMPILER, "import prelude")
    check_import_line(COMPILER, "import parser")
    check_import_line(PARSER, "import prelude")
    check_import_line(COMPILER, "import emit")
    print("=== bootstrap prelude check OK ===")


if __name__ == "__main__":
    main()
