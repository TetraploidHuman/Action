#!/usr/bin/env python3
"""Verify compiler.ac uses `import modload` (namespace import, M29)."""
from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
COMPILER = ROOT / "bootstrap" / "compiler.ac"
MODLOAD = ROOT / "bootstrap" / "modload.ac"
FIXTURE = ROOT / "tests" / "fixtures" / "bootstrap" / "modload.ac"


def sync_fixture() -> None:
    body = MODLOAD.read_text()
    if FIXTURE.is_file() and FIXTURE.read_text() == body:
        return
    FIXTURE.parent.mkdir(parents=True, exist_ok=True)
    FIXTURE.write_text(body)


def main() -> None:
    sync_fixture()
    text = COMPILER.read_text()
    lines = [ln.strip() for ln in text.splitlines()]
    non_comment = [ln for ln in lines if ln and not ln.startswith("//")]
    if "import modload" not in non_comment:
        raise SystemExit(f"{COMPILER}: expected `import modload`")
    if any(ln.startswith("import modload.{") for ln in non_comment):
        raise SystemExit(f"{COMPILER}: use `import modload` with modload.fn() calls")
    for fn in ("importClear", "importIsLoaded", "importAllowed", "parseImportSkipItems"):
        if f"fun {fn}(" in text:
            raise SystemExit(f"{COMPILER}: {fn} should live in modload.ac")
    for fn in ("parseImport", "parseImportLoadModule"):
        if f"fun {fn}(" not in text:
            raise SystemExit(f"{COMPILER}: {fn} should remain in compiler.ac")
    # M33: preScanImport lives in pscan.ac (import orchestration without parseProgram cycle).
    pscan = ROOT / "bootstrap" / "pscan.ac"
    if f"fun preScanImport(" not in pscan.read_text():
        raise SystemExit(f"{pscan}: preScanImport should live in pscan.ac")
    if f"fun preScanImport(" in text:
        raise SystemExit(f"{COMPILER}: preScanImport should live in pscan.ac (M33)")
    mod = MODLOAD.read_text()
    if "import typeenv" not in mod:
        raise SystemExit(f"{MODLOAD}: expected import typeenv")
    if '"modload" -> true' not in mod:
        raise SystemExit(f"{MODLOAD}: importAllowed must allow modload")
    print("=== bootstrap modload check OK ===")


if __name__ == "__main__":
    main()
