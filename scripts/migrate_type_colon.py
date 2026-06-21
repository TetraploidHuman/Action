#!/usr/bin/env python3
"""Migrate space-separated val/var/const type annotations to colon form: val x: Int = 1."""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BINDING = re.compile(
    r"^(\s*)((?:lazy )?(?:val|var|const))\s+(\w+)\s+"
    r"((?:[A-Z]\w*(?:\[[^\]]*\])*\?*))\s*(=)"
)


def migrate_line(line: str) -> str:
    if re.match(r"^\s*(?:lazy )?(?:val|var|const)\s+\w+\s*:", line):
        return line
    m = BINDING.match(line)
    if not m:
        return line
    indent, kw, name, ty, eq = m.groups()
    rest = line[m.end() :]
    return f"{indent}{kw} {name}: {ty} {eq}{rest}"


def migrate_file(path: Path) -> bool:
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)
    out: list[str] = []
    changed = False
    for line in lines:
        nl = "\n" if line.endswith("\n") else ""
        body = line[:-1] if line.endswith("\n") else line
        new_body = migrate_line(body)
        if new_body != body:
            changed = True
        out.append(new_body + nl)
    if changed:
        path.write_text("".join(out), encoding="utf-8")
    return changed


def main() -> int:
    paths = [Path(p) for p in sys.argv[1:]] if len(sys.argv) > 1 else list(ROOT.rglob("*.at"))
    n = 0
    for path in sorted(paths):
        if "_dev" in path.parts or path.suffix != ".at":
            continue
        if migrate_file(path):
            print(path.relative_to(ROOT))
            n += 1
    print(f"migrated {n} files")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
