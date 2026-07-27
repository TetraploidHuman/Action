#!/usr/bin/env python3
"""Phase 5: rewrite trailing `{ a, b -> body }` → param-line form.

Keeps `{ it … }` and bare `{ body }` trailing. Only rewrites arrow-param trailings
after `)` or UFCS `.method`.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

ARROW_TRAILING = re.compile(
    r"\{\s*((?:[A-Za-z_][A-Za-z0-9_]*\s*,\s*)*[A-Za-z_][A-Za-z0-9_]*)\s*->\s*"
)


def is_trailing_context(src: str, brace_idx: int) -> bool:
    i = brace_idx - 1
    while i >= 0 and src[i] in " \t\n\r":
        i -= 1
    if i < 0:
        return False
    if src[i] == ")":
        # Exclude `fun name(...) {`
        # Walk back to matching '(' and see if `fun` precedes.
        depth = 0
        j = i
        while j >= 0:
            if src[j] == ")":
                depth += 1
            elif src[j] == "(":
                depth -= 1
                if depth == 0:
                    break
            j -= 1
        k = j - 1
        while k >= 0 and src[k] in " \t\n\r":
            k -= 1
        while k >= 0 and (src[k].isalnum() or src[k] == "_"):
            k -= 1
        while k >= 0 and src[k] in " \t\n\r":
            k -= 1
        if k >= 0 and src[k] == ">":
            # skip `<…>`
            depth = 0
            while k >= 0:
                if src[k] == ">":
                    depth += 1
                elif src[k] == "<":
                    depth -= 1
                    if depth == 0:
                        break
                k -= 1
            k -= 1
            while k >= 0 and src[k] in " \t\n\r":
                k -= 1
        if k >= 2 and src[k - 2 : k + 1] == "fun":
            if k == 2 or not (src[k - 3].isalnum() or src[k - 3] == "_"):
                return False
        return True
    if src[i].isalnum() or src[i] == "_":
        j = i
        while j >= 0 and (src[j].isalnum() or src[j] == "_"):
            j -= 1
        k = j
        while k >= 0 and src[k] in " \t":
            k -= 1
        if k >= 0 and src[k] == ".":
            return True
    return False


def find_matching_brace(src: str, open_idx: int) -> int:
    depth = 0
    i = open_idx
    while i < len(src):
        c = src[i]
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return i
        elif c == '"':
            i += 1
            while i < len(src) and src[i] != '"':
                if src[i] == "\\":
                    i += 2
                    continue
                i += 1
        elif c == "/" and i + 1 < len(src) and src[i + 1] == "/":
            while i < len(src) and src[i] != "\n":
                i += 1
            continue
        i += 1
    raise ValueError(f"unmatched brace at {open_idx}")


def migrate(src: str) -> str:
    out: list[str] = []
    i = 0
    n = len(src)
    while i < n:
        if src[i] == "/" and i + 1 < n and src[i + 1] == "/":
            while i < n and src[i] != "\n":
                out.append(src[i])
                i += 1
            continue
        if src[i] == '"':
            out.append(src[i])
            i += 1
            while i < n and src[i] != '"':
                out.append(src[i])
                if src[i] == "\\":
                    i += 1
                    if i < n:
                        out.append(src[i])
                        i += 1
                    continue
                i += 1
            if i < n:
                out.append(src[i])
                i += 1
            continue

        if src[i] != "{":
            out.append(src[i])
            i += 1
            continue

        if not is_trailing_context(src, i):
            out.append("{")
            i += 1
            continue

        m = ARROW_TRAILING.match(src, i)
        if not m:
            end = find_matching_brace(src, i)
            out.append(src[i : end + 1])
            i = end + 1
            continue

        params = re.sub(r"\s+", " ", m.group(1).strip())
        params = re.sub(r"\s*,\s*", ", ", params)
        end = find_matching_brace(src, i)
        body = src[m.end() : end].strip()
        # Prefer `;` same-line for short bodies; newline for multi-line.
        if "\n" in body or len(body) > 40:
            out.append(f"{{ {params}\n    {body}\n}}")
        else:
            out.append(f"{{ {params}; {body} }}")
        i = end + 1

    return "".join(out)


def main() -> int:
    paths = sorted(ROOT.glob("examples/**/*.ac"))
    # Path B trailing fixtures are `{ it }` only — skip unless they gain `->`.
    paths += sorted(ROOT.glob("tests/fixtures/**/*.ac"))
    changed = 0
    for path in paths:
        old = path.read_text(encoding="utf-8")
        if "->" not in old:
            continue
        new = migrate(old)
        if new != old:
            path.write_text(new, encoding="utf-8")
            changed += 1
            print(f"updated {path.relative_to(ROOT)}")
    print(f"done: {changed} files changed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
