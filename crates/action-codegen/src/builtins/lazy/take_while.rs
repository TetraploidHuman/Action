use crate::{llvm_err, CodeGen, TypedValue};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    /// LazyList take_count sentinel: takeWhile mode (filter_fn holds predicate; stop on first false).
    const LAZY_TAKE_WHILE_TC: i64 = -2;

    /// Store takeWhile predicate on a LazyList without materializing (deferred in `toList`).
    pub(crate) fn lazy_take_while_impl(
        &mut self,
        pred_fn_ptr: inkwell::values::PointerValue<'ctx>,
        ll_ptr: inkwell::values::PointerValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, ll_ptr, "ltw_ll")
            .map_err(llvm_err)?
            .into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_sv, 0, "ltw_head")
            .map_err(llvm_err)?;
        let step_fn = self
            .builder
            .build_extract_value(ll_sv, 1, "ltw_sf")
            .map_err(llvm_err)?;
        let state_val = self
            .builder
            .build_extract_value(ll_sv, 2, "ltw_st")
            .map_err(llvm_err)?;
        let map_fn = self
            .builder
            .build_extract_value(ll_sv, 4, "ltw_map")
            .map_err(llvm_err)?;
        let take_while_tc = self
            .i64_ty()
            .const_int(Self::LAZY_TAKE_WHILE_TC as u64, true);
        let result_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "ltw_lazy")
            .map_err(llvm_err)?;
        let v0 = self
            .builder
            .build_insert_value(ll_sv, head_val, 0, "ltw_v0")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn, 1, "ltw_v1")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state_val, 2, "ltw_v2")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, take_while_tc, 3, "ltw_v3")
            .map_err(llvm_err)?;
        let v4 = self
            .builder
            .build_insert_value(v3, map_fn, 4, "ltw_v4")
            .map_err(llvm_err)?;
        let v5 = self
            .builder
            .build_insert_value(v4, pred_fn_ptr, 5, "ltw_v5")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, v5)
            .map_err(llvm_err)?;
        Ok(TypedValue::LazyList(result_alloca))
    }

    /// Fused lazy `.filter{}.map{}`: compose deferred filter+map without eager materialization.
    pub(crate) fn fused_lazy_filter_map_hir(
        &mut self,
        filter_fn: &action_frontend::hir::HirExpr,
        inner: &action_frontend::hir::HirExpr,
        map_fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let filter_ptr = match self.compile_hir_expr(filter_fn)? {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("lazy filter+map: filter function required".to_string()),
        };
        let map_ptr = match map_fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("lazy filter+map: map function required".to_string()),
        };
        let inner_val = self.compile_hir_expr(inner)?;
        let ll_ptr = match inner_val {
            TypedValue::LazyList(p) => p,
            _ => return Err("lazy filter+map: LazyList receiver required".to_string()),
        };
        let filtered = self.lazy_filter_impl(filter_ptr, ll_ptr)?;
        let filtered_ptr = match filtered {
            TypedValue::LazyList(p) => p,
            _ => return Err("lazy filter+map: filter did not return LazyList".to_string()),
        };
        self.lazy_map_impl(map_ptr, filtered_ptr)
    }

    pub(crate) fn builtin_lazy_take_while_values(
        &mut self,
        fn_val: TypedValue<'ctx>,
        lazy_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, _) = match fn_val {
            TypedValue::Fn(p, _) => (p, fn_val),
            _ => return Err("lazyTakeWhile: first argument must be a function".to_string()),
        };
        if let TypedValue::LazyList(ll_ptr) = lazy_val {
            return self.lazy_take_while_impl(fn_ptr, ll_ptr);
        }
        let lazy_ptr = self.ensure_list_ptr(&lazy_val, "ltw")?;
        let list = self.load_list(lazy_ptr)?;
        let len = self
            .builder
            .build_extract_value(list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let data = self
            .builder
            .build_extract_value(list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();

        let cc = self.call_rt("action_list_create", &[len.into()])?;
        let new_list = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "ltw_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_list)
            .map_err(llvm_err)?;

        let i64 = self.i64_ty();
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let i_alloca = self.builder.build_alloca(i64, "ltw_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_hdr = self.context.append_basic_block(current_fn, "ltw_hdr");
        let loop_bdy = self.context.append_basic_block(current_fn, "ltw_bdy");
        let loop_ins = self.context.append_basic_block(current_fn, "ltw_ins");
        let loop_ext = self.context.append_basic_block(current_fn, "ltw_ext");

        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_hdr);
        let i = self
            .builder
            .build_load(i64, i_alloca, "ltw_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i, len, "ltw_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_bdy, loop_ext);

        self.builder.position_at_end(loop_bdy);
        let src_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, data, &[i], "ltw_sp")
                .map_err(llvm_err)
        }?;
        let elem = self
            .builder
            .build_load(self.string_type, src_ptr, "ltw_el")
            .map_err(llvm_err)?
            .into_struct_value();
        let tag = self
            .builder
            .build_extract_value(elem, 0, "ltw_tag")
            .map_err(llvm_err)?
            .into_int_value();

        let fat_ty = self.string_type;
        let lam_fn_type = fat_ty.fn_type(&[i64.into()], false);
        let cc = self
            .builder
            .build_indirect_call(lam_fn_type, fn_ptr, &[tag.into()], "ltw_call")
            .map_err(llvm_err)?;
        let pred_bv = cc.try_as_basic_value().basic().ok_or("ltw call failed")?;
        let pred_tag = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let keep = self
            .builder
            .build_int_compare(IntPredicate::NE, pred_tag, i64.const_int(0, false), "keep")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(keep, loop_ins, loop_ext);

        self.builder.position_at_end(loop_ins);
        let cur = self.load_list(result_alloca)?;
        let pcc = self.call_rt("action_list_push", &[cur.into(), elem.into()])?;
        let nl = pcc.try_as_basic_value().basic().ok_or("list_push failed")?;
        self.builder
            .build_store(result_alloca, nl)
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(i, i64.const_int(1, false), "ltw_ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_ext);
        Ok(TypedValue::List(result_alloca))
    }
}
