// Submodule: builtins_stdlib_collection (R6)

mod aggregate;
mod list_basic;
mod list_gen;
mod list_misc;
mod list_transform;
mod map_set;

use crate::call_arg::CallArg;
use crate::{CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_stdlib_collection(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        if let Some(v) = self.collection_dispatch_list_basic(name, args)? {
            return Ok(v);
        }
        if let Some(v) = self.collection_dispatch_list_gen(name, args)? {
            return Ok(v);
        }
        if let Some(v) = self.collection_dispatch_list_misc(name, args)? {
            return Ok(v);
        }
        if let Some(v) = self.collection_dispatch_list_transform(name, args)? {
            return Ok(v);
        }
        if let Some(v) = self.collection_dispatch_map_set(name, args)? {
            return Ok(v);
        }
        if let Some(v) = self.collection_dispatch_aggregate(name, args)? {
            return Ok(v);
        }
        Err(format!("Unknown collection builtin: {}", name))
    }
}
