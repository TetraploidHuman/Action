// Action Language Compiler — library crate (facade)
//
// Layered layout (see doc/ARCHITECTURE.md):
//   action-span       — source locations
//   action-frontend   — lex / parse / typecheck / load
//   action-codegen    — LLVM codegen + runtime IR
//   action-driver     — compile orchestration (load → compile → emit)
//   action (root)     — CLI, LSP, REPL + backward-compatible re-exports

pub mod backend;

pub use action_frontend as frontend;
pub use action_span as span;

// ── Backward-compatible re-exports (existing `crate::lexer` paths) ────────────
pub use action_codegen as codegen;
pub use action_codegen::CodeGen;
pub use action_frontend::{
    ast, builtin, checked, config, error, fmt, hir, lexer, loader, parser, registry, session,
    typecheck, types,
};
pub use action_span::Span;

// Legacy alias: typecheck + codegen both used `builtin_registry`
pub use action_frontend::builtin as builtin_registry;

pub mod driver;
pub mod http_runtime;
pub mod lsp;
pub mod repl;
pub mod runtime_json;
pub mod runtime_threading;
pub mod test_runner;
