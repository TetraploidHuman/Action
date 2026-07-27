#!/usr/bin/env python3
"""Verify compiler.ac uses `import pdecl` (namespace import, M32)."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPILER = ROOT / "bootstrap" / "compiler.ac"
PDECL = ROOT / "bootstrap" / "pdecl.ac"
FIXTURE = ROOT / "tests" / "fixtures" / "bootstrap" / "pdecl.ac"
MODLOAD = ROOT / "bootstrap" / "modload.ac"
FIXTURE_MODLOAD = ROOT / "tests" / "fixtures" / "bootstrap" / "modload.ac"

FUN_START = re.compile(r"^fun\s+(\w+)\s*\(")
EXPECTED_FUNS = {
    "advanceTypeFieldSep",
    "parseEnumVars",
    "parseEnumVar",
    "parseEnum",
    "parseTypeFields",
    "parseTypeField",
    "parseTypeAlias",
    "parseTypeAliasAfterEq",
    "parseTypeAliasLegacyReject",
    "parseTypeAliasNamed",
    "parseTypeAliasStruct",
    "parseBlockRet",
    "parseFunParam",
    "parseFunParams",
    "parseFun",
    "skipExternalParams",
    "skipExternalParam",
    "parseExternal",
}


def sync_fixture(src: Path, dest: Path) -> None:
    body = src.read_text()
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.is_file() and dest.read_text() == body:
        return
    dest.write_text(body)


def main() -> None:
    sync_fixture(PDECL, FIXTURE)
    sync_fixture(MODLOAD, FIXTURE_MODLOAD)
    text = COMPILER.read_text()
    lines = [ln.strip() for ln in text.splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if "import pdecl" not in non_comment:
        raise SystemExit(f"{COMPILER}: expected `import pdecl`")
    if any(ln.startswith("import pdecl.{") for ln in non_comment):
        raise SystemExit(f"{COMPILER}: use `import pdecl` with pdecl.fn() calls")
    for fn in ("parseFun", "parseEnum", "parseTypeAlias", "parseExternal", "parseBlockRet"):
        if f"fun {fn}(" in text:
            raise SystemExit(f"{COMPILER}: {fn} should live in pdecl.ac")
    for fn in ("parseProgram", "parseImport", "parseTopLevel"):
        if f"fun {fn}(" not in text:
            raise SystemExit(f"{COMPILER}: {fn} should remain in compiler.ac")
    # M33 moved preScanProgram into pscan.ac.
    if "fun preScanProgram(" in text:
        raise SystemExit(f"{COMPILER}: preScanProgram should live in pscan.ac (M33)")
    pdecl = PDECL.read_text()
    if "import pstmt" not in pdecl:
        raise SystemExit(f"{PDECL}: expected import pstmt")
    if "import modload" not in pdecl:
        raise SystemExit(f"{PDECL}: expected import modload")
    # Host check of pdecl alone needs namespaces used by imported modules.
    if "import whenty" not in pdecl:
        raise SystemExit(f"{PDECL}: expected import whenty (for host check via pstmt/pexpr)")
    if "import pexpr" not in pdecl:
        raise SystemExit(f"{PDECL}: expected import pexpr (for host check via pstmt)")
    if "import compiler" in pdecl:
        raise SystemExit(f"{PDECL}: must not import compiler")
    names = [m.group(1) for ln in pdecl.splitlines() if (m := FUN_START.match(ln.strip()))]
    if len(names) != len(set(names)):
        raise SystemExit(f"{PDECL}: duplicate fun definitions: {names}")
    if set(names) != EXPECTED_FUNS:
        raise SystemExit(
            f"{PDECL}: fun set mismatch\n  got={sorted(names)}\n  want={sorted(EXPECTED_FUNS)}"
        )
    if '"pdecl" -> true' not in MODLOAD.read_text():
        raise SystemExit(f"{MODLOAD}: importAllowed must allow pdecl")
    print("=== bootstrap pdecl check OK ===")


if __name__ == "__main__":
    main()
