#!/usr/bin/env python3
"""Migrate Action sources:

  val a: Int = 0              → val a Int = 0
  fun f(x: Int) -> Int { }    → fun f(x Int) Int { }
  when x { Red -> 1 }         → when x { Red { 1 } }

Keeps function types `(Int) -> Bool` and map/tuple `k: v` colons.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

TYPE_START = r"(?:[A-Z][A-Za-z0-9_]*|List|Map|Set|Task|LazyList|Ptr|Stream|CString|FileHandle|fun)\b"


def strip_line_comment(line: str) -> tuple[str, str]:
    in_str = False
    i = 0
    while i < len(line):
        c = line[i]
        if c == '"' and (i == 0 or line[i - 1] != "\\"):
            in_str = not in_str
        elif not in_str and c == "/" and i + 1 < len(line) and line[i + 1] == "/":
            return line[:i], line[i:]
        i += 1
    return line, ""


def migrate_binding_colons(body: str) -> str:
    body = re.sub(
        r"\b(lazy\s+val|val|var|const)\s+([A-Za-z_][A-Za-z0-9_]*)\s*:\s*",
        r"\1 \2 ",
        body,
    )
    return body


def migrate_typed_name_colons(body: str) -> str:
    """`name: Type` / `self: Type` where Type looks like a type (PascalCase / builtins / fun / ()."""
    # self: Type
    body = re.sub(rf"\bself\s*:\s*(?={TYPE_START}|\()", "self ", body)
    # general name: Type — skip string keys ("x":) already excluded by \b before name
    # Avoid transforming after digits or quotes via word boundary on name.
    body = re.sub(
        rf"\b([A-Za-z_][A-Za-z0-9_]*)\s*:\s*(?={TYPE_START}|\()",
        r"\1 ",
        body,
    )
    return body


def migrate_fun_return_arrows(body: str) -> str:
    if not re.search(r"\b(fun|external)\b", body):
        return body
    # Only rewrite the return-type arrow: the `) ->` whose closing paren
    # matches the function parameter list (paren depth returns to 0), not
    # arrows inside function-typed parameters like `f: (Int) -> Int`.
    depth = 0
    i = 0
    last_top_close = None
    while i < len(body):
        c = body[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                last_top_close = i
        i += 1
    if last_top_close is None:
        return body
    m = re.match(r"\)\s*->\s*", body[last_top_close:])
    if not m:
        return body
    start = last_top_close
    end = last_top_close + m.end()
    return body[:start] + ") " + body[end:]


def migrate_when_arrows(body: str) -> str:
    if "->" not in body:
        return body
    # Skip function types / fun returns
    if re.search(r"\)\s*->", body):
        return body
    if re.search(r"\bfun\b", body) and "->" in body:
        return body
    # Skip legacy brace-lambda `{ a, b ->`
    if re.search(
        r"\{\s*[A-Za-z_][A-Za-z0-9_]*(?:\s*,\s*[A-Za-z_][A-Za-z0-9_]*)*\s*->",
        body,
    ):
        return body

    def repl(m: re.Match[str]) -> str:
        left = m.group(1).rstrip()
        right = m.group(2).strip()
        semi = ""
        if right.endswith(";"):
            right = right[:-1].rstrip()
            semi = ";"
        if right.startswith("{") and right.endswith("}"):
            return f"{left} {right}{semi}"
        return f"{left} {{ {right} }}{semi}"

    return re.sub(r"^(.*?)(?<![=-])->\s*(.+)$", repl, body)


def migrate_text(code: str) -> str:
    out = []
    for line in code.splitlines(keepends=True):
        body, comment = strip_line_comment(line)
        body = migrate_binding_colons(body)
        body = migrate_typed_name_colons(body)
        body = migrate_fun_return_arrows(body)
        body = migrate_when_arrows(body)
        out.append(body + comment)
    return "".join(out)


def main() -> int:
    paths: list[Path] = []
    for base in [
        ROOT / "examples",
        ROOT / "tests" / "fixtures",
        ROOT / "bootstrap",
        ROOT / "lib",
    ]:
        if base.exists():
            paths.extend(sorted(base.rglob("*.ac")))

    changed = 0
    for p in paths:
        text = p.read_text(encoding="utf-8")
        new = migrate_text(text)
        if new != text:
            p.write_text(new, encoding="utf-8")
            changed += 1
            print(f"updated {p.relative_to(ROOT)}")
    print(f"done: {changed} files changed of {len(paths)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
