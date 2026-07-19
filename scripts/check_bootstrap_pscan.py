#!/usr/bin/env python3
"""Verify compiler.ac uses `import pscan` (namespace import, M33)."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPILER = ROOT / "bootstrap" / "compiler.ac"
PSCAN = ROOT / "bootstrap" / "pscan.ac"
FIXTURE = ROOT / "tests" / "fixtures" / "bootstrap" / "pscan.ac"
MODLOAD = ROOT / "bootstrap" / "modload.ac"
FIXTURE_MODLOAD = ROOT / "tests" / "fixtures" / "bootstrap" / "modload.ac"

FUN_START = re.compile(r"^fun\s+(\w+)\s*\(")
EXPECTED_FUNS = {
    "skipBraceDepthDelta",
    "skipBracePack",
    "skipBraceUnpackPos",
    "skipBraceUnpackDepth",
    "skipBraceDepthStep",
    "skipBraceDepth",
    "skipBalancedBlock",
    "preScanFunParams",
    "preScanFunParamRest",
    "preScanFun",
    "preScanExternal",
    "preScanSkipUnknown",
    "preScanEnum",
    "preScanEnumVars",
    "preScanEnumDone",
    "preScanEnumVarOne",
    "preScanTypeAlias",
    "preScanImportLoadModule",
    "preScanImportLoad",
    "preScanImport",
    "preScanTopLevel",
    "preScanProgram",
}


def sync_fixture(src: Path, dest: Path) -> None:
    body = src.read_text()
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.is_file() and dest.read_text() == body:
        return
    dest.write_text(body)


def main() -> None:
    sync_fixture(PSCAN, FIXTURE)
    sync_fixture(MODLOAD, FIXTURE_MODLOAD)
    text = COMPILER.read_text()
    lines = [ln.strip() for ln in text.splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if "import pscan" not in non_comment:
        raise SystemExit(f"{COMPILER}: expected `import pscan`")
    if any(ln.startswith("import pscan.{") for ln in non_comment):
        raise SystemExit(f"{COMPILER}: use `import pscan` with pscan.fn() calls")
    for fn in ("preScanProgram", "preScanFun", "skipBalancedBlock", "preScanImport"):
        if f"fun {fn}(" in text:
            raise SystemExit(f"{COMPILER}: {fn} should live in pscan.ac")
    for fn in ("parseProgram", "parseImport", "parseTopLevel", "sessionReset", "main"):
        if f"fun {fn}(" not in text:
            raise SystemExit(f"{COMPILER}: {fn} should remain in compiler.ac")
    pscan = PSCAN.read_text()
    if "import pdecl" not in pscan:
        raise SystemExit(f"{PSCAN}: expected import pdecl")
    if "import modload" not in pscan:
        raise SystemExit(f"{PSCAN}: expected import modload")
    if "import compiler" in pscan:
        raise SystemExit(f"{PSCAN}: must not import compiler")
    names = [m.group(1) for ln in pscan.splitlines() if (m := FUN_START.match(ln.strip()))]
    if len(names) != len(set(names)):
        raise SystemExit(f"{PSCAN}: duplicate fun definitions: {names}")
    if set(names) != EXPECTED_FUNS:
        raise SystemExit(
            f"{PSCAN}: fun set mismatch\n  got={sorted(names)}\n  want={sorted(EXPECTED_FUNS)}"
        )
    if '"pscan" -> true' not in MODLOAD.read_text():
        raise SystemExit(f"{MODLOAD}: importAllowed must allow pscan")
    print("=== bootstrap pscan check OK ===")


if __name__ == "__main__":
    main()
