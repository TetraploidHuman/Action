use crate::{llvm_err, CodeGen, TypedValue};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_lazy_map_values(
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
    pub(crate) fn lazy_map_impl(
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

    pub(crate) fn builtin_lazy_filter_values(
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
    pub(crate) fn lazy_filter_impl(
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
}
