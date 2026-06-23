#!/usr/bin/env python3
"""Split define_list_core.rs into core/ submodules (R3-4)."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "crates/action-codegen/src/runtime_decl/list/define_list_core.rs"
DST = ROOT / "crates/action-codegen/src/runtime_decl/list/core"

HEADER = """// Submodule: runtime_decl/list/core/{name}
//
// Split from define_list_core.rs (R3-4).

use crate::{{llvm_err, CodeGen}};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {{
    pub(in crate::runtime_decl) fn {method}(&self) -> Result<(), String> {{
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let _str_ty = self.string_type;
        let b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let zero = self.i64_ty().const_int(0, false);

{body}
        Ok(())
    }}
}}
"""

MOD_RS = """//! List core runtime: create, get, mutate, walk helpers.

mod create;
mod push;
mod get;
mod format;
mod cow;
mod query;
mod walk_map;
mod walk_map_filter;
mod walk_filter_map;
mod walk_take_while;
mod walk_map_take_while;
mod walk_find;
mod walk_fold;
mod walk_any_all;

use super::super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    pub(in crate::runtime_decl) fn define_list_core(&self) -> Result<(), String> {
        self.define_list_core_create()?;
        self.define_list_core_push()?;
        self.define_list_core_get()?;
        self.define_list_core_format()?;
        self.define_list_core_cow()?;
        self.define_list_core_query()?;
        self.define_list_core_walk_map()?;
        self.define_list_core_walk_map_filter()?;
        self.define_list_core_walk_filter_map()?;
        self.define_list_core_walk_take_while()?;
        self.define_list_core_walk_map_take_while()?;
        self.define_list_core_walk_find()?;
        self.define_list_core_walk_fold()?;
        self.define_list_core_walk_any_all()?;
        Ok(())
    }
}
"""

# (filename, method_suffix, start_line, end_line inclusive) — 1-based line numbers
SEGMENTS = [
    ("create", "create", 30, 72),
    ("push", "push", 73, 1633),
    ("get", "get", 1634, 2002),
    ("format", "format", 2003, 2084),
    ("cow", "cow", 2085, 2934),
    ("query", "query", 2935, 3356),
    ("walk_map", "walk_map", 3357, 3865),
    ("walk_map", "walk_map", 3866, 4385),  # append to walk_map file
    ("walk_map_filter", "walk_map_filter", 4386, 4944),
    ("walk_filter_map", "walk_filter_map", 4945, 5494),
    ("walk_take_while", "walk_take_while", 5495, 6035),
    ("walk_map_take_while", "walk_map_take_while", 6036, 6615),
    ("walk_find", "walk_find", 6616, 7077),
    ("walk_fold", "walk_fold", 7078, 7404),
    ("walk_any_all", "walk_any_all", 7405, 8050),
    ("query", "query", 8051, 8084),  # contains — append to query
]


def main():
    lines = SRC.read_text().splitlines()
    DST.mkdir(parents=True, exist_ok=True)

    bodies: dict[str, list[str]] = {}
    for fname, method, start, end in SEGMENTS:
        chunk = lines[start - 1 : end]
        if fname in bodies:
            bodies[fname].extend(chunk)
        else:
            bodies[fname] = chunk

    methods = {
        "create": "define_list_core_create",
        "push": "define_list_core_push",
        "get": "define_list_core_get",
        "format": "define_list_core_format",
        "cow": "define_list_core_cow",
        "query": "define_list_core_query",
        "walk_map": "define_list_core_walk_map",
        "walk_map_filter": "define_list_core_walk_map_filter",
        "walk_filter_map": "define_list_core_walk_filter_map",
        "walk_take_while": "define_list_core_walk_take_while",
        "walk_map_take_while": "define_list_core_walk_map_take_while",
        "walk_find": "define_list_core_walk_find",
        "walk_fold": "define_list_core_walk_fold",
        "walk_any_all": "define_list_core_walk_any_all",
    }

    for fname, body_lines in bodies.items():
        body = "\n".join("        " + ln if ln.strip() else "" for ln in body_lines)
        content = HEADER.format(name=fname + ".rs", method=methods[fname], body=body)
        (DST / f"{fname}.rs").write_text(content)

    (DST / "mod.rs").write_text(MOD_RS)
    print(f"Wrote {len(bodies)} files to {DST}")


if __name__ == "__main__":
    main()
