#[cfg(test)]
mod tests {
    use crate::handlers::{
        document::{handle_did_change, handle_did_close, handle_did_open, handle_formatting},
        editing::handle_completion,
        navigation::{handle_goto_definition, handle_hover, handle_references},
        symbols::{handle_document_symbols, handle_semantic_tokens, handle_workspace_symbol},
        ServerState,
    };
    use crate::project::Project;
    use action_frontend::typecheck::TypeRegistry;
    use lsp_types::Url;
    use lsp_types::{
        CompletionParams, DidChangeTextDocumentParams, DidCloseTextDocumentParams,
        DidOpenTextDocumentParams, DocumentFormattingParams, DocumentSymbolParams,
        GotoDefinitionParams, HoverParams, Position, ReferenceParams, SemanticTokensParams,
        TextDocumentContentChangeEvent, TextDocumentItem, TextDocumentPositionParams,
        VersionedTextDocumentIdentifier, WorkspaceSymbolParams,
    };
    use std::collections::HashMap;

    fn make_state(source: &str) -> ServerState {
        let proj = Project::with_stdlib(TypeRegistry::new(), HashMap::new(), Vec::new());
        let mut state = ServerState::new(proj);
        let uri = Url::parse("file:///test.ac").unwrap();
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
        Url::parse("file:///test.ac").unwrap()
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
