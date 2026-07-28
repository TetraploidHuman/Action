#!/usr/bin/env python3
"""Second-pass: migrate leftover when-arm `->` including `Some(v) -> expr`."""
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


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


def arrow_only_in_strings(body: str) -> bool:
    return "->" not in re.sub(r'"([^"\\]|\\.)*"', '""', body)


def is_function_type_arrow(left: str, right: str) -> bool:
    """True for `(Int) -> Int` / `) -> Int,` inside signatures — not `Some(v) -> expr`."""
    if not left.rstrip().endswith(")"):
        return False
    rs = right.strip()
    # Function type continuation: type then immediately `,` or `)` or end / `{` for fun body after return
    # Distinguish Some(v)->print(v): right starts with call/expr, not a lone type before comma/paren.
    # Param function type: `) -> Int,` or `) -> Int)` or `) -> (Int) -> Int`
    m = re.match(
        r"^((?:[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]]*\])?|\([^;]*?\))(?:\s*->\s*(?:[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]]*\])?|\([^;]*?\)))*)\s*([,)].*)$",
        rs,
    )
    if m:
        return True
    # Return type on fun line: `) -> Int {` or `) -> Int =`
    if re.match(
        r"^[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]]*\])?\s*[\{\=]",
        rs,
    ):
        # Could be return type — but those should already be migrated. If still present, leave.
        # When arm `) -> {` wouldn't match `[\{\=]` after type only... `Some(v) -> {` has `{` immediately.
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]]*\])?\s*[\{\=]", rs):
            # `Int {` as when body starting with type name? rare. Treat as function return if fun on line.
            return True
    return False


def migrate_line(body: str) -> str:
    if "->" not in body or arrow_only_in_strings(body):
        return body
    if re.search(
        r"\{\s*[A-Za-z_][A-Za-z0-9_]*(?:\s*,\s*[A-Za-z_][A-Za-z0-9_]*)*\s*->",
        body,
    ):
        return body  # legacy brace lambda
    if re.search(r"=\s*\([^)]*\)\s*->", body) and "when" not in body:
        return body  # type alias function type

    def repl(m: re.Match[str]) -> str:
        left = m.group(1)
        right = m.group(2)
        if is_function_type_arrow(left, right):
            return m.group(0)
        rs = right.strip()
        semi = ""
        if rs.endswith(";"):
            rs = rs[:-1].rstrip()
            semi = ";"
        if rs.startswith("{") and rs.endswith("}"):
            return f"{left.rstrip()} {rs}{semi}"
        return f"{left.rstrip()} {{ {rs} }}{semi}"

    return re.sub(r"^(.*?)(?<![=-])->\s*(.+)$", repl, body)


def main() -> None:
    paths: list[Path] = []
    for base in ["examples", "tests/fixtures", "bootstrap", "lib"]:
        p = ROOT / base
        if p.exists():
            paths.extend(sorted(p.rglob("*.ac")))
    n = 0
    for path in paths:
        text = path.read_text(encoding="utf-8")
        out_lines = []
        changed = False
        for line in text.splitlines(keepends=True):
            body, comment = strip_line_comment(line)
            new_body = migrate_line(body)
            if new_body != body:
                changed = True
            out_lines.append(new_body + comment)
        if changed:
            path.write_text("".join(out_lines), encoding="utf-8")
            n += 1
            print(f"updated {path.relative_to(ROOT)}")
    print(f"done: {n} files")


if __name__ == "__main__":
    main()
