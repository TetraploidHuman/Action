//! Shared load → compile helpers for CLI and test harnesses.

use crate::backend::CodeGen;
use crate::frontend::checked::CheckedProgram;
use crate::frontend::config::ProjectConfig;
use crate::frontend::error::CompilerError;
use crate::frontend::loader;
use inkwell::context::Context;
use std::fs;
use std::path::Path;

pub fn format_loader_errors(errors: &[CompilerError]) -> String {
    errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn load_checked(path: &Path, explain: bool) -> Result<CheckedProgram, String> {
    let path_buf = path.to_path_buf();
    loader::load_checked(&path_buf, explain).map_err(|errors| format_loader_errors(&errors))
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

/// Write HIR JSON to stdout or `<source>.hir.json`.
pub fn emit_hir(checked: &CheckedProgram, src_path: &Path, to_stdout: bool) -> Result<(), String> {
    let json = checked
        .hir_json_pretty()
        .map_err(|e| format!("HIR serialization failed: {}", e))?;
    if to_stdout {
        println!("=== HIR JSON ===");
        println!("{}", json);
    } else {
        let out = src_path.with_extension("hir.json");
        fs::write(&out, json)
            .map_err(|e| format!("Cannot write to '{}': {}", out.display(), e))?;
        println!("HIR written to: {}", out.display());
    }
    Ok(())
}
