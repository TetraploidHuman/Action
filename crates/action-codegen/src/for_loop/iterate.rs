//! For-loop codegen (R4-4).

use inkwell::basic_block::BasicBlock;
use inkwell::values::{IntValue, PointerValue};
use inkwell::IntPredicate;

use super::ForExprSrc;
use super::{llvm_err, CodeGen, ForIterable, TypedValue, ValKind};
use crate::Scope;
use action_frontend::hir::{HirExpr, HirExprKind, HirForKind, HirStmt};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_for_iterate(
        &mut self,
        variable: &str,
        iterator: ForExprSrc<'_>,
        body: ForExprSrc<'_>,
        collect: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);

        match iterator.classify_iterable(self)? {
            ForIterable::Map { data_ptr, cap } => {
                let use_key = body.iterate_use_map_keys(variable);
                return self.compile_for_iterate_hash(
                    variable, data_ptr, cap, use_key, use_key, body, collect,
                );
            }
            ForIterable::Set { data_ptr, cap } => {
                return self
                    .compile_for_iterate_hash(variable, data_ptr, cap, true, false, body, collect);
            }
            ForIterable::Range { start, end } => self
                .compile_for_iterate_range_list(variable, start, end, None, body, collect, false),
            ForIterable::List { list_ptr, len } => {
                // M75: List[String] elements — same `len(var)` cue Map keys use for ValKind::Str.
                let bind_str = body.iterate_bind_str(variable);
                self.compile_for_iterate_range_list(
                    variable,
                    zero,
                    len,
                    Some(list_ptr),
                    body,
                    collect,
                    bind_str,
                )
            }
        }
    }

    fn compile_for_iterate_hash(
        &mut self,
        variable: &str,
        data_ptr: PointerValue<'ctx>,
        cap: IntValue<'ctx>,
        use_key: bool,
        bind_str: bool,
        body: ForExprSrc<'_>,
        collect: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function".to_string())?;

        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);

        let result_list = if collect {
            let list_cc = self.call_rt("action_list_create", &[cap.into()])?;
            let list_bv = list_cc
                .try_as_basic_value()
                .basic()
                .ok_or("list_create failed")?;
            let result_alloca = self
                .builder
                .build_alloca(self.list_type, "collect_result")
                .map_err(llvm_err)?;
            self.builder
                .build_store(result_alloca, list_bv)
                .map_err(llvm_err)?;
            Some(result_alloca)
        } else {
            None
        };

        let slot_alloca = self
            .builder
            .build_alloca(i64, "for_map_slot")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slot_alloca, zero)
            .map_err(llvm_err)?;

        let val_alloca = if bind_str {
            None
        } else {
            Some(
                self.builder
                    .build_alloca(i64, "for_map_val")
                    .map_err(llvm_err)?,
            )
        };
        let str_alloca = if bind_str {
            Some(
                self.builder
                    .build_alloca(self.string_type, "for_map_str")
                    .map_err(llvm_err)?,
            )
        } else {
            None
        };

        let loop_header = self.context.append_basic_block(current_fn, "for_map_hdr");
        let loop_chk = self.context.append_basic_block(current_fn, "for_map_chk");
        let loop_body = self.context.append_basic_block(current_fn, "for_map_body");
        let loop_next = self.context.append_basic_block(current_fn, "for_map_nxt");
        let loop_exit = self.context.append_basic_block(current_fn, "for_map_ext");

        let saved_continue_target = self.loop_control.continue_target;
        let saved_break_target = self.loop_control.break_target;
        self.loop_control.continue_target = Some(loop_next);
        self.loop_control.break_target = Some(loop_exit);

        self.builder
            .build_unconditional_branch(loop_header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_header);
        let slot = self
            .builder
            .build_load(i64, slot_alloca, "slot_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, slot, cap, "for_map_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, loop_chk, loop_exit)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_chk);
        self.ht_branch_if_slot_active(data_ptr, slot, loop_body, loop_next)?;

        self.builder.position_at_end(loop_body);
        let item_fat = if use_key {
            self.ht_key_fat_at(data_ptr, slot)?
        } else {
            self.ht_val_fat_at(data_ptr, slot)?
        };
        if let Some(str_a) = str_alloca {
            self.builder
                .build_store(str_a, item_fat)
                .map_err(llvm_err)?;
        } else if let Some(val_a) = val_alloca {
            let item_tag = self
                .builder
                .build_extract_value(item_fat.into_struct_value(), 0, "hash_item_tag")
                .map_err(llvm_err)?
                .into_int_value();
            self.builder
                .build_store(val_a, item_tag)
                .map_err(llvm_err)?;
        }

        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::with_parent(saved_scope);
        if let Some(str_a) = str_alloca {
            self.scope.set(
                variable.to_string(),
                str_a,
                self.string_type.into(),
                ValKind::Str,
            );
        } else if let Some(val_a) = val_alloca {
            self.scope
                .set(variable.to_string(), val_a, i64.into(), ValKind::Int);
        }

        let body_val = body.compile(self)?;

        if let Some(list_ptr) = result_list {
            let list_loaded = self.load_list(list_ptr)?;
            let elem_fat = self.to_fat_struct(&body_val)?;
            let push_cc =
                self.call_rt("action_list_push", &[list_loaded.into(), elem_fat.into()])?;
            let pushed = push_cc
                .try_as_basic_value()
                .basic()
                .ok_or("list_push failed")?;
            self.builder
                .build_store(list_ptr, pushed)
                .map_err(llvm_err)?;
        } else {
            self.rc_discard_value(&body_val)?;
        }

        self.builder
            .build_unconditional_branch(loop_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_next);
        let mut parent = Scope::new();
        std::mem::swap(&mut self.scope, &mut parent);
        if let Some(p) = parent.parent {
            self.scope = *p;
        }

        let next_slot = self
            .builder
            .build_load(i64, slot_alloca, "slot_next")
            .map_err(llvm_err)?
            .into_int_value();
        let one = i64.const_int(1, false);
        let inc = self
            .builder
            .build_int_add(next_slot, one, "slot_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slot_alloca, inc)
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_exit);
        self.loop_control.continue_target = saved_continue_target;
        self.loop_control.break_target = saved_break_target;

        if let Some(list_ptr) = result_list {
            Ok(TypedValue::List(list_ptr))
        } else {
            Ok(TypedValue::Unit)
        }
    }

    fn compile_for_iterate_range_list(
        &mut self,
        variable: &str,
        start_val: IntValue<'ctx>,
        end_val: IntValue<'ctx>,
        input_list_ptr: Option<PointerValue<'ctx>>,
        body: ForExprSrc<'_>,
        collect: bool,
        bind_str: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function".to_string())?;

        let i64 = self.i64_ty();

        // Create result list if collecting
        let result_list = if collect {
            let len = self
                .builder
                .build_int_sub(end_val, start_val, "est_len")
                .map_err(llvm_err)?;
            let list_cc = self.call_rt("action_list_create", &[len.into()])?;
            let list_bv = list_cc
                .try_as_basic_value()
                .basic()
                .ok_or("list_create failed")?;
            let result_alloca = self
                .builder
                .build_alloca(self.list_type, "collect_result")
                .map_err(llvm_err)?;
            self.builder
                .build_store(result_alloca, list_bv)
                .map_err(llvm_err)?;
            Some(result_alloca)
        } else {
            None
        };

        // Track write position in result list (separate from loop counter,
        // needed when continue skips some elements)
        let _collect_pos = if result_list.is_some() {
            let pos = self
                .builder
                .build_alloca(i64, "collect_pos")
                .map_err(llvm_err)?;
            self.builder
                .build_store(pos, i64.const_int(0, false))
                .map_err(llvm_err)?;
            Some(pos)
        } else {
            None
        };

        // Allocate loop counter (index)
        let idx_alloca = self
            .builder
            .build_alloca(i64, "for_idx")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, start_val)
            .map_err(llvm_err)?;

        // Sequential get cache for list iteration (for x in lst / walk optimization)
        let list_get_cache = if input_list_ptr.is_some() {
            Some(self.alloc_list_get_cache()?)
        } else {
            None
        };

        // For list iteration, allocate element storage (i64 tag or full String fat).
        let val_alloca = if input_list_ptr.is_some() && !bind_str {
            Some(
                self.builder
                    .build_alloca(i64, "for_val")
                    .map_err(llvm_err)?,
            )
        } else {
            None
        };
        let str_alloca = if input_list_ptr.is_some() && bind_str {
            Some(
                self.builder
                    .build_alloca(self.string_type, "for_str")
                    .map_err(llvm_err)?,
            )
        } else {
            None
        };

        // Create blocks
        let loop_header = self.context.append_basic_block(current_fn, "for_header");
        let loop_body = self.context.append_basic_block(current_fn, "for_body");
        let loop_next = self.context.append_basic_block(current_fn, "for_next"); // continue target + increment
        let loop_exit = self.context.append_basic_block(current_fn, "for_exit");

        // Set continue target so `continue` inside the body branches here
        let saved_continue_target = self.loop_control.continue_target;
        let saved_break_target = self.loop_control.break_target;
        self.loop_control.continue_target = Some(loop_next);
        self.loop_control.break_target = Some(loop_exit);

        // Branch to header
        let _ = self.builder.build_unconditional_branch(loop_header);

        // Loop header: check condition
        self.builder.position_at_end(loop_header);
        let current = self
            .builder
            .build_load(i64, idx_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, current, end_val, "for_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        // Loop body
        self.builder.position_at_end(loop_body);

        // For list iteration: load element via cached sequential walk (O(1) within leaf)
        if let (Some(list_ptr), Some(cache)) = (input_list_ptr, list_get_cache) {
            if let Some(str_a) = str_alloca {
                let fat = self.list_get_cached_fat(list_ptr, current, cache)?;
                self.builder.build_store(str_a, fat).map_err(llvm_err)?;
            } else if let Some(va) = val_alloca {
                let tag = self.list_get_cached_tag(list_ptr, current, cache)?;
                self.builder.build_store(va, tag).map_err(llvm_err)?;
            }
        }

        // Add loop variable to scope
        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::with_parent(saved_scope);
        if let Some(str_a) = str_alloca {
            self.scope.set(
                variable.to_string(),
                str_a,
                self.string_type.into(),
                ValKind::Str,
            );
        } else if let Some(va) = val_alloca {
            self.scope
                .set(variable.to_string(), va, i64.into(), ValKind::Int);
        } else {
            self.scope
                .set(variable.to_string(), idx_alloca, i64.into(), ValKind::Int);
        };

        // Compile body
        let body_val = body.compile(self)?;

        // Collect result if needed
        if let Some(list_ptr) = result_list {
            // action_list_push handles rc_inc of the element data_ptr internally
            let list_loaded = self.load_list(list_ptr)?;
            let elem_fat = self.to_fat_struct(&body_val)?;
            let push_cc =
                self.call_rt("action_list_push", &[list_loaded.into(), elem_fat.into()])?;
            let pushed = push_cc
                .try_as_basic_value()
                .basic()
                .ok_or("list_push failed")?;
            self.builder
                .build_store(list_ptr, pushed)
                .map_err(llvm_err)?;
        } else {
            self.rc_discard_value(&body_val)?;
        }

        // Branch to loop_next (increment)
        self.builder
            .build_unconditional_branch(loop_next)
            .map_err(llvm_err)?;

        // loop_next: restore scope, increment, loop back (also the continue target)
        self.builder.position_at_end(loop_next);

        // Restore scope
        let mut parent = Scope::new();
        std::mem::swap(&mut self.scope, &mut parent);
        if let Some(p) = parent.parent {
            self.scope = *p;
        }

        // Increment counter
        let next_val = self
            .builder
            .build_load(i64, idx_alloca, "i_next")
            .map_err(llvm_err)?
            .into_int_value();
        let one = i64.const_int(1, false);
        let inc = self
            .builder
            .build_int_add(next_val, one, "i_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, inc)
            .map_err(llvm_err)?;

        // Jump back to header
        let _ = self.builder.build_unconditional_branch(loop_header);

        // Continue at exit
        self.builder.position_at_end(loop_exit);

        // Restore continue target
        self.loop_control.continue_target = saved_continue_target;
        self.loop_control.break_target = saved_break_target;

        if let Some(list_ptr) = result_list {
            Ok(TypedValue::List(list_ptr))
        } else {
            Ok(TypedValue::Unit)
        }
    }

    pub(crate) fn compile_for_with_index(
        &mut self,
        vars: &[String],
        iterator: ForExprSrc<'_>,
        body: ForExprSrc<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        if vars.len() != 2 {
            return Err("for with index requires exactly two variables".to_string());
        }
        let index_var = &vars[0];
        let item_var = &vars[1];

        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);

        let iterable = iterator.classify_iterable(self)?;
        match iterable {
            ForIterable::Map { data_ptr, cap } => {
                return self
                    .compile_for_with_index_hash(index_var, item_var, data_ptr, cap, false, body);
            }
            ForIterable::Set { data_ptr, cap } => {
                return self
                    .compile_for_with_index_hash(index_var, item_var, data_ptr, cap, true, body);
            }
            _ => {}
        }

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function".to_string())?;

        enum IterMode<'a> {
            Range {
                start: IntValue<'a>,
                count: IntValue<'a>,
            },
            List {
                list_ptr: PointerValue<'a>,
                len: IntValue<'a>,
            },
        }

        let mode = match iterable {
            ForIterable::Range { start, end } => {
                let count = self
                    .builder
                    .build_int_sub(end, start, "range_count")
                    .map_err(llvm_err)?;
                IterMode::Range { start, count }
            }
            ForIterable::List { list_ptr, len } => IterMode::List { list_ptr, len },
            ForIterable::Map { .. } | ForIterable::Set { .. } => {
                unreachable!("handled above")
            }
        };

        let idx_alloca = self
            .builder
            .build_alloca(i64, "for_idx_pos")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, zero)
            .map_err(llvm_err)?;

        let list_get_cache = match &mode {
            IterMode::List { .. } => Some(self.alloc_list_get_cache()?),
            _ => None,
        };

        let item_alloca = self
            .builder
            .build_alloca(i64, "for_idx_item")
            .map_err(llvm_err)?;
        let bind_item_str = body.iterate_bind_str(item_var);
        let item_str_alloca = if bind_item_str {
            Some(
                self.builder
                    .build_alloca(self.string_type, "for_idx_str")
                    .map_err(llvm_err)?,
            )
        } else {
            None
        };

        let loop_header = self.context.append_basic_block(current_fn, "for_idx_hdr");
        let loop_body = self.context.append_basic_block(current_fn, "for_idx_body");
        let loop_next = self.context.append_basic_block(current_fn, "for_idx_next");
        let loop_exit = self.context.append_basic_block(current_fn, "for_idx_exit");

        let saved_continue_target = self.loop_control.continue_target;
        let saved_break_target = self.loop_control.break_target;
        self.loop_control.continue_target = Some(loop_next);
        self.loop_control.break_target = Some(loop_exit);

        self.builder
            .build_unconditional_branch(loop_header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_header);
        let current_idx = self
            .builder
            .build_load(i64, idx_alloca, "idx_val")
            .map_err(llvm_err)?
            .into_int_value();
        let bound = match &mode {
            IterMode::Range { count, .. } => *count,
            IterMode::List { len, .. } => *len,
        };
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, current_idx, bound, "for_idx_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, loop_body, loop_exit)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_body);
        match &mode {
            IterMode::Range { start, .. } => {
                let item_val = self
                    .builder
                    .build_int_add(*start, current_idx, "range_item")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(item_alloca, item_val)
                    .map_err(llvm_err)?;
            }
            IterMode::List { list_ptr, .. } => {
                let cache = list_get_cache.expect("list for-with-index always has get cache");
                if let Some(str_a) = item_str_alloca {
                    let fat = self.list_get_cached_fat(*list_ptr, current_idx, cache)?;
                    self.builder.build_store(str_a, fat).map_err(llvm_err)?;
                } else {
                    let tag = self.list_get_cached_tag(*list_ptr, current_idx, cache)?;
                    self.builder
                        .build_store(item_alloca, tag)
                        .map_err(llvm_err)?;
                }
            }
        }

        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::with_parent(saved_scope);
        self.scope
            .set(index_var.clone(), idx_alloca, i64.into(), ValKind::Int);
        if let Some(str_a) = item_str_alloca {
            self.scope.set(
                item_var.clone(),
                str_a,
                self.string_type.into(),
                ValKind::Str,
            );
        } else {
            self.scope
                .set(item_var.clone(), item_alloca, i64.into(), ValKind::Int);
        }

        let body_val = body.compile(self)?;
        self.rc_discard_value(&body_val)?;

        self.builder
            .build_unconditional_branch(loop_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_next);
        let mut parent = Scope::new();
        std::mem::swap(&mut self.scope, &mut parent);
        if let Some(p) = parent.parent {
            self.scope = *p;
        }

        let next_idx = self
            .builder
            .build_load(i64, idx_alloca, "idx_next")
            .map_err(llvm_err)?
            .into_int_value();
        let one = i64.const_int(1, false);
        let inc = self
            .builder
            .build_int_add(next_idx, one, "idx_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, inc)
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_exit);
        self.loop_control.continue_target = saved_continue_target;
        self.loop_control.break_target = saved_break_target;

        Ok(TypedValue::Unit)
    }

    fn compile_for_with_index_hash(
        &mut self,
        index_var: &str,
        item_var: &str,
        data_ptr: PointerValue<'ctx>,
        cap: IntValue<'ctx>,
        use_key: bool,
        body: ForExprSrc<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function".to_string())?;

        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);

        let idx_alloca = self
            .builder
            .build_alloca(i64, "for_map_idx")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, zero)
            .map_err(llvm_err)?;

        let slot_alloca = self
            .builder
            .build_alloca(i64, "for_map_slot")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slot_alloca, zero)
            .map_err(llvm_err)?;

        let item_alloca = self
            .builder
            .build_alloca(i64, "for_map_item")
            .map_err(llvm_err)?;

        let loop_header = self
            .context
            .append_basic_block(current_fn, "for_map_idx_hdr");
        let loop_chk = self
            .context
            .append_basic_block(current_fn, "for_map_idx_chk");
        let loop_body = self
            .context
            .append_basic_block(current_fn, "for_map_idx_body");
        let loop_next = self
            .context
            .append_basic_block(current_fn, "for_map_idx_nxt");
        let loop_exit = self
            .context
            .append_basic_block(current_fn, "for_map_idx_ext");

        let saved_continue_target = self.loop_control.continue_target;
        let saved_break_target = self.loop_control.break_target;
        self.loop_control.continue_target = Some(loop_next);
        self.loop_control.break_target = Some(loop_exit);

        self.builder
            .build_unconditional_branch(loop_header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_header);
        let slot = self
            .builder
            .build_load(i64, slot_alloca, "slot_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, slot, cap, "for_map_idx_cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, loop_chk, loop_exit)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_chk);
        self.ht_branch_if_slot_active(data_ptr, slot, loop_body, loop_next)?;

        self.builder.position_at_end(loop_body);
        let item_fat = if use_key {
            self.ht_key_fat_at(data_ptr, slot)?
        } else {
            self.ht_val_fat_at(data_ptr, slot)?
        };
        let item_tag = self
            .builder
            .build_extract_value(item_fat.into_struct_value(), 0, "hash_item_tag")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(item_alloca, item_tag)
            .map_err(llvm_err)?;

        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::with_parent(saved_scope);
        self.scope
            .set(index_var.to_string(), idx_alloca, i64.into(), ValKind::Int);
        self.scope
            .set(item_var.to_string(), item_alloca, i64.into(), ValKind::Int);

        let body_val = body.compile(self)?;
        self.rc_discard_value(&body_val)?;

        let next_idx = self
            .builder
            .build_load(i64, idx_alloca, "idx_next")
            .map_err(llvm_err)?
            .into_int_value();
        let one = i64.const_int(1, false);
        let idx_inc = self
            .builder
            .build_int_add(next_idx, one, "idx_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(idx_alloca, idx_inc)
            .map_err(llvm_err)?;

        self.builder
            .build_unconditional_branch(loop_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_next);
        let mut parent = Scope::new();
        std::mem::swap(&mut self.scope, &mut parent);
        if let Some(p) = parent.parent {
            self.scope = *p;
        }

        let next_slot = self
            .builder
            .build_load(i64, slot_alloca, "slot_next")
            .map_err(llvm_err)?
            .into_int_value();
        let slot_inc = self
            .builder
            .build_int_add(next_slot, one, "slot_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slot_alloca, slot_inc)
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_header)
            .map_err(llvm_err)?;

        self.builder.position_at_end(loop_exit);
        self.loop_control.continue_target = saved_continue_target;
        self.loop_control.break_target = saved_break_target;

        Ok(TypedValue::Unit)
    }

    pub(crate) fn compile_for_nested_iterate(
        &mut self,
        bindings: &[(String, ForExprSrc<'_>)],
        body: ForExprSrc<'_>,
        collect: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile nested for outside function")?;

        let i64 = self.i64_ty();
        let saved_continue_target = self.loop_control.continue_target;
        let saved_break_target = self.loop_control.break_target;

        // Pre-allocate all loop counters and bounds: (idx_alloca, start_val, end_val)
        let mut loops: Vec<(PointerValue, IntValue, IntValue)> = Vec::new();
        for (i, (_var, iterable)) in bindings.iter().enumerate() {
            let (start, end) = if let Some((s, e)) = iterable.range_start_end(self)? {
                (s, e)
            } else {
                let (zero, len, _list_ptr) = iterable.compile_list_iterable(self)?;
                (zero, len)
            };
            let idx = self
                .builder
                .build_alloca(i64, &format!("nested_idx_{}", i))
                .map_err(llvm_err)?;
            self.builder.build_store(idx, start).map_err(llvm_err)?;
            loops.push((idx, start, end));
        }

        // Create result list if collecting
        let result_list = if collect {
            let cap = i64.const_int(16, false);
            let list_cc = self.call_rt("action_list_create", &[cap.into()])?;
            let list_bv = list_cc
                .try_as_basic_value()
                .basic()
                .ok_or("list_create failed")?;
            let ra = self
                .builder
                .build_alloca(self.list_type, "nested_result")
                .map_err(llvm_err)?;
            self.builder.build_store(ra, list_bv).map_err(llvm_err)?;
            Some(ra)
        } else {
            None
        };

        let n = loops.len();

        // Create basic blocks for each loop level
        let mut headers: Vec<BasicBlock> = Vec::with_capacity(n);
        let mut nexts: Vec<BasicBlock> = Vec::with_capacity(n);
        for i in 0..n {
            headers.push(
                self.context
                    .append_basic_block(current_fn, &format!("nh{}", i)),
            );
            nexts.push(
                self.context
                    .append_basic_block(current_fn, &format!("nn{}", i)),
            );
        }
        let innermost_body = self.context.append_basic_block(current_fn, "nested_body");
        let exit_block = self.context.append_basic_block(current_fn, "nested_exit");

        // continue targets the innermost next block so `continue` inside the inner loop body
        // increments the innermost counter (not the outermost one).
        self.loop_control.continue_target = Some(nexts[n - 1]);
        self.loop_control.break_target = Some(exit_block);

        // Branch to first header
        let _ = self.builder.build_unconditional_branch(headers[0]);

        // Build loop structure for each level
        for i in 0..n {
            self.builder.position_at_end(headers[i]);
            let (idx, _start, end) = loops[i];
            let cur_val = self
                .builder
                .build_load(i64, idx, &format!("lv{}", i))
                .map_err(llvm_err)?
                .into_int_value();
            let cond = self
                .builder
                .build_int_compare(IntPredicate::SLT, cur_val, end, &format!("lc{}", i))
                .map_err(llvm_err)?;

            // When condition fails, branch to parent's next (or exit for level 0)
            let fail_target = if i > 0 { nexts[i - 1] } else { exit_block };

            if i < n - 1 {
                let _ = self
                    .builder
                    .build_conditional_branch(cond, headers[i + 1], fail_target);
            } else {
                let _ = self
                    .builder
                    .build_conditional_branch(cond, innermost_body, fail_target);
            }

            // Build the "next" block for this level
            // (increment counter, reset inner counters, branch to this level's header)
            self.builder.position_at_end(nexts[i]);
            let cur_load = self
                .builder
                .build_load(i64, idx, &format!("nl{}", i))
                .map_err(llvm_err)?
                .into_int_value();
            let inc = self
                .builder
                .build_int_add(cur_load, i64.const_int(1, false), &format!("ni{}", i))
                .map_err(llvm_err)?;
            self.builder.build_store(idx, inc).map_err(llvm_err)?;
            // Reset all inner loop counters to their start values
            for j in (i + 1)..n {
                let (inner_idx, inner_start, _) = loops[j];
                self.builder
                    .build_store(inner_idx, inner_start)
                    .map_err(llvm_err)?;
            }
            let _ = self.builder.build_unconditional_branch(headers[i]);
        }

        // ---- Innermost body ----
        self.builder.position_at_end(innermost_body);

        // Set up scope with all binding variables
        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::with_parent(saved_scope);
        for (i, (var, _)) in bindings.iter().enumerate() {
            let (idx, _, _) = loops[i];
            self.scope.set(var.clone(), idx, i64.into(), ValKind::Int);
        }

        // Compile body
        let body_val = body.compile(self)?;

        // Collect result
        if let Some(list_ptr) = result_list {
            // action_list_push handles rc_inc of the element data_ptr internally
            let list_loaded = self.load_list(list_ptr)?;
            let elem_fat = self.to_fat_struct(&body_val)?;
            let push_cc =
                self.call_rt("action_list_push", &[list_loaded.into(), elem_fat.into()])?;
            let pushed = push_cc
                .try_as_basic_value()
                .basic()
                .ok_or("list_push failed")?;
            self.builder
                .build_store(list_ptr, pushed)
                .map_err(llvm_err)?;
        } else {
            self.rc_discard_value(&body_val)?;
        }

        // Restore scope
        let mut parent = Scope::new();
        std::mem::swap(&mut self.scope, &mut parent);
        if let Some(p) = parent.parent {
            self.scope = *p;
        }

        // Branch to the innermost next block (increment inner counter)
        let _ = self.builder.build_unconditional_branch(nexts[n - 1]);

        // ---- Exit ----
        self.builder.position_at_end(exit_block);

        self.loop_control.continue_target = saved_continue_target;
        self.loop_control.break_target = saved_break_target;

        if let Some(list_ptr) = result_list {
            Ok(TypedValue::List(list_ptr))
        } else {
            Ok(TypedValue::Unit)
        }
    }
}

impl<'a> ForExprSrc<'a> {
    fn iterate_use_map_keys(&self, var: &str) -> bool {
        self.iterate_bind_str(var)
    }

    /// Bind loop var as `ValKind::Str` when body uses `len(var)` (Map keys / List[String]).
    fn iterate_bind_str(&self, var: &str) -> bool {
        match self {
            ForExprSrc::Hir(h) => hir_body_uses_len_on_var(h, var),
        }
    }
}

fn hir_body_uses_len_on_var(expr: &HirExpr, var: &str) -> bool {
    match &expr.kind {
        HirExprKind::Call { func, args, .. } => {
            if matches!(&func.kind, HirExprKind::Ident(n) if n == "len") {
                args.first()
                    .is_some_and(|a| matches!(&a.kind, HirExprKind::Ident(n) if n == var))
            } else {
                args.iter().any(|a| hir_body_uses_len_on_var(a, var))
                    || hir_body_uses_len_on_var(func, var)
            }
        }
        HirExprKind::FieldAccess(obj, method) if method == "len" => {
            matches!(&obj.kind, HirExprKind::Ident(n) if n == var)
        }
        HirExprKind::Block(stmts) => stmts.iter().any(|s| hir_stmt_uses_len_on_var(s, var)),
        HirExprKind::Binary(lhs, _, rhs)
        | HirExprKind::Assign {
            target: lhs,
            value: rhs,
            ..
        } => hir_body_uses_len_on_var(lhs, var) || hir_body_uses_len_on_var(rhs, var),
        HirExprKind::Unary(_, inner) => hir_body_uses_len_on_var(inner, var),
        HirExprKind::When(w) => hir_when_uses_len_on_var(w, var),
        HirExprKind::For(f) => hir_for_uses_len_on_var(f, var),
        HirExprKind::OrBlock { fallible, fallback } => {
            hir_body_uses_len_on_var(fallible, var) || hir_body_uses_len_on_var(fallback, var)
        }
        HirExprKind::Lambda { body, .. } => hir_body_uses_len_on_var(body, var),
        HirExprKind::Copy(inner) | HirExprKind::Unsafe(inner) => {
            hir_body_uses_len_on_var(inner, var)
        }
        HirExprKind::StructLiteral(fields) => {
            fields.iter().any(|(_, e)| hir_body_uses_len_on_var(e, var))
        }
        HirExprKind::MapLiteral(entries) => entries
            .iter()
            .any(|(k, v)| hir_body_uses_len_on_var(k, var) || hir_body_uses_len_on_var(v, var)),
        HirExprKind::SetLiteral(items) => items.iter().any(|i| hir_body_uses_len_on_var(i, var)),
        HirExprKind::Tuple(items) => items.iter().any(|(_, v)| hir_body_uses_len_on_var(v, var)),
        HirExprKind::Index(obj, idx) => {
            hir_body_uses_len_on_var(obj, var) || hir_body_uses_len_on_var(idx, var)
        }
        HirExprKind::FieldAccess(obj, _) => hir_body_uses_len_on_var(obj, var),
        HirExprKind::Range(start, end) => {
            hir_body_uses_len_on_var(start, var) || hir_body_uses_len_on_var(end, var)
        }
        HirExprKind::StringInterpolate(parts) => parts.iter().any(|p| match p {
            action_frontend::hir::HirStringPart::Expr(e) => hir_body_uses_len_on_var(e, var),
            _ => false,
        }),
        HirExprKind::Ident(_) | HirExprKind::Literal(_) => false,
        _ => false,
    }
}

fn hir_stmt_uses_len_on_var(stmt: &HirStmt, var: &str) -> bool {
    match stmt {
        HirStmt::Expr { expr, .. } => hir_body_uses_len_on_var(expr, var),
        HirStmt::Let { value, .. } => hir_body_uses_len_on_var(value, var),
        HirStmt::Return { value: Some(v), .. } => hir_body_uses_len_on_var(v, var),
        HirStmt::Return { value: None, .. } => false,
        _ => false,
    }
}

fn hir_when_uses_len_on_var(w: &action_frontend::hir::HirWhen, var: &str) -> bool {
    use action_frontend::hir::HirWhenKind;
    match &w.kind {
        HirWhenKind::OneLine {
            then_expr,
            else_expr,
            ..
        } => hir_body_uses_len_on_var(then_expr, var) || hir_body_uses_len_on_var(else_expr, var),
        HirWhenKind::ValueMatch { value, arms } => {
            hir_body_uses_len_on_var(value, var)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .is_some_and(|g| hir_body_uses_len_on_var(g, var))
                        || hir_body_uses_len_on_var(&arm.body, var)
                })
        }
        HirWhenKind::ConditionChain { arms } => arms.iter().any(|arm| {
            arm.guard
                .as_ref()
                .is_some_and(|g| hir_body_uses_len_on_var(g, var))
                || hir_body_uses_len_on_var(&arm.body, var)
        }),
    }
}

fn hir_for_uses_len_on_var(f: &action_frontend::hir::HirFor, var: &str) -> bool {
    match &f.kind {
        HirForKind::Iterate { iterable, body, .. } => {
            hir_body_uses_len_on_var(iterable, var) || hir_body_uses_len_on_var(body, var)
        }
        HirForKind::IterateWithIndex { iterable, body, .. } => {
            hir_body_uses_len_on_var(iterable, var) || hir_body_uses_len_on_var(body, var)
        }
        HirForKind::Condition { condition, body } => {
            hir_body_uses_len_on_var(condition, var) || hir_body_uses_len_on_var(body, var)
        }
        HirForKind::Infinite { body } => hir_body_uses_len_on_var(body, var),
        HirForKind::NestedIterate { bindings, body, .. } => {
            bindings
                .iter()
                .any(|(_, e)| hir_body_uses_len_on_var(e, var))
                || hir_body_uses_len_on_var(body, var)
        }
    }
}
