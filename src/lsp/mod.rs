pub mod document;
pub mod handlers;
pub mod position;
pub mod project;
pub mod symbols;

use std::path::PathBuf;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    CodeActionProviderCapability, CompletionOptions, InitializeResult, RenameOptions,
    SemanticTokensOptions, ServerCapabilities, SignatureHelpOptions, TextDocumentSyncCapability,
    TextDocumentSyncKind,
};

use crate::session::FrontendSession;

use self::handlers::ServerState;
use self::project::Project;

/// Start the LSP server on stdin/stdout
pub fn start_lsp() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Action LSP server starting...");

    let (connection, io_threads) = Connection::stdio();

    // Initialize
    let (init_id, init_params) = connection.initialize_start()?;

    let capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        references_provider: Some(lsp_types::OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![".".to_string(), ":".to_string()]),
            ..Default::default()
        }),
        semantic_tokens_provider: Some(
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                SemanticTokensOptions {
                    legend: symbols::get_semantic_tokens_legend(),
                    full: Some(lsp_types::SemanticTokensFullOptions::Bool(true)),
                    ..Default::default()
                },
            ),
        ),
        document_symbol_provider: Some(lsp_types::OneOf::Left(true)),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec!["(".to_string(), ",".to_string()]),
            ..Default::default()
        }),
        document_highlight_provider: Some(lsp_types::OneOf::Left(true)),
        rename_provider: Some(lsp_types::OneOf::Right(RenameOptions {
            prepare_provider: Some(true),
            work_done_progress_options: lsp_types::WorkDoneProgressOptions {
                work_done_progress: None,
            },
        })),
        folding_range_provider: Some(lsp_types::FoldingRangeProviderCapability::Simple(true)),
        inlay_hint_provider: Some(lsp_types::OneOf::Left(true)),
        document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        workspace_symbol_provider: Some(lsp_types::OneOf::Left(true)),
        ..Default::default()
    };

    let init_result = InitializeResult {
        capabilities,
        server_info: Some(lsp_types::ServerInfo {
            name: "action-lsp".to_string(),
            version: Some("0.2.0".to_string()),
        }),
    };

    connection.initialize_finish(init_id, serde_json::to_value(&init_result)?)?;

    // Extract workspace roots from initialize params
    let root_uri = init_params
        .get("rootUri")
        .and_then(|v| serde_json::from_value::<Option<lsp_types::Url>>(v.clone()).ok())
        .flatten();
    let workspace_folders: Option<Vec<lsp_types::WorkspaceFolder>> = init_params
        .get("workspaceFolders")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .flatten();

    // Load stdlib via shared frontend session search-dir strategy
    let search_dirs = workspace_search_dirs(root_uri.as_ref(), workspace_folders.as_deref());
    let (stdlib_registry, stdlib_type_env) = FrontendSession::load_stdlib_context(&search_dirs);
    let session = FrontendSession::with_context(search_dirs, stdlib_registry, stdlib_type_env)
        .expect("stdlib load should not fail after context build");
    let project = Project::new(session);
    let mut state = ServerState::new(project);

    // Main loop
    main_loop(&connection, &mut state)?;

    io_threads.join()?;
    Ok(())
}

fn main_loop(
    connection: &Connection,
    state: &mut ServerState,
) -> Result<(), Box<dyn std::error::Error>> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                let response = handle_request(state, &req);
                connection.sender.send(Message::Response(response))?;
            }
            Message::Notification(not) => {
                handle_notification(state, &not, connection)?;
            }
            Message::Response(_) => {
                // We don't send requests to the client, ignore responses
            }
        }
    }
    Ok(())
}

