#!/usr/bin/env python3
"""Verify compiler.ac uses `import pexpr` (namespace import, M30)."""
from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPILER = ROOT / "bootstrap" / "compiler.ac"
PEXPR = ROOT / "bootstrap" / "pexpr.ac"
FIXTURE = ROOT / "tests" / "fixtures" / "bootstrap" / "pexpr.ac"
MODLOAD = ROOT / "bootstrap" / "modload.ac"
FIXTURE_MODLOAD = ROOT / "tests" / "fixtures" / "bootstrap" / "modload.ac"


def sync_fixture(src: Path, dest: Path) -> None:
    body = src.read_text()
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.is_file() and dest.read_text() == body:
        return
    dest.write_text(body)


def main() -> None:
    sync_fixture(PEXPR, FIXTURE)
    sync_fixture(MODLOAD, FIXTURE_MODLOAD)
    text = COMPILER.read_text()
    lines = [ln.strip() for ln in text.splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if "import pexpr" not in non_comment:
        raise SystemExit(f"{COMPILER}: expected `import pexpr`")
    if any(ln.startswith("import pexpr.{") for ln in non_comment):
        raise SystemExit(f"{COMPILER}: use `import pexpr` with pexpr.fn() calls")
    for fn in ("parseExpr", "parsePrimary", "parseCmpLoop", "skipOptionalSemi"):
        if f"fun {fn}(" in text:
            raise SystemExit(f"{COMPILER}: {fn} should live in pexpr.ac")
    # M31 moved stmt/let into pstmt.ac; keep program/import orchestration here.
    for fn in ("parseProgram", "parseImport"):
        if f"fun {fn}(" not in text:
            raise SystemExit(f"{COMPILER}: {fn} should remain in compiler.ac")
    for fn in ("parseStmt", "parseLet"):
        if f"fun {fn}(" in text:
            raise SystemExit(f"{COMPILER}: {fn} should live in pstmt.ac (M31)")
    pexpr = PEXPR.read_text()
    if "import modload" not in pexpr:
        raise SystemExit(f"{PEXPR}: expected import modload")
    if "import compiler" in pexpr:
        raise SystemExit(f"{PEXPR}: must not import compiler")
    if "fun parseLambdaParamNames(" not in pexpr:
        raise SystemExit(f"{PEXPR}: expected parseLambdaParamNames (M121)")
    if "fun parseCallTrailing(" not in pexpr:
        raise SystemExit(f"{PEXPR}: expected parseCallTrailing (M122)")
    if "fun parseBraceLambda(" not in pexpr:
        raise SystemExit(f"{PEXPR}: expected parseBraceLambda (M122)")
    if "fun parseLambdaBlock(" not in pexpr:
        raise SystemExit(f"{PEXPR}: expected parseLambdaBlock (M123)")
    if "fun parseLambdaBlockStmts(" not in pexpr:
        raise SystemExit(f"{PEXPR}: expected parseLambdaBlockStmts (M125)")
    if "fun parsePlainBlockBody(" not in pexpr:
        raise SystemExit(f"{PEXPR}: expected parsePlainBlockBody (M126)")
    if "fun parsePlainBlockLet(" not in pexpr:
        raise SystemExit(f"{PEXPR}: expected parsePlainBlockLet (M127)")
    if '"pexpr" -> true' not in MODLOAD.read_text():
        raise SystemExit(f"{MODLOAD}: importAllowed must allow pexpr")
    print("=== bootstrap pexpr check OK ===")


if __name__ == "__main__":
    main()
