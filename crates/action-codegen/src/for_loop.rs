// Submodule: for_loop

use action_frontend::ast::*;
use action_frontend::hir::{HirExpr, HirExprKind};
use inkwell::basic_block::BasicBlock;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValueEnum, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, Scope, TypedValue, ValKind};

pub(super) enum ForExprSrc<'a> {
    Ast(&'a Expr),
    Hir(&'a HirExpr),
}

impl<'a> ForExprSrc<'a> {
    fn compile<'ctx>(&self, gen: &mut CodeGen<'ctx>) -> Result<TypedValue<'ctx>, String> {
        match self {
            ForExprSrc::Ast(e) => gen.compile_expr(e),
            ForExprSrc::Hir(h) => gen.compile_hir_expr(h),
        }
    }

    fn range_start_end<'ctx>(
        &self,
        gen: &mut CodeGen<'ctx>,
    ) -> Result<Option<(IntValue<'ctx>, IntValue<'ctx>)>, String> {
        match self {
            ForExprSrc::Ast(e) => match &e.kind {
                ExprKind::Binary(lhs, BinaryOp::Range, rhs)
                | ExprKind::Binary(lhs, BinaryOp::RangeExclusive, rhs) => {
                    let start_v = gen.compile_expr(lhs)?;
                    let end_v = gen.compile_expr(rhs)?;
                    match (start_v, end_v) {
                        (TypedValue::Int(s), TypedValue::Int(e)) => Ok(Some((s, e))),
                        _ => Err("Range bounds must be integers".to_string()),
                    }
                }
                _ => Ok(None),
            },
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

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn store_value_to_alloca(
        &mut self,
        v: &TypedValue<'ctx>,
        alloca: PointerValue<'ctx>,
    ) -> Result<(), String> {
        match v {
            TypedValue::Str(ptr) => {
                let str_val = self.load_string(*ptr)?;
                self.builder
                    .build_store(alloca, str_val)
                    .map_err(llvm_err)?;
            }
            TypedValue::List(ptr) => {
                let list_val = self.load_list(*ptr)?;
                self.builder
                    .build_store(alloca, list_val)
                    .map_err(llvm_err)?;
            }
            TypedValue::Map(ptr) => {
                let map_val = self.load_list(*ptr)?;
                self.builder
                    .build_store(alloca, map_val)
                    .map_err(llvm_err)?;
            }
            TypedValue::Set(ptr) => {
                let set_val = self.load_list(*ptr)?;
                self.builder
                    .build_store(alloca, set_val)
                    .map_err(llvm_err)?;
            }
            TypedValue::Task(ptr) => {
                self.builder.build_store(alloca, *ptr).map_err(llvm_err)?;
            }
            TypedValue::Stream(ptr) => {
                self.builder.build_store(alloca, *ptr).map_err(llvm_err)?;
            }
            TypedValue::LazyList(ptr) => {
                let ll_val = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "ll_ld")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, ll_val).map_err(llvm_err)?;
            }
            TypedValue::CString(p) | TypedValue::Ptr(p) | TypedValue::FileHandle(p) => {
                self.builder.build_store(alloca, *p).map_err(llvm_err)?;
            }
            TypedValue::Struct(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "struct_ld")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
            }
            TypedValue::Enum(ptr, ty, ..) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "enum_ld")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
            }
            TypedValue::Nullable(ptr, ty) => {
                let loaded = self
                    .builder
                    .build_load(*ty, *ptr, "nullable_ld")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
            }
            _ => {
                if let Some(bv) = v.to_bv() {
                    self.builder.build_store(alloca, bv).map_err(llvm_err)?;
                }
            }
        }
        Ok(())
    }

    /// Store a TypedValue to an alloca, coercing types when the alloca type differs.
    pub(super) fn store_typed_value(
        &mut self,
        v: &TypedValue<'ctx>,
        alloca: PointerValue<'ctx>,
        target_ty: BasicTypeEnum<'ctx>,
    ) -> Result<(), String> {
        match (v, target_ty) {
            // Int -> Float coercion
            (TypedValue::Int(iv), BasicTypeEnum::FloatType(_)) => {
                let fv = self
                    .builder
                    .build_signed_int_to_float(*iv, self.f64_ty(), "int2float")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, fv).map_err(llvm_err)?;
            }
            // Float -> Int coercion
            (TypedValue::Float(fv), BasicTypeEnum::IntType(_)) => {
                let iv = self
                    .builder
                    .build_float_to_signed_int(*fv, self.i64_ty(), "float2int")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, iv).map_err(llvm_err)?;
            }
            _ => self.store_value_to_alloca(v, alloca)?,
        }
        Ok(())
    }

    pub(super) fn compile_for(&mut self, f: &For) -> Result<TypedValue<'ctx>, String> {
        match &f.kind {
            ForKind::Iterate {
                var,
                iterable,
                body,
                collect,
                ..
            } => self.compile_for_iterate(
                var,
                ForExprSrc::Ast(iterable),
                ForExprSrc::Ast(body),
                *collect,
            ),
            ForKind::Condition {
                condition, body, ..
            } => {
                if let Some(result) = self.try_compile_for_sequential_list_get(condition, body)? {
                    return Ok(result);
                }
                self.compile_for_condition(condition, body)
            }
            ForKind::Infinite { body, .. } => self.compile_for_infinite(body),
            ForKind::NestedIterate {
                bindings,
                body,
                collect,
            } => self.compile_for_nested_iterate(
                &bindings
                    .iter()
                    .map(|(n, e)| (n.clone(), ForExprSrc::Ast(e)))
                    .collect::<Vec<_>>(),
                ForExprSrc::Ast(body),
                *collect,
            ),
            ForKind::IterateWithIndex {
                vars,
                iterable,
                body,
            } => {
                self.compile_for_with_index(vars, ForExprSrc::Ast(iterable), ForExprSrc::Ast(body))
            }
        }
    }

    pub(super) fn compile_for_condition(
        &mut self,
        condition: &Expr,
        body: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;

        let header = self.context.append_basic_block(current_fn, "for_cond_hdr");
        let body_block = self.context.append_basic_block(current_fn, "for_cond_body");
        let exit = self.context.append_basic_block(current_fn, "for_cond_exit");

        let saved_continue = self.continue_target;
        let saved_break = self.break_target;
        self.continue_target = Some(header);
        self.break_target = Some(exit);

        let _ = self.builder.build_unconditional_branch(header);
        self.builder.position_at_end(header);
        let cv = self.compile_expr(condition)?;
        let cond_val = match cv {
            TypedValue::Bool(b) => b,
            TypedValue::Int(v) => self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::NE,
                    v,
                    self.i64_ty().const_int(0, false),
                    "cond",
                )
                .map_err(llvm_err)?,
            _ => return Err("for condition must evaluate to Bool or Int".to_string()),
        };
        let _ = self
            .builder
            .build_conditional_branch(cond_val, body_block, exit);

        self.builder.position_at_end(body_block);
        let body_val = self.compile_expr(body)?;
        self.rc_discard_value(&body_val)?;
        let _ = self.builder.build_unconditional_branch(header);

        self.builder.position_at_end(exit);
        self.continue_target = saved_continue;
        self.break_target = saved_break;

        Ok(TypedValue::Unit)
    }

    pub(super) fn compile_for_infinite(&mut self, body: &Expr) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;

        let body_block = self.context.append_basic_block(current_fn, "for_inf_body");
        let exit = self.context.append_basic_block(current_fn, "for_inf_exit");

        let saved_continue = self.continue_target;
        let saved_break = self.break_target;
        self.continue_target = Some(body_block);
        self.break_target = Some(exit);

        let _ = self.builder.build_unconditional_branch(body_block);
        self.builder.position_at_end(body_block);
        let body_val = self.compile_expr(body)?;
        self.rc_discard_value(&body_val)?;
        let _ = self.builder.build_unconditional_branch(body_block);

        self.builder.position_at_end(exit);
        self.continue_target = saved_continue;
        self.break_target = saved_break;

        Ok(TypedValue::Unit)
    }

    pub(super) fn compile_for_iterate(
        &mut self,
        variable: &str,
        iterator: ForExprSrc<'_>,
        body: ForExprSrc<'_>,
        collect: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function".to_string())?;

        let i64 = self.i64_ty();

        // Determine iteration kind: range or list
        let (start_val, end_val, input_list_ptr) =
            if let Some((s, e)) = iterator.range_start_end(self)? {
                (s, e, None)
            } else {
                let (zero, len, list_ptr) = iterator.compile_list_iterable(self)?;
                (zero, len, Some(list_ptr))
            };

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

        // For list iteration, allocate separate element value storage (fat struct {i64, ptr})
        let val_alloca = if input_list_ptr.is_some() {
            Some(
                self.builder
                    .build_alloca(i64, "for_val")
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
        let saved_continue_target = self.continue_target;
        let saved_break_target = self.break_target;
        self.continue_target = Some(loop_next);
        self.break_target = Some(loop_exit);

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
        if let (Some(va), Some(list_ptr), Some(cache)) =
            (val_alloca, input_list_ptr, list_get_cache)
        {
            let tag = self.list_get_cached_tag(list_ptr, current, cache)?;
            self.builder.build_store(va, tag).map_err(llvm_err)?;
        }

        // Add loop variable to scope
        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::with_parent(saved_scope);
        if let Some(va) = val_alloca {
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
        self.continue_target = saved_continue_target;
        self.break_target = saved_break_target;

        if let Some(list_ptr) = result_list {
            Ok(TypedValue::List(list_ptr))
        } else {
            Ok(TypedValue::Unit)
        }
    }

    pub(super) fn compile_for_with_index(
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

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function".to_string())?;

        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);

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

        let mode = if let Some((start, end)) = iterator.range_start_end(self)? {
            let count = self
                .builder
                .build_int_sub(end, start, "range_count")
                .map_err(llvm_err)?;
            IterMode::Range { start, count }
        } else {
            let (_, len, list_ptr) = iterator.compile_list_iterable(self)?;
            IterMode::List { list_ptr, len }
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

        let loop_header = self.context.append_basic_block(current_fn, "for_idx_hdr");
        let loop_body = self.context.append_basic_block(current_fn, "for_idx_body");
        let loop_next = self.context.append_basic_block(current_fn, "for_idx_next");
        let loop_exit = self.context.append_basic_block(current_fn, "for_idx_exit");

        let saved_continue_target = self.continue_target;
        let saved_break_target = self.break_target;
        self.continue_target = Some(loop_next);
        self.break_target = Some(loop_exit);

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
                if let Some(cache) = list_get_cache {
                    let tag = self.list_get_cached_tag(*list_ptr, current_idx, cache)?;
                    self.builder
                        .build_store(item_alloca, tag)
                        .map_err(llvm_err)?;
                } else {
                    let loaded = self.load_list(*list_ptr)?;
                    let list_get_cc =
                        self.call_rt("action_list_get", &[loaded.into(), current_idx.into()])?;
                    let fat_elem = list_get_cc
                        .try_as_basic_value()
                        .basic()
                        .ok_or("list_get failed")?;
                    let tag = self
                        .builder
                        .build_extract_value(fat_elem.into_struct_value(), 0, "elem_tag")
                        .map_err(llvm_err)?;
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
        self.scope
            .set(item_var.clone(), item_alloca, i64.into(), ValKind::Int);

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
        self.continue_target = saved_continue_target;
        self.break_target = saved_break_target;

        Ok(TypedValue::Unit)
    }

    pub(super) fn compile_for_nested_iterate(
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
        let saved_continue_target = self.continue_target;
        let saved_break_target = self.break_target;

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
        self.continue_target = Some(nexts[n - 1]);
        self.break_target = Some(exit_block);

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

        self.continue_target = saved_continue_target;
        self.break_target = saved_break_target;

        if let Some(list_ptr) = result_list {
            Ok(TypedValue::List(list_ptr))
        } else {
            Ok(TypedValue::Unit)
        }
    }

    /// Cache alloca for action_list_get_cached: 32 bytes {valid, last_idx, leaf, pos}.
    pub(super) fn alloc_list_get_cache(&mut self) -> Result<PointerValue<'ctx>, String> {
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

    fn list_get_cached_tag(
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
    pub(super) fn list_get_cached_fat(
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

    /// `for idx < end { lst.get(idx); idx = idx + 1 }` — cached sequential walk.
    fn try_compile_for_sequential_list_get(
        &mut self,
        condition: &Expr,
        body: &Expr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let (idx_var, end_expr): (String, Expr) = match &condition.kind {
            ExprKind::Binary(lhs, BinaryOp::Lt, rhs) => match (&lhs.kind, &rhs.kind) {
                (ExprKind::Ident(v), end) => (v.clone(), ExprKind::clone(end).into()),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_expr, get_idx_var) = match Self::find_list_get_in_expr(body) {
            Some(v) => v,
            None => return Ok(None),
        };
        if get_idx_var != idx_var {
            return Ok(None);
        }
        if !Self::body_increments_var(body, &idx_var) {
            return Ok(None);
        }

        let list_val = self.compile_expr(&list_expr)?;
        let list_ptr = match &list_val {
            TypedValue::List(p) => *p,
            _ => return Ok(None),
        };

        let end_val = self.compile_expr(&end_expr)?;
        let end_bound = match end_val {
            TypedValue::Int(v) => v,
            _ => return Ok(None),
        };

        self.compile_sequential_list_get_loop(list_ptr, end_bound)
            .map(Some)
    }

    fn compile_sequential_list_get_loop(
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
        let saved_continue = self.continue_target;
        let saved_break = self.break_target;
        self.continue_target = Some(header);
        self.break_target = Some(exit);
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
        self.continue_target = saved_continue;
        self.break_target = saved_break;
        Ok(TypedValue::Unit)
    }

    fn find_list_get_in_expr(body: &Expr) -> Option<(Expr, String)> {
        match &body.kind {
            ExprKind::Block(stmts) => {
                for stmt in stmts {
                    if let Some(v) = Self::find_list_get_in_stmt(stmt) {
                        return Some(v);
                    }
                }
                None
            }
            other => Self::find_list_get_in_expr_inner(&Expr::from(other.clone())),
        }
    }

    fn find_list_get_in_stmt(stmt: &Stmt) -> Option<(Expr, String)> {
        match stmt {
            Stmt::Let { value, .. } => Self::find_list_get_in_expr_inner(&value.kind),
            Stmt::Expr { expr, .. } => Self::find_list_get_in_expr_inner(&expr.kind),
            _ => None,
        }
    }

    fn find_list_get_in_expr_inner(kind: &ExprKind) -> Option<(Expr, String)> {
        match kind {
            ExprKind::Call { func, args, .. } => {
                if let ExprKind::FieldAccess(obj, method) = &func.kind {
                    if method == "get" && args.len() == 1 {
                        if let ExprKind::Ident(idx) = &args[0].kind {
                            return Some(((*obj.clone()).clone(), idx.clone()));
                        }
                    }
                }
                None
            }
            ExprKind::Block(stmts) => {
                Self::find_list_get_in_expr(&ExprKind::Block(stmts.clone()).into())
            }
            _ => None,
        }
    }

    fn body_increments_var(body: &Expr, var: &str) -> bool {
        match &body.kind {
            ExprKind::Block(stmts) => stmts.iter().any(|s| Self::stmt_increments_var(s, var)),
            ExprKind::Assign { target, value } => Self::is_var_increment(target, value, var),
            _ => false,
        }
    }

    fn stmt_increments_var(stmt: &Stmt, var: &str) -> bool {
        match stmt {
            Stmt::Expr { expr, .. } => match &expr.kind {
                ExprKind::Assign { target, value } => Self::is_var_increment(target, value, var),
                _ => false,
            },
            _ => false,
        }
    }

    fn is_var_increment(target: &Expr, value: &Expr, var: &str) -> bool {
        match (&target.kind, &value.kind) {
            (ExprKind::Ident(t), ExprKind::Binary(lhs, BinaryOp::Add, rhs)) if t == var => {
                matches!(&lhs.kind, ExprKind::Ident(v) if v == var)
                    || matches!(&rhs.kind, ExprKind::Ident(v) if v == var)
            }
            _ => false,
        }
    }

    pub(super) fn compile_hir_for(
        &mut self,
        f: &action_frontend::hir::HirFor,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirForKind;
        match &f.kind {
            HirForKind::Iterate {
                var,
                iterable,
                body,
                collect,
            } => self.compile_for_iterate_hir(var, iterable, body, *collect),
            HirForKind::Condition { condition, body } => {
                if let Some(result) =
                    self.try_compile_for_sequential_list_get_hir(condition, body)?
                {
                    return Ok(result);
                }
                self.compile_for_condition_hir(condition, body)
            }
            HirForKind::Infinite { body } => self.compile_for_infinite_hir(body),
            HirForKind::IterateWithIndex {
                vars,
                iterable,
                body,
            } => self.compile_for_with_index_hir(vars, iterable, body),
            HirForKind::NestedIterate {
                bindings,
                body,
                collect,
            } => self.compile_for_nested_iterate_hir(bindings, body, *collect),
        }
    }

    fn compile_for_condition_hir(
        &mut self,
        condition: &action_frontend::hir::HirExpr,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;

        let header = self.context.append_basic_block(current_fn, "for_cond_hdr");
        let body_block = self.context.append_basic_block(current_fn, "for_cond_body");
        let exit = self.context.append_basic_block(current_fn, "for_cond_exit");

        let saved_continue = self.continue_target;
        let saved_break = self.break_target;
        self.continue_target = Some(header);
        self.break_target = Some(exit);

        let _ = self.builder.build_unconditional_branch(header);
        self.builder.position_at_end(header);
        let cv = self.compile_hir_expr(condition)?;
        let cond_val = match cv {
            TypedValue::Bool(b) => b,
            TypedValue::Int(v) => self
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::NE,
                    v,
                    self.i64_ty().const_int(0, false),
                    "cond",
                )
                .map_err(llvm_err)?,
            _ => return Err("for condition must evaluate to Bool or Int".to_string()),
        };
        let _ = self
            .builder
            .build_conditional_branch(cond_val, body_block, exit);

        self.builder.position_at_end(body_block);
        let body_val = self.compile_hir_expr(body)?;
        self.rc_discard_value(&body_val)?;
        let _ = self.builder.build_unconditional_branch(header);

        self.builder.position_at_end(exit);
        self.continue_target = saved_continue;
        self.break_target = saved_break;

        Ok(TypedValue::Unit)
    }

    fn compile_for_infinite_hir(
        &mut self,
        body: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile for outside function")?;

        let body_block = self.context.append_basic_block(current_fn, "for_inf_body");
        let exit = self.context.append_basic_block(current_fn, "for_inf_exit");

        let saved_continue = self.continue_target;
        let saved_break = self.break_target;
        self.continue_target = Some(body_block);
        self.break_target = Some(exit);

        let _ = self.builder.build_unconditional_branch(body_block);
        self.builder.position_at_end(body_block);
        let body_val = self.compile_hir_expr(body)?;
        self.rc_discard_value(&body_val)?;
        let _ = self.builder.build_unconditional_branch(body_block);

        self.builder.position_at_end(exit);
        self.continue_target = saved_continue;
        self.break_target = saved_break;

        Ok(TypedValue::Unit)
    }

    fn compile_for_iterate_hir(
        &mut self,
        variable: &str,
        iterator: &HirExpr,
        body: &HirExpr,
        collect: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_for_iterate(
            variable,
            ForExprSrc::Hir(iterator),
            ForExprSrc::Hir(body),
            collect,
        )
    }

    fn compile_for_with_index_hir(
        &mut self,
        vars: &[String],
        iterator: &HirExpr,
        body: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_for_with_index(vars, ForExprSrc::Hir(iterator), ForExprSrc::Hir(body))
    }

    fn compile_for_nested_iterate_hir(
        &mut self,
        bindings: &[(String, HirExpr)],
        body: &HirExpr,
        collect: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        let hir_bindings: Vec<(String, ForExprSrc<'_>)> = bindings
            .iter()
            .map(|(n, e)| (n.clone(), ForExprSrc::Hir(e)))
            .collect();
        self.compile_for_nested_iterate(&hir_bindings, ForExprSrc::Hir(body), collect)
    }

    fn try_compile_for_sequential_list_get_hir(
        &mut self,
        condition: &HirExpr,
        body: &HirExpr,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;
        let (idx_var, end_hir): (String, HirExpr) = match &condition.kind {
            HirExprKind::Binary(lhs, BinaryOp::Lt, rhs) => match (&lhs.kind, &rhs.kind) {
                (HirExprKind::Ident(v), _) => (v.clone(), rhs.as_ref().clone()),
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let (list_hir, get_idx_var) = match Self::find_list_get_in_hir(body) {
            Some(v) => v,
            None => return Ok(None),
        };
        if get_idx_var != idx_var {
            return Ok(None);
        }
        if !Self::body_increments_var_hir(body, &idx_var) {
            return Ok(None);
        }
        let list_val = self.compile_hir_expr(&list_hir)?;
        let list_ptr = match &list_val {
            TypedValue::List(p) => *p,
            _ => return Ok(None),
        };
        let end_val = self.compile_hir_expr(&end_hir)?;
        let end_bound = match end_val {
            TypedValue::Int(v) => v,
            _ => return Ok(None),
        };
        self.compile_sequential_list_get_loop(list_ptr, end_bound)
            .map(Some)
    }

    fn find_list_get_in_hir(body: &HirExpr) -> Option<(HirExpr, String)> {
        use action_frontend::hir::HirExprKind;
        match &body.kind {
            HirExprKind::Block(stmts) => {
                for stmt in stmts {
                    if let Some(v) = Self::find_list_get_in_hir_stmt(stmt) {
                        return Some(v);
                    }
                }
                None
            }
            _ => Self::find_list_get_in_hir_inner(body),
        }
    }

    fn find_list_get_in_hir_stmt(
        stmt: &action_frontend::hir::HirStmt,
    ) -> Option<(HirExpr, String)> {
        use action_frontend::hir::{HirExprKind, HirStmt};
        match stmt {
            HirStmt::Let { value, .. } => Self::find_list_get_in_hir_inner(value),
            HirStmt::Expr { expr, .. } => Self::find_list_get_in_hir_inner(expr),
            _ => None,
        }
    }

    fn find_list_get_in_hir_inner(expr: &HirExpr) -> Option<(HirExpr, String)> {
        use action_frontend::hir::HirExprKind;
        match &expr.kind {
            HirExprKind::Call { func, args, .. } => {
                if let HirExprKind::FieldAccess(obj, method) = &func.kind {
                    if method == "get" && args.len() == 1 {
                        if let HirExprKind::Ident(idx) = &args[0].kind {
                            return Some((obj.as_ref().clone(), idx.clone()));
                        }
                    }
                }
                None
            }
            HirExprKind::Block(_) => Self::find_list_get_in_hir(expr),
            _ => None,
        }
    }

    fn body_increments_var_hir(body: &HirExpr, var: &str) -> bool {
        use action_frontend::hir::{HirExprKind, HirStmt};
        match &body.kind {
            HirExprKind::Block(stmts) => {
                stmts.iter().any(|s| Self::hir_stmt_increments_var(s, var))
            }
            HirExprKind::Assign { target, value } => Self::is_var_increment_hir(target, value, var),
            _ => false,
        }
    }

    fn hir_stmt_increments_var(stmt: &action_frontend::hir::HirStmt, var: &str) -> bool {
        use action_frontend::hir::{HirExprKind, HirStmt};
        match stmt {
            HirStmt::Expr { expr, .. } => match &expr.kind {
                HirExprKind::Assign { target, value } => {
                    Self::is_var_increment_hir(target, value, var)
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn is_var_increment_hir(target: &HirExpr, value: &HirExpr, var: &str) -> bool {
        use action_frontend::ast::BinaryOp;
        use action_frontend::hir::HirExprKind;
        match (&target.kind, &value.kind) {
            (HirExprKind::Ident(t), HirExprKind::Binary(lhs, BinaryOp::Add, rhs)) if t == var => {
                matches!(&lhs.kind, HirExprKind::Ident(v) if v == var)
                    || matches!(&rhs.kind, HirExprKind::Ident(v) if v == var)
            }
            _ => false,
        }
    }
}
