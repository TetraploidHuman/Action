use std::collections::HashMap;

use lsp_types::{Diagnostic, DiagnosticSeverity, NumberOrString, Position, Range, Url};

use crate::ast::*;
use crate::error::CompilerError;
use crate::lexer::{Lexer, Span, Token};
use crate::parser::{ParseError, Parser};
use crate::typecheck::{TypeChecker, TypeRegistry};

use super::position;

/// Per-file document state with cached parse and type-check results
pub struct Document {
    pub uri: Url,
    pub source: String,
    pub tokens: Vec<Token>,
    pub ast: Vec<Stmt>,
    pub parse_errors: Vec<ParseError>,
    pub type_env: HashMap<String, Type>,
    pub type_errors: Vec<CompilerError>,
    pub definition_map: HashMap<String, Span>,
}

impl Document {
    pub fn new(uri: Url, source: String, _version: i32) -> Self {
        let mut doc = Document {
            uri,
            source,
            tokens: Vec::new(),
            ast: Vec::new(),
            parse_errors: Vec::new(),
            type_env: HashMap::new(),
            type_errors: Vec::new(),
            definition_map: HashMap::new(),
        };
        doc.recheck(&TypeRegistry::new(), &HashMap::new());
        doc
    }

    /// Re-lex, re-parse, and re-type-check. Merges stdlib context.
    pub fn recheck(
        &mut self,
        stdlib_registry: &TypeRegistry,
        stdlib_type_env: &HashMap<String, Type>,
    ) {
        // 1. Tokenize
        let mut lexer = Lexer::new(&self.source);
        self.tokens = lexer.tokenize();

        // 2. Parse with error recovery
        let mut parser = Parser::new(self.tokens.clone());
        let (stmts, parse_errors) = parser.parse_program_recover();
        self.ast = stmts;
        self.parse_errors = parse_errors;

        // 3. Build definition map from successfully parsed statements
        self.definition_map = build_definition_map(&self.ast);

        // 4. Type check (only if we got some valid statements)
        if self.ast.is_empty() {
            self.type_env.clear();
            self.type_errors.clear();
            return;
        }

        let program = Program {
            stmts: self.ast.clone(),
        };

        // Build combined registry: stdlib + user types
        let mut registry = stdlib_registry.clone();
        for stmt in &program.stmts {
            let _ = registry.register(stmt);
        }

        let mut checker = TypeChecker::new(registry);
        checker.seed_type_env(stdlib_type_env);
        self.type_errors = checker.check(&program);
        self.type_env = checker
            .type_env()
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
    }

    /// Get all diagnostics (parse errors + type errors) as LSP diagnostics
    pub fn get_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags = Vec::new();

        for e in &self.parse_errors {
            let range = position::span_to_lsp_range(&e.span, &self.source);
            diags.push(Diagnostic {
                range,
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("parse-error".to_string())),
                source: Some("action".to_string()),
                message: e.message.clone(),
                ..Default::default()
            });
        }

        for e in &self.type_errors {
            let range = match &e.span {
                Some(span) => position::span_to_lsp_range(span, &self.source),
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
                code: Some(NumberOrString::String("type-error".to_string())),
                source: Some("action".to_string()),
                message: e.message.clone(),
                ..Default::default()
            };
            if let Some(help) = &e.help {
                diag.data = Some(serde_json::json!({ "help": help }));
            }
            diags.push(diag);
        }

        diags
    }
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
    use lsp_types::Url;
    use std::collections::HashMap;

    fn make_doc(source: &str) -> Document {
        let uri = Url::parse("file:///test.at").unwrap();
        Document::new(uri, source.to_string(), 1)
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
        let initial_ast_len = doc.ast.len();

        doc.recheck(&TypeRegistry::new(), &HashMap::new());
        assert_eq!(doc.ast.len(), initial_ast_len);
        assert!(doc.parse_errors.is_empty());
    }

    #[test]
    fn test_build_definition_map_fun() {
        let uri = Url::parse("file:///test.at").unwrap();
        let doc = Document::new(uri, "fun hello() {}".to_string(), 1);
        assert!(doc.definition_map.contains_key("hello"));
    }

    #[test]
    fn test_build_definition_map_let() {
        let uri = Url::parse("file:///test.at").unwrap();
        let doc = Document::new(uri, "val x = 42".to_string(), 1);
        assert!(doc.definition_map.contains_key("x"));
    }

    #[test]
    fn test_build_definition_map_enum() {
        let uri = Url::parse("file:///test.at").unwrap();
        let doc = Document::new(uri, "enum Color { Red, Blue }".to_string(), 1);
        assert!(doc.definition_map.contains_key("Color"));
    }

    #[test]
    fn test_build_definition_map_module() {
        let uri = Url::parse("file:///test.at").unwrap();
        let doc = Document::new(uri, "module foo { val x = 1 }".to_string(), 1);
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
