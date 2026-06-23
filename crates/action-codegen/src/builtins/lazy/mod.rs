// Submodule: builtins_lazy


use crate::call_arg::CallArg;
use crate::{CodeGen, TypedValue};

mod head_zip;
mod map_filter;
mod take_drop;
mod take_while;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_lazy_take_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let n_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_take_values(n_val, lazy_val)
    }

    pub(crate) fn builtin_lazy_drop_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let n_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_drop_values(n_val, lazy_val)
    }

    pub(crate) fn builtin_lazy_map_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_map_values(fn_val, lazy_val)
    }

    pub(crate) fn builtin_lazy_filter_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_filter_values(fn_val, lazy_val)
    }

    pub(crate) fn builtin_lazy_take_while_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self.compile_call_arg(a)?;
        let lazy_val = self.compile_call_arg(b)?;
        self.builtin_lazy_take_while_values(fn_val, lazy_val)
    }

    pub(crate) fn builtin_lazy_head_call_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let lazy_val = self.compile_call_arg(arg)?;
        self.builtin_lazy_head_value(lazy_val)
    }

    pub(crate) fn builtin_lazy_zip_call_args(
        &mut self,
        a: CallArg<'_>,
        b: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let v1 = self.compile_call_arg(a)?;
        let v2 = self.compile_call_arg(b)?;
        self.builtin_lazy_zip_values(v1, v2)
    }
}
