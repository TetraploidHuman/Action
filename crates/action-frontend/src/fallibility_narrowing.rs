//! Control-flow facts for fallibility narrowing (Kotlin-style smart bounds).
//!
//! When a fact holds (e.g. loop `i < lst.len()`), collection index / `.get(i)` need not use `or { }`.

use crate::ast::{BinaryOp, Expr, ExprKind, Literal};
use crate::fallible_safety::{call_is_compile_time_safe, index_access_is_compile_time_safe};
use std::collections::HashMap;

/// Active interval / length facts while type-checking a region.
#[derive(Clone, Debug, Default)]
pub struct NarrowingContext {
    upper: HashMap<String, UpperBound>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum UpperBound {
    /// `var < N` (loop counter assumed non-negative at entry).
    ConstLessThan(i64),
    /// `var < obj.len()` — safe to index `obj[var]`.
    LessThanLenOf(String),
}

impl NarrowingContext {
    pub fn from_loop_condition(condition: &Expr) -> Self {
        let mut ctx = Self::default();
        ctx.apply_condition(condition);
        ctx
    }

    pub fn with_guard(&self, condition: &Expr) -> Self {
        let mut ctx = self.clone();
        ctx.apply_condition(condition);
        ctx
    }

    fn apply_condition(&mut self, condition: &Expr) {
        match &condition.kind {
            ExprKind::Binary(l, BinaryOp::Lt, r) => {
                if let Some((var, bound)) = parse_lt(l, r) {
                    self.upper.insert(var, bound);
                }
            }
            ExprKind::Binary(l, BinaryOp::And, r) => {
                self.apply_condition(l);
                self.apply_condition(r);
            }
            ExprKind::Binary(l, BinaryOp::Gte, r) => {
                // `i >= 0` — recorded implicitly via loop; no-op for upper bound.
                let _ = (l, r);
            }
            _ => {}
        }
    }
}

fn parse_lt(l: &Expr, r: &Expr) -> Option<(String, UpperBound)> {
    let ExprKind::Ident(var) = &l.kind else {
        return None;
    };
    if let Some(n) = const_int(r) {
        return Some((var.clone(), UpperBound::ConstLessThan(n)));
    }
    if let Some(obj) = len_receiver_name(r) {
        return Some((var.clone(), UpperBound::LessThanLenOf(obj)));
    }
    None
}

fn const_int(expr: &Expr) -> Option<i64> {
    match &expr.kind {
        ExprKind::Literal(Literal::Int(n)) => Some(*n),
        _ => None,
    }
}

fn len_receiver_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Call { func, args, .. } => {
            if matches!(&func.kind, ExprKind::Ident(n) if n == "len") {
                args.first().and_then(ident_name)
            } else {
                None
            }
        }
        ExprKind::FieldAccess(obj, method) if method == "len" => ident_name(obj),
        _ => None,
    }
}

fn ident_name(expr: &Expr) -> Option<String> {
    match &expr.kind {
        ExprKind::Ident(n) => Some(n.clone()),
        _ => None,
    }
}

fn obj_matches_ident(obj: &Expr, name: &str) -> bool {
    ident_name(obj).is_some_and(|n| n == name)
}

/// Index access proven in-bounds by compile-time literals or active narrowing facts.
pub fn index_access_is_proven_safe(obj: &Expr, idx: &Expr, narrowing: &NarrowingContext) -> bool {
    if index_access_is_compile_time_safe(obj, idx) {
        return true;
    }
    let ExprKind::Ident(var) = &idx.kind else {
        return false;
    };
    let Some(bound) = narrowing.upper.get(var) else {
        return false;
    };
    match bound {
        UpperBound::LessThanLenOf(obj_name) => obj_matches_ident(obj, obj_name),
        UpperBound::ConstLessThan(_) => false,
    }
}

/// Fallible call proven safe (currently `.get` / `get` with narrowed index).
pub fn call_is_proven_safe(func: &Expr, args: &[Expr], narrowing: &NarrowingContext) -> bool {
    if call_is_compile_time_safe(func, args) {
        return true;
    }
    match &func.kind {
        ExprKind::Ident(name) if name == "get" && args.len() >= 2 => {
            index_access_is_proven_safe(&args[0], &args[1], narrowing)
        }
        ExprKind::FieldAccess(obj, method) if method == "get" => args
            .first()
            .is_some_and(|idx| index_access_is_proven_safe(obj, idx, narrowing)),
        _ => false,
    }
}

// ---- HIR mirrors (codegen) ----

use crate::hir::{HirExpr, HirExprKind};

