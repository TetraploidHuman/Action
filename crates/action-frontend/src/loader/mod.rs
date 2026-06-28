mod resolve;
mod stdlib;

pub use resolve::{resolve_imports, transform_module_access};
pub use stdlib::{builtin_types, load_path_dependencies, load_stdlib, parse_ac_file};

use crate::ast::{Program, Stmt};
use crate::checked::CheckedProgram;
use crate::error::CompilerError;
use crate::registry::TypeRegistry;
use crate::session::FrontendSession;
use std::path::PathBuf;

fn registration_error(message: String, stmt: &Stmt) -> CompilerError {
    CompilerError::new(message).with_span(stmt.span())
}

/// Register all type definitions from the program; propagates duplicate/invalid defs.
pub fn build_type_registry(program: &Program) -> Result<TypeRegistry, Vec<CompilerError>> {
    let mut registry = TypeRegistry::new();
    let mut errors = Vec::new();
    for stmt in &program.stmts {
        if let Err(e) = registry.register(stmt) {
            errors.push(registration_error(e, stmt));
        }
    }
    if errors.is_empty() {
        Ok(registry)
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Stmt, Type};
    use action_span::Span;

    #[test]
    fn build_type_registry_rejects_duplicate_type_alias() {
        let span = Span::default();
        let program = Program {
            stmts: vec![
                Stmt::TypeAlias {
                    name: "Foo".into(),
                    type_params: vec![],
                    definition: Type::Named("Int".into()),
                    span,
                },
                Stmt::TypeAlias {
                    name: "Foo".into(),
                    type_params: vec![],
                    definition: Type::Named("String".into()),
                    span,
                },
            ],
        };
        match build_type_registry(&program) {
            Err(err) => {
                assert_eq!(err.len(), 1);
                assert!(err[0].message.contains("duplicate type definition 'Foo'"));
            }
            Ok(_) => panic!("expected duplicate type error"),
        }
    }
}

/// Register all type definitions from the program.
#[deprecated(
    since = "0.5.5",
    note = "use `build_type_registry`; errors are no longer swallowed"
)]
pub fn register_types(program: &Program) -> TypeRegistry {
    build_type_registry(program).unwrap_or_else(|errors| {
        panic!(
            "register_types failed: {}",
            errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join("; ")
        )
    })
}

/// Load, type-check, lower to HIR, and return the full checked bundle.
pub fn check_file(path: &PathBuf, explain: bool) -> Result<CheckedProgram, Vec<CompilerError>> {
    let session =
        FrontendSession::for_source_file(path).map_err(|e| vec![CompilerError::new(e)])?;
    session.check_file(path, explain)
}

/// Load, type-check, lower to HIR, and return the full checked bundle.
#[deprecated(since = "0.5.5", note = "use `check_file`")]
pub fn load_checked(path: &PathBuf, explain: bool) -> Result<CheckedProgram, Vec<CompilerError>> {
    check_file(path, explain)
}

/// Load, resolve imports, register types, and type-check a program (discards HIR).
#[deprecated(
    since = "0.5.5",
    note = "use `check_file` and take `.program` / `.registry`; HIR is discarded here"
)]
pub fn load_program(
    path: &PathBuf,
    explain: bool,
) -> Result<(Program, TypeRegistry), Vec<CompilerError>> {
    check_file(path, explain).map(|c| (c.program, c.registry))
}
