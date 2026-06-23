#!/usr/bin/env python3
"""Concatenate list core/tree *.inc.rs fragments into body.inc.rs (block-wrapped)."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

CORE_ORDER = [
    "create.inc.rs", "push_head.inc.rs", "push_tail.inc.rs", "get.inc.rs", "format.inc.rs", "cow.inc.rs", "query.inc.rs",
    "walk_map.inc.rs", "walk_map_filter.inc.rs", "walk_filter_map.inc.rs", "walk_take_while.inc.rs",
    "walk_map_take_while.inc.rs", "walk_find.inc.rs", "walk_fold.inc.rs", "walk_any_all.inc.rs",
    "query_contains.inc.rs",
]
TREE_ORDER = [
    "slice.inc.rs", "insert.inc.rs", "remove.inc.rs", "flatten.inc.rs", "push_subtree.inc.rs",
    "split_chunks.inc.rs", "index_of.inc.rs", "concat.inc.rs",
]

def concat(d, order):
    base = ROOT / f"crates/action-codegen/src/runtime_decl/list/{d}"
    body = "".join((base / n).read_text() for n in order)
    (base / "body.inc.rs").write_text("{\n" + body + "}\n")
    print(f"{d}/body.inc.rs: {len(body.splitlines())} lines from {len(order)} fragments")

if __name__ == "__main__":
    concat("core", CORE_ORDER)
    concat("tree", TREE_ORDER)
