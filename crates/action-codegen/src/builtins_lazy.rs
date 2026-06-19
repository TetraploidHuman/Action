// Submodule: builtins_lazy

use inkwell::types::BasicTypeEnum;
use inkwell::IntPredicate;

use super::call_arg::CallArg;
use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn builtin_lazy_take_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let n_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_take_values(n_val, lazy_val)
    }

    pub(super) fn builtin_lazy_drop_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let n_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_drop_values(n_val, lazy_val)
    }

    pub(super) fn builtin_lazy_map_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_map_values(fn_val, lazy_val)
    }

    pub(super) fn builtin_lazy_filter_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_filter_values(fn_val, lazy_val)
    }

    pub(super) fn builtin_lazy_take_while_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_take_while_values(fn_val, lazy_val)
    }

    pub(super) fn builtin_lazy_head_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let lazy_val = self.compile_call_arg(arg)?;
        self.builtin_lazy_head_value(lazy_val)
    }

    pub(super) fn builtin_lazy_zip_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v1 = self.compile_call_arg(a)?;
        let v2 = self.compile_call_arg(b)?;
        self.builtin_lazy_zip_values(v1, v2)
    }

    pub(super) fn builtin_lazy_take_values(
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

    pub(super) fn builtin_lazy_drop_values(
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

    pub(super) fn builtin_lazy_map_values(
        &mut self,
        fn_val: TypedValue<'ctx>,
        lazy_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (map_fn_ptr, _fn_type) = match fn_val {
            TypedValue::Fn(p, ft) => (p, ft),
            _ => return Err("lazyMap: first argument must be a function".to_string()),
        };
        match &lazy_val {
            TypedValue::LazyList(ll_ptr) => self.lazy_map_impl(map_fn_ptr, *ll_ptr),
            TypedValue::List(_) => {
                let ll_val = self.builtin_to_lazy_list_value(lazy_val.clone())?;
                match ll_val {
                    TypedValue::LazyList(ll_ptr) => self.lazy_map_impl(map_fn_ptr, ll_ptr),
                    _ => Err("lazyMap: toLazyList did not return LazyList".to_string()),
                }
            }
            _ => Err("lazyMap: second argument must be a LazyList or List".to_string()),
        }
    }

    /// lazy_map_impl: store map_fn in the LazyList for deferred application during toList()
    pub(super) fn lazy_map_impl(
        &mut self,
        map_fn_ptr: inkwell::values::PointerValue<'ctx>,
        ll_ptr: inkwell::values::PointerValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, ll_ptr, "lm_ll")
            .map_err(llvm_err)?
            .into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_sv, 0, "lm_head")
            .map_err(llvm_err)?;
        let step_fn = self
            .builder
            .build_extract_value(ll_sv, 1, "lm_sf")
            .map_err(llvm_err)?;
        let state_val = self
            .builder
            .build_extract_value(ll_sv, 2, "lm_st")
            .map_err(llvm_err)?;
        let take_count = self
            .builder
            .build_extract_value(ll_sv, 3, "lm_tc")
            .map_err(llvm_err)?;
        let old_map_fn = self
            .builder
            .build_extract_value(ll_sv, 4, "lm_old_map")
            .map_err(llvm_err)?;
        let filter_fn = self
            .builder
            .build_extract_value(ll_sv, 5, "lm_filt")
            .map_err(llvm_err)?;

        // Compose with existing map_fn if present
        let has_old_map = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                old_map_fn.into_pointer_value(),
                self.ptr_ty().const_null(),
                "has_old_map",
            )
            .map_err(llvm_err)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let compose_block = self.context.append_basic_block(current_fn, "lm_compose");
        let no_compose_block = self.context.append_basic_block(current_fn, "lm_no_compose");
        let merge_block = self.context.append_basic_block(current_fn, "lm_merge");

        let _ = self
            .builder
            .build_conditional_branch(has_old_map, compose_block, no_compose_block);

        // Compose: new_fn(x) = map_fn_ptr(old_map_fn(x))
        self.builder.position_at_end(compose_block);
        let wrapper_name = format!("lm_compose_{}", self.wrapper_counter);
        self.wrapper_counter += 1;
        let fat_ty = self.string_type;
        let wrapper_fn = self.module.add_function(
            &wrapper_name,
            fat_ty.fn_type(&[self.i64_ty().into()], false),
            None,
        );
        let wrapper_entry = self.context.append_basic_block(wrapper_fn, "entry");
        let saved_block = self.builder.get_insert_block();

        let cap_ty = self
            .context
            .struct_type(&[self.ptr_ty().into(), self.ptr_ty().into()], false);
        let cap_global = self.add_module_global(cap_ty, &format!("{}_cap", wrapper_name))?;
        cap_global.set_initializer(&cap_ty.const_zero());
        let cap_ptr = cap_global.as_pointer_value();
        let c_gep0 = self
            .builder
            .build_struct_gep(cap_ty, cap_ptr, 0, "cg0")
            .map_err(llvm_err)?;
        self.builder
            .build_store(c_gep0, old_map_fn)
            .map_err(llvm_err)?;
        let c_gep1 = self
            .builder
            .build_struct_gep(cap_ty, cap_ptr, 1, "cg1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(c_gep1, map_fn_ptr)
            .map_err(llvm_err)?;

        self.builder.position_at_end(wrapper_entry);
        let w_state = wrapper_fn.get_first_param().unwrap().into_int_value();
        let cap_load = self
            .builder
            .build_load(cap_ty, cap_ptr, "cap_load")
            .map_err(llvm_err)?
            .into_struct_value();
        let w_old_fn = self
            .builder
            .build_extract_value(cap_load, 0, "w_old")
            .map_err(llvm_err)?
            .into_pointer_value();
        let w_new_fn = self
            .builder
            .build_extract_value(cap_load, 1, "w_new")
            .map_err(llvm_err)?
            .into_pointer_value();
        let map_fn_type = fat_ty.fn_type(&[self.i64_ty().into()], false);
        let old_call = self
            .builder
            .build_indirect_call(map_fn_type, w_old_fn, &[w_state.into()], "w_old_call")
            .map_err(llvm_err)?;
        let old_result = old_call
            .try_as_basic_value()
            .basic()
            .ok_or("old call failed")?;
        let old_val = if old_result.is_struct_value() {
            self.builder
                .build_extract_value(old_result.into_struct_value(), 0, "w_old_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            old_result.into_int_value()
        };
        let new_call = self
            .builder
            .build_indirect_call(map_fn_type, w_new_fn, &[old_val.into()], "w_new_call")
            .map_err(llvm_err)?;
        let new_result = new_call
            .try_as_basic_value()
            .basic()
            .ok_or("new call failed")?;
        self.builder
            .build_return(Some(&new_result))
            .map_err(llvm_err)?;

        self.builder.position_at_end(saved_block.unwrap());
        let composed_fn = wrapper_fn.as_global_value().as_pointer_value();
        let _ = self.builder.build_unconditional_branch(merge_block);

        self.builder.position_at_end(no_compose_block);
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Merge: pick the right map_fn
        self.builder.position_at_end(merge_block);
        let phi_map = self
            .builder
            .build_phi(self.ptr_ty(), "lm_phi_map")
            .map_err(llvm_err)?;
        phi_map.add_incoming(&[
            (&composed_fn, compose_block),
            (&map_fn_ptr, no_compose_block),
        ]);

        // Build result LazyList with updated map_fn, head unchanged (deferred mapping in toList)
        let result_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "lm_result")
            .map_err(llvm_err)?;
        let undef = self.lazylist_type.get_undef();
        let v0 = self
            .builder
            .build_insert_value(undef, head_val, 0, "lm_h")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn, 1, "lm_f")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state_val, 2, "lm_s")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, take_count, 3, "lm_t")
            .map_err(llvm_err)?;
        let v4 = self
            .builder
            .build_insert_value(
                v3,
                phi_map.as_basic_value().into_pointer_value(),
                4,
                "lm_map",
            )
            .map_err(llvm_err)?;
        let v5 = self
            .builder
            .build_insert_value(v4, filter_fn, 5, "lm_filt")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, v5)
            .map_err(llvm_err)?;
        Ok(TypedValue::LazyList(result_alloca))
    }

    pub(super) fn builtin_lazy_filter_values(
        &mut self,
        fn_val: TypedValue<'ctx>,
        lazy_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (filter_fn_ptr, _) = match fn_val {
            TypedValue::Fn(p, _) => (p, fn_val),
            _ => return Err("lazyFilter: first argument must be a function".to_string()),
        };
        match &lazy_val {
            TypedValue::LazyList(ll_ptr) => self.lazy_filter_impl(filter_fn_ptr, *ll_ptr),
            TypedValue::List(_) => {
                let ll_val = self.builtin_to_lazy_list_value(lazy_val.clone())?;
                match ll_val {
                    TypedValue::LazyList(ll_ptr) => self.lazy_filter_impl(filter_fn_ptr, ll_ptr),
                    _ => Err("lazyFilter: toLazyList did not return LazyList".to_string()),
                }
            }
            _ => Err("lazyFilter: second argument must be a LazyList or List".to_string()),
        }
    }

    /// lazy_filter_impl: store filter_fn in the LazyList for deferred application during toList()
    pub(super) fn lazy_filter_impl(
        &mut self,
        filter_fn_ptr: inkwell::values::PointerValue<'ctx>,
        ll_ptr: inkwell::values::PointerValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, ll_ptr, "lf_ll")
            .map_err(llvm_err)?
            .into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_sv, 0, "lf_head")
            .map_err(llvm_err)?;
        let step_fn = self
            .builder
            .build_extract_value(ll_sv, 1, "lf_sf")
            .map_err(llvm_err)?;
        let state_val = self
            .builder
            .build_extract_value(ll_sv, 2, "lf_st")
            .map_err(llvm_err)?;
        let take_count = self
            .builder
            .build_extract_value(ll_sv, 3, "lf_tc")
            .map_err(llvm_err)?;
        let map_fn = self
            .builder
            .build_extract_value(ll_sv, 4, "lf_map")
            .map_err(llvm_err)?;
        let old_filter_fn = self
            .builder
            .build_extract_value(ll_sv, 5, "lf_old_filt")
            .map_err(llvm_err)?;

        // Compose filters if there's already a filter_fn
        let has_old_filter = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                old_filter_fn.into_pointer_value(),
                self.ptr_ty().const_null(),
                "has_old_filt",
            )
            .map_err(llvm_err)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let compose_block = self.context.append_basic_block(current_fn, "lf_compose");
        let no_compose_block = self.context.append_basic_block(current_fn, "lf_no_compose");
        let merge_block = self.context.append_basic_block(current_fn, "lf_merge");

        let _ =
            self.builder
                .build_conditional_branch(has_old_filter, compose_block, no_compose_block);

        // Compose: new_filter(x) = old_filter(x) && new_filter(x)
        self.builder.position_at_end(compose_block);
        let wrapper_name = format!("lf_compose_{}", self.wrapper_counter);
        self.wrapper_counter += 1;
        let fat_ty = self.string_type;
        let wrapper_fn = self.module.add_function(
            &wrapper_name,
            fat_ty.fn_type(&[self.i64_ty().into()], false),
            None,
        );
        let wrapper_entry = self.context.append_basic_block(wrapper_fn, "entry");
        let saved_block = self.builder.get_insert_block();

        // Capture both filter functions via global
        let cap_ty = self
            .context
            .struct_type(&[self.ptr_ty().into(), self.ptr_ty().into()], false);
        let cap_global = self.add_module_global(cap_ty, &format!("{}_cap", wrapper_name))?;
        cap_global.set_initializer(&cap_ty.const_zero());
        let cap_ptr = cap_global.as_pointer_value();
        let c_gep0 = self
            .builder
            .build_struct_gep(cap_ty, cap_ptr, 0, "cg0")
            .map_err(llvm_err)?;
        self.builder
            .build_store(c_gep0, old_filter_fn)
            .map_err(llvm_err)?;
        let c_gep1 = self
            .builder
            .build_struct_gep(cap_ty, cap_ptr, 1, "cg1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(c_gep1, filter_fn_ptr)
            .map_err(llvm_err)?;

        self.builder.position_at_end(wrapper_entry);
        let w_state = wrapper_fn.get_first_param().unwrap().into_int_value();
        let cap_load = self
            .builder
            .build_load(cap_ty, cap_ptr, "cap_load")
            .map_err(llvm_err)?
            .into_struct_value();
        let w_old_fn = self
            .builder
            .build_extract_value(cap_load, 0, "w_old")
            .map_err(llvm_err)?
            .into_pointer_value();
        let w_new_fn = self
            .builder
            .build_extract_value(cap_load, 1, "w_new")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Call old_filter(state)
        let filt_fn_type = fat_ty.fn_type(&[self.i64_ty().into()], false);
        let old_call = self
            .builder
            .build_indirect_call(filt_fn_type, w_old_fn, &[w_state.into()], "w_old_call")
            .map_err(llvm_err)?;
        let old_result = old_call
            .try_as_basic_value()
            .basic()
            .ok_or("old filt call failed")?;
        let old_val = if old_result.is_struct_value() {
            self.builder
                .build_extract_value(old_result.into_struct_value(), 0, "w_old_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            old_result.into_int_value()
        };
        let old_true = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                old_val,
                self.i64_ty().const_int(0, false),
                "old_true",
            )
            .map_err(llvm_err)?;

        let then_block = self.context.append_basic_block(wrapper_fn, "then_call");
        let else_block = self.context.append_basic_block(wrapper_fn, "else_zero");
        let w_merge = self.context.append_basic_block(wrapper_fn, "w_merge");
        let _ = self
            .builder
            .build_conditional_branch(old_true, then_block, else_block);

        self.builder.position_at_end(then_block);
        let new_call = self
            .builder
            .build_indirect_call(filt_fn_type, w_new_fn, &[w_state.into()], "w_new_call")
            .map_err(llvm_err)?;
        let new_result = new_call
            .try_as_basic_value()
            .basic()
            .ok_or("new filt call failed")?;
        let new_val = if new_result.is_struct_value() {
            self.builder
                .build_extract_value(new_result.into_struct_value(), 0, "w_new_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            new_result.into_int_value()
        };
        let new_true = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                new_val,
                self.i64_ty().const_int(0, false),
                "new_true",
            )
            .map_err(llvm_err)?;
        let new_i64 = self
            .builder
            .build_int_z_extend(new_true, self.i64_ty(), "new_i64")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(w_merge);

        self.builder.position_at_end(else_block);
        let _ = self.builder.build_unconditional_branch(w_merge);

        self.builder.position_at_end(w_merge);
        let phi = self
            .builder
            .build_phi(self.i64_ty(), "filt_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[
            (&new_i64, then_block),
            (&self.i64_ty().const_int(0, false), else_block),
        ]);
        // Return as fat struct {i64, i8*}
        let undef_ret = fat_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef_ret, phi.as_basic_value().into_int_value(), 0, "fr_v")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, self.ptr_ty().const_null(), 1, "fr_p")
            .map_err(llvm_err)?;
        self.builder.build_return(Some(&r2)).map_err(llvm_err)?;

        self.builder.position_at_end(saved_block.unwrap());
        let composed_fn = wrapper_fn.as_global_value().as_pointer_value();
        let _ = self.builder.build_unconditional_branch(merge_block);

        // No composition needed
        self.builder.position_at_end(no_compose_block);
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Merge: pick the right filter_fn
        self.builder.position_at_end(merge_block);
        let phi_filt = self
            .builder
            .build_phi(self.ptr_ty(), "lf_phi_filt")
            .map_err(llvm_err)?;
        phi_filt.add_incoming(&[
            (&composed_fn, compose_block),
            (&filter_fn_ptr, no_compose_block),
        ]);

        let result_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "lf_result")
            .map_err(llvm_err)?;
        let undef = self.lazylist_type.get_undef();
        let v0 = self
            .builder
            .build_insert_value(undef, head_val, 0, "lf_h")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn, 1, "lf_f")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state_val, 2, "lf_s")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, take_count, 3, "lf_t")
            .map_err(llvm_err)?;
        let v4 = self
            .builder
            .build_insert_value(v3, map_fn, 4, "lf_map")
            .map_err(llvm_err)?;
        let v5 = self
            .builder
            .build_insert_value(
                v4,
                phi_filt.as_basic_value().into_pointer_value(),
                5,
                "lf_filt",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, v5)
            .map_err(llvm_err)?;
        Ok(TypedValue::LazyList(result_alloca))
    }

    /// LazyList take_count sentinel: takeWhile mode (filter_fn holds predicate; stop on first false).
    const LAZY_TAKE_WHILE_TC: i64 = -2;

    /// Store takeWhile predicate on a LazyList without materializing (deferred in `toList`).
    pub(super) fn lazy_take_while_impl(
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
    pub(super) fn fused_lazy_filter_map_hir(
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

    pub(super) fn builtin_lazy_take_while_values(
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

    pub(super) fn builtin_lazy_head_value(
        &mut self,
        lazy_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (head_val, is_empty) = match &lazy_val {
            TypedValue::LazyList(ptr) => {
                let ll_sv = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "head_ll")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let h = self
                    .builder
                    .build_extract_value(ll_sv, 0, "head_h")
                    .map_err(llvm_err)?;
                // Check take_count (field 3): 0 = empty, != 0 = has elements
                let take_count = self
                    .builder
                    .build_extract_value(ll_sv, 3, "head_tc")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_empty = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        take_count,
                        self.i64_ty().const_int(0, false),
                        "ll_is_empty",
                    )
                    .map_err(llvm_err)?;
                (h, is_empty)
            }
            TypedValue::List(ptr) => {
                let list = self.load_list(*ptr)?;
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
                let zero = self.i64_ty().const_int(0, false);
                let is_empty_cond = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "is_empty")
                    .map_err(llvm_err)?;
                // Load first element's fat struct
                let first_ptr = unsafe {
                    self.builder
                        .build_gep(self.fat_return_type, data, &[zero], "head_gep")
                        .map_err(llvm_err)
                }?;
                let first_fat = self
                    .builder
                    .build_load(self.fat_return_type, first_ptr, "head_fat")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let h = self
                    .builder
                    .build_extract_value(first_fat, 0, "head_h")
                    .map_err(llvm_err)?;
                (h, is_empty_cond)
            }
            _ => return Err("lazyHead: argument must be a LazyList or List".to_string()),
        };

        let i64 = self.i64_ty();

        // Create nullable {i1, i64} for nullable Int
        let nullable_ty = self.get_nullable_type(i64.into(), "Nullable<Int>");
        let null_bt: BasicTypeEnum = nullable_ty.into();

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;

        let result_alloca = self
            .builder
            .build_alloca(nullable_ty, "lh_result")
            .map_err(llvm_err)?;

        let merge_block = self.context.append_basic_block(current_fn, "lh_merge");
        let some_block = self.context.append_basic_block(current_fn, "lh_some");
        let none_block = self.context.append_basic_block(current_fn, "lh_none");

        let _ = self
            .builder
            .build_conditional_branch(is_empty, none_block, some_block);

        // Some branch: head_val contains the i64 value
        self.builder.position_at_end(some_block);
        let head_i64 = head_val.into_int_value();

        // Build nullable {flag=0, value} — inline, no heap allocation
        let undef = nullable_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(
                undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "lh_some_flag",
            )
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, head_i64, 1, "lh_some_val")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, r2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        // None branch: nullable {flag=1, undef}
        self.builder.position_at_end(none_block);
        let undef2 = nullable_ty.get_undef();
        let n1 = self
            .builder
            .build_insert_value(
                undef2,
                self.null_flag_ty().const_int(1, false),
                0,
                "lh_none_flag",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, n1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        self.builder.position_at_end(merge_block);
        Ok(TypedValue::Nullable(result_alloca, null_bt))
    }

    pub(super) fn builtin_lazy_zip_values(
        &mut self,
        v1: TypedValue<'ctx>,
        v2: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let p1 = self.ensure_list_ptr(&v1, "lz1")?;
        let p2 = self.ensure_list_ptr(&v2, "lz2")?;
        let l1 = self.load_list(p1)?;
        let l2 = self.load_list(p2)?;
        let len1 = self
            .builder
            .build_extract_value(l1, 1, "lz_len1")
            .map_err(llvm_err)?
            .into_int_value();
        let len2 = self
            .builder
            .build_extract_value(l2, 1, "lz_len2")
            .map_err(llvm_err)?
            .into_int_value();
        let d1 = self
            .builder
            .build_extract_value(l1, 0, "lz_d1")
            .map_err(llvm_err)?
            .into_pointer_value();
        let d2 = self
            .builder
            .build_extract_value(l2, 0, "lz_d2")
            .map_err(llvm_err)?
            .into_pointer_value();

        let i64 = self.i64_ty();
        let is_len1_lt_len2 = self
            .builder
            .build_int_compare(IntPredicate::SLT, len1, len2, "is_len1_lt_len2")
            .map_err(llvm_err)?;
        let min_len = self
            .builder
            .build_select(is_len1_lt_len2, len1, len2, "lz_min")
            .map_err(llvm_err)?
            .into_int_value();

        let cc = self.call_rt("action_list_create", &[min_len.into()])?;
        let new_list = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "lz_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_list)
            .map_err(llvm_err)?;

        // Zip elements as tuple-like: store (tag1, tag2) as two sequential entries
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let i_alloca = self.builder.build_alloca(i64, "lz_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_hdr = self.context.append_basic_block(current_fn, "lz_hdr");
        let loop_bdy = self.context.append_basic_block(current_fn, "lz_bdy");
        let loop_ext = self.context.append_basic_block(current_fn, "lz_ext");

        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_hdr);
        let i = self
            .builder
            .build_load(i64, i_alloca, "lz_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i, min_len, "lz_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_bdy, loop_ext);

        self.builder.position_at_end(loop_bdy);
        let sp1 = unsafe {
            self.builder
                .build_gep(self.string_type, d1, &[i], "lz_sp1")
                .map_err(llvm_err)
        }?;
        let e1 = self
            .builder
            .build_load(self.string_type, sp1, "lz_e1")
            .map_err(llvm_err)?;
        let sp2 = unsafe {
            self.builder
                .build_gep(self.string_type, d2, &[i], "lz_sp2")
                .map_err(llvm_err)
        }?;
        let e2 = self
            .builder
            .build_load(self.string_type, sp2, "lz_e2")
            .map_err(llvm_err)?;

        // Push both as separate elements (pair is two sequential entries)
        let cur = self.load_list(result_alloca)?;
        let cc = self.call_rt("action_list_push", &[cur.into(), e1.into()])?;
        let nl = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_push e1 failed")?;
        self.builder
            .build_store(result_alloca, nl)
            .map_err(llvm_err)?;
        let cur2 = self.load_list(result_alloca)?;
        let cc2 = self.call_rt("action_list_push", &[cur2.into(), e2.into()])?;
        let nl2 = cc2
            .try_as_basic_value()
            .basic()
            .ok_or("list_push e2 failed")?;
        self.builder
            .build_store(result_alloca, nl2)
            .map_err(llvm_err)?;

        let ni = self
            .builder
            .build_int_add(i, i64.const_int(1, false), "lz_ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_ext);
        Ok(TypedValue::List(result_alloca))
    }
}
