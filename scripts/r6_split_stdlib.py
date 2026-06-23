#!/usr/bin/env python3
"""R6: split collection.rs and datetime.rs into submodules with Optional dispatch."""

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CODEGEN = ROOT / "crates/action-codegen/src/builtins/stdlib"


def read_lines(path: Path) -> list[str]:
    return path.read_text().splitlines(keepends=True)


def write_file(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content)


def transform_ok_some(block: str) -> str:
    lines = block.split("\n")
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        if "Ok(TypedValue" in line and "Ok(Some(TypedValue" not in line:
            acc = [line]
            parens = line.count("(") - line.count(")")
            j = i + 1
            while parens > 0 and j < len(lines):
                acc.append(lines[j])
                parens += lines[j].count("(") - lines[j].count(")")
                j += 1
            acc[0] = acc[0].replace("Ok(TypedValue", "Ok(Some(TypedValue", 1)
            last = acc[-1].rstrip()
            if "//" in last:
                idx = last.index("//")
                before = last[:idx].rstrip()
                comment = last[idx:]
                acc[-1] = before + ")" + comment
            elif last.endswith("),"):
                acc[-1] = last[:-2] + ")),"
            elif last.endswith(");"):
                acc[-1] = last[:-2] + "));"
            elif last.endswith(")"):
                acc[-1] = last + ")"
            out.extend(acc)
            i = j
        else:
            out.append(line)
            i += 1
    return "\n".join(out)


def wrap_helper_returns(block: str) -> str:
    import re

    return re.sub(
        r"^(\s*)self\.(build_nullable_list|emit_today_now)\((.+)\)\s*$",
        r"\1Ok(Some(self.\2(\3)?))",
        block,
        flags=re.MULTILINE,
    )


def extract_match_arms(lines: list[str], start: int, end: int) -> str:
    chunk = "".join(lines[start - 1:end])
    return wrap_helper_returns(transform_ok_some(chunk.rstrip("\n")))


def wrap_collection_module(name: str, extra_impl: str, match_body: str) -> str:
    return f"""// Submodule: builtins_stdlib_collection/{name}

use crate::call_arg::CallArg;
use crate::{{llvm_err, CodeGen, TypedValue}};
use action_frontend::ast::Type;
use inkwell::values::{{BasicValue, IntValue, StructValue}};
use inkwell::IntPredicate;

{extra_impl}

impl<'ctx> CodeGen<'ctx> {{
    pub(crate) fn collection_dispatch_{name}(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {{
        match name {{
{match_body}
            _ => Ok(None),
        }}
    }}
}}
"""


def wrap_datetime_module(name: str, match_body: str) -> str:
    return f"""// Submodule: builtins_stdlib_datetime/{name}

use inkwell::values::{{IntValue, PointerValue}};
use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{{llvm_err, CodeGen, GepCursor, InnerType, TypedValue}};

impl<'ctx> CodeGen<'ctx> {{
    pub(crate) fn datetime_dispatch_{name}(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {{
        match name {{
{match_body}
            _ => Ok(None),
        }}
    }}
}}
"""


collection_src = read_lines(CODEGEN / "collection.rs")
datetime_src = read_lines(CODEGEN / "datetime.rs")

collection_chunks = [
    ("list_basic", 100, 608),
    ("list_gen", 609, 809),
    ("list_misc", 810, 970),
    ("list_transform", 971, 1122),
    ("map_set", 1123, 1299),
    ("aggregate", 1300, 1696),
]

sum_impl = "".join(collection_src[15:92])
col_dir = CODEGEN / "collection"
col_dir.mkdir(exist_ok=True)

for name, start, end in collection_chunks:
    body = extract_match_arms(collection_src, start, end)
    extra_impl = ""
    if name == "list_basic":
        extra_impl = f"impl<'ctx> CodeGen<'ctx> {{\n{sum_impl}}}\n"
    content = wrap_collection_module(name, extra_impl, body)
    write_file(col_dir / f"{name}.rs", content)

dispatch_calls = "\n".join(
    f"        if let Some(v) = self.collection_dispatch_{name}(name, args)? {{\n"
    f"            return Ok(v);\n"
    f"        }}"
    for name, _, _ in collection_chunks
)

col_mod = f"""// Submodule: builtins_stdlib_collection (R6)

mod aggregate;
mod list_basic;
mod list_gen;
mod list_misc;
mod list_transform;
mod map_set;

use crate::call_arg::CallArg;
use crate::{{CodeGen, TypedValue}};

impl<'ctx> CodeGen<'ctx> {{
    pub(crate) fn builtin_stdlib_collection(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {{
{dispatch_calls}
        Err(format!("Unknown collection builtin: {{}}", name))
    }}
}}
"""
write_file(col_dir / "mod.rs", col_mod)

datetime_chunks = [
    ("format_parse", 20, 376),
    ("construct", 377, 865),
    ("random", 866, 1027),
    ("accessors", 1028, 1337),
    ("weekday_utc", 1338, 1668),
]

dt_dir = CODEGEN / "datetime"
dt_dir.mkdir(exist_ok=True)

for name, start, end in datetime_chunks:
    body = extract_match_arms(datetime_src, start, end)
    content = wrap_datetime_module(name, body)
    write_file(dt_dir / f"{name}.rs", content)

today_now_body = "".join(datetime_src[1672:1876])
today_now = f"""// Submodule: builtins_stdlib_datetime/today_now

use inkwell::values::{{IntValue, PointerValue}};
use inkwell::IntPredicate;

use crate::{{llvm_err, CodeGen, GepCursor, TypedValue}};

impl<'ctx> CodeGen<'ctx> {{
{today_now_body}
}}
"""
write_file(dt_dir / "today_now.rs", today_now)

dt_dispatch = "\n".join(
    f"        if let Some(v) = self.datetime_dispatch_{name}(name, args)? {{\n"
    f"            return Ok(v);\n"
    f"        }}"
    for name, _, _ in datetime_chunks
)

dt_mod = f"""// Submodule: builtins_stdlib_datetime (R6)

mod accessors;
mod construct;
mod format_parse;
mod random;
mod today_now;
mod weekday_utc;

use crate::call_arg::CallArg;
use crate::{{CodeGen, TypedValue}};

impl<'ctx> CodeGen<'ctx> {{
    pub(crate) fn builtin_stdlib_datetime(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {{
{dt_dispatch}
        Err(format!("Unknown datetime builtin: {{}}", name))
    }}
}}
"""
write_file(dt_dir / "mod.rs", dt_mod)

print("Created collection/ and datetime/ submodules")