fn handle_request(state: &mut ServerState, req: &Request) -> Response {
    let id = req.id.clone();

    match req.method.as_str() {
        "textDocument/hover" => dispatch(state, req, id, handlers::handle_hover),
        "textDocument/definition" => dispatch(state, req, id, handlers::handle_goto_definition),
        "textDocument/completion" => dispatch(state, req, id, handlers::handle_completion),
        "textDocument/semanticTokens/full" => {
            dispatch(state, req, id, handlers::handle_semantic_tokens)
        }
        "textDocument/documentSymbol" => {
            dispatch(state, req, id, handlers::handle_document_symbols)
        }
        "textDocument/signatureHelp" => dispatch(state, req, id, handlers::handle_signature_help),
        "textDocument/references" => dispatch(state, req, id, handlers::handle_references),
        "textDocument/documentHighlight" => {
            dispatch(state, req, id, handlers::handle_document_highlight)
        }
        "textDocument/prepareRename" => dispatch(state, req, id, handlers::handle_prepare_rename),
        "textDocument/rename" => dispatch(state, req, id, handlers::handle_rename),
        "textDocument/foldingRange" => dispatch(state, req, id, handlers::handle_folding_range),
        "textDocument/inlayHint" => dispatch(state, req, id, handlers::handle_inlay_hints),
        "textDocument/formatting" => dispatch(state, req, id, handlers::handle_formatting),
        "textDocument/codeAction" => dispatch(state, req, id, handlers::handle_code_actions),
        "workspace/symbol" => dispatch(state, req, id, handlers::handle_workspace_symbol),
        _ => Response::new_ok(id, Option::<()>::None),
    }
}

/// Parse request params and dispatch to handler. On parse error, returns the
/// error response directly (instead of swallowing it like the old code did).
fn dispatch<T, R>(
    state: &mut ServerState,
    req: &Request,
    id: lsp_server::RequestId,
    handler: fn(&ServerState, T) -> R,
) -> Response
where
    T: serde::de::DeserializeOwned,
    R: serde::Serialize,
{
    match parse_params::<T>(req) {
        Ok(p) => {
            let result = handler(state, p);
            Response::new_ok(id, result)
        }
        Err(err_response) => err_response,
    }
}

fn handle_notification(
    state: &mut ServerState,
    not: &Notification,
    connection: &Connection,
) -> Result<(), Box<dyn std::error::Error>> {
    match not.method.as_str() {
        "textDocument/didOpen" => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::DidOpenTextDocumentParams>(not.params.clone())
            {
                let uri = params.text_document.uri.clone();
                let diags = handlers::handle_did_open(state, params);
                publish_diagnostics(connection, &uri, diags)?;
            }
        }
        "textDocument/didChange" => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::DidChangeTextDocumentParams>(not.params.clone())
            {
                let diags = handlers::handle_did_change(state, params.clone());
                publish_diagnostics(connection, &params.text_document.uri, diags)?;
            }
        }
        "textDocument/didClose" => {
            if let Ok(params) =
                serde_json::from_value::<lsp_types::DidCloseTextDocumentParams>(not.params.clone())
            {
                handlers::handle_did_close(state, params);
            }
        }
        _ => {}
    }
    Ok(())
}

fn publish_diagnostics(
    connection: &Connection,
    uri: &lsp_types::Url,
    diagnostics: Vec<lsp_types::Diagnostic>,
) -> Result<(), Box<dyn std::error::Error>> {
    let params = lsp_types::PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics,
        version: None,
    };
    let not = Notification::new("textDocument/publishDiagnostics".to_string(), params);
    connection.sender.send(Message::Notification(not))?;
    Ok(())
}

fn parse_params<T: serde::de::DeserializeOwned>(req: &Request) -> Result<T, Response> {
    serde_json::from_value(req.params.clone())
        .map_err(|e| Response::new_err(req.id.clone(), -32700, format!("Parse error: {}", e)))
}

// ---- Stdlib loading ----

/// Workspace lib/ roots plus shared frontend fallback dirs (cwd, exe-relative).
fn workspace_search_dirs(
    root_uri: Option<&lsp_types::Url>,
    workspace_folders: Option<&[lsp_types::WorkspaceFolder]>,
) -> Vec<PathBuf> {
    let mut extra: Vec<PathBuf> = Vec::new();
    if let Some(folders) = workspace_folders {
        for wf in folders {
            if let Ok(path) = wf.uri.to_file_path() {
                extra.push(path.join("lib"));
            }
        }
    }
    if let Some(uri) = root_uri {
        if let Ok(path) = uri.to_file_path() {
            extra.push(path.join("lib"));
        }
    }
    FrontendSession::search_dirs_for_workspace(extra)
}
