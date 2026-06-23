use action_frontend::error::CompilerError;
use std::fs;
use std::path::Path;

pub fn emit_diagnostics_json(
    errors: &[CompilerError],
    src_path: &Path,
    explain: bool,
    to_stdout: bool,
) -> Result<(), String> {
    let file = src_path.to_string_lossy();
    let json = action_frontend::error::diagnostics_to_json_pretty(errors, &file, explain)
        .map_err(|e| format!("Diagnostics serialization failed: {}", e))?;
    if to_stdout {
        print!("{}", json);
    } else {
        let out = src_path.with_extension("diagnostics.json");
        fs::write(&out, json).map_err(|e| format!("Cannot write to '{}': {}", out.display(), e))?;
        eprintln!("Diagnostics written to: {}", out.display());
    }
    Ok(())
}
