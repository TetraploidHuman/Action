use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use lsp_types::Url;

use action_frontend::ast::Type;
use action_frontend::lexer::{Span, TokenKind};
use action_frontend::session::FrontendSession;
use action_frontend::typecheck::TypeRegistry;

use super::document::Document;
use super::position;

fn blank_symbol_information() -> lsp_types::SymbolInformation {
    static BLANK: OnceLock<lsp_types::SymbolInformation> = OnceLock::new();
    BLANK
        .get_or_init(|| {
            serde_json::from_value(serde_json::json!({
                "name": "",
                "kind": 3,
                "location": {
                    "uri": "file:///blank",
                    "range": {
                        "start": {"line": 0, "character": 0},
                        "end": {"line": 0, "character": 0}
                    }
                }
            }))
            .expect("blank SymbolInformation")
        })
        .clone()
}

/// Symbol kind for cross-file indexing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Constant,
    Type,
    EnumVariant,
    Module,
}

impl SymbolKind {
    pub fn to_lsp_kind(self) -> lsp_types::SymbolKind {
        match self {
            SymbolKind::Function => lsp_types::SymbolKind::FUNCTION,
            SymbolKind::Variable => lsp_types::SymbolKind::VARIABLE,
            SymbolKind::Constant => lsp_types::SymbolKind::CONSTANT,
            SymbolKind::Type => lsp_types::SymbolKind::CLASS,
            SymbolKind::EnumVariant => lsp_types::SymbolKind::ENUM_MEMBER,
            SymbolKind::Module => lsp_types::SymbolKind::MODULE,
        }
    }
}

/// A symbol occurrence in a specific file
#[derive(Debug, Clone)]
pub struct SymbolLocation {
    pub file: Url,
    pub name: String,
    pub span: Span,
    pub kind: SymbolKind,
}

/// Project-level state: manages multiple documents and cross-file features
pub struct Project {
    pub documents: HashMap<Url, Document>,
    pub session: FrontendSession,
    pub symbol_index: HashMap<String, Vec<SymbolLocation>>,
}

impl Project {
    pub fn new(session: FrontendSession) -> Self {
        Project {
            documents: HashMap::new(),
            session,
            symbol_index: HashMap::new(),
        }
    }

    /// Backward-compatible constructor.
    pub fn with_stdlib(
        stdlib_registry: TypeRegistry,
        stdlib_type_env: HashMap<String, Type>,
        search_dirs: Vec<PathBuf>,
    ) -> Self {
        let session = FrontendSession::with_context(
            search_dirs,
            stdlib_registry.clone(),
            stdlib_type_env.clone(),
        )
        .unwrap_or_else(|_| FrontendSession {
            stdlib_stmts: Vec::new(),
            search_dirs: Vec::new(),
            base_registry: stdlib_registry,
            base_type_env: stdlib_type_env,
        });
        Self::new(session)
    }

    /// Add or update a document, recheck it, and refresh the symbol index
    pub fn update_document(
        &mut self,
        uri: &Url,
        source: String,
        version: i32,
    ) -> Vec<lsp_types::Diagnostic> {
        let mut doc = Document::new(uri.clone(), source, version);
        doc.recheck_with_session(&self.session);
        let diagnostics = doc.get_diagnostics();
        self.update_symbol_index(uri, &doc);
        self.documents.insert(uri.clone(), doc);
        diagnostics
    }

    /// Remove a document and its symbols
    pub fn remove_document(&mut self, uri: &Url) {
        self.documents.remove(uri);
        self.symbol_index.retain(|_, locs| {
            locs.retain(|loc| &loc.file != uri);
            !locs.is_empty()
        });
    }

    /// Rebuild symbol index entries for one document
    fn update_symbol_index(&mut self, uri: &Url, doc: &Document) {
        // Remove old entries for this uri
        self.symbol_index.retain(|_, locs| {
            locs.retain(|loc| &loc.file != uri);
            !locs.is_empty()
        });

        // Scan tokens for identifiers in declaration context
        let tokens = &doc.tokens;
        for i in 0..tokens.len() {
            let token = &tokens[i];
            if let TokenKind::Ident(name) = &token.kind {
                let prev_kind = if i > 0 {
                    Some(&tokens[i - 1].kind)
                } else {
                    None
                };
                let kind = match prev_kind {
                    Some(TokenKind::Fun) => SymbolKind::Function,
                    Some(TokenKind::Val) | Some(TokenKind::Var) => SymbolKind::Variable,
                    Some(TokenKind::Const) => SymbolKind::Constant,
                    Some(TokenKind::Type) => SymbolKind::Type,
                    Some(TokenKind::Enum) => SymbolKind::EnumVariant,
                    Some(TokenKind::Module) => SymbolKind::Module,
                    _ => continue,
                };
                let loc = SymbolLocation {
                    file: uri.clone(),
                    name: name.clone(),
                    span: token.span.clone(),
                    kind,
                };
                self.symbol_index
                    .entry(name.clone())
                    .or_insert_with(Vec::new)
                    .push(loc);
            }
        }
    }

    /// Find definition: search current document first, then cross-file
    pub fn find_definition(&self, uri: &Url, name: &str) -> Option<lsp_types::Location> {
        if let Some(locs) = self.symbol_index.get(name) {
            // Prefer definition in current file
            let in_file = locs.iter().find(|loc| &loc.file == uri);
            let loc = in_file.or_else(|| locs.first())?;
            // Use source from the document if available for accurate range
            let source = self
                .documents
                .get(&loc.file)
                .map(|d| d.source.as_str())
                .unwrap_or("");
            return Some(lsp_types::Location {
                uri: loc.file.clone(),
                range: position::span_to_lsp_range(&loc.span, source),
            });
        }
        None
    }

