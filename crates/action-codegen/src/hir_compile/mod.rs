//! HIR-native codegen: compile from typed IR without reading `CheckedProgram::program`.

use super::{llvm_err, CodeGen, TypedValue};
use action_frontend::ast::*;
use action_frontend::hir::*;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use std::collections::HashMap;

mod control;
mod expr;
mod stmt;
mod values;

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
            self.registry.register_hir(stmt)?;
            match stmt {
                HirStmt::TypeAlias {
                    name, definition, ..
                } => {
                    if let Type::Struct(fields) = definition {
                        let field_tys: Vec<BasicTypeEnum> = fields
                            .iter()
                            .map(|(_, ty)| self.ast_type_to_basic_type(ty))
                            .collect::<Result<_, _>>()?;
                        let struct_ty = self.context.struct_type(&field_tys, false);
                        self.type_layout
                            .named_structs
                            .insert(name.clone(), struct_ty);
                    }
                }
                HirStmt::Enum { name, .. } => {
                    let i64 = self.i64_ty();
                    let ptr = self.ptr_ty();
                    let enum_ty = self.context.struct_type(&[i64.into(), ptr.into()], false);
                    self.type_layout.enum_types.insert(name.clone(), enum_ty);
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
                fn_or_fallback,
                ..
            } = stmt
            {
                if !type_params.is_empty() {
                    self.mono_cache
                        .generic_fun_defs
                        .insert(name.clone(), stmt.clone());
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
                    .collect::<Result<_, _>>()?;
                let ret_type = if name == "main" {
                    Some(Type::Named("Int".into()))
                } else {
                    return_type.clone().or_else(|| {
                        if all_typed {
                            Some(self.infer_hir_expr_type(body))
                        } else {
                            None
                        }
                    })
                };
                let is_propagating = fn_or_fallback.is_none()
                    && name != "main"
                    && self
                        .fallibility
                        .symbols
                        .get(name)
                        .is_some_and(|s| s.is_fallible);
                let fn_type = if is_propagating {
                    self.build_fallible_fn_type(ret_type.as_ref().unwrap(), &param_llvm_tys)?
                } else {
                    self.build_fn_type(ret_type.as_ref(), &mangled, &param_llvm_tys)?
                };
                self.module.add_function(&mangled, fn_type, None);
                if is_propagating {
                    self.mono_cache.fallible_user_fns.insert(mangled.clone());
                }
                if name != "main" {
                    if let Some(rt) = ret_type {
                        self.mono_cache.fun_return_types.insert(mangled, rt);
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
                            .collect::<Result<_, _>>()?;
                        let ret_type = return_type.clone().or_else(|| {
                            if params.iter().all(|p| p.ty.is_some()) {
                                Some(self.infer_hir_expr_type(fn_body))
                            } else {
                                None
                            }
                        });
                        let is_propagating = self
                            .fallibility
                            .symbols
                            .get(&mangled)
                            .is_some_and(|s| s.is_fallible);
                        let fn_type = if is_propagating {
                            self.build_fallible_fn_type(
                                ret_type.as_ref().unwrap(),
                                &param_llvm_tys,
                            )?
                        } else {
                            self.build_fn_type(ret_type.as_ref(), &mangled, &param_llvm_tys)?
                        };
                        self.module.add_function(&mangled, fn_type, None);
                        if is_propagating {
                            self.mono_cache.fallible_user_fns.insert(mangled.clone());
                        }
                        if let Some(rt) = ret_type {
                            self.mono_cache.fun_return_types.insert(mangled, rt);
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
                            .collect::<Result<_, _>>()?;
                        let ret_type = return_type.clone().or_else(|| {
                            if params.iter().all(|p| p.ty.is_some()) {
                                Some(self.infer_hir_expr_type(body))
                            } else {
                                None
                            }
                        });
                        let fn_type =
                            self.build_fn_type(ret_type.as_ref(), &fn_name, &param_llvm_tys)?;
                        self.module.add_function(&fn_name, fn_type, None);
                        if let Some(rt) = ret_type {
                            self.mono_cache.fun_return_types.insert(fn_name, rt);
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
                self.compile_hir_let(stmt)?;
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
                        self.ast_type_to_basic_type(ann)?,
                        self.param_val_kind(Some(ann))?,
                        Some(ann.clone()),
                    )
                } else {
                    (
                        self.ast_type_to_basic_type(hir_ty)?,
                        self.param_val_kind(Some(hir_ty))?,
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
                if self.try_compile_mutating_ufcs_stmt_writeback(expr)? {
                    return Ok(());
                }
                let result = self.compile_hir_expr(expr)?;
                self.rc_discard_value(&result)?;
            }
            HirStmt::Return { value: Some(v), .. } => {
                if self.try_compile_hir_return_tco(v)? {
                    return Ok(());
                }
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
                fn_or_fallback,
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
                self.compile_fun_def_hir(
                    &fn_name,
                    name,
                    params,
                    return_type.as_ref(),
                    body,
                    fn_or_fallback.as_ref(),
                )?;
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
                        fn_or_fallback,
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
                            fn_or_fallback.as_ref(),
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
            HirStmt::External { .. } => self.compile_hir_external(stmt)?,
            HirStmt::ExternalType { .. } => self.compile_hir_external_type(stmt)?,
        }
        Ok(())
    }

    pub(super) fn compile_hir_expr(&mut self, expr: &HirExpr) -> Result<TypedValue<'ctx>, String> {
        match &expr.kind {
            HirExprKind::Literal(lit) => self.compile_literal(lit),
            HirExprKind::Ident(name) => self.compile_ident(name),
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
            } => {
                if self.in_fallible_region() {
                    if let Some(v) = self.try_compile_fallible_call_in_region(expr)? {
                        return self.unwrap_fallible_value(v);
                    }
                }
                if action_frontend::fallibility_narrowing::hir_call_is_proven_safe(
                    func,
                    args,
                    &self.narrowing,
                ) {
                    if let Some(v) = self.try_compile_fallible_expr(expr)? {
                        return self.unwrap_fallible_value(v);
                    }
                    let v = self.compile_hir_call(func, args, trailing_lambda.as_ref())?;
                    return self.unwrap_fallible_value(v);
                }
                let v = self.compile_hir_call(func, args, trailing_lambda.as_ref())?;
                self.unwrap_if_compile_time_safe_call(func, args, v)
            }
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
            HirExprKind::OrBlock { fallible, fallback } => {
                self.compile_hir_or_block(fallible, fallback)
            }
            HirExprKind::Copy(inner) => {
                let val = self.compile_hir_expr(inner)?;
                self.compile_copy_value(val)
            }
            HirExprKind::Unsafe(inner) => self.compile_hir_expr(inner),
        }
    }
}
