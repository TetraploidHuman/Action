//! Type-checked program bundle: AST + registry + HIR.

use crate::ast::Program;
use crate::hir::{lower_program, HirModule};
use crate::registry::TypeRegistry;
use crate::typecheck::TypeChecker;

/// Result of frontend compilation after successful type-checking.
#[derive(Clone)]
pub struct CheckedProgram {
    pub program: Program,
    pub registry: TypeRegistry,
    pub hir: HirModule,
}

impl CheckedProgram {
    /// Build HIR from an already type-checked program.
    pub fn new(program: Program, registry: TypeRegistry, checker: &TypeChecker) -> Self {
        let hir = lower_program(&program, checker);
        Self {
            program,
            registry,
            hir,
        }
    }

    /// Verify HIR round-trips to the same AST (debug / tests).
    pub fn verify_hir_round_trip(&self) -> bool {
        self.hir.to_program() == self.program
    }

    /// Serialize HIR as pretty JSON (bootstrap / `--emit hir`).
    pub fn hir_json_pretty(&self) -> Result<String, serde_json::Error> {
        self.hir.to_json_pretty()
    }
}
