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
        "textDocument/hover" => {
            let result = parse_params::<lsp_types::HoverParams>(req)
                .and_then(|p| Ok(handlers::handle_hover(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/definition" => {
            let result = parse_params::<lsp_types::GotoDefinitionParams>(req)
                .and_then(|p| Ok(handlers::handle_goto_definition(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/completion" => {
            let result = parse_params::<lsp_types::CompletionParams>(req)
                .and_then(|p| Ok(handlers::handle_completion(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/semanticTokens/full" => {
            let result = parse_params::<lsp_types::SemanticTokensParams>(req)
                .and_then(|p| Ok(handlers::handle_semantic_tokens(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/documentSymbol" => {
            let result = parse_params::<lsp_types::DocumentSymbolParams>(req)
                .and_then(|p| Ok(handlers::handle_document_symbols(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/signatureHelp" => {
            let result = parse_params::<lsp_types::SignatureHelpParams>(req)
                .and_then(|p| Ok(handlers::handle_signature_help(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/references" => {
            let result = parse_params::<lsp_types::ReferenceParams>(req)
                .and_then(|p| Ok(handlers::handle_references(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/documentHighlight" => {
            let result = parse_params::<lsp_types::DocumentHighlightParams>(req)
                .and_then(|p| Ok(handlers::handle_document_highlight(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/prepareRename" => {
            let result = parse_params::<lsp_types::TextDocumentPositionParams>(req)
                .and_then(|p| Ok(handlers::handle_prepare_rename(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/rename" => {
            let result = parse_params::<lsp_types::RenameParams>(req)
                .and_then(|p| Ok(handlers::handle_rename(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/foldingRange" => {
            let result = parse_params::<lsp_types::FoldingRangeParams>(req)
                .and_then(|p| Ok(handlers::handle_folding_range(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/inlayHint" => {
            let result = parse_params::<lsp_types::InlayHintParams>(req)
                .and_then(|p| Ok(handlers::handle_inlay_hints(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/formatting" => {
            let result = parse_params::<lsp_types::DocumentFormattingParams>(req)
                .and_then(|p| Ok(handlers::handle_formatting(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "textDocument/codeAction" => {
            let result = parse_params::<lsp_types::CodeActionParams>(req)
                .and_then(|p| Ok(handlers::handle_code_actions(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        "workspace/symbol" => {
            let result = parse_params::<lsp_types::WorkspaceSymbolParams>(req)
                .and_then(|p| Ok(handlers::handle_workspace_symbol(state, p)))
                .unwrap_or(None);
            Response::new_ok(id, result)
        }
        _ => Response::new_ok(id, Option::<()>::None),
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
    builtin_enums_for_lsp(&mut registry);
    builtin_types_for_lsp(&mut registry);

    let mut type_env: HashMap<String, Type> = HashMap::new();

    // Try to load stdlib files
    let stdlib_dir = Path::new("lib");
    for filename in &["option.at", "result.at", "math.at"] {
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

fn builtin_enums_for_lsp(registry: &mut TypeRegistry) {
    // Register Option and Result enum types as builtins
    use crate::ast::{EnumVariantParam, Stmt, Type};

    let span = crate::lexer::Span::new(0, 1, 1).with_end(1);

    let option_variants = vec![
        crate::ast::EnumVariant {
            name: "Some".to_string(),
            params: vec![EnumVariantParam::Positional(Type::Named("T".to_string()))],
        },
        crate::ast::EnumVariant {
            name: "None".to_string(),
            params: vec![],
        },
    ];
    let option_stmt = Stmt::Enum {
        name: "Option".to_string(),
        type_params: vec!["T".to_string()],
        variants: option_variants,
        span,
    };
    let _ = registry.register(&option_stmt);

    let result_variants = vec![
        crate::ast::EnumVariant {
            name: "Ok".to_string(),
            params: vec![EnumVariantParam::Positional(Type::Named("T".to_string()))],
        },
        crate::ast::EnumVariant {
            name: "Err".to_string(),
            params: vec![EnumVariantParam::Positional(Type::Named("E".to_string()))],
        },
    ];
    let result_stmt = Stmt::Enum {
        name: "Result".to_string(),
        type_params: vec!["T".to_string(), "E".to_string()],
        variants: result_variants,
        span,
    };
    let _ = registry.register(&result_stmt);
}

fn builtin_types_for_lsp(registry: &mut TypeRegistry) {
    use crate::ast::{Stmt, Type};
    let span = crate::lexer::Span::new(0, 1, 1).with_end(1);
    // Register TimeoutError as a type alias
    let timeout_stmt = Stmt::TypeAlias {
        name: "TimeoutError".to_string(),
        type_params: vec![],
        definition: Type::Named("TimeoutError".to_string()),
        span,
    };
    let _ = registry.register(&timeout_stmt);
}
