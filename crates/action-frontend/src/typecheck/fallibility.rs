//! Fallibility analysis context and R1–R9 rules (R7 vertical slice).
//!
//! `EMIT_E001` is gated off until the full error surface is enabled.

use crate::ast::{Expr, ExprKind, Stmt, Type};
use crate::builtin::{self, UfcsReceiverKind};
use crate::error::{e001_or_required, e002_or_type_mismatch, CompilerError};
use crate::function_symbol::FunctionSymbol;
use crate::types::types_compatible;
use std::collections::HashMap;

/// When false, E001 (fallible value used without `or {}`) is not emitted.
pub const EMIT_E001: bool = true;

/// Tracks fallibility state during type-checking.
#[derive(Clone, Debug, Default)]
pub struct FallibilityContext {
    /// Whether we are inside an `or { }` block (expression or function fallback).
    pub in_or_block: bool,
    /// Function-level `or { }` fallback expression, if any.
    pub fn_or_fallback: bool,
    /// Inside a user function body that may propagate fallible failures (no fn-level `or {}`).
    pub allow_bare_fallible_in_fn: bool,
    /// Resolved symbols: name → metadata.
    pub symbols: HashMap<String, FunctionSymbol>,
}

impl FallibilityContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_or_block(&self) -> Self {
        let mut ctx = self.clone();
        ctx.in_or_block = true;
        ctx
    }

    pub fn register_builtin(&mut self, name: &str) {
        if let Some(def) = builtin::lookup(name) {
            self.symbols.insert(
                name.to_string(),
                FunctionSymbol::new(def.fallible, def.return_type.clone()),
            );
        }
    }

    pub fn register_ufcs(&mut self, method: &str) {
        for kind in [
            UfcsReceiverKind::List,
            UfcsReceiverKind::String,
            UfcsReceiverKind::Map,
            UfcsReceiverKind::Set,
            UfcsReceiverKind::Collection,
            UfcsReceiverKind::Global,
        ] {
            if let Some(def) = builtin::lookup_ufcs(kind, method) {
                self.symbols.insert(
                    format!("ufcs:{}", method),
                    FunctionSymbol::new(def.fallible, def.return_type.clone()),
                );
                break;
            }
        }
    }

    pub fn callee_requires_or(&self, name: &str) -> bool {
        call_requires_or(name, self)
    }

    /// Map `mod.fn` module access to the imported LLVM symbol `mod_fn`.
    pub fn module_callee_symbol(mod_name: &str, field: &str) -> String {
        format!("{}_{}", mod_name, field)
    }

    /// R1: fallible call outside `or {}` with non-nullable expectation → E001 (when enabled).
    pub fn check_r1_fallible_needs_or(
        &self,
        span: action_span::Span,
        name: &str,
        expected: &Type,
    ) -> Option<CompilerError> {
        if self.in_or_block || self.fn_or_fallback || self.allow_bare_fallible_in_fn {
            return None;
        }
        let sym = self.symbols.get(name)?;
        if !sym.is_fallible {
            return None;
        }
        let _ = expected;
        if EMIT_E001 {
            Some(e001_or_required(name, span))
        } else {
            None
        }
    }

    /// R2–R9 placeholders for future rules (skeleton only).
    pub fn check_r2_or_block_result_type(
        &self,
        nullable_ty: &Type,
        fallback_ty: &Type,
        span: action_span::Span,
    ) -> Option<CompilerError> {
        let lhs = match nullable_ty {
            Type::Nullable(inner) => inner.as_ref(),
            other => other,
        };
        if types_compatible(lhs, fallback_ty) || types_compatible(fallback_ty, lhs) {
            None
        } else {
            Some(e002_or_type_mismatch(span))
        }
    }

    pub fn check_r3_fn_or_return_match(
        &self,
        declared: &Type,
        fallback: &Type,
        span: action_span::Span,
    ) -> Option<CompilerError> {
        if !crate::types::types_compatible(declared, fallback) {
            Some(crate::error::e003_fn_or_return(span))
        } else {
            None
        }
    }

    pub fn effective_return_type(&self, sym: &FunctionSymbol, in_or: bool) -> Type {
        if sym.is_fallible && in_or {
            match &sym.return_type {
                Type::Nullable(inner) => *inner.clone(),
                other => other.clone(),
            }
        } else {
            sym.return_type.clone()
        }
    }

    /// Analyze a function definition for fallibility metadata.
    pub fn analyze_function(
        &mut self,
        name: &str,
        return_type: &Option<Type>,
        fn_or_fallback: &Option<Expr>,
        body: &Expr,
    ) {
        let had_fn_or = self.fn_or_fallback;
        self.fn_or_fallback = fn_or_fallback.is_some();
        if let Some(fb) = fn_or_fallback {
            if let Some(ret) = return_type {
                let _ = self.check_r3_fn_or_return_match(ret, &infer_expr_type_simple(fb), fb.span);
            }
        }
        walk_expr(body, self);
        self.fn_or_fallback = had_fn_or;

        let has_bare = expr_has_bare_propagating_fallible(body, self);
        let is_fallible = fn_or_fallback.is_none() && has_bare;
        let ret = return_type
            .clone()
            .unwrap_or_else(|| infer_expr_type_simple(body));
        self.symbols
            .insert(name.to_string(), FunctionSymbol::new(is_fallible, ret));
    }

    /// Walk expression tree registering fallible call sites (R4–R9 skeleton).
    pub fn analyze_expr(&mut self, expr: &Expr) {
        walk_expr(expr, self);
    }
}

