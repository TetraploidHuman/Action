//! HIR-native codegen: compile from typed IR without reading `CheckedProgram::program`.

use super::{llvm_err, CodeGen, TypedValue};
use action_frontend::ast::*;
use action_frontend::hir::*;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
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
                            self.infer_return_type(&body.as_expr())
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
                                self.infer_return_type(&fn_body.as_expr())
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
                                self.infer_return_type(&body.as_expr())
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
                    value.as_expr(),
                    ast_type,
                );
                let _ = mutable;
            }
            _ => self.compile_stmt(&stmt.as_stmt())?,
        }
        Ok(())
    }

    pub(super) fn compile_hir_expr(&mut self, expr: &HirExpr) -> Result<TypedValue<'ctx>, String> {
        match &expr.kind {
            HirExprKind::Literal(lit) => self.compile_literal(lit),
            HirExprKind::Ident(name) => self.compile_ident(name),
            HirExprKind::Null => self.compile_null(),
            HirExprKind::Continue => {
                if let Some(target) = self.continue_target {
                    self.builder
                        .build_unconditional_branch(target)
                        .map_err(llvm_err)?;
                    Ok(TypedValue::Unit)
                } else {
                    Err("continue outside loop".to_string())
                }
            }
            HirExprKind::Break => {
                if let Some(target) = self.break_target {
                    self.builder
                        .build_unconditional_branch(target)
                        .map_err(llvm_err)?;
                    Ok(TypedValue::Unit)
                } else {
                    Err("break outside loop".to_string())
                }
            }
            HirExprKind::FunctionRef(name) => self.compile_function_ref(name),
            HirExprKind::Block(stmts) => self.compile_hir_block(stmts, &expr.ty),
            HirExprKind::Binary(lhs, op, rhs) => {
                if matches!(
                    op,
                    BinaryOp::And | BinaryOp::Or | BinaryOp::Is | BinaryOp::In
                ) {
                    return self.compile_binary(&lhs.as_expr(), *op, &rhs.as_expr());
                }
                let left = self.compile_hir_expr(lhs)?;
                let right = self.compile_hir_expr(rhs)?;
                self.compile_binary_values(*op, &left, &right, &expr.ty)
            }
            HirExprKind::Unary(op, inner) => self.compile_unary(*op, &inner.as_expr()),
            HirExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => self.compile_hir_call(func, args, trailing_lambda.as_ref()),
            HirExprKind::When(w) => self.compile_when(&w.to_when()),
            HirExprKind::For(f) => self.compile_for(&f.to_for()),
            HirExprKind::Assign { target, value } => {
                self.compile_assign(&target.as_expr(), &value.as_expr())
            }
            HirExprKind::StringInterpolate(parts) => {
                let ast_parts: Vec<StringPart> = parts
                    .iter()
                    .map(|p| match p {
                        HirStringPart::Literal(s) => StringPart::Literal(s.clone()),
                        HirStringPart::Expr(e) => StringPart::Expr(Box::new(e.as_expr())),
                    })
                    .collect();
                self.compile_string_interp(&ast_parts)
            }
            HirExprKind::FieldAccess(obj, field) => {
                self.compile_field_access(&obj.as_expr(), field)
            }
            HirExprKind::StructLiteral(fields) => {
                let ast_fields: Vec<(String, Expr)> = fields
                    .iter()
                    .map(|(n, e)| (n.clone(), e.as_expr()))
                    .collect();
                self.compile_struct_lit(&ast_fields)
            }
            HirExprKind::MapLiteral(entries) => {
                let ast_entries: Vec<(Expr, Expr)> = entries
                    .iter()
                    .map(|(k, v)| (k.as_expr(), v.as_expr()))
                    .collect();
                self.compile_map_lit(&ast_entries)
            }
            HirExprKind::SetLiteral(elements) => {
                let ast_elems: Vec<Expr> = elements.iter().map(HirExpr::as_expr).collect();
                self.compile_set_lit(&ast_elems)
            }
            HirExprKind::Lambda { params, body, .. } => {
                self.compile_lambda(params, &body.as_expr())
            }
            HirExprKind::Index(obj, idx) => self.compile_index(&obj.as_expr(), &idx.as_expr()),
            HirExprKind::Range(start, end) => self.compile_range(&start.as_expr(), &end.as_expr()),
            HirExprKind::Tuple(items) => {
                let ast_items: Vec<(Option<String>, Expr)> = items
                    .iter()
                    .map(|(n, e)| (n.clone(), e.as_expr()))
                    .collect();
                self.compile_tuple(&ast_items)
            }
            HirExprKind::OrBlock { nullable, fallback } => {
                self.compile_or_block(&nullable.as_expr(), &fallback.as_expr())
            }
            HirExprKind::Copy(inner) => {
                let val = self.compile_hir_expr(inner)?;
                self.compile_copy_value(val)
            }
            HirExprKind::Unsafe(inner) => self.compile_hir_expr(inner),
        }
    }

    fn compile_hir_block(
        &mut self,
        stmts: &[HirStmt],
        result_ty: &Type,
    ) -> Result<TypedValue<'ctx>, String> {
        let ast_stmts: Vec<Stmt> = stmts.iter().map(HirStmt::as_stmt).collect();
        let _ = result_ty;
        self.compile_block(&ast_stmts)
    }

    fn compile_hir_call(
        &mut self,
        func: &HirExpr,
        args: &[HirExpr],
        trailing_lambda: Option<&Box<HirExpr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_call(
            &func.as_expr(),
            &args.iter().map(HirExpr::as_expr).collect::<Vec<_>>(),
            &trailing_lambda.map(|b| Box::new(b.as_expr())),
        )
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
}