impl NarrowingContext {
    pub fn from_hir_loop_condition(condition: &HirExpr) -> Self {
        let mut ctx = Self::default();
        ctx.apply_hir_condition(condition);
        ctx
    }

    pub fn with_hir_guard(&self, condition: &HirExpr) -> Self {
        let mut ctx = self.clone();
        ctx.apply_hir_condition(condition);
        ctx
    }

    fn apply_hir_condition(&mut self, condition: &HirExpr) {
        match &condition.kind {
            HirExprKind::Binary(l, BinaryOp::Lt, r) => {
                if let Some((var, bound)) = parse_hir_lt(l, r) {
                    self.upper.insert(var, bound);
                }
            }
            HirExprKind::Binary(l, BinaryOp::And, r) => {
                self.apply_hir_condition(l);
                self.apply_hir_condition(r);
            }
            _ => {}
        }
    }
}

fn parse_hir_lt(l: &HirExpr, r: &HirExpr) -> Option<(String, UpperBound)> {
    let HirExprKind::Ident(var) = &l.kind else {
        return None;
    };
    if let Some(n) = hir_const_int(r) {
        return Some((var.clone(), UpperBound::ConstLessThan(n)));
    }
    if let Some(obj) = hir_len_receiver_name(r) {
        return Some((var.clone(), UpperBound::LessThanLenOf(obj)));
    }
    None
}

fn hir_const_int(expr: &HirExpr) -> Option<i64> {
    match &expr.kind {
        HirExprKind::Literal(Literal::Int(n)) => Some(*n),
        _ => None,
    }
}

fn hir_len_receiver_name(expr: &HirExpr) -> Option<String> {
    match &expr.kind {
        HirExprKind::Call { func, args, .. } => {
            if matches!(&func.kind, HirExprKind::Ident(n) if n == "len") {
                args.first().and_then(hir_ident_name)
            } else {
                None
            }
        }
        HirExprKind::FieldAccess(obj, method) if method == "len" => hir_ident_name(obj),
        _ => None,
    }
}

fn hir_ident_name(expr: &HirExpr) -> Option<String> {
    match &expr.kind {
        HirExprKind::Ident(n) => Some(n.clone()),
        _ => None,
    }
}

fn hir_obj_matches_ident(obj: &HirExpr, name: &str) -> bool {
    hir_ident_name(obj).is_some_and(|n| n == name)
}

pub fn hir_index_access_is_proven_safe(
    obj: &HirExpr,
    idx: &HirExpr,
    narrowing: &NarrowingContext,
) -> bool {
    if crate::fallible_safety::hir_index_access_is_compile_time_safe(obj, idx) {
        return true;
    }
    let HirExprKind::Ident(var) = &idx.kind else {
        return false;
    };
    let Some(bound) = narrowing.upper.get(var) else {
        return false;
    };
    match bound {
        UpperBound::LessThanLenOf(obj_name) => hir_obj_matches_ident(obj, obj_name),
        UpperBound::ConstLessThan(_) => false,
    }
}

pub fn hir_call_is_proven_safe(
    func: &HirExpr,
    args: &[HirExpr],
    narrowing: &NarrowingContext,
) -> bool {
    if crate::fallible_safety::hir_call_is_compile_time_safe(func, args) {
        return true;
    }
    match &func.kind {
        HirExprKind::Ident(name) if name == "get" && args.len() >= 2 => {
            hir_index_access_is_proven_safe(&args[0], &args[1], narrowing)
        }
        HirExprKind::FieldAccess(obj, method) if method == "get" => args
            .first()
            .is_some_and(|idx| hir_index_access_is_proven_safe(obj, idx, narrowing)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Expr;

    fn int(n: i64) -> Expr {
        ExprKind::Literal(Literal::Int(n)).into()
    }

    fn ident(name: &str) -> Expr {
        ExprKind::Ident(name.to_string()).into()
    }

    fn lt(l: Expr, r: Expr) -> Expr {
        ExprKind::Binary(Box::new(l), BinaryOp::Lt, Box::new(r)).into()
    }

    fn len_of(obj: Expr) -> Expr {
        ExprKind::Call {
            func: Box::new(ident("len")),
            args: vec![obj],
            trailing_lambda: None,
        }
        .into()
    }

    #[test]
    fn loop_i_lt_lst_len_narrows_get() {
        let cond = lt(ident("i"), len_of(ident("lst")));
        let ctx = NarrowingContext::from_loop_condition(&cond);
        let idx = ident("i");
        assert!(index_access_is_proven_safe(&ident("lst"), &idx, &ctx));
    }
}