fn infer_expr_type_simple(expr: &Expr) -> Type {
    match &expr.kind {
        ExprKind::Literal(crate::ast::Literal::Int(_)) => Type::Named("Int".into()),
        ExprKind::Literal(crate::ast::Literal::String(_)) => Type::Named("String".into()),
        ExprKind::Block(stmts) => stmts
            .last()
            .and_then(|s| match s {
                Stmt::Expr { expr, .. } => Some(infer_expr_type_simple(expr)),
                Stmt::Return { value: Some(v), .. } => Some(infer_expr_type_simple(v)),
                _ => None,
            })
            .unwrap_or(Type::Unit),
        _ => Type::Named("Int".into()),
    }
}

fn call_requires_or(name: &str, ctx: &FallibilityContext) -> bool {
    if let Some(def) = builtin::lookup(name) {
        return def.fallible;
    }
    ctx.symbols.get(name).is_some_and(|sym| sym.is_fallible)
}

/// Bare fallible call (builtin/UFCS or propagating user fn) makes the enclosing function use `{T,i1}` ABI.
fn call_makes_fn_propagating(name: &str, ctx: &FallibilityContext) -> bool {
    call_requires_or(name, ctx)
}

fn ufcs_makes_fn_propagating(method: &str) -> bool {
    for kind in [
        UfcsReceiverKind::List,
        UfcsReceiverKind::String,
        UfcsReceiverKind::Map,
        UfcsReceiverKind::Set,
        UfcsReceiverKind::Collection,
        UfcsReceiverKind::Global,
    ] {
        if let Some(def) = builtin::lookup_ufcs(kind, method) {
            return def.fallible;
        }
    }
    false
}

fn expr_has_bare_propagating_fallible(expr: &Expr, ctx: &FallibilityContext) -> bool {
    expr_has_bare_propagating_inner(expr, ctx, false, ctx.fn_or_fallback)
}

