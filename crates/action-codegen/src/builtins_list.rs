// Submodule: builtins_list

use action_frontend::ast::*;
use inkwell::IntPredicate;

use super::call_arg::CallArg;
use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn builtin_list(
        &mut self,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        let len = self.i64_ty().const_int(args.len() as u64, false);
        let cc = self.call_rt("action_list_create", &[len.into()])?;
        let list_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let list_alloca = self
            .builder
            .build_alloca(self.list_type, "list_tmp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(list_alloca, list_bv)
            .map_err(llvm_err)?;

        for arg in args {
            let v = self.compile_call_arg(*arg)?;
            // action_list_push handles rc_inc of the element data_ptr internally
            let elem_fat = self.to_fat_struct(&v)?;
            let list_val = self.load_list(list_alloca)?;
            let cc = self.call_rt("action_list_push", &[list_val.into(), elem_fat.into()])?;
            let new_list = cc.try_as_basic_value().basic().ok_or("list_push failed")?;
            self.builder
                .build_store(list_alloca, new_list)
                .map_err(llvm_err)?;
        }

        Ok(TypedValue::List(list_alloca))
    }

    /// lazy_list(seed) - create a lazy list with a seed value
    /// lazy_list(seed) { fn } - create a lazy list with seed and step function
    pub(super) fn builtin_lazy_list(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if args.is_empty() {
            return Err("lazy_list expects at least 1 argument (seed)".to_string());
        }
        let seed = self.compile_call_arg(args[0])?;
        let seed_i64 = match &seed {
            TypedValue::Int(v) => *v,
            _ => return Err("lazy_list: seed must be an Int".to_string()),
        };

        // Compile step function if provided
        let (step_fn_ptr, state, take_count) = if let Some(lam) = trailing {
            let step_fn_val = self.compile_lambda_for_lazy_call_arg(lam)?;
            // -1 means "infinite" — only bounded by explicit take()
            (
                step_fn_val,
                seed_i64,
                self.i64_ty().const_int(-1_i64 as u64, true),
            )
        } else {
            // No step function: only the seed element
            (
                self.ptr_ty().const_null(),
                self.i64_ty().const_int(0, false),
                self.i64_ty().const_int(0, false),
            )
        };

        // Build LazyList struct: {head_val: i64, step_fn: i8*, state: i64, take_count: i64, map_fn: i8*}
        let ll_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "ll")
            .map_err(llvm_err)?;
        let undef = self.lazylist_type.get_undef();
        let v0 = self
            .builder
            .build_insert_value(undef, seed_i64, 0, "ll_head")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn_ptr, 1, "ll_fn")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state, 2, "ll_state")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, take_count, 3, "ll_tc")
            .map_err(llvm_err)?;
        let v4 = self
            .builder
            .build_insert_value(v3, self.ptr_ty().const_null(), 4, "ll_map")
            .map_err(llvm_err)?;
        let v5 = self
            .builder
            .build_insert_value(v4, self.ptr_ty().const_null(), 5, "ll_filt")
            .map_err(llvm_err)?;
        self.builder.build_store(ll_alloca, v5).map_err(llvm_err)?;
        Ok(TypedValue::LazyList(ll_alloca))
    }

    /// Compile a lambda CallArg for use as a lazy list step function.
    fn compile_lambda_for_lazy_call_arg(
        &mut self,
        lam: CallArg<'_>,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        match lam {
            CallArg::Ast(e) => self.compile_lambda_for_lazy(e),
            CallArg::Hir(h) => match &h.kind {
                action_frontend::hir::HirExprKind::Lambda { params, body, .. } => {
                    if params.is_empty() {
                        return Err("lazy_list step function expects 1 parameter".to_string());
                    }
                    let fn_val = self.compile_lambda_hir(params, body)?;
                    match fn_val {
                        TypedValue::Fn(ptr, _) => Ok(ptr),
                        TypedValue::Closure { fn_ptr, .. } => Ok(fn_ptr),
                        _ => Err("lazy_list: step function compilation failed".to_string()),
                    }
                }
                _ => Err("lazy_list: expected lambda body".to_string()),
            },
        }
    }

    /// Compile a lambda for use as a lazy list step function.
    /// Returns a function pointer that can be called with (i64 state) -> next_i64.
    fn compile_lambda_for_lazy(
        &mut self,
        lam: &Expr,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        match &lam.kind {
            ExprKind::Lambda { params, body, .. } => {
                if params.is_empty() {
                    return Err("lazy_list step function expects 1 parameter".to_string());
                }
                let fn_val = self.compile_lambda(params, body)?;
                match fn_val {
                    TypedValue::Fn(ptr, _) => Ok(ptr),
                    TypedValue::Closure { fn_ptr, .. } => Ok(fn_ptr),
                    _ => Err("lazy_list: step function compilation failed".to_string()),
                }
            }
            _ => Err("lazy_list: expected lambda body".to_string()),
        }
    }

    // ---- LazyList operations ----

    /// If the value is a LazyList, convert it to a List and return the list alloca pointer.
    /// If it's already a List, return the pointer directly.
    pub(super) fn ensure_list_ptr(
        &self,
        val: &TypedValue<'ctx>,
        prefix: &str,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        match val {
            TypedValue::LazyList(_) => {
                let list_sv = self.convert_lazylist_to_list(val)?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, &format!("{}_list", prefix))
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(alloca, list_sv)
                    .map_err(llvm_err)?;
                Ok(alloca)
            }
            TypedValue::List(p) => Ok(*p),
            _ => Err(format!("{}: argument must be a List or LazyList", prefix)),
        }
    }

    /// Convert a LazyList to a List struct value (i.e., the loaded StructValue of the list).
    /// This forces evaluation: iterates the step function take_count times.
    pub(super) fn convert_lazylist_to_list(
        &self,
        ll_val: &TypedValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let ll_ptr = match ll_val {
            TypedValue::LazyList(p) => *p,
            _ => return Err("convert_lazylist_to_list: expected LazyList".to_string()),
        };
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, ll_ptr, "ll_conv")
            .map_err(llvm_err)?;
        let ll_struct = ll_sv.into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_struct, 0, "ll_head")
            .map_err(llvm_err)?
            .into_int_value();
        let step_fn = self
            .builder
            .build_extract_value(ll_struct, 1, "ll_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let state_val = self
            .builder
            .build_extract_value(ll_struct, 2, "ll_state")
            .map_err(llvm_err)?
            .into_int_value();
        let take_count_val = self
            .builder
            .build_extract_value(ll_struct, 3, "ll_tc")
            .map_err(llvm_err)?
            .into_int_value();
        let map_fn = self
            .builder
            .build_extract_value(ll_struct, 4, "ll_map")
            .map_err(llvm_err)?
            .into_pointer_value();
        let filter_fn = self
            .builder
            .build_extract_value(ll_struct, 5, "ll_filt")
            .map_err(llvm_err)?
            .into_pointer_value();

        let zero = self.i64_ty().const_int(0, false);
        let one = self.i64_ty().const_int(1, false);
        let neg_one = self.i64_ty().const_int((-1_i64) as u64, true);

        let has_step = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                step_fn,
                self.ptr_ty().const_null(),
                "has_step",
            )
            .map_err(llvm_err)?;
        let state_nz = self
            .builder
            .build_int_compare(IntPredicate::NE, state_val, zero, "state_nz")
            .map_err(llvm_err)?;
        // list-backed: no step fn but state holds a valid data pointer (from toLazyList)
        let not_has_step = self
            .builder
            .build_not(has_step, "not_has_step")
            .map_err(llvm_err)?;
        let is_list_backed = self
            .builder
            .build_and(not_has_step, state_nz, "is_list_backed")
            .map_err(llvm_err)?;

        let tc_gt_zero = self
            .builder
            .build_int_compare(IntPredicate::SGT, take_count_val, zero, "tc_gt0")
            .map_err(llvm_err)?;
        let tc_is_neg1 = self
            .builder
            .build_int_compare(IntPredicate::EQ, take_count_val, neg_one, "tc_neg1")
            .map_err(llvm_err)?;
        let tc_or_inf = self
            .builder
            .build_or(tc_gt_zero, tc_is_neg1, "tc_or_inf")
            .map_err(llvm_err)?;
        let should_generate = self
            .builder
            .build_and(has_step, tc_or_inf, "should_gen")
            .map_err(llvm_err)?;

        // Compute final_count:
        //   list-backed: use take_count (at least 1 for the already-pushed head)
        //   has_step:    use max(1, take_count)
        //   head-only:   1
        let total_elems = self
            .builder
            .build_select(tc_is_neg1, one, take_count_val, "total_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let step_count = self
            .builder
            .build_select(has_step, total_elems, one, "step_count")
            .map_err(llvm_err)?
            .into_int_value();
        // For list-backed, take_count is already the right count (>=0); ensure at least 1 for head
        let lb_count = self
            .builder
            .build_select(tc_gt_zero, take_count_val, one, "lb_count")
            .map_err(llvm_err)?
            .into_int_value();
        let final_count = self
            .builder
            .build_select(is_list_backed, lb_count, step_count, "final_count")
            .map_err(llvm_err)?
            .into_int_value();

        // Create result list
        let cc = self.call_rt("action_list_create", &[final_count.into()])?;
        let list_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "ll_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, list_bv)
            .map_err(llvm_err)?;

        let has_map = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                map_fn,
                self.ptr_ty().const_null(),
                "has_map",
            )
            .map_err(llvm_err)?;
        let has_filter = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                filter_fn,
                self.ptr_ty().const_null(),
                "has_filt",
            )
            .map_err(llvm_err)?;
        let map_fn_type = self.string_type.fn_type(&[self.i64_ty().into()], false);
        let filt_fn_type = self.string_type.fn_type(&[self.i64_ty().into()], false);
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;

        // ---- Head push with optional map and filter ----
        // Flow: map_head_bb / no_map_head_bb → head_check_bb
        //       head_check_bb: phi → if has_filter → call_filt_head_bb else → head_push_bb
        //       call_filt_head_bb: call filter → if pass → head_push_bb else → head_skip_bb
        //       head_push_bb: push, i=1 → after_head_bb
        //       head_skip_bb: i=0 → after_head_bb
        //       after_head_bb: check need_more → loop_hdr or loop_exit
        let map_head_bb = self.context.append_basic_block(current_fn, "map_head");
        let no_map_head_bb = self.context.append_basic_block(current_fn, "no_map_head");
        let head_check_bb = self.context.append_basic_block(current_fn, "head_check");
        let call_filt_head_bb = self
            .context
            .append_basic_block(current_fn, "call_filt_head");
        let head_push_bb = self.context.append_basic_block(current_fn, "head_push");
        let head_skip_bb = self.context.append_basic_block(current_fn, "head_skip");
        let after_head_bb = self.context.append_basic_block(current_fn, "after_head");
        let _ = self
            .builder
            .build_conditional_branch(has_map, map_head_bb, no_map_head_bb);

        // Map head
        self.builder.position_at_end(map_head_bb);
        let mapped_head = self
            .builder
            .build_indirect_call(map_fn_type, map_fn, &[head_val.into()], "mh_call")
            .map_err(llvm_err)?;
        let mapped_head_bv = mapped_head
            .try_as_basic_value()
            .basic()
            .ok_or("map head call failed")?;
        let mapped_head_val = if mapped_head_bv.is_struct_value() {
            self.builder
                .build_extract_value(mapped_head_bv.into_struct_value(), 0, "mh_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            mapped_head_bv.into_int_value()
        };
        let _ = self.builder.build_unconditional_branch(head_check_bb);

        // No map head
        self.builder.position_at_end(no_map_head_bb);
        let _ = self.builder.build_unconditional_branch(head_check_bb);

        // ---- head_check_bb: phi for head, then branch on has_filter ----
        self.builder.position_at_end(head_check_bb);
        let head_phi = self
            .builder
            .build_phi(self.i64_ty(), "head_phi")
            .map_err(llvm_err)?;
        head_phi.add_incoming(&[(&mapped_head_val, map_head_bb), (&head_val, no_map_head_bb)]);
        let candidate_head = head_phi.as_basic_value().into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(has_filter, call_filt_head_bb, head_push_bb);

        // ---- call_filt_head_bb: call filter on head ----
        self.builder.position_at_end(call_filt_head_bb);
        let filt_head_call = self
            .builder
            .build_indirect_call(filt_fn_type, filter_fn, &[candidate_head.into()], "fh_call")
            .map_err(llvm_err)?;
        let filt_head_bv = filt_head_call
            .try_as_basic_value()
            .basic()
            .ok_or("filt head call failed")?;
        let filt_head_tag = if filt_head_bv.is_struct_value() {
            self.builder
                .build_extract_value(filt_head_bv.into_struct_value(), 0, "fh_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            filt_head_bv.into_int_value()
        };
        let head_passes = self
            .builder
            .build_int_compare(IntPredicate::NE, filt_head_tag, zero, "head_passes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(head_passes, head_push_bb, head_skip_bb);

        // ---- head_push_bb: push head, i=1 ----
        self.builder.position_at_end(head_push_bb);
        let head_fat = self.make_int_fat(candidate_head)?;
        let cur_list_h = self.load_list(result_alloca)?;
        let cc_h = self.call_rt("action_list_push", &[cur_list_h.into(), head_fat.into()])?;
        let new_list_h = cc_h
            .try_as_basic_value()
            .basic()
            .ok_or("push head failed")?;
        self.builder
            .build_store(result_alloca, new_list_h)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(after_head_bb);

        // ---- head_skip_bb: head filtered out, i=0 ----
        self.builder.position_at_end(head_skip_bb);
        let _ = self.builder.build_unconditional_branch(after_head_bb);

        // ---- after_head_bb: init i counter and state, check need_more ----
        self.builder.position_at_end(after_head_bb);
        let i_init_phi = self
            .builder
            .build_phi(self.i64_ty(), "i_init")
            .map_err(llvm_err)?;
        i_init_phi.add_incoming(&[(&one, head_push_bb), (&zero, head_skip_bb)]);
        let i_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ll_i")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i_init_phi.as_basic_value().into_int_value())
            .map_err(llvm_err)?;
        let state_phi_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ll_state_phi")
            .map_err(llvm_err)?;
        self.builder
            .build_store(state_phi_alloca, state_val)
            .map_err(llvm_err)?;

        let need_more = self
            .builder
            .build_or(should_generate, is_list_backed, "need_more")
            .map_err(llvm_err)?;

        let loop_hdr = self.context.append_basic_block(current_fn, "ll_gen_hdr");
        let loop_body = self.context.append_basic_block(current_fn, "ll_gen_body");
        let loop_exit = self.context.append_basic_block(current_fn, "ll_gen_exit");
        let _ = self
            .builder
            .build_conditional_branch(need_more, loop_hdr, loop_exit);

        // ---- loop_hdr: check i < final_count ----
        self.builder.position_at_end(loop_hdr);
        let i_loaded = self
            .builder
            .build_load(self.i64_ty(), i_alloca, "ll_i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_loaded, final_count, "ll_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        // ---- loop_body: generate next element ----
        self.builder.position_at_end(loop_body);

        let data_ptr = self
            .builder
            .build_int_to_ptr(state_val, self.ptr_ty(), "data_ptr")
            .map_err(llvm_err)?;
        let step_block = self.context.append_basic_block(current_fn, "ll_step_blk");
        let lb_block = self.context.append_basic_block(current_fn, "ll_lb_blk");
        let merge_block = self.context.append_basic_block(current_fn, "ll_merge_blk");
        let _ = self
            .builder
            .build_conditional_branch(is_list_backed, lb_block, step_block);

        // Step-function path
        self.builder.position_at_end(step_block);
        let current_state = self
            .builder
            .build_load(self.i64_ty(), state_phi_alloca, "ll_cur_state")
            .map_err(llvm_err)?
            .into_int_value();
        let fn_type = self.fat_return_type.fn_type(&[self.i64_ty().into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, step_fn, &[current_state.into()], "ll_step_call")
            .map_err(llvm_err)?;
        let step_fat = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("step call returned void")?;
        let step_fat_sv = step_fat.into_struct_value();
        let step_elem = self
            .builder
            .build_extract_value(step_fat_sv, 0, "ll_next")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(state_phi_alloca, step_elem)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        // List-backed path
        self.builder.position_at_end(lb_block);
        let elem_gep = unsafe {
            self.builder
                .build_gep(self.fat_return_type, data_ptr, &[i_loaded], "lb_gep")
                .map_err(llvm_err)
        }?;
        let elem_fat = self
            .builder
            .build_load(self.fat_return_type, elem_gep, "lb_fat")
            .map_err(llvm_err)?
            .into_struct_value();
        let lb_elem = self
            .builder
            .build_extract_value(elem_fat, 0, "lb_elem")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Merge element
        self.builder.position_at_end(merge_block);
        let phi = self
            .builder
            .build_phi(self.i64_ty(), "elem_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&step_elem, step_block), (&lb_elem, lb_block)]);
        let elem_val = phi.as_basic_value().into_int_value();

        // Apply map_fn if present
        let map_elem_bb = self.context.append_basic_block(current_fn, "map_elem");
        let no_map_elem_bb = self.context.append_basic_block(current_fn, "no_map_elem");
        let filt_elem_check_bb = self
            .context
            .append_basic_block(current_fn, "filt_elem_check");
        let _ = self
            .builder
            .build_conditional_branch(has_map, map_elem_bb, no_map_elem_bb);

        self.builder.position_at_end(map_elem_bb);
        let mapped_elem_call = self
            .builder
            .build_indirect_call(map_fn_type, map_fn, &[elem_val.into()], "me_call")
            .map_err(llvm_err)?;
        let mapped_elem_bv = mapped_elem_call
            .try_as_basic_value()
            .basic()
            .ok_or("map elem call failed")?;
        let mapped_elem_val = if mapped_elem_bv.is_struct_value() {
            self.builder
                .build_extract_value(mapped_elem_bv.into_struct_value(), 0, "me_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            mapped_elem_bv.into_int_value()
        };
        let _ = self.builder.build_unconditional_branch(filt_elem_check_bb);

        self.builder.position_at_end(no_map_elem_bb);
        let _ = self.builder.build_unconditional_branch(filt_elem_check_bb);

        // ---- filt_elem_check_bb: phi for mapped/unmapped elem, branch on has_filter ----
        self.builder.position_at_end(filt_elem_check_bb);
        let elem_phi_filt = self
            .builder
            .build_phi(self.i64_ty(), "elem_phi_filt")
            .map_err(llvm_err)?;
        elem_phi_filt.add_incoming(&[(&mapped_elem_val, map_elem_bb), (&elem_val, no_map_elem_bb)]);
        let candidate_elem = elem_phi_filt.as_basic_value().into_int_value();

        let call_filt_elem_bb = self
            .context
            .append_basic_block(current_fn, "call_filt_elem");
        let elem_pass_bb = self.context.append_basic_block(current_fn, "elem_pass");
        let elem_fail_bb = self.context.append_basic_block(current_fn, "elem_fail");
        let _ = self
            .builder
            .build_conditional_branch(has_filter, call_filt_elem_bb, elem_pass_bb);

        // ---- call_filt_elem_bb: call filter on element ----
        self.builder.position_at_end(call_filt_elem_bb);
        let filt_elem_call = self
            .builder
            .build_indirect_call(filt_fn_type, filter_fn, &[candidate_elem.into()], "fe_call")
            .map_err(llvm_err)?;
        let filt_elem_bv = filt_elem_call
            .try_as_basic_value()
            .basic()
            .ok_or("filt elem call failed")?;
        let filt_elem_tag = if filt_elem_bv.is_struct_value() {
            self.builder
                .build_extract_value(filt_elem_bv.into_struct_value(), 0, "fe_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            filt_elem_bv.into_int_value()
        };
        let elem_passes = self
            .builder
            .build_int_compare(IntPredicate::NE, filt_elem_tag, zero, "elem_passes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(elem_passes, elem_pass_bb, elem_fail_bb);

        // ---- elem_pass_bb: push element, increment i ----
        self.builder.position_at_end(elem_pass_bb);
        let elem_fat = self.make_int_fat(candidate_elem)?;
        let cur_list2 = self.load_list(result_alloca)?;
        let cc2 = self.call_rt("action_list_push", &[cur_list2.into(), elem_fat.into()])?;
        let new_list2 = cc2.try_as_basic_value().basic().ok_or("push2 failed")?;
        self.builder
            .build_store(result_alloca, new_list2)
            .map_err(llvm_err)?;
        let new_i = self
            .builder
            .build_int_add(i_loaded, one, "ll_i_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, new_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        // ---- elem_fail_bb: skip this element, try next ----
        self.builder.position_at_end(elem_fail_bb);
        let _ = self.builder.build_unconditional_branch(loop_body);

        // ---- loop_exit ----
        self.builder.position_at_end(loop_exit);
        let final_list = self.load_list(result_alloca)?;
        Ok(final_list)
    }

    /// Create a fat struct {i64, i8*} from an i64 value (using string_type to match list_push expectations)
    pub(super) fn make_int_fat(
        &self,
        val: inkwell::values::IntValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let undef = self.string_type.get_undef();
        let null_ptr = self.ptr_ty().const_null();
        let aggregate = self
            .builder
            .build_insert_value(undef, val, 0, "fat_v")
            .map_err(llvm_err)?;
        let aggregate2 = self
            .builder
            .build_insert_value(aggregate, null_ptr, 1, "fat_p")
            .map_err(llvm_err)?;
        Ok(aggregate2.into_struct_value())
    }
}
