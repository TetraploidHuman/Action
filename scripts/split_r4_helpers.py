#!/usr/bin/env python3
"""R4-5: split action-lsp handlers/helpers.rs into submodule tree."""
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
src = ROOT / "crates/action-lsp/src/handlers/helpers.rs"
lines = src.read_text().splitlines()
dst = ROOT / "crates/action-lsp/src/handlers/helpers"
dst.mkdir(parents=True, exist_ok=True)

HEADER = """#![allow(unused_imports)]
use std::collections::HashMap;

use crate::position::{self, find_node_at, FoundNode};
use crate::symbols;
use action_frontend::ast::{Expr, ExprKind, Stmt, Type};
use action_frontend::builtin::{
    format_ufcs_method_detail, receiver_kind_from_type, ufcs_methods_for_kind,
};
use action_frontend::lexer::{Span, Token, TokenKind};
use action_frontend::typecheck::TypeRegistry;
use lsp_types::{CompletionItem, CompletionItemKind, Position, Range};
"""

sections = [
    ("completion.rs", 17, 268),
    ("scope.rs", 274, 695),
    ("signature.rs", 701, 956),
]

for fname, start, end in sections:
    body = "\n".join(lines[start - 1 : end])
    (dst / fname).write_text(HEADER + "\n" + body + "\n")

(dst / "tests.rs").write_text("\n".join(lines[957:]) + "\n")

(dst / "mod.rs").write_text(
    """//! LSP handler helpers (R4-5).

mod completion;
mod scope;
mod signature;

pub(crate) use completion::*;
pub(crate) use scope::*;
pub(crate) use signature::*;

#[cfg(test)]
mod tests;
"""
)

src.unlink()
print("Split helpers.rs -> helpers/ (4 files)")
