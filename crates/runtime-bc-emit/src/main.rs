//! Build-time helper: emit LLVM bitcode for the Action runtime module.
//! Built as a separate crate so action's build.rs can invoke it without deadlocking.

use action_codegen::CodeGen;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let out_path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: runtime-bc-emit <output.bc>");
        process::exit(1);
    });

    match emit_runtime_bitcode(Path::new(&out_path)) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("runtime-bc-emit: {e}");
            process::exit(1);
        }
    }
}

fn emit_runtime_bitcode(path: &Path) -> Result<(), String> {
    let bitcode = CodeGen::generate_runtime_bitcode()?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    fs::write(path, &bitcode).map_err(|e| e.to_string())?;
    Ok(())
}
