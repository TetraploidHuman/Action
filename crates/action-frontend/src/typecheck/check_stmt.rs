use super::*;
use crate::types::{infer_type_args, mangle_name, types_compatible};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectionIndexKind {
    List,
    Map,
    Set,
}

impl TypeChecker {
    fn collection_kind_from_type(ty: &Type) -> Option<CollectionIndexKind> {
        match ty {
            Type::Generic(base, _) if matches!(base.as_ref(), Type::Named(n) if n == "List") => {
                Some(CollectionIndexKind::List)
            }
            Type::LazyList(_) => Some(CollectionIndexKind::List),
            Type::Map(_, _) => Some(CollectionIndexKind::Map),
            Type::Set(_) => Some(CollectionIndexKind::Set),
            Type::Named(n) if n == "List" => Some(CollectionIndexKind::List),
            _ => None,
        }
    }

    fn collection_kind_from_ast(expr: &Expr) -> Option<CollectionIndexKind> {
        match &expr.kind {
            ExprKind::Call { func, .. } => match &func.kind {
                ExprKind::Ident(name) if name == "List" || name == "__list" => {
                    Some(CollectionIndexKind::List)
                }
                ExprKind::Ident(name) if name == "Map" || name == "__map" => {
                    Some(CollectionIndexKind::Map)
                }
                ExprKind::Ident(name) if name == "Set" || name == "__set" => {
                    Some(CollectionIndexKind::Set)
                }
                _ => None,
            },
            ExprKind::MapLiteral(_) => Some(CollectionIndexKind::Map),
            ExprKind::SetLiteral(_) => Some(CollectionIndexKind::Set),
            ExprKind::Index(obj, _) | ExprKind::FieldAccess(obj, _) => {
                Self::collection_kind_from_ast(obj)
            }
            _ => None,
        }
    }

    fn collection_index_receiver_kind(&self, obj: &Expr) -> Option<CollectionIndexKind> {
        Self::collection_kind_from_type(&self.try_infer_expr_type(obj))
            .or_else(|| {
                if let ExprKind::Ident(name) = &obj.kind {
                    self.type_env
                        .get(name)
                        .and_then(|ty| Self::collection_kind_from_type(ty))
                } else {
                    None
                }
            })
            .or_else(|| Self::collection_kind_from_ast(obj))
    }

