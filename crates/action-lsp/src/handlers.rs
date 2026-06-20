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

use super::position::{self, find_node_at, FoundNode};
use super::project::Project;
use super::symbols;

/// Server state holding all documents and the stdlib context
pub struct ServerState {
    pub project: Project,
}

impl ServerState {
    pub fn new(project: Project) -> Self {
        ServerState { project }
    }
}

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

// ---- Request handlers ----

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
        .and_then(|hir| super::hir_lookup::find_hir_expr_type(hir, &doc.source, &pos))
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
                .and_then(|hir| super::hir_lookup::find_call_param_names(hir, &lookup_key));
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
                            super::hir_lookup::find_hir_expr_type(hir, &doc.source, &pos)
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

pub fn handle_workspace_symbol(
    state: &ServerState,
    params: WorkspaceSymbolParams,
) -> Option<Vec<lsp_types::SymbolInformation>> {
    Some(state.project.workspace_symbols(&params.query))
}

// ============================================================
//  FEATURE 1: Member completion after `.` / `::`
// ============================================================

fn member_completion_items(
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

fn dot_member_items(
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

fn colon_member_items(
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

fn named_type_key(ty: &Type) -> Option<String> {
    match ty {
        Type::Named(name) => Some(name.clone()),
        Type::Generic(base, _) => named_type_key(base),
        Type::LazyList(inner) => named_type_key(inner),
        _ => None,
    }
}

fn push_extension_methods(
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

fn format_method_type(name: &str, ty: &Type) -> String {
    match ty {
        Type::Function(params, ret) => {
            let ps: Vec<String> = params.iter().map(|p| format!("{}", p)).collect();
            format!("{}({}) -> {}", name, ps.join(", "), ret)
        }
        other => format!("{}: {}", name, other),
    }
}

fn push_method_labels(
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

fn find_scope_aware_definition(
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

fn collect_pattern_bindings(pattern: &action_frontend::ast::Pattern, map: &mut HashMap<String, Span>) {
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

fn add_stmts_to_scope(stmts: &[Stmt], scope_map: &mut HashMap<String, Span>) {
    for stmt in stmts {
        add_stmt_to_scope(stmt, scope_map);
    }
}

fn add_stmt_to_scope(stmt: &Stmt, scope_map: &mut HashMap<String, Span>) {
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

fn extract_doc_comment(source: &str, ast: &[Stmt], name: &str) -> Option<String> {
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

fn find_stmt_span_for_name(stmts: &[Stmt], name: &str) -> Option<Span> {
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

fn lookup_type_in_envs<'a>(
    name: &str,
    type_env: &'a HashMap<String, Type>,
    stdlib_type_env: &'a HashMap<String, Type>,
) -> Option<&'a Type> {
    type_env.get(name).or_else(|| stdlib_type_env.get(name))
}

fn push_env_completion_items(
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

fn lookup_function_signature(
    name: &str,
    type_env: &HashMap<String, Type>,
    stdlib_type_env: &HashMap<String, Type>,
    hir: Option<&action_frontend::hir::HirModule>,
) -> Option<String> {
    let func_type = type_env.get(name).or_else(|| stdlib_type_env.get(name))?;
    match func_type {
        Type::Function(param_types, ret_type) => {
            let param_names = hir.and_then(|h| super::hir_lookup::find_call_param_names(h, name));
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

fn format_param_label(i: usize, ty: &Type, param_names: Option<&[String]>) -> String {
    if let Some(names) = param_names {
        if let Some(n) = names.get(i) {
            return format!("{}: {}", n, ty);
        }
    }
    format!("p{}: {}", i, ty)
}

fn format_function_signature(
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

fn resolve_call_type(
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

fn lookup_type<'a>(
    type_env: &'a HashMap<String, Type>,
    stdlib_type_env: &'a HashMap<String, Type>,
    key: &str,
) -> Option<&'a Type> {
    type_env.get(key).or_else(|| stdlib_type_env.get(key))
}

fn find_ufcs_type_env_key(
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

fn get_word_prefix(source: &str, pos: &Position) -> String {
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

fn find_call_target(
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
    use super::super::project::Project;
    use super::*;
    use action_frontend::typecheck::TypeRegistry;
    use lsp_types::{
        CompletionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DocumentSymbolParams, GotoDefinitionParams, HoverParams,
        ReferenceParams, SemanticTokensParams, TextDocumentContentChangeEvent, TextDocumentItem,
        TextDocumentPositionParams, VersionedTextDocumentIdentifier, WorkspaceSymbolParams,
    };
    use std::collections::HashMap;

    fn make_state(source: &str) -> ServerState {
        let proj = Project::with_stdlib(TypeRegistry::new(), HashMap::new(), Vec::new());
        let mut state = ServerState::new(proj);
        let uri = Url::parse("file:///test.at").unwrap();
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
        Url::parse("file:///test.at").unwrap()
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
