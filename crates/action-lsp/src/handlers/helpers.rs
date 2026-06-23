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
// ============================================================
//  FEATURE 1: Member completion after `.` / `::`
// ============================================================

pub(crate) fn member_completion_items(
    tokens: &[Token],
    source: &str,
    pos: &Position,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
    file_registry: &TypeRegistry,
    stdlib_registry: &TypeRegistry,
    prefix: &str,
) -> Option<Vec<CompletionItem>> {
    let offset = position::lsp_position_to_offset(source, pos);

    // Find the last token at or before the cursor
    let cursor_idx = tokens.iter().rposition(|t| t.span.end <= offset)?;

    // If the token at cursor is an Ident (partial word), skip it to look for the `.`
    let sep_idx = if matches!(tokens[cursor_idx].kind, TokenKind::Ident(_)) && cursor_idx > 0 {
        cursor_idx - 1
    } else {
        cursor_idx
    };

    let is_dot = match &tokens[sep_idx].kind {
        TokenKind::Dot => true,
        TokenKind::ColonColon => false,
        _ => return None,
    };

    if sep_idx == 0 {
        return None;
    }
    let receiver_idx = sep_idx - 1;

    let receiver_name = match &tokens[receiver_idx].kind {
        TokenKind::Ident(name) => name.clone(),
        _ => return None,
    };

    let receiver_type = type_env
        .get(&receiver_name)
        .or_else(|| stdlib_type_env.get(&receiver_name))?;

    let items = if is_dot {
        dot_member_items(
            receiver_type,
            prefix,
            type_env,
            stdlib_type_env,
            file_registry,
            stdlib_registry,
        )
    } else {
        colon_member_items(receiver_type, prefix, file_registry, stdlib_registry)
    };

    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

pub(crate) fn dot_member_items(
    receiver_type: &Type,
    prefix: &str,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
    file_registry: &TypeRegistry,
    stdlib_registry: &TypeRegistry,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let mut seen = std::collections::HashSet::new();

    if let Type::Nullable(inner) = receiver_type {
        if "or".starts_with(prefix) {
            items.push(CompletionItem {
                label: "or".to_string(),
                detail: Some("or { fallback } -> T".to_string()),
                kind: Some(CompletionItemKind::METHOD),
                ..Default::default()
            });
        }
        items.extend(dot_member_items(
            inner,
            prefix,
            type_env,
            stdlib_type_env,
            file_registry,
            stdlib_registry,
        ));
        return items;
    }

    if let Some(kind) = receiver_kind_from_type(receiver_type) {
        for def in ufcs_methods_for_kind(kind) {
            if def.name.starts_with(prefix) && seen.insert(def.name.to_string()) {
                items.push(CompletionItem {
                    label: def.name.to_string(),
                    detail: Some(format_ufcs_method_detail(def)),
                    kind: Some(CompletionItemKind::METHOD),
                    ..Default::default()
                });
            }
        }
    }

    push_extension_methods(
        &mut items,
        &mut seen,
        receiver_type,
        prefix,
        type_env,
        stdlib_type_env,
    );

    if let Some(type_name) = named_type_key(receiver_type) {
        for registry in [file_registry, stdlib_registry] {
            if let Some(st) = registry.get_struct(&type_name) {
                for (field, fty) in &st.fields {
                    if field.starts_with(prefix) && seen.insert(format!("field:{field}")) {
                        items.push(CompletionItem {
                            label: field.clone(),
                            detail: Some(format!("{}: {}", field, fty)),
                            kind: Some(CompletionItemKind::FIELD),
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }

    match receiver_type {
        Type::Stream(_) => push_method_labels(
            &mut items,
            &mut seen,
            prefix,
            &[
                ("send", "send(value)"),
                ("receive", "receive() -> T"),
                ("close", "close()"),
            ],
        ),
        Type::Task(_) => push_method_labels(
            &mut items,
            &mut seen,
            prefix,
            &[
                ("cancel", "cancel()"),
                ("is_done", "is_done() -> Bool"),
                ("is_cancelled", "is_cancelled() -> Bool"),
                ("wait", "wait() -> T"),
            ],
        ),
        _ => {}
    }

    items
}

pub(crate) fn colon_member_items(
    receiver_type: &Type,
    prefix: &str,
    file_registry: &TypeRegistry,
    stdlib_registry: &TypeRegistry,
) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    let Some(type_name) = named_type_key(receiver_type) else {
        return items;
    };

    for registry in [file_registry, stdlib_registry] {
        if let Some(enum_info) = registry.enums.get(&type_name) {
            for variant in &enum_info.variants {
                if variant.name.starts_with(prefix) {
                    items.push(CompletionItem {
                        label: variant.name.clone(),
                        detail: Some(format!("enum {}::{}", type_name, variant.name)),
                        kind: Some(CompletionItemKind::ENUM_MEMBER),
                        ..Default::default()
                    });
                }
            }
        }
    }
    items
}

pub(crate) fn named_type_key(ty: &Type) -> Option<String> {
    match ty {
        Type::Named(name) => Some(name.clone()),
        Type::Generic(base, _) => named_type_key(base),
        Type::LazyList(inner) => named_type_key(inner),
        _ => None,
    }
}

pub(crate) fn push_extension_methods(
    items: &mut Vec<CompletionItem>,
    seen: &mut std::collections::HashSet<String>,
    receiver_type: &Type,
    prefix: &str,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
) {
    let Some(type_name) = named_type_key(receiver_type) else {
        return;
    };
    let lookup_prefix = format!("{type_name}.");
    for env in [type_env, stdlib_type_env] {
        for (key, fn_ty) in env {
            if let Some(method) = key.strip_prefix(&lookup_prefix) {
                if !method.contains('.') && method.starts_with(prefix) && seen.insert(key.clone()) {
                    items.push(CompletionItem {
                        label: method.to_string(),
                        detail: Some(format_method_type(method, fn_ty)),
                        kind: Some(CompletionItemKind::METHOD),
                        ..Default::default()
                    });
                }
            }
        }
    }
}

pub(crate) fn format_method_type(name: &str, ty: &Type) -> String {
    match ty {
        Type::Function(params, ret) => {
            let ps: Vec<String> = params.iter().map(|p| format!("{}", p)).collect();
            format!("{}({}) -> {}", name, ps.join(", "), ret)
        }
        other => format!("{}: {}", name, other),
    }
}

pub(crate) fn push_method_labels(
    items: &mut Vec<CompletionItem>,
    seen: &mut std::collections::HashSet<String>,
    prefix: &str,
    methods: &[(&str, &str)],
) {
    for (name, detail) in methods {
        if name.starts_with(prefix) && seen.insert(name.to_string()) {
            items.push(CompletionItem {
                label: name.to_string(),
                detail: Some(detail.to_string()),
                kind: Some(CompletionItemKind::METHOD),
                ..Default::default()
            });
        }
    }
}

// ============================================================
//  FEATURE 2: AST scope-aware definition lookup
// ============================================================

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
            ExprKind::OrBlock { nullable, fallback } => {
                self.walk_expr(&nullable);
                self.walk_expr(&fallback);
            }
            ExprKind::Tuple(items) => {
                for (_, e) in items {
                    self.walk_expr(&e);
                }
            }
            ExprKind::StructLiteral(fields) => {
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
            | ExprKind::Null
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

// ============================================================
//  FEATURE 3: Doc comment extraction + function signature
// ============================================================

pub(crate) fn extract_doc_comment(source: &str, ast: &[Stmt], name: &str) -> Option<String> {
    let def_span = find_stmt_span_for_name(ast, name)?;
    let start = def_span.start;

    let before = &source[..start];
    let lines: Vec<&str> = before.lines().rev().collect();

    let mut comments: Vec<String> = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if let Some(content) = trimmed.strip_prefix("///") {
            comments.push(content.trim().to_string());
        } else if let Some(content) = trimmed.strip_prefix("//") {
            comments.push(content.trim().to_string());
        } else if trimmed.is_empty() {
            if comments.is_empty() {
                continue;
            } else {
                break;
            }
        } else {
            break;
        }
    }

    if comments.is_empty() {
        return None;
    }

    comments.reverse();
    Some(comments.join("\n"))
}

pub(crate) fn find_stmt_span_for_name(stmts: &[Stmt], name: &str) -> Option<Span> {
    for stmt in stmts {
        match stmt {
            Stmt::Fun { name: n, span, .. } if n == name => return Some(*span),
            Stmt::Let { name: n, span, .. } if n == name => return Some(*span),
            Stmt::Const { name: n, span, .. } if n == name => return Some(*span),
            Stmt::Enum { name: n, span, .. } if n == name => return Some(*span),
            Stmt::TypeAlias { name: n, span, .. } if n == name => return Some(*span),
            Stmt::Module {
                name: n,
                body,
                span,
                ..
            } => {
                if n == name {
                    return Some(*span);
                }
                if let Some(inner) = find_stmt_span_for_name(body, name) {
                    return Some(inner);
                }
            }
            _ => {}
        }
    }
    None
}

pub(crate) fn lookup_type_in_envs<'a>(
    name: &str,
    type_env: &'a HashMap<String, Type>,
    stdlib_type_env: &'a HashMap<String, Type>,
) -> Option<&'a Type> {
    type_env.get(name).or_else(|| stdlib_type_env.get(name))
}

pub(crate) fn push_env_completion_items(
    items: &mut Vec<CompletionItem>,
    seen: &mut std::collections::HashSet<String>,
    prefix: &str,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
) {
    for env in [type_env, stdlib_type_env] {
        for (name, ty) in env {
            if !name.starts_with(prefix) || !seen.insert(name.clone()) {
                continue;
            }
            let kind = match ty {
                Type::Function(..) => CompletionItemKind::FUNCTION,
                Type::Named(n) if n.chars().next().is_some_and(|c| c.is_uppercase()) => {
                    CompletionItemKind::CLASS
                }
                _ => CompletionItemKind::VARIABLE,
            };
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(kind),
                detail: Some(format!("{}", ty)),
                ..Default::default()
            });
        }
    }
}

pub(crate) fn lookup_function_signature(
    name: &str,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
    hir: Option<&action_frontend::hir::HirModule>,
) -> Option<String> {
    let func_type = type_env.get(name).or_else(|| stdlib_type_env.get(name))?;
    match func_type {
        Type::Function(param_types, ret_type) => {
            let param_names = hir.and_then(|h| crate::hir_lookup::find_call_param_names(h, name));
            Some(format_function_signature(
                name,
                param_types,
                ret_type.as_ref(),
                param_names.as_deref(),
            ))
        }
        _ => None,
    }
}

pub(crate) fn format_param_label(i: usize, ty: &Type, param_names: Option<&[String]>) -> String {
    if let Some(names) = param_names {
        if let Some(n) = names.get(i) {
            return format!("{}: {}", n, ty);
        }
    }
    format!("p{}: {}", i, ty)
}

pub(crate) fn format_function_signature(
    name: &str,
    param_types: &[Type],
    ret_type: &Type,
    param_names: Option<&[String]>,
) -> String {
    let params_str: Vec<String> = param_types
        .iter()
        .enumerate()
        .map(|(i, t)| format_param_label(i, t, param_names))
        .collect();
    format!("fun {}({}) -> {}", name, params_str.join(", "), ret_type)
}

pub(crate) fn resolve_call_type(
    tokens: &[action_frontend::lexer::Token],
    source: &str,
    pos: &Position,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
) -> Option<(String, Type)> {
    let method_name = find_call_target(tokens, source, pos)?;

    if let Some(ty) = lookup_type(type_env, stdlib_type_env, &method_name) {
        return Some((method_name, ty.clone()));
    }

    if let Some(key) = find_ufcs_type_env_key(tokens, source, pos, type_env, stdlib_type_env) {
        if let Some(ty) = lookup_type(type_env, stdlib_type_env, &key) {
            return Some((key, ty.clone()));
        }
    }

    None
}

pub(crate) fn lookup_type<'a>(
    type_env: &'a HashMap<String, Type>,
    stdlib_type_env: &'a HashMap<String, Type>,
    key: &str,
) -> Option<&'a Type> {
    type_env.get(key).or_else(|| stdlib_type_env.get(key))
}

