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

    /// map+reduce: single walk — apply map then reduce without materializing List.
    pub(crate) fn fused_map_reduce_hir(
        &mut self,
        map_fn_expr: &action_frontend::hir::HirExpr,
        reduce_fn_expr: &action_frontend::hir::HirExpr,
        list_expr: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let map_fn_val = self.compile_hir_expr(map_fn_expr)?;
        let reduce_fn_val = self.compile_hir_expr(reduce_fn_expr)?;
        let list_val = self.compile_hir_expr(list_expr)?;
        self.fused_map_reduce_values(map_fn_val, reduce_fn_val, list_val, list_expr)
    }

    pub(crate) fn fused_map_reduce_values(
        &mut self,
        map_fn_val: TypedValue<'ctx>,
        reduce_fn_val: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
        list_expr: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        use inkwell::IntPredicate;

        let map_fn_ptr = match map_fn_val {
            TypedValue::Fn(p, _) | TypedValue::Closure { fn_ptr: p, .. } => p,
            _ => return Err("fused map+reduce: map function required".to_string()),
        };
        let reduce_fn_ptr = match reduce_fn_val {
            TypedValue::Fn(p, _) | TypedValue::Closure { fn_ptr: p, .. } => p,
            _ => return Err("fused map+reduce: reduce function required".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("fused map+reduce: list required".to_string()),
        };

        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let is_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, input_len, zero, "mr_empty")
            .map_err(llvm_err)?;

        let acc_a = self
            .builder
            .build_alloca(self.string_type, "mr_acc")
            .map_err(llvm_err)?;
        let found_flag_a = self
            .builder
            .build_alloca(self.bool_ty(), "mr_found")
            .map_err(llvm_err)?;
        let acc_result_a = self
            .builder
            .build_alloca(self.string_type, "mr_acc_s")
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "mr_i").map_err(llvm_err)?;
        self.builder.build_store(i_a, one).map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;

        let init_bb = self.context.append_basic_block(current_fn, "mr_init");
        let loop_hdr = self.context.append_basic_block(current_fn, "mr_hdr");
        let loop_bdy = self.context.append_basic_block(current_fn, "mr_bdy");
        let loop_ext = self.context.append_basic_block(current_fn, "mr_ext");
        let empty_bb = self.context.append_basic_block(current_fn, "mr_empty");
        let merge_bb = self.context.append_basic_block(current_fn, "mr_merge");
        let done_bb = self.context.append_basic_block(current_fn, "mr_done");

        let _ = self
            .builder
            .build_conditional_branch(is_empty, empty_bb, init_bb);

        self.builder.position_at_end(init_bb);
        let first_elem = self.list_get_cached_fat(list_ptr, zero, get_cache)?;
        let first_tag = self
            .builder
            .build_extract_value(first_elem.into_struct_value(), 0, "et0")
            .map_err(llvm_err)?
            .into_int_value();
        let map_ty = self.string_type.fn_type(&[i64.into()], false);
        let mapped0 = self
            .builder
            .build_indirect_call(map_ty, map_fn_ptr, &[first_tag.into()], "map0")
            .map_err(llvm_err)?;
        let acc0 = mapped0.try_as_basic_value().basic().ok_or("map0 failed")?;
        self.builder.build_store(acc_a, acc0).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "mr_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "mr_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_bdy, loop_ext);

        self.builder.position_at_end(loop_bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "mr_et")
            .map_err(llvm_err)?
            .into_int_value();
        let mapped = self
            .builder
            .build_indirect_call(map_ty, map_fn_ptr, &[elem_tag.into()], "mr_map")
            .map_err(llvm_err)?;
        let mapped_val = mapped.try_as_basic_value().basic().ok_or("mr_map failed")?;
        let mapped_tag = self
            .builder
            .build_extract_value(mapped_val.into_struct_value(), 0, "mr_mt")
            .map_err(llvm_err)?
            .into_int_value();
        let acc_fat = self
            .builder
            .build_load(self.string_type, acc_a, "mr_acc_f")
            .map_err(llvm_err)?;
        let acc_tag = self
            .builder
            .build_extract_value(acc_fat.into_struct_value(), 0, "mr_acc_t")
            .map_err(llvm_err)?
            .into_int_value();
        let reduce_ty = self.string_type.fn_type(&[i64.into(), i64.into()], false);
        let reduced = self
            .builder
            .build_indirect_call(
                reduce_ty,
                reduce_fn_ptr,
                &[acc_tag.into(), mapped_tag.into()],
                "mr_reduce",
            )
            .map_err(llvm_err)?;
        let new_acc = reduced
            .try_as_basic_value()
            .basic()
            .ok_or("mr_reduce failed")?;
        self.builder.build_store(acc_a, new_acc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, one, "mr_ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_ext);
        let final_acc = self
            .builder
            .build_load(self.string_type, acc_a, "mr_final")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);

        self.builder.position_at_end(empty_bb);
        let _ = self.builder.build_unconditional_branch(merge_bb);

        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.string_type, "mr_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[
            (&final_acc, loop_ext),
            (&self.string_type.get_undef(), empty_bb),
        ]);
        let phi_flag = self
            .builder
            .build_phi(self.bool_ty(), "mr_flag")
            .map_err(llvm_err)?;
        phi_flag.add_incoming(&[
            (&self.bool_ty().const_int(1, false), loop_ext),
            (&self.bool_ty().const_zero(), empty_bb),
        ]);
        let _ = self.builder.build_unconditional_branch(done_bb);

        self.builder.position_at_end(done_bb);
        self.builder
            .build_store(found_flag_a, phi_flag.as_basic_value())
            .map_err(llvm_err)?;
        self.builder
            .build_store(acc_result_a, phi.as_basic_value())
            .map_err(llvm_err)?;
        let elem_ty = self.list_element_ast_type(CallArg::Hir(list_expr));
        self.build_fallible_from_fat_found_flag(acc_result_a, found_flag_a, &elem_ty)
    }
}
