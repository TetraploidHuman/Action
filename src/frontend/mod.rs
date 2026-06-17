//! Compiler frontend: lex → parse → typecheck → load.
//!
//! No dependency on LLVM / codegen. See `doc/ARCHITECTURE.md`.

pub mod ast;
pub mod builtin;
pub mod config;
pub mod error;
pub mod fmt;
pub mod lexer;
pub mod loader;
pub mod parser;
pub mod typecheck;
pub mod types;
