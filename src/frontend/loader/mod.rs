mod resolve;
mod stdlib;

pub use resolve::{resolve_imports, transform_module_access};
pub use stdlib::{builtin_types, load_path_dependencies, load_stdlib};

use crate::frontend::ast::Program;
use crate::frontend::checked::CheckedProgram;
use crate::frontend::error::CompilerError;
use crate::frontend::registry::TypeRegistry;
use crate::frontend::session::FrontendSession;
use std::path::PathBuf;

/// Register all type definitions from the program
pub fn register_types(program: &Program) -> TypeRegistry {
    let mut registry = TypeRegistry::new();
    for stmt in &program.stmts {
        let _ = registry.register(stmt);
    }
    registry
}

/// Load, type-check, lower to HIR, and return the full checked bundle.
pub fn load_checked(path: &PathBuf, explain: bool) -> Result<CheckedProgram, Vec<CompilerError>> {
    let session =
        FrontendSession::for_source_file(path).map_err(|e| vec![CompilerError::new(e)])?;
    session.compile_checked(path, explain)
}

/// Load, resolve imports, register types, and type-check a program.
pub fn load_program(
    path: &PathBuf,
    explain: bool,
) -> Result<(Program, TypeRegistry), Vec<CompilerError>> {
    load_checked(path, explain).map(|c| (c.program, c.registry))
}
