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

use super::completion::named_type_key;

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
