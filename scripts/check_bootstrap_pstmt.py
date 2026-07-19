#!/usr/bin/env python3
"""Verify compiler.ac uses `import pstmt` (namespace import, M31)."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPILER = ROOT / "bootstrap" / "compiler.ac"
PSTMT = ROOT / "bootstrap" / "pstmt.ac"
FIXTURE = ROOT / "tests" / "fixtures" / "bootstrap" / "pstmt.ac"
MODLOAD = ROOT / "bootstrap" / "modload.ac"
FIXTURE_MODLOAD = ROOT / "tests" / "fixtures" / "bootstrap" / "modload.ac"

FUN_START = re.compile(r"^fun\s+(\w+)\s*\(")
EXPECTED_FUNS = {
    "emitLetHdr",
    "emitLetMut",
    "finishLet",
    "parseLet",
    "emitBlockTy",
    "parseBlock",
    "parseReturn",
    "forBodyUsesLenAfterLen",
    "forBodyUsesLenScan",
    "forBodyUsesLen",
    "forInBindTag",
    "forWithIndexBindMap",
    "forWithIndexBindList",
    "forWithIndexBind",
    "parseForCond",
    "parseForIn",
    "parseForInfinite",
    "parseForWithIndex",
    "parseForWithVar",
    "parseFor",
    "parseBuiltinPrint",
    "parseExprStmt",
    "isAssignStmt",
    "parseAssignStmt",
    "parseStmtFallback",
    "parseStmt",
    "parseStmts",
}


def sync_fixture(src: Path, dest: Path) -> None:
    body = src.read_text()
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.is_file() and dest.read_text() == body:
        return
    dest.write_text(body)


def main() -> None:
    sync_fixture(PSTMT, FIXTURE)
    sync_fixture(MODLOAD, FIXTURE_MODLOAD)
    text = COMPILER.read_text()
    lines = [ln.strip() for ln in text.splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if "import pstmt" not in non_comment:
        raise SystemExit(f"{COMPILER}: expected `import pstmt`")
    if any(ln.startswith("import pstmt.{") for ln in non_comment):
        raise SystemExit(f"{COMPILER}: use `import pstmt` with pstmt.fn() calls")
    for fn in ("parseStmt", "parseStmts", "parseBlock", "parseLet", "parseFor"):
        if f"fun {fn}(" in text:
            raise SystemExit(f"{COMPILER}: {fn} should live in pstmt.ac")
    for fn in ("parseProgram", "parseImport", "parseTopLevel"):
        if f"fun {fn}(" not in text:
            raise SystemExit(f"{COMPILER}: {fn} should remain in compiler.ac")
    pstmt = PSTMT.read_text()
    if "import pexpr" not in pstmt:
        raise SystemExit(f"{PSTMT}: expected import pexpr")
    # Host check of pstmt alone needs modload visible (pexpr uses modload.*).
    if "import modload" not in pstmt:
        raise SystemExit(f"{PSTMT}: expected import modload (for host check via pexpr)")
    if "import compiler" in pstmt:
        raise SystemExit(f"{PSTMT}: must not import compiler")
    names = [m.group(1) for ln in pstmt.splitlines() if (m := FUN_START.match(ln.strip()))]
    if len(names) != len(set(names)):
        raise SystemExit(f"{PSTMT}: duplicate fun definitions: {names}")
    if set(names) != EXPECTED_FUNS:
        raise SystemExit(f"{PSTMT}: fun set mismatch\n  got={sorted(names)}\n  want={sorted(EXPECTED_FUNS)}")
    if '"pstmt" -> true' not in MODLOAD.read_text():
        raise SystemExit(f"{MODLOAD}: importAllowed must allow pstmt")
    print("=== bootstrap pstmt check OK ===")


if __name__ == "__main__":
    main()
