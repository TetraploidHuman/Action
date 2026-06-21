// Runtime LLVM attributes for AOT/JIT codegen.

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    /// Attach LLVM attributes to hot runtime helpers (helps IPO / inlining in AOT -O2).
    pub(super) fn apply_runtime_fn_attrs(&self) {
        use inkwell::attributes::{Attribute, AttributeLoc};

        let ctx = self.context;
        let nounwind_id = Attribute::get_named_enum_kind_id("nounwind");
        let nounwind = ctx.create_enum_attribute(nounwind_id, 0);

        let nounwind_fns = [
            "action_list_get",
            "action_list_get_cached",
            "action_list_len",
            "action_list_is_empty",
            "action_list_find",
            "action_list_push",
            "action_list_insert",
            "action_list_remove",
            "action_list_concat",
            "action_list_reverse",
            "action_list_reverse_walk_rec",
            "action_list_map_walk",
            "action_list_filter_walk",
            "action_list_fold_walk",
            "action_map_get",
            "action_map_len",
            "action_map_contains_key",
            "action_ht_insert",
            "action_ht_from_list",
            "action_rc_inc",
            "action_rc_dec",
        ];
        for name in nounwind_fns {
            if let Some(f) = self.module.get_function(name) {
                f.add_attribute(AttributeLoc::Function, nounwind);
            }
        }
    }
}