fn expr_has_bare_propagating_inner(
    expr: &Expr,
    ctx: &FallibilityContext,
    in_or: bool,
    in_fn_or: bool,
) -> bool {
    match &expr.kind {
        ExprKind::Call { func, args, .. } => {
            let mut bare = false;
            match &func.kind {
                ExprKind::Ident(name) => {
                    if !in_or && !in_fn_or && call_makes_fn_propagating(name, ctx) {
                        bare = true;
                    }
                }
                ExprKind::FieldAccess(obj, method) => {
                    if let ExprKind::Ident(mod_name) = &obj.kind {
                        let mangled = FallibilityContext::module_callee_symbol(mod_name, method);
                        if !in_or && !in_fn_or && call_requires_or(&mangled, ctx) {
                            bare = true;
                        }
                    } else if !in_or && !in_fn_or && ufcs_makes_fn_propagating(method) {
                        bare = true;
                    }
                }
                _ => {}
            }
            bare || args
                .iter()
                .any(|a| expr_has_bare_propagating_inner(a, ctx, in_or, in_fn_or))
        }
        ExprKind::OrBlock { nullable, fallback } => {
            expr_has_bare_propagating_inner(nullable, ctx, true, in_fn_or)
                || expr_has_bare_propagating_inner(fallback, ctx, true, in_fn_or)
        }
        ExprKind::FieldAccess(obj, method) => {
            expr_has_bare_propagating_inner(obj, ctx, in_or, in_fn_or)
                || (!in_or && !in_fn_or && ufcs_makes_fn_propagating(method))
        }
        ExprKind::Index(obj, idx) => {
            expr_has_bare_propagating_inner(obj, ctx, in_or, in_fn_or)
                || expr_has_bare_propagating_inner(idx, ctx, in_or, in_fn_or)
        }
        ExprKind::Binary(a, _, b) => {
            expr_has_bare_propagating_inner(a, ctx, in_or, in_fn_or)
                || expr_has_bare_propagating_inner(b, ctx, in_or, in_fn_or)
        }
        ExprKind::Unary(_, a) => expr_has_bare_propagating_inner(a, ctx, in_or, in_fn_or),
        ExprKind::Block(stmts) => stmts
            .iter()
            .any(|s| stmt_has_bare_propagating(s, ctx, in_or, in_fn_or)),
        ExprKind::When(w) => when_has_bare_propagating(w, ctx, in_or, in_fn_or),
        ExprKind::For(f) => for_has_bare_propagating(f, ctx, in_or, in_fn_or),
        ExprKind::Lambda { body, .. } => {
            expr_has_bare_propagating_inner(body, ctx, in_or, in_fn_or)
        }
        ExprKind::Assign { target, value, .. } => {
            expr_has_bare_propagating_inner(target, ctx, in_or, in_fn_or)
                || expr_has_bare_propagating_inner(value, ctx, in_or, in_fn_or)
        }
        ExprKind::Copy(a) | ExprKind::Unsafe(a) => {
            expr_has_bare_propagating_inner(a, ctx, in_or, in_fn_or)
        }
        ExprKind::StructLiteral(fields) => fields
            .iter()
            .any(|(_, e)| expr_has_bare_propagating_inner(e, ctx, in_or, in_fn_or)),
        ExprKind::MapLiteral(entries) => entries.iter().any(|(k, v)| {
            expr_has_bare_propagating_inner(k, ctx, in_or, in_fn_or)
                || expr_has_bare_propagating_inner(v, ctx, in_or, in_fn_or)
        }),
        ExprKind::SetLiteral(elems) => elems
            .iter()
            .any(|e| expr_has_bare_propagating_inner(e, ctx, in_or, in_fn_or)),
        ExprKind::Tuple(items) => items
            .iter()
            .any(|(_, e)| expr_has_bare_propagating_inner(e, ctx, in_or, in_fn_or)),
        ExprKind::StringInterpolate(parts) => {
            for p in parts {
                if let crate::ast::StringPart::Expr(e) = p {
                    if expr_has_bare_propagating_inner(e, ctx, in_or, in_fn_or) {
                        return true;
                    }
                }
            }
            false
        }
        _ => false,
    }
}

