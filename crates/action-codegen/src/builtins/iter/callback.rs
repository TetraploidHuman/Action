//! Iterator builtins: map, filter, fold, find (R4-1).

use crate::call_arg::CallArg;
use crate::{CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_callback_map(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "mapFilter" => self.builtin_map_filter(args, trailing),
            "mapMapValues" => self.builtin_map_map_values(args, trailing),
            "mapFold" => self.builtin_map_fold(args, trailing),
            _ => Err(format!("Unknown callback map builtin: {}", name)),
        }
    }
}
