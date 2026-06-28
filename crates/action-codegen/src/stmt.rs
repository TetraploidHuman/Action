// Submodule: stmt

use action_frontend::ast::*;
use action_frontend::hir::{HirExpr, HirExprKind, HirWhenKind};
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::PointerValue;
use inkwell::IntPredicate;

use super::TcoState;
use super::{llvm_err, CodeGen, Scope, TypedValue, ValKind};

enum FunBody<'a> {
    Hir(&'a action_frontend::hir::HirExpr),
}

impl<'a> FunBody<'a> {
    fn compile<'ctx>(&self, cg: &mut CodeGen<'ctx>) -> Result<TypedValue<'ctx>, String> {
        match self {
            FunBody::Hir(e) => cg.compile_hir_expr(e),
        }
    }
}

impl<'ctx> CodeGen<'ctx> {
    /// AST statement compilation (test-only; production uses [`compile_hir_stmt`]).

    /// Extract TCO state if `expr` is a tail-recursive self-call.
    /// Returns (param_slots clone, tail_entry block).

    /// Compile `when cond then then_expr else else_expr` where at least one branch is a TCO call.
    /// The non-TCO branch is compiled normally and returned; the TCO branch stores args
    /// and branches to tail_entry.

    /// Emit a return instruction for a TypedValue, handling all types.
    pub(super) fn build_return_for_value(&self, v: &TypedValue<'ctx>) -> Result<(), String> {
        if let Some(bv) = v.to_bv() {
            let _ = self.builder.build_return(Some(&bv));
            return Ok(());
        }
        match v {
            TypedValue::Str(ptr) => {
                let sv = self.load_string(*ptr)?;
                let _ = self.builder.build_return(Some(&sv));
            }
            TypedValue::Enum(ptr, ty, ..) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "ret_enum")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&loaded));
            }
            TypedValue::Struct(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "ret_struct")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&loaded));
            }
            TypedValue::Stream(ptr) => {
                let list_field = self
                    .builder
                    .build_struct_gep(self.stream_type, *ptr, 1, "ret_sl")
                    .map_err(llvm_err)?;
                let sv = self
                    .builder
                    .build_load(self.list_type, list_field, "ret_sv")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&sv));
            }
            TypedValue::Task(ptr) => {
                let sv = self
                    .builder
                    .build_load(self.task_type, *ptr, "ret_task2")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&sv));
            }
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                let sv = self.load_list(*ptr)?;
                let _ = self.builder.build_return(Some(&sv));
            }
            TypedValue::LazyList(ptr) => {
                let ll_val = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "ret_ll2")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&ll_val));
            }
            _ => {
                let _ = self.builder.build_return(None);
            }
        }
        Ok(())
    }

    /// Extract TCO state if `expr` is a tail-recursive self-call (HIR).
    pub(super) fn extract_tco_info_hir(
        &self,
        expr: &HirExpr,
    ) -> Option<(
        Vec<(
            inkwell::values::PointerValue<'ctx>,
            inkwell::types::BasicTypeEnum<'ctx>,
            ValKind,
        )>,
        inkwell::basic_block::BasicBlock<'ctx>,
    )> {
        if let HirExprKind::Call {
            func,
            args,
            trailing_lambda: None,
        } = &expr.kind
        {
            if let HirExprKind::Ident(fn_name) = &func.kind {
                if let Some(ref tco) = self.tco_state {
                    if *fn_name == tco.fn_name && args.len() <= tco.param_slots.len() {
                        return Some((tco.param_slots.clone(), tco.tail_entry));
                    }
                }
            }
        }
        None
    }

    /// Compile tail-recursive `return` without growing the stack (HIR).
    pub(super) fn try_compile_hir_return_tco(&mut self, expr: &HirExpr) -> Result<bool, String> {
        if let Some((param_slots, tail_entry)) = self.extract_tco_info_hir(expr) {
            if let HirExprKind::Call { args, .. } = &expr.kind {
                let arg_vals: Vec<TypedValue<'ctx>> = args
                    .iter()
                    .map(|a| self.compile_hir_expr(a))
                    .collect::<Result<_, _>>()?;
                for (i, arg_val) in arg_vals.iter().enumerate() {
                    let (alloca, ty, _kind) = &param_slots[i];
                    self.store_typed_value(arg_val, *alloca, *ty)?;
                }
                self.builder
                    .build_unconditional_branch(tail_entry)
                    .map_err(llvm_err)?;
                return Ok(true);
            }
        }

        if let HirExprKind::When(w) = &expr.kind {
            if let HirWhenKind::OneLine {
                condition,
                then_expr,
                else_expr,
            } = &w.kind
            {
                let then_tco = self.extract_tco_info_hir(then_expr);
                let else_tco = self.extract_tco_info_hir(else_expr);
                if then_tco.is_some() || else_tco.is_some() {
                    self.compile_tco_when_hir(
                        condition, then_expr, else_expr, &then_tco, &else_tco,
                    )?;
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// HIR variant of [`Self::compile_tco_when`].
    pub(super) fn compile_tco_when_hir(
        &mut self,
        condition: &HirExpr,
        then_expr: &HirExpr,
        else_expr: &HirExpr,
        then_tco: &Option<(
            Vec<(
                inkwell::values::PointerValue<'ctx>,
                inkwell::types::BasicTypeEnum<'ctx>,
                ValKind,
            )>,
            inkwell::basic_block::BasicBlock<'ctx>,
        )>,
        else_tco: &Option<(
            Vec<(
                inkwell::values::PointerValue<'ctx>,
                inkwell::types::BasicTypeEnum<'ctx>,
                ValKind,
            )>,
            inkwell::basic_block::BasicBlock<'ctx>,
        )>,
    ) -> Result<(), String> {
        let cond_val = self.compile_hir_expr(condition)?;
        let cond_as_bool = match cond_val {
            TypedValue::Bool(b) => b,
            _ => {
                let bv = cond_val
                    .to_bv()
                    .ok_or("When condition must be a basic value")?;
                self.builder
                    .build_int_compare(
                        IntPredicate::NE,
                        bv.into_int_value(),
                        self.i64_ty().const_int(0, false),
                        "when_cond",
                    )
                    .map_err(llvm_err)?
            }
        };

        let current_fn = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let then_block = self.context.append_basic_block(current_fn, "tco_when_then");
        let else_block = self.context.append_basic_block(current_fn, "tco_when_else");

        self.builder
            .build_conditional_branch(cond_as_bool, then_block, else_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(else_block);
        if let Some((ref param_slots, tail_entry)) = else_tco {
            if let HirExprKind::Call { args, .. } = &else_expr.kind {
                let arg_vals: Vec<TypedValue<'ctx>> = args
                    .iter()
                    .map(|a| self.compile_hir_expr(a))
                    .collect::<Result<_, _>>()?;
                for (i, arg_val) in arg_vals.iter().enumerate() {
                    let (alloca, ty, _kind) = &param_slots[i];
                    self.store_typed_value(arg_val, *alloca, *ty)?;
                }
                self.builder
                    .build_unconditional_branch(*tail_entry)
                    .map_err(llvm_err)?;
            }
        } else {
            let v = self.compile_hir_expr(else_expr)?;
            self.build_return_for_value(&v)?;
        }

        self.builder.position_at_end(then_block);
        if let Some((ref param_slots, tail_entry)) = then_tco {
            if let HirExprKind::Call { args, .. } = &then_expr.kind {
                let arg_vals: Vec<TypedValue<'ctx>> = args
                    .iter()
                    .map(|a| self.compile_hir_expr(a))
                    .collect::<Result<_, _>>()?;
                for (i, arg_val) in arg_vals.iter().enumerate() {
                    let (alloca, ty, _kind) = &param_slots[i];
                    self.store_typed_value(arg_val, *alloca, *ty)?;
                }
                self.builder
                    .build_unconditional_branch(*tail_entry)
                    .map_err(llvm_err)?;
            }
        } else {
            let v = self.compile_hir_expr(then_expr)?;
            self.build_return_for_value(&v)?;
        }

        Ok(())
    }

    pub(super) fn compile_fun_def_hir(
        &mut self,
        name: &str,
        original_name: &str,
        params: &[Param],
        return_type: Option<&Type>,
        body: &action_frontend::hir::HirExpr,
        fn_or_fallback: Option<&action_frontend::hir::HirExpr>,
    ) -> Result<(), String> {
        self.compile_fun_def_inner(
            name,
            original_name,
            params,
            return_type,
            FunBody::Hir(body),
            fn_or_fallback,
        )
    }

    fn compile_fun_def_inner(
        &mut self,
        name: &str,
        _original_name: &str,
        params: &[Param],
        _return_type: Option<&Type>,
        body: FunBody<'_>,
        fn_or_fallback: Option<&action_frontend::hir::HirExpr>,
    ) -> Result<(), String> {
        // Function was already declared in Pass 1; just look it up
        let function = self.module.get_function(name).ok_or_else(|| {
            format!(
                "Function '{}' not found in module (should have been declared in Pass 1)",
                name
            )
        })?;
        let entry = self.context.append_basic_block(function, "entry");

        // Save builder position only when resuming in-progress codegen (monomorphization).
        let saved_pos = self.builder.get_insert_block().filter(|bb| {
            let Some(parent) = bb.get_parent() else {
                return false;
            };
            if parent.get_name().to_string_lossy().starts_with("action_") {
                return false;
            }
            bb.get_terminator().is_none()
        });
        self.builder.position_at_end(entry);

        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::new();
        self.ht_result_scratch = None;
        let ht_scratch = self
            .builder
            .build_alloca(self.list_type, "ht_result_scratch")
            .map_err(llvm_err)?;
        self.ht_result_scratch = Some(ht_scratch);

        let mut param_slots: Vec<(PointerValue<'ctx>, BasicTypeEnum<'ctx>, ValKind)> = Vec::new();
        for (i, param) in params.iter().enumerate() {
            if let Some(pv) = function.get_nth_param(i as u32) {
                let alloca = self
                    .builder
                    .build_alloca(pv.get_type(), &param.name)
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, pv).map_err(llvm_err)?;
                let kind = self.param_val_kind(param.ty.as_ref());
                param_slots.push((alloca, pv.get_type(), kind));
                if let Some(Type::Function(param_tys_ast, ret_ast)) = param.ty.as_ref() {
                    let ret = Some(ret_ast.as_ref());
                    let param_llvm_tys: Vec<BasicMetadataTypeEnum> = param_tys_ast
                        .iter()
                        .map(|t| self.ast_type_to_llvm(Some(t)))
                        .collect();
                    let fn_type = self.build_fn_type(ret, name, &param_llvm_tys);
                    self.scope.set_with_fn_type(
                        param.name.clone(),
                        alloca,
                        pv.get_type(),
                        kind,
                        Some(fn_type),
                    );
                } else {
                    self.scope
                        .set(param.name.clone(), alloca, pv.get_type(), kind);
                }
                // Enum parameters carry heap-allocated data that needs RC cleanup
                if kind == ValKind::Enum {
                    self.scope.set_enum_data_rc_managed(&param.name, true);
                }
                // RC inc for heap-typed parameters so the callee holds a reference
                match kind {
                    ValKind::Str => {
                        let sv = self.load_string(alloca)?;
                        let pdata = self
                            .builder
                            .build_extract_value(sv, 1, "pdata")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        self.rc_inc(pdata)?;
                    }
                    ValKind::List | ValKind::Map | ValKind::Set => {
                        let lv = self.load_list(alloca)?;
                        let pdata = self
                            .builder
                            .build_extract_value(lv, 0, "pdata")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        self.rc_inc(pdata)?;
                    }
                    ValKind::Enum => {
                        // Enum data pointer was rc_inc'd by the caller;
                        // scope cleanup will rc_dec it when the parameter goes out of scope
                    }
                    ValKind::Struct => {
                        if let BasicTypeEnum::StructType(st) = pv.get_type() {
                            let loaded = self
                                .builder
                                .build_load(st, alloca, "param_struct_inc")
                                .map_err(llvm_err)?
                                .into_struct_value();
                            self.rc_struct_fields(loaded, st, true)?;
                        }
                    }
                    ValKind::Stream => {
                        let heap_ptr = self
                            .builder
                            .build_load(self.ptr_ty(), alloca, "stream_param_ptr")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let list_gep = self
                            .builder
                            .build_struct_gep(self.stream_type, heap_ptr, 3, "sp_list_gep")
                            .map_err(llvm_err)?;
                        let lv = self
                            .builder
                            .build_load(self.list_type, list_gep, "sp_list")
                            .map_err(llvm_err)?;
                        let pdata = self
                            .builder
                            .build_extract_value(lv.into_struct_value(), 0, "sp_data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        self.rc_inc(pdata)?;
                    }
                    ValKind::Task => {
                        let heap_ptr = self
                            .builder
                            .build_load(self.ptr_ty(), alloca, "task_param_ptr")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let list_gep = self
                            .builder
                            .build_struct_gep(self.task_type, heap_ptr, 4, "tp_list_gep")
                            .map_err(llvm_err)?;
                        let lv = self
                            .builder
                            .build_load(self.list_type, list_gep, "tp_list")
                            .map_err(llvm_err)?;
                        let pdata = self
                            .builder
                            .build_extract_value(lv.into_struct_value(), 0, "tp_data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        self.rc_inc(pdata)?;
                    }
                    _ => {}
                }
            }
        }

        let is_propagating = fn_or_fallback.is_none()
            && _original_name != "main"
            && _return_type.is_some()
            && self
                .fallibility
                .symbols
                .get(_original_name)
                .is_some_and(|s| s.is_fallible);

        let mut fn_or_fail_bb = None;
        let mut fn_or_fallback_expr = None;
        let mut fn_propagate_fail = false;

        if let Some(fb) = fn_or_fallback {
            let fail_bb = self.context.append_basic_block(function, "fn_or_fail");
            self.push_fallible_fail_bb(fail_bb);
            fn_or_fail_bb = Some(fail_bb);
            fn_or_fallback_expr = Some(fb);
        } else if is_propagating {
            let fail_bb = self.context.append_basic_block(function, "fn_prop_fail");
            self.push_fallible_fail_bb(fail_bb);
            self.propagating_fallible_ret = _return_type.cloned();
            fn_or_fail_bb = Some(fail_bb);
            fn_propagate_fail = true;
        }

        // Set up TCO: create a tail_entry block that reloads params from allocas
        let tail_entry = self.context.append_basic_block(function, "tail_entry");
        let _ = self.builder.build_unconditional_branch(tail_entry);
        self.builder.position_at_end(tail_entry);
        self.tco_state = Some(TcoState {
            tail_entry,
            param_slots,
            fn_name: _original_name.to_string(),
        });

        let mut result = body.compile(self)?;
        if fn_or_fallback.is_some() {
            result = self.unwrap_fallible_value(result)?;
        }

        // If the body already ended with a return/break/continue, the current block
        // already has a terminator — skip the fallback ret.
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| "No insert block")?;
        if current_block.get_terminator().is_none() {
            let llvm_void: bool = function.get_type().get_return_type().is_none();

            if name == "main" {
                self.emit_scope_cleanup()?;
                // Flush stdout before exit so buffered printf output is written
                // even when the program uses print() (no newline) on Windows.
                if let Some(fflush_fn) = self.module.get_function("fflush") {
                    let _ = self.builder.build_call(
                        fflush_fn,
                        &[self.ptr_ty().const_null().into()],
                        "",
                    );
                }
                let _ = self
                    .builder
                    .build_return(Some(&self.i64_ty().const_int(0, false)));
            } else if llvm_void {
                self.emit_scope_cleanup()?;
                let _ = self.builder.build_return(None);
            } else if is_propagating {
                if self.is_scope_variable(&result) {
                    self.rc_inc_typed_value(&result)?;
                }
                self.emit_scope_cleanup()?;
                if let Some(ret_ty) = _return_type {
                    self.build_fallible_ok_return(&result, ret_ty)?;
                }
            } else {
                // RC inc the return value before cleaning up scope — same
                // pattern as Stmt::Return.
                if self.is_scope_variable(&result) {
                    self.rc_inc_typed_value(&result)?;
                }
                self.emit_scope_cleanup()?;
                match &result {
                    TypedValue::Str(ptr) => {
                        let str_val = self.load_string(*ptr)?;
                        // If the function returns fat_return_type, convert
                        if function
                            .get_type()
                            .get_return_type()
                            .map_or(false, |rt| rt == self.fat_return_type.into())
                        {
                            let sv = str_val;
                            let len = self
                                .builder
                                .build_extract_value(sv, 0, "slen")
                                .map_err(llvm_err)?;
                            let data = self
                                .builder
                                .build_extract_value(sv, 1, "sdata")
                                .map_err(llvm_err)?;
                            let undef_fat = self.fat_return_type.get_undef();
                            let f1 = self
                                .builder
                                .build_insert_value(undef_fat, len, 0, "ftag")
                                .map_err(llvm_err)?;
                            let f2 = self
                                .builder
                                .build_insert_value(f1, data, 1, "fdata")
                                .map_err(llvm_err)?;
                            let _ = self.builder.build_return(Some(&f2));
                        } else {
                            let _ = self.builder.build_return(Some(&str_val));
                        }
                    }
                    TypedValue::Enum(ptr, ty, ..) => {
                        let bt: BasicTypeEnum = (*ty).into();
                        let loaded = self
                            .builder
                            .build_load(bt, *ptr, "ret_enum")
                            .map_err(llvm_err)?;
                        // If the function returns fat_return_type, convert
                        if function
                            .get_type()
                            .get_return_type()
                            .map_or(false, |rt| rt == self.fat_return_type.into())
                        {
                            let sv = loaded.into_struct_value();
                            let tag = self
                                .builder
                                .build_extract_value(sv, 0, "etag")
                                .map_err(llvm_err)?;
                            let data = self
                                .builder
                                .build_extract_value(sv, 1, "edata")
                                .map_err(llvm_err)?;
                            let undef_fat = self.fat_return_type.get_undef();
                            let f1 = self
                                .builder
                                .build_insert_value(undef_fat, tag, 0, "ftag")
                                .map_err(llvm_err)?;
                            let f2 = self
                                .builder
                                .build_insert_value(f1, data, 1, "fdata")
                                .map_err(llvm_err)?;
                            let _ = self.builder.build_return(Some(&f2));
                        } else {
                            let _ = self.builder.build_return(Some(&loaded));
                        }
                    }
                    TypedValue::Struct(ptr, ty) => {
                        let bt: BasicTypeEnum = (*ty).into();
                        let loaded = self
                            .builder
                            .build_load(bt, *ptr, "ret_struct")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_return(Some(&loaded));
                    }
                    TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                        let list_val = self.load_list(*ptr)?;
                        let _ = self.builder.build_return(Some(&list_val));
                    }
                    TypedValue::LazyList(ptr) => {
                        let ll_val = self
                            .builder
                            .build_load(self.lazylist_type, *ptr, "ret_ll")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_return(Some(&ll_val));
                    }
                    _ => {
                        if let Some(bv) = result.to_bv() {
                            // If the function returns a struct (enum, fat, etc.) but
                            // the body produced a scalar, pack it into the struct.
                            let ret_ty_opt = function.get_type().get_return_type();
                            let need_pack = ret_ty_opt.map_or(false, |rt| rt.is_struct_type());
                            if need_pack {
                                let struct_ty = ret_ty_opt
                                    .ok_or_else(|| "Missing return type".to_string())?
                                    .into_struct_type();
                                if let Some((fat_alloca, _fat_ty)) = self.last_fat_ret.take() {
                                    if struct_ty != self.fat_return_type {
                                        let ptr_ty =
                                            self.context.ptr_type(inkwell::AddressSpace::default());
                                        let cast_ptr = self
                                            .builder
                                            .build_bit_cast(fat_alloca, ptr_ty, "ret_bc")
                                            .map_err(llvm_err)?;
                                        let val = self
                                            .builder
                                            .build_load(
                                                struct_ty,
                                                cast_ptr.into_pointer_value(),
                                                "ret_cast",
                                            )
                                            .map_err(llvm_err)?;
                                        let _ = self.builder.build_return(Some(&val));
                                    } else {
                                        let alloca = self
                                            .builder
                                            .build_alloca(struct_ty, "ret_pack")
                                            .map_err(llvm_err)?;
                                        let zero = struct_ty.const_zero();
                                        self.builder.build_store(alloca, zero).map_err(llvm_err)?;
                                        let gep0 = self
                                            .builder
                                            .build_struct_gep(struct_ty, alloca, 0, "ret_pack0")
                                            .map_err(llvm_err)?;
                                        self.builder.build_store(gep0, bv).map_err(llvm_err)?;
                                        let loaded = self
                                            .builder
                                            .build_load(struct_ty, alloca, "ret_packed")
                                            .map_err(llvm_err)?;
                                        let _ = self.builder.build_return(Some(&loaded));
                                    }
                                } else {
                                    let alloca = self
                                        .builder
                                        .build_alloca(struct_ty, "ret_pack")
                                        .map_err(llvm_err)?;
                                    let zero = struct_ty.const_zero();
                                    self.builder.build_store(alloca, zero).map_err(llvm_err)?;
                                    let gep0 = self
                                        .builder
                                        .build_struct_gep(struct_ty, alloca, 0, "ret_pack0")
                                        .map_err(llvm_err)?;
                                    self.builder.build_store(gep0, bv).map_err(llvm_err)?;
                                    let loaded = self
                                        .builder
                                        .build_load(struct_ty, alloca, "ret_packed")
                                        .map_err(llvm_err)?;
                                    let _ = self.builder.build_return(Some(&loaded));
                                }
                            } else {
                                let _ = self.builder.build_return(Some(&bv));
                            }
                        } else {
                            // Unit, Str, List, etc. — return zero fat struct if needed
                            if let Some(ret_ty) = function.get_type().get_return_type() {
                                if ret_ty.is_struct_type() {
                                    let zero = ret_ty.into_struct_type().const_zero();
                                    let _ = self.builder.build_return(Some(&zero));
                                } else if ret_ty.is_int_type() {
                                    // IntTypes use const_zero for their particular bit width
                                    let zero = match ret_ty {
                                        BasicTypeEnum::IntType(it) => it.const_zero(),
                                        _ => self.i64_ty().const_int(0, false),
                                    };
                                    let _ = self.builder.build_return(Some(&zero));
                                } else if ret_ty.is_float_type() {
                                    let zero = match ret_ty {
                                        BasicTypeEnum::FloatType(ft) => ft.const_zero(),
                                        _ => self.f64_ty().const_zero(),
                                    };
                                    let _ = self.builder.build_return(Some(&zero));
                                } else if ret_ty.is_pointer_type() {
                                    let _ = self
                                        .builder
                                        .build_return(Some(&self.ptr_ty().const_null()));
                                } else {
                                    let _ = self.builder.build_return(None);
                                }
                            } else {
                                let _ = self.builder.build_return(None);
                            }
                        }
                    }
                }
            }
        }

        if let Some(fail_bb) = fn_or_fail_bb {
            self.pop_fallible_fail_bb();
            self.propagating_fallible_ret = None;
            self.builder.position_at_end(fail_bb);
            if let Some(fallback) = fn_or_fallback_expr {
                self.compile_fn_or_fallback_return(fallback)?;
            } else if fn_propagate_fail {
                if let Some(ret_ty) = _return_type {
                    self.emit_scope_cleanup()?;
                    self.build_fallible_fail_return(ret_ty)?;
                }
            }
        }

        // Note: don't call add_function here — it was already declared in Pass 1

        self.tco_state = None;
        self.scope = saved_scope;

        // Restore builder position
        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        } else {
            self.detach_builder()?;
        }

        Ok(())
    }

    pub(super) fn ast_type_to_llvm(
        &mut self,
        ty: Option<&Type>,
    ) -> inkwell::types::BasicMetadataTypeEnum<'ctx> {
        match ty {
            None | Some(Type::Unit) => self.i64_ty().into(),
            Some(Type::Named(n)) => match n.as_str() {
                "Float" | "Double" => self.f64_ty().into(),
                "Bool" => self.bool_ty().into(),
                "String" | "Str" => self.string_type.into(),
                "Unit" => self.i64_ty().into(),
                name => {
                    if let Some(st) = self.type_layout.named_structs.get(name) {
                        (*st).into()
                    } else if let Some(et) = self.type_layout.enum_types.get(name) {
                        (*et).into()
                    } else {
                        self.i64_ty().into()
                    }
                }
            },
            Some(Type::Function(_, _)) => self.ptr_ty().into(),
            Some(Type::Generic(base, _)) => match base.as_ref() {
                Type::Named(n) => match n.as_str() {
                    "list" | "set" | "map" => self.list_type.into(),
                    "Task" => self.task_type.into(),
                    "Stream" => self.ptr_ty().into(),
                    "LazyList" => self.lazylist_type.into(),
                    "Ptr" => self.ptr_ty().into(),
                    _ => self.i64_ty().into(),
                },
                _ => self.i64_ty().into(),
            },
            _ => self.i64_ty().into(),
        }
    }

    pub(super) fn ast_type_to_basic_type(&mut self, ty: &Type) -> BasicTypeEnum<'ctx> {
        match ty {
            Type::Named(n) => match n.as_str() {
                "Int" => self.i64_ty().into(),
                "Float" | "Double" => self.f64_ty().into(),
                "Bool" => self.bool_ty().into(),
                "String" | "Str" => self.string_type.into(),
                "Unit" => self.i64_ty().into(),
                "list" | "set" | "map" => self.list_type.into(),
                "LazyList" => self.lazylist_type.into(),
                "Task" => self.task_type.into(),
                "Stream" => self.ptr_ty().into(),
                "Ptr" | "CString" | "FileHandle" => self.ptr_ty().into(),
                name => {
                    if let Some(st) = self.type_layout.named_structs.get(name) {
                        (*st).into()
                    } else if let Some(et) = self.type_layout.enum_types.get(name) {
                        (*et).into()
                    } else {
                        self.i64_ty().into()
                    }
                }
            },
            Type::Struct(fields) => {
                let field_tys: Vec<BasicTypeEnum> = fields
                    .iter()
                    .map(|(_, fty)| self.ast_type_to_basic_type(fty))
                    .collect();
                self.context.struct_type(&field_tys, false).into()
            }
            Type::Function(_, _) => self.ptr_ty().into(),
            Type::Map(_, _) => self.list_type.into(),
            Type::Set(_) => self.list_type.into(),
            Type::Task(_) => self.task_type.into(),
            Type::Stream(_) => self.ptr_ty().into(),
            Type::LazyList(_) => self.lazylist_type.into(),
            Type::CString | Type::Ptr(_) | Type::FileHandle => self.ptr_ty().into(),
            Type::Generic(base, _) => match base.as_ref() {
                Type::Named(n) => match n.as_str() {
                    "list" => return self.list_type.into(),
                    "set" => return self.list_type.into(),
                    "map" => return self.list_type.into(),
                    "Task" => return self.task_type.into(),
                    "Stream" => return self.ptr_ty().into(),
                    "LazyList" => return self.lazylist_type.into(),
                    "Ptr" => return self.ptr_ty().into(),
                    _ => self.i64_ty().into(),
                },
                _ => self.i64_ty().into(),
            },
            _ => self.i64_ty().into(),
        }
    }

    pub(super) fn param_val_kind(&self, ty: Option<&Type>) -> ValKind {
        match ty {
            Some(Type::Named(n)) => match n.as_str() {
                "Float" => ValKind::Float,
                "Bool" => ValKind::Bool,
                "String" | "Str" => ValKind::Str,
                name => {
                    if self.type_layout.named_structs.contains_key(name) {
                        ValKind::Struct
                    } else if self.type_layout.enum_types.contains_key(name) {
                        ValKind::Enum
                    } else {
                        ValKind::Int
                    }
                }
            },
            Some(Type::Function(_, _)) => ValKind::Fn,
            Some(Type::Map(_, _)) => ValKind::Map,
            Some(Type::Set(_)) => ValKind::Set,
            Some(Type::Task(_)) => ValKind::Task,
            Some(Type::Stream(_)) => ValKind::Stream,
            Some(Type::LazyList(_)) => ValKind::LazyList,
            Some(Type::Generic(base, _)) => match base.as_ref() {
                Type::Named(n) => match n.as_str() {
                    "Float" => ValKind::Float,
                    "Bool" => ValKind::Bool,
                    "String" | "Str" => ValKind::Str,
                    "list" => ValKind::List,
                    "set" => ValKind::Set,
                    "map" => ValKind::Map,
                    "Task" => ValKind::Task,
                    "Stream" => ValKind::Stream,
                    "LazyList" => ValKind::LazyList,
                    "Ptr" => ValKind::Ptr,
                    _ => ValKind::Int,
                },
                _ => ValKind::Int,
            },
            _ => ValKind::Int,
        }
    }

    // ---- Expressions (all &mut self since they may assign) ----
}