fn stmt_has_bare_propagating(
    stmt: &Stmt,
    ctx: &FallibilityContext,
    in_or: bool,
    in_fn_or: bool,
) -> bool {
    match stmt {
        Stmt::Let { value, .. } | Stmt::Expr { expr: value, .. } => {
            expr_has_bare_propagating_inner(value, ctx, in_or, in_fn_or)
        }
        Stmt::Return { value: Some(v), .. } => {
            expr_has_bare_propagating_inner(v, ctx, in_or, in_fn_or)
        }
        Stmt::Fun {
            body,
            fn_or_fallback,
            ..
        } => {
            let inner_fn_or = fn_or_fallback.is_some();
            let fb_bare = fn_or_fallback
                .as_ref()
                .is_some_and(|fb| expr_has_bare_propagating_inner(fb, ctx, true, inner_fn_or));
            fb_bare || expr_has_bare_propagating_inner(body, ctx, in_or, in_fn_or || inner_fn_or)
        }
        _ => false,
    }
}

fn when_has_bare_propagating(
    w: &crate::ast::When,
    ctx: &FallibilityContext,
    in_or: bool,
    in_fn_or: bool,
) -> bool {
    use crate::ast::WhenKind;
    match &w.kind {
        WhenKind::OneLine {
            condition,
            then_expr,
            else_expr,
        } => {
            expr_has_bare_propagating_inner(condition, ctx, in_or, in_fn_or)
                || expr_has_bare_propagating_inner(then_expr, ctx, in_or, in_fn_or)
                || expr_has_bare_propagating_inner(else_expr, ctx, in_or, in_fn_or)
        }
        WhenKind::ValueMatch { value, arms } => {
            expr_has_bare_propagating_inner(value, ctx, in_or, in_fn_or)
                || arms.iter().any(|arm| {
                    arm.guard
                        .as_ref()
                        .is_some_and(|g| expr_has_bare_propagating_inner(g, ctx, in_or, in_fn_or))
                        || expr_has_bare_propagating_inner(&arm.body, ctx, in_or, in_fn_or)
                })
        }
        WhenKind::ConditionChain { arms } => arms.iter().any(|arm| {
            arm.guard
                .as_ref()
                .is_some_and(|g| expr_has_bare_propagating_inner(g, ctx, in_or, in_fn_or))
                || expr_has_bare_propagating_inner(&arm.body, ctx, in_or, in_fn_or)
        }),
    }
}

fn for_has_bare_propagating(
    f: &crate::ast::For,
    ctx: &FallibilityContext,
    in_or: bool,
    in_fn_or: bool,
) -> bool {
    use crate::ast::ForKind;
    match &f.kind {
        ForKind::Iterate { iterable, body, .. } => {
            expr_has_bare_propagating_inner(iterable, ctx, in_or, in_fn_or)
                || expr_has_bare_propagating_inner(body, ctx, in_or, in_fn_or)
        }
        ForKind::IterateWithIndex { iterable, body, .. } => {
            expr_has_bare_propagating_inner(iterable, ctx, in_or, in_fn_or)
                || expr_has_bare_propagating_inner(body, ctx, in_or, in_fn_or)
        }
        ForKind::Condition { condition, body } => {
            expr_has_bare_propagating_inner(condition, ctx, in_or, in_fn_or)
                || expr_has_bare_propagating_inner(body, ctx, in_or, in_fn_or)
        }
        ForKind::NestedIterate { bindings, body, .. } => {
            bindings
                .iter()
                .any(|(_, e)| expr_has_bare_propagating_inner(e, ctx, in_or, in_fn_or))
                || expr_has_bare_propagating_inner(body, ctx, in_or, in_fn_or)
        }
        ForKind::Infinite { body } => expr_has_bare_propagating_inner(body, ctx, in_or, in_fn_or),
    }
}

