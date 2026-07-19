use super::*;
use crate::fallibility_narrowing::{
    call_is_proven_safe, index_access_is_proven_safe, NarrowingContext,
};
use crate::types::{infer_type_args, mangle_name, types_compatible};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CollectionIndexKind {
    List,
    Map,
    Set,
}

/// M110: root Ident of an lvalue chain (`x`, `p.x`, `xs[0]`, `p.x[i]`).
fn lvalue_root_ident(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Ident(name) => Some(name.as_str()),
        ExprKind::FieldAccess(recv, _) => lvalue_root_ident(recv),
        ExprKind::Index(recv, _) => lvalue_root_ident(recv),
        _ => None,
    }
}

impl TypeChecker {
    /// `break` / `continue` (or a block whose last stmt is one) never produce a value.
    fn expr_is_diverging_control(expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Break | ExprKind::Continue => true,
            ExprKind::Block(stmts) => match stmts.last() {
                Some(Stmt::Break { .. } | Stmt::Continue { .. }) => true,
                Some(Stmt::Expr { expr, .. }) => Self::expr_is_diverging_control(expr),
                _ => false,
            },
            _ => false,
        }
    }

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

    /// M87: List/Set literal elements (or Map values) must share a compatible type.
    fn collection_elems_homogeneous_error(
        &self,
        kind: &str,
        elems: &[Expr],
    ) -> Option<CompilerError> {
        if elems.len() < 2 {
            return None;
        }
        let first = self.try_infer_expr_type(&elems[0]);
        for (i, e) in elems.iter().enumerate().skip(1) {
            let got = self.try_infer_expr_type(e);
            if !types_compatible(&first, &got) {
                return Some(
                    CompilerError::new(format!(
                        "{kind} element {} has type '{got}' but element 1 has '{first}'",
                        i + 1
                    ))
                    .with_span(e.span),
                );
            }
        }
        None
    }

    /// M91: List/LazyList index key must be Int; Map key must be String.
    fn check_index_key_type(&self, obj: &Expr, idx: &Expr, errors: &mut Vec<CompilerError>) {
        let obj_ty = self.try_infer_expr_type(obj);
        let idx_ty = self.try_infer_expr_type(idx);
        let expected = match &obj_ty {
            Type::Map(_, _) => Some(Type::Named("String".into())),
            Type::Named(n) if n == "Map" => Some(Type::Named("String".into())),
            Type::Generic(base, _) if matches!(base.as_ref(), Type::Named(n) if n == "List") => {
                Some(Type::Named("Int".into()))
            }
            Type::LazyList(_) => Some(Type::Named("Int".into())),
            Type::Named(n) if n == "List" => Some(Type::Named("Int".into())),
            // M113: String[i] key must be Int (bootstrap tyCheckIndexKey recvTag==5).
            Type::Named(n) if n == "String" => Some(Type::Named("Int".into())),
            _ => None,
        };
        let Some(expected) = expected else {
            return;
        };
        if !types_compatible(&expected, &idx_ty) {
            errors.push(
                CompilerError::new(format!("Index key expects '{expected}' but got '{idx_ty}'"))
                    .with_span(idx.span),
            );
        }
    }

    fn index_access_is_proven_safe(&self, obj: &Expr, idx: &Expr) -> bool {
        index_access_is_proven_safe(obj, idx, &self.narrowing)
    }

    fn call_is_proven_safe(&self, func: &Expr, args: &[Expr]) -> bool {
        call_is_proven_safe(func, args, &self.narrowing)
    }

    fn expr_contains_fallible(&self, expr: &Expr) -> bool {
        match &expr.kind {
            ExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => {
                // Same tree walk as collect_expr_errors: callee *or* any arg / trailing
                // body may be fallible (so `print(lst[i]) or { d }` satisfies R7).
                (self.fallibility.call_expr_is_fallible(func)
                    && !self.call_is_proven_safe(func, args))
                    || args.iter().any(|a| self.expr_contains_fallible(a))
                    || trailing_lambda
                        .as_ref()
                        .is_some_and(|lam| self.expr_contains_fallible(lam))
            }
            ExprKind::Index(obj, idx) => {
                // Nested `a[i][j] or { d }`: OrBlock wraps the outer Index, but R6 fires on
                // the inner List index — recurse so R7 matches the collect_expr_errors walk.
                (self.collection_index_receiver_kind(obj).is_some()
                    && !self.index_access_is_proven_safe(obj, idx))
                    || self.expr_contains_fallible(obj)
                    || self.expr_contains_fallible(idx)
            }
            ExprKind::Block(stmts) => stmts.iter().any(|s| self.stmt_contains_fallible(s)),
            ExprKind::Binary(l, _, r) => {
                self.expr_contains_fallible(l) || self.expr_contains_fallible(r)
            }
            ExprKind::Unary(_, inner) => self.expr_contains_fallible(inner),
            ExprKind::When(w) => self.when_contains_fallible(w),
            ExprKind::For(f) => self.for_contains_fallible(f),
            ExprKind::OrBlock { fallible, .. } => self.expr_contains_fallible(fallible),
            ExprKind::Assign { target, value, .. } => {
                self.expr_contains_fallible(target) || self.expr_contains_fallible(value)
            }
            ExprKind::Lambda { body, .. } => self.expr_contains_fallible(body),
            ExprKind::Copy(inner) | ExprKind::Unsafe(inner) => self.expr_contains_fallible(inner),
            ExprKind::StructLiteral(fields) => {
                fields.iter().any(|(_, e)| self.expr_contains_fallible(e))
            }
            ExprKind::MapLiteral(entries) => entries
                .iter()
                .any(|(k, v)| self.expr_contains_fallible(k) || self.expr_contains_fallible(v)),
            ExprKind::SetLiteral(elems) => elems.iter().any(|e| self.expr_contains_fallible(e)),
            ExprKind::Tuple(items) => items.iter().any(|(_, e)| self.expr_contains_fallible(e)),
            ExprKind::StringInterpolate(parts) => parts.iter().any(
                |p| matches!(p, crate::ast::StringPart::Expr(e) if self.expr_contains_fallible(e)),
            ),
            _ => false,
        }
    }

    fn stmt_contains_fallible(&self, stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Let { value, .. } | Stmt::Expr { expr: value, .. } => {
                self.expr_contains_fallible(value)
            }
            Stmt::Return { value: Some(v), .. } => self.expr_contains_fallible(v),
            _ => false,
        }
    }

    fn when_contains_fallible(&self, w: &When) -> bool {
        use crate::ast::WhenKind;
        match &w.kind {
            WhenKind::OneLine {
                condition,
                then_expr,
                else_expr,
            } => {
                self.expr_contains_fallible(condition)
                    || self.expr_contains_fallible(then_expr)
                    || self.expr_contains_fallible(else_expr)
            }
            WhenKind::ValueMatch { value, arms } => {
                self.expr_contains_fallible(value)
                    || arms.iter().any(|arm| {
                        arm.guard
                            .as_ref()
                            .is_some_and(|g| self.expr_contains_fallible(g))
                            || self.expr_contains_fallible(&arm.body)
                    })
            }
            WhenKind::ConditionChain { arms } => arms.iter().any(|arm| {
                arm.guard
                    .as_ref()
                    .is_some_and(|g| self.expr_contains_fallible(g))
                    || self.expr_contains_fallible(&arm.body)
            }),
        }
    }

    fn expr_uses_len_on_var(&self, expr: &Expr, var: &str) -> bool {
        match &expr.kind {
            ExprKind::Call { func, args, .. } => {
                if matches!(&func.kind, ExprKind::Ident(n) if n == "len") {
                    args.first()
                        .is_some_and(|a| matches!(&a.kind, ExprKind::Ident(n) if n == var))
                } else {
                    args.iter().any(|a| self.expr_uses_len_on_var(a, var))
                        || self.expr_uses_len_on_var(func, var)
                }
            }
            ExprKind::FieldAccess(obj, method) if method == "len" => {
                matches!(&obj.kind, ExprKind::Ident(n) if n == var)
            }
            ExprKind::Block(stmts) => stmts.iter().any(|s| self.stmt_uses_len_on_var(s, var)),
            ExprKind::Binary(l, _, r)
            | ExprKind::Assign {
                target: l,
                value: r,
                ..
            } => self.expr_uses_len_on_var(l, var) || self.expr_uses_len_on_var(r, var),
            ExprKind::Unary(_, inner) => self.expr_uses_len_on_var(inner, var),
            ExprKind::When(w) => self.when_uses_len_on_var(w, var),
            ExprKind::For(f) => self.for_uses_len_on_var(f, var),
            ExprKind::OrBlock { fallible, fallback } => {
                self.expr_uses_len_on_var(fallible, var) || self.expr_uses_len_on_var(fallback, var)
            }
            ExprKind::Lambda { body, .. } => self.expr_uses_len_on_var(body, var),
            ExprKind::Copy(inner) | ExprKind::Unsafe(inner) => {
                self.expr_uses_len_on_var(inner, var)
            }
            ExprKind::StructLiteral(fields) => fields
                .iter()
                .any(|(_, e)| self.expr_uses_len_on_var(e, var)),
            ExprKind::MapLiteral(entries) => entries.iter().any(|(k, v)| {
                self.expr_uses_len_on_var(k, var) || self.expr_uses_len_on_var(v, var)
            }),
            ExprKind::SetLiteral(items) => items.iter().any(|e| self.expr_uses_len_on_var(e, var)),
            ExprKind::Tuple(items) => items.iter().any(|(_, e)| self.expr_uses_len_on_var(e, var)),
            ExprKind::Index(obj, idx) => {
                self.expr_uses_len_on_var(obj, var) || self.expr_uses_len_on_var(idx, var)
            }
            ExprKind::FieldAccess(obj, _) => self.expr_uses_len_on_var(obj, var),
            ExprKind::Ident(_) | ExprKind::Literal(_) => false,
            _ => false,
        }
    }

    fn stmt_uses_len_on_var(&self, stmt: &Stmt, var: &str) -> bool {
        match stmt {
            Stmt::Expr { expr, .. } => self.expr_uses_len_on_var(expr, var),
            Stmt::Let { value, .. } | Stmt::Const { value, .. } => {
                self.expr_uses_len_on_var(value, var)
            }
            Stmt::Return { value: Some(v), .. } => self.expr_uses_len_on_var(v, var),
            Stmt::Return { value: None, .. } => false,
            Stmt::Destructure { value, .. } => self.expr_uses_len_on_var(value, var),
            Stmt::Fun { body, .. } => self.expr_uses_len_on_var(body, var),
            _ => false,
        }
    }

    fn when_uses_len_on_var(&self, w: &When, var: &str) -> bool {
        use crate::ast::WhenKind;
        match &w.kind {
            WhenKind::OneLine {
                then_expr,
                else_expr,
                ..
            } => {
                self.expr_uses_len_on_var(then_expr, var)
                    || self.expr_uses_len_on_var(else_expr, var)
            }
            WhenKind::ValueMatch { value, arms } => {
                self.expr_uses_len_on_var(value, var)
                    || arms.iter().any(|arm| {
                        arm.guard
                            .as_ref()
                            .is_some_and(|g| self.expr_uses_len_on_var(g, var))
                            || self.expr_uses_len_on_var(&arm.body, var)
                    })
            }
            WhenKind::ConditionChain { arms } => arms.iter().any(|arm| {
                arm.guard
                    .as_ref()
                    .is_some_and(|g| self.expr_uses_len_on_var(g, var))
                    || self.expr_uses_len_on_var(&arm.body, var)
            }),
        }
    }

    fn for_uses_len_on_var(&self, f: &For, var: &str) -> bool {
        use crate::ast::ForKind;
        match &f.kind {
            ForKind::Iterate { iterable, body, .. } => {
                self.expr_uses_len_on_var(iterable, var) || self.expr_uses_len_on_var(body, var)
            }
            ForKind::IterateWithIndex { iterable, body, .. } => {
                self.expr_uses_len_on_var(iterable, var) || self.expr_uses_len_on_var(body, var)
            }
            ForKind::Condition { condition, body } => {
                self.expr_uses_len_on_var(condition, var) || self.expr_uses_len_on_var(body, var)
            }
            ForKind::Infinite { body } => self.expr_uses_len_on_var(body, var),
            ForKind::NestedIterate { bindings, body, .. } => {
                bindings
                    .iter()
                    .any(|(_, e)| self.expr_uses_len_on_var(e, var))
                    || self.expr_uses_len_on_var(body, var)
            }
        }
    }

    /// Loop binding type for `for var in iterable` (Map keys vs values; others default to Int).
    fn for_iterate_loop_var_type(&self, iterable: &Expr, var: &str, body: &Expr) -> Type {
        let iter_ty = self.try_infer_expr_type(iterable);
        match &iter_ty {
            Type::Map(key_ty, val_ty) => {
                if self.expr_uses_len_on_var(body, var) {
                    (**key_ty).clone()
                } else {
                    (**val_ty).clone()
                }
            }
            Type::Named(n) if n == "Map" => {
                if self.expr_uses_len_on_var(body, var) {
                    Type::Named("String".into())
                } else {
                    Type::Named("Int".into())
                }
            }
            _ => Type::Named("Int".into()),
        }
    }

    fn for_contains_fallible(&self, f: &For) -> bool {
        use crate::ast::ForKind;
        match &f.kind {
            ForKind::Iterate { iterable, body, .. } => {
                self.expr_contains_fallible(iterable) || self.expr_contains_fallible(body)
            }
            ForKind::IterateWithIndex { iterable, body, .. } => {
                self.expr_contains_fallible(iterable) || self.expr_contains_fallible(body)
            }
            ForKind::Condition { condition, body } => {
                self.expr_contains_fallible(condition) || self.expr_contains_fallible(body)
            }
            ForKind::Infinite { body } => self.expr_contains_fallible(body),
            ForKind::NestedIterate { bindings, body, .. } => {
                bindings.iter().any(|(_, e)| self.expr_contains_fallible(e))
                    || self.expr_contains_fallible(body)
            }
        }
    }

    fn expr_is_fallible(&self, expr: &Expr) -> bool {
        self.expr_contains_fallible(expr)
    }

    fn collect_lvalue_errors(&mut self, expr: &Expr, errors: &mut Vec<CompilerError>) {
        match &expr.kind {
            ExprKind::Ident(_) => {}
            ExprKind::FieldAccess(obj, field) => {
                self.collect_lvalue_errors(obj, errors);
                // M67: assign `p.z = …` must hard-error like read-side M65 E013.
                if let Some(err) = self.unknown_struct_field_error(obj, field, expr.span) {
                    errors.push(err);
                }
            }
            ExprKind::Index(obj, idx) => {
                self.collect_lvalue_errors(obj, errors);
                self.collect_expr_errors(idx, errors);
                // M91: key type on assign lvalue (read-side checked in collect_expr_errors).
                self.check_index_key_type(obj, idx, errors);
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
                    for arm in arms {
                        self.collect_pattern_constructor_errors(
                            &arm.pattern,
                            errors,
                            self.current_span,
                        );
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

                        let saved_narrow = self.narrowing.clone();
                        if let Pattern::Expr(cond_expr) = &arm.pattern {
                            // M101: ConditionChain arm expr must be Bool.
                            self.collect_expr_errors(cond_expr, errors);
                            let cond_ty = self.try_infer_expr_type(cond_expr);
                            if format!("{cond_ty}") != "Bool" {
                                errors.push(
                                    CompilerError::new(format!(
                                        "When condition requires Bool expression, got '{cond_ty}'"
                                    ))
                                    .with_span(cond_expr.span),
                                );
                            }
                        }
                        if let Some(guard) = &arm.guard {
                            self.collect_expr_errors(guard, errors);
                            // M83: when-arm `and <guard>` must be Bool (bootstrap M79 parity).
                            let guard_ty = self.try_infer_expr_type(guard);
                            if format!("{guard_ty}") != "Bool" {
                                errors.push(
                                    CompilerError::new(format!(
                                        "When guard requires Bool expression, got '{guard_ty}'"
                                    ))
                                    .with_span(guard.span),
                                );
                            }
                            self.narrowing = self.narrowing.with_guard(guard);
                        }
                        self.collect_expr_errors(&arm.body, errors);
                        self.narrowing = saved_narrow;
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
                    // M101 / E017: if condition must be Bool (bootstrap tyCheckGuard parity).
                    let cond_ty = self.try_infer_expr_type(condition);
                    if format!("{cond_ty}") != "Bool" {
                        errors.push(crate::error::e017_if_condition_not_bool(
                            &format!("{cond_ty}"),
                            condition.span,
                        ));
                    }
                    self.collect_expr_errors(then_expr, errors);
                    self.collect_expr_errors(else_expr, errors);
                    // E018: then/else branches must share a type.
                    // `break` / `continue` are diverging (bottom): compatible with any arm type
                    // (Kotlin Nothing / for-yield filter idiom: `if c { x } else { continue }`).
                    if !Self::expr_is_diverging_control(then_expr)
                        && !Self::expr_is_diverging_control(else_expr)
                    {
                        let then_ty = self.try_infer_expr_type(then_expr);
                        let else_ty = self.try_infer_expr_type(else_expr);
                        if !types_compatible(&then_ty, &else_ty) {
                            errors.push(crate::error::e018_if_branch_type_mismatch(
                                &format!("{then_ty}"),
                                &format!("{else_ty}"),
                                then_expr.span,
                            ));
                        }
                    }
                }
            }
            ExprKind::Call {
                func,
                args,
                trailing_lambda,
            } => {
                if !self.call_is_proven_safe(func, args) {
                    self.check_fallible_call_e001(func, errors);
                }
                if let Err(e) = self.check_call(func, args) {
                    errors.push(e);
                }
                self.check_call_struct_literal_shapes(func, args, errors);
                // Only recurse into the function expression if it's not a simple
                // identifier — simple idents in call position are function names
                // (builtins like `println`, user-defined functions, etc.) and
                // should not be checked as variable references.
                // FieldAccess callees: recurse into receiver only — field name may be UFCS
                // (M62); bare unknown fields are E013 elsewhere (M65).
                match &func.kind {
                    ExprKind::Ident(_) => {}
                    ExprKind::FieldAccess(recv, _) => {
                        self.collect_expr_errors(recv, errors);
                    }
                    _ => self.collect_expr_errors(func, errors),
                }
                for a in args {
                    self.collect_expr_errors(a, errors);
                }
                if let Some(lam) = trailing_lambda {
                    self.collect_expr_errors(lam, errors);
                }
            }
            ExprKind::Block(stmts) => {
                // Branch / nested blocks must not leak `val`/`var` into the outer scope.
                let saved_env = self.type_env.clone();
                let saved_mut = self.mutable_vars.clone();
                for s in stmts {
                    self.collect_stmt_errors(s, errors);
                }
                self.type_env = saved_env;
                self.mutable_vars = saved_mut;
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
                    let loop_ty = self.for_iterate_loop_var_type(iterable, variable, body);
                    let old_var = self.type_env.insert(variable.clone(), loop_ty);
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
                    // M103: for-condition must be Bool (bootstrap tyCheckGuard parity).
                    let cond_ty = self.try_infer_expr_type(condition);
                    if format!("{cond_ty}") != "Bool" {
                        errors.push(
                            CompilerError::new(format!(
                                "For condition requires Bool expression, got '{cond_ty}'"
                            ))
                            .with_span(condition.span),
                        );
                    }
                    let saved = self.narrowing.clone();
                    self.narrowing = NarrowingContext::from_loop_condition(condition);
                    self.collect_expr_errors(body, errors);
                    self.narrowing = saved;
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
            ExprKind::FieldAccess(obj, field) => {
                self.collect_expr_errors(obj, errors);
                if let Some(err) = self.unknown_struct_field_error(obj, field, expr.span) {
                    errors.push(err);
                }
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
            ExprKind::Unary(op, inner) => {
                self.collect_expr_errors(inner, errors);
                if matches!(op, UnaryOp::Not) {
                    let inner_ty = self.try_infer_expr_type(inner);
                    if format!("{inner_ty}") != "Bool" {
                        errors.push(
                            CompilerError::new(format!(
                                "Unary operator '!' requires Bool operand, got '{inner_ty}'"
                            ))
                            .with_span(expr.span),
                        );
                    }
                }
                // M97/M108: Neg/Pos reject Bool/String (bootstrap tyCheckNumericUnary parity).
                if matches!(op, UnaryOp::Neg | UnaryOp::Pos) {
                    let inner_ty = self.try_infer_expr_type(inner);
                    let ty_s = format!("{inner_ty}");
                    if ty_s == "Bool" || ty_s == "String" {
                        errors.push(
                            CompilerError::new(format!(
                                "Unary operator '{op}' not supported for {ty_s}"
                            ))
                            .with_span(expr.span),
                        );
                    }
                }
            }
            ExprKind::Index(obj, idx) => {
                self.collect_expr_errors(obj, errors);
                self.collect_expr_errors(idx, errors);
                // M91: List index must be Int; Map key must be String.
                self.check_index_key_type(obj, idx, errors);
                // M64: tuple/struct slots are compile-time fields — hard E005, never E006.
                if let Type::Struct(fields) = self.try_infer_expr_type(obj) {
                    let span = expr.span;
                    match &idx.kind {
                        ExprKind::Literal(Literal::Int(n)) => {
                            let n = *n;
                            if n < 0 || (n as usize) >= fields.len() {
                                errors.push(crate::error::e005_struct_index_invalid(
                                    format!(
                                        "tuple/struct index {n} is out of range for {} field(s)",
                                        fields.len()
                                    ),
                                    span,
                                ));
                            }
                        }
                        _ => {
                            errors.push(crate::error::e005_struct_index_invalid(
                                "tuple/struct index must be an integer literal",
                                span,
                            ));
                        }
                    }
                }
                if !self.index_access_is_proven_safe(obj, idx) {
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
            }
            ExprKind::Assign { target, value, .. } => {
                self.collect_lvalue_errors(target, errors);
                self.collect_expr_errors(value, errors);
                let target_ty = self.try_infer_expr_type(target);
                self.check_expr_struct_literal_against_expected(&target_ty, value, errors);
                // M85: assign rhs must match target type.
                let value_ty = self.try_infer_expr_type(value);
                if !types_compatible(&target_ty, &value_ty) {
                    errors.push(
                        CompilerError::new(format!(
                            "Cannot assign '{value_ty}' to variable of type '{target_ty}'"
                        ))
                        .with_span(value.span),
                    );
                }
                // M109/M110: Ident / FieldAccess / Index assign requires mutable root.
                if let Some(name) = lvalue_root_ident(target) {
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
                // M99: same Int-bound rule as Binary Range / RangeExclusive.
                let ls = format!("{}", self.try_infer_expr_type(start));
                let rs = format!("{}", self.try_infer_expr_type(end));
                if ls != "Int" || rs != "Int" {
                    errors.push(
                        CompilerError::new(format!(
                            "Range bounds must be Int, got '{ls}' and '{rs}'"
                        ))
                        .with_span(expr.span),
                    );
                }
            }
            ExprKind::StructLiteral(fields) => {
                for (_, v) in fields {
                    self.collect_expr_errors(v, errors);
                }
                // M71: unique name-set match → same E013/E015/E016 as annotated/expected Named.
                let names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                if let Some(info) = self.registry.find_struct_by_fields(&names) {
                    let struct_name = info.name.clone();
                    self.check_struct_literal_against_named(
                        &struct_name,
                        fields,
                        expr.span,
                        errors,
                    );
                }
            }
            ExprKind::MapLiteral(entries) => {
                for (k, v) in entries {
                    self.collect_expr_errors(k, errors);
                    self.collect_expr_errors(v, errors);
                }
                // M87: Map values must be homogeneous.
                if entries.len() > 1 {
                    let first = self.try_infer_expr_type(&entries[0].1);
                    for (i, (_, v)) in entries.iter().enumerate().skip(1) {
                        let got = self.try_infer_expr_type(v);
                        if !types_compatible(&first, &got) {
                            errors.push(
                                CompilerError::new(format!(
                                    "Map entry {} has value type '{got}' but entry 1 has '{first}'",
                                    i + 1
                                ))
                                .with_span(v.span),
                            );
                        }
                    }
                }
            }
            ExprKind::SetLiteral(elements) => {
                for e in elements {
                    self.collect_expr_errors(e, errors);
                }
                // M87: Set elements must be homogeneous.
                if let Some(err) = self.collection_elems_homogeneous_error("Set", elements) {
                    errors.push(err);
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
                if let Some(ann) = type_ann {
                    if let (Type::Named(struct_name), ExprKind::StructLiteral(fields)) =
                        (ann, &value.kind)
                    {
                        self.check_struct_literal_against_named(
                            struct_name,
                            fields,
                            value.span,
                            errors,
                        );
                    }
                    // M85: in-function let ann↔rhs (parity with top-level check()).
                    let inferred = self
                        .infer_expr_type(value)
                        .unwrap_or(Type::Named("Int".into()));
                    if !types_compatible(ann, &inferred) {
                        errors.push(
                            CompilerError::new(format!(
                                "Variable '{name}' declared as '{ann}' but initialized with '{inferred}'"
                            ))
                            .with_span(value.span),
                        );
                    }
                }
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
                if let Some(ann) = type_ann {
                    if let (Type::Named(struct_name), ExprKind::StructLiteral(fields)) =
                        (ann, &value.kind)
                    {
                        self.check_struct_literal_against_named(
                            struct_name,
                            fields,
                            value.span,
                            errors,
                        );
                    }
                }
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
                    if let Some(ret) = self.current_return_type.clone() {
                        self.check_expr_struct_literal_against_expected(&ret, e, errors);
                    }
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
                // M106: Add rejects Bool (bootstrap tyCheckArith parity); String concat still ok.
                if ls == "Bool" || rs == "Bool" {
                    return Err(CompilerError::new(format!(
                        "Arithmetic operation '+' not supported for Bool"
                    ))
                    .with_span(self.current_span));
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
            BinaryOp::Range | BinaryOp::RangeExclusive => {
                let ls = format!("{}", lt);
                let rs = format!("{}", rt);
                // M99: range endpoints must be Int (bootstrap tyCheckRange parity).
                if ls != "Int" || rs != "Int" {
                    return Err(CompilerError::new(format!(
                        "Range bounds must be Int, got '{}' and '{}'",
                        lt, rt
                    ))
                    .with_span(self.current_span));
                }
            }
            BinaryOp::Assign | BinaryOp::In | BinaryOp::Is => {}
        }
        Ok(())
    }

    pub(crate) fn check_call(&self, func: &Expr, args: &[Expr]) -> Result<(), CompilerError> {
        if let ExprKind::Ident(name) = &func.kind {
            // Intrinsic constructors / coroutine launches are not registry builtins.
            match name.as_str() {
                "List" | "__list" | "Set" => {
                    // M87: collection literal elements must be homogeneous.
                    if let Some(err) = self.collection_elems_homogeneous_error(name, args) {
                        return Err(err);
                    }
                    return Ok(());
                }
                "Map" | "Stream" | "launch" => {
                    return Ok(());
                }
                // Host hook: defined by importer modules; prelude fixtures typecheck alone.
                "keywordKindOpsTail" => {
                    return Ok(());
                }
                _ => {}
            }
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
                return Ok(());
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
                        return Ok(());
                    }
                    if self.overloaded_names.contains(name) {
                        return Err(CompilerError::new(format!(
                            "No matching overload of '{name}' for argument types: {arg_tys:?}"
                        ))
                        .with_span(self.current_span));
                    }
                }
                if crate::builtin::lookup(name).is_some() {
                    return Ok(());
                }
                return Err(crate::error::e004_unknown_call(name, self.current_span));
            }
        } else if let ExprKind::FieldAccess(receiver, method) = &func.kind {
            let recv_type = self.infer_expr_type(receiver)?;
            let type_name = match &recv_type {
                Type::Named(n) => n.clone(),
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
                return Ok(());
            }
            // Registered UFCS builtin for this receiver kind.
            if let Some(kind) = crate::builtin::receiver_kind_from_type(&recv_type) {
                if crate::builtin::lookup_ufcs(kind, method).is_some() {
                    return Ok(());
                }
            }
            // Language UFCS: `x.f(args)` → `f(x, args)` for any in-scope function / Global builtin.
            if matches!(self.type_env.get(method), Some(Type::Function(_, _)))
                || crate::builtin::lookup(method).is_some()
            {
                return Ok(());
            }
            return Err(crate::error::e004_unknown_call(
                &format!("{type_name}.{method}"),
                self.current_span,
            ));
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

    /// M66: Constructor patterns must name a declared enum variant (and match arity).
    pub(crate) fn collect_pattern_constructor_errors(
        &self,
        pattern: &Pattern,
        errors: &mut Vec<CompilerError>,
        span: Span,
    ) {
        match pattern {
            Pattern::Constructor {
                name,
                args,
                named_fields,
            } => {
                match self.registry.lookup_variant(name) {
                    None => {
                        errors.push(crate::error::e014_unknown_enum_constructor(name, span));
                    }
                    Some((_ei, vi)) => {
                        // Positional patterns: same arity rule as enum calls.
                        if named_fields.is_empty() && vi.params.len() != args.len() {
                            errors.push(
                                CompilerError::new(format!(
                                    "Enum variant '{}' expects {} arguments, but got {}",
                                    name,
                                    vi.params.len(),
                                    args.len()
                                ))
                                .with_span(span),
                            );
                        }
                    }
                }
                for a in args {
                    self.collect_pattern_constructor_errors(a, errors, span);
                }
                for (_, p) in named_fields {
                    self.collect_pattern_constructor_errors(p, errors, span);
                }
            }
            Pattern::Or(patterns) | Pattern::Tuple(patterns) => {
                for p in patterns {
                    self.collect_pattern_constructor_errors(p, errors, span);
                }
            }
            _ => {}
        }
    }

    /// M65/M67: known Named struct → unknown `.field` is hard E013.
    pub(crate) fn unknown_struct_field_error(
        &self,
        obj: &Expr,
        field: &str,
        span: Span,
    ) -> Option<CompilerError> {
        let Type::Named(type_name) = self.try_infer_expr_type(obj) else {
            return None;
        };
        let struct_name = match type_name.as_str() {
            "Str" => "String",
            "Double" => "Float",
            other => other,
        };
        let info = self.registry.structs.get(struct_name)?;
        if info.field_index.contains_key(field) {
            return None;
        }
        Some(crate::error::e013_unknown_struct_field(
            struct_name,
            field,
            span,
        ))
    }

    /// M67/M70: under expected Named struct — field name set + field value types.
    pub(crate) fn check_struct_literal_against_named(
        &self,
        struct_name: &str,
        fields: &[(String, Expr)],
        span: Span,
        errors: &mut Vec<CompilerError>,
    ) {
        let Some(info) = self.registry.structs.get(struct_name) else {
            return;
        };
        let lit: std::collections::HashSet<&str> = fields.iter().map(|(n, _)| n.as_str()).collect();
        let decl: std::collections::HashSet<&str> =
            info.fields.iter().map(|(n, _)| n.as_str()).collect();
        for name in &lit {
            if !decl.contains(name) {
                errors.push(crate::error::e013_unknown_struct_field(
                    struct_name,
                    name,
                    span,
                ));
            }
        }
        for name in &decl {
            if !lit.contains(name) {
                errors.push(crate::error::e015_struct_literal_missing_field(
                    struct_name,
                    name,
                    span,
                ));
            }
        }
        // M70: value types for fields that exist on both sides (name-keyed).
        for (fname, fexpr) in fields {
            let Some((_, decl_ty)) = info.fields.iter().find(|(n, _)| n == fname) else {
                continue;
            };
            let got = self.try_infer_expr_type(fexpr);
            if matches!(got, Type::InferVar(_) | Type::TypeVar(_)) {
                // Soft: unresolved inference — don't claim a concrete mismatch.
                continue;
            }
            if !types_compatible(decl_ty, &got) {
                errors.push(crate::error::e016_struct_field_type_mismatch(
                    struct_name,
                    fname,
                    &format!("{decl_ty}"),
                    &format!("{got}"),
                    fexpr.span,
                ));
            }
        }
    }

    /// Peel block / nested return-expr wrappers so expected-type checks see the literal.
    pub(crate) fn peel_expr_for_struct_check<'a>(expr: &'a Expr) -> &'a Expr {
        match &expr.kind {
            ExprKind::Block(stmts) => {
                for stmt in stmts.iter().rev() {
                    match stmt {
                        Stmt::Expr { expr: inner, .. } => {
                            return Self::peel_expr_for_struct_check(inner);
                        }
                        Stmt::Return {
                            value: Some(inner), ..
                        } => {
                            return Self::peel_expr_for_struct_check(inner);
                        }
                        _ => {}
                    }
                }
                expr
            }
            _ => expr,
        }
    }

    /// M68: if `expected` is a Named struct and `expr` (peeled) is a StructLiteral, shape-check.
    pub(crate) fn check_expr_struct_literal_against_expected(
        &self,
        expected: &Type,
        expr: &Expr,
        errors: &mut Vec<CompilerError>,
    ) {
        let peeled = Self::peel_expr_for_struct_check(expr);
        if let (Type::Named(struct_name), ExprKind::StructLiteral(fields)) =
            (expected, &peeled.kind)
        {
            self.check_struct_literal_against_named(struct_name, fields, peeled.span, errors);
        }
    }

    /// M68: StructLiteral call args against user function parameter Named structs.
    pub(crate) fn check_call_struct_literal_shapes(
        &self,
        func: &Expr,
        args: &[Expr],
        errors: &mut Vec<CompilerError>,
    ) {
        let ExprKind::Ident(name) = &func.kind else {
            return;
        };
        let param_tys: Option<Vec<Type>> = match self.type_env.get(name) {
            Some(Type::Function(params, _)) => Some(params.clone()),
            _ => {
                let arg_tys: Vec<Type> = args
                    .iter()
                    .filter(|a| !matches!(&a.kind, ExprKind::Lambda { .. }))
                    .map(|a| self.try_infer_expr_type(a))
                    .collect();
                let mangled = mangle_name(name, &arg_tys);
                match self.type_env.get(&mangled) {
                    Some(Type::Function(params, _)) => Some(params.clone()),
                    _ => None,
                }
            }
        };
        let Some(param_tys) = param_tys else {
            return;
        };
        for (arg, param_ty) in args.iter().zip(param_tys.iter()) {
            if matches!(&arg.kind, ExprKind::Lambda { .. }) {
                continue;
            }
            self.check_expr_struct_literal_against_expected(param_ty, arg, errors);
        }
    }
}
