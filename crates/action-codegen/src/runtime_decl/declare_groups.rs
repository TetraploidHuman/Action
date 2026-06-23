// Submodule: runtime_decl/declare_groups (R3-3)
//
// Dispatch table for runtime IR generation groups.

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_runtime_groups(&self) -> Result<(), String> {
        self.define_str_core()?;
        self.define_print()?;
        self.define_str_basic()?;
        self.define_list_core()?;
        self.define_list_map_filter_map()?;
        self.define_list_filter_map_fold()?;
        self.define_list_filter_fold()?;
        self.define_list_map_fold()?;
        self.define_list_insert_split_child()?;
        self.define_lazy_list()?;
        self.define_list_insert_rec()?;
        self.define_list_insert_split_intl()?;
        self.define_list_remove_rec()?;
        self.define_list_iter()?;
        self.define_list_xform()?;
        self.define_list_index_of_walk()?;
        self.define_str_util()?;
        self.define_hash_table()?;
        self.define_map()?;
        self.define_str_extra()?;
        self.define_file_parse()?;
        self.define_rand()?;
        self.define_str_adv()?;
        self.define_list_extra()?;
        self.define_list_tree()?;
        self.define_math_ms()?;
        self.define_misc()?;
        self.define_list_rc_assign()?;
        Ok(())
    }
}
