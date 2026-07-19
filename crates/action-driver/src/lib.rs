//! Driver helpers: unify CLI / test_runner compile orchestration.

mod bootstrap;
mod compile;
mod emit;

pub use bootstrap::{
    check_file_bootstrap, emit_bootstrap_hir, find_project_root, is_bootstrap_allowlisted,
    verify_bootstrap_hir, BootstrapCheckResult, BOOTSTRAP_FRONTEND_ALLOWLIST,
};
pub use compile::{
    check_file, codegen_checked, effective_opt_level, format_loader_errors, CheckError,
};
pub use emit::{emit_diagnostics_json, emit_hir, report_check_errors};

/// Deprecated aliases kept for backward compatibility.
#[allow(deprecated)]
pub use compile::{compile_checked, load_checked, load_checked_errors};
