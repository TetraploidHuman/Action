//! List core runtime fragments; `body.inc.rs` is assembled from `*.inc.rs` by `scripts/concat_list_body.py` at build time.

use crate::{llvm_err, CodeGen};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(in crate::runtime_decl) fn define_list_core(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let _str_ty = self.string_type;
        let b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let zero = self.i64_ty().const_int(0, false);

        let _malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let fmt_lb_ptr = self.make_global_str(".fmt_lb", b"[\0")?;
        let fmt_rb_ptr = self.make_global_str(".fmt_rb", b"]\0")?;
        let fmt_sep_ptr = self.make_global_str(".fmt_sep", b", \0")?;
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let printf_fn = self.module.get_function("printf").unwrap();
        let fmt_int_ptr = self.make_global_str(".fmt_int", b"%lld\0")?;

        include!("body.inc.rs");
        Ok(())
    }
}
