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
