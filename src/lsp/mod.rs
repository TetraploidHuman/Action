pub mod document;
pub mod handlers;
pub mod position;
pub mod project;
pub mod symbols;

use std::collections::HashMap;
use std::path::Path;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::{
    CodeActionProviderCapability, CompletionOptions, InitializeResult, RenameOptions,
    SemanticTokensOptions, ServerCapabilities, SignatureHelpOptions, TextDocumentSyncCapability,
    TextDocumentSyncKind,
};

use crate::ast::Type;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::typecheck::{TypeChecker, TypeRegistry};

use self::handlers::ServerState;
use self::project::Project;

/// Start the LSP server on stdin/stdout
pub fn start_lsp() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Action LSP server starting...");

    let (connection, io_threads) = Connection::stdio();

    // Initialize
    let (init_id, _init_params) = connection.initialize_start()?;

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

    // Load stdlib
    let (stdlib_registry, stdlib_type_env) = load_stdlib_context();

    let project = Project::new(stdlib_registry, stdlib_type_env);
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

/// Pre-load stdlib modules and build a combined TypeRegistry + type_env
fn load_stdlib_context() -> (TypeRegistry, HashMap<String, Type>) {
    let mut registry = TypeRegistry::new();

    // Always register built-in types
    builtin_types_for_lsp(&mut registry);

    let mut type_env: HashMap<String, Type> = HashMap::new();

    // Try to load stdlib files
    let stdlib_dir = Path::new("lib");
    for filename in &["math.at"] {
        let path = stdlib_dir.join(filename);
        if let Ok(source) = std::fs::read_to_string(&path) {
            let mut lexer = Lexer::new(&source);
            let tokens = lexer.tokenize();
            let mut parser = Parser::new(tokens);
            let (stmts, _errors) = parser.parse_program_recover();

            for stmt in &stmts {
                let _ = registry.register(stmt);
            }

            let program = crate::ast::Program { stmts };
            let mut checker = TypeChecker::new(registry.clone());
            checker.seed_type_env(&type_env);
            let _ = checker.check(&program);
            for (k, v) in checker.type_env() {
                type_env.entry(k.clone()).or_insert_with(|| v.clone());
            }
            registry = checker.registry_ref().clone();
        }
    }

    (registry, type_env)
}

fn builtin_types_for_lsp(_registry: &mut TypeRegistry) {
    // Built-in types are now registered elsewhere; keep this function as a hook
    // for future LSP-specific type registrations.
}
