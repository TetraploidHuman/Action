#!/usr/bin/env python3
"""R4: split large impl modules into subdirectories (multiple impl CodeGen blocks)."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def split_impl_file(
    src_rel: str,
    dst_dir_rel: str,
    header: str,
    segments: list[tuple[str, int, int]],
    mod_doc: str,
):
    src = ROOT / src_rel
    lines = src.read_text().splitlines()
    dst = ROOT / dst_dir_rel
    dst.mkdir(parents=True, exist_ok=True)
    for fname, start, end in segments:
        body = "\n".join(lines[start - 1 : end])
        content = f"{header}\n\nimpl<'ctx> CodeGen<'ctx> {{\n{body}\n}}\n"
        (dst / fname).write_text(content)
    mods = "\n".join(f"mod {fname.replace('.rs', '')};" for fname, _, _ in segments)
    (dst / "mod.rs").write_text(f"{mod_doc}\n\n{mods}\n")
    src.unlink()
    print(f"Split {src_rel} -> {dst_dir_rel} ({len(segments)} files)")


def split_with_preamble(
    src_rel: str,
    dst_dir_rel: str,
    preamble_lines: tuple[int, int],
    header: str,
    segments: list[tuple[str, int, int]],
    mod_doc: str,
    tail_lines: tuple[int, int] | None = None,
):
    src = ROOT / src_rel
    lines = src.read_text().splitlines()
    dst = ROOT / dst_dir_rel
    dst.mkdir(parents=True, exist_ok=True)
    pre = "\n".join(lines[preamble_lines[0] - 1 : preamble_lines[1]])
    for fname, start, end in segments:
        body = "\n".join(lines[start - 1 : end])
        content = f"{header}\n\nimpl<'ctx> CodeGen<'ctx> {{\n{body}\n}}\n"
        (dst / fname).write_text(content)
    mods = "\n".join(f"mod {fname.replace('.rs', '')};" for fname, _, _ in segments)
    tail = ""
    if tail_lines:
        tail = "\n" + "\n".join(lines[tail_lines[0] - 1 : tail_lines[1]]) + "\n"
    (dst / "mod.rs").write_text(f"{mod_doc}\n\n{pre}\n\n{mods}\n{tail}")
    src.unlink()
    print(f"Split {src_rel} -> {dst_dir_rel}")


ITER_HEADER = """//! Iterator builtins: map, filter, fold, find (R4-1).

use inkwell::values::{IntValue, PointerValue};
use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};"""

MONO_HEADER = """//! Monomorphic lambda direct-call specialization (R4-2).

use inkwell::types::BasicTypeEnum;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};"""

EXPR_HEADER = """//! Expression codegen (R4-3).

use action_frontend::ast::*;
use inkwell::builder::BuilderError;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum, FunctionType, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};
use inkwell::{FloatPredicate, IntPredicate};

use super::{llvm_err, CodeGen, InnerType, Scope, TypedValue, ValKind};"""

FOR_HEADER = """//! For-loop codegen (R4-4).

use action_frontend::ast::*;
use action_frontend::hir::{HirExpr, HirExprKind, HirForIterKind, HirStmt};
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue, ValKind};"""


def main():
    split_impl_file(
        "crates/action-codegen/src/builtins/iter.rs",
        "crates/action-codegen/src/builtins/iter",
        ITER_HEADER,
        [
            ("map.rs", 10, 91),
            ("fuse.rs", 93, 414),
            ("filter.rs", 416, 499),
            ("fold_core.rs", 501, 620),
            ("any_all.rs", 621, 710),
            ("find.rs", 711, 1062),
            ("reduce.rs", 1063, 1315),
            ("advanced.rs", 1316, 1703),
            ("extract.rs", 1704, 2422),
            ("callback.rs", 2424, 2436),
        ],
        "//! Iterator builtins submodule tree.",
    )

    split_with_preamble(
        "crates/action-codegen/src/lambda_mono.rs",
        "crates/action-codegen/src/mono",
        (1, 25),
        MONO_HEADER,
        [
            ("cache.rs", 28, 281),
            ("map_walk.rs", 283, 789),
            ("filter_walk.rs", 790, 1304),
            ("fold_walk.rs", 1305, 1617),
            ("any_all_walk.rs", 1618, 2417),
        ],
        "//! Lambda monomorphization submodule tree.",
    )

    split_with_preamble(
        "crates/action-codegen/src/expr.rs",
        "crates/action-codegen/src/expr",
        (1, 10),
        EXPR_HEADER,
        [
            ("lambda.rs", 12, 380),
            ("literal.rs", 381, 845),
            ("fat_return.rs", 850, 1008),
            ("binop.rs", 1016, 1627),
            ("coerce.rs", 1629, 1917),
        ],
        "//! Expression codegen submodule tree.",
        tail_lines=(1920, 2193),
    )

    split_with_preamble(
        "crates/action-codegen/src/for_loop.rs",
        "crates/action-codegen/src/for_loop",
        (1, 80),
        FOR_HEADER,
        [
            ("store.rs", 82, 188),
            ("iterate.rs", 189, 740),
            ("cache.rs", 742, 871),
            ("hir.rs", 872, 1993),
        ],
        "//! For-loop codegen submodule tree.",
    )


if __name__ == "__main__":
    main()
