// Submodule: compile
//
// The main compilation entry point: handles LLVM module setup, function
// declaration passes, and top-level compilation orchestration.

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    /// Compile from a type-checked bundle (HIR path).
    pub fn compile_checked(
        &mut self,
        checked: &action_frontend::checked::CheckedProgram,
    ) -> Result<(), String> {
        debug_assert!(
            checked.verify_hir_round_trip(),
            "HIR must round-trip to AST in debug builds"
        );
        self.compile_hir(&checked.hir)
    }
}
