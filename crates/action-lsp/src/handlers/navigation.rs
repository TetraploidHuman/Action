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

pub fn handle_hover(state: &ServerState, params: HoverParams) -> Option<Hover> {
    let pos = params.text_document_position_params.position;
    let uri = &params.text_document_position_params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let node = find_node_at(&doc.tokens, &doc.source, &pos)?;
    let name = match &node {
        FoundNode::Ident(name) => name.clone(),
        FoundNode::Keyword(kw) => kw.clone(),
        _ => return None,
    };

    // Look up type: HIR expression type first, then top-level type_env
    let type_str = doc
        .hir
        .as_ref()
        .and_then(|hir| crate::hir_lookup::find_hir_expr_type(hir, &doc.source, &pos))
        .map(|t| format!("```action\n{}\n```", t))
        .or_else(|| {
            doc.type_env
                .get(&name)
                .or_else(|| state.project.session.base_type_env.get(&name))
                .map(|t| format!("```action\n{}: {}\n```", name, t))
        })
        .unwrap_or_else(|| format!("```action\n{}\n```", name));

    // Extract doc comment from source
    let doc_comment = extract_doc_comment(&doc.source, &doc.ast, &name);

    // Get function signature from type_env / HIR param names
    let signature = lookup_function_signature(
        &name,
        &doc.type_env,
        &state.project.session.base_type_env,
        doc.hir.as_ref(),
    );

    // Build markdown content
    let mut parts: Vec<String> = Vec::new();
    parts.push(type_str);
    if let Some(sig) = signature {
        parts.push(format!("---\n**Signature**\n```action\n{}\n```", sig));
    }
    if let Some(comment) = doc_comment {
        parts.push(format!("---\n{}", comment));
    }

    let contents = HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: parts.join("\n\n"),
    });

    let offset = position::lsp_position_to_offset(&doc.source, &pos);
    let token = position::find_token_at(&doc.tokens, offset);
    let range = token.map(|t| position::span_to_lsp_range(&t.span, &doc.source));

    Some(Hover { contents, range })
}

pub fn handle_goto_definition(
    state: &ServerState,
    params: GotoDefinitionParams,
) -> Option<GotoDefinitionResponse> {
    let pos = params.text_document_position_params.position;
    let uri = &params.text_document_position_params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let node = find_node_at(&doc.tokens, &doc.source, &pos)?;
    let name = match &node {
        FoundNode::Ident(name) => name.clone(),
        _ => return None,
    };

    // 1. Try AST scope-aware lookup (handles shadowing correctly)
    if let Some(span) = find_scope_aware_definition(&doc.ast, &doc.source, &pos, &name) {
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri: uri.clone(),
            range: position::span_to_lsp_range(&span, &doc.source),
        }));
    }

    // 2. Try current document's flat definition_map
    if let Some(span) = doc.definition_map.get(&name) {
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri: uri.clone(),
            range: position::span_to_lsp_range(span, &doc.source),
        }));
    }

    // 3. Search all open documents
    if let Some(loc) = state.project.find_definition(uri, &name) {
        return Some(GotoDefinitionResponse::Scalar(loc));
    }

    None
}

pub fn handle_references(state: &ServerState, params: ReferenceParams) -> Option<Vec<Location>> {
    let pos = params.text_document_position.position;
    let uri = &params.text_document_position.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let node = find_node_at(&doc.tokens, &doc.source, &pos)?;
    let name = match &node {
        FoundNode::Ident(name) => name.clone(),
        _ => return None,
    };

    Some(state.project.find_references(&name))
}

pub fn handle_document_highlight(
    state: &ServerState,
    params: DocumentHighlightParams,
) -> Option<Vec<DocumentHighlight>> {
    let pos = params.text_document_position_params.position;
    let uri = &params.text_document_position_params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let node = find_node_at(&doc.tokens, &doc.source, &pos)?;
    let name = match &node {
        FoundNode::Ident(name) => name.clone(),
        _ => return None,
    };

    let mut highlights = Vec::new();
    for token in &doc.tokens {
        if let TokenKind::Ident(token_name) = &token.kind {
            if token_name == &name {
                highlights.push(DocumentHighlight {
                    range: position::span_to_lsp_range(&token.span, &doc.source),
                    kind: Some(DocumentHighlightKind::TEXT),
                });
            }
        }
    }
    Some(highlights)
}
