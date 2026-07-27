use super::inference::InferenceEngine;
use super::*;
use crate::ast::resolve_type_vars;
use crate::types::infer_type_args;

impl TypeChecker {
    fn hm_expr_diverges(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Break | ExprKind::Continue => true,
            ExprKind::Block(stmts) => match stmts.last() {
                Some(Stmt::Break { .. } | Stmt::Continue { .. }) => true,
                Some(Stmt::Expr { expr, .. }) => Self::hm_expr_diverges(expr),
                _ => false,
            },
            _ => false,
        }
    }

    /// Infer the type of an expression (Hindley-Milner with structural fallback)
    pub(crate) fn infer_expr_type(&self, expr: &Expr) -> Result<Type, CompilerError> {
        self.infer_expr_type_with_locals(expr, &HashMap::new())
    }

    pub(crate) fn pattern_local_types(&self, pattern: &Pattern) -> HashMap<String, Type> {
        let mut engine = InferenceEngine::new();
        let mut locals = HashMap::new();
        self.collect_pattern_locals_hm(pattern, &mut engine, &mut locals);
        locals
            .into_iter()
            .map(|(k, v)| (k, engine.resolve(&v)))
            .collect()
    }

    fn collect_pattern_locals_hm(
        &self,
        pattern: &Pattern,
        engine: &mut InferenceEngine,
        out: &mut HashMap<String, Type>,
    ) {
        match pattern {
            Pattern::Variable(name) => {
                out.entry(name.clone())
                    .or_insert_with(|| engine.fresh_var());
            }
            Pattern::Constructor {
                name,
                args,
                named_fields,
            } => {
                let param_tys: Vec<Type> = self
                    .registry
                    .lookup_variant(name)
                    .map(|(_, v)| {
                        v.params
                            .iter()
                            .map(|p| match p {
                                EnumVariantParam::Positional(t) => t.clone(),
                                EnumVariantParam::Named { ty, .. } => ty.clone(),
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                for (arg_pat, ty) in args.iter().zip(param_tys.iter()) {
                    if let Pattern::Variable(var) = arg_pat {
                        out.insert(var.clone(), ty.clone());
                    } else {
                        self.collect_pattern_locals_hm(arg_pat, engine, out);
                    }
                }
                for (_, p) in named_fields {
                    self.collect_pattern_locals_hm(p, engine, out);
                }
            }
            Pattern::Or(ps) | Pattern::Tuple(ps) => {
                for p in ps {
                    self.collect_pattern_locals_hm(p, engine, out);
                }
            }
            Pattern::Range(_, _)
            | Pattern::Literal(_)
            | Pattern::IsType(_)
            | Pattern::Wildcard
            | Pattern::Expr(_) => {}
        }
    }

    pub(crate) fn infer_expr_type_with_locals(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
    ) -> Result<Type, CompilerError> {
        let mut engine = InferenceEngine::new();
        let ty = self.hm_infer_expr(expr, locals, &mut engine)?;
        Ok(engine.resolve(&ty))
    }

    fn hm_infer_expr(
        &self,
        expr: &Expr,
        locals: &HashMap<String, Type>,
        engine: &mut InferenceEngine,
    ) -> Result<Type, CompilerError> {
        match &expr.kind {
            ExprKind::Literal(Literal::String(_)) | ExprKind::StringInterpolate(_) => {
                Ok(Type::Named("String".into()))
            }
            ExprKind::Literal(Literal::Int(_)) => Ok(Type::Named("Int".into())),
            ExprKind::Literal(Literal::Float(_)) => Ok(Type::Named("Float".into())),
            ExprKind::Literal(Literal::Bool(_)) => Ok(Type::Named("Bool".into())),
            ExprKind::Literal(Literal::Char(_)) => Ok(Type::Named("Char".into())),
            ExprKind::Literal(Literal::Unit) => Ok(Type::Unit),
            ExprKind::MapLiteral(entries) => {
                if entries.is_empty() {
                    let k = engine.fresh_var();
                    let v = engine.fresh_var();
                    return Ok(Type::Map(Box::new(k), Box::new(v)));
                }
                let mut key_ty = self.hm_infer_expr(&entries[0].0, locals, engine)?;
                let mut val_ty = self.hm_infer_expr(&entries[0].1, locals, engine)?;
                for (k, v) in entries.iter().skip(1) {
                    let kt = self.hm_infer_expr(k, locals, engine)?;
                    let vt = self.hm_infer_expr(v, locals, engine)?;
                    engine
                        .unify(&key_ty, &kt)
                        .map_err(|e| CompilerError::new(e))?;
                    engine
                        .unify(&val_ty, &vt)
                        .map_err(|e| CompilerError::new(e))?;
                    key_ty = engine.resolve(&key_ty);
                    val_ty = engine.resolve(&val_ty);
                }
                Ok(Type::Map(Box::new(key_ty), Box::new(val_ty)))
            }
            ExprKind::SetLiteral(elems) => {
                if elems.is_empty() {
                    return Ok(Type::Set(Box::new(engine.fresh_var())));
                }
                let mut elem_ty = self.hm_infer_expr(&elems[0], locals, engine)?;
                for e in elems.iter().skip(1) {
                    let t = self.hm_infer_expr(e, locals, engine)?;
                    engine
                        .unify(&elem_ty, &t)
                        .map_err(|e| CompilerError::new(e))?;
                    elem_ty = engine.resolve(&elem_ty);
                }
                Ok(Type::Set(Box::new(elem_ty)))
            }
            ExprKind::Binary(lhs, op, rhs) => {
                let lt = self.hm_infer_expr(lhs, locals, engine)?;
                let rt = self.hm_infer_expr(rhs, locals, engine)?;
                if *op == BinaryOp::Add {
                    if matches!(&engine.resolve(&lt), Type::Named(ref n) if n == "String")
                        || matches!(&engine.resolve(&rt), Type::Named(ref n) if n == "String")
                    {
                        return Ok(Type::Named("String".into()));
                    }
                }
                if matches!(
                    op,
                    BinaryOp::And
                        | BinaryOp::Or
                        | BinaryOp::Eq
                        | BinaryOp::Neq
                        | BinaryOp::Lt
                        | BinaryOp::Gt
                        | BinaryOp::Lte
                        | BinaryOp::Gte
                        | BinaryOp::In
                        | BinaryOp::Is
                ) {
                    return Ok(Type::Named("Bool".into()));
                }
                if matches!(
                    op,
                    BinaryOp::BitAnd
                        | BinaryOp::BitOr
                        | BinaryOp::BitXor
                        | BinaryOp::Shl
                        | BinaryOp::Shr
                ) {
                    return Ok(Type::Named("Int".into()));
                }
                if matches!(op, BinaryOp::Range | BinaryOp::RangeExclusive) {
                    // Same surface type as ExprKind::Range — enables List UFCS (contains/toList).
                    return Ok(Type::Generic(
                        Box::new(Type::Named("List".into())),
                        vec![Type::Named("Int".into())],
                    ));
                }
                let lt = engine.resolve(&lt);
                let rt = engine.resolve(&rt);
                if *op == BinaryOp::Pow {
                    if matches!(&lt, Type::Named(ref n) if n == "Float")
                        || matches!(&rt, Type::Named(ref n) if n == "Float")
                    {
                        return Ok(Type::Named("Float".into()));
                    }
                    return Ok(lt);
                }
                if matches!(&lt, Type::Named(ref n) if n == "Float")
                    || matches!(&rt, Type::Named(ref n) if n == "Float")
                {
                    return Ok(Type::Named("Float".into()));
                }
                Ok(Type::Named("Int".into()))
            }
            ExprKind::Call { func, args, .. } => {
                if let ExprKind::Ident(name) = &func.kind {
                    match name.as_str() {
                        "List" | "__list" => {
                            if args.is_empty() {
                                let elem = engine.fresh_var();
                                return Ok(Type::Generic(
                                    Box::new(Type::Named("List".into())),
                                    vec![elem],
                                ));
                            }
                            let mut elem_ty = self.hm_infer_expr(&args[0], locals, engine)?;
                            for arg in args.iter().skip(1) {
                                let t = self.hm_infer_expr(arg, locals, engine)?;
                                engine
                                    .unify(&elem_ty, &t)
                                    .map_err(|e| CompilerError::new(e))?;
                                elem_ty = engine.resolve(&elem_ty);
                            }
                            Ok(Type::Generic(
                                Box::new(Type::Named("List".into())),
                                vec![elem_ty],
                            ))
                        }
                        "Set" => {
                            if args.is_empty() {
                                return Ok(Type::Set(Box::new(engine.fresh_var())));
                            }
                            let mut elem_ty = self.hm_infer_expr(&args[0], locals, engine)?;
                            for arg in args.iter().skip(1) {
                                let t = self.hm_infer_expr(arg, locals, engine)?;
                                engine
                                    .unify(&elem_ty, &t)
                                    .map_err(|e| CompilerError::new(e))?;
                                elem_ty = engine.resolve(&elem_ty);
                            }
                            Ok(Type::Set(Box::new(elem_ty)))
                        }
                        "launch" => Ok(Type::Task(Box::new(engine.fresh_var()))),
                        "Stream" => Ok(Type::Stream(Box::new(engine.fresh_var()))),
                        _ => {
                            if self.registry.lookup_variant(name).is_some() {
                                let enum_name = self
                                    .registry
                                    .variant_to_enum
                                    .get(name)
                                    .cloned()
                                    .unwrap_or_default();
                                return Ok(Type::Named(enum_name));
                            }
                            if let Some(generic_stmt) = self.generic_funs.get(name) {
                                return Ok(self.infer_generic_return_type(generic_stmt, args));
                            }
                            if let Some(Type::Function(param_tys, ret)) = self.type_env.get(name) {
                                for (arg, pt) in args.iter().zip(param_tys.iter()) {
                                    let at = self.hm_infer_expr(arg, locals, engine)?;
                                    let _ = engine.unify(pt, &at);
                                }
                                return Ok(*ret.clone());
                            }
                            let mut arg_tys = Vec::with_capacity(args.len());
                            for arg in args {
                                arg_tys.push(self.hm_infer_expr(arg, locals, engine)?);
                            }
                            if let Some(ty) = builtin::lookup_return_type_for_args(name, &arg_tys) {
                                return Ok(ty);
                            }
                            Ok(engine.fresh_var())
                        }
                    }
                } else if let ExprKind::FieldAccess(receiver, method) = &func.kind {
                    let recv_type = self.hm_infer_expr(receiver, locals, engine)?;
                    let recv_type = engine.resolve(&recv_type);
                    if let Some(kind) = builtin::receiver_kind_from_type(&recv_type) {
                        if let Some(ty) = builtin::lookup_ufcs_return_type(kind, method) {
                            return Ok(ty);
                        }
                    }
                    let mut all_args = vec![receiver.as_ref().clone()];
                    all_args.extend(args.iter().cloned());
                    self.hm_infer_expr(
                        &ExprKind::Call {
                            func: Box::new(Expr::ident(method)),
                            args: all_args,
                            trailing_lambda: None,
                        }
                        .into(),
                        locals,
                        engine,
                    )
                } else if let ExprKind::Ident(_) = &func.kind {
                    Ok(engine.fresh_var())
                } else {
                    let ft = self.hm_infer_expr(func, locals, engine)?;
                    let ft = engine.resolve(&ft);
                    if let Type::Function(param_tys, ret) = ft {
                        for (arg, pt) in args.iter().zip(param_tys.iter()) {
                            let at = self.hm_infer_expr(arg, locals, engine)?;
                            let _ = engine.unify(pt, &at);
                        }
                        Ok(*ret)
                    } else {
                        Ok(engine.fresh_var())
                    }
                }
            }
            ExprKind::When(w) => {
                if let WhenKind::OneLine {
                    then_expr,
                    else_expr,
                    ..
                } = &w.kind
                {
                    let then_div = Self::hm_expr_diverges(then_expr);
                    let else_div = Self::hm_expr_diverges(else_expr);
                    if then_div && else_div {
                        return Ok(Type::Unit);
                    }
                    if then_div {
                        return self.hm_infer_expr(else_expr, locals, engine);
                    }
                    if else_div {
                        return self.hm_infer_expr(then_expr, locals, engine);
                    }
                    let then_ty = self.hm_infer_expr(then_expr, locals, engine)?;
                    let else_ty = self.hm_infer_expr(else_expr, locals, engine)?;
                    engine
                        .unify(&then_ty, &else_ty)
                        .map_err(|e| CompilerError::new(e))?;
                    return Ok(engine.resolve(&then_ty));
                }
                let arms = self.when_arms(w);
                if arms.is_empty() {
                    return Ok(Type::Unit);
                }
                let mut result: Option<Type> = None;
                for arm in arms {
                    let arm_locals = self.pattern_local_types(&arm.pattern);
                    let merged: HashMap<String, Type> = locals
                        .iter()
                        .chain(arm_locals.iter())
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    let body_ty = self.hm_infer_expr(&arm.body, &merged, engine)?;
                    if let Some(ref prev) = result {
                        engine
                            .unify(prev, &body_ty)
                            .map_err(|e| CompilerError::new(e))?;
                    } else {
                        result = Some(body_ty);
                    }
                }
                Ok(result.unwrap_or(Type::Unit))
            }
            ExprKind::Continue | ExprKind::Break => Ok(Type::Unit),
            ExprKind::For(_) => Ok(Type::Unit),
            ExprKind::FunctionRef(name) => {
                if let Some(ty) = self.type_env.get(name) {
                    Ok(ty.clone())
                } else if let Some(def) = builtin::lookup(name) {
                    Ok(def.return_type.clone())
                } else {
                    let a = engine.fresh_var();
                    let b = engine.fresh_var();
                    Ok(Type::Function(vec![a], Box::new(b)))
                }
            }
            ExprKind::Copy(inner) => self.hm_infer_expr(inner, locals, engine),
            ExprKind::OrBlock { fallible, fallback } => {
                let fallible_ty = self.hm_infer_expr(fallible, locals, engine)?;
                let fallback_ty = self.hm_infer_expr(fallback, locals, engine)?;
                let fallible_ty = engine.resolve(&fallible_ty);
                let fallback_ty = engine.resolve(&fallback_ty);
                let _ = engine.unify(&fallible_ty, &fallback_ty);
                Ok(engine.resolve(&fallible_ty))
            }
            ExprKind::Unsafe(inner) => self.hm_infer_expr(inner, locals, engine),
            ExprKind::Block(stmts) => {
                // Scoped locals for block `val`/`var` (Kotlin-style last-expr value).
                let mut block_locals = locals.clone();
                let mut last_ty = Type::Unit;
                for s in stmts {
                    match s {
                        Stmt::Let {
                            name,
                            type_ann,
                            value,
                            ..
                        }
                        | Stmt::Const {
                            name,
                            type_ann,
                            value,
                            ..
                        } => {
                            let inferred = self.hm_infer_expr(value, &block_locals, engine)?;
                            let ty = type_ann.clone().unwrap_or(inferred);
                            block_locals.insert(name.clone(), ty);
                            last_ty = Type::Unit;
                        }
                        Stmt::Expr { expr: e, .. } => {
                            last_ty = self.hm_infer_expr(e, &block_locals, engine)?;
                        }
                        Stmt::Return { value: e, .. } => {
                            last_ty = e
                                .as_ref()
                                .map(|re| self.hm_infer_expr(re, &block_locals, engine))
                                .transpose()?
                                .unwrap_or(Type::Unit);
                        }
                        _ => {
                            last_ty = Type::Unit;
                        }
                    }
                }
                Ok(last_ty)
            }
            ExprKind::Ident(name) => {
                if let Some(ty) = locals.get(name) {
                    return Ok(ty.clone());
                }
                if self.registry.lookup_variant(name).is_some() {
                    let enum_name = self
                        .registry
                        .variant_to_enum
                        .get(name)
                        .cloned()
                        .unwrap_or_default();
                    Ok(Type::Named(enum_name))
                } else if let Some(ty) = self.type_env.get(name) {
                    Ok(ty.clone())
                } else if let Some(def) = builtin::lookup(name) {
                    Ok(def.return_type.clone())
                } else {
                    Err(CompilerError::new(format!("Unknown variable: '{}'", name)))
                }
            }
            ExprKind::Lambda { params, body, .. } => {
                if params.is_empty() {
                    return self.hm_infer_expr(body, locals, engine);
                }
                let mut lambda_locals = locals.clone();
                let mut param_tys = Vec::new();
                for p in params {
                    let pt = engine.fresh_var();
                    lambda_locals.insert(p.clone(), pt.clone());
                    param_tys.push(pt);
                }
                let body_ty = self.hm_infer_expr(body, &lambda_locals, engine)?;
                Ok(Type::Function(param_tys, Box::new(body_ty)))
            }
            ExprKind::Index(obj, idx) => {
                let obj_ty = self.hm_infer_expr(obj, locals, engine)?;
                let obj_type = engine.resolve(&obj_ty);
                match obj_type {
                    Type::Generic(base, args) if args.len() == 1 => {
                        if matches!(base.as_ref(), Type::Named(ref n) if n == "List") {
                            return Ok(args[0].clone());
                        }
                    }
                    Type::Map(_, v) => return Ok((*v).clone()),
                    Type::Set(e) => return Ok((*e).clone()),
                    // M89: bare Named collection (no element args) — soft defaults for index lvalues.
                    Type::Named(ref n) if n == "List" => {
                        return Ok(Type::Named("Int".into()));
                    }
                    Type::Named(ref n) if n == "Map" => {
                        return Ok(Type::Named("Int".into()));
                    }
                    Type::Named(ref n) if n == "String" => return Ok(Type::Named("Int".into())),
                    Type::Struct(fields) => {
                        // Anonymous tuple / pair: only in-bounds integer literals select a field
                        // (matches codegen + M64 E005).
                        if let ExprKind::Literal(Literal::Int(n)) = &idx.kind {
                            let n = *n;
                            if n >= 0 {
                                let i = n as usize;
                                if let Some((_, ty)) = fields.get(i) {
                                    return Ok(ty.clone());
                                }
                            }
                            return Err(crate::error::e005_struct_index_invalid(
                                format!(
                                    "tuple/struct index {n} is out of range for {} field(s)",
                                    fields.len()
                                ),
                                expr.span,
                            ));
                        }
                        return Err(crate::error::e005_struct_index_invalid(
                            "tuple/struct index must be an integer literal",
                            expr.span,
                        ));
                    }
                    _ => {}
                }
                Ok(engine.fresh_var())
            }
            ExprKind::FieldAccess(obj, field) => {
                let obj_ty = self.hm_infer_expr(obj, locals, engine)?;
                let obj_type = engine.resolve(&obj_ty);
                if let Type::Named(type_name) = &obj_type {
                    let struct_name = match type_name.as_str() {
                        "Str" => "String",
                        "Double" => "Float",
                        other => other,
                    };
                    if let Some(struct_info) = self.registry.structs.get(struct_name) {
                        if let Some(index) = struct_info.field_index.get(field) {
                            return Ok(struct_info.fields[*index].1.clone());
                        }
                        return Err(crate::error::e013_unknown_struct_field(
                            struct_name,
                            field,
                            expr.span,
                        ));
                    }
                }
                Ok(engine.fresh_var())
            }
            ExprKind::StructLiteral { type_name, fields } => {
                if let Some(name) = type_name {
                    if self.registry.get_struct(name).is_some() {
                        Ok(Type::Named(name.clone()))
                    } else {
                        Err(CompilerError::new(format!(
                            "Unknown struct type '{}'",
                            name
                        ))
                        .with_span(expr.span))
                    }
                } else {
                    let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                    if let Some(struct_info) = self.registry.find_struct_by_fields(&field_names) {
                        Ok(Type::Named(struct_info.name.clone()))
                    } else {
                        Ok(engine.fresh_var())
                    }
                }
            }
            ExprKind::Assign { value, .. } => self.hm_infer_expr(value, locals, engine),
            ExprKind::Unary(op, inner) => match op {
                UnaryOp::Not => Ok(Type::Named("Bool".into())),
                UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot => {
                    self.hm_infer_expr(inner, locals, engine)
                }
            },
            ExprKind::Tuple(exprs) => {
                let field_tys: Result<Vec<(String, Type)>, CompilerError> = exprs
                    .iter()
                    .enumerate()
                    .map(|(i, (_, e))| {
                        self.hm_infer_expr(e, locals, engine)
                            .map(|t| (format!("_{}", i), t))
                    })
                    .collect();
                Ok(Type::Struct(field_tys?))
            }
            ExprKind::Range(_, _) => Ok(Type::Generic(
                Box::new(Type::Named("List".into())),
                vec![Type::Named("Int".into())],
            )),
        }
    }

    /// Infer the return type of a generic function call by unifying parameter types
    /// and substituting the result into the declared return type.
    pub(crate) fn infer_generic_return_type(&self, stmt: &Stmt, args: &[Expr]) -> Type {
        if let Stmt::Fun {
            params,
            return_type,
            ..
        } = stmt
        {
            let param_tys: Vec<Type> = params
                .iter()
                .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                .collect();
            let mut arg_tys = Vec::new();
            let mut filtered_params = Vec::new();
            for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                if matches!(&arg.kind, ExprKind::Lambda { .. }) {
                    continue;
                }
                arg_tys.push(self.try_infer_expr_type(arg));
                filtered_params.push(param_ty.clone());
            }
            if let Ok(type_map) = infer_type_args(&filtered_params, &arg_tys) {
                if let Some(ret) = return_type {
                    return resolve_type_vars(ret, &type_map);
                }
            }
        }
        Type::Named("Int".into())
    }

    /// Infer an unannotated function parameter type from its usage in the body.
    pub(crate) fn infer_param_type_from_body(
        &self,
        param: &str,
        all_params: &[Param],
        body: &Expr,
    ) -> Type {
        let mut eng = InferenceEngine::new();
        let mut locals = HashMap::new();
        let pv = eng.fresh_var();
        locals.insert(param.to_string(), pv.clone());
        for p in all_params {
            if p.name != param {
                if let Some(ty) = &p.ty {
                    locals.insert(p.name.clone(), ty.clone());
                } else if p.name != "self" {
                    locals.insert(p.name.clone(), eng.fresh_var());
                }
            }
        }
        let _ = self.hm_infer_expr(body, &locals, &mut eng);
        eng.resolve(&pv)
    }
}
