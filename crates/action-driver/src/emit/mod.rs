//! Emit helpers: HIR JSON and diagnostics JSON.

mod diagnostics;
mod hir;

pub use diagnostics::emit_diagnostics_json;
pub use hir::emit_hir;
