use action_frontend::error::CompilerError;
use std::fs;
use std::path::Path;

pub fn report_check_errors(path: &Path, errors: &[CompilerError]) {
    if let Ok(source) = fs::read_to_string(path) {
        action_frontend::error::report_compiler_errors(
            &source,
            &path.to_string_lossy(),
            errors,
        );
    } else {
        for e in errors {
            eprintln!("Error: {}", e);
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use action_frontend::error::CompilerError;
    use std::path::PathBuf;

    #[test]
    fn report_check_errors_does_not_panic_on_missing_file() {
        let path = PathBuf::from("/nonexistent/action_report_test.ac");
        let errors = vec![CompilerError::new("type error".to_string())];
        report_check_errors(&path, &errors);
    }
}
