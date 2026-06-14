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

use crate::ast::{Expr, Stmt, Type};
use crate::lexer::{Span, Token, TokenKind};
use crate::typecheck::TypeRegistry;

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

    // Look up type in document's type_env first, then stdlib
    let type_str = doc
        .type_env
        .get(&name)
        .or_else(|| state.project.stdlib_type_env.get(&name))
        .map(|t| format!("```action\n{}: {}\n```", name, t))
        .unwrap_or_else(|| format!("```action\n{}\n```", name));

    // Extract doc comment from source
    let doc_comment = extract_doc_comment(&doc.source, &doc.ast, &name);

    // Get function signature if applicable
    let signature = extract_function_signature(&doc.ast, &name);

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
        &state.project.stdlib_type_env,
        &state.project.stdlib_registry,
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

    for name in doc.type_env.keys() {
        if name.starts_with(&prefix) && seen.insert(name.clone()) {
            let kind = if matches!(doc.type_env.get(name), Some(Type::Function(..))) {
                CompletionItemKind::FUNCTION
            } else {
                CompletionItemKind::VARIABLE
            };
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(kind),
                detail: doc.type_env.get(name).map(|t| format!("{}", t)),
                ..Default::default()
            });
        }
    }

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

    let func_type = doc
        .type_env
        .get(&func_name)
        .or_else(|| state.project.stdlib_type_env.get(&func_name));

    match func_type {
        Some(Type::Function(param_types, ret_type)) => {
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
    _registry: &TypeRegistry,
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
        dot_member_items(receiver_type, prefix)
    } else {
        Vec::new()
    };

    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

fn dot_member_items(receiver_type: &Type, prefix: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    match receiver_type {
        Type::Named(type_name) => {
            // Suggest UFCS methods for known types
            let struct_name = type_name.as_str();
            let methods = known_type_methods(struct_name);
            for (method, detail) in &methods {
                if method.starts_with(prefix) {
                    items.push(CompletionItem {
                        label: method.clone(),
                        detail: Some(detail.clone()),
                        kind: Some(CompletionItemKind::METHOD),
                        ..Default::default()
                    });
                }
            }
        }
        Type::Map(_, v) => {
            let methods: Vec<(&str, String)> = vec![
                ("contains", format!("contains(key) -> Bool")),
                ("isEmpty", "isEmpty -> Bool".to_string()),
                ("insert", format!("insert(key, value)")),
                ("remove", format!("remove(key) -> {}?", v)),
                ("get", format!("get(key) -> {}?", v)),
            ];
            for (name, detail) in &methods {
                if name.starts_with(prefix) {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        detail: Some(detail.clone()),
                        kind: Some(CompletionItemKind::METHOD),
                        ..Default::default()
                    });
                }
            }
        }
        Type::Set(e) => {
            let methods: Vec<(&str, String)> = vec![
                ("contains", format!("contains(elem) -> Bool")),
                ("isEmpty", "isEmpty -> Bool".to_string()),
                ("insert", format!("insert(elem)")),
                ("remove", format!("remove(elem) -> {}?", e)),
            ];
            for (name, detail) in &methods {
                if name.starts_with(prefix) {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        detail: Some(detail.clone()),
                        kind: Some(CompletionItemKind::METHOD),
                        ..Default::default()
                    });
                }
            }
        }
        Type::Stream(_) => {
            let methods = ["send", "receive", "close"];
            for name in &methods {
                if name.starts_with(prefix) {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        ..Default::default()
                    });
                }
            }
        }
        Type::Task(_) => {
            let methods = ["cancel", "is_done", "is_cancelled", "wait"];
            for name in &methods {
                if name.starts_with(prefix) {
                    items.push(CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::METHOD),
                        ..Default::default()
                    });
                }
            }
        }
        Type::Nullable(inner) => {
            if "or".starts_with(prefix) {
                items.push(CompletionItem {
                    label: "or".to_string(),
                    detail: Some("or { fallback } -> T".to_string()),
                    kind: Some(CompletionItemKind::METHOD),
                    ..Default::default()
                });
            }
            let inner_items = dot_member_items(inner, prefix);
            items.extend(inner_items);
        }
        _ => {}
    }

    items
}

