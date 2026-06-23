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

// ---- Notification handlers ----

pub fn handle_did_open(
    state: &mut ServerState,
    params: DidOpenTextDocumentParams,
) -> Vec<Diagnostic> {
    let uri = params.text_document.uri.clone();
    let source = params.text_document.text;
    let version = params.text_document.version;
    state.project.update_document(&uri, source, version)
}

pub fn handle_did_change(
    state: &mut ServerState,
    params: DidChangeTextDocumentParams,
) -> Vec<Diagnostic> {
    let uri = params.text_document.uri.clone();
    let version = params.text_document.version;
    let source = params
        .content_changes
        .last()
        .map(|c| c.text.clone())
        .unwrap_or_default();
    state.project.update_document(&uri, source, version)
}

pub fn handle_did_close(state: &mut ServerState, params: DidCloseTextDocumentParams) {
    state.project.remove_document(&params.text_document.uri);
}

pub fn handle_formatting(
    state: &ServerState,
    params: DocumentFormattingParams,
) -> Option<Vec<TextEdit>> {
    let uri = &params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let source = &doc.source;
    let options = FormatOptions {
        tab_size: params.options.tab_size as usize,
        insert_spaces: params.options.insert_spaces,
    };
    let formatted = fmt::format_source(source, &doc.tokens, &options);
    if formatted == *source {
        return None;
    }

    let end_line = source.lines().count().saturating_sub(1) as u32;
    let end_char = source
        .lines()
        .last()
        .map(|l| l.chars().count() as u32)
        .unwrap_or(0);

    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: end_line,
                character: end_char,
            },
        },
        new_text: formatted,
    }])
}
