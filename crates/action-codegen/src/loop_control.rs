//! Loop control state for for/while/break/continue codegen.

use inkwell::basic_block::BasicBlock;
use inkwell::values::PointerValue;

pub(crate) struct LoopControl<'ctx> {
    pub continue_target: Option<BasicBlock<'ctx>>,
    pub break_target: Option<BasicBlock<'ctx>>,
    pub list_loop_get_cache: Option<PointerValue<'ctx>>,
}

impl<'ctx> Default for LoopControl<'ctx> {
    fn default() -> Self {
        Self {
            continue_target: None,
            break_target: None,
            list_loop_get_cache: None,
        }
    }
}
