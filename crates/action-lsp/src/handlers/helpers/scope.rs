#![allow(unused_imports)]
use std::collections::HashMap;

use crate::position::{self, find_node_at, FoundNode};
use crate::symbols;
use action_frontend::ast::{Expr, ExprKind, Stmt, Type};
use action_frontend::builtin::{
    format_ufcs_method_detail, receiver_kind_from_type, ufcs_methods_for_kind,
};
use action_frontend::lexer::{Span, Token, TokenKind};
use action_frontend::typecheck::TypeRegistry;
use lsp_types::{CompletionItem, CompletionItemKind, Position, Range};

pub(crate) fn find_scope_aware_definition(
    stmts: &[Stmt],
    source: &str,
    pos: &Position,
    target_name: &str,
) -> Option<Span> {
    let target_offset = position::lsp_position_to_offset(source, pos);

    let mut walker = ScopeWalker {
        target_offset,
        target_name,
        scope_stack: vec![HashMap::new()],
        result: None,
    };

    add_stmts_to_scope(stmts, &mut walker.scope_stack[0]);
    walker.walk_stmts(stmts);
    walker.result
}

struct ScopeWalker<'a> {
    target_offset: usize,
    target_name: &'a str,
    scope_stack: Vec<HashMap<String, Span>>,
    result: Option<Span>,
}

impl<'a> ScopeWalker<'a> {
    fn enter_scope(&mut self, defs: HashMap<String, Span>) {
        self.scope_stack.push(defs);
    }

    fn exit_scope(&mut self) {
        self.scope_stack.pop();
    }

    fn lookup(&self) -> Option<Span> {
        for frame in self.scope_stack.iter().rev() {
            if let Some(span) = frame.get(self.target_name) {
                return Some(*span);
            }
        }
        None
    }

    fn contains(&self, span: &Span) -> bool {
        self.target_offset >= span.start && self.target_offset <= span.end
    }

    fn walk_stmts(&mut self, stmts: &[Stmt]) {
        for stmt in stmts {
            if self.result.is_some() {
                return;
            }
            self.walk_stmt(stmt);
        }
    }

    fn walk_stmt(&mut self, stmt: &Stmt) {
        if self.result.is_some() {
            return;
        }
        let span = stmt.span();
        if !self.contains(&span) {
            return;
        }

        match stmt {
            Stmt::Let {
                name, value, span, ..
            } => {
                self.walk_expr(value);
                if name == self.target_name {
                    self.result = Some(*span);
                }
            }
            Stmt::Destructure {
                names,
                renames,
                value,
                span,
                ..
            } => {
                self.walk_expr(value);
                for n in names {
                    if n == self.target_name {
                        self.result = Some(*span);
                        return;
                    }
                }
                for (_, local) in renames {
                    if local == self.target_name {
                        self.result = Some(*span);
                        return;
                    }
                }
            }
            Stmt::Const {
                name, value, span, ..
            } => {
                self.walk_expr(value);
                if name == self.target_name {
                    self.result = Some(*span);
                }
            }
            Stmt::Fun {
                name: fn_name,
                params,
                body,
                span,
                ..
            } => {
                if fn_name == self.target_name && self.contains(span) {
                    self.result = Some(*span);
                    return;
                }
                let mut fn_scope = HashMap::new();
                for p in params {
                    fn_scope.insert(p.name.clone(), *span);
                }
                self.enter_scope(fn_scope);
                self.walk_expr(body);
                self.exit_scope();
            }
            Stmt::Expr { expr, .. } => {
                self.walk_expr(expr);
            }
            Stmt::Return {
                value: Some(expr), ..
            } => {
                self.walk_expr(expr);
            }
            Stmt::Return { value: None, .. } => {}
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Module { body, .. } => {
                self.walk_stmts(body);
            }
            Stmt::Export { stmt: inner, .. } => {
                self.walk_stmt(inner);
            }
            Stmt::Extension { methods, .. } => {
                self.walk_stmts(methods);
            }
            Stmt::Enum { .. }
            | Stmt::TypeAlias { .. }
            | Stmt::Import { .. }
            | Stmt::External { .. }
            | Stmt::ExternalType { .. } => {}
        }
    }

