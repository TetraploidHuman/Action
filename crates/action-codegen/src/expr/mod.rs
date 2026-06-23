//! Expression codegen submodule tree.

// Submodule: expr

use super::{llvm_err, CodeGen, InnerType, Scope, TypedValue, ValKind};

mod binop;
mod coerce;
mod fat_return;
mod lambda;
mod literal;

pub(crate) fn collect_free_vars_hir(
    expr: &action_frontend::hir::HirExpr,
    params: &[String],
    bound: &mut Vec<String>,
    free: &mut Vec<String>,
) {
    use action_frontend::hir::{HirExprKind, HirStringPart};
    match &expr.kind {
        HirExprKind::Ident(name) => {
            if !params.contains(name) && !bound.contains(name) && !free.contains(name) {
                free.push(name.clone());
            }
        }
        HirExprKind::Lambda {
            params: inner_params,
            body,
            ..
        } => {
            let mut inner_bound = bound.clone();
            inner_bound.extend(inner_params.iter().cloned());
            collect_free_vars_hir(body.as_ref(), inner_params, &mut inner_bound, free);
        }
        HirExprKind::Block(stmts) => {
            for stmt in stmts {
                visit_hir_stmt_free_vars(stmt, params, bound, free);
            }
        }
        HirExprKind::Binary(lhs, _, rhs) => {
            collect_free_vars_hir(lhs, params, bound, free);
            collect_free_vars_hir(rhs, params, bound, free);
        }
        HirExprKind::Unary(_, e) => collect_free_vars_hir(e, params, bound, free),
        HirExprKind::Call {
            func,
            args,
            trailing_lambda,
        } => {
            collect_free_vars_hir(func, params, bound, free);
            for a in args {
                collect_free_vars_hir(a, params, bound, free);
            }
            if let Some(t) = trailing_lambda {
                collect_free_vars_hir(t, params, bound, free);
            }
        }
        HirExprKind::FieldAccess(e, _) => collect_free_vars_hir(e, params, bound, free),
        HirExprKind::Index(target, index) => {
            collect_free_vars_hir(target, params, bound, free);
            collect_free_vars_hir(index, params, bound, free);
        }
        HirExprKind::When(w) => visit_hir_when_free_vars(w, params, bound, free),
        HirExprKind::For(f) => visit_hir_for_free_vars(f, params, bound, free),
        HirExprKind::StructLiteral(fields) => {
            for (_, v) in fields {
                collect_free_vars_hir(v, params, bound, free);
            }
        }
        HirExprKind::SetLiteral(elems) => {
            for e in elems {
                collect_free_vars_hir(e, params, bound, free);
            }
        }
        HirExprKind::MapLiteral(pairs) => {
            for (k, v) in pairs {
                collect_free_vars_hir(k, params, bound, free);
                collect_free_vars_hir(v, params, bound, free);
            }
        }
        HirExprKind::Null => {}
        HirExprKind::OrBlock { nullable, fallback } => {
            collect_free_vars_hir(nullable, params, bound, free);
            collect_free_vars_hir(fallback, params, bound, free);
        }
        HirExprKind::Range(start, end) => {
            collect_free_vars_hir(start, params, bound, free);
            collect_free_vars_hir(end, params, bound, free);
        }
        HirExprKind::Tuple(fields) => {
            for (_, e) in fields {
                collect_free_vars_hir(e, params, bound, free);
            }
        }
        HirExprKind::Assign { target, value } => {
            collect_free_vars_hir(target, params, bound, free);
            collect_free_vars_hir(value, params, bound, free);
        }
        HirExprKind::Copy(e) | HirExprKind::Unsafe(e) => {
            collect_free_vars_hir(e, params, bound, free);
        }
        HirExprKind::StringInterpolate(parts) => {
            for p in parts {
                if let HirStringPart::Expr(e) = p {
                    collect_free_vars_hir(e, params, bound, free);
                }
            }
        }
        HirExprKind::Literal(_)
        | HirExprKind::FunctionRef(_)
        | HirExprKind::Break
        | HirExprKind::Continue => {}
    }
}

