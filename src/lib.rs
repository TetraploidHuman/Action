// Action Language Compiler — library crate (facade)
//
// Layered layout (see doc/ARCHITECTURE.md):
//   action-span       — source locations
//   action-frontend   — lex / parse / typecheck / load
//   action-codegen    — LLVM codegen + runtime IR
//   action-driver     — compile orchestration (load → compile → emit)
//   action-cli        — CLI binary (crates/action-cli/src/main.rs)
//   action (root)     — backward-compatible re-exports

pub use action_frontend as frontend;
pub use action_span as span;

pub use action_codegen as codegen;
pub use action_codegen::CodeGen;
pub use action_driver as driver;
pub use action_frontend::{
    ast, builtin, checked, config, error, fmt, hir, lexer, loader, parser, registry, session,
    typecheck, types,
};
pub use action_span::Span;

pub use action_frontend::builtin as builtin_registry;

pub use action_lsp as lsp;

// Link host runtime symbols (JSON/HTTP/threading) for JIT; AOT uses libaction_host_rt.a via build.rs.
use action_host_rt as _;
