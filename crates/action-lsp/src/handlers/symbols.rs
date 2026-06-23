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

pub fn handle_semantic_tokens(
    state: &ServerState,
    params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
    let uri = &params.text_document.uri;
    let doc = state.project.documents.get(uri)?;
    let tokens = symbols::compute_semantic_tokens(
        &doc.tokens,
        &doc.type_env,
        &state.project.session.base_type_env,
        &doc.definition_map,
    );
    Some(SemanticTokensResult::Tokens(lsp_types::SemanticTokens {
        result_id: None,
        data: tokens,
    }))
}

pub fn handle_document_symbols(
    state: &ServerState,
    params: DocumentSymbolParams,
) -> Option<DocumentSymbolResponse> {
    let uri = &params.text_document.uri;
    let doc = state.project.documents.get(uri)?;
    let symbols = symbols::extract_document_symbols(&doc.ast, &doc.source);
    Some(DocumentSymbolResponse::Nested(symbols))
}

pub fn handle_folding_range(
    state: &ServerState,
    params: FoldingRangeParams,
) -> Option<Vec<FoldingRange>> {
    let uri = &params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let source = &doc.source;
    let mut stack: Vec<(usize, u32)> = Vec::new();
    let mut ranges = Vec::new();

    for (byte_offset, ch) in source.char_indices() {
        match ch {
            '{' => {
                let line = position::offset_to_lsp_position(source, byte_offset).line;
                stack.push((byte_offset, line));
            }
            '}' => {
                if let Some((_open_offset, open_line)) = stack.pop() {
                    let close_line = position::offset_to_lsp_position(source, byte_offset).line;
                    if close_line > open_line {
                        ranges.push(FoldingRange {
                            start_line: open_line,
                            start_character: None,
                            end_line: close_line,
                            end_character: None,
                            kind: Some(lsp_types::FoldingRangeKind::Region),
                            collapsed_text: None,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    if ranges.is_empty() {
        None
    } else {
        Some(ranges)
    }
}

pub fn handle_workspace_symbol(
    state: &ServerState,
    params: WorkspaceSymbolParams,
) -> Option<Vec<lsp_types::SymbolInformation>> {
    Some(state.project.workspace_symbols(&params.query))
}
