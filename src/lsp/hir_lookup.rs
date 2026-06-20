//! HIR span lookup for LSP hover / expression types.

use crate::ast::Type;
use crate::hir::{HirExpr, HirExprKind, HirModule, HirStmt};
use crate::lexer::Span;
use lsp_types::Position;

use super::position;

/// Find parameter names for a top-level or module-scoped function by name.
pub fn find_fun_param_names(hir: &HirModule, name: &str) -> Option<Vec<String>> {
    find_fun_param_names_in_stmts(&hir.stmts, name)
}

fn find_fun_param_names_in_stmts(stmts: &[HirStmt], name: &str) -> Option<Vec<String>> {
    for stmt in stmts {
        if let Some(names) = find_fun_param_names_in_stmt(stmt, name) {
            return Some(names);
        }
    }
    None
}

fn find_fun_param_names_in_stmt(stmt: &HirStmt, name: &str) -> Option<Vec<String>> {
    match stmt {
        HirStmt::Fun {
            name: n,
            params,
            ..
        } if n == name => Some(params.iter().map(|p| p.name.clone()).collect()),
        HirStmt::Module { body, .. } => find_fun_param_names_in_stmts(body, name),
        HirStmt::Export { stmt, .. } => find_fun_param_names_in_stmt(stmt, name),
        _ => None,
    }
}

/// Find the innermost HIR expression whose span contains `pos`.
pub fn find_hir_expr_type(hir: &HirModule, source: &str, pos: &Position) -> Option<Type> {
    let offset = position::lsp_position_to_offset(source, pos);
    find_hir_expr_at_offset(&hir.stmts, offset).map(|e| e.ty.clone())
}

fn find_hir_expr_at_offset(stmts: &[HirStmt], offset: usize) -> Option<&HirExpr> {
    let mut best: Option<&HirExpr> = None;
    for stmt in stmts {
        if let Some(expr) = find_hir_expr_in_stmt(stmt, offset) {
            best = pick_smaller_span(best, expr);
        }
    }
    best
}

fn pick_smaller_span<'a>(
    current: Option<&'a HirExpr>,
    candidate: &'a HirExpr,
) -> Option<&'a HirExpr> {
    match current {
        None => Some(candidate),
        Some(prev) => {
            if span_len(&candidate.span) < span_len(&prev.span) {
                Some(candidate)
            } else {
                Some(prev)
            }
        }
    }
}

fn span_len(span: &Span) -> usize {
    span.end.saturating_sub(span.start)
}

fn find_hir_expr_in_stmt(stmt: &HirStmt, offset: usize) -> Option<&HirExpr> {
    match stmt {
        HirStmt::Let { value, .. }
        | HirStmt::Destructure { value, .. }
        | HirStmt::Expr { expr: value, .. } => find_hir_expr_at(value, offset),
        HirStmt::Return {
            value: Some(value), ..
        } => find_hir_expr_at(value, offset),
        HirStmt::Fun { body, .. } => find_hir_expr_at(body, offset),
        HirStmt::Const { value, .. } => find_hir_expr_at(value, offset),
        HirStmt::Module { body, .. } => find_hir_expr_at_offset(body, offset),
        HirStmt::Export { stmt, .. } => find_hir_expr_in_stmt(stmt, offset),
        HirStmt::Extension { methods, .. } => {
            let mut best = None;
            for m in methods {
                if let Some(expr) = find_hir_expr_in_stmt(m, offset) {
                    best = pick_smaller_span(best, expr);
                }
            }
            best
        }
        _ => None,
    }
}

