#![allow(unused_imports)]
use std::collections::HashMap;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionParams, CodeActionResponse, CompletionItem,
    CompletionItemKind, CompletionParams, CompletionResponse, Diagnostic,
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DocumentFormattingParams, DocumentHighlight, DocumentHighlightKind, DocumentHighlightParams,
    DocumentSymbolParams, DocumentSymbolResponse, FoldingRange, FoldingRangeParams,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverContents, HoverParams, InlayHint,
    InlayHintParams, Location, MarkupContent, MarkupKind, Position, PrepareRenameResponse, Range,
    ReferenceParams, RenameParams, SemanticTokensParams, SemanticTokensResult, SignatureHelp,
    SignatureHelpParams, TextDocumentPositionParams, TextEdit, Url, WorkspaceEdit,
    WorkspaceSymbolParams,
};

use action_frontend::ast::{Expr, ExprKind, Stmt, Type};
use action_frontend::builtin::{
    all as all_builtins, format_builtin_detail, format_ufcs_method_detail, receiver_kind_from_type,
    ufcs_methods_for_kind,
};
use action_frontend::fmt::{self, FormatOptions};
use action_frontend::lexer::{Span, Token, TokenKind};
use action_frontend::typecheck::TypeRegistry;

use crate::hir_lookup;
use crate::position::{self, find_node_at, FoundNode};
use crate::project::Project;
use crate::symbols;

use super::helpers::*;
use super::ServerState;

pub fn handle_completion(
    state: &ServerState,
    params: CompletionParams,
) -> Option<CompletionResponse> {
    let pos = params.text_document_position.position;
    let uri = &params.text_document_position.text_document.uri;

    let doc = match state.project.documents.get(uri) {
        Some(d) => d,
        None => return None,
    };

    let prefix = get_word_prefix(&doc.source, &pos);

    // Try member completion first (after `.` or `::`)
    if let Some(member_items) = member_completion_items(
        &doc.tokens,
        &doc.source,
        &pos,
        &doc.type_env,
        &state.project.session.base_type_env,
        &doc.registry,
        &state.project.session.base_registry,
        &prefix,
    ) {
        return Some(CompletionResponse::Array(member_items));
    }

    let mut items: Vec<CompletionItem> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    let keywords = &[
        "val",
        "var",
        "fun",
        "when",
        "else",
        "for",
        "in",
        "is",
        "break",
        "continue",
        "return",
        "enum",
        "type",
        "import",
        "module",
        "export",
        "const",
        "copy",
        "lazy",
        "unsafe",
        "external",
        "extension",
        "and",
        "or",
        "not",
        "as",
        "task",
    ];
    for kw in keywords {
        if kw.starts_with(&prefix) {
            seen.insert(kw.to_string());
            items.push(CompletionItem {
                label: kw.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                ..Default::default()
            });
        }
    }

    let builtins = all_builtins();
    for def in builtins {
        if def.name.starts_with(&prefix) && seen.insert(def.name.to_string()) {
            items.push(CompletionItem {
                label: def.name.to_string(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format_builtin_detail(def)),
                ..Default::default()
            });
        }
    }

    push_env_completion_items(
        &mut items,
        &mut seen,
        &prefix,
        &doc.type_env,
        &state.project.session.base_type_env,
    );

    for name in doc.definition_map.keys() {
        if name.starts_with(&prefix) && seen.insert(name.clone()) {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(match name.chars().next() {
                    Some(c) if c.is_uppercase() => CompletionItemKind::CLASS,
                    _ => CompletionItemKind::FUNCTION,
                }),
                ..Default::default()
            });
        }
    }

    Some(CompletionResponse::Array(items))
}

