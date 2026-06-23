#!/usr/bin/env python3
"""Re-split define_list_core using include! fragments (shared scope)."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
# Restore from git if needed - use tree/core fragments approach
CORE_DIR = ROOT / "crates/action-codegen/src/runtime_decl/list/core"
TREE_DIR = ROOT / "crates/action-codegen/src/runtime_decl/list/tree"

CORE_SEGMENTS = [
    ("create.inc.rs", 30, 72),
    ("push.inc.rs", 73, 1633),
    ("get.inc.rs", 1634, 2002),
    ("format.inc.rs", 2003, 2084),
    ("cow.inc.rs", 2085, 2934),
    ("query.inc.rs", 2935, 3356),
    ("walk_map.inc.rs", 3357, 4385),
    ("walk_map_filter.inc.rs", 4386, 4944),
    ("walk_filter_map.inc.rs", 4945, 5494),
    ("walk_take_while.inc.rs", 5495, 6035),
    ("walk_map_take_while.inc.rs", 6036, 6615),
    ("walk_find.inc.rs", 6616, 7077),
    ("walk_fold.inc.rs", 7078, 7404),
    ("walk_any_all.inc.rs", 7405, 8050),
    ("query_contains.inc.rs", 8051, 8083),
]

TREE_SEGMENTS = [
    ("slice.inc.rs", 28, 442),
    ("insert.inc.rs", 443, 1220),
    ("remove.inc.rs", 1221, 2208),
    ("flatten.inc.rs", 2209, 2390),
    ("push_subtree.inc.rs", 2391, 3032),
    ("split_chunks.inc.rs", 3033, 3401),
    ("index_of.inc.rs", 3402, 3425),
    ("concat.inc.rs", 3426, 3885),
]

def write_fragments(segments, src_path, dst_dir, wrapper_name, method_name):
    if not src_path.exists():
        print(f"Missing {src_path}, skip")
        return
    lines = src_path.read_text().splitlines()
    dst_dir.mkdir(parents=True, exist_ok=True)
    for fname, start, end in segments:
        chunk = "\n".join(lines[start - 1 : end])
        (dst_dir / fname).write_text(chunk + "\n")

    includes = "\n".join(f'        include!("{fname}");' for fname, _, _ in segments)
    mod_rs = f"""//! List {wrapper_name} runtime fragments (R3-4/R3-5 include! split).

use crate::{{llvm_err, CodeGen}};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {{
    pub(in crate::runtime_decl) fn {method_name}(&self) -> Result<(), String> {{
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let _str_ty = self.string_type;
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let zero = self.i64_ty().const_int(0, false);

{includes}
        Ok(())
    }}
}}
"""
    (dst_dir / "mod.rs").write_text(mod_rs)
    # remove old standalone impl .rs files (not .inc.rs fragments)
    for p in dst_dir.iterdir():
        if p.suffix == ".rs" and p.name != "mod.rs" and ".inc" not in p.name:
            p.unlink()
    print(f"Wrote {wrapper_name} with {len(segments)} fragments")

if __name__ == "__main__":
    # Need original files - restore from git
    import subprocess
    subprocess.run(["git", "checkout", "HEAD", "--",
        "crates/action-codegen/src/runtime_decl/list/define_list_core.rs",
        "crates/action-codegen/src/runtime_decl/list/define_list_tree.rs"], cwd=ROOT, check=True)
    core_src = ROOT / "crates/action-codegen/src/runtime_decl/list/define_list_core.rs"
    tree_src = ROOT / "crates/action-codegen/src/runtime_decl/list/define_list_tree.rs"
    write_fragments(CORE_SEGMENTS, core_src, CORE_DIR, "core", "define_list_core")
    write_fragments(TREE_SEGMENTS, tree_src, TREE_DIR, "tree", "define_list_tree")
    core_src.unlink()
    tree_src.unlink()
    print("Done")