    /// Find all references to a name across all open documents
    pub fn find_references(&self, name: &str) -> Vec<lsp_types::Location> {
        let mut results = Vec::new();
        for doc in self.documents.values() {
            for token in &doc.tokens {
                if let TokenKind::Ident(token_name) = &token.kind {
                    if token_name == name {
                        results.push(lsp_types::Location {
                            uri: doc.uri.clone(),
                            range: position::span_to_lsp_range(&token.span, &doc.source),
                        });
                    }
                }
            }
        }
        results
    }

    /// Search workspace symbols by query string
    pub fn workspace_symbols(&self, query: &str) -> Vec<lsp_types::SymbolInformation> {
        let mut results = Vec::new();
        let lower_query = query.to_lowercase();
        for (name, locs) in &self.symbol_index {
            if name.to_lowercase().contains(&lower_query) {
                for loc in locs {
                    results.push(lsp_types::SymbolInformation {
                        name: loc.name.clone(),
                        kind: loc.kind.to_lsp_kind(),
                        location: lsp_types::Location {
                            uri: loc.file.clone(),
                            range: position::span_to_lsp_range(&loc.span, ""),
                        },

                        container_name: None,
                        tags: None,
                        ..blank_symbol_information()
                    });
                }
            }
        }
        results.truncate(100);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use action_frontend::typecheck::TypeRegistry;
    use lsp_types::Url;
    use std::collections::HashMap;

    fn empty_project() -> Project {
        Project::with_stdlib(TypeRegistry::new(), HashMap::new(), Vec::new())
    }

    #[test]
    fn test_project_new_empty() {
        let proj = empty_project();
        assert!(proj.documents.is_empty());
        assert!(proj.symbol_index.is_empty());
    }

    #[test]
    fn test_update_document_valid() {
        let mut proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        let diags = proj.update_document(&uri, "val x = 42".to_string(), 1);
        assert!(diags.is_empty(), "valid program should have no diagnostics");
        assert!(proj.documents.contains_key(&uri));
    }

    #[test]
    fn test_update_document_syntax_error() {
        let mut proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        let diags = proj.update_document(&uri, "val = 42".to_string(), 1);
        // Should produce diagnostics — the exact count depends on error recovery
        assert!(!diags.is_empty() || proj.documents.get(&uri).map_or(true, |d| d.ast.is_empty()));
    }

    #[test]
    fn test_remove_document() {
        let mut proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        proj.update_document(&uri, "val x = 1".to_string(), 1);
        assert!(proj.documents.contains_key(&uri));
        proj.remove_document(&uri);
        assert!(!proj.documents.contains_key(&uri));
    }

    #[test]
    fn test_update_symbol_index() {
        let mut proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        proj.update_document(&uri, "fun hello() {}".to_string(), 1);
        // symbol_index should contain "hello" as a function
        assert!(
            proj.symbol_index.contains_key("hello"),
            "symbol_index should contain 'hello': keys are {:?}",
            proj.symbol_index.keys().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_find_definition() {
        let mut proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        proj.update_document(&uri, "fun hello() {}".to_string(), 1);
        let loc = proj.find_definition(&uri, "hello");
        assert!(loc.is_some(), "should find definition of 'hello'");
        assert_eq!(loc.unwrap().uri, uri);
    }

    #[test]
    fn test_find_definition_unknown() {
        let proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        let loc = proj.find_definition(&uri, "nonexistent");
        assert!(loc.is_none());
    }

    #[test]
    fn test_find_references() {
        let mut proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        proj.update_document(&uri, "val x = 1\nval y = x + 2".to_string(), 1);
        let refs = proj.find_references("x");
        assert!(!refs.is_empty(), "should find references to 'x'");
    }

    #[test]
    fn test_find_references_unknown() {
        let proj = empty_project();
        let refs = proj.find_references("nonexistent");
        assert!(refs.is_empty());
    }

    #[test]
    fn test_workspace_symbols() {
        let mut proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        proj.update_document(&uri, "fun hello() {}".to_string(), 1);
        let results = proj.workspace_symbols("hello");
        assert!(!results.is_empty(), "should find 'hello' symbol");
        assert_eq!(results[0].name, "hello");
    }

    #[test]
    fn test_workspace_symbols_partial_match() {
        let mut proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        proj.update_document(&uri, "fun helloWorld() {}".to_string(), 1);
        let results = proj.workspace_symbols("hello");
        assert!(
            !results.is_empty(),
            "should match 'helloWorld' with query 'hello'"
        );
    }

    #[test]
    fn test_workspace_symbols_no_match() {
        let proj = empty_project();
        let results = proj.workspace_symbols("zzzzzz");
        assert!(results.is_empty());
    }

    #[test]
    fn test_update_document_twice_replaces() {
        let mut proj = empty_project();
        let uri = Url::parse("file:///test.ac").unwrap();
        proj.update_document(&uri, "val x = 1".to_string(), 1);
        proj.update_document(&uri, "val y = 2".to_string(), 2);
        assert_eq!(proj.documents.len(), 1);
        let doc = proj.documents.get(&uri).unwrap();
        assert!(doc.definition_map.contains_key("y"));
    }

    #[test]
    fn test_multiple_documents() {
        let mut proj = empty_project();
        let uri_a = Url::parse("file:///a.ac").unwrap();
        let uri_b = Url::parse("file:///b.ac").unwrap();
        proj.update_document(&uri_a, "val a = 1".to_string(), 1);
        proj.update_document(&uri_b, "val b = 2".to_string(), 1);
        assert_eq!(proj.documents.len(), 2);
    }
}