    fn expr_is_fallible(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Call { func, .. } => self.fallibility.call_expr_is_fallible(func),
            ExprKind::Index(_, _) => true,
            ExprKind::FieldAccess(_, _) => true,
            ExprKind::OrBlock { fallible, .. } => self.expr_is_fallible(fallible),
            _ => false,
        }
    }

    fn collect_lvalue_errors(&mut self, expr: &Expr, errors: &mut Vec<CompilerError>) {
        match &expr.kind {
            ExprKind::Ident(_) => {}
            ExprKind::FieldAccess(obj, _) => self.collect_lvalue_errors(obj, errors),
            ExprKind::Index(obj, idx) => {
                self.collect_lvalue_errors(obj, errors);
                self.collect_expr_errors(idx, errors);
            }
            ExprKind::Tuple(items) => {
                for (_, e) in items {
                    self.collect_lvalue_errors(e, errors);
                }
            }
            _ => self.collect_expr_errors(expr, errors),
        }
    }

    pub(crate) fn collect_expr_errors(&mut self, expr: &Expr, errors: &mut Vec<CompilerError>) {
        match &expr.kind {
            ExprKind::Binary(lhs, op, rhs) => {
                if let Err(e) = self.check_binary_op(lhs, *op, rhs) {
                    errors.push(e);
                }
                self.collect_expr_errors(lhs, errors);
                self.collect_expr_errors(rhs, errors);
            }
            ExprKind::When(w) => {
                let arms = self.when_arms(w);
                if !arms.is_empty() {
                    if let Err(e) = self.check_when_arms(arms) {
                        errors.push(e);
                    }
                    if let Err(msg) = self.registry.check_when_exhaustive(arms) {
                        errors.push(CompilerError::new(msg).with_span(self.current_span));
                    }
                    for arm in arms {
                        // Add pattern-bound variables to type_env before checking body
                        let mut saved_pattern_vars: Vec<(String, Option<Type>)> = Vec::new();
                        fn collect_vars(p: &Pattern, out: &mut Vec<String>) {
                            match p {
                                Pattern::Variable(name) => out.push(name.clone()),
                                Pattern::Constructor {
                                    args, named_fields, ..
                                } => {
                                    for a in args {
                                        collect_vars(a, out);
                                    }
                                    for (_, p) in named_fields {
                                        collect_vars(p, out);
                                    }
                                }
                                Pattern::Or(patterns) => {
                                    for p in patterns {
                                        collect_vars(p, out);
                                    }
                                }
                                Pattern::Tuple(patterns) => {
                                    for p in patterns {
                                        collect_vars(p, out);
                                    }
                                }
                                _ => {}
                            }
                        }
                        let mut pattern_vars = Vec::new();
                        collect_vars(&arm.pattern, &mut pattern_vars);
                        for pv in &pattern_vars {
                            let old = self.type_env.insert(pv.clone(), Type::Named("Int".into()));
                            saved_pattern_vars.push((pv.clone(), old));
                        }

                        self.collect_expr_errors(&arm.body, errors);
                        // Restore pattern variable bindings
                        for (name, old) in saved_pattern_vars {
                            if let Some(old_val) = old {
                                self.type_env.insert(name, old_val);
                            } else {
                                self.type_env.remove(&name);
                            }
                        }
                    }
                }
                if let WhenKind::OneLine {
                    condition,
                    then_expr,
                    else_expr,
                } = &w.kind
                {
                    self.collect_expr_errors(condition, errors);
                    self.collect_expr_errors(then_expr, errors);
                    self.collect_expr_errors(else_expr, errors);
                }
            }
            ExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => {
                self.check_fallible_call_e001(func, errors);
                if let Err(e) = self.check_call(func, args) {
                    errors.push(e);
                }
                // Only recurse into the function expression if it's not a simple
                // identifier — simple idents in call position are function names
                // (builtins like `println`, user-defined functions, etc.) and
                // should not be checked as variable references.
                if !matches!(&func.as_ref().kind, ExprKind::Ident(_)) {
                    self.collect_expr_errors(func, errors);
                }
                for a in args {
                    self.collect_expr_errors(a, errors);
                }
                if let Some(lam) = trailing_lambda {
                    self.collect_expr_errors(lam, errors);
                }
            }
            ExprKind::Block(stmts) => {
                for s in stmts {
                    self.collect_stmt_errors(s, errors);
                }
            }
            ExprKind::For(for_expr) => match &for_expr.kind {
                ForKind::Iterate {
                    var: variable,
                    iterable,
                    body,
                    ..
                } => {
                    self.collect_expr_errors(iterable, errors);
                    // Skip iterable type validation - Range is correctly inferred
                    // as Int but IS iterable, and other collection types are handled
                    // by the codegen.
                    // Add loop variable to type_env
                    let old_var = self
                        .type_env
                        .insert(variable.clone(), Type::Named("Int".into()));
                    self.collect_expr_errors(body, errors);
                    if let Some(old_val) = old_var {
                        self.type_env.insert(variable.clone(), old_val);
                    } else {
                        self.type_env.remove(variable);
                    }
                }
                ForKind::IterateWithIndex {
                    vars,
                    iterable,
                    body,
                    ..
                } => {
                    self.collect_expr_errors(iterable, errors);
                    // Skip iterable type validation - Range is correctly inferred
                    // as Int but IS iterable, and other collection types are handled
                    // by the codegen.
                    // Add loop variables to type_env
                    let mut saved_vars: Vec<(String, Option<Type>)> = Vec::new();
                    for v in vars {
                        let old = self.type_env.insert(v.clone(), Type::Named("Int".into()));
                        saved_vars.push((v.clone(), old));
                    }
                    self.collect_expr_errors(body, errors);
                    for (name, old) in saved_vars {
                        if let Some(old_val) = old {
                            self.type_env.insert(name, old_val);
                        } else {
                            self.type_env.remove(&name);
                        }
                    }
                }
                ForKind::Condition {
                    condition, body, ..
                } => {
                    self.collect_expr_errors(condition, errors);
                    self.collect_expr_errors(body, errors);
                }
                ForKind::Infinite { body, .. } => {
                    self.collect_expr_errors(body, errors);
                }
                ForKind::NestedIterate { bindings, body, .. } => {
                    for (_, e) in bindings {
                        self.collect_expr_errors(e, errors);
                    }
                    // Add nested iterate variables to type_env
                    let mut saved_nested: Vec<(String, Option<Type>)> = Vec::new();
                    for (var_name, _) in bindings {
                        let old = self
                            .type_env
                            .insert(var_name.clone(), Type::Named("Int".into()));
                        saved_nested.push((var_name.clone(), old));
                    }
                    self.collect_expr_errors(body, errors);
                    for (name, old) in saved_nested {
                        if let Some(old_val) = old {
                            self.type_env.insert(name, old_val);
                        } else {
                            self.type_env.remove(&name);
                        }
                    }
                }
            },
            ExprKind::Lambda {
                params,
                body,
                implicit_it,
            } => {
                // Add lambda parameters to type environment so the body
                // can reference them without triggering undefined variable errors
                let mut saved_params: Vec<(String, Option<Type>)> = Vec::new();
                for param_name in params {
                    let param_ty = Type::Named("Int".into());
                    let old = self.type_env.insert(param_name.clone(), param_ty);
                    saved_params.push((param_name.clone(), old));
                }
                if *implicit_it {
                    let old = self
                        .type_env
                        .insert("it".to_string(), Type::Named("Int".into()));
                    saved_params.push(("it".to_string(), old));
                }
                self.collect_expr_errors(body, errors);
                // Restore previous bindings
                for (name, old) in saved_params {
                    if let Some(old_val) = old {
                        self.type_env.insert(name, old_val);
                    } else {
                        self.type_env.remove(&name);
                    }
                }
            }
            ExprKind::FieldAccess(obj, _) => {
                self.collect_expr_errors(obj, errors);
            }
            ExprKind::Copy(inner) => {
                self.collect_expr_errors(inner, errors);
            }
            ExprKind::Unsafe(inner) => {
                self.collect_expr_errors(inner, errors);
            }
            ExprKind::OrBlock { fallible, fallback } => {
                let lhs_ty = self.try_infer_expr_type(fallible);
                let fb_ty = self.try_infer_expr_type(fallback);
                if !self.expr_is_fallible(fallible) {
                    if let Some(err) = self.fallibility.check_r7_or_unnecessary(self.current_span) {
                        errors.push(err);
                    }
                }
                if let Some(err) = self.fallibility.check_r2_or_block_result_type(
                    &lhs_ty,
                    &fb_ty,
                    self.current_span,
                ) {
                    errors.push(err);
                }
                let saved = self.fallibility.in_or_block;
                self.fallibility.in_or_block = true;
                self.collect_expr_errors(fallible, errors);
                self.fallibility.in_or_block = saved;
                self.collect_expr_errors(fallback, errors);
            }
            ExprKind::Unary(_, inner) => {
                self.collect_expr_errors(inner, errors);
            }
            ExprKind::Index(obj, idx) => {
                self.collect_expr_errors(obj, errors);
                self.collect_expr_errors(idx, errors);
                match self.collection_index_receiver_kind(obj) {
                    Some(CollectionIndexKind::List) => {
                        if let Some(err) = self
                            .fallibility
                            .check_r6_fallible_index_needs_or(self.current_span)
                        {
                            errors.push(err);
                        }
                    }
                    Some(CollectionIndexKind::Map) => {
                        if let Some(err) = self
                            .fallibility
                            .check_r8_map_index_needs_or(self.current_span)
                        {
                            errors.push(err);
                        }
                    }
                    Some(CollectionIndexKind::Set) => {
                        if let Some(err) = self
                            .fallibility
                            .check_r9_set_index_needs_or(self.current_span)
                        {
                            errors.push(err);
                        }
                    }
                    None => {}
                }
            }
            ExprKind::Assign { target, value, .. } => {
                self.collect_lvalue_errors(target, errors);
                self.collect_expr_errors(value, errors);
                // Check if target is an immutable variable
                if let ExprKind::Ident(name) = &target.as_ref().kind {
                    if self.type_env.contains_key(name) && !self.mutable_vars.contains(name) {
                        errors.push(
                            CompilerError::new(format!(
                                "Cannot assign to immutable variable '{}'",
                                name
                            ))
                            .with_span(self.current_span),
                        );
                    }
                }
            }
            ExprKind::Tuple(elements) => {
                for (_, e) in elements {
                    self.collect_expr_errors(e, errors);
                }
            }
            ExprKind::Range(start, end) => {
                self.collect_expr_errors(start, errors);
                self.collect_expr_errors(end, errors);
            }
            ExprKind::StructLiteral(fields) => {
                for (_, v) in fields {
                    self.collect_expr_errors(v, errors);
                }
            }
            ExprKind::MapLiteral(entries) => {
                for (k, v) in entries {
                    self.collect_expr_errors(k, errors);
                    self.collect_expr_errors(v, errors);
                }
            }
            ExprKind::SetLiteral(elements) => {
                for e in elements {
                    self.collect_expr_errors(e, errors);
                }
            }
            ExprKind::StringInterpolate(parts) => {
                for part in parts {
                    if let StringPart::Expr(e) = part {
                        self.collect_expr_errors(e, errors);
                    }
                }
            }
            ExprKind::Ident(name) => {
                // Check if the variable is defined in the type environment
                // (except for enum variants which are handled by registry)
                if !self.type_env.contains_key(name) && self.registry.lookup_variant(name).is_none()
                {
                    errors.push(
                        CompilerError::new(format!("Undefined variable '{}'", name))
                            .with_span(self.current_span),
                    );
                }
            }
            _ => {} // Literal, Continue, Break, etc.
        }
    }
    pub(crate) fn collect_stmt_errors(&mut self, stmt: &Stmt, errors: &mut Vec<CompilerError>) {
        match stmt {
            Stmt::Expr { expr, .. } => self.collect_expr_errors(expr, errors),
            Stmt::Let {
                mutable,
                name,
                type_ann,
                value,
                ..
            } => {
                self.collect_expr_errors(value, errors);
                let inferred = self
                    .infer_expr_type(value)
                    .unwrap_or(Type::Named("Int".into()));
                let ty = type_ann.clone().unwrap_or(inferred);
                self.type_env.insert(name.clone(), ty);
                if *mutable {
                    self.mutable_vars.insert(name.clone());
                }
            }
            Stmt::Const {
                name,
                type_ann,
                value,
                ..
            } => {
                self.collect_expr_errors(value, errors);
                let inferred = self
                    .infer_expr_type(value)
                    .unwrap_or(Type::Named("Int".into()));
                let ty = type_ann.clone().unwrap_or(inferred);
                self.type_env.insert(name.clone(), ty);
            }
            Stmt::Destructure { names, value, .. } => {
                self.collect_expr_errors(value, errors);
                // Add destructured variable names to type_env so subsequent
                // statements can reference them without "undefined variable" errors
                for name in names {
                    self.type_env
                        .insert(name.clone(), Type::Named("Int".into()));
                }
            }
            Stmt::Return { value: expr, .. } => {
                if let Some(e) = expr {
                    self.collect_expr_errors(e, errors);
                }
            }
            _ => {}
        }
    }

    pub(crate) fn check_fallible_call_e001(
        &mut self,
        func: &Expr,
        errors: &mut Vec<CompilerError>,
    ) {
        if let Some(err) = self
            .fallibility
            .check_r1_fallible_call(self.current_span, func)
        {
            errors.push(err);
        }
    }

    pub(crate) fn check_binary_op(
        &self,
        lhs: &Expr,
        op: BinaryOp,
        rhs: &Expr,
    ) -> Result<(), CompilerError> {
        let lt = self.infer_expr_type(lhs)?;
        let rt = self.infer_expr_type(rhs)?;

        match op {
            BinaryOp::Add => {
                let ls = format!("{}", lt);
                let rs = format!("{}", rt);
                if ls == "String" || rs == "String" {
                    return Ok(());
                }
            }
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod | BinaryOp::Pow => {
                let ls = format!("{}", lt);
                let rs = format!("{}", rt);
                if ls == "String" || rs == "String" || ls == "Bool" || rs == "Bool" {
                    return Err(CompilerError::new(format!(
                        "Arithmetic operation '{}' not supported for {}",
                        op,
                        if ls == "Bool" || rs == "Bool" {
                            "Bool"
                        } else {
                            "String"
                        }
                    ))
                    .with_span(self.current_span));
                }
            }
            BinaryOp::Eq | BinaryOp::Neq => {
                return Ok(());
            }
            BinaryOp::Lt | BinaryOp::Gt | BinaryOp::Lte | BinaryOp::Gte => {
                let ls = format!("{}", lt);
                let rs = format!("{}", rt);
                // Allow Bool comparison (True > False), but disallow mixed Bool/other types
                if (ls == "Bool" || rs == "Bool") && ls != rs {
                    return Err(CompilerError::new(format!(
                        "Cannot compare '{}' with '{}'",
                        ls, rs
                    ))
                    .with_span(self.current_span));
                }
            }
            BinaryOp::And | BinaryOp::Or => {
                if format!("{}", lt) != "Bool" || format!("{}", rt) != "Bool" {
                    return Err(CompilerError::new(format!(
                        "Logical operator '{}' requires Bool operands, got '{}' and '{}'",
                        op, lt, rt
                    ))
                    .with_span(self.current_span));
                }
            }
            BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr => {
                let ls = format!("{}", lt);
                let rs = format!("{}", rt);
                if ls != "Int" || rs != "Int" {
                    return Err(CompilerError::new(format!(
                        "Bitwise operator '{}' requires Int operands, got '{}' and '{}'",
                        op, lt, rs
                    ))
                    .with_span(self.current_span));
                }
            }
            BinaryOp::Range
            | BinaryOp::RangeExclusive
            | BinaryOp::Assign
            | BinaryOp::In
            | BinaryOp::Is => {}
        }
        Ok(())
    }

    pub(crate) fn check_call(&self, func: &Expr, args: &[Expr]) -> Result<(), CompilerError> {
        if let ExprKind::Ident(name) = &func.kind {
            if let Some((_ei, vi)) = self.registry.lookup_variant(name) {
                let expected = vi.params.len();
                let actual = args.len();
                if expected != actual {
                    return Err(CompilerError::new(format!(
                        "Enum variant '{}' expects {} arguments, but got {}",
                        name, expected, actual
                    ))
                    .with_span(self.current_span));
                }
            }
            // Check generic function via type inference
            if let Some(generic_stmt) = self.generic_funs.get(name) {
                if let Stmt::Fun {
                    params,
                    type_params,
                    ..
                } = generic_stmt
                {
                    if !type_params.is_empty() {
                        let param_tys: Vec<Type> = params
                            .iter()
                            .map(|p| p.ty.clone().unwrap_or(Type::Named("Int".into())))
                            .collect();
                        if args.len() != param_tys.len() {
                            return Err(CompilerError::new(format!(
                                "Function '{}' expects {} arguments, but got {}",
                                name,
                                param_tys.len(),
                                args.len()
                            ))
                            .with_span(self.current_span));
                        }
                        // Collect arg types, skipping lambdas
                        let mut arg_tys = Vec::new();
                        let mut filtered_params = Vec::new();
                        for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
                            if matches!(&arg.kind, ExprKind::Lambda { .. }) {
                                continue;
                            }
                            arg_tys.push(self.try_infer_expr_type(arg));
                            filtered_params.push(param_ty.clone());
                        }
                        if !filtered_params.is_empty() {
                            if let Err(msg) = infer_type_args(&filtered_params, &arg_tys) {
                                return Err(CompilerError::new(format!(
                                    "Cannot infer type arguments for '{}': {}",
                                    name, msg
                                ))
                                .with_span(self.current_span));
                            }
                        }
                        return Ok(());
                    }
                }
            }
            // Check function argument types
            if let Some(fn_type) = self.type_env.get(name) {
                match fn_type {
                    Type::Function(param_tys, _ret_ty) => {
                        if args.len() != param_tys.len() {
                            return Err(CompilerError::new(format!(
                                "Function '{}' expects {} arguments, but got {}",
                                name,
                                param_tys.len(),
                                args.len()
                            ))
                            .with_span(self.current_span));
                        }
                        for (i, (arg, param_ty)) in args.iter().zip(param_tys.iter()).enumerate() {
                            // Skip lambdas — infer_expr_type returns body type, not function type
                            if matches!(&arg.kind, ExprKind::Lambda { .. }) {
                                continue;
                            }
                            let arg_ty = self.infer_expr_type(arg)?;
                            if !types_compatible(param_ty, &arg_ty) {
                                return Err(CompilerError::new(format!(
                                    "Argument {} to '{}' expects '{}' but got '{}'",
                                    i + 1,
                                    name,
                                    param_ty,
                                    arg_ty
                                ))
                                .with_span(self.current_span));
                            }
                        }
                    }
                    _ => {
                        // Variable has a non-function type in type_env.
                        // This can happen legitimately when a let-binding shadows
                        // a function name (type_env is mutable). The codegen will
                        // handle the actual resolution.
                    }
                }
            } else {
                // Overloaded function: resolve mangled name from argument types
                let arg_tys: Result<Vec<Type>, CompilerError> = args
                    .iter()
                    .filter(|a| !matches!(&a.kind, ExprKind::Lambda { .. }))
                    .map(|a| self.infer_expr_type(a))
                    .collect();
                if let Ok(arg_tys) = arg_tys {
                    let mangled = mangle_name(name, &arg_tys);
                    if let Some(Type::Function(param_tys, _ret_ty)) = self.type_env.get(&mangled) {
                        if args.len() != param_tys.len() {
                            return Err(CompilerError::new(format!(
                                "Function '{}' expects {} arguments, but got {}",
                                name,
                                param_tys.len(),
                                args.len()
                            ))
                            .with_span(self.current_span));
                        }
                        for (i, (arg, param_ty)) in args.iter().zip(param_tys.iter()).enumerate() {
                            if matches!(&arg.kind, ExprKind::Lambda { .. }) {
                                continue;
                            }
                            let arg_ty = self.infer_expr_type(arg)?;
                            if !types_compatible(param_ty, &arg_ty) {
                                return Err(CompilerError::new(format!(
                                    "Argument {} to '{}' expects '{}' but got '{}'",
                                    i + 1,
                                    name,
                                    param_ty,
                                    arg_ty
                                ))
                                .with_span(self.current_span));
                            }
                        }
                    }
                }
            }
        } else if let ExprKind::FieldAccess(receiver, method) = &func.kind {
            let recv_type = self.infer_expr_type(receiver)?;
            let type_name = match recv_type {
                Type::Named(n) => n,
                Type::Map(_, _) => "Map".to_string(),
                Type::Set(_) => "Set".to_string(),
                Type::Generic(base, _) => format!("{}", base),
                other => format!("{}", other),
            };
            let lookup_key = format!("{}.{}", type_name, method);
            if let Some(Type::Function(param_tys, _ret_ty)) = self.type_env.get(&lookup_key) {
                if args.len() != param_tys.len() {
                    return Err(CompilerError::new(format!(
                        "Method '{}.{}' expects {} arguments, but got {}",
                        type_name,
                        method,
                        param_tys.len(),
                        args.len()
                    ))
                    .with_span(self.current_span));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn check_when_arms(&self, arms: &[WhenArm]) -> Result<(), CompilerError> {
        if arms.is_empty() {
            return Ok(());
        }

        // Collect arm types (with pattern-local bindings for inference)
        let types: Vec<Type> = arms
            .iter()
            .map(|a| {
                let locals = self.pattern_local_types(&a.pattern);
                self.infer_expr_type_with_locals(&a.body, &locals)
            })
            .collect::<Result<Vec<Type>, _>>()?;
        let first = &types[0];

        // Only skip checking when ALL arms are Int (un-inferred fallback)
        let all_int = types
            .iter()
            .all(|t| matches!(t, Type::Named(ref n) if n == "Int"));
        if all_int {
            return Ok(());
        }

        for (i, t) in types.iter().enumerate().skip(1) {
            if !types_compatible(first, t) {
                return Err(CompilerError::new(format!(
                    "When arm type mismatch: arm 1 is '{}' but arm {} is '{}'",
                    first,
                    i + 1,
                    t
                ))
                .with_span(self.current_span));
            }
        }
        Ok(())
    }
}
