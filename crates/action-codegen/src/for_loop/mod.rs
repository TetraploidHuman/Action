//! For-loop codegen submodule tree.

// Submodule: for_loop

use action_frontend::ast::*;
use action_frontend::hir::{HirExpr, HirExprKind};
use inkwell::values::{IntValue, PointerValue};

use super::{llvm_err, CodeGen, TypedValue, ValKind};

pub(super) enum ForExprSrc<'a> {
    Hir(&'a HirExpr),
}

/// Classified iterable for `for` loop codegen.
pub(super) enum ForIterable<'ctx> {
    Range {
        start: IntValue<'ctx>,
        end: IntValue<'ctx>,
    },
    List {
        list_ptr: PointerValue<'ctx>,
        len: IntValue<'ctx>,
    },
    Map {
        data_ptr: PointerValue<'ctx>,
        cap: IntValue<'ctx>,
    },
    Set {
        data_ptr: PointerValue<'ctx>,
        cap: IntValue<'ctx>,
    },
}

/// Fused term for `{ x -> x + i }(arg)` when the sole capture is the loop index.
pub(super) enum CapturedIdxAddTerm {
    /// `{ x -> x + i }(i)` → `i + i`
    IdxPlusIdx,
    /// `{ x -> x + i }(K)` → `K + i`
    ConstPlusIdx(u64),
}

impl<'a> ForExprSrc<'a> {
    fn compile<'ctx>(&self, gen: &mut CodeGen<'ctx>) -> Result<TypedValue<'ctx>, String> {
        match self {
            ForExprSrc::Hir(h) => gen.compile_hir_expr(h),
        }
    }

    fn range_start_end<'ctx>(
        &self,
        gen: &mut CodeGen<'ctx>,
    ) -> Result<Option<(IntValue<'ctx>, IntValue<'ctx>)>, String> {
        match self {
            ForExprSrc::Hir(h) => match &h.kind {
                HirExprKind::Binary(lhs, BinaryOp::Range, rhs)
                | HirExprKind::Binary(lhs, BinaryOp::RangeExclusive, rhs)
                | HirExprKind::Range(lhs, rhs) => {
                    let start_v = ForExprSrc::Hir(lhs).compile(gen)?;
                    let end_v = ForExprSrc::Hir(rhs).compile(gen)?;
                    match (start_v, end_v) {
                        (TypedValue::Int(s), TypedValue::Int(e)) => Ok(Some((s, e))),
                        _ => Err("Range bounds must be integers".to_string()),
                    }
                }
                _ => Ok(None),
            },
        }
    }

    fn classify_iterable<'ctx>(
        &self,
        gen: &mut CodeGen<'ctx>,
    ) -> Result<ForIterable<'ctx>, String> {
        if let Some((start, end)) = self.range_start_end(gen)? {
            return Ok(ForIterable::Range { start, end });
        }

        let val = self.compile(gen)?;
        match val {
            TypedValue::Map(map_ptr) => {
                let loaded = gen.load_list(map_ptr)?;
                let data_ptr = gen.list_data_ptr(loaded)?;
                let cap = gen
                    .builder
                    .build_extract_value(loaded, 2, "map_cap")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(ForIterable::Map { data_ptr, cap })
            }
            TypedValue::Set(set_ptr) => {
                let loaded = gen.load_list(set_ptr)?;
                let data_ptr = gen.list_data_ptr(loaded)?;
                let cap = gen
                    .builder
                    .build_extract_value(loaded, 2, "set_cap")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(ForIterable::Set { data_ptr, cap })
            }
            TypedValue::List(list_ptr) => {
                let loaded = gen.load_list(list_ptr)?;
                let len = gen.list_len_val(loaded)?;
                Ok(ForIterable::List { list_ptr, len })
            }
            TypedValue::Stream(stream_ptr) => {
                let list_ptr = gen
                    .builder
                    .build_struct_gep(gen.stream_type, stream_ptr, 1, "for_sl")
                    .map_err(llvm_err)?;
                let loaded = gen.load_list(list_ptr)?;
                let len = gen.list_len_val(loaded)?;
                Ok(ForIterable::List { list_ptr, len })
            }
            TypedValue::LazyList(_) => {
                let converted = gen.convert_lazylist_to_list(&val)?;
                let list_ptr = gen
                    .builder
                    .build_alloca(gen.list_type, "ll_to_list")
                    .map_err(llvm_err)?;
                gen.builder
                    .build_store(list_ptr, converted)
                    .map_err(llvm_err)?;
                let loaded = gen.load_list(list_ptr)?;
                let len = gen.list_len_val(loaded)?;
                Ok(ForIterable::List { list_ptr, len })
            }
            _ => Err(
                "Only range iterators (1..10), lists, sets, maps, streams and lazy lists are supported for for expressions"
                    .to_string(),
            ),
        }
    }

    fn compile_list_iterable<'ctx>(
        &self,
        gen: &mut CodeGen<'ctx>,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>, PointerValue<'ctx>), String> {
        let i64 = gen.i64_ty();
        let zero = i64.const_int(0, false);
        match self.classify_iterable(gen)? {
            ForIterable::List { list_ptr, len } => Ok((zero, len, list_ptr)),
            ForIterable::Map { .. } | ForIterable::Set { .. } => Err(
                "compile_list_iterable called on Map/Set — use hash-table iteration path"
                    .to_string(),
            ),
            ForIterable::Range { .. } => {
                Err("compile_list_iterable called on Range — use range_start_end".to_string())
            }
        }
    }
}

mod cache;
mod hir;
mod iterate;
mod store;
