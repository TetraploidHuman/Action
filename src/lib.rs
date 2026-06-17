// Action Language Compiler — library crate
// Core modules re-exported for the CLI binary and external consumers.

pub mod ast;
#[path = "codegen/builtin_registry.rs"]
pub mod builtin_registry;
pub mod codegen;
pub mod config;
pub mod error;
pub mod fmt;
pub mod http_runtime;
pub mod lexer;
pub mod loader;
pub mod lsp;
pub mod parser;
pub mod repl;
pub mod runtime_json;
pub mod runtime_threading;
pub mod test_runner;
pub mod typecheck;
pub mod types;
