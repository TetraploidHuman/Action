//! Compiler frontend: lex → parse → typecheck → load.
//!
//! No dependency on LLVM / codegen. See `doc/ARCHITECTURE.md`.

pub mod ast;
pub mod builtin;
pub mod checked;
pub mod config;
pub mod error;
pub mod exhaustive;
pub mod fmt;
pub mod hir;
pub mod lexer;
pub mod loader;
pub mod parser;
pub mod type_registry;
pub use type_registry as registry;
pub mod session;
pub mod typecheck;
pub mod types;
