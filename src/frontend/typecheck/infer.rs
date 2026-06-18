use super::*;
use crate::frontend::ast::resolve_type_vars;
use crate::frontend::types::{infer_type_args, types_compatible};

impl TypeChecker {
    /// Infer the type of an expression (structural, not full HM inference)
    pub(crate) fn infer_expr_type(&self, expr: &Expr) -> Result<Type, CompilerError> {
        self.infer_expr_type_with_locals(expr, &HashMap::new())
    }

    pub(crate) fn pattern_local_types(&self, pattern: &Pattern) -> HashMap<String, Type> {
        let mut locals = HashMap::new();
        self.collect_pattern_locals(pattern, &mut locals);
        locals
    }

    pub(crate) fn collect_pattern_locals(
        &self,
        pattern: &Pattern,
        out: &mut HashMap<String, Type>,
    ) {
        match pattern {
            Pattern::Variable(name) => {
                out.entry(name.clone()).or_insert(Type::Named("Int".into()));
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
                        self.collect_pattern_locals(arg_pat, out);
                    }
                }
                for (_, p) in named_fields {
                    self.collect_pattern_locals(p, out);
                }
            }
            Pattern::Or(ps) | Pattern::Tuple(ps) => {
                for p in ps {
                    self.collect_pattern_locals(p, out);
                }
            }
            Pattern::Range(_, _)
            | Pattern::Null
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
        match expr {
            Expr::Literal(Literal::String(_)) | Expr::StringInterpolate(_) => {
                Ok(Type::Named("String".into()))
            }
            Expr::Literal(Literal::Int(_)) => Ok(Type::Named("Int".into())),
            Expr::Literal(Literal::Float(_)) => Ok(Type::Named("Float".into())),
            Expr::Literal(Literal::Bool(_)) => Ok(Type::Named("Bool".into())),
            Expr::Literal(Literal::Char(_)) => Ok(Type::Named("Char".into())),
            Expr::Literal(Literal::Unit) => Ok(Type::Unit),
            Expr::MapLiteral(_) => Ok(Type::Map(
                Box::new(Type::Named("String".into())),
                Box::new(Type::Named("Int".into())),
            )),
            Expr::SetLiteral(_) => Ok(Type::Set(Box::new(Type::Named("Int".into())))),
            Expr::Binary(lhs, op, rhs) => {
                let lt = self.infer_expr_type_with_locals(lhs, locals)?;
                let rt = self.infer_expr_type_with_locals(rhs, locals)?;
                if *op == BinaryOp::Add {
                    if matches!(&lt, Type::Named(ref n) if n == "String")
                        || matches!(&rt, Type::Named(ref n) if n == "String")
                    {
                        return Ok(Type::Named("String".into()));
                    }
                }
                if *op == BinaryOp::And
                    || *op == BinaryOp::Or
                    || *op == BinaryOp::Eq
                    || *op == BinaryOp::Neq
                    || *op == BinaryOp::Lt
                    || *op == BinaryOp::Gt
                    || *op == BinaryOp::Lte
                    || *op == BinaryOp::Gte
                    || *op == BinaryOp::In
                    || *op == BinaryOp::Is
                {
                    return Ok(Type::Named("Bool".into()));
                }
                if *op == BinaryOp::BitAnd
                    || *op == BinaryOp::BitOr
                    || *op == BinaryOp::BitXor
                    || *op == BinaryOp::Shl
                    || *op == BinaryOp::Shr
                {
                    return Ok(Type::Named("Int".into()));
                }
                if *op == BinaryOp::Pow {
                    // Return Float if either operand is Float
                    if matches!(&lt, Type::Named(ref n) if n == "Float")
                        || matches!(&rt, Type::Named(ref n) if n == "Float")
                    {
                        return Ok(Type::Named("Float".into()));
                    }
                    return Ok(lt);
                }
                // Arithmetic: return Float if either operand is Float, else Int
                if matches!(&lt, Type::Named(ref n) if n == "Float")
                    || matches!(&rt, Type::Named(ref n) if n == "Float")
                {
                    return Ok(Type::Named("Float".into()));
                }
                Ok(Type::Named("Int".into()))
            }
            Expr::Call { func, args, .. } => {
                if let Expr::Ident(name) = func.as_ref() {
                    match name.as_str() {
                        "print" | "println" | "send" | "close" | "cancel" => Ok(Type::Unit),
                        "toCString" => Ok(Type::Named("CString".into())),
                        "fromCString" => Ok(Type::Named("String".into())),
                        "readLine" => Ok(Type::Nullable(Box::new(Type::Named("String".into())))),
                        "httpRequest" | "jsonEscape" | "unwrapOr" | "substring" | "str" => {
                            Ok(Type::Named("String".into()))
                        }
                        "toString" | "toUpper" | "toLower" => Ok(Type::Named("String".into())),
                        "receive" | "wait" => Ok(Type::Named("Int".into())),
                        "launch" => Ok(Type::Task(Box::new(Type::Named("Int".into())))),
                        "Stream" => Ok(Type::Stream(Box::new(Type::Named("Int".into())))),
                        "is_done" | "is_cancelled" => Ok(Type::Named("Bool".into())),
                        "withTimeout" => Ok(Type::Nullable(Box::new(Type::Named("Int".into())))),
                        "coroutineScope" => Ok(Type::Named("list".into())),
                        "find" | "findIndex" | "reduce" => {
                            Ok(Type::Nullable(Box::new(Type::Named("Int".into()))))
                        }
                        "foldRight" => Ok(Type::Named("Int".into())),
                        "takeWhile" | "dropWhile" | "sortedBy" => Ok(Type::Named("list".into())),
                        _ => {
                            if let Some(def) = builtin::lookup(name) {
                                return Ok(def.return_type.clone());
                            }
                            if self.registry.lookup_variant(name).is_some() {
                                let enum_name = self
                                    .registry
                                    .variant_to_enum
                                    .get(name)
                                    .cloned()
                                    .unwrap_or_default();
                                Ok(Type::Named(enum_name))
                            } else if let Some(generic_stmt) = self.generic_funs.get(name) {
                                // Generic function: infer type args and resolve return type
                                Ok(self.infer_generic_return_type(generic_stmt, args))
                            } else if let Some(Type::Function(_, ret)) = self.type_env.get(name) {
                                Ok(*ret.clone())
                            } else {
                                Ok(Type::Named("Int".into()))
                            }
                        }
                    }
                } else if let Expr::FieldAccess(receiver, method) = func.as_ref() {
                    let recv_type = self.infer_expr_type_with_locals(receiver, locals)?;
                    if let Some(kind) = builtin::receiver_kind_from_type(&recv_type) {
                        if let Some(def) = builtin::lookup_ufcs(kind, method) {
                            return Ok(def.return_type.clone());
                        }
                    }
                    match (recv_type, method.as_str()) {
                        // Map/Set UFCS methods
                        (Type::Map(_, _), "contains")
                        | (Type::Set(_), "contains")
                        | (Type::Map(_, _), "isEmpty")
                        | (Type::Set(_), "isEmpty") => Ok(Type::Named("Bool".into())),
                        (Type::Map(_, _), "insert") | (Type::Set(_), "insert") => Ok(Type::Unit),
                        (Type::Map(_, _), "remove")
                        | (Type::Map(_, _), "get")
                        | (Type::Set(_), "remove") => {
                            Ok(Type::Nullable(Box::new(Type::Named("Int".into()))))
                        }
                        // Stream UFCS methods
                        (Type::Stream(_), "send") => Ok(Type::Unit),
                        (Type::Stream(_), "receive") => Ok(Type::Named("Int".into())),
                        (Type::Stream(_), "close") => Ok(Type::Unit),
                        // Task UFCS methods
                        (Type::Task(_), "cancel") => Ok(Type::Unit),
                        (Type::Task(_), "is_done") | (Type::Task(_), "is_cancelled") => {
                            Ok(Type::Named("Bool".into()))
                        }
                        (Type::Task(_), "wait") => Ok(Type::Named("Int".into())),
                        _ => {
                            // UFCS fallback: receiver.method(args) → method(receiver, args)
                            let mut all_args = vec![receiver.as_ref().clone()];
                            all_args.extend(args.iter().cloned());
                            self.infer_expr_type_with_locals(
                                &Expr::Call {
                                    func: Box::new(Expr::Ident(method.clone())),
                                    args: all_args,
                                    trailing_lambda: None,
                                },
                                locals,
                            )
                        }
                    }
                } else {
                    Ok(Type::Named("Int".into()))
                }
            }
            Expr::When(w) => {
                let arms = self.when_arms(w);
                if let Some(arm) = arms.first() {
                    let arm_locals = self.pattern_local_types(&arm.pattern);
                    let merged = locals
                        .iter()
                        .chain(arm_locals.iter())
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                    return self.infer_expr_type_with_locals(&arm.body, &merged);
                }
                if let WhenKind::OneLine { then_expr, .. } = &w.kind {
                    return self.infer_expr_type_with_locals(then_expr, locals);
                }
                Ok(Type::Unit)
            }
            Expr::Continue | Expr::Break => Ok(Type::Unit),
            Expr::For(_) => Ok(Type::Unit),
            Expr::FunctionRef(name) => {
                if let Some(ty) = self.type_env.get(name) {
                    Ok(ty.clone())
                } else {
                    Ok(Type::Function(
                        vec![Type::Named("Int".into())],
                        Box::new(Type::Named("Int".into())),
                    ))
                }
            }
            Expr::Copy(inner) => self.infer_expr_type_with_locals(inner, locals),
            Expr::Null => Ok(Type::Nullable(Box::new(Type::Named("Nothing".into())))),
            Expr::OrBlock { nullable, fallback } => {
                let nullable_ty = self.infer_expr_type_with_locals(nullable, locals)?;
                let fallback_ty = self.infer_expr_type_with_locals(fallback, locals)?;
                // Or-block unwraps nullable: T? or { ... } -> T
                Ok(match nullable_ty {
                    Type::Nullable(inner) => {
                        if types_compatible(&inner, &fallback_ty) {
                            *inner
                        } else {
                            fallback_ty
                        }
                    }
                    _ => nullable_ty,
                })
            }
            Expr::Unsafe(inner) => self.infer_expr_type_with_locals(inner, locals),
            Expr::Block(stmts) => stmts
                .last()
                .map(|s| match s {
                    Stmt::Expr { expr: e, .. } => self.infer_expr_type_with_locals(e, locals),
                    Stmt::Return { value: e, .. } => e
                        .as_ref()
                        .map(|re| self.infer_expr_type_with_locals(re, locals))
                        .unwrap_or(Ok(Type::Unit)),
                    _ => Ok(Type::Unit),
                })
                .unwrap_or(Ok(Type::Unit)),
            Expr::Ident(name) => {
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
                    // Smart cast: if variable is known non-null, unwrap nullable type
                    if self.not_null_set.borrow().contains(name) {
                        if let Type::Nullable(inner) = ty {
                            return Ok(*inner.clone());
                        }
                    }
                    Ok(ty.clone())
                } else if builtin::lookup(name).is_some() {
                    Ok(builtin::lookup(name).unwrap().return_type.clone())
                } else {
                    Err(CompilerError::new(format!("Unknown variable: '{}'", name)))
                }
            }
            Expr::Lambda { body, .. } => self.infer_expr_type_with_locals(body, locals),
            Expr::Index(obj, _) => {
                let obj_type = self.infer_expr_type_with_locals(obj, locals)?;
                match obj_type {
                    // Map/Set indexing returns nullable T? (was Option<T>)
                    Type::Map(_, v) => Ok(Type::Nullable(v.clone())),
                    Type::Set(e) => Ok(Type::Nullable(e.clone())),
                    Type::Named(ref n) if n == "String" => Ok(Type::Named("Int".into())),
                    // If obj is nullable, indexing auto short-circuits to nullable
                    Type::Nullable(inner) => match *inner {
                        Type::Map(_, v) => Ok(Type::Nullable(v)),
                        Type::Set(e) => Ok(Type::Nullable(e)),
                        Type::Named(ref n) if n == "String" => Ok(Type::Named("Int".into())),
                        _ => Ok(Type::Nullable(Box::new(Type::Named("Int".into())))),
                    },
                    _ => Ok(Type::Named("Int".into())),
                }
            }
            Expr::FieldAccess(obj, field) => {
                let obj_type = self.infer_expr_type_with_locals(obj, locals)?;
                // If obj is nullable, field access short-circuits to nullable result
                let (inner_obj_type, is_nullable) = match &obj_type {
                    Type::Nullable(inner) => (inner.as_ref(), true),
                    other => (other, false),
                };
                let field_type: Type = if let Type::Named(type_name) = inner_obj_type {
                    let struct_name = match type_name.as_str() {
                        "Str" => "String",
                        "Double" => "Float",
                        other => other,
                    };
                    if let Some(struct_info) = self.registry.structs.get(struct_name) {
                        if let Some(index) = struct_info.field_index.get(field) {
                            struct_info.fields[*index].1.clone()
                        } else {
                            Type::Named("Int".into())
                        }
                    } else {
                        Type::Named("Int".into())
                    }
                } else {
                    Type::Named("Int".into())
                };
                if is_nullable {
                    Ok(Type::Nullable(Box::new(field_type)))
                } else {
                    Ok(field_type)
                }
            }
            Expr::StructLiteral(fields) => {
                let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                if let Some(struct_info) = self.registry.find_struct_by_fields(&field_names) {
                    Ok(Type::Named(struct_info.name.clone()))
                } else {
                    Ok(Type::Named("Int".into()))
                }
            }
            Expr::Assign { value, .. } => self.infer_expr_type_with_locals(value, locals),
            Expr::Unary(op, inner) => match op {
                UnaryOp::Not => Ok(Type::Named("Bool".into())),
                UnaryOp::Neg | UnaryOp::BitNot => {
                    Ok(self.infer_expr_type_with_locals(inner, locals)?)
                }
            },
            _ => Ok(Type::Named("Int".into())),
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
                if matches!(arg, Expr::Lambda { .. }) {
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
}
