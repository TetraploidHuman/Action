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

use crate::lexer::TokenKind;

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
    // Full text sync: take the last content change
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

    let offset = position::lsp_position_to_offset(&doc.source, &pos);
    let node = find_node_at(&doc.tokens, &doc.source, &pos)?;

    let name = match &node {
        FoundNode::Ident(name) => name.clone(),
        FoundNode::Keyword(kw) => kw.clone(),
        _ => return None,
    };

    // Look up type in document's type_env first, then stdlib
    let type_str = doc
        .type_env
        .get(&name)
        .or_else(|| state.project.stdlib_type_env.get(&name))
        .map(|t| format!("{}", t))
        .unwrap_or_else(|| "unknown".to_string());

    let contents = HoverContents::Markup(MarkupContent {
        kind: MarkupKind::Markdown,
        value: format!("`{}: {}`", name, type_str),
    });

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

    // Search current document first
    if let Some(span) = doc.definition_map.get(&name) {
        return Some(GotoDefinitionResponse::Scalar(Location {
            uri: uri.clone(),
            range: position::span_to_lsp_range(span, &doc.source),
        }));
    }

    // Search all open documents
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

    let mut items: Vec<CompletionItem> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Keywords
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

    // Builtins
    let builtins = &[
        ("print", CompletionItemKind::FUNCTION),
        ("println", CompletionItemKind::FUNCTION),
        ("len", CompletionItemKind::FUNCTION),
        ("push", CompletionItemKind::FUNCTION),
        ("pop", CompletionItemKind::FUNCTION),
        ("get", CompletionItemKind::FUNCTION),
        ("map", CompletionItemKind::FUNCTION),
        ("filter", CompletionItemKind::FUNCTION),
        ("reduce", CompletionItemKind::FUNCTION),
        ("fold", CompletionItemKind::FUNCTION),
        ("range", CompletionItemKind::FUNCTION),
        ("true", CompletionItemKind::CONSTANT),
        ("false", CompletionItemKind::CONSTANT),
    ];
    for (name, kind) in builtins {
        if name.starts_with(&prefix) && seen.insert(name.to_string()) {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(*kind),
                ..Default::default()
            });
        }
    }

    // Symbols from current document type_env
    for name in doc.type_env.keys() {
        if name.starts_with(&prefix) && seen.insert(name.clone()) {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::VARIABLE),
                ..Default::default()
            });
        }
    }

    // Symbols from definition_map (deduplicate against type_env)
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
    let tokens = symbols::compute_semantic_tokens(&doc.tokens);
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

    let func_name = find_call_target(&doc.tokens, &doc.source, &pos)?;

    // Look up the function's type
    let func_type = doc
        .type_env
        .get(&func_name)
        .or_else(|| state.project.stdlib_type_env.get(&func_name));

    match func_type {
        Some(crate::ast::Type::Function(param_types, ret_type)) => {
            let label = format!(
                "{}({}) -> {}",
                func_name,
                param_types
                    .iter()
                    .enumerate()
                    .map(|(i, t)| format!("p{}: {}", i, t))
                    .collect::<Vec<_>>()
                    .join(", "),
                ret_type
            );
            Some(SignatureHelp {
                signatures: vec![lsp_types::SignatureInformation {
                    label,
                    documentation: None,
                    parameters: Some(
                        param_types
                            .iter()
                            .enumerate()
                            .map(|(i, t)| lsp_types::ParameterInformation {
                                label: lsp_types::ParameterLabel::Simple(format!("p{}: {}", i, t)),
                                documentation: None,
                            })
                            .collect(),
                    ),
                    active_parameter: Some(0),
                }],
                active_signature: Some(0),
                active_parameter: Some(0),
            })
        }
        _ => Some(SignatureHelp {
            signatures: vec![lsp_types::SignatureInformation {
                label: format!("{}()", func_name),
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
    let mut stack: Vec<(usize, u32)> = Vec::new(); // (byte_offset, line)
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
        // Look for val/var keyword followed by an identifier
        if matches!(token.kind, TokenKind::Val | TokenKind::Var) {
            if let Some(next_token) = doc.tokens.get(i + 1) {
                if let TokenKind::Ident(name) = &next_token.kind {
                    if let Some(ty) = doc.type_env.get(name) {
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
    let tab_size = params.options.tab_size as usize;
    let use_spaces = params.options.insert_spaces;

    let indent_str = if use_spaces {
        " ".repeat(tab_size)
    } else {
        "\t".to_string()
    };

    // Build per-line brace counts from tokens (avoids counting braces in strings/comments)
    let mut line_braces: HashMap<u32, (u32, u32)> = HashMap::new();
    for token in &doc.tokens {
        let line = (token.span.line as u32).saturating_sub(1);
        let entry = line_braces.entry(line).or_insert((0, 0));
        match token.kind {
            TokenKind::LBrace => entry.0 += 1,
            TokenKind::RBrace => entry.1 += 1,
            _ => {}
        }
    }

    let mut edits = Vec::new();
    let mut expected_depth: i32 = 0;

    for (line_num, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }

        let (open_count, close_count) = line_braces
            .get(&(line_num as u32))
            .copied()
            .unwrap_or((0, 0));

        let current_depth = expected_depth.saturating_sub(close_count as i32);

        let current_indent_len = line.len() - trimmed.len();
        let expected_indent = indent_str.repeat(current_depth as usize);

        if current_indent_len != expected_indent.len() || !line.starts_with(&expected_indent) {
            let line_start = lsp_types::Position {
                line: line_num as u32,
                character: 0,
            };
            let line_end = lsp_types::Position {
                line: line_num as u32,
                character: current_indent_len as u32,
            };
            edits.push(TextEdit {
                range: Range {
                    start: line_start,
                    end: line_end,
                },
                new_text: expected_indent,
            });
        }

        let next_depth = current_depth.saturating_add(open_count as i32);
        expected_depth = next_depth;
    }

    if edits.is_empty() {
        None
    } else {
        Some(edits)
    }
}

pub fn handle_code_actions(
    state: &ServerState,
    params: CodeActionParams,
) -> Option<CodeActionResponse> {
    let uri = &params.text_document.uri;
    let doc = state.project.documents.get(uri)?;

    let mut actions = Vec::new();

    // Offer "Add else branch" for non-exhaustive when diagnostics
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
                // Find the closing brace of the when expression and insert else branch
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

/// Create a TextEdit that inserts `else { ... }` before the closing `}` of a when block.
/// Searches from the diagnostic position to find the when expression's end.
fn make_add_else_edit(
    tokens: &[crate::lexer::Token],
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

    // Use token stream to find the `when` keyword (avoids matching
    // "when" inside string literals, comments, or identifiers).
    let when_token = tokens
        .iter()
        .rev()
        .find(|t| matches!(t.kind, TokenKind::When) && t.span.end <= offset)?;
    let when_pos = when_token.span.start;

    // Find the opening `{` of the when body (first `{` after `when` keyword)
    let after_when = &source[when_pos..];
    let open_brace = after_when.find('{')?;
    let open_pos = when_pos + open_brace;

    // Find matching closing `}`
    let close_pos = find_matching_brace(source, open_pos)?;

    // Insert `else { ... }` just before the closing `}`
    // Preserve indentation: use the indentation of the closing brace line
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

/// Find the matching `}` for the `{` at open_pos. Returns the byte offset of the `}`.
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

// ---- Helpers ----

/// Extract the word prefix before the cursor position (used for completion filtering)
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

/// Find the function name being called at the cursor position
/// Walks tokens tracking parenthesis depth
fn find_call_target(
    tokens: &[crate::lexer::Token],
    source: &str,
    pos: &Position,
) -> Option<String> {
    let offset = position::lsp_position_to_offset(source, pos);

    // Walk backwards through tokens to find the function name
    let mut depth = 0;
    let mut found_ident = None;

    for token in tokens.iter().rev() {
        if token.span.start > offset {
            continue;
        }

        match &token.kind {
            TokenKind::LParen => {
                if depth == 0 {
                    // This is the opening paren of the call — look for the identifier before it
                    return found_ident;
                }
                depth -= 1;
            }
            TokenKind::RParen => depth += 1,
            TokenKind::Ident(name) if depth == 0 => {
                found_ident = Some(name.clone());
            }
            _ if depth == 0 => {
                // Non-ident token at depth 0 — not a simple call
                return None;
            }
            _ => {}
        }
    }

    found_ident
}
