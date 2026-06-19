//! HIR-native codegen: compile from typed IR without reading `CheckedProgram::program`.

use super::{llvm_err, CodeGen, TypedValue};
use action_frontend::ast::*;
use action_frontend::hir::*;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::IntPredicate;
use std::collections::HashMap;

impl<'ctx> CodeGen<'ctx> {
    /// Compile a type-checked HIR module (production entry point).
    pub fn compile_hir(&mut self, hir: &HirModule) -> Result<(), String> {
        self.compile_inner_hir(hir)
    }

    fn compile_inner_hir(&mut self, hir: &HirModule) -> Result<(), String> {
        self.define_runtime()?;
        self.detach_builder()?;

        // Pass 0: Register type definitions and create LLVM types
        for stmt in &hir.stmts {
            self.registry.register(&stmt.as_stmt())?;
            match stmt {
                HirStmt::TypeAlias {
                    name, definition, ..
                } => {
                    if let Type::Struct(fields) = definition {
                        let field_tys: Vec<BasicTypeEnum> = fields
                            .iter()
                            .map(|(_, ty)| self.ast_type_to_basic_type(ty))
                            .collect();
                        let struct_ty = self.context.struct_type(&field_tys, false);
                        self.named_structs.insert(name.clone(), struct_ty);
                    }
                }
                HirStmt::Enum { name, .. } => {
                    let i64 = self.i64_ty();
                    let ptr = self.ptr_ty();
                    let enum_ty = self.context.struct_type(&[i64.into(), ptr.into()], false);
                    self.enum_types.insert(name.clone(), enum_ty);
                }
                _ => {}
            }
        }

        // Detect overloaded function names
        let mut name_counts: HashMap<String, usize> = HashMap::new();
        for stmt in &hir.stmts {
            if let HirStmt::Fun { name, params, .. } = stmt {
                if params.iter().all(|p| p.ty.is_some()) {
                    *name_counts.entry(name.clone()).or_insert(0) += 1;
                }
            }
        }
        let overloaded_names: std::collections::HashSet<String> = name_counts
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .map(|(name, _)| name)
            .collect();

        // Pass 1: Declare all user-defined functions
        for stmt in &hir.stmts {
            if let HirStmt::Fun {
                name,
                params,
                return_type,
                body,
                type_params,
                ..
            } = stmt
            {
                if !type_params.is_empty() {
                    self.generic_fun_defs.insert(name.clone(), stmt.as_stmt());
                    continue;
                }
                let param_types: Vec<Type> = params
                    .iter()
                    .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                    .collect();
                let all_typed = params.iter().all(|p| p.ty.is_some());
                let mangled = if all_typed && overloaded_names.contains(name.as_str()) {
                    Self::mangle_name(name, &param_types)
                } else {
                    name.clone()
                };

                if all_typed && overloaded_names.contains(name.as_str()) {
                    self.overloaded_functions
                        .entry(name.clone())
                        .or_insert_with(Vec::new)
                        .push((param_types.clone(), mangled.clone()));
                }

                let param_llvm_tys: Vec<BasicMetadataTypeEnum> = params
                    .iter()
                    .map(|p| self.ast_type_to_llvm(p.ty.as_ref()))
                    .collect();
                let ret_type = if name == "main" {
                    Some(Type::Named("Int".into()))
                } else {
                    return_type.clone().or_else(|| {
                        if all_typed {
                            Some(body.ty.clone())
                        } else {
                            None
                        }
                    })
                };
                let fn_type = self.build_fn_type(ret_type.as_ref(), &mangled, &param_llvm_tys);
                self.module.add_function(&mangled, fn_type, None);
                if name != "main" {
                    if let Some(rt) = ret_type {
                        self.fun_return_types.insert(mangled, rt);
                    }
                }
            }
            if let HirStmt::Module {
                name: mod_name,
                body,
                ..
            } = stmt
            {
                let prefix = format!("{}_", mod_name);
                for inner_stmt in body {
                    if let HirStmt::Fun {
                        name: fn_name,
                        params,
                        return_type,
                        body: fn_body,
                        type_params,
                        ..
                    } = inner_stmt
                    {
                        if !type_params.is_empty() {
                            continue;
                        }
                        let mangled = format!("{}{}", prefix, fn_name);
                        let param_llvm_tys: Vec<BasicMetadataTypeEnum> = params
                            .iter()
                            .map(|p| self.ast_type_to_llvm(p.ty.as_ref()))
                            .collect();
                        let ret_type = return_type.clone().or_else(|| {
                            if params.iter().all(|p| p.ty.is_some()) {
                                Some(fn_body.ty.clone())
                            } else {
                                None
                            }
                        });
                        let fn_type =
                            self.build_fn_type(ret_type.as_ref(), &mangled, &param_llvm_tys);
                        self.module.add_function(&mangled, fn_type, None);
                        if let Some(rt) = ret_type {
                            self.fun_return_types.insert(mangled, rt);
                        }
                    }
                }
            }
            if let HirStmt::Extension {
                type_name, methods, ..
            } = stmt
            {
                for m in methods {
                    if let HirStmt::Fun {
                        name,
                        params,
                        return_type,
                        body,
                        ..
                    } = m
                    {
                        let fn_name = format!("{}_{}", type_name, name);
                        self.extension_methods
                            .insert(format!("{}.{}", type_name, name), fn_name.clone());
                        let param_llvm_tys: Vec<BasicMetadataTypeEnum> = params
                            .iter()
                            .map(|p| self.ast_type_to_llvm(p.ty.as_ref()))
                            .collect();
                        let ret_type = return_type.clone().or_else(|| {
                            if params.iter().all(|p| p.ty.is_some()) {
                                Some(body.ty.clone())
                            } else {
                                None
                            }
                        });
                        let fn_type =
                            self.build_fn_type(ret_type.as_ref(), &fn_name, &param_llvm_tys);
                        self.module.add_function(&fn_name, fn_type, None);
                        if let Some(rt) = ret_type {
                            self.fun_return_types.insert(fn_name, rt);
                        }
                    }
                }
            }
        }

        // Pass 2: Compile function bodies and top-level statements
        let mut has_main = false;
        for stmt in &hir.stmts {
            if let HirStmt::Fun { name, .. } = stmt {
                if name == "main" {
                    has_main = true;
                }
            }
        }

        if !has_main {
            let main_fn = self.i64_ty().fn_type(&[], false);
            let main_func = self.module.add_function("main", main_fn, None);
            let entry = self.context.append_basic_block(main_func, "entry");
            self.builder.position_at_end(entry);

            for stmt in &hir.stmts {
                match stmt {
                    HirStmt::Fun { type_params, .. } if !type_params.is_empty() => {}
                    HirStmt::Fun { .. } | HirStmt::Extension { .. } => {
                        self.compile_hir_stmt(stmt)?;
                    }
                    HirStmt::TypeAlias { .. } | HirStmt::Enum { .. } => {}
                    _ => {
                        self.compile_hir_stmt(stmt)?;
                    }
                }
            }
            if let Some(fflush_fn) = self.module.get_function("fflush") {
                let _ =
                    self.builder
                        .build_call(fflush_fn, &[self.ptr_ty().const_null().into()], "");
            }
            let _ = self
                .builder
                .build_return(Some(&self.i64_ty().const_int(0, false)));
        } else {
            for stmt in &hir.stmts {
                match stmt {
                    HirStmt::Fun { type_params, .. } if !type_params.is_empty() => {}
                    _ => {
                        self.compile_hir_stmt(stmt)?;
                    }
                }
            }
        }

        self.finalize_codegen_anchor()?;
        Ok(())
    }