fn find_hir_expr_at(expr: &HirExpr, offset: usize) -> Option<&HirExpr> {
    if !span_contains(&expr.span, offset) {
        return None;
    }
    let mut best = Some(expr);
    match &expr.kind {
        HirExprKind::Binary(lhs, _, rhs) => {
            if let Some(inner) = find_hir_expr_at(lhs, offset) {
                best = pick_smaller_span(best, inner);
            }
            if let Some(inner) = find_hir_expr_at(rhs, offset) {
                best = pick_smaller_span(best, inner);
            }
        }
        HirExprKind::Unary(_, inner) => {
            if let Some(inner) = find_hir_expr_at(inner, offset) {
                best = pick_smaller_span(best, inner);
            }
        }
        HirExprKind::Call {
            func,
            args,
            trailing_lambda,
        } => {
            if let Some(inner) = find_hir_expr_at(func, offset) {
                best = pick_smaller_span(best, inner);
            }
            for arg in args {
                if let Some(inner) = find_hir_expr_at(arg, offset) {
                    best = pick_smaller_span(best, inner);
                }
            }
            if let Some(lam) = trailing_lambda {
                if let Some(inner) = find_hir_expr_at(lam, offset) {
                    best = pick_smaller_span(best, inner);
                }
            }
        }
        HirExprKind::Lambda { body, .. } => {
            if let Some(inner) = find_hir_expr_at(body, offset) {
                best = pick_smaller_span(best, inner);
            }
        }
        HirExprKind::When(w) => match &w.kind {
            crate::hir::HirWhenKind::OneLine {
                condition,
                then_expr,
                else_expr,
            } => {
                for sub in [condition.as_ref(), then_expr.as_ref(), else_expr.as_ref()] {
                    if let Some(inner) = find_hir_expr_at(sub, offset) {
                        best = pick_smaller_span(best, inner);
                    }
                }
            }
            crate::hir::HirWhenKind::ValueMatch { value, arms } => {
                if let Some(inner) = find_hir_expr_at(value, offset) {
                    best = pick_smaller_span(best, inner);
                }
                for arm in arms {
                    if let Some(inner) = find_hir_expr_at(&arm.body, offset) {
                        best = pick_smaller_span(best, inner);
                    }
                }
            }
            crate::hir::HirWhenKind::ConditionChain { arms } => {
                for arm in arms {
                    if let Some(inner) = find_hir_expr_at(&arm.body, offset) {
                        best = pick_smaller_span(best, inner);
                    }
                }
            }
        },
        HirExprKind::For(f) => match &f.kind {
            crate::hir::HirForKind::Iterate { iterable, body, .. } => {
                if let Some(inner) = find_hir_expr_at(iterable, offset) {
                    best = pick_smaller_span(best, inner);
                }
                if let Some(inner) = find_hir_expr_at(body, offset) {
                    best = pick_smaller_span(best, inner);
                }
            }
            crate::hir::HirForKind::Condition { condition, body } => {
                if let Some(inner) = find_hir_expr_at(condition, offset) {
                    best = pick_smaller_span(best, inner);
                }
                if let Some(inner) = find_hir_expr_at(body, offset) {
                    best = pick_smaller_span(best, inner);
                }
            }
            crate::hir::HirForKind::Infinite { body } => {
                if let Some(inner) = find_hir_expr_at(body, offset) {
                    best = pick_smaller_span(best, inner);
                }
            }
            _ => {}
        },
        HirExprKind::Assign { target, value } => {
            if let Some(inner) = find_hir_expr_at(target, offset) {
                best = pick_smaller_span(best, inner);
            }
            if let Some(inner) = find_hir_expr_at(value, offset) {
                best = pick_smaller_span(best, inner);
            }
        }
        HirExprKind::FieldAccess(obj, _) => {
            if let Some(inner) = find_hir_expr_at(obj, offset) {
                best = pick_smaller_span(best, inner);
            }
        }
        HirExprKind::Index(obj, idx) => {
            if let Some(inner) = find_hir_expr_at(obj, offset) {
                best = pick_smaller_span(best, inner);
            }
            if let Some(inner) = find_hir_expr_at(idx, offset) {
                best = pick_smaller_span(best, inner);
            }
        }
        HirExprKind::Block(stmts) => {
            if let Some(inner) = find_hir_expr_at_offset(stmts, offset) {
                best = pick_smaller_span(best, inner);
            }
        }
        HirExprKind::StructLiteral(fields) => {
            for (_, v) in fields {
                if let Some(inner) = find_hir_expr_at(v, offset) {
                    best = pick_smaller_span(best, inner);
                }
            }
        }
        HirExprKind::MapLiteral(fields) => {
            for (k, v) in fields {
                if let Some(inner) = find_hir_expr_at(k, offset) {
                    best = pick_smaller_span(best, inner);
                }
                if let Some(inner) = find_hir_expr_at(v, offset) {
                    best = pick_smaller_span(best, inner);
                }
            }
        }
        HirExprKind::SetLiteral(elems) => {
            for elem in elems {
                if let Some(inner) = find_hir_expr_at(elem, offset) {
                    best = pick_smaller_span(best, inner);
                }
            }
        }
        HirExprKind::Range(lhs, rhs) => {
            if let Some(inner) = find_hir_expr_at(lhs, offset) {
                best = pick_smaller_span(best, inner);
            }
            if let Some(inner) = find_hir_expr_at(rhs, offset) {
                best = pick_smaller_span(best, inner);
            }
        }
        HirExprKind::StringInterpolate(parts) => {
            for part in parts {
                if let crate::hir::HirStringPart::Expr(e) = part {
                    if let Some(inner) = find_hir_expr_at(e, offset) {
                        best = pick_smaller_span(best, inner);
                    }
                }
            }
        }
        _ => {}
    }
    best
}

fn span_contains(span: &Span, offset: usize) -> bool {
    offset >= span.start && offset <= span.end
}
