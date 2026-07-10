//! Compile-time proofs that collection access / fallible calls cannot fail.
//!
//! When a proof holds, callers need not wrap the expression in `or { }` / `??`.

use crate::ast::{Expr, ExprKind, Literal};

/// `List[...]` / `__list(...)` element count when the receiver is a list literal.
pub fn list_literal_len(expr: &Expr) -> Option<usize> {
    match &expr.kind {
        ExprKind::Call { func, args, .. } => {
            if matches!(
                &func.kind,
                ExprKind::Ident(name) if name == "List" || name == "__list"
            ) {
                Some(args.len())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn const_int_lit(expr: &Expr) -> Option<i64> {
    match &expr.kind {
        ExprKind::Literal(Literal::Int(n)) => Some(*n),
        _ => None,
    }
}

fn literal_key(expr: &Expr) -> Option<Literal> {
    match &expr.kind {
        ExprKind::Literal(lit) => Some(lit.clone()),
        _ => None,
    }
}

fn literals_equal(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Literal(la), ExprKind::Literal(lb)) => la == lb,
        _ => false,
    }
}

/// Receiver is a collection that uses bracket indexing in the type system.
pub fn is_collection_index_receiver(expr: &Expr) -> bool {
    list_literal_len(expr).is_some()
        || map_literal_entries(expr).is_some()
        || set_literal_elements(expr).is_some()
        || matches!(
            &expr.kind,
            ExprKind::Call { func, .. }
                if matches!(
                    &func.kind,
                    ExprKind::Ident(name) if name == "Map" || name == "__map"
                        || name == "Set" || name == "__set"
                )
        )
        || matches!(
            &expr.kind,
            ExprKind::MapLiteral(_) | ExprKind::SetLiteral(_)
        )
}

pub(crate) fn map_literal_entries(expr: &Expr) -> Option<&[(Expr, Expr)]> {
    match &expr.kind {
        ExprKind::MapLiteral(entries) => Some(entries.as_slice()),
        _ => None,
    }
}

pub(crate) fn set_literal_elements(expr: &Expr) -> Option<&[Expr]> {
    match &expr.kind {
        ExprKind::SetLiteral(elems) => Some(elems.as_slice()),
        _ => None,
    }
}

/// `lst[i]` / `map[k]` / `set[e]` cannot fail at runtime (literal receiver + static key/index).
pub fn index_access_is_compile_time_safe(obj: &Expr, idx: &Expr) -> bool {
    if let Some(len) = list_literal_len(obj) {
        if let Some(i) = const_int_lit(idx) {
            return i >= 0 && (i as usize) < len;
        }
        return false;
    }
    if let Some(entries) = map_literal_entries(obj) {
        if let Some(key_lit) = literal_key(idx) {
            return entries
                .iter()
                .any(|(k, _)| matches!(&k.kind, ExprKind::Literal(l) if l == &key_lit));
        }
        return false;
    }
    if let Some(elems) = set_literal_elements(obj) {
        return elems.iter().any(|e| literals_equal(e, idx));
    }
    false
}

/// `get` / UFCS `.get(i)` / `head` / `last` / `tail` / `init` on literal receivers, etc.
pub fn call_is_compile_time_safe(func: &Expr, args: &[Expr]) -> bool {
    match &func.kind {
        ExprKind::Ident(name) => match name.as_str() {
            "get" => args.len() >= 2 && index_access_is_compile_time_safe(&args[0], &args[1]),
            "head" | "last" | "tail" | "init" => args
                .first()
                .and_then(|a| list_literal_len(a))
                .is_some_and(|len| len > 0),
            _ => false,
        },
        ExprKind::FieldAccess(obj, method) => match method.as_str() {
            "get" => args
                .first()
                .is_some_and(|idx| index_access_is_compile_time_safe(obj, idx)),
            "head" | "last" | "tail" | "init" => list_literal_len(obj).is_some_and(|len| len > 0),
            _ => false,
        },
        _ => false,
    }
}

pub fn is_parse_numeric_call(func: &Expr) -> bool {
    match &func.kind {
        ExprKind::Ident(name) => matches!(name.as_str(), "parseInt" | "toInt" | "toFloat"),
        ExprKind::FieldAccess(_, method) => matches!(method.as_str(), "toInt" | "toFloat"),
        _ => false,
    }
}

// ---- HIR mirrors (codegen fast paths) ----

use crate::hir::{HirExpr, HirExprKind};

pub fn hir_index_access_is_compile_time_safe(obj: &HirExpr, idx: &HirExpr) -> bool {
    if matches!(&idx.kind, HirExprKind::Literal(Literal::Int(i)) if *i < 0) {
        return false;
    }
    hir_list_literal_len(obj).is_some_and(|len| {
        matches!(&idx.kind, HirExprKind::Literal(Literal::Int(i)) if *i >= 0 && (*i as usize) < len)
    }) || hir_map_literal_has_key(obj, idx)
        || hir_set_literal_has_elem(obj, idx)
}

fn hir_list_literal_len(expr: &HirExpr) -> Option<usize> {
    match &expr.kind {
        HirExprKind::Call { func, args, .. } => {
            if matches!(
                &func.kind,
                HirExprKind::Ident(name) if name == "List" || name == "__list"
            ) {
                Some(args.len())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn hir_map_literal_has_key(obj: &HirExpr, idx: &HirExpr) -> bool {
    match (&obj.kind, &idx.kind) {
        (HirExprKind::MapLiteral(entries), HirExprKind::Literal(key_lit)) => entries
            .iter()
            .any(|(k, _)| matches!(&k.kind, HirExprKind::Literal(l) if l == key_lit)),
        _ => false,
    }
}

fn hir_set_literal_has_elem(obj: &HirExpr, idx: &HirExpr) -> bool {
    match &obj.kind {
        HirExprKind::SetLiteral(elems) => elems.iter().any(|e| hir_literals_equal(e, idx)),
        _ => false,
    }
}

fn hir_literals_equal(a: &HirExpr, b: &HirExpr) -> bool {
    matches!(
        (&a.kind, &b.kind),
        (HirExprKind::Literal(la), HirExprKind::Literal(lb)) if la == lb
    )
}

pub fn hir_call_is_compile_time_safe(func: &HirExpr, args: &[HirExpr]) -> bool {
    match &func.kind {
        HirExprKind::Ident(name) => match name.as_str() {
            "get" => args.len() >= 2 && hir_index_access_is_compile_time_safe(&args[0], &args[1]),
            "head" | "last" | "tail" | "init" => args
                .first()
                .and_then(|a| hir_list_literal_len(a))
                .is_some_and(|len| len > 0),
            _ => false,
        },
        HirExprKind::FieldAccess(obj, method) => match method.as_str() {
            "get" => args
                .first()
                .is_some_and(|idx| hir_index_access_is_compile_time_safe(obj, idx)),
            "head" | "last" | "tail" | "init" => {
                hir_list_literal_len(obj).is_some_and(|len| len > 0)
            }
            _ => false,
        },
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

    fn list(items: Vec<Expr>) -> Expr {
        Expr::call(ExprKind::Ident("__list".to_string()).into(), items)
    }

    fn index(obj: Expr, idx: Expr) -> Expr {
        ExprKind::Index(Box::new(obj), Box::new(idx)).into()
    }

    #[test]
    fn list_literal_in_bounds_index_is_safe() {
        let lst = list(vec![int(1), int(2), int(3)]);
        assert!(index_access_is_compile_time_safe(&lst, &int(0)));
        assert!(index_access_is_compile_time_safe(&lst, &int(2)));
        assert!(!index_access_is_compile_time_safe(&lst, &int(3)));
        assert!(!index_access_is_compile_time_safe(&list(vec![]), &int(0)));
        let _ = index(lst, int(0));
    }

    #[test]
    fn head_on_nonempty_list_literal_is_safe() {
        let lst = list(vec![int(1)]);
        let func = ExprKind::Ident("head".to_string()).into();
        assert!(call_is_compile_time_safe(
            &func,
            std::slice::from_ref(&lst)
        ));
        assert!(!call_is_compile_time_safe(
            &func,
            std::slice::from_ref(&list(vec![]))
        ));
    }
}