    pub(super) fn compile_hir_stmt(&mut self, stmt: &HirStmt) -> Result<(), String> {
        match stmt {
            HirStmt::Let {
                lazy_init: false, ..
            } => {
                self.compile_stmt(&stmt.as_stmt())?;
            }
            HirStmt::Let {
                name,
                type_ann,
                value,
                mutable,
                lazy_init: true,
                ..
            } => {
                let hir_ty = &value.ty;
                let (ty, kind, ast_type) = if let Some(ann) = type_ann {
                    (
                        self.ast_type_to_basic_type(ann),
                        self.param_val_kind(Some(ann)),
                        Some(ann.clone()),
                    )
                } else {
                    (
                        self.ast_type_to_basic_type(hir_ty),
                        self.param_val_kind(Some(hir_ty)),
                        Some(hir_ty.clone()),
                    )
                };
                let alloca = self.builder.build_alloca(ty, name).map_err(llvm_err)?;
                let flag = self
                    .builder
                    .build_alloca(self.bool_ty(), &format!("{}_lazy_flag", name))
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(flag, self.bool_ty().const_int(0, false))
                    .map_err(llvm_err)?;
                self.scope.set_lazy(
                    name.clone(),
                    alloca,
                    ty,
                    kind,
                    flag,
                    value.clone(),
                    ast_type,
                );
                let _ = mutable;
            }
            HirStmt::Expr { expr, .. } => {
                let result = self.compile_hir_expr(expr)?;
                self.rc_discard_value(&result)?;
            }
            HirStmt::Return { value: Some(v), .. } => {
                let val = self.compile_hir_expr(v)?;
                let _ = self.compile_return_value(val);
            }
            HirStmt::Return { value: None, .. } => {
                let _ = self.compile_return_void();
            }
            HirStmt::Destructure {
                mutable,
                names,
                renames,
                rest,
                is_list,
                is_struct,
                value,
                ..
            } => {
                self.compile_destructure_hir(
                    *mutable, names, renames, rest, *is_list, *is_struct, value,
                )?;
            }
            HirStmt::Fun {
                name,
                params,
                return_type,
                body,
                ..
            } => {
                let all_typed = params.iter().all(|p| p.ty.is_some());
                let fn_name = if all_typed && self.overloaded_functions.contains_key(name.as_str())
                {
                    let param_types: Vec<Type> = params
                        .iter()
                        .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                        .collect();
                    Self::mangle_name(name, &param_types)
                } else {
                    name.clone()
                };
                self.compile_fun_def_hir(&fn_name, name, params, return_type.as_ref(), body)?;
            }
            HirStmt::Extension {
                type_name, methods, ..
            } => {
                for m in methods {
                    if let HirStmt::Fun {
                        name,
                        params,
                        return_type,
                        body,
                        ..
                    } = m
                    {
                        let fn_name = format!("{}_{}", type_name, name);
                        self.compile_fun_def_hir(
                            &fn_name,
                            name,
                            params,
                            return_type.as_ref(),
                            body,
                        )?;
                    }
                }
            }
            HirStmt::Module { name, body, .. } => {
                let prefix = format!("{}_", name);
                let mut saved_scope = super::Scope::new();
                std::mem::swap(&mut self.scope, &mut saved_scope);
                self.scope = super::Scope::with_parent(saved_scope);
                for inner in body {
                    let renamed = self.rename_module_hir_stmt(inner, &prefix);
                    self.compile_hir_stmt(&renamed)?;
                }
                let mut parent = super::Scope::new();
                std::mem::swap(&mut self.scope, &mut parent);
                if let Some(p) = parent.parent {
                    self.scope = *p;
                }
            }
            HirStmt::Break { .. } => {
                let _ = self.compile_hir_break()?;
            }
            HirStmt::Continue { .. } => {
                let _ = self.compile_hir_continue()?;
            }
            HirStmt::Const { name, value, .. } => {
                self.compile_hir_const(name, value)?;
            }
            HirStmt::TypeAlias { .. } | HirStmt::Enum { .. } | HirStmt::Import { .. } => {}
            HirStmt::Export { stmt, .. } => self.compile_hir_stmt(stmt)?,
            HirStmt::External { .. } | HirStmt::ExternalType { .. } => {
                self.compile_stmt(&stmt.as_stmt())?;
            }
        }
        Ok(())
    }