pub(crate) fn find_ufcs_type_env_key(
    tokens: &[action_frontend::lexer::Token],
    source: &str,
    pos: &Position,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
) -> Option<String> {
    let offset = position::lsp_position_to_offset(source, pos);
    let method_name = find_call_target(tokens, source, pos)?;

    let method_idx = tokens.iter().position(|t| {
        matches!(&t.kind, TokenKind::Ident(n) if n == &method_name)
            && t.span.start <= offset
            && offset <= t.span.end + 1
    })?;
    if method_idx == 0 {
        return None;
    }
    let dot_idx = method_idx - 1;
    if !matches!(tokens[dot_idx].kind, TokenKind::Dot) {
        return None;
    }
    if dot_idx == 0 {
        return None;
    }
    let receiver_name = match &tokens[dot_idx - 1].kind {
        TokenKind::Ident(name) => name.clone(),
        _ => return None,
    };
    let receiver_type = lookup_type(type_env, stdlib_type_env, &receiver_name)?;
    let type_name = named_type_key(receiver_type)?;
    Some(format!("{type_name}.{method_name}"))
}

// ---- Helpers ----

pub(crate) fn get_word_prefix(source: &str, pos: &Position) -> String {
    let offset = position::lsp_position_to_offset(source, pos);
    let before = &source[..offset.min(source.len())];
    before
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect::<String>()
        .chars()
        .rev()
        .collect()
}