fn visit_hir_when_free_vars(
    w: &action_frontend::hir::HirWhen,
    params: &[String],
    bound: &mut Vec<String>,
    free: &mut Vec<String>,
) {
    use action_frontend::hir::HirWhenKind;
    match &w.kind {
        HirWhenKind::OneLine {
            condition,
            then_expr,
            else_expr,
        } => {
            collect_free_vars_hir(condition, params, bound, free);
            collect_free_vars_hir(then_expr, params, bound, free);
            collect_free_vars_hir(else_expr, params, bound, free);
        }
        HirWhenKind::ValueMatch { value, arms } => {
            collect_free_vars_hir(value, params, bound, free);
            for arm in arms {
                let pat_vars = collect_hir_pattern_vars(&arm.pattern);
                bound.extend(pat_vars.iter().cloned());
                if let Some(guard) = &arm.guard {
                    collect_free_vars_hir(guard, params, bound, free);
                }
                collect_free_vars_hir(arm.body.as_ref(), params, bound, free);
                for _ in &pat_vars {
                    bound.pop();
                }
            }
        }
        HirWhenKind::ConditionChain { arms } => {
            for arm in arms {
                let pat_vars = collect_hir_pattern_vars(&arm.pattern);
                bound.extend(pat_vars.iter().cloned());
                if let Some(guard) = &arm.guard {
                    collect_free_vars_hir(guard, params, bound, free);
                }
                collect_free_vars_hir(arm.body.as_ref(), params, bound, free);
                for _ in &pat_vars {
                    bound.pop();
                }
            }
        }
    }
}

fn visit_hir_for_free_vars(
    f: &action_frontend::hir::HirFor,
    params: &[String],
    bound: &mut Vec<String>,
    free: &mut Vec<String>,
) {
    use action_frontend::hir::HirForKind;
    match &f.kind {
        HirForKind::Iterate {
            var,
            iterable,
            body,
            ..
        } => {
            collect_free_vars_hir(iterable, params, bound, free);
            bound.push(var.clone());
            collect_free_vars_hir(body.as_ref(), params, bound, free);
            bound.pop();
        }
        HirForKind::IterateWithIndex {
            vars,
            iterable,
            body,
        } => {
            collect_free_vars_hir(iterable, params, bound, free);
            for v in vars {
                bound.push(v.clone());
            }
            collect_free_vars_hir(body.as_ref(), params, bound, free);
            for _ in vars {
                bound.pop();
            }
        }
        HirForKind::NestedIterate { bindings, body, .. } => {
            for (var, iter) in bindings {
                collect_free_vars_hir(iter, params, bound, free);
                bound.push(var.clone());
            }
            collect_free_vars_hir(body.as_ref(), params, bound, free);
            for _ in bindings {
                bound.pop();
            }
        }
        HirForKind::Condition { condition, body } => {
            collect_free_vars_hir(condition, params, bound, free);
            collect_free_vars_hir(body.as_ref(), params, bound, free);
        }
        HirForKind::Infinite { body } => collect_free_vars_hir(body, params, bound, free),
    }
}

fn visit_hir_stmt_free_vars(
    stmt: &action_frontend::hir::HirStmt,
    params: &[String],
    bound: &mut Vec<String>,
    free: &mut Vec<String>,
) {
    use action_frontend::hir::HirStmt;
    match stmt {
        HirStmt::Let { name, value, .. } => {
            collect_free_vars_hir(value, params, bound, free);
            bound.push(name.clone());
        }
        HirStmt::Destructure {
            names,
            renames,
            rest,
            value,
            ..
        } => {
            collect_free_vars_hir(value, params, bound, free);
            for n in names {
                bound.push(n.clone());
            }
            for (_, to) in renames {
                bound.push(to.clone());
            }
            if let Some(r) = rest {
                bound.push(r.clone());
            }
        }
        HirStmt::Expr { expr, .. } => collect_free_vars_hir(expr, params, bound, free),
        HirStmt::Return { value, .. } => {
            if let Some(e) = value {
                collect_free_vars_hir(e, params, bound, free);
            }
        }
        HirStmt::Fun { .. }
        | HirStmt::TypeAlias { .. }
        | HirStmt::Enum { .. }
        | HirStmt::Module { .. }
        | HirStmt::Export { .. }
        | HirStmt::Import { .. }
        | HirStmt::Const { .. }
        | HirStmt::Extension { .. }
        | HirStmt::External { .. }
        | HirStmt::ExternalType { .. } => {}
        HirStmt::Break { .. } | HirStmt::Continue { .. } => {}
    }
}

fn collect_hir_pattern_vars(pat: &action_frontend::hir::HirPattern) -> Vec<String> {
    use action_frontend::hir::HirPattern;
    match pat {
        HirPattern::Variable(name) => vec![name.clone()],
        HirPattern::Constructor {
            args, named_fields, ..
        } => {
            let mut v: Vec<String> = args.iter().flat_map(collect_hir_pattern_vars).collect();
            for (_, p) in named_fields {
                v.extend(collect_hir_pattern_vars(p));
            }
            v
        }
        HirPattern::Or(pats) => pats.iter().flat_map(collect_hir_pattern_vars).collect(),
        HirPattern::Tuple(pats) => pats.iter().flat_map(collect_hir_pattern_vars).collect(),
        HirPattern::Expr(_)
        | HirPattern::Null
        | HirPattern::Wildcard
        | HirPattern::Literal(_)
        | HirPattern::Range(_, _)
        | HirPattern::IsType(_) => vec![],
    }
}
