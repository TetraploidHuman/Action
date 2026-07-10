//! For-loop codegen (R4-4).

use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue, ValKind};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn alloc_list_get_cache(&mut self) -> Result<PointerValue<'ctx>, String> {
        let i8 = self.context.i8_type();
        let cache = self
            .builder
            .build_alloca(i8.array_type(32), "list_get_cache")
            .map_err(llvm_err)?;
        let cache_i8 = self
            .builder
            .build_pointer_cast(
                cache,
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "cache_i8",
            )
            .map_err(llvm_err)?;
        let zero_i8 = i8.const_int(0, false);
        self.builder
            .build_store(cache_i8, zero_i8)
            .map_err(llvm_err)?;
        Ok(cache)
    }

    pub(crate) fn list_get_cached_tag(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        idx: IntValue<'ctx>,
        cache: PointerValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let loaded = self.load_list(list_ptr)?;
        let cc = self.call_rt(
            "action_list_get_cached",
            &[loaded.into(), idx.into(), cache.into()],
        )?;
        let fat_elem = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_get_cached failed")?;
        self.builder
            .build_extract_value(fat_elem.into_struct_value(), 0, "elem_tag")
            .map_err(llvm_err)
            .map(|v| v.into_int_value())
    }

    /// Like list_get_cached_tag but returns the full fat struct {tag, data}.
    pub(crate) fn list_get_cached_fat(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        idx: IntValue<'ctx>,
        cache: PointerValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let loaded = self.load_list(list_ptr)?;
        let cc = self.call_rt(
            "action_list_get_cached",
            &[loaded.into(), idx.into(), cache.into()],
        )?;
        cc.try_as_basic_value()
            .basic()
            .ok_or_else(|| "list_get_cached_fat failed".to_string())
    }

    /// `sum += Σ i` for `i in [start, end)` — closed form matching range for-loop semantics.
    pub(crate) fn compile_range_int_sum_update(
        &mut self,
        sum_ptr: PointerValue<'ctx>,
        start: IntValue<'ctx>,
        end: IntValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let one = i64.const_int(1, false);
        let two = i64.const_int(2, false);
        let n = self
            .builder
            .build_int_sub(end, start, "rs_n")
            .map_err(llvm_err)?;
        let last = self
            .builder
            .build_int_sub(end, one, "rs_last")
            .map_err(llvm_err)?;
        let first_plus_last = self
            .builder
            .build_int_add(start, last, "rs_fpl")
            .map_err(llvm_err)?;
        let product = self
            .builder
            .build_int_mul(n, first_plus_last, "rs_prod")
            .map_err(llvm_err)?;
        let delta = self
            .builder
            .build_int_signed_div(product, two, "rs_delta")
            .map_err(llvm_err)?;
        let cur = self
            .builder
            .build_load(i64, sum_ptr, "rs_cur")
            .map_err(llvm_err)?
            .into_int_value();
        let new_sum = self
            .builder
            .build_int_add(cur, delta, "rs_new")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sum_ptr, new_sum)
            .map_err(llvm_err)?;
        Ok(())
    }

    /// `for i < end { sum += i * k; i++ }` from loop-local `{ x -> x * k }(i)`.
    pub(crate) fn compile_invariant_lambda_acc_loop(
        &mut self,
        sum_ptr: PointerValue<'ctx>,
        idx_ptr: PointerValue<'ctx>,
        end_bound: IntValue<'ctx>,
        mul_const: u64,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;
        let i64 = self.i64_ty();
        let one = i64.const_int(1, false);
        let k = i64.const_int(mul_const, false);

        let header = self.context.append_basic_block(current_fn, "lamacc_hdr");
        let body_bb = self.context.append_basic_block(current_fn, "lamacc_body");
        let exit = self.context.append_basic_block(current_fn, "lamacc_exit");

        let saved_continue = self.loop_control.continue_target;
        let saved_break = self.loop_control.break_target;
        self.loop_control.continue_target = Some(header);
        self.loop_control.break_target = Some(exit);

        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;
        self.builder.position_at_end(header);
        let cur = self
            .builder
            .build_load(i64, idx_ptr, "lamacc_i")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur, end_bound, "lamacc_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body_bb, exit)
            .map_err(llvm_err)?;

        self.builder.position_at_end(body_bb);
        let term = self
            .builder
            .build_int_mul(cur, k, "lamacc_term")
            .map_err(llvm_err)?;
        let sum_cur = self
            .builder
            .build_load(i64, sum_ptr, "lamacc_sum")
            .map_err(llvm_err)?
            .into_int_value();
        let sum_new = self
            .builder
            .build_int_add(sum_cur, term, "lamacc_sum_new")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sum_ptr, sum_new)
            .map_err(llvm_err)?;
        let next = self
            .builder
            .build_int_add(cur, one, "lamacc_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_ptr, next)
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(exit);
        self.loop_control.continue_target = saved_continue;
        self.loop_control.break_target = saved_break;
        Ok(TypedValue::Unit)
    }

    /// `for i < end { sum += { x -> x + i }(arg); i++ }` without per-iter closure alloc.
    pub(crate) fn compile_captured_lambda_acc_loop(
        &mut self,
        sum_ptr: PointerValue<'ctx>,
        idx_ptr: PointerValue<'ctx>,
        end_bound: IntValue<'ctx>,
        term: super::CapturedIdxAddTerm,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;
        let i64 = self.i64_ty();
        let one = i64.const_int(1, false);

        let header = self.context.append_basic_block(current_fn, "capacc_hdr");
        let body_bb = self.context.append_basic_block(current_fn, "capacc_body");
        let exit = self.context.append_basic_block(current_fn, "capacc_exit");

        let saved_continue = self.loop_control.continue_target;
        let saved_break = self.loop_control.break_target;
        self.loop_control.continue_target = Some(header);
        self.loop_control.break_target = Some(exit);

        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;
        self.builder.position_at_end(header);
        let cur = self
            .builder
            .build_load(i64, idx_ptr, "capacc_i")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur, end_bound, "capacc_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body_bb, exit)
            .map_err(llvm_err)?;

        self.builder.position_at_end(body_bb);
        let term_val = match term {
            super::CapturedIdxAddTerm::IdxPlusIdx => self
                .builder
                .build_int_add(cur, cur, "capacc_term")
                .map_err(llvm_err)?,
            super::CapturedIdxAddTerm::ConstPlusIdx(k) => {
                let k_val = i64.const_int(k, false);
                self.builder
                    .build_int_add(k_val, cur, "capacc_term")
                    .map_err(llvm_err)?
            }
        };
        let sum_cur = self
            .builder
            .build_load(i64, sum_ptr, "capacc_sum")
            .map_err(llvm_err)?
            .into_int_value();
        let sum_new = self
            .builder
            .build_int_add(sum_cur, term_val, "capacc_sum_new")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sum_ptr, sum_new)
            .map_err(llvm_err)?;
        let next = self
            .builder
            .build_int_add(cur, one, "capacc_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_ptr, next)
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(exit);
        self.loop_control.continue_target = saved_continue;
        self.loop_control.break_target = saved_break;
        Ok(TypedValue::Unit)
    }

    /// `for i < end { lst = lst.append({ x -> x + i }(K)); i++ }` → push `K + i` each iter.
    pub(crate) fn compile_captured_lambda_append_loop(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        idx_ptr: PointerValue<'ctx>,
        end_bound: IntValue<'ctx>,
        const_arg: u64,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;
        let i64 = self.i64_ty();
        let one = i64.const_int(1, false);
        let k = i64.const_int(const_arg, false);

        let header = self.context.append_basic_block(current_fn, "capapp_hdr");
        let body_bb = self.context.append_basic_block(current_fn, "capapp_body");
        let exit = self.context.append_basic_block(current_fn, "capapp_exit");

        let saved_continue = self.loop_control.continue_target;
        let saved_break = self.loop_control.break_target;
        self.loop_control.continue_target = Some(header);
        self.loop_control.break_target = Some(exit);

        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;
        self.builder.position_at_end(header);
        let cur = self
            .builder
            .build_load(i64, idx_ptr, "capapp_i")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur, end_bound, "capapp_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body_bb, exit)
            .map_err(llvm_err)?;

        self.builder.position_at_end(body_bb);
        let elem = self
            .builder
            .build_int_add(k, cur, "capapp_elem")
            .map_err(llvm_err)?;
        let elem_fat = self.make_int_fat(elem)?;
        let list_loaded = self.load_list(list_ptr)?;
        let cc = self.call_rt(
            "action_list_push",
            &[list_loaded.into(), elem_fat.into()],
        )?;
        let new_list = cc
            .try_as_basic_value()
            .basic()
            .ok_or("captured lambda append fusion push failed")?;
        self.builder
            .build_store(list_ptr, new_list)
            .map_err(llvm_err)?;
        let next = self
            .builder
            .build_int_add(cur, one, "capapp_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_ptr, next)
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(exit);
        self.loop_control.continue_target = saved_continue;
        self.loop_control.break_target = saved_break;
        Ok(TypedValue::Unit)
    }

    /// `for idx < end { lst = lst.remove(0); idx = idx + 1 }` → single `drop(end)`.
    pub(crate) fn compile_remove_front_loop(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        end_bound: IntValue<'ctx>,
        idx_ptr: PointerValue<'ctx>,
        coll_var: &str,
    ) -> Result<TypedValue<'ctx>, String> {
        let list_loaded = self.load_list(list_ptr)?;
        let drop_cc = self.call_rt(
            "action_list_drop",
            &[list_loaded.into(), end_bound.into()],
        )?;
        let new_list = drop_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list drop front fusion failed")?;
        let scratch = self
            .builder
            .build_alloca(self.list_type, "drop_front")
            .map_err(llvm_err)?;
        self.builder
            .build_store(scratch, new_list)
            .map_err(llvm_err)?;
        self.assign_mutable_ident(coll_var, TypedValue::List(scratch))?;
        self.builder
            .build_store(idx_ptr, end_bound)
            .map_err(llvm_err)?;
        Ok(TypedValue::Unit)
    }

    /// `for idx < end { lst.get(idx); idx = idx + 1 }` — cached sequential walk.

    pub(crate) fn compile_sequential_list_get_loop(
        &mut self,
        list_ptr: inkwell::values::PointerValue<'ctx>,
        end_bound: inkwell::values::IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let idx_alloca = self
            .builder
            .build_alloca(i64, "seq_idx")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, zero)
            .map_err(llvm_err)?;
        let cache = self.alloc_list_get_cache()?;
        let tmp_alloca = self
            .builder
            .build_alloca(i64, "seq_tmp")
            .map_err(llvm_err)?;
        let header = self.context.append_basic_block(current_fn, "seq_hdr");
        let body_bb = self.context.append_basic_block(current_fn, "seq_body");
        let exit = self.context.append_basic_block(current_fn, "seq_exit");
        let saved_continue = self.loop_control.continue_target;
        let saved_break = self.loop_control.break_target;
        self.loop_control.continue_target = Some(header);
        self.loop_control.break_target = Some(exit);
        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;
        self.builder.position_at_end(header);
        let cur = self
            .builder
            .build_load(i64, idx_alloca, "seq_i")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur, end_bound, "seq_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body_bb, exit)
            .map_err(llvm_err)?;
        self.builder.position_at_end(body_bb);
        let tag = self.list_get_cached_tag(list_ptr, cur, cache)?;
        self.builder
            .build_store(tmp_alloca, tag)
            .map_err(llvm_err)?;
        let next = self
            .builder
            .build_int_add(cur, one, "seq_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, next)
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;
        self.builder.position_at_end(exit);
        self.loop_control.continue_target = saved_continue;
        self.loop_control.break_target = saved_break;
        Ok(TypedValue::Unit)
    }

    /// Minimum loop iterations to hoist `action_ht_from_list` for repeated `contains`.
    pub(super) const CONTAINS_HT_FUSION_MIN_ITERS: u64 = 16;

    /// `for idx < end { list.contains(key); idx = idx + 1 }` when `list` is loop-invariant:
    /// build ephemeral hash set once, then O(1) lookups per iteration.
    pub(crate) fn compile_invariant_contains_loop(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        end_bound: IntValue<'ctx>,
        idx_ptr: PointerValue<'ctx>,
        key_hir: &action_frontend::hir::HirExpr,
        inc_hir: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;
        let i64 = self.i64_ty();

        let list_loaded = self.load_list(list_ptr)?;
        let set_cc = self.call_rt("action_ht_from_list", &[list_loaded.into()])?;
        let set_init = set_cc
            .try_as_basic_value()
            .basic()
            .ok_or("ht_from_list failed")?
            .into_struct_value();
        let set_alloca = self
            .builder
            .build_alloca(self.list_type, "contains_set")
            .map_err(llvm_err)?;
        self.builder
            .build_store(set_alloca, set_init)
            .map_err(llvm_err)?;

        let header = self.context.append_basic_block(current_fn, "contains_hdr");
        let body_bb = self.context.append_basic_block(current_fn, "contains_body");
        let exit = self.context.append_basic_block(current_fn, "contains_exit");

        let saved_continue = self.loop_control.continue_target;
        let saved_break = self.loop_control.break_target;
        self.loop_control.continue_target = Some(header);
        self.loop_control.break_target = Some(exit);

        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;
        self.builder.position_at_end(header);
        let cur_idx = self
            .builder
            .build_load(i64, idx_ptr, "contains_i")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur_idx, end_bound, "contains_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body_bb, exit)
            .map_err(llvm_err)?;

        self.builder.position_at_end(body_bb);
        let key_val = self.compile_hir_expr(key_hir)?;
        let fat = self.to_fat_struct(&key_val)?;
        let set_loaded = self.load_list(set_alloca)?;
        let hit_cc = self.call_rt("action_ht_contains", &[set_loaded.into(), fat.into()])?;
        let _hit = hit_cc
            .try_as_basic_value()
            .basic()
            .ok_or("ht_contains failed")?
            .into_int_value();
        let inc_val = self.compile_hir_expr(inc_hir)?;
        self.rc_discard_value(&inc_val)?;
        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(exit);
        self.loop_control.continue_target = saved_continue;
        self.loop_control.break_target = saved_break;

        let set_final = self
            .builder
            .build_load(self.list_type, set_alloca, "contains_set_final")
            .map_err(llvm_err)?
            .into_struct_value();
        self.rc_dec_heap_collection(set_final, ValKind::Set)?;

        Ok(TypedValue::Unit)
    }

    pub(super) const MAP_INSERT_PRESIZE_MIN_ITERS: u64 = 64;

    /// `for idx < end { coll = coll.insert(...); idx = idx + 1 }` with empty initial map/set:
    /// presize hash table once to avoid repeated rehash.
    pub(crate) fn compile_collection_insert_build_loop(
        &mut self,
        coll_ptr: PointerValue<'ctx>,
        end_bound: IntValue<'ctx>,
        idx_ptr: PointerValue<'ctx>,
        key_hir: &action_frontend::hir::HirExpr,
        val_hir: Option<&action_frontend::hir::HirExpr>,
        inc_hir: &action_frontend::hir::HirExpr,
        kind: ValKind,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);

        let coll_loaded = self.load_list(coll_ptr)?;
        let cur_len = self.map_len_val(coll_loaded)?;
        let needs_presize = self
            .builder
            .build_int_compare(IntPredicate::EQ, cur_len, zero, "coll_empty")
            .map_err(llvm_err)?;
        let presize_bb = self.context.append_basic_block(current_fn, "coll_presize");
        let presize_done = self
            .context
            .append_basic_block(current_fn, "coll_presize_done");
        let _ = self
            .builder
            .build_conditional_branch(needs_presize, presize_bb, presize_done);
        self.builder.position_at_end(presize_bb);
        let presized = self
            .call_rt("action_map_create", &[end_bound.into()])?
            .try_as_basic_value()
            .basic()
            .ok_or("map_create presize failed")?;
        self.builder
            .build_store(coll_ptr, presized)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(presize_done);

        self.builder.position_at_end(presize_done);
        let header = self.context.append_basic_block(current_fn, "coll_ins_hdr");
        let body_bb = self.context.append_basic_block(current_fn, "coll_ins_body");
        let exit = self.context.append_basic_block(current_fn, "coll_ins_exit");

        let saved_continue = self.loop_control.continue_target;
        let saved_break = self.loop_control.break_target;
        self.loop_control.continue_target = Some(header);
        self.loop_control.break_target = Some(exit);

        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;
        self.builder.position_at_end(header);
        let cur_idx = self
            .builder
            .build_load(i64, idx_ptr, "coll_i")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur_idx, end_bound, "coll_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body_bb, exit)
            .map_err(llvm_err)?;

        self.builder.position_at_end(body_bb);
        let key_val = self.compile_hir_expr(key_hir)?;
        let key_fat = self.to_fat_struct(&key_val)?;
        let null_val: inkwell::values::BasicValueEnum = {
            let undef = self.string_type.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, zero, 0, "sn0")
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(r1, self.ptr_ty().const_zero(), 1, "sn1")
                .map_err(llvm_err)?
                .as_basic_value_enum()
        };
        let (insert_val_fat, val_owned) = if let Some(val_hir) = val_hir {
            let val_val = self.compile_hir_expr(val_hir)?;
            let fat = self.to_fat_struct(&val_val)?;
            (fat, Some(val_val))
        } else {
            (null_val, None)
        };
        let coll_loaded = self.load_list(coll_ptr)?;
        let ins_cc = self.call_rt(
            "action_map_insert",
            &[coll_loaded.into(), key_fat.into(), insert_val_fat.into()],
        )?;
        let new_coll = ins_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_insert failed")?;
        self.builder
            .build_store(coll_ptr, new_coll)
            .map_err(llvm_err)?;
        let _ = self.rc_free_intermediate(&key_val);
        if let Some(val_val) = val_owned {
            let _ = self.rc_free_intermediate(&val_val);
        }
        let inc_val = self.compile_hir_expr(inc_hir)?;
        self.rc_discard_value(&inc_val)?;
        self.builder
            .build_unconditional_branch(header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(exit);
        self.loop_control.continue_target = saved_continue;
        self.loop_control.break_target = saved_break;
        let _ = kind;
        Ok(TypedValue::Unit)
    }
}
