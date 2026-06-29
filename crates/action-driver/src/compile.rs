//! Shared load → compile helpers for CLI and test harnesses.

use action_codegen::CodeGen;
use action_frontend::checked::CheckedProgram;
use action_frontend::config::ProjectConfig;
use action_frontend::error::CompilerError;
use action_frontend::loader;
use inkwell::context::Context;
use std::path::Path;

/// Frontend type-check failures (structured diagnostics).
pub type CheckError = Vec<CompilerError>;

pub fn format_loader_errors(errors: &[CompilerError]) -> String {
    errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Type-check a source file and lower to HIR (`CheckedProgram`).
pub fn check_file(path: &Path, explain: bool) -> Result<CheckedProgram, CheckError> {
    let path_buf = path.to_path_buf();
    loader::check_file(&path_buf, explain)
}

/// Type-check failures formatted as plain text (legacy CLI path).
#[deprecated(
    since = "0.5.5",
    note = "use `check_file` and `emit::report_check_errors`"
)]
pub fn load_checked(path: &Path, explain: bool) -> Result<CheckedProgram, String> {
    check_file(path, explain).map_err(|errors| format_loader_errors(&errors))
}

/// Type-check a source file and lower to HIR.
#[deprecated(since = "0.5.5", note = "use `check_file`")]
pub fn load_checked_errors(path: &Path, explain: bool) -> Result<CheckedProgram, CheckError> {
    check_file(path, explain)
}

pub fn effective_opt_level(path: &Path, cli_opt: u8) -> u8 {
    ProjectConfig::find_and_load(path)
        .as_ref()
        .map(|c| c.effective_opt_level(cli_opt))
        .unwrap_or(cli_opt)
}

/// Build verified LLVM codegen from a checked program.
pub fn codegen_checked<'ctx>(
    context: &'ctx Context,
    entry_name: &str,
    checked: &CheckedProgram,
    opt: u8,
    target: &str,
) -> Result<CodeGen<'ctx>, String> {
    let target_opt = if target == "native" {
        None
    } else {
        Some(target.to_string())
    };
    let mut cg = CodeGen::new(context, entry_name, checked.registry.clone(), target_opt);
    cg.set_opt_level(opt);
    cg.compile_from_checked(checked)?;
    cg.verify()?;
    Ok(cg)
}

/// Build verified LLVM codegen from a checked program.
#[deprecated(since = "0.5.5", note = "use `codegen_checked`")]
pub fn compile_checked<'ctx>(
    context: &'ctx Context,
    entry_name: &str,
    checked: &CheckedProgram,
    opt: u8,
    target: &str,
) -> Result<CodeGen<'ctx>, String> {
    codegen_checked(context, entry_name, checked, opt, target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn format_loader_errors_joins_messages() {
        let errors = vec![
            CompilerError::new("first error".to_string()),
            CompilerError::new("second error".to_string()),
        ];
        let out = format_loader_errors(&errors);
        assert!(out.contains("first error"));
        assert!(out.contains("second error"));
        assert!(out.contains('\n'));
    }

    #[test]
    fn effective_opt_level_falls_back_to_cli() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        assert_eq!(effective_opt_level(&path, 2), 2);
    }

    #[test]
    fn check_file_on_missing_file() {
        let missing = PathBuf::from("/nonexistent/action_test_missing.ac");
        let result = check_file(&missing, false);
        match result {
            Err(errors) => {
                assert!(!errors.is_empty());
                let formatted = format_loader_errors(&errors);
                assert!(!formatted.is_empty());
            }
            Ok(_) => panic!("expected load to fail for missing file"),
        }
    }
}