pub fn handle_signature_help(
    state: &ServerState,
    params: SignatureHelpParams,
) -> Option<SignatureHelp> {
    let pos = params.text_document_position_params.position;
    let uri = &params.text_document_position_params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let (lookup_key, func_type) = resolve_call_type(
        &doc.tokens,
        &doc.source,
        &pos,
        &doc.type_env,
        &state.project.session.base_type_env,
    )?;

    let display_name = lookup_key
        .rsplit_once('.')
        .map(|(_, method)| method.to_string())
        .unwrap_or_else(|| lookup_key.clone());

    match func_type {
        Type::Function(param_types, ret_type) => {
            let param_names = doc
                .hir
                .as_ref()
                .and_then(|hir| crate::hir_lookup::find_call_param_names(hir, &lookup_key));
            let label = format_function_signature(
                &display_name,
                &param_types,
                ret_type.as_ref(),
                param_names.as_deref(),
            );
            let parameters: Vec<lsp_types::ParameterInformation> = param_types
                .iter()
                .enumerate()
                .map(|(i, t)| lsp_types::ParameterInformation {
                    label: lsp_types::ParameterLabel::Simple(format_param_label(
                        i,
                        t,
                        param_names.as_deref(),
                    )),
                    documentation: None,
                })
                .collect();
            Some(SignatureHelp {
                signatures: vec![lsp_types::SignatureInformation {
                    label,
                    documentation: None,
                    parameters: Some(parameters),
                    active_parameter: Some(0),
                }],
                active_signature: Some(0),
                active_parameter: Some(0),
            })
        }
        _ => Some(SignatureHelp {
            signatures: vec![lsp_types::SignatureInformation {
                label: format!("{}()", display_name),
                documentation: None,
                parameters: None,
                active_parameter: None,
            }],
            active_signature: Some(0),
            active_parameter: None,
        }),
    }
}

pub fn handle_code_actions(
    state: &ServerState,
    params: CodeActionParams,
) -> Option<CodeActionResponse> {
    let uri = &params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let mut actions = Vec::new();

    for diag in &params.context.diagnostics {
        if let Some(ref code) = diag.code {
            let code_str = match code {
                lsp_types::NumberOrString::String(s) => s.as_str(),
                _ => "",
            };
            if code_str == "type-error"
                && (diag.message.contains("non-exhaustive")
                    || diag.message.contains("Non-exhaustive"))
            {
                if let Some(edit) = make_add_else_edit(&doc.tokens, &doc.source, &diag.range) {
                    let uri_clone = uri.clone();
                    actions.push(lsp_types::CodeActionOrCommand::CodeAction(CodeAction {
                        title: "Add else branch".to_string(),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![diag.clone()]),
                        edit: Some(WorkspaceEdit {
                            changes: Some(vec![(uri_clone, vec![edit])].into_iter().collect()),
                            ..Default::default()
                        }),
                        command: None,
                        is_preferred: Some(true),
                        ..Default::default()
                    }));
                }
            }
        }
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn make_add_else_edit(
    tokens: &[action_frontend::lexer::Token],
    source: &str,
    range: &Range,
) -> Option<TextEdit> {
    let offset = position::lsp_position_to_offset(
        source,
        &Position {
            line: range.end.line,
            character: range.end.character,
        },
    );

    let when_token = tokens
        .iter()
        .rev()
        .find(|t| matches!(t.kind, TokenKind::When) && t.span.end <= offset)?;
    let when_pos = when_token.span.start;

    let after_when = &source[when_pos..];
    let open_brace = after_when.find('{')?;
    let open_pos = when_pos + open_brace;

    let close_pos = find_matching_brace(source, open_pos)?;

    let line_start = source[..close_pos].rfind('\n').map(|p| p + 1).unwrap_or(0);
    let indent = &source[line_start..close_pos];
    let indent_str = if indent.chars().all(|c| c == ' ' || c == '\t') {
        indent.to_string()
    } else {
        "    ".to_string()
    };

    let insert_text = format!("{}else {{\n{0}    ...\n{0}}}\n{0}", indent_str);

    let lsp_pos = position::offset_to_lsp_position(source, close_pos);

    Some(TextEdit {
        range: Range {
            start: lsp_pos.clone(),
            end: lsp_pos,
        },
        new_text: insert_text,
    })
}

fn find_matching_brace(source: &str, open_pos: usize) -> Option<usize> {
    let mut depth = 0u32;
    for (i, ch) in source[open_pos..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open_pos + i);
                }
            }
            _ => {}
        }
    }
    None
}
