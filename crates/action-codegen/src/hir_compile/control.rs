use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_hir_continue(&mut self) -> Result<TypedValue<'ctx>, String> {
        if let Some(target) = self.loop_control.continue_target {
            self.builder
                .build_unconditional_branch(target)
                .map_err(llvm_err)?;
            Ok(TypedValue::Unit)
        } else {
            Err("continue outside loop".to_string())
        }
    }

    pub(crate) fn compile_hir_break(&mut self) -> Result<TypedValue<'ctx>, String> {
        if let Some(target) = self.loop_control.break_target {
            self.builder
                .build_unconditional_branch(target)
                .map_err(llvm_err)?;
            Ok(TypedValue::Unit)
        } else {
            Err("break outside loop".to_string())
        }
    }
}
