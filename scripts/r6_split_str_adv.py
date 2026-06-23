#!/usr/bin/env python3
"""R6: split define_str_adv.rs into str_adv/ submodules."""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "crates/action-codegen/src/runtime_decl/define_str_adv.rs"
OUT = ROOT / "crates/action-codegen/src/runtime_decl/str_adv"

MARKERS = [
    ("split", "action_string_split"),
    ("join", "action_string_join"),
    ("replace", "action_string_replace"),
    ("contains", "action_string_contains"),
    ("repeat", "action_string_repeat"),
    ("trim_start", "action_string_trim_start"),
    ("trim_end", "action_string_trim_end"),
]

PREAMBLE = """
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let ptr = self.ptr_ty();
        let b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let memcmp_fn = self.module.get_function("memcmp").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        let str_data_fn = self.module.get_function("action_string_data").unwrap();
"""


def read_lines() -> list[str]:
    return SRC.read_text().splitlines(keepends=True)


def find_marker_line(lines: list[str], fn_name: str) -> int:
    pat = f"// ---- {fn_name}"
    for i, line in enumerate(lines):
        if pat in line:
            return i
    raise SystemExit(f"marker not found: {fn_name}")


lines = read_lines()
first_marker = find_marker_line(lines, MARKERS[0][1])

OUT.mkdir(exist_ok=True)

chunks: list[tuple[str, list[str]]] = []
for idx, (mod_name, fn_name) in enumerate(MARKERS):
    start = find_marker_line(lines, fn_name)
    if idx + 1 < len(MARKERS):
        end = find_marker_line(lines, MARKERS[idx + 1][1])
    else:
        end = len(lines)
        while end > start and lines[end - 1].strip() in ("", "Ok(())", "}"):
            end -= 1
    chunk = lines[start:end]
    chunks.append((mod_name, chunk))

for mod_name, chunk in chunks:
    body = "".join(chunk)
    method_name = f"define_str_{mod_name}"
    content = f"""// Submodule: runtime_decl/str_adv/{mod_name} (R6)

use crate::{{llvm_err, CodeGen}};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {{
    pub(super) fn {method_name}(&self) -> Result<(), String> {{
{PREAMBLE}
{body}
        Ok(())
    }}
}}
"""
    (OUT / f"{mod_name}.rs").write_text(content)

calls = "\n".join(f"        self.define_str_{name}()?;" for name, _ in MARKERS)

mod_rs = f"""// Submodule: runtime_decl/str_adv (R6)

mod contains;
mod join;
mod repeat;
mod replace;
mod split;
mod trim_end;
mod trim_start;

use super::{{CodeGen}};

impl<'ctx> CodeGen<'ctx> {{
    pub(super) fn define_str_adv(&self) -> Result<(), String> {{
{calls}
        Ok(())
    }}
}}
"""
(OUT / "mod.rs").write_text(mod_rs)
print("Created str_adv/ submodules")
