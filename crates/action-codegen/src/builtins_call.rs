// Submodule: builtins_call

use super::call_arg::CallArg;
use action_frontend::ast::*;
use action_frontend::builtin::UfcsReceiverKind;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_call(
        &mut self,
        func: &Expr,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let call_args = Self::call_args_from_ast(args);
        let trailing_ca = Self::trailing_call_arg(trailing);
        // Named calls: shared router (HIR/AST) with fallthrough for higher-order targets
        if let ExprKind::Ident(name) = &func.kind {
            match self.dispatch_named_call(name, &call_args, trailing_ca) {
                Ok(v) => return Ok(v),
                Err(e) if e.starts_with("Unknown function or builtin:") => {}
                Err(e) => return Err(e),
            }
        }

        // Module-qualified call: module.function(args) → module_function(args)
        if let ExprKind::FieldAccess(module_expr, method) = &func.kind {
            if let ExprKind::Ident(module_name) = &module_expr.kind {
                // List.of(...) → List[...] (equivalent to list literal)
                if module_name == "list" && method == "of" {
                    return self.builtin_list(&call_args);
                }
                // Set.of(...) → Set literal
                if module_name == "set" && method == "of" {
                    return self.builtin_set_of(&call_args);
                }
                let mangled = format!("{}_{}", module_name, method);
                match self.dispatch_named_call(&mangled, &call_args, trailing_ca) {
                    Ok(v) => return Ok(v),
                    Err(e) if e.starts_with("Unknown function or builtin:") => {}
                    Err(e) => return Err(e),
                }
            }
        }

        // UFCS method call: receiver.method(args) → TypeName_method(receiver, args)
        if let ExprKind::FieldAccess(receiver, method) = &func.kind {
            // Fuse `.map { }.filter { }` before compiling the map receiver (avoids mono_map + walk).
            if method == "filter" {
                if let Some((map_fn_expr, inner_list_expr)) = Self::extract_map_call_args(receiver)
                {
                    let filter_fn_val = if let Some(lam) = trailing {
                        self.compile_expr(lam)?
                    } else if args.len() == 1 {
                        self.compile_expr(&args[0])?
                    } else {
                        return Err(
                            "filter with trailing lambda expects 0 args; filter(fn, list) expects 2"
                                .to_string(),
                        );
                    };
                    return self.fused_map_filter(map_fn_expr, inner_list_expr, filter_fn_val);
                }
                if let Some((flat_fn_expr, inner_list_expr)) =
                    Self::extract_flatmap_call_args(receiver)
                {
                    let filter_fn_val = if let Some(lam) = trailing {
                        self.compile_expr(lam)?
                    } else if args.len() == 1 {
                        self.compile_expr(&args[0])?
                    } else {
                        return Err(
                            "filter with trailing lambda expects 0 args; filter(fn, list) expects 2"
                                .to_string(),
                        );
                    };
                    return self.fused_flatmap_filter_ast(
                        flat_fn_expr,
                        inner_list_expr,
                        filter_fn_val,
                    );
                }
            }

            return self.compile_ufcs_method(
                CallArg::ast(receiver.as_ref()),
                method,
                &call_args,
                trailing_ca,
            );
        }

        // Higher-order call: compile the call target expression
        let target = self.compile_expr(func)?;
        self.compile_indirect_call(target, args, trailing)
    }

    /// Perform an indirect function call through a TypedValue::Fn, TypedValue::Closure, or TypedValue::Int.
    pub(super) fn compile_indirect_call(
        &mut self,
        target: TypedValue<'ctx>,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        match target {
            TypedValue::Fn(fn_ptr, fn_type) => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                for a in args {
                    let av = self.compile_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    ca.push(bv.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    ca.push(bv.into());
                }

                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect")
                    .map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            TypedValue::Closure {
                fn_ptr,
                actual_fn_type,
                closure_ptr,
                closure_ty,
                alloca,
            } => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                // First arg: the closure struct pointer (captures context)
                ca.push(closure_ptr.into());
                for a in args {
                    let av = self.compile_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    ca.push(bv.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    ca.push(bv.into());
                }

                let cc = self
                    .builder
                    .build_indirect_call(actual_fn_type, fn_ptr, &ca, "indirect_closure")
                    .map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                // Free intermediate closure's captures struct after the call.
                // Scope-variable closures (alloca = Some) are handled by scope cleanup.
                if alloca.is_none() {
                    self.rc_inc(closure_ptr)?;
                    self.rc_dec_closure_captures(closure_ptr, closure_ty)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, actual_fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            // Handle untyped parameters (fallback to Int) used as function callbacks.
            // Use fat return type to preserve enum/string/struct values through the
            // untyped boundary. The named fat_return_type is distinct from enum types,
            // so bv_to_typed won't confuse packed scalars with enums.
            TypedValue::Int(iv) => {
                let total_args = args.len() + trailing.as_ref().map_or(0, |_| 1);
                let param_tys: Vec<BasicMetadataTypeEnum<'ctx>> =
                    (0..total_args).map(|_| self.i64_ty().into()).collect();
                let ret_ty = self.fat_return_type;
                let fn_type = ret_ty.fn_type(&param_tys, false);
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_ptr = self
                    .builder
                    .build_int_to_ptr(iv, ptr_type, "fn_ptr_cast")
                    .map_err(llvm_err)?;
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                for a in args {
                    let av = self.compile_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    ca.push(bv.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    ca.push(bv.into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect_untyped")
                    .map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, Some(BasicTypeEnum::StructType(ret_ty))),
                    None => Ok(TypedValue::Unit),
                }
            }
            _ => Err("Call target is not a function".to_string()),
        }
    }

    /// HIR-native indirect call (function values, closures, untyped callbacks).
    pub(super) fn compile_indirect_call_hir(
        &mut self,
        target: TypedValue<'ctx>,
        args: &[action_frontend::hir::HirExpr],
        trailing: Option<&Box<action_frontend::hir::HirExpr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        match target {
            TypedValue::Fn(fn_ptr, fn_type) => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                for a in args {
                    let av = self.compile_hir_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    ca.push(bv.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load_hir(lam)?;
                    ca.push(bv.into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect")
                    .map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            TypedValue::Closure {
                fn_ptr,
                actual_fn_type,
                closure_ptr,
                closure_ty,
                alloca,
            } => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                ca.push(closure_ptr.into());
                for a in args {
                    let av = self.compile_hir_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    ca.push(bv.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load_hir(lam)?;
                    ca.push(bv.into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(actual_fn_type, fn_ptr, &ca, "indirect_closure")
                    .map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                if alloca.is_none() {
                    self.rc_inc(closure_ptr)?;
                    self.rc_dec_closure_captures(closure_ptr, closure_ty)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, actual_fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            TypedValue::Int(iv) => {
                let total_args = args.len() + trailing.map_or(0, |_| 1);
                let param_tys: Vec<BasicMetadataTypeEnum<'ctx>> =
                    (0..total_args).map(|_| self.i64_ty().into()).collect();
                let ret_ty = self.fat_return_type;
                let fn_type = ret_ty.fn_type(&param_tys, false);
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_ptr = self
                    .builder
                    .build_int_to_ptr(iv, ptr_type, "fn_ptr_cast")
                    .map_err(llvm_err)?;
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                for a in args {
                    let av = self.compile_hir_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    ca.push(bv.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load_hir(lam)?;
                    ca.push(bv.into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect_untyped")
                    .map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, Some(BasicTypeEnum::StructType(ret_ty))),
                    None => Ok(TypedValue::Unit),
                }
            }
            _ => Err("Call target is not a function".to_string()),
        }
    }

    /// Read-only List UFCS methods using the already-compiled receiver value.
    /// Returns `None` when `method` is not handled here.
    pub(super) fn compile_list_readonly_ufcs(
        &mut self,
        lp: PointerValue<'ctx>,
        recv_val: &TypedValue<'ctx>,
        method: &str,
        args: &[super::call_arg::CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let Some(def) = action_frontend::builtin::lookup_ufcs(UfcsReceiverKind::List, method)
        else {
            return Ok(None);
        };
        if !def.readonly {
            return Ok(None);
        }
        let lv = self.load_list(lp)?;
        let zero = self.i64_ty().const_int(0, false);
        match method {
            "len" => {
                let len = self.list_len_val(lv)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::Int(len)))
            }
            "isEmpty" => {
                let len = self.list_len_val(lv)?;
                let is_empty = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                    .map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::Bool(is_empty)))
            }
            "head" => {
                if !args.is_empty() {
                    return Err("list.head expects 0 arguments".to_string());
                }
                let len = self.list_len_val(lv)?;
                let empty = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                    .map_err(llvm_err)?;
                let nullable_ty = self.get_nullable_type(self.i64_ty().into(), "Nullable<Int>");
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no fn")?;
                let some_bb = self
                    .context
                    .append_basic_block(current_fn, "ufcs_head_some");
                let none_bb = self
                    .context
                    .append_basic_block(current_fn, "ufcs_head_none");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "ufcs_head_merge");
                let _ = self
                    .builder
                    .build_conditional_branch(empty, none_bb, some_bb);
                self.builder.position_at_end(some_bb);
                let elem = self.call_rt("action_list_get", &[lv.into(), zero.into()])?;
                let elem_tag = elem
                    .try_as_basic_value()
                    .basic()
                    .ok_or("get failed")?
                    .into_struct_value();
                let elem_tag = self
                    .builder
                    .build_extract_value(elem_tag, 0, "elem_tag")
                    .map_err(llvm_err)?;
                let some_struct = {
                    let undef = nullable_ty.get_undef();
                    let r1 = self
                        .builder
                        .build_insert_value(
                            undef,
                            self.null_flag_ty().const_int(0, false),
                            0,
                            "s_flag",
                        )
                        .map_err(llvm_err)?;
                    self.builder
                        .build_insert_value(r1, elem_tag, 1, "s_val")
                        .map_err(llvm_err)?
                };
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(none_bb);
                let none_struct = {
                    let undef = nullable_ty.get_undef();
                    self.builder
                        .build_insert_value(
                            undef,
                            self.null_flag_ty().const_int(1, false),
                            0,
                            "n_flag",
                        )
                        .map_err(llvm_err)?
                };
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(nullable_ty, "ufcs_head_result")
                    .map_err(llvm_err)?;
                phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                let alloca = self
                    .builder
                    .build_alloca(nullable_ty, "ufcs_head")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(alloca, phi.as_basic_value())
                    .map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::Nullable(alloca, nullable_ty.into())))
            }
            "tail" => {
                if !args.is_empty() {
                    return Err("list.tail expects 0 arguments".to_string());
                }
                let len = self.list_len_val(lv)?;
                let is_empty = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                    .map_err(llvm_err)?;
                let cc = self.call_rt("action_list_tail", &[lv.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("tail failed")?
                    .into_struct_value();
                self.rc_free_intermediate(recv_val)?;
                self.build_nullable_list(result, is_empty).map(Some)
            }
            "get" => {
                if args.len() != 1 {
                    return Err("list.get expects 1 argument".to_string());
                }
                let idx_val = self.compile_call_arg(args[0])?;
                let iv = match idx_val {
                    TypedValue::Int(v) => v,
                    _ => return Err("list.get: index must be Int".to_string()),
                };
                let len = self.list_len_val(lv)?;
                let neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, iv, zero, "neg")
                    .map_err(llvm_err)?;
                let ge_len = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, iv, len, "ge_len")
                    .map_err(llvm_err)?;
                let oob = self
                    .builder
                    .build_or(neg, ge_len, "oob")
                    .map_err(llvm_err)?;
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no fn")?;
                let some_bb = self.context.append_basic_block(current_fn, "ufcs_get_some");
                let none_bb = self.context.append_basic_block(current_fn, "ufcs_get_none");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "ufcs_get_merge");
                let _ = self.builder.build_conditional_branch(oob, none_bb, some_bb);
                self.builder.position_at_end(some_bb);
                let elem = self.call_rt("action_list_get", &[lv.into(), iv.into()])?;
                let elem_bv = elem.try_as_basic_value().basic().ok_or("get failed")?;
                let nullable_ty = self.get_nullable_type(self.string_type.into(), "Nullable<Str>");
                let some_struct = {
                    let undef = nullable_ty.get_undef();
                    let r1 = self
                        .builder
                        .build_insert_value(
                            undef,
                            self.null_flag_ty().const_int(0, false),
                            0,
                            "s_flag",
                        )
                        .map_err(llvm_err)?;
                    self.builder
                        .build_insert_value(r1, elem_bv, 1, "s_val")
                        .map_err(llvm_err)?
                };
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(none_bb);
                let none_struct = {
                    let undef = nullable_ty.get_undef();
                    self.builder
                        .build_insert_value(
                            undef,
                            self.null_flag_ty().const_int(1, false),
                            0,
                            "n_flag",
                        )
                        .map_err(llvm_err)?
                };
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(nullable_ty, "ufcs_get_result")
                    .map_err(llvm_err)?;
                phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                let alloca = self
                    .builder
                    .build_alloca(nullable_ty, "ufcs_get")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(alloca, phi.as_basic_value())
                    .map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::Nullable(alloca, nullable_ty.into())))
            }
            "contains" => {
                if args.len() != 1 {
                    return Err("list.contains expects 1 argument".to_string());
                }
                let elem_val = self.compile_call_arg(args[0])?;
                let fat = self.to_fat_struct(&elem_val)?;
                let cc = self.call_rt("action_list_contains", &[lv.into(), fat.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("contains failed")?
                    .into_int_value();
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::Bool(result)))
            }
            "indexOf" => {
                if args.len() != 1 {
                    return Err("list.indexOf expects 1 argument".to_string());
                }
                let elem_val = self.compile_call_arg(args[0])?;
                let fat = self.to_fat_struct(&elem_val)?;
                let cc = self.call_rt("action_list_index_of", &[lv.into(), fat.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("indexOf failed")?
                    .into_int_value();
                let found = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, result, zero, "found")
                    .map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                self.build_nullable_int(result, found).map(Some)
            }
            "last" => {
                if !args.is_empty() {
                    return Err("list.last expects 0 arguments".to_string());
                }
                let len = self.list_len_val(lv)?;
                let empty = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                    .map_err(llvm_err)?;
                let last_idx = self
                    .builder
                    .build_int_sub(len, self.i64_ty().const_int(1, false), "last_idx")
                    .map_err(llvm_err)?;
                let nullable_ty = self.get_nullable_type(self.i64_ty().into(), "Nullable<Int>");
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no fn")?;
                let some_bb = self
                    .context
                    .append_basic_block(current_fn, "ufcs_last_some");
                let none_bb = self
                    .context
                    .append_basic_block(current_fn, "ufcs_last_none");
                let merge_bb = self
                    .context
                    .append_basic_block(current_fn, "ufcs_last_merge");
                let _ = self
                    .builder
                    .build_conditional_branch(empty, none_bb, some_bb);
                self.builder.position_at_end(some_bb);
                let elem = self.call_rt("action_list_get", &[lv.into(), last_idx.into()])?;
                let elem_tag = elem
                    .try_as_basic_value()
                    .basic()
                    .ok_or("get failed")?
                    .into_struct_value();
                let elem_tag = self
                    .builder
                    .build_extract_value(elem_tag, 0, "elem_tag")
                    .map_err(llvm_err)?;
                let some_struct = {
                    let undef = nullable_ty.get_undef();
                    let r1 = self
                        .builder
                        .build_insert_value(
                            undef,
                            self.null_flag_ty().const_int(0, false),
                            0,
                            "s_flag",
                        )
                        .map_err(llvm_err)?;
                    self.builder
                        .build_insert_value(r1, elem_tag, 1, "s_val")
                        .map_err(llvm_err)?
                };
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(none_bb);
                let none_struct = {
                    let undef = nullable_ty.get_undef();
                    self.builder
                        .build_insert_value(
                            undef,
                            self.null_flag_ty().const_int(1, false),
                            0,
                            "n_flag",
                        )
                        .map_err(llvm_err)?
                };
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(nullable_ty, "ufcs_last_result")
                    .map_err(llvm_err)?;
                phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                let alloca = self
                    .builder
                    .build_alloca(nullable_ty, "ufcs_last")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(alloca, phi.as_basic_value())
                    .map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::Nullable(alloca, nullable_ty.into())))
            }
            "reverse" => {
                if !args.is_empty() {
                    return Err("list.reverse expects 0 arguments".to_string());
                }
                let cc = self.call_rt("action_list_reverse", &[lv.into()])?;
                let result = cc.try_as_basic_value().basic().ok_or("reverse failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "ufcs_rev")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, result).map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::List(alloca)))
            }
            "sum" => {
                if !args.is_empty() {
                    return Err("list.sum expects 0 arguments".to_string());
                }
                let result = self.list_sum_from_loaded(lv)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::Int(result)))
            }
            "withIndex" => {
                if !args.is_empty() {
                    return Err("list.withIndex expects 0 arguments".to_string());
                }
                let cc = self.call_rt("action_list_with_index", &[lv.into()])?;
                let result = cc.try_as_basic_value().basic().ok_or("withIndex failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "ufcs_wi")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, result).map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::List(alloca)))
            }
            _ => Ok(None),
        }
    }

    /// Convert a TypedValue to a BasicValueEnum suitable for passing as a
    /// function call argument, without re-compiling the expression.
    pub(super) fn typed_value_to_bv(&self, av: &TypedValue<'ctx>) -> BasicValueEnum<'ctx> {
        av.to_bv().unwrap_or_else(|| match av {
            TypedValue::Str(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.string_type, *ptr, "arg_str")
                    .unwrap();
                ld.into()
            }
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.list_type, *ptr, "arg_list")
                    .unwrap();
                ld.into()
            }
            TypedValue::LazyList(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "arg_ll")
                    .unwrap();
                ld.into()
            }
            TypedValue::Task(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.task_type, *ptr, "arg_task")
                    .unwrap();
                ld.into()
            }
            TypedValue::Stream(ptr) => {
                let lf = self
                    .builder
                    .build_struct_gep(self.stream_type, *ptr, 3, "arg_slf")
                    .unwrap();
                let ld = self
                    .builder
                    .build_load(self.list_type, lf, "arg_sl")
                    .unwrap();
                ld.into()
            }
            TypedValue::Struct(ptr, st) => {
                let ld = self.builder.build_load(*st, *ptr, "arg_struct").unwrap();
                ld.into()
            }
            TypedValue::Enum(ptr, et, ..) => {
                let ld = self.builder.build_load(*et, *ptr, "arg_enum").unwrap();
                ld.into()
            }
            TypedValue::Nullable(ptr, ty) => {
                self.builder.build_load(*ty, *ptr, "arg_nullable").unwrap()
            }
            TypedValue::CString(p) | TypedValue::Ptr(p) | TypedValue::FileHandle(p) => (*p).into(),
            _ => self.i64_ty().const_int(0, false).into(),
        })
    }
}
