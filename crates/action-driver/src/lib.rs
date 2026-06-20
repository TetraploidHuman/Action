//! Driver helpers: unify CLI / test_runner compile orchestration.

mod compile;

pub use compile::{
    compile_checked, effective_opt_level, emit_diagnostics_json, emit_hir, format_loader_errors,
    load_checked, load_checked_errors,
};