pub(crate) fn find_call_target(
    tokens: &[action_frontend::lexer::Token],
    source: &str,
    pos: &Position,
) -> Option<String> {
    let offset = position::lsp_position_to_offset(source, pos);

    let mut depth = 0;
    let mut found_ident = None;

    for token in tokens.iter().rev() {
        if token.span.start > offset {
            continue;
        }

        match &token.kind {
            TokenKind::LParen => {
                if depth == 0 {
                    return found_ident;
                }
                depth -= 1;
            }
            TokenKind::RParen => depth += 1,
            TokenKind::Ident(name) if depth == 0 => {
                found_ident = Some(name.clone());
            }
            _ if depth == 0 => {
                return None;
            }
            _ => {}
        }
    }

    found_ident
}

#[cfg(test)]
mod tests {
    use super::super::document::{
        handle_did_change, handle_did_close, handle_did_open, handle_formatting,
    };
    use super::super::editing::handle_completion;
    use super::super::navigation::{handle_goto_definition, handle_hover, handle_references};
    use super::super::symbols::{
        handle_document_symbols, handle_semantic_tokens, handle_workspace_symbol,
    };
    use super::super::ServerState;
    use super::*;
    use crate::project::Project;
    use action_frontend::typecheck::TypeRegistry;
    use lsp_types::Url;
    use lsp_types::{
        CompletionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DocumentFormattingParams, DocumentSymbolParams,
        GotoDefinitionParams, HoverParams, ReferenceParams, SemanticTokensParams,
        TextDocumentContentChangeEvent, TextDocumentItem, TextDocumentPositionParams,
        VersionedTextDocumentIdentifier, WorkspaceSymbolParams,
    };
    use std::collections::HashMap;

