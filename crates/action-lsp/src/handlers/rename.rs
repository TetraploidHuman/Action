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

pub fn handle_prepare_rename(
    state: &ServerState,
    params: TextDocumentPositionParams,
) -> Option<PrepareRenameResponse> {
    let uri = &params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let node = find_node_at(&doc.tokens, &doc.source, &params.position)?;
    match &node {
        FoundNode::Ident(_name) => {
            let offset = position::lsp_position_to_offset(&doc.source, &params.position);
            let token = position::find_token_at(&doc.tokens, offset)?;
            Some(PrepareRenameResponse::Range(position::span_to_lsp_range(
                &token.span,
                &doc.source,
            )))
        }
        _ => None,
    }
}

pub fn handle_rename(state: &ServerState, params: RenameParams) -> Option<WorkspaceEdit> {
    let pos = params.text_document_position.position;
    let uri = &params.text_document_position.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let node = find_node_at(&doc.tokens, &doc.source, &pos)?;
    let old_name = match &node {
        FoundNode::Ident(name) => name.clone(),
        _ => return None,
    };

    let new_name = params.new_name;

    let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();
    let locations = state.project.find_references(&old_name);

    for loc in &locations {
        let range = loc.range;
        changes
            .entry(loc.uri.clone())
            .or_insert_with(Vec::new)
            .push(TextEdit {
                range,
                new_text: new_name.clone(),
            });
    }

    Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

pub fn handle_inlay_hints(state: &ServerState, params: InlayHintParams) -> Option<Vec<InlayHint>> {
    let uri = &params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let mut hints = Vec::new();

    for i in 0..doc.tokens.len() {
        let token = &doc.tokens[i];
        if matches!(token.kind, TokenKind::Val | TokenKind::Var) {
            if let Some(next_token) = doc.tokens.get(i + 1) {
                if let TokenKind::Ident(name) = &next_token.kind {
                    let ty = lookup_type_in_envs(
                        name,
                        &doc.type_env,
                        &state.project.session.base_type_env,
                    )
                    .cloned()
                    .or_else(|| {
                        doc.hir.as_ref().and_then(|hir| {
                            let pos = position::offset_to_lsp_position(
                                &doc.source,
                                next_token.span.start,
                            );
                            crate::hir_lookup::find_hir_expr_type(hir, &doc.source, &pos)
                        })
                    });
                    if let Some(ty) = ty {
                        let pos =
                            position::offset_to_lsp_position(&doc.source, next_token.span.end);
                        hints.push(InlayHint {
                            position: pos,
                            label: lsp_types::InlayHintLabel::String(format!(": {}", ty)),
                            kind: Some(lsp_types::InlayHintKind::TYPE),
                            text_edits: None,
                            tooltip: None,
                            padding_left: None,
                            padding_right: None,
                            data: None,
                        });
                    }
                }
            }
        }
    }

    if hints.is_empty() {
        None
    } else {
        Some(hints)
    }
}
