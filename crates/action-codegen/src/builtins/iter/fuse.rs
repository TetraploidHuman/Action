//! Iterator builtins: map, filter, fold, find (R4-1).

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// Fused map+filter: single tree walk over inner list.

    /// Fused filter+map: single B-tree walk (filter then map on survivors).

    pub(crate) fn fused_filter_map_hir(
        &mut self,
        filter_fn_expr: &action_frontend::hir::HirExpr,
        inner_list_expr: &action_frontend::hir::HirExpr,
        map_fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let filter_fn_val = self.compile_hir_expr(filter_fn_expr)?;
        let inner_list_val = self.compile_hir_expr(inner_list_expr)?;
        self.fused_filter_map_values(filter_fn_val, inner_list_val, map_fn_val)
    }

    pub(crate) fn fused_filter_map_values(
        &mut self,
        filter_fn_val: TypedValue<'ctx>,
        inner_list_val: TypedValue<'ctx>,
        map_fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let filter_fn_ptr = match filter_fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused filter+map: filter function required".to_string()),
        };
        let map_fn_ptr = match map_fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fused filter+map: map function required".to_string()),
        };
        let list_ptr = match inner_list_val {
            TypedValue::List(p) => p,
            _ => return Err("fused filter+map: list required".to_string()),
        };
        let list_struct = self.load_list(list_ptr)?;
        let result_cc = self.call_rt(
            "action_list_filter_map_walk",
            &[list_struct.into(), filter_fn_ptr.into(), map_fn_ptr.into()],
        )?;
        let result_bv = result_cc
            .try_as_basic_value()
            .basic()
            .ok_or("filter_map_walk failed")?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "fm_res")
            .map_err(llvm_err)?;
        self.builder
            .build_store(res_a, result_bv)
            .map_err(llvm_err)?;
        Ok(TypedValue::List(res_a))
    }

    /// filter+map+fold: single B-tree walk (no intermediate List).
    pub(crate) fn fused_filter_map_fold_values(
        &mut self,
        filter_fn_val: TypedValue<'ctx>,
        map_fn_val: TypedValue<'ctx>,
        fold_fn_val: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
        init: inkwell::values::IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let filter_fn_ptr = match filter_fn_val {
            TypedValue::Fn(p, _) | TypedValue::Closure { fn_ptr: p, .. } => p,
            _ => return Err("fused filter+map+fold: filter function required".to_string()),
        };
        let map_fn_ptr = match map_fn_val {
            TypedValue::Fn(p, _) | TypedValue::Closure { fn_ptr: p, .. } => p,
            _ => return Err("fused filter+map+fold: map function required".to_string()),
        };
        let fold_fn_ptr = match fold_fn_val {
            TypedValue::Fn(p, _) | TypedValue::Closure { fn_ptr: p, .. } => p,
            _ => return Err("fused filter+map+fold: fold function required".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("fused filter+map+fold: list required".to_string()),
        };
        let list_struct = self.load_list(list_ptr)?;
        let fm_cc = self.call_rt(
            "action_list_filter_map_fold_walk",
            &[
                list_struct.into(),
                filter_fn_ptr.into(),
                map_fn_ptr.into(),
                fold_fn_ptr.into(),
                init.into(),
            ],
        )?;
        let acc = fm_cc
            .try_as_basic_value()
            .basic()
            .ok_or("filter_map_fold_walk failed")?
            .into_int_value();
        Ok(TypedValue::Int(acc))
    }

    pub(crate) fn fused_filter_map_fold_hir(
        &mut self,
        filter_fn_expr: &action_frontend::hir::HirExpr,
        map_fn_expr: &action_frontend::hir::HirExpr,
        fold_fn_expr: &action_frontend::hir::HirExpr,
        list_expr: &action_frontend::hir::HirExpr,
        init_expr: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let filter_fn_val = self.compile_hir_expr(filter_fn_expr)?;
        let map_fn_val = self.compile_hir_expr(map_fn_expr)?;
        let fold_fn_val = self.compile_hir_expr(fold_fn_expr)?;
        let list_val = self.compile_hir_expr(list_expr)?;
        let init_val = self.compile_hir_expr(init_expr)?;
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Err("fused filter+map+fold: init must be Int".to_string()),
        };
        self.fused_filter_map_fold_values(
            filter_fn_val,
            map_fn_val,
            fold_fn_val,
            list_val,
            init_i64,
        )
    }

    pub(crate) fn try_fused_filter_map_fold_fold_args(
        &mut self,
        init_arg: &CallArg<'_>,
        list_arg: &CallArg<'_>,
        fold_lam: &CallArg<'_>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let CallArg::Hir(list_hir) = list_arg;
        let (map_lam, filter_inner) = match Self::extract_map_call_args_hir(list_hir) {
            Some(v) => v,
            None => return Ok(None),
        };
        let (filter_lam, base_list) = match Self::extract_filter_call_args_hir(filter_inner) {
            Some(v) => v,
            None => return Ok(None),
        };
        let init_val = self.compile_call_arg(*init_arg)?;
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Ok(None),
        };
        let filter_fn_val = self.compile_call_arg(CallArg::Hir(filter_lam))?;
        let map_fn_val = self.compile_call_arg(CallArg::Hir(map_lam))?;
        let fold_fn_val = self.compile_call_arg(*fold_lam)?;
        let list_val = self.compile_call_arg(CallArg::Hir(base_list))?;
        Ok(Some(self.fused_filter_map_fold_values(
            filter_fn_val,
            map_fn_val,
            fold_fn_val,
            list_val,
            init_i64,
        )?))
    }

    /// map+fold: single B-tree walk (no intermediate List).
    pub(crate) fn fused_map_fold_values(
        &mut self,
        map_fn_val: TypedValue<'ctx>,
        fold_fn_val: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
        init: inkwell::values::IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let map_fn_ptr = match map_fn_val {
            TypedValue::Fn(p, _) | TypedValue::Closure { fn_ptr: p, .. } => p,
            _ => return Err("fused map+fold: map function required".to_string()),
        };
        let fold_fn_ptr = match fold_fn_val {
            TypedValue::Fn(p, _) | TypedValue::Closure { fn_ptr: p, .. } => p,
            _ => return Err("fused map+fold: fold function required".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("fused map+fold: list required".to_string()),
        };
        let list_struct = self.load_list(list_ptr)?;
        let mf_cc = self.call_rt(
            "action_list_map_fold_walk",
            &[
                list_struct.into(),
                map_fn_ptr.into(),
                fold_fn_ptr.into(),
                init.into(),
            ],
        )?;
        let acc = mf_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_fold_walk failed")?
            .into_int_value();
        Ok(TypedValue::Int(acc))
    }

    pub(crate) fn fused_map_fold_hir(
        &mut self,
        map_fn_expr: &action_frontend::hir::HirExpr,
        fold_fn_expr: &action_frontend::hir::HirExpr,
        list_expr: &action_frontend::hir::HirExpr,
        init_expr: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let map_fn_val = self.compile_hir_expr(map_fn_expr)?;
        let fold_fn_val = self.compile_hir_expr(fold_fn_expr)?;
        let list_val = self.compile_hir_expr(list_expr)?;
        let init_val = self.compile_hir_expr(init_expr)?;
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Err("fused map+fold: init must be Int".to_string()),
        };
        self.fused_map_fold_values(map_fn_val, fold_fn_val, list_val, init_i64)
    }

    pub(crate) fn try_fused_map_fold_fold_args(
        &mut self,
        init_arg: &CallArg<'_>,
        list_arg: &CallArg<'_>,
        fold_lam: &CallArg<'_>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let CallArg::Hir(list_hir) = list_arg;
        let (map_lam, base_list) = match Self::extract_map_call_args_hir(list_hir) {
            Some(v) => v,
            None => return Ok(None),
        };
        let init_val = self.compile_call_arg(*init_arg)?;
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Ok(None),
        };
        let map_fn_val = self.compile_call_arg(CallArg::Hir(map_lam))?;
        let fold_fn_val = self.compile_call_arg(*fold_lam)?;
        let list_val = self.compile_call_arg(CallArg::Hir(base_list))?;
        Ok(Some(self.fused_map_fold_values(
            map_fn_val,
            fold_fn_val,
            list_val,
            init_i64,
        )?))
    }

    /// filter+fold: single B-tree walk (no intermediate List).
    pub(crate) fn fused_filter_fold_values(
        &mut self,
        filter_fn_val: TypedValue<'ctx>,
        fold_fn_val: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
        init: inkwell::values::IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let filter_fn_ptr = match filter_fn_val {
            TypedValue::Fn(p, _) | TypedValue::Closure { fn_ptr: p, .. } => p,
            _ => return Err("fused filter+fold: filter function required".to_string()),
        };
        let fold_fn_ptr = match fold_fn_val {
            TypedValue::Fn(p, _) | TypedValue::Closure { fn_ptr: p, .. } => p,
            _ => return Err("fused filter+fold: fold function required".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("fused filter+fold: list required".to_string()),
        };
        let list_struct = self.load_list(list_ptr)?;
        let ff_cc = self.call_rt(
            "action_list_filter_fold_walk",
            &[
                list_struct.into(),
                filter_fn_ptr.into(),
                fold_fn_ptr.into(),
                init.into(),
            ],
        )?;
        let acc = ff_cc
            .try_as_basic_value()
            .basic()
            .ok_or("filter_fold_walk failed")?
            .into_int_value();
        Ok(TypedValue::Int(acc))
    }

    pub(crate) fn fused_filter_fold_hir(
        &mut self,
        filter_fn_expr: &action_frontend::hir::HirExpr,
        fold_fn_expr: &action_frontend::hir::HirExpr,
        list_expr: &action_frontend::hir::HirExpr,
        init_expr: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let filter_fn_val = self.compile_hir_expr(filter_fn_expr)?;
        let fold_fn_val = self.compile_hir_expr(fold_fn_expr)?;
        let list_val = self.compile_hir_expr(list_expr)?;
        let init_val = self.compile_hir_expr(init_expr)?;
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Err("fused filter+fold: init must be Int".to_string()),
        };
        self.fused_filter_fold_values(filter_fn_val, fold_fn_val, list_val, init_i64)
    }

    pub(crate) fn try_fused_filter_fold_fold_args(
        &mut self,
        init_arg: &CallArg<'_>,
        list_arg: &CallArg<'_>,
        fold_lam: &CallArg<'_>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let CallArg::Hir(list_hir) = list_arg;
        let (filter_lam, base_list) = match Self::extract_filter_call_args_hir(list_hir) {
            Some(v) => v,
            None => return Ok(None),
        };
        let init_val = self.compile_call_arg(*init_arg)?;
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Ok(None),
        };
        let filter_fn_val = self.compile_call_arg(CallArg::Hir(filter_lam))?;
        let fold_fn_val = self.compile_call_arg(*fold_lam)?;
        let list_val = self.compile_call_arg(CallArg::Hir(base_list))?;
        Ok(Some(self.fused_filter_fold_values(
            filter_fn_val,
            fold_fn_val,
            list_val,
            init_i64,
        )?))
    }
}
