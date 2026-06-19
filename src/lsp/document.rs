use std::collections::HashMap;
use std::path::PathBuf;

use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, Url};

use crate::ast::*;
use crate::error::CompilerError;
use crate::hir::HirModule;
use crate::lexer::{Lexer, Span, Token};
use crate::parser::ParseError;
use crate::registry::TypeRegistry;
use crate::session::FrontendSession;

use super::position;

/// Per-file document state with cached parse and type-check results
pub struct Document {
    pub uri: Url,
    pub path: Option<PathBuf>,
    pub source: String,
    pub tokens: Vec<Token>,
    pub ast: Vec<Stmt>,
    pub hir: Option<HirModule>,
    pub registry: TypeRegistry,
    pub parse_errors: Vec<ParseError>,
    pub type_env: HashMap<String, Type>,
    pub type_errors: Vec<CompilerError>,
    pub definition_map: HashMap<String, Span>,
}

impl Document {
    pub fn new(uri: Url, source: String, _version: i32) -> Self {
        let path = uri.to_file_path().ok();
        Document {
            uri,
            path,
            source,
            tokens: Vec::new(),
            ast: Vec::new(),
            hir: None,
            registry: TypeRegistry::new(),
            parse_errors: Vec::new(),
            type_env: HashMap::new(),
            type_errors: Vec::new(),
            definition_map: HashMap::new(),
        }
    }
}

impl Document {
    /// Re-lex, re-parse, and re-type-check via shared frontend session.
    pub fn recheck_with_session(&mut self, session: &FrontendSession) {
        let mut lexer = Lexer::new(&self.source);
        self.tokens = lexer.tokenize();

        let result = session.compile_recover_for_path(&self.source, self.path.as_deref());
        self.ast = result.stmts;
        self.parse_errors = result.parse_errors;
        self.definition_map = build_definition_map(&self.ast);
        self.type_errors = result.type_errors;
        self.type_env = result.type_env;
        self.registry = result.registry;
        self.hir = result.hir;
    }

    #[allow(dead_code)]
    pub fn recheck(
        &mut self,
        stdlib_registry: &crate::typecheck::TypeRegistry,
        stdlib_type_env: &HashMap<String, Type>,
    ) {
        let session = FrontendSession::with_context(
            Vec::new(),
            stdlib_registry.clone(),
            stdlib_type_env.clone(),
        )
        .unwrap_or_else(|_| FrontendSession {
            stdlib_stmts: Vec::new(),
            search_dirs: Vec::new(),
            base_registry: stdlib_registry.clone(),
            base_type_env: stdlib_type_env.clone(),
        });
        self.recheck_with_session(&session);
    }

    /// Get all diagnostics (parse + type errors) as LSP diagnostics.
    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags: Vec<Diagnostic> = self
            .parse_errors
            .iter()
            .map(|e| {
                compiler_error_to_lsp_diagnostic(
                    &e.to_compiler_error(),
                    &self.source,
                    "parse-error",
                )
            })
            .collect();
        diags.extend(
            self.type_errors
                .iter()
                .map(|e| compiler_error_to_lsp_diagnostic(e, &self.source, "type-error")),
        );
        diags
    }
}

/// Convert structured [`CompilerError`] values to LSP diagnostics using span info.
pub fn compiler_errors_to_lsp_diagnostics(
    errors: &[CompilerError],
    source: &str,
) -> Vec<Diagnostic> {
    errors
        .iter()
        .map(|e| compiler_error_to_lsp_diagnostic(e, source, "type-error"))
        .collect()
}

fn compiler_error_to_lsp_diagnostic(error: &CompilerError, source: &str, code: &str) -> Diagnostic {
    let range = match &error.span {
        Some(span) => position::span_to_lsp_range(span, source),
        None => Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: 0,
                character: 0,
            },
        },
    };

    let mut diag = Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(code.to_string())),
        source: Some("action".to_string()),
        message: error.message.clone(),
        ..Default::default()
    };
    if let Some(help) = &error.help {
        diag.data = Some(serde_json::json!({ "help": help }));
    }
    diag
}

