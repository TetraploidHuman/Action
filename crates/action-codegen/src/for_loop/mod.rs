//! For-loop codegen submodule tree.

// Submodule: for_loop

use action_frontend::ast::*;
use action_frontend::hir::{HirExpr, HirExprKind};
use inkwell::values::{IntValue, PointerValue};

use super::{llvm_err, CodeGen, TypedValue, ValKind};

pub(super) enum ForExprSrc<'a> {
    Hir(&'a HirExpr),
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

    fn compile_list_iterable<'ctx>(
        &self,
        gen: &mut CodeGen<'ctx>,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>, PointerValue<'ctx>), String> {
        let i64 = gen.i64_ty();
        let list_val = self.compile(gen)?;
        let list_ptr = match &list_val {
            TypedValue::List(p) | TypedValue::Set(p) | TypedValue::Map(p) => *p,
            TypedValue::Stream(p) => gen
                .builder
                .build_struct_gep(gen.stream_type, *p, 1, "for_sl")
                .map_err(llvm_err)?,
            TypedValue::LazyList(_) => {
                let converted = gen.convert_lazylist_to_list(&list_val)?;
                let alloca = gen
                    .builder
                    .build_alloca(gen.list_type, "ll_to_list")
                    .map_err(llvm_err)?;
                gen.builder
                    .build_store(alloca, converted)
                    .map_err(llvm_err)?;
                alloca
            }
            _ => {
                return Err(
                    "Only range iterators (1..10), lists, sets, maps, streams and lazy lists are supported for for expressions"
                        .to_string(),
                );
            }
        };
        let loaded = gen.load_list(list_ptr)?;
        let len = gen.list_len_val(loaded)?;
        let zero = i64.const_int(0, false);
        Ok((zero, len, list_ptr))
    }
}

mod cache;
mod hir;
mod iterate;
mod store;