    pub(super) fn compile_hir_expr(&mut self, expr: &HirExpr) -> Result<TypedValue<'ctx>, String> {
        match &expr.kind {
            HirExprKind::Literal(lit) => self.compile_literal(lit),
            HirExprKind::Ident(name) => self.compile_ident(name),
            HirExprKind::Null => self.compile_null(),
            HirExprKind::Continue => self.compile_hir_continue(),
            HirExprKind::Break => self.compile_hir_break(),
            HirExprKind::FunctionRef(name) => self.compile_function_ref(name),
            HirExprKind::Block(stmts) => self.compile_hir_block(stmts, &expr.ty),
            HirExprKind::Binary(lhs, op, rhs) => self.compile_hir_binary(lhs, *op, rhs, &expr.ty),
            HirExprKind::Unary(op, inner) => self.compile_hir_unary(*op, inner),
            HirExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => self.compile_hir_call(func, args, trailing_lambda.as_ref()),
            HirExprKind::When(w) => self.compile_hir_when(w),
            HirExprKind::For(f) => self.compile_hir_for(f),
            HirExprKind::Assign { target, value } => self.compile_hir_assign(target, value),
            HirExprKind::StringInterpolate(parts) => self.compile_hir_string_interp(parts),
            HirExprKind::FieldAccess(obj, field) => self.compile_hir_field_access(obj, field),
            HirExprKind::StructLiteral(fields) => self.compile_hir_struct_lit(fields),
            HirExprKind::MapLiteral(entries) => self.compile_hir_map_lit(entries),
            HirExprKind::SetLiteral(elements) => self.compile_hir_set_lit(elements),
            HirExprKind::Lambda { params, body, .. } => self.compile_hir_lambda(params, body),
            HirExprKind::Index(obj, idx) => self.compile_hir_index(obj, idx),
            HirExprKind::Range(start, end) => self.compile_hir_range(start, end),
            HirExprKind::Tuple(items) => self.compile_hir_tuple(items),
            HirExprKind::OrBlock { nullable, fallback } => {
                self.compile_hir_or_block(nullable, fallback)
            }
            HirExprKind::Copy(inner) => {
                let val = self.compile_hir_expr(inner)?;
                self.compile_copy_value(val)
            }
            HirExprKind::Unsafe(inner) => self.compile_hir_expr(inner),
        }
    }

    fn compile_hir_continue(&mut self) -> Result<TypedValue<'ctx>, String> {
        if let Some(target) = self.continue_target {
            self.builder
                .build_unconditional_branch(target)
                .map_err(llvm_err)?;
            Ok(TypedValue::Unit)
        } else {
            Err("continue outside loop".to_string())
        }
    }

    fn compile_hir_break(&mut self) -> Result<TypedValue<'ctx>, String> {
        if let Some(target) = self.break_target {
            self.builder
                .build_unconditional_branch(target)
                .map_err(llvm_err)?;
            Ok(TypedValue::Unit)
        } else {
            Err("break outside loop".to_string())
        }
    }

    fn compile_hir_binary(
        &mut self,
        lhs: &HirExpr,
        op: BinaryOp,
        rhs: &HirExpr,
        result_ty: &Type,
    ) -> Result<TypedValue<'ctx>, String> {
        if matches!(
            op,
            BinaryOp::And | BinaryOp::Or | BinaryOp::Is | BinaryOp::In
        ) {
            return match op {
                BinaryOp::And => self.compile_and_hir(lhs, rhs),
                BinaryOp::Or => self.compile_or_hir(lhs, rhs),
                BinaryOp::Is => self.bin_is_hir(lhs, rhs),
                BinaryOp::In => self.bin_in_hir(lhs, rhs),
                _ => unreachable!(),
            };
        }
        let left = self.compile_hir_expr(lhs)?;
        let right = self.compile_hir_expr(rhs)?;
        self.compile_binary_values(op, &left, &right, result_ty)
    }

    fn compile_hir_unary(
        &mut self,
        op: UnaryOp,
        inner: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_hir_expr(inner)?;
        self.compile_unary_values(op, val)
    }

    fn compile_hir_assign(
        &mut self,
        target: &HirExpr,
        value: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_assign_hir(target, value)
    }

    fn compile_hir_field_access(
        &mut self,
        obj: &HirExpr,
        field: &str,
    ) -> Result<TypedValue<'ctx>, String> {
        let obj_val = self.compile_hir_expr(obj)?;
        self.compile_field_access_value(obj_val, field)
    }

    fn compile_hir_lambda(
        &mut self,
        params: &[String],
        body: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_lambda_hir(params, body)
    }

    fn compile_hir_index(
        &mut self,
        obj: &HirExpr,
        idx: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::ast::Literal;
        use action_frontend::hir::HirExprKind;

        let obj_val = self.compile_hir_expr(obj)?;
        if let TypedValue::Nullable(nullable_ptr, inner_bt) = obj_val {
            return self.compile_nullable_index_values(
                nullable_ptr,
                inner_bt,
                super::call_arg::CallArg::hir(idx),
            );
        }
        match obj_val {
            TypedValue::Map(map_ptr) => {
                let key_val = self.compile_hir_expr(idx)?;
                self.compile_map_index_key(map_ptr, key_val)
            }
            TypedValue::Set(set_ptr) => {
                let elem_val = self.compile_hir_expr(idx)?;
                self.compile_set_index_key(set_ptr, elem_val)
            }
            TypedValue::Struct(ptr, struct_ty) => {
                let index = match &idx.kind {
                    HirExprKind::Literal(Literal::Int(n)) => *n as u32,
                    _ => return Err("Tuple/struct index must be an integer literal".to_string()),
                };
                let bt: inkwell::types::BasicTypeEnum = struct_ty.into();
                let loaded = self
                    .builder
                    .build_load(bt, ptr, "tuple_ld")
                    .map_err(super::llvm_err)?;
                let struct_val = loaded.into_struct_value();
                let field_val = self
                    .builder
                    .build_extract_value(struct_val, index, "tuple_idx")
                    .map_err(super::llvm_err)?;
                self.bv_to_typed(field_val)
            }
            other => {
                let idx_val = self.compile_hir_expr(idx)?;
                self.compile_index_values(other, idx_val)
            }
        }
    }

    fn compile_hir_range(
        &mut self,
        start: &HirExpr,
        end: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let start_val = self.compile_hir_expr(start)?;
        let end_val = self.compile_hir_expr(end)?;
        self.compile_range_values(start_val, end_val)
    }

    fn compile_hir_struct_lit(
        &mut self,
        fields: &[(String, HirExpr)],
    ) -> Result<TypedValue<'ctx>, String> {
        let names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
        let mut vals = Vec::with_capacity(fields.len());
        for (_, e) in fields {
            vals.push(self.compile_hir_expr(e)?);
        }
        self.compile_struct_lit_values(&names, vals)
    }

    fn compile_hir_map_lit(
        &mut self,
        entries: &[(HirExpr, HirExpr)],
    ) -> Result<TypedValue<'ctx>, String> {
        let mut keys = Vec::with_capacity(entries.len());
        let mut vals = Vec::with_capacity(entries.len());
        for (k, v) in entries {
            keys.push(self.compile_hir_expr(k)?);
            vals.push(self.compile_hir_expr(v)?);
        }
        self.compile_map_lit_values(&keys, &vals)
    }

    fn compile_hir_set_lit(&mut self, elements: &[HirExpr]) -> Result<TypedValue<'ctx>, String> {
        let mut vals = Vec::with_capacity(elements.len());
        for e in elements {
            vals.push(self.compile_hir_expr(e)?);
        }
        self.compile_set_lit_values(&vals)
    }

    fn compile_hir_tuple(
        &mut self,
        items: &[(Option<String>, HirExpr)],
    ) -> Result<TypedValue<'ctx>, String> {
        let mut compiled = Vec::with_capacity(items.len());
        for (n, e) in items {
            compiled.push((n.clone(), self.compile_hir_expr(e)?));
        }
        self.compile_tuple_values(&compiled)
    }

    fn compile_hir_or_block(
        &mut self,
        nullable: &HirExpr,
        fallback: &HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_or_block_hir(nullable, fallback)
    }

    fn compile_hir_string_interp(
        &mut self,
        parts: &[HirStringPart],
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_string_interp_hir(parts)
    }

    fn compile_hir_block(
        &mut self,
        stmts: &[HirStmt],
        _result_ty: &Type,
    ) -> Result<TypedValue<'ctx>, String> {
        let mut saved = super::Scope::new();
        std::mem::swap(&mut self.scope, &mut saved);
        self.scope = super::Scope::with_parent(saved);

        self.block_did_rc_inc = false;

        let mut last = TypedValue::Unit;
        for s in stmts {
            match s {
                HirStmt::Expr { expr, .. } => {
                    self.rc_discard_value(&last)?;
                    last = self.compile_hir_expr(expr)?;
                }
                _ => self.compile_hir_stmt(s)?,
            }
        }

        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("compile_hir_block: builder has no insert block")?;
        if current_block.get_terminator().is_none() {
            if self.is_scope_variable(&last) {
                self.rc_inc_typed_value(&last)?;
                self.block_did_rc_inc = true;
            } else {
                self.block_did_rc_inc = false;
            }
            self.emit_scope_cleanup()?;
        } else {
            self.block_did_rc_inc = false;
        }

        let mut parent = super::Scope::new();
        std::mem::swap(&mut self.scope, &mut parent);
        if let Some(p) = parent.parent {
            self.scope = *p;
        }
        Ok(last)
    }

    fn compile_hir_call(
        &mut self,
        func: &HirExpr,
        args: &[HirExpr],
        trailing_lambda: Option<&Box<HirExpr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_call_hir(func, args, trailing_lambda)
    }

    fn compile_destructure_hir(
        &mut self,
        mutable: bool,
        names: &[String],
        renames: &[(String, String)],
        rest: &Option<String>,
        is_list: bool,
        is_struct: bool,
        value: &HirExpr,
    ) -> Result<(), String> {
        let _ = (renames,);
        if !is_list {
            return self.compile_stmt(
                &HirStmt::Destructure {
                    mutable,
                    names: names.to_vec(),
                    renames: renames.to_vec(),
                    rest: rest.clone(),
                    is_list,
                    is_struct,
                    value: value.clone(),
                    span: value.span,
                }
                .as_stmt(),
            );
        }
        let val = self.compile_hir_expr(value)?;
        if is_list {
            let list_ptr = match val {
                TypedValue::List(ptr) => ptr,
                _ => return Err("List destructuring requires a list value".to_string()),
            };
            let list_val = self.load_list(list_ptr)?;
            let data = self
                .builder
                .build_extract_value(list_val, 0, "data")
                .map_err(llvm_err)?
                .into_pointer_value();
            let len = self
                .builder
                .build_extract_value(list_val, 1, "len")
                .map_err(llvm_err)?
                .into_int_value();
            let data_str = self
                .builder
                .build_pointer_cast(data, self.ptr_ty(), "data_str")
                .map_err(llvm_err)?;
            for (i, name) in names.iter().enumerate() {
                let idx = self.i64_ty().const_int(i as u64, false);
                let elem_ptr = unsafe {
                    self.builder
                        .build_gep(self.string_type, data_str, &[idx], "delem_ptr")
                }
                .map_err(llvm_err)?;
                let loaded = self
                    .builder
                    .build_load(self.string_type, elem_ptr, "delem")
                    .map_err(llvm_err)?;
                let ss = loaded.into_struct_value();
                let tag = self
                    .builder
                    .build_extract_value(ss, 0, "tag")
                    .map_err(llvm_err)?
                    .into_int_value();
                let tag_ty = tag.get_type();
                let alloca = self.builder.build_alloca(tag_ty, name).map_err(llvm_err)?;
                self.builder.build_store(alloca, tag).map_err(llvm_err)?;
                if mutable {
                    self.scope.set_mutable(
                        name.clone(),
                        alloca,
                        tag_ty.into(),
                        super::ValKind::Int,
                        None,
                    );
                } else {
                    self.scope
                        .set(name.clone(), alloca, tag_ty.into(), super::ValKind::Int);
                }
            }
            if let Some(rest_name) = rest {
                let start_idx = names.len() as u64;
                let cap = self.i64_ty().const_int(4, false);
                let new_list_cc = self.call_rt("action_list_create", &[cap.into()])?;
                let new_list_bv = new_list_cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("rest list create fail")?;
                let rest_alloca = self
                    .builder
                    .build_alloca(self.list_type, rest_name)
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(rest_alloca, new_list_bv)
                    .map_err(llvm_err)?;
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let i64 = self.i64_ty();
                let i_a = self.builder.build_alloca(i64, "ri").map_err(llvm_err)?;
                self.builder
                    .build_store(i_a, i64.const_int(start_idx, false))
                    .map_err(llvm_err)?;
                let rest_hdr = self.context.append_basic_block(current_fn, "rest_hdr");
                let rest_bdy = self.context.append_basic_block(current_fn, "rest_bdy");
                let rest_ext = self.context.append_basic_block(current_fn, "rest_ext");
                let _ = self.builder.build_unconditional_branch(rest_hdr);
                self.builder.position_at_end(rest_hdr);
                let cur = self
                    .builder
                    .build_load(i64, i_a, "rc")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, cur, len, "rc")
                    .map_err(llvm_err)?;
                let _ = self
                    .builder
                    .build_conditional_branch(cond, rest_bdy, rest_ext);
                self.builder.position_at_end(rest_bdy);
                let elem = self.call_rt("action_list_get", &[list_val.into(), cur.into()])?;
                let elem_bv = elem.try_as_basic_value().basic().ok_or("rest get fail")?;
                let rest_loaded = self.load_list(rest_alloca)?;
                let _ = self.call_rt("action_list_push", &[rest_loaded.into(), elem_bv.into()])?;
                let nxt = self
                    .builder
                    .build_int_add(cur, i64.const_int(1, false), "rn")
                    .map_err(llvm_err)?;
                self.builder.build_store(i_a, nxt).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(rest_hdr);
                self.builder.position_at_end(rest_ext);
                let _ = rest_name;
            }
        } else {
            return Err("Only list destructuring is supported in HIR codegen".to_string());
        }
        Ok(())
    }

    fn compile_hir_const(&mut self, name: &str, value: &HirExpr) -> Result<(), String> {
        match &value.kind {
            HirExprKind::Literal(lit) => {
                let (global_ptr, ty, kind): (
                    inkwell::values::PointerValue<'ctx>,
                    BasicTypeEnum<'ctx>,
                    super::ValKind,
                ) = match lit {
                    Literal::Int(n) => {
                        let g = self.add_module_global(self.i64_ty(), name)?;
                        g.set_initializer(&self.i64_ty().const_int(*n as u64, true));
                        (
                            g.as_pointer_value(),
                            self.i64_ty().into(),
                            super::ValKind::Int,
                        )
                    }
                    Literal::Float(n) => {
                        let g = self.add_module_global(self.f64_ty(), name)?;
                        g.set_initializer(&self.f64_ty().const_float(*n));
                        (
                            g.as_pointer_value(),
                            self.f64_ty().into(),
                            super::ValKind::Float,
                        )
                    }
                    Literal::Bool(b) => {
                        let g = self.add_module_global(self.bool_ty(), name)?;
                        g.set_initializer(&self.bool_ty().const_int(if *b { 1 } else { 0 }, false));
                        (
                            g.as_pointer_value(),
                            self.bool_ty().into(),
                            super::ValKind::Bool,
                        )
                    }
                    Literal::Char(c) => {
                        let g = self.add_module_global(self.i64_ty(), name)?;
                        g.set_initializer(&self.i64_ty().const_int(*c as u64, false));
                        (
                            g.as_pointer_value(),
                            self.i64_ty().into(),
                            super::ValKind::Int,
                        )
                    }
                    Literal::Unit => {
                        let g = self.add_module_global(self.i64_ty(), name)?;
                        g.set_initializer(&self.i64_ty().const_int(0, false));
                        (
                            g.as_pointer_value(),
                            self.i64_ty().into(),
                            super::ValKind::Unit,
                        )
                    }
                    Literal::String(s) => {
                        let content_bytes: Vec<u8> = s.bytes().chain(std::iter::once(0)).collect();
                        let arr_ty = self
                            .context
                            .i8_type()
                            .array_type(content_bytes.len() as u32);
                        let str_data_g =
                            self.add_module_global(arr_ty, &format!("__const_str_data_{}", name))?;
                        let arr_val = self.context.const_string(&content_bytes, false);
                        str_data_g.set_initializer(&arr_val);
                        let len_val = self.i64_ty().const_int(s.len() as u64, false);
                        let i8_ptr_ty = self.context.ptr_type(inkwell::AddressSpace::default());
                        let data_ptr = str_data_g.as_pointer_value();
                        let data_ptr_i8 = data_ptr.const_cast(i8_ptr_ty);
                        let fat_struct = self
                            .context
                            .const_struct(&[len_val.into(), data_ptr_i8.into()], false);
                        let g = self.add_module_global(self.string_type, name)?;
                        g.set_initializer(&fat_struct);
                        (
                            g.as_pointer_value(),
                            self.string_type.into(),
                            super::ValKind::Str,
                        )
                    }
                };
                self.consts.insert(name.to_string(), (global_ptr, ty, kind));
            }
            _ => {
                let val = self.compile_hir_expr(value)?;
                if let Some(bv) = val.to_bv() {
                    let ty = bv.get_type();
                    let g = self.add_module_global(ty, name)?;
                    g.set_initializer(&bv);
                    self.consts
                        .insert(name.to_string(), (g.as_pointer_value(), ty, val.val_kind()));
                } else {
                    return Err(format!("Non-basic-value const '{}' is not supported", name));
                }
            }
        }
        Ok(())
    }

    fn rename_module_hir_stmt(&self, stmt: &HirStmt, prefix: &str) -> HirStmt {
        match stmt {
            HirStmt::Fun {
                name,
                params,
                return_type,
                body,
                type_params,
                is_single_expr,
                is_test,
                span,
            } => HirStmt::Fun {
                name: format!("{}{}", prefix, name),
                params: params.clone(),
                return_type: return_type.clone(),
                body: body.clone(),
                type_params: type_params.clone(),
                is_single_expr: *is_single_expr,
                is_test: *is_test,
                span: *span,
            },
            HirStmt::Const {
                name,
                type_ann,
                value,
                span,
            } => HirStmt::Const {
                name: format!("{}{}", prefix, name),
                type_ann: type_ann.clone(),
                value: value.clone(),
                span: *span,
            },
            other => other.clone(),
        }
    }

    fn compile_binary_values(
        &mut self,
        op: BinaryOp,
        left: &TypedValue<'ctx>,
        right: &TypedValue<'ctx>,
        _result_ty: &Type,
    ) -> Result<TypedValue<'ctx>, String> {
        match op {
            BinaryOp::Add => self.bin_add(left, right),
            BinaryOp::Sub => self.bin_arith(
                left,
                right,
                "sub",
                |b, l, r| b.build_int_sub(l, r, "sub"),
                |b, l, r| b.build_float_sub(l, r, "sub"),
            ),
            BinaryOp::Mul => self.bin_arith(
                left,
                right,
                "mul",
                |b, l, r| b.build_int_mul(l, r, "mul"),
                |b, l, r| b.build_float_mul(l, r, "mul"),
            ),
            BinaryOp::Div => self.bin_div(left, right),
            BinaryOp::Mod => self.bin_mod(left, right),
            BinaryOp::Pow => self.bin_pow(left, right),
            BinaryOp::Eq => self.compare_eq(left, right),
            BinaryOp::Neq => self.compare_neq(left, right),
            BinaryOp::Lt => self.compare(
                inkwell::IntPredicate::SLT,
                inkwell::FloatPredicate::OLT,
                left,
                right,
            ),
            BinaryOp::Gt => self.compare(
                inkwell::IntPredicate::SGT,
                inkwell::FloatPredicate::OGT,
                left,
                right,
            ),
            BinaryOp::Lte => self.compare(
                inkwell::IntPredicate::SLE,
                inkwell::FloatPredicate::OLE,
                left,
                right,
            ),
            BinaryOp::Gte => self.compare(
                inkwell::IntPredicate::SGE,
                inkwell::FloatPredicate::OGE,
                left,
                right,
            ),
            BinaryOp::BitAnd => {
                self.bin_bitwise(left, right, "and", |b, l, r| b.build_and(l, r, "and"))
            }
            BinaryOp::BitOr => {
                self.bin_bitwise(left, right, "or", |b, l, r| b.build_or(l, r, "or"))
            }
            BinaryOp::BitXor => {
                self.bin_bitwise(left, right, "xor", |b, l, r| b.build_xor(l, r, "xor"))
            }
            BinaryOp::Shl => self.bin_bitwise(left, right, "shl", |b, l, r| {
                b.build_left_shift(l, r, "shl")
            }),
            BinaryOp::Shr => self.bin_bitwise(left, right, "shr", |b, l, r| {
                b.build_right_shift(l, r, false, "shr")
            }),
            BinaryOp::Range | BinaryOp::RangeExclusive => {
                let inclusive = matches!(op, BinaryOp::Range);
                let start_int = match left {
                    TypedValue::Int(v) => *v,
                    _ => return Err("Range start must be integer".into()),
                };
                let end_int = match right {
                    TypedValue::Int(v) => *v,
                    _ => return Err("Range end must be integer".into()),
                };
                let range_ty = self.context.struct_type(
                    &[
                        self.i64_ty().into(),
                        self.i64_ty().into(),
                        self.i64_ty().into(),
                    ],
                    false,
                );
                let alloca = self
                    .builder
                    .build_alloca(range_ty, "range")
                    .map_err(llvm_err)?;
                let sptr = self
                    .builder
                    .build_struct_gep(range_ty, alloca, 0, "r_start")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(sptr, start_int)
                    .map_err(llvm_err)?;
                let eptr = self
                    .builder
                    .build_struct_gep(range_ty, alloca, 1, "r_end")
                    .map_err(llvm_err)?;
                self.builder.build_store(eptr, end_int).map_err(llvm_err)?;
                let iptr = self
                    .builder
                    .build_struct_gep(range_ty, alloca, 2, "r_inc")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(
                        iptr,
                        self.i64_ty()
                            .const_int(if inclusive { 1 } else { 0 }, false),
                    )
                    .map_err(llvm_err)?;
                Ok(TypedValue::Struct(alloca, range_ty))
            }
            BinaryOp::And | BinaryOp::Or | BinaryOp::Is | BinaryOp::In => {
                unreachable!("handled before compile_binary_values")
            }
            BinaryOp::Assign => Err("assign is not a binary operator expression".to_string()),
        }
    }

    fn compile_copy_value(&mut self, val: TypedValue<'ctx>) -> Result<TypedValue<'ctx>, String> {
        match &val {
            TypedValue::Int(_)
            | TypedValue::Float(_)
            | TypedValue::Bool(_)
            | TypedValue::Unit
            | TypedValue::Fn(_, _)
            | TypedValue::Closure { .. }
            | TypedValue::CString(_)
            | TypedValue::Ptr(_)
            | TypedValue::FileHandle(_) => Ok(val),
            TypedValue::Str(ptr) => {
                let loaded = self.load_string(*ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.string_type, "str_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Str(new_alloca))
            }
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "struct_copy_ld")
                    .map_err(llvm_err)?;
                let new_alloca = self
                    .builder
                    .build_alloca(bt, "struct_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Struct(new_alloca, *st))
            }
            TypedValue::Enum(ptr, et, inner_type, rc_managed) => {
                let bt: BasicTypeEnum = (*et).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "enum_copy_ld")
                    .map_err(llvm_err)?;
                let new_alloca = self
                    .builder
                    .build_alloca(bt, "enum_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Enum(new_alloca, *et, *inner_type, *rc_managed))
            }
            TypedValue::List(ptr) => {
                let loaded = self.load_list(*ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "list_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::List(new_alloca))
            }
            TypedValue::Map(ptr) => {
                let loaded = self.load_list(*ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "map_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Map(new_alloca))
            }
            TypedValue::Set(ptr) => {
                let loaded = self.load_list(*ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "set_copy")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, loaded)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Set(new_alloca))
            }
            _ => Err("copy not supported for this type".to_string()),
        }
    }

    // ---- HIR-native helper methods ----

    /// Emit scope cleanup and return the given value.
    fn compile_return_value(&mut self, val: TypedValue<'ctx>) -> Result<(), String> {
        if self.is_scope_variable(&val) {
            self.rc_inc_typed_value(&val)?;
        }
        self.emit_scope_cleanup()?;
        if let Some(bv) = val.to_bv() {
            let _ = self.builder.build_return(Some(&bv));
            return Ok(());
        }
        match &val {
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
                    .build_struct_gep(self.stream_type, *ptr, 1, "ret_sl2")
                    .map_err(llvm_err)?;
                let sv = self
                    .builder
                    .build_load(self.list_type, list_field, "ret_sv2")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&sv));
            }
            TypedValue::Task(ptr) => {
                let sv = self
                    .builder
                    .build_load(self.task_type, *ptr, "ret_task")
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
                    .build_load(self.lazylist_type, *ptr, "ret_ll")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&ll_val));
            }
            TypedValue::Nullable(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "ret_nullable")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_return(Some(&loaded));
            }
            _ => {
                let _ = self.builder.build_return(None);
            }
        }
        Ok(())
    }

    /// Emit scope cleanup and return void.
    fn compile_return_void(&mut self) -> Result<(), String> {
        self.emit_scope_cleanup()?;
        let _ = self.builder.build_return(None);
        Ok(())
    }

    /// Compile a unary operation on an already-compiled value.
    fn compile_unary_values(
        &mut self,
        op: UnaryOp,
        val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match op {
            UnaryOp::Neg => match val {
                TypedValue::Int(v) => Ok(TypedValue::Int(
                    self.builder.build_int_neg(v, "neg").map_err(llvm_err)?,
                )),
                TypedValue::Float(v) => Ok(TypedValue::Float(
                    self.builder.build_float_neg(v, "neg").map_err(llvm_err)?,
                )),
                _ => Err("Cannot negate this type".to_string()),
            },
            UnaryOp::Not => match val {
                TypedValue::Bool(v) => Ok(TypedValue::Bool(
                    self.builder.build_not(v, "not").map_err(llvm_err)?,
                )),
                _ => Err("'not' requires boolean operand".to_string()),
            },
            UnaryOp::BitNot => match val {
                TypedValue::Int(v) => Ok(TypedValue::Int(
                    self.builder.build_not(v, "bitnot").map_err(llvm_err)?,
                )),
                _ => Err("'~' requires integer operand".to_string()),
            },
        }
    }

    /// Compile field access on an already-compiled value.
    fn compile_field_access_value(
        &mut self,
        obj_val: TypedValue<'ctx>,
        field: &str,
    ) -> Result<TypedValue<'ctx>, String> {
        // Struct field access: load the struct and extract by field name
        if let TypedValue::Struct(ptr, struct_ty) = &obj_val {
            let bt: BasicTypeEnum = (*struct_ty).into();
            let loaded = self
                .builder
                .build_load(bt, *ptr, "struct_ld")
                .map_err(llvm_err)?
                .into_struct_value();

            // Try numeric index for tuple access: .0, .1, etc.
            if let Ok(idx) = field.parse::<usize>() {
                let field_val = self
                    .builder
                    .build_extract_value(loaded, idx as u32, field)
                    .map_err(llvm_err)?;
                return self.bv_to_typed(field_val);
            }

            let field_names = self.lookup_struct_field_names(*struct_ty);
            let idx = field_names
                .iter()
                .position(|n| n == field)
                .ok_or_else(|| format!("Field '{}' not found on struct", field))?;
            let field_val = self
                .builder
                .build_extract_value(loaded, idx as u32, field)
                .map_err(llvm_err)?;
            return self.bv_to_typed(field_val);
        }

        // Delegate to compile_field_access_on_typed_value for other types
        let val_bt = obj_val.get_type_for_alloca(self);
        self.compile_field_access_on_typed_value(&obj_val, field, val_bt)
    }

    /// Compile range creation from already-compiled start/end values.
    fn compile_range_values(
        &mut self,
        start_val: TypedValue<'ctx>,
        end_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let start_int = match start_val {
            TypedValue::Int(v) => v,
            _ => return Err("Range start must be integer".to_string()),
        };
        let end_int = match end_val {
            TypedValue::Int(v) => v,
            _ => return Err("Range end must be integer".to_string()),
        };
        let range_ty = self.range_type;
        let alloca = self
            .builder
            .build_alloca(range_ty, "range")
            .map_err(llvm_err)?;
        let sptr = self
            .builder
            .build_struct_gep(range_ty, alloca, 0, "r_start")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sptr, start_int)
            .map_err(llvm_err)?;
        let eptr = self
            .builder
            .build_struct_gep(range_ty, alloca, 1, "r_end")
            .map_err(llvm_err)?;
        self.builder.build_store(eptr, end_int).map_err(llvm_err)?;
        let iptr = self
            .builder
            .build_struct_gep(range_ty, alloca, 2, "r_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(iptr, self.i64_ty().const_int(1, false))
            .map_err(llvm_err)?;
        Ok(TypedValue::Struct(alloca, range_ty))
    }
}