fn walk_expr(expr: &Expr, ctx: &mut FallibilityContext) {
    match &expr.kind {
        ExprKind::Call { func, args, .. } => {
            if let ExprKind::Ident(name) = &func.kind {
                ctx.register_builtin(name);
            }
            for a in args {
                walk_expr(a, ctx);
            }
        }
        ExprKind::OrBlock { nullable, fallback } => {
            walk_expr(nullable, ctx);
            let saved = ctx.in_or_block;
            ctx.in_or_block = true;
            walk_expr(fallback, ctx);
            ctx.in_or_block = saved;
        }
        ExprKind::FieldAccess(obj, method) => {
            walk_expr(obj, ctx);
            ctx.register_ufcs(method);
        }
        ExprKind::Index(obj, idx) => {
            walk_expr(obj, ctx);
            walk_expr(idx, ctx);
        }
        ExprKind::Binary(a, _, b) => {
            walk_expr(a, ctx);
            walk_expr(b, ctx);
        }
        ExprKind::Unary(_, a) => walk_expr(a, ctx),
        ExprKind::Block(stmts) => {
            for s in stmts {
                walk_stmt(s, ctx);
            }
        }
        ExprKind::When(w) => walk_when(w, ctx),
        ExprKind::For(f) => walk_for(f, ctx),
        ExprKind::Lambda { body, .. } => walk_expr(body, ctx),
        ExprKind::Assign { target, value, .. } => {
            walk_expr(target, ctx);
            walk_expr(value, ctx);
        }
        ExprKind::Copy(a) | ExprKind::Unsafe(a) => walk_expr(a, ctx),
        ExprKind::StructLiteral(fields) => {
            for (_, e) in fields {
                walk_expr(e, ctx);
            }
        }
        ExprKind::MapLiteral(entries) => {
            for (k, v) in entries {
                walk_expr(k, ctx);
                walk_expr(v, ctx);
            }
        }
        ExprKind::SetLiteral(elems) => {
            for e in elems {
                walk_expr(e, ctx);
            }
        }
        ExprKind::Tuple(items) => {
            for (_, e) in items {
                walk_expr(e, ctx);
            }
        }
        ExprKind::StringInterpolate(parts) => {
            for p in parts {
                if let crate::ast::StringPart::Expr(e) = p {
                    walk_expr(e, ctx);
                }
            }
        }
        _ => {}
    }
}

fn walk_stmt(stmt: &Stmt, ctx: &mut FallibilityContext) {
    match stmt {
        Stmt::Let { value, .. } | Stmt::Expr { expr: value, .. } => walk_expr(value, ctx),
        Stmt::Return { value: Some(v), .. } => walk_expr(v, ctx),
        Stmt::Fun {
            body,
            fn_or_fallback,
            ..
        } => {
            if let Some(fb) = fn_or_fallback {
                let saved = ctx.in_or_block;
                ctx.in_or_block = true;
                walk_expr(fb, ctx);
                ctx.in_or_block = saved;
            }
            walk_expr(body, ctx);
        }
        _ => {}
    }
}

fn walk_when(w: &crate::ast::When, ctx: &mut FallibilityContext) {
    use crate::ast::WhenKind;
    match &w.kind {
        WhenKind::OneLine {
            condition,
            then_expr,
            else_expr,
        } => {
            walk_expr(condition, ctx);
            walk_expr(then_expr, ctx);
            walk_expr(else_expr, ctx);
        }
        WhenKind::ValueMatch { value, arms } => {
            walk_expr(value, ctx);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    walk_expr(g, ctx);
                }
                walk_expr(&arm.body, ctx);
            }
        }
        WhenKind::ConditionChain { arms } => {
            for arm in arms {
                if let Some(g) = &arm.guard {
                    walk_expr(g, ctx);
                }
                walk_expr(&arm.body, ctx);
            }
        }
    }
}

fn walk_for(f: &crate::ast::For, ctx: &mut FallibilityContext) {
    use crate::ast::ForKind;
    match &f.kind {
        ForKind::Iterate { iterable, body, .. } => {
            walk_expr(iterable, ctx);
            walk_expr(body, ctx);
        }
        ForKind::IterateWithIndex { iterable, body, .. } => {
            walk_expr(iterable, ctx);
            walk_expr(body, ctx);
        }
        ForKind::Condition { condition, body } => {
            walk_expr(condition, ctx);
            walk_expr(body, ctx);
        }
        ForKind::NestedIterate { bindings, body, .. } => {
            for (_, e) in bindings {
                walk_expr(e, ctx);
            }
            walk_expr(body, ctx);
        }
        ForKind::Infinite { body } => walk_expr(body, ctx),
    }
}
