use action_frontend::checked::CheckedProgram;
use std::fs;
use std::path::Path;

pub fn emit_hir(checked: &CheckedProgram, src_path: &Path, to_stdout: bool) -> Result<(), String> {
    let json = checked
        .hir_json_pretty()
        .map_err(|e| format!("HIR serialization failed: {}", e))?;
    if to_stdout {
        println!("=== HIR JSON ===");
        println!("{}", json);
    } else {
        let out = src_path.with_extension("hir.json");
        fs::write(&out, json).map_err(|e| format!("Cannot write to '{}': {}", out.display(), e))?;
        println!("HIR written to: {}", out.display());
    }
    Ok(())
}
