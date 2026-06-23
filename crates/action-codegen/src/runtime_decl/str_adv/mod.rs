// Submodule: runtime_decl/str_adv (R6)

mod contains;
mod join;
mod repeat;
mod replace;
mod split;
mod trim_end;
mod trim_start;

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_adv(&self) -> Result<(), String> {
        self.define_str_split()?;
        self.define_str_join()?;
        self.define_str_replace()?;
        self.define_str_contains()?;
        self.define_str_repeat()?;
        self.define_str_trim_start()?;
        self.define_str_trim_end()?;
        Ok(())
    }
}
