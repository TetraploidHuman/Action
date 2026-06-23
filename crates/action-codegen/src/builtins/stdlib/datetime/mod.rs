// Submodule: builtins_stdlib_datetime (R6)

mod accessors;
mod construct;
mod format_parse;
mod random;
mod today_now;
mod weekday_utc;

use crate::call_arg::CallArg;
use crate::{CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_stdlib_datetime(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        if let Some(v) = self.datetime_dispatch_format_parse(name, args)? {
            return Ok(v);
        }
        if let Some(v) = self.datetime_dispatch_construct(name, args)? {
            return Ok(v);
        }
        if let Some(v) = self.datetime_dispatch_random(name, args)? {
            return Ok(v);
        }
        if let Some(v) = self.datetime_dispatch_accessors(name, args)? {
            return Ok(v);
        }
        if let Some(v) = self.datetime_dispatch_weekday_utc(name, args)? {
            return Ok(v);
        }
        Err(format!("Unknown datetime builtin: {}", name))
    }
}