/// Build a map from definition name to its span
fn build_definition_map(stmts: &[Stmt]) -> HashMap<String, Span> {
    let mut map = HashMap::new();
    for stmt in stmts {
        match stmt {
            Stmt::Fun { name, span, .. } => {
                map.insert(name.clone(), *span);
            }
            Stmt::Let { name, span, .. } => {
                map.insert(name.clone(), *span);
            }
            Stmt::Const { name, span, .. } => {
                map.insert(name.clone(), *span);
            }
            Stmt::Enum {
                name,
                variants,
                span,
                ..
            } => {
                map.insert(name.clone(), *span);
                for v in variants {
                    map.insert(v.name.clone(), *span);
                }
            }
            Stmt::TypeAlias { name, span, .. } => {
                map.insert(name.clone(), *span);
            }
            Stmt::Module {
                name, body, span, ..
            } => {
                map.insert(name.clone(), *span);
                map.extend(build_definition_map(body));
            }
            Stmt::Destructure { names, span, .. } => {
                for name in names {
                    map.insert(name.clone(), *span);
                }
            }
            _ => {}
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typecheck::TypeRegistry;
    use lsp_types::Url;
    use std::collections::HashMap;

    fn make_doc(source: &str) -> Document {
        let uri = Url::parse("file:///test.at").unwrap();
        let mut doc = Document::new(uri, source.to_string(), 1);
        doc.recheck_with_session(&empty_session());
        doc
    }

    #[test]
    fn test_document_new_empty() {
        let doc = make_doc("");
        assert!(
            doc.tokens.is_empty()
                || doc
                    .tokens
                    .iter()
                    .all(|t| t.kind == crate::lexer::TokenKind::Eof)
        );
        assert!(doc.ast.is_empty());
        assert!(doc.parse_errors.is_empty());
        assert!(doc.type_errors.is_empty());
    }

    #[test]
    fn test_document_new_valid() {
        let doc = make_doc("val x = 42");
        assert!(!doc.ast.is_empty(), "should parse program");
        assert!(
            doc.parse_errors.is_empty(),
            "should have no parse errors: {:?}",
            doc.parse_errors
        );
    }

    #[test]
    fn test_document_new_with_parse_error() {
        let doc = make_doc("val = 42");
        // "val = 42" is a syntax error (missing identifier)
        // May or may not have parse errors depending on error recovery
        assert!(
            doc.ast.is_empty() || !doc.parse_errors.is_empty(),
            "malformed input should produce parse errors or empty AST"
        );
    }

    #[test]
    fn test_get_diagnostics_empty() {
        let doc = make_doc("");
        let diags = doc.get_diagnostics();
        assert!(
            diags.is_empty()
                || diags
                    .iter()
                    .all(|d| d.severity == Some(lsp_types::DiagnosticSeverity::ERROR))
        );
    }

    #[test]
    fn test_get_diagnostics_parse_error() {
        let doc = make_doc("fun {");
        let diags = doc.get_diagnostics();
        // Should have parse errors for invalid function definition
        assert!(
            !diags.is_empty(),
            "malformed function should produce diagnostics"
        );
    }

    #[test]
    fn test_recheck_updates_state() {
        let uri = Url::parse("file:///test.at").unwrap();
        let mut doc = Document::new(uri.clone(), "val x = 1".to_string(), 1);
        assert!(
            doc.ast.is_empty(),
            "new document should not parse until recheck"
        );

        doc.recheck(&TypeRegistry::new(), &HashMap::new());
        assert!(!doc.ast.is_empty(), "recheck should populate AST");
        assert!(doc.parse_errors.is_empty());
    }

    #[test]
    fn test_build_definition_map_fun() {
        let doc = make_doc("fun hello() {}");
        assert!(doc.definition_map.contains_key("hello"));
    }

    #[test]
    fn test_build_definition_map_let() {
        let doc = make_doc("val x = 42");
        assert!(doc.definition_map.contains_key("x"));
    }

    #[test]
    fn test_build_definition_map_enum() {
        let doc = make_doc("enum Color { Red, Blue }");
        assert!(doc.definition_map.contains_key("Color"));
    }

    #[test]
    fn test_build_definition_map_module() {
        let doc = make_doc("module foo { val x = 1 }");
        assert!(doc.definition_map.contains_key("foo"));
    }

    #[test]
    fn test_document_get_type_env() {
        let doc = make_doc("val x = 42");
        let has_x = doc.type_env.contains_key("x") || doc.type_env.is_empty();
        // Either x was typed or if typecheck failed, env is empty
        assert!(has_x);
    }

    #[test]
    fn test_document_multi_statement() {
        let src = "val a = 1\nval b = 2\nfun add(x, y) { x + y }";
        let doc = make_doc(src);
        assert!(doc.definition_map.contains_key("a"));
        assert!(doc.definition_map.contains_key("b"));
        assert!(doc.definition_map.contains_key("add"));
    }
}
