//! Shared load → compile helpers for CLI and test harnesses.

use action_codegen::CodeGen;
use action_frontend::checked::CheckedProgram;
use action_frontend::config::ProjectConfig;
use action_frontend::error::CompilerError;
use action_frontend::loader;
use inkwell::context::Context;
use std::path::Path;

pub fn format_loader_errors(errors: &[CompilerError]) -> String {
    errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn load_checked(path: &Path, explain: bool) -> Result<CheckedProgram, String> {
    load_checked_errors(path, explain).map_err(|errors| format_loader_errors(&errors))
}

pub fn load_checked_errors(
    path: &Path,
    explain: bool,
) -> Result<CheckedProgram, Vec<CompilerError>> {
    let path_buf = path.to_path_buf();
    loader::load_checked(&path_buf, explain)
}

pub fn effective_opt_level(path: &Path, cli_opt: u8) -> u8 {
    ProjectConfig::find_and_load(path)
        .as_ref()
        .map(|c| c.effective_opt_level(cli_opt))
        .unwrap_or(cli_opt)
}

pub fn compile_checked<'ctx>(
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
    cg.compile_checked(checked)?;
    cg.verify()?;
    Ok(cg)
}