    fn walk_expr(&mut self, expr: &Expr) {
        if self.result.is_some() {
            return;
        }
        match &expr.kind {
            ExprKind::Block(stmts) => {
                self.enter_scope(HashMap::new());
                self.walk_stmts(stmts);
                self.exit_scope();
            }
            ExprKind::Call {
                func,
                args,
                trailing_lambda,
                ..
            } => {
                self.walk_expr(func);
                for a in args {
                    self.walk_expr(a);
                }
                if let Some(lam) = trailing_lambda {
                    self.walk_expr(lam);
                }
            }
            ExprKind::Lambda { params, body, .. } => {
                let mut lam_scope = HashMap::new();
                for p in params {
                    lam_scope.insert(p.clone(), Span::default());
                }
                self.enter_scope(lam_scope);
                self.walk_expr(body);
                self.exit_scope();
            }
            ExprKind::Binary(lhs, _, rhs) => {
                self.walk_expr(lhs);
                self.walk_expr(rhs);
            }
            ExprKind::Unary(_, inner) => self.walk_expr(inner),
            ExprKind::FieldAccess(obj, _) => self.walk_expr(obj),
            ExprKind::Index(obj, idx) => {
                self.walk_expr(obj);
                self.walk_expr(idx);
            }
            ExprKind::When(w) => match &w.kind {
                action_frontend::ast::WhenKind::OneLine {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    self.walk_expr(&condition);
                    self.walk_expr(&then_expr);
                    self.walk_expr(&else_expr);
                }
                action_frontend::ast::WhenKind::ValueMatch { value, arms } => {
                    self.walk_expr(&value);
                    for arm in arms {
                        let mut arm_scope = HashMap::new();
                        collect_pattern_bindings(&arm.pattern, &mut arm_scope);
                        self.enter_scope(arm_scope);
                        self.walk_expr(&arm.body);
                        self.exit_scope();
                    }
                }
                action_frontend::ast::WhenKind::ConditionChain { arms } => {
                    for arm in arms {
                        let mut arm_scope = HashMap::new();
                        collect_pattern_bindings(&arm.pattern, &mut arm_scope);
                        if let Some(guard) = &arm.guard {
                            self.walk_expr(guard);
                        }
                        self.enter_scope(arm_scope);
                        self.walk_expr(&arm.body);
                        self.exit_scope();
                    }
                }
            },
            ExprKind::For(fr) => match &fr.kind {
                action_frontend::ast::ForKind::Iterate {
                    var,
                    iterable,
                    body,
                    ..
                } => {
                    self.walk_expr(&iterable);
                    let mut for_scope = HashMap::new();
                    for_scope.insert(var.clone(), Span::default());
                    self.enter_scope(for_scope);
                    self.walk_expr(&body);
                    self.exit_scope();
                }
                action_frontend::ast::ForKind::IterateWithIndex {
                    vars,
                    iterable,
                    body,
                } => {
                    self.walk_expr(&iterable);
                    let mut for_scope = HashMap::new();
                    for v in vars {
                        for_scope.insert(v.clone(), Span::default());
                    }
                    self.enter_scope(for_scope);
                    self.walk_expr(&body);
                    self.exit_scope();
                }
                action_frontend::ast::ForKind::NestedIterate { bindings, body, .. } => {
                    for (_, iter) in bindings {
                        self.walk_expr(&iter);
                    }
                    let mut for_scope = HashMap::new();
                    for (v, _) in bindings {
                        for_scope.insert(v.clone(), Span::default());
                    }
                    self.enter_scope(for_scope);
                    self.walk_expr(&body);
                    self.exit_scope();
                }
                action_frontend::ast::ForKind::Condition { condition, body } => {
                    self.walk_expr(&condition);
                    self.walk_expr(&body);
                }
                action_frontend::ast::ForKind::Infinite { body } => {
                    self.walk_expr(&body);
                }
            },
            ExprKind::Assign { target, value } => {
                self.walk_expr(&target);
                self.walk_expr(&value);
            }
            ExprKind::OrBlock { fallible, fallback } => {
                self.walk_expr(&fallible);
                self.walk_expr(&fallback);
            }
            ExprKind::Tuple(items) => {
                for (_, e) in items {
                    self.walk_expr(&e);
                }
            }
            ExprKind::StructLiteral { fields, .. } => {
                for (_, e) in fields {
                    self.walk_expr(&e);
                }
            }
            ExprKind::MapLiteral(entries) => {
                for (k, v) in entries {
                    self.walk_expr(k);
                    self.walk_expr(v);
                }
            }
            ExprKind::SetLiteral(elements) => {
                for e in elements {
                    self.walk_expr(e);
                }
            }
            ExprKind::Range(start, end) => {
                self.walk_expr(start);
                self.walk_expr(end);
            }
            ExprKind::Unsafe(inner) => self.walk_expr(inner),
            ExprKind::Copy(inner) => self.walk_expr(inner),
            ExprKind::StringInterpolate(parts) => {
                for part in parts {
                    if let action_frontend::ast::StringPart::Expr(e) = part {
                        self.walk_expr(&e);
                    }
                }
            }
            ExprKind::Ident(name) => {
                if name == self.target_name && self.result.is_none() {
                    if let Some(span) = self.lookup() {
                        self.result = Some(span);
                    }
                }
            }
            ExprKind::Literal(_)
            | ExprKind::Continue
            | ExprKind::Break
            | ExprKind::FunctionRef(_) => {}
        }
    }
}

