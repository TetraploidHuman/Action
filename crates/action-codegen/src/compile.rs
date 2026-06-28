// Submodule: compile
//
// The main compilation entry point: handles LLVM module setup, function
// declaration passes, and top-level compilation orchestration.

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    /// Compile from a type-checked bundle (HIR path).
    pub fn compile_from_checked(
        &mut self,
        checked: &action_frontend::checked::CheckedProgram,
    ) -> Result<(), String> {
        debug_assert!(
            checked.verify_hir_round_trip(),
            "HIR must round-trip to AST in debug builds"
        );
        self.fallibility = checked.fallibility.clone();
        self.compile_hir(&checked.hir)
    }

    /// Compile from a type-checked bundle (HIR path).
    #[deprecated(since = "0.5.5", note = "use `compile_from_checked`")]
    pub fn compile_checked(
        &mut self,
        checked: &action_frontend::checked::CheckedProgram,
    ) -> Result<(), String> {
        self.compile_from_checked(checked)
    }
}
