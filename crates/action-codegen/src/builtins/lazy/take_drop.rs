use crate::{llvm_err, CodeGen, TypedValue};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_lazy_take_values(
        &mut self,
        n_val: TypedValue<'ctx>,
        lazy_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let n = match n_val {
            TypedValue::Int(v) => v,
            _ => return Err("lazyTake: first argument must be an Int".to_string()),
        };
        let lazy_ptr = match &lazy_val {
            TypedValue::LazyList(p) => *p,
            _ => return Err("lazyTake: second argument must be a LazyList".to_string()),
        };
        // Load the LazyList struct, copy it with updated take_count
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, lazy_ptr, "lt_ll")
            .map_err(llvm_err)?
            .into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_sv, 0, "lt_head")
            .map_err(llvm_err)?;
        let step_fn = self
            .builder
            .build_extract_value(ll_sv, 1, "lt_fn")
            .map_err(llvm_err)?;
        let state_val = self
            .builder
            .build_extract_value(ll_sv, 2, "lt_st")
            .map_err(llvm_err)?;
        let map_fn = self
            .builder
            .build_extract_value(ll_sv, 4, "lt_map")
            .map_err(llvm_err)?;
        let filter_fn = self
            .builder
            .build_extract_value(ll_sv, 5, "lt_filt")
            .map_err(llvm_err)?;

        let result_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "lt_result")
            .map_err(llvm_err)?;
        let undef = self.lazylist_type.get_undef();
        let v0 = self
            .builder
            .build_insert_value(undef, head_val, 0, "lt_h")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn, 1, "lt_f")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state_val, 2, "lt_s")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, n, 3, "lt_n")
            .map_err(llvm_err)?;
        let v4 = self
            .builder
            .build_insert_value(v3, map_fn, 4, "lt_map")
            .map_err(llvm_err)?;
        let v5 = self
            .builder
            .build_insert_value(v4, filter_fn, 5, "lt_filt")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, v5)
            .map_err(llvm_err)?;
        Ok(TypedValue::LazyList(result_alloca))
    }

    pub(crate) fn builtin_lazy_drop_values(
        &mut self,
        n_val: TypedValue<'ctx>,
        lazy_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let n = match n_val {
            TypedValue::Int(v) => v,
            _ => return Err("lazyDrop: first argument must be an Int".to_string()),
        };
        let lazy_ptr = match &lazy_val {
            TypedValue::LazyList(p) => *p,
            _ => return Err("lazyDrop: second argument must be a LazyList".to_string()),
        };

        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, lazy_ptr, "ld_ll")
            .map_err(llvm_err)?
            .into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_sv, 0, "ld_head")
            .map_err(llvm_err)?
            .into_int_value();
        let step_fn = self
            .builder
            .build_extract_value(ll_sv, 1, "ld_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let state_val = self
            .builder
            .build_extract_value(ll_sv, 2, "ld_st")
            .map_err(llvm_err)?
            .into_int_value();
        let take_count_val = self
            .builder
            .build_extract_value(ll_sv, 3, "ld_tc")
            .map_err(llvm_err)?
            .into_int_value();
        let map_fn = self
            .builder
            .build_extract_value(ll_sv, 4, "ld_map")
            .map_err(llvm_err)?
            .into_pointer_value();
        let filter_fn = self
            .builder
            .build_extract_value(ll_sv, 5, "ld_filt")
            .map_err(llvm_err)?
            .into_pointer_value();

        let zero = self.i64_ty().const_int(0, false);
        let one = self.i64_ty().const_int(1, false);
        let neg_one = self.i64_ty().const_int((-1_i64) as u64, true);

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;

        // Determine if list-backed (no step fn, state holds data ptr)
        let has_step = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                step_fn,
                self.ptr_ty().const_null(),
                "ld_has_step",
            )
            .map_err(llvm_err)?;
        let state_nz = self
            .builder
            .build_int_compare(IntPredicate::NE, state_val, zero, "ld_state_nz")
            .map_err(llvm_err)?;
        let not_has_step = self
            .builder
            .build_not(has_step, "ld_not_step")
            .map_err(llvm_err)?;
        let is_list_backed = self
            .builder
            .build_and(not_has_step, state_nz, "ld_is_lb")
            .map_err(llvm_err)?;

        // Check if n >= take_count (result is empty)
        let tc_is_inf = self
            .builder
            .build_int_compare(IntPredicate::EQ, take_count_val, neg_one, "ld_tc_inf")
            .map_err(llvm_err)?;
        let n_ge_tc = self
            .builder
            .build_int_compare(IntPredicate::SGE, n, take_count_val, "ld_n_ge_tc")
            .map_err(llvm_err)?;
        let not_inf = self
            .builder
            .build_not(tc_is_inf, "ld_not_inf")
            .map_err(llvm_err)?;
        let becomes_empty = self
            .builder
            .build_and(not_inf, n_ge_tc, "ld_empty")
            .map_err(llvm_err)?;

        // Branch: empty? → fast path; otherwise → drop path
        let empty_block = self.context.append_basic_block(current_fn, "ld_empty");
        let drop_block = self.context.append_basic_block(current_fn, "ld_drop");
        let merge_block = self.context.append_basic_block(current_fn, "ld_merge");
        let _ = self
            .builder
            .build_conditional_branch(becomes_empty, empty_block, drop_block);

        // Empty result: head=0, no step fn, state=0, tc=0, keep map/filter (won't matter)
        self.builder.position_at_end(empty_block);
        let e_result = self
            .builder
            .build_alloca(self.lazylist_type, "ld_e_result")
            .map_err(llvm_err)?;
        let e_undef = self.lazylist_type.get_undef();
        let e0 = self
            .builder
            .build_insert_value(e_undef, zero, 0, "e_h")
            .map_err(llvm_err)?;
        let e1 = self
            .builder
            .build_insert_value(e0, self.ptr_ty().const_null(), 1, "e_fn")
            .map_err(llvm_err)?;
        let e2 = self
            .builder
            .build_insert_value(e1, zero, 2, "e_st")
            .map_err(llvm_err)?;
        let e3 = self
            .builder
            .build_insert_value(e2, zero, 3, "e_tc")
            .map_err(llvm_err)?;
        let e4 = self
            .builder
            .build_insert_value(e3, self.ptr_ty().const_null(), 4, "e_map")
            .map_err(llvm_err)?;
        let e5 = self
            .builder
            .build_insert_value(e4, self.ptr_ty().const_null(), 5, "e_filt")
            .map_err(llvm_err)?;
        self.builder.build_store(e_result, e5).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Drop path: advance head/state by n
        self.builder.position_at_end(drop_block);

        // Branch on list-backed vs step-function
        let lb_drop_block = self.context.append_basic_block(current_fn, "ld_lb_drop");
        let step_drop_block = self.context.append_basic_block(current_fn, "ld_step_drop");
        let drop_merge_block = self.context.append_basic_block(current_fn, "ld_drop_merge");
        let _ =
            self.builder
                .build_conditional_branch(is_list_backed, lb_drop_block, step_drop_block);

        // List-backed drop: advance data ptr by n elements, load new head
        self.builder.position_at_end(lb_drop_block);
        let data_ptr = self
            .builder
            .build_int_to_ptr(state_val, self.ptr_ty(), "ld_dp")
            .map_err(llvm_err)?;
        let new_data_gep = unsafe {
            self.builder
                .build_gep(self.fat_return_type, data_ptr, &[n], "ld_ndp")
                .map_err(llvm_err)
        }?;
        let new_data_i64 = self
            .builder
            .build_ptr_to_int(new_data_gep, self.i64_ty(), "ld_ndp_i64")
            .map_err(llvm_err)?;
        let new_head_fat = self
            .builder
            .build_load(self.fat_return_type, new_data_gep, "ld_nh_fat")
            .map_err(llvm_err)?
            .into_struct_value();
        let new_head = self
            .builder
            .build_extract_value(new_head_fat, 0, "ld_nh")
            .map_err(llvm_err)?
            .into_int_value();
        let new_tc_lb = self
            .builder
            .build_int_sub(take_count_val, n, "ld_new_tc_lb")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(drop_merge_block);

        // Step-function drop: call step_fn n times to advance
        self.builder.position_at_end(step_drop_block);
        let i_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ld_i")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, zero).map_err(llvm_err)?;
        let cur_state_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ld_cs")
            .map_err(llvm_err)?;
        self.builder
            .build_store(cur_state_alloca, state_val)
            .map_err(llvm_err)?;
        let cur_head_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ld_ch")
            .map_err(llvm_err)?;
        self.builder
            .build_store(cur_head_alloca, head_val)
            .map_err(llvm_err)?;

        let step_loop_hdr = self.context.append_basic_block(current_fn, "ld_step_hdr");
        let step_loop_body = self.context.append_basic_block(current_fn, "ld_step_body");
        let step_done = self.context.append_basic_block(current_fn, "ld_step_done");
        let _ = self.builder.build_unconditional_branch(step_loop_hdr);

        self.builder.position_at_end(step_loop_hdr);
        let i_val = self
            .builder
            .build_load(self.i64_ty(), i_alloca, "ld_i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let i_lt_n = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, n, "ld_i_lt_n")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(i_lt_n, step_loop_body, step_done);

        self.builder.position_at_end(step_loop_body);
        let cs = self
            .builder
            .build_load(self.i64_ty(), cur_state_alloca, "ld_cs_val")
            .map_err(llvm_err)?
            .into_int_value();
        let fn_type = self.fat_return_type.fn_type(&[self.i64_ty().into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, step_fn, &[cs.into()], "ld_step_call")
            .map_err(llvm_err)?;
        let step_fat = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("step call returned void")?;
        let step_fat_sv = step_fat.into_struct_value();
        let next_val = self
            .builder
            .build_extract_value(step_fat_sv, 0, "ld_next")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(cur_state_alloca, next_val)
            .map_err(llvm_err)?;
        self.builder
            .build_store(cur_head_alloca, next_val)
            .map_err(llvm_err)?;
        let new_i = self
            .builder
            .build_int_add(i_val, one, "ld_ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, new_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(step_loop_hdr);

        self.builder.position_at_end(step_done);
        let new_head_step = self
            .builder
            .build_load(self.i64_ty(), cur_head_alloca, "ld_nh_step")
            .map_err(llvm_err)?
            .into_int_value();
        let new_state = self
            .builder
            .build_load(self.i64_ty(), cur_state_alloca, "ld_ns")
            .map_err(llvm_err)?
            .into_int_value();
        let new_tc_step = self
            .builder
            .build_select(
                tc_is_inf,
                take_count_val,
                self.builder
                    .build_int_sub(take_count_val, n, "ld_tc_sub")
                    .map_err(llvm_err)?,
                "ld_new_tc_step",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(drop_merge_block);

        // Drop merge: phi for new head, new state, new take_count
        self.builder.position_at_end(drop_merge_block);
        let d_nh_phi = self
            .builder
            .build_phi(self.i64_ty(), "ld_nh_phi")
            .map_err(llvm_err)?;
        d_nh_phi.add_incoming(&[(&new_head, lb_drop_block), (&new_head_step, step_done)]);
        let d_ns_phi = self
            .builder
            .build_phi(self.i64_ty(), "ld_ns_phi")
            .map_err(llvm_err)?;
        d_ns_phi.add_incoming(&[(&new_data_i64, lb_drop_block), (&new_state, step_done)]);
        let d_ntc_phi = self
            .builder
            .build_phi(self.i64_ty(), "ld_ntc_phi")
            .map_err(llvm_err)?;
        d_ntc_phi.add_incoming(&[(&new_tc_lb, lb_drop_block), (&new_tc_step, step_done)]);

        let d_result = self
            .builder
            .build_alloca(self.lazylist_type, "ld_d_result")
            .map_err(llvm_err)?;
        let d_undef = self.lazylist_type.get_undef();
        let d0 = self
            .builder
            .build_insert_value(
                d_undef,
                d_nh_phi.as_basic_value().into_int_value(),
                0,
                "d_h",
            )
            .map_err(llvm_err)?;
        let d1 = self
            .builder
            .build_insert_value(d0, step_fn, 1, "d_fn")
            .map_err(llvm_err)?;
        let d2 = self
            .builder
            .build_insert_value(d1, d_ns_phi.as_basic_value().into_int_value(), 2, "d_st")
            .map_err(llvm_err)?;
        let d3 = self
            .builder
            .build_insert_value(d2, d_ntc_phi.as_basic_value().into_int_value(), 3, "d_tc")
            .map_err(llvm_err)?;
        let d4 = self
            .builder
            .build_insert_value(d3, map_fn, 4, "d_map")
            .map_err(llvm_err)?;
        let d5 = self
            .builder
            .build_insert_value(d4, filter_fn, 5, "d_filt")
            .map_err(llvm_err)?;
        self.builder.build_store(d_result, d5).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Final merge: phi for the result LazyList pointer
        self.builder.position_at_end(merge_block);
        let m_phi = self
            .builder
            .build_phi(self.ptr_ty(), "ld_m_phi")
            .map_err(llvm_err)?;
        m_phi.add_incoming(&[(&e_result, empty_block), (&d_result, drop_merge_block)]);
        let result_ptr = m_phi.as_basic_value().into_pointer_value();
        Ok(TypedValue::LazyList(result_ptr))
    }
}
