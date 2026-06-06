use std::collections::HashMap;

use lsp_types::Url;

use crate::ast::Type;
use crate::lexer::{Span, TokenKind};
use crate::typecheck::TypeRegistry;

use super::document::Document;
use super::position;

/// Symbol kind for cross-file indexing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    Function,
    Variable,
    Constant,
    Type,
    Enum,
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
            SymbolKind::Enum => lsp_types::SymbolKind::ENUM,
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
    pub stdlib_registry: TypeRegistry,
    pub stdlib_type_env: HashMap<String, Type>,
    pub symbol_index: HashMap<String, Vec<SymbolLocation>>,
}

impl Project {
    pub fn new(stdlib_registry: TypeRegistry, stdlib_type_env: HashMap<String, Type>) -> Self {
        Project {
            documents: HashMap::new(),
            stdlib_registry,
            stdlib_type_env,
            symbol_index: HashMap::new(),
        }
    }

    /// Add or update a document, recheck it, and refresh the symbol index
    pub fn update_document(
        &mut self,
        uri: &Url,
        source: String,
        version: i32,
    ) -> Vec<lsp_types::Diagnostic> {
        let mut doc = Document::new(uri.clone(), source, version);
        doc.recheck(&self.stdlib_registry, &self.stdlib_type_env);
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
                        deprecated: None,
                    });
                }
            }
        }
        results.truncate(100);
        results
    }
}
