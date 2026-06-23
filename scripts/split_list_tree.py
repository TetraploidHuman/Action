#!/usr/bin/env python3
"""Split define_list_tree.rs into tree/ submodules (R3-5)."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "crates/action-codegen/src/runtime_decl/list/define_list_tree.rs"
DST = ROOT / "crates/action-codegen/src/runtime_decl/list/tree"

HEADER = """// Submodule: runtime_decl/list/tree/{name}
//
// Split from define_list_tree.rs (R3-5).

use crate::{{llvm_err, CodeGen}};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {{
    pub(in crate::runtime_decl) fn {method}(&self) -> Result<(), String> {{
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let _void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let zero = self.i64_ty().const_int(0, false);

{body}
        Ok(())
    }}
}}
"""

MOD_RS = """//! List tree runtime: insert, remove, concat, flatten.

mod slice;
mod insert;
mod remove;
mod flatten;
mod push_subtree;
mod split_chunks;
mod index_of;
mod concat;

use super::super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    pub(in crate::runtime_decl) fn define_list_tree(&self) -> Result<(), String> {
        self.define_list_tree_slice()?;
        self.define_list_tree_insert()?;
        self.define_list_tree_remove()?;
        self.define_list_tree_flatten()?;
        self.define_list_tree_push_subtree()?;
        self.define_list_tree_split_chunks()?;
        self.define_list_tree_index_of()?;
        self.define_list_tree_concat()?;
        Ok(())
    }
}
"""

SEGMENTS = [
    ("slice", "slice", 28, 442),
    ("insert", "insert", 443, 1220),
    ("remove", "remove", 1221, 2208),
    ("flatten", "flatten", 2209, 2390),
    ("push_subtree", "push_subtree", 2391, 3032),
    ("split_chunks", "split_chunks", 3033, 3401),
    ("index_of", "index_of", 3402, 3425),
    ("concat", "concat", 3426, 3887),
]


def main():
    lines = SRC.read_text().splitlines()
    DST.mkdir(parents=True, exist_ok=True)

    methods = {
        "slice": "define_list_tree_slice",
        "insert": "define_list_tree_insert",
        "remove": "define_list_tree_remove",
        "flatten": "define_list_tree_flatten",
        "push_subtree": "define_list_tree_push_subtree",
        "split_chunks": "define_list_tree_split_chunks",
        "index_of": "define_list_tree_index_of",
        "concat": "define_list_tree_concat",
    }

    for fname, _, start, end in SEGMENTS:
        chunk = lines[start - 1 : end]
        body = "\n".join("        " + ln if ln.strip() else "" for ln in chunk)
        content = HEADER.format(name=fname + ".rs", method=methods[fname], body=body)
        (DST / f"{fname}.rs").write_text(content)

    (DST / "mod.rs").write_text(MOD_RS)
    print(f"Wrote {len(SEGMENTS)} files to {DST}")


if __name__ == "__main__":
    main()
