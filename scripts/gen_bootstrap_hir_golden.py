#!/usr/bin/env python3
"""Compile bootstrap fixture(s) with compiler.ac and write span-stripped HIR golden."""
import json
import subprocess
import sys
from pathlib import Path


def strip_spans(value):
    if isinstance(value, dict):
        return {k: strip_spans(v) for k, v in value.items() if k != "span"}
    if isinstance(value, list):
        return [strip_spans(v) for v in value]
    return value


def action_bin(root: Path) -> Path:
    release = root / "target" / "release" / "action"
    if release.is_file():
        return release
    debug = root / "target" / "debug" / "action"
    if debug.is_file():
        return debug
    raise SystemExit("action binary not found; run cargo build first")


def emit_golden(root: Path, stem: str) -> Path:
    fixture = root / "tests" / "fixtures" / "bootstrap" / f"{stem}.ac"
    if not fixture.is_file():
        raise SystemExit(f"fixture not found: {fixture}")

    compile_input = root / "bootstrap" / "_compile_input.txt"
    compile_input.write_text(fixture.read_text())

    action = action_bin(root)
    compiler_ac = root / "bootstrap" / "compiler.ac"
    proc = subprocess.run(
        [str(action), "run", str(compiler_ac)],
        cwd=root,
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        print(proc.stderr or proc.stdout, file=sys.stderr)
        raise SystemExit(f"bootstrap compiler failed on {stem}.ac (exit {proc.returncode})")

    src = root / "bootstrap" / "_hir_out.json"
    dst = root / "tests" / "fixtures" / "bootstrap" / f"{stem}.bootstrap_hir.json"
    value = json.loads(src.read_text())
    dst.write_text(json.dumps(strip_spans(value), separators=(",", ":")))
    return dst


def all_stems(root: Path) -> list[str]:
    fixture_dir = root / "tests" / "fixtures" / "bootstrap"
    skip = {"env_scope_leak", "prelude", "parser", "emit", "typeenv", "whenty"}  # TC3 negative / flat modules
    return sorted(
        p.stem
        for p in fixture_dir.glob("*.ac")
        if p.stem not in skip
    )


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} <fixture_stem|--all>", file=sys.stderr)
        sys.exit(1)
    arg = sys.argv[1]
    if arg == "--all":
        for stem in all_stems(root):
            dst = emit_golden(root, stem)
            print(dst)
        return
    print(emit_golden(root, arg))


if __name__ == "__main__":
    main()
