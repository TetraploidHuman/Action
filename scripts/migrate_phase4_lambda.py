#!/usr/bin/env python3
"""Migrate non-trailing brace lambdas to `lambda` keyword (Phase 4).

Keeps trailing forms: `f(...) { … }` and `recv.method { … }`.
Does not treat `fun name(...) {` as trailing.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

FILES = [
    "examples/lambda.ac",
    "examples/fn_type2.ac",
    "examples/bench_lambda.ac",
    "examples/lazy_drop_test.ac",
    "examples/lazy_filter_test.ac",
    "examples/tutorial.ac",
    "examples/test_pat_cb.ac",
    "examples/test_lambda_capture.ac",
    "examples/test_closure_loop.ac",
    "examples/rc_pressure_test.ac",
    "examples/rc_cycle_test.ac",
    "examples/test_simple_cb.ac",
    "examples/test_cb4.ac",
    "examples/test_cb2.ac",
    "examples/test_cb5.ac",
    "examples/test_higher_order.ac",
    "examples/test_nested_closure.ac",
    "tests/fixtures/bootstrap/lambda_it_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_lambda_it_ty.ac",
    "tests/fixtures/bootstrap/plain_block_lambda_it_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_plain_block_lambda_it_ty.ac",
    "tests/fixtures/bootstrap/lambda_multi_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_lambda_multi_ty.ac",
    "tests/fixtures/bootstrap/plain_block_lambda_multi_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_plain_block_lambda_multi_ty.ac",
    "tests/fixtures/bootstrap/lambda_block_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_lambda_block_ty.ac",
    "tests/fixtures/bootstrap/plain_block_lambda_block_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_plain_block_lambda_block_ty.ac",
    "tests/fixtures/bootstrap/lambda_stmts_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_lambda_stmts_ty.ac",
    "tests/fixtures/bootstrap/plain_block_lambda_stmts_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_plain_block_lambda_stmts_ty.ac",
    "tests/fixtures/bootstrap/lambda_val_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_lambda_val_ty.ac",
    "tests/fixtures/bootstrap/plain_block_lambda_val_ok.ac",
    "tests/fixtures/bootstrap_forbidden/bad_plain_block_lambda_val_ty.ac",
]

ARROW_HEAD = re.compile(
    r"\{\s*((?:[A-Za-z_][A-Za-z0-9_]*\s*,\s*)*[A-Za-z_][A-Za-z0-9_]*)\s*->"
)
IT_HEAD = re.compile(r"\{\s*it\b")


def skip_ws_back(src: str, i: int) -> int:
    while i >= 0 and src[i] in " \t\n\r":
        i -= 1
    return i


def match_paren_open(src: str, close_idx: int) -> int | None:
    depth = 0
    j = close_idx
    while j >= 0:
        c = src[j]
        if c == ")":
            depth += 1
        elif c == "(":
            depth -= 1
            if depth == 0:
                return j
        j -= 1
    return None


def is_fun_param_list_close(src: str, close_idx: int) -> bool:
    """True if `)` closes `fun [ <…> ] name ( … )`."""
    open_idx = match_paren_open(src, close_idx)
    if open_idx is None:
        return False
    k = skip_ws_back(src, open_idx - 1)
    if k < 0:
        return False
    # function name
    if not (src[k].isalnum() or src[k] == "_"):
        return False
    while k >= 0 and (src[k].isalnum() or src[k] == "_"):
        k -= 1
    k = skip_ws_back(src, k)
    # optional type params `<…>`
    if k >= 0 and src[k] == ">":
        depth = 0
        while k >= 0:
            if src[k] == ">":
                depth += 1
            elif src[k] == "<":
                depth -= 1
                if depth == 0:
                    break
            k -= 1
        k = skip_ws_back(src, k - 1)
    if k < 2:
        return False
    # `fun`
    start = k - 2
    if start >= 0 and src[start : k + 1] == "fun":
        if start == 0 or not (src[start - 1].isalnum() or src[start - 1] == "_"):
            return True
    return False


def is_trailing_lambda(src: str, brace_idx: int) -> bool:
    """Trailing `call(...) { }` or `recv.method { }` (keep until Phase 5)."""
    i = skip_ws_back(src, brace_idx - 1)
    if i < 0:
        return False
    if src[i] == ")":
        return not is_fun_param_list_close(src, i)
    if src[i].isalnum() or src[i] == "_":
        j = i
        while j >= 0 and (src[j].isalnum() or src[j] == "_"):
            j -= 1
        k = skip_ws_back(src, j)
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
        # line comments
        if src[i] == "/" and i + 1 < n and src[i + 1] == "/":
            while i < n and src[i] != "\n":
                out.append(src[i])
                i += 1
            continue
        # strings
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

        # Keep trailing call/UFCS lambdas verbatim (Phase 5).
        if is_trailing_lambda(src, i):
            end = find_matching_brace(src, i)
            out.append(src[i : end + 1])
            i = end + 1
            continue

        # Already `lambda … {`
        j = skip_ws_back(src, i - 1)
        if j >= 5 and src[j - 5 : j + 1] == "lambda":
            out.append("{")
            i += 1
            continue
        # `lambda a, b {` — word before `{` is a param name, and `lambda` precedes params
        if j >= 0 and (src[j].isalnum() or src[j] == "_"):
            # check if this is already keyword-lambda body: look for `lambda` before param list
            k = j
            while k >= 0 and (src[k].isalnum() or src[k] in "_ \t,"):
                k -= 1
            # k at char before params; params start after
            rest = src[k + 1 : i]
            if "lambda" in rest.split() or rest.lstrip().startswith("lambda"):
                # e.g. already migrated `lambda x, y {`
                out.append("{")
                i += 1
                continue

        m_arrow = ARROW_HEAD.match(src, i)
        if m_arrow:
            end = find_matching_brace(src, i)
            before = skip_ws_back(src, i - 1)
            after = end + 1
            while after < n and src[after] in " \t\n\r":
                after += 1
            called = after < n and src[after] == "("
            assigned = before >= 0 and src[before] == "="
            as_arg = before >= 0 and src[before] in "(,"
            if not (called or assigned or as_arg):
                # when/if bodies etc. — keep brace, scan inside
                out.append("{")
                i += 1
                continue
            params = re.sub(r"\s+", " ", m_arrow.group(1).strip())
            params = re.sub(r"\s*,\s*", ", ", params)
            body_start = m_arrow.end()
            body = src[body_start:end].strip()
            out.append(f"lambda {params} {{ {body} }}")
            i = end + 1
            continue

        m_it = IT_HEAD.match(src, i)
        if m_it:
            end = find_matching_brace(src, i)
            before = skip_ws_back(src, i - 1)
            after = end + 1
            while after < n and src[after] in " \t\n\r":
                after += 1
            called = after < n and src[after] == "("
            assigned = before >= 0 and src[before] == "="
            as_arg = before >= 0 and src[before] in "(,"
            if not (called or assigned or as_arg):
                out.append("{")
                i += 1
                continue
            inner = src[i + 1 : end].strip()
            out.append(f"lambda {{ {inner} }}")
            i = end + 1
            continue

        end = find_matching_brace(src, i)
        before = skip_ws_back(src, i - 1)
        after = end + 1
        while after < n and src[after] in " \t\n\r":
            after += 1
        called = after < n and src[after] == "("
        assigned = before >= 0 and src[before] == "="
        as_arg = before >= 0 and src[before] in "(,"
        if called or assigned or as_arg:
            inner = src[i + 1 : end]
            # anonymous struct — leave alone
            if re.match(r"\s*[A-Za-z_][A-Za-z0-9_]*\s*=", inner) or re.match(
                r"\s*[A-Za-z_][A-Za-z0-9_]*\s*,\s*[A-Za-z_]", inner
            ):
                out.append("{")
                i += 1
                continue
            body = inner.strip()
            out.append(f"lambda {{ {body} }}")
            i = end + 1
            continue

        # Ordinary block / control body — enter and keep scanning inside
        out.append("{")
        i += 1

    return "".join(out)


def main() -> int:
    changed = 0
    for rel in FILES:
        path = ROOT / rel
        if not path.exists():
            print(f"MISSING {rel}", file=sys.stderr)
            continue
        old = path.read_text(encoding="utf-8")
        new = migrate(old)
        if new != old:
            path.write_text(new, encoding="utf-8")
            changed += 1
            print(f"updated {rel}")
        else:
            print(f"unchanged {rel}")
    print(f"done: {changed} files changed")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
