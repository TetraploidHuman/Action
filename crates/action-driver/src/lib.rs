//! Driver helpers: unify CLI / test_runner compile orchestration.

mod compile;
mod emit;

pub use compile::{
    check_file, codegen_checked, effective_opt_level, format_loader_errors, CheckError,
};
pub use emit::{emit_diagnostics_json, emit_hir, report_check_errors};

/// Deprecated aliases kept for backward compatibility.
#[allow(deprecated)]
pub use compile::{compile_checked, load_checked, load_checked_errors};