pub(crate) fn collect_pattern_bindings(
    pattern: &action_frontend::ast::Pattern,
    map: &mut HashMap<String, Span>,
) {
    use action_frontend::ast::Pattern;
    match pattern {
        Pattern::Variable(name) => {
            map.insert(name.clone(), Span::default());
        }
        Pattern::Constructor {
            args, named_fields, ..
        } => {
            for arg in args {
                collect_pattern_bindings(arg, map);
            }
            for (_, p) in named_fields {
                collect_pattern_bindings(p, map);
            }
        }
        Pattern::Or(patterns) => {
            for p in patterns {
                collect_pattern_bindings(p, map);
            }
        }
        Pattern::Tuple(patterns) => {
            for p in patterns {
                collect_pattern_bindings(p, map);
            }
        }
        _ => {}
    }
}

pub(crate) fn add_stmts_to_scope(stmts: &[Stmt], scope_map: &mut HashMap<String, Span>) {
    for stmt in stmts {
        add_stmt_to_scope(stmt, scope_map);
    }
}

pub(crate) fn add_stmt_to_scope(stmt: &Stmt, scope_map: &mut HashMap<String, Span>) {
    match stmt {
        Stmt::Fun { name, span, .. } => {
            scope_map.insert(name.clone(), *span);
        }
        Stmt::Let { name, span, .. } => {
            scope_map.insert(name.clone(), *span);
        }
        Stmt::Const { name, span, .. } => {
            scope_map.insert(name.clone(), *span);
        }
        Stmt::Enum {
            name,
            variants,
            span,
            ..
        } => {
            scope_map.insert(name.clone(), *span);
            for v in variants {
                scope_map.insert(v.name.clone(), *span);
            }
        }
        Stmt::TypeAlias { name, span, .. } => {
            scope_map.insert(name.clone(), *span);
        }
        Stmt::Module {
            name, body, span, ..
        } => {
            scope_map.insert(name.clone(), *span);
            add_stmts_to_scope(body, scope_map);
        }
        Stmt::Destructure {
            names,
            renames,
            span,
            ..
        } => {
            for n in names {
                scope_map.insert(n.clone(), *span);
            }
            for (_, local) in renames {
                scope_map.insert(local.clone(), *span);
            }
        }
        Stmt::Extension { methods, .. } => {
            add_stmts_to_scope(methods, scope_map);
        }
        _ => {}
    }
}
