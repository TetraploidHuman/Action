//! List tree runtime fragments (R3-5 include! split).

use crate::{llvm_err, CodeGen};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(in crate::runtime_decl) fn define_list_tree(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let _void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let zero = self.i64_ty().const_int(0, false);
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let memcmp_fn = self.module.get_function("memcmp").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        let _list_create_fn = self.module.get_function("action_list_create").unwrap();
        let _list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();

        include!("body.inc.rs");
        Ok(())
    }
}
