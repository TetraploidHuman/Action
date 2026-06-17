// Action Language Compiler — library crate
//
// Layered layout (see doc/ARCHITECTURE.md):
//   span       — source locations (no deps)
//   frontend   — lex / parse / typecheck / load
//   backend    — LLVM codegen + runtime IR
//   driver     — CLI, LSP, REPL (main binary)

pub mod span;

pub mod frontend;
pub mod backend;

// ── Backward-compatible re-exports (existing `crate::lexer` paths) ────────────
pub use span::Span;
pub use frontend::{
    ast, builtin, config, error, fmt, lexer, loader, parser, typecheck, types,
};
pub use backend::codegen;
pub use backend::CodeGen;

// Legacy alias: typecheck + codegen both used `builtin_registry`
pub use frontend::builtin as builtin_registry;

pub mod http_runtime;
pub mod lsp;
pub mod repl;
pub mod runtime_json;
pub mod runtime_threading;
pub mod test_runner;