    fn make_state(source: &str) -> ServerState {
        let proj = Project::with_stdlib(TypeRegistry::new(), HashMap::new(), Vec::new());
        let mut state = ServerState::new(proj);
        let uri = Url::parse("file:///test.ac").unwrap();
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "action".to_string(),
                version: 1,
                text: source.to_string(),
            },
        };
        handle_did_open(&mut state, params);
        state
    }

    fn test_uri() -> Url {
        Url::parse("file:///test.ac").unwrap()
    }

    fn state_with_proj(proj: Project) -> ServerState {
        ServerState::new(proj)
    }

    #[test]
    fn test_handle_did_open() {
        let mut state = state_with_proj(Project::with_stdlib(
            TypeRegistry::new(),
            HashMap::new(),
            Vec::new(),
        ));
        let uri = test_uri();
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "action".to_string(),
                version: 1,
                text: "val x = 42".to_string(),
            },
        };
        let diags = handle_did_open(&mut state, params);
        assert!(diags.is_empty(), "valid program should have no diagnostics");
        assert!(state.project.documents.contains_key(&uri));
    }

    #[test]
    fn test_handle_did_change() {
        let mut state = make_state("val x = 42");
        let uri = test_uri();
        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: 2,
            },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: "val y = 100".to_string(),
            }],
        };
        let diags = handle_did_change(&mut state, params);
        assert!(diags.is_empty(), "changed program should be valid");
        let doc = state.project.documents.get(&uri).unwrap();
        assert!(doc.definition_map.contains_key("y"));
    }

    #[test]
    fn test_handle_did_close() {
        let mut state = make_state("val x = 42");
        let uri = test_uri();
        assert!(state.project.documents.contains_key(&uri));
        let params = DidCloseTextDocumentParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
        };
        handle_did_close(&mut state, params);
        assert!(!state.project.documents.contains_key(&uri));
    }

    #[test]
    fn test_handle_hover_on_ident() {
        let state = make_state("val x = 42");
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier { uri: test_uri() },
                position: Position {
                    line: 0,
                    character: 4,
                },
            },
            work_done_progress_params: Default::default(),
        };
        let hover = handle_hover(&state, params);
        // Should return hover info for 'x'
        assert!(hover.is_some(), "should have hover info for 'x'");
    }

    #[test]
    fn test_handle_hover_on_keyword() {
        let state = make_state("val x = 42");
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier { uri: test_uri() },
                position: Position {
                    line: 0,
                    character: 0,
                },
            },
            work_done_progress_params: Default::default(),
        };
        let hover = handle_hover(&state, params);
        // Should return hover info for 'val' keyword
        assert!(hover.is_some(), "should have hover info for 'val' keyword");
    }

    #[test]
    fn test_handle_hover_outside_range() {
        let state = make_state("val x = 42");
        let params = HoverParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier { uri: test_uri() },
                position: Position {
                    line: 99,
                    character: 99,
                },
            },
            work_done_progress_params: Default::default(),
        };
        let hover = handle_hover(&state, params);
        assert!(hover.is_none(), "out-of-range position should return None");
    }

    #[test]
    fn test_handle_goto_definition() {
        let state = make_state("fun hello() {}\nval y = hello");
        let uri = test_uri();
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 1,
                    character: 8,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let result = handle_goto_definition(&state, params);
        assert!(result.is_some(), "should find definition for 'hello'");
    }

    #[test]
    fn test_handle_goto_definition_unknown() {
        let state = make_state("val x = 1");
        let uri = test_uri();
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 0,
                    character: 8,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let result = handle_goto_definition(&state, params);
        // Position 8 is at '=' or '1', not an identifier - may or may not find
        // Just check it doesn't panic
        let _ = result;
    }

    #[test]
    fn test_handle_completion_keyword() {
        let state = make_state("val x = ");
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier { uri: test_uri() },
                position: Position {
                    line: 0,
                    character: 8,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: None,
        };
        let result = handle_completion(&state, params);
        // Should return completion items including keywords
        assert!(result.is_some(), "completion should return items");
    }

    #[test]
    fn test_handle_semantic_tokens() {
        let state = make_state("val x = 42");
        let params = SemanticTokensParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: test_uri() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let result = handle_semantic_tokens(&state, params);
        assert!(result.is_some(), "semantic tokens should be returned");
    }

    #[test]
    fn test_handle_document_symbols() {
        let state = make_state("fun hello() {}\nval x = 42");
        let params = DocumentSymbolParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: test_uri() },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let result = handle_document_symbols(&state, params);
        assert!(result.is_some(), "document symbols should be returned");
    }

    #[test]
    fn test_handle_references() {
        let state = make_state("val x = 1\nval y = x + 2");
        let uri = test_uri();
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier { uri: uri.clone() },
                position: Position {
                    line: 0,
                    character: 4,
                },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
            context: lsp_types::ReferenceContext {
                include_declaration: true,
            },
        };
        let result = handle_references(&state, params);
        assert!(result.is_some(), "references should be found for 'x'");
        let refs = result.unwrap();
        assert!(!refs.is_empty(), "should have at least one reference");
    }

    #[test]
    fn test_handle_workspace_symbol() {
        let state = make_state("fun myFunction() {}");
        let params = WorkspaceSymbolParams {
            query: "myFunc".to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let result = handle_workspace_symbol(&state, params);
        assert!(result.is_some(), "should find workspace symbol");
        let symbols = result.unwrap();
        assert!(!symbols.is_empty(), "should match 'myFunction'");
    }

    #[test]
    fn test_handle_workspace_symbol_no_match() {
        let state = make_state("fun hello() {}");
        let params = WorkspaceSymbolParams {
            query: "zzzzz".to_string(),
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };
        let result = handle_workspace_symbol(&state, params);
        assert!(result.is_some(), "should return empty result, not None");
        let symbols = result.unwrap();
        assert!(symbols.is_empty());
    }

    #[test]
    fn test_handle_formatting() {
        let state = make_state("val x = 1");
        let params = DocumentFormattingParams {
            text_document: lsp_types::TextDocumentIdentifier { uri: test_uri() },
            options: lsp_types::FormattingOptions {
                tab_size: 4,
                insert_spaces: true,
                ..Default::default()
            },
            work_done_progress_params: Default::default(),
        };
        let result = handle_formatting(&state, params);
        // Formatting may return None if not supported, just check no panic
        let _ = result;
    }
}