fn known_type_methods(type_name: &str) -> Vec<(String, String)> {
    match type_name {
        "String" | "Str" => vec![
            ("len".into(), "len() -> Int".into()),
            ("contains".into(), "contains(substr: String) -> Bool".into()),
            (
                "startsWith".into(),
                "startsWith(prefix: String) -> Bool".into(),
            ),
            ("endsWith".into(), "endsWith(suffix: String) -> Bool".into()),
            (
                "substring".into(),
                "substring(start: Int, end: Int) -> String".into(),
            ),
            ("toUpper".into(), "toUpper() -> String".into()),
            ("toLower".into(), "toLower() -> String".into()),
            ("trim".into(), "trim() -> String".into()),
            (
                "split".into(),
                "split(delim: String) -> List<String>".into(),
            ),
            (
                "replace".into(),
                "replace(old: String, new: String) -> String".into(),
            ),
            ("isEmpty".into(), "isEmpty() -> Bool".into()),
        ],
        "Int" => vec![
            ("toFloat".into(), "toFloat() -> Float".into()),
            ("toString".into(), "toString() -> String".into()),
            ("abs".into(), "abs() -> Int".into()),
            ("min".into(), "min(other: Int) -> Int".into()),
            ("max".into(), "max(other: Int) -> Int".into()),
        ],
        "Float" | "Double" => vec![
            ("toInt".into(), "toInt() -> Int".into()),
            ("toString".into(), "toString() -> String".into()),
            ("round".into(), "round() -> Int".into()),
            ("floor".into(), "floor() -> Int".into()),
            ("ceil".into(), "ceil() -> Int".into()),
        ],
        _ if type_name.starts_with("List")
            || type_name.starts_with("list")
            || type_name.starts_with("Array")
            || type_name.starts_with("Vec") =>
        {
            vec![
                ("len".into(), "len() -> Int".into()),
                ("isEmpty".into(), "isEmpty() -> Bool".into()),
                ("push".into(), "push(value: T)".into()),
                ("pop".into(), "pop() -> T".into()),
                ("get".into(), "get(index: Int) -> T".into()),
                ("map".into(), "map(fn: T -> U) -> List<U>".into()),
                ("filter".into(), "filter(fn: T -> Bool) -> List<T>".into()),
                ("reduce".into(), "reduce(fn: (T, T) -> T) -> T".into()),
                (
                    "fold".into(),
                    "fold(initial: T, fn: (T, T) -> T) -> T".into(),
                ),
                ("any".into(), "any(fn: T -> Bool) -> Bool".into()),
                ("all".into(), "all(fn: T -> Bool) -> Bool".into()),
                ("find".into(), "find(fn: T -> Bool) -> T?".into()),
                ("contains".into(), "contains(value: T) -> Bool".into()),
                ("sorted".into(), "sorted() -> List<T>".into()),
                ("reversed".into(), "reversed() -> List<T>".into()),
            ]
        }
        _ => vec![],
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
        match expr {
            Expr::Block(stmts) => {
                self.enter_scope(HashMap::new());
                self.walk_stmts(stmts);
                self.exit_scope();
            }
            Expr::Call {
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
            Expr::Lambda { params, body, .. } => {
                let mut lam_scope = HashMap::new();
                for p in params {
                    lam_scope.insert(p.clone(), Span::default());
                }
                self.enter_scope(lam_scope);
                self.walk_expr(body);
                self.exit_scope();
            }
            Expr::Binary(lhs, _, rhs) => {
                self.walk_expr(lhs);
                self.walk_expr(rhs);
            }
            Expr::Unary(_, inner) => self.walk_expr(inner),
            Expr::FieldAccess(obj, _) => self.walk_expr(obj),
            Expr::Index(obj, idx) => {
                self.walk_expr(obj);
                self.walk_expr(idx);
            }
            Expr::When(w) => match &w.kind {
                crate::ast::WhenKind::OneLine {
                    condition,
                    then_expr,
                    else_expr,
                } => {
                    self.walk_expr(condition);
                    self.walk_expr(then_expr);
                    self.walk_expr(else_expr);
                }
                crate::ast::WhenKind::ValueMatch { value, arms } => {
                    self.walk_expr(value);
                    for arm in arms {
                        let mut arm_scope = HashMap::new();
                        collect_pattern_bindings(&arm.pattern, &mut arm_scope);
                        self.enter_scope(arm_scope);
                        self.walk_expr(&arm.body);
                        self.exit_scope();
                    }
                }
                crate::ast::WhenKind::ConditionChain { arms } => {
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
            Expr::For(fr) => match &fr.kind {
                crate::ast::ForKind::Iterate {
                    var,
                    iterable,
                    body,
                    ..
                } => {
                    self.walk_expr(iterable);
                    let mut for_scope = HashMap::new();
                    for_scope.insert(var.clone(), Span::default());
                    self.enter_scope(for_scope);
                    self.walk_expr(body);
                    self.exit_scope();
                }
                crate::ast::ForKind::IterateWithIndex {
                    vars,
                    iterable,
                    body,
                } => {
                    self.walk_expr(iterable);
                    let mut for_scope = HashMap::new();
                    for v in vars {
                        for_scope.insert(v.clone(), Span::default());
                    }
                    self.enter_scope(for_scope);
                    self.walk_expr(body);
                    self.exit_scope();
                }
                crate::ast::ForKind::NestedIterate { bindings, body, .. } => {
                    for (_, iter) in bindings {
                        self.walk_expr(iter);
                    }
                    let mut for_scope = HashMap::new();
                    for (v, _) in bindings {
                        for_scope.insert(v.clone(), Span::default());
                    }
                    self.enter_scope(for_scope);
                    self.walk_expr(body);
                    self.exit_scope();
                }
                crate::ast::ForKind::Condition { condition, body } => {
                    self.walk_expr(condition);
                    self.walk_expr(body);
                }
                crate::ast::ForKind::Infinite { body } => {
                    self.walk_expr(body);
                }
            },
            Expr::Assign { target, value } => {
                self.walk_expr(target);
                self.walk_expr(value);
            }
            Expr::OrBlock { nullable, fallback } => {
                self.walk_expr(nullable);
                self.walk_expr(fallback);
            }
            Expr::Tuple(items) => {
                for (_, e) in items {
                    self.walk_expr(e);
                }
            }
            Expr::StructLiteral(fields) => {
                for (_, e) in fields {
                    self.walk_expr(e);
                }
            }
            Expr::MapLiteral(entries) => {
                for (k, v) in entries {
                    self.walk_expr(k);
                    self.walk_expr(v);
                }
            }
            Expr::SetLiteral(elements) => {
                for e in elements {
                    self.walk_expr(e);
                }
            }
            Expr::Range(start, end) => {
                self.walk_expr(start);
                self.walk_expr(end);
            }
            Expr::Unsafe(inner) => self.walk_expr(inner),
            Expr::Copy(inner) => self.walk_expr(inner),
            Expr::StringInterpolate(parts) => {
                for part in parts {
                    if let crate::ast::StringPart::Expr(e) = part {
                        self.walk_expr(e);
                    }
                }
            }
            Expr::Ident(name) => {
                if name == self.target_name && self.result.is_none() {
                    if let Some(span) = self.lookup() {
                        self.result = Some(span);
                    }
                }
            }
            Expr::Literal(_) | Expr::Null | Expr::Continue | Expr::Break | Expr::FunctionRef(_) => {
            }
        }
    }
}

fn collect_pattern_bindings(pattern: &crate::ast::Pattern, map: &mut HashMap<String, Span>) {
    use crate::ast::Pattern;
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

fn extract_function_signature(stmts: &[Stmt], name: &str) -> Option<String> {
    for stmt in stmts {
        match stmt {
            Stmt::Fun {
                name: n,
                params,
                return_type,
                ..
            } if n == name => {
                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| {
                        if let Some(ty) = &p.ty {
                            format!("{}: {}", p.name, ty)
                        } else {
                            p.name.clone()
                        }
                    })
                    .collect();
                let ret = return_type
                    .as_ref()
                    .map(|t| format!("{}", t))
                    .unwrap_or_else(|| "?".to_string());
                return Some(format!(
                    "fun {}({}) -> {}",
                    name,
                    params_str.join(", "),
                    ret
                ));
            }
            Stmt::Module { body, .. } => {
                if let Some(sig) = extract_function_signature(body, name) {
                    return Some(sig);
                }
            }
            _ => {}
        }
    }
    None
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
    tokens: &[crate::lexer::Token],
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
    use crate::typecheck::TypeRegistry;
    use lsp_types::{
        CompletionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DocumentSymbolParams, GotoDefinitionParams, HoverParams,
        ReferenceParams, SemanticTokensParams, TextDocumentContentChangeEvent, TextDocumentItem,
        TextDocumentPositionParams, VersionedTextDocumentIdentifier, WorkspaceSymbolParams,
    };
    use std::collections::HashMap;
    // use std::path::PathBuf;

    fn make_state(source: &str) -> ServerState {
        let proj = Project::new(TypeRegistry::new(), HashMap::new(), Vec::new());
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
        let mut state = state_with_proj(Project::new(
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
