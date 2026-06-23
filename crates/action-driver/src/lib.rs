//! Driver helpers: unify CLI / test_runner compile orchestration.

mod compile;
mod emit;

pub use compile::{
    compile_checked, effective_opt_level, format_loader_errors, load_checked, load_checked_errors,
};
pub use emit::{emit_diagnostics_json, emit_hir};
