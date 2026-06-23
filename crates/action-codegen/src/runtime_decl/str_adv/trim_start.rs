// Submodule: runtime_decl/str_adv/trim_start (R6)

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_trim_start(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let _ptr = self.ptr_ty();
        let _b1 = self.bool_ty();
        let _i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let _malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let _memcmp_fn = self.module.get_function("memcmp").unwrap();
        let _memcpy_fn = self.module.get_function("memcpy").unwrap();
        let str_data_fn = self.module.get_function("action_string_data").unwrap();

        // ---- action_string_trim_start({i64, ptr}) -> {i64, ptr} ----
        let ts_fn = self.module.add_function(
            "action_string_trim_start",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let ts_entry = self.context.append_basic_block(ts_fn, "entry");
        self.builder.position_at_end(ts_entry);
        let ts_str = ts_fn.get_first_param().unwrap().into_struct_value();
        let ts_len = self
            .builder
            .build_extract_value(ts_str, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let ts_ptr_cc = self
            .builder
            .build_call(str_data_fn, &[ts_str.into()], "dp")
            .map_err(llvm_err)?;
        let ts_ptr = ts_ptr_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let ts_loop_bb = self.context.append_basic_block(ts_fn, "ts_loop");
        let ts_done_bb = self.context.append_basic_block(ts_fn, "ts_done");
        let _ = self.builder.build_unconditional_branch(ts_loop_bb);
        self.builder.position_at_end(ts_loop_bb);
        let ts_i = self.builder.build_phi(i64, "ts_i").map_err(llvm_err)?;
        let ts_cp = unsafe {
            self.builder
                .build_gep(i8, ts_ptr, &[ts_i.as_basic_value().into_int_value()], "cp")
                .map_err(llvm_err)
        }?;
        let ts_c = self
            .builder
            .build_load(i8, ts_cp, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let ts_space = i8.const_int(0x20, false);
        let ts_tab = i8.const_int(0x09, false);
        let ts_nl = i8.const_int(0x0a, false);
        let ts_cr = i8.const_int(0x0d, false);
        let ts_is_space = self
            .builder
            .build_int_compare(IntPredicate::EQ, ts_c, ts_space, "is_space")
            .map_err(llvm_err)?;
        let ts_is_tab = self
            .builder
            .build_int_compare(IntPredicate::EQ, ts_c, ts_tab, "is_tab")
            .map_err(llvm_err)?;
        let ts_is_nl = self
            .builder
            .build_int_compare(IntPredicate::EQ, ts_c, ts_nl, "is_nl")
            .map_err(llvm_err)?;
        let ts_is_cr = self
            .builder
            .build_int_compare(IntPredicate::EQ, ts_c, ts_cr, "is_cr")
            .map_err(llvm_err)?;
        let ts_is_ws1 = self
            .builder
            .build_or(ts_is_space, ts_is_tab, "ws1")
            .map_err(llvm_err)?;
        let ts_is_ws2 = self
            .builder
            .build_or(ts_is_nl, ts_is_cr, "ws2")
            .map_err(llvm_err)?;
        let ts_is_ws = self
            .builder
            .build_or(ts_is_ws1, ts_is_ws2, "is_ws")
            .map_err(llvm_err)?;
        let ts_i_next = self
            .builder
            .build_int_add(
                ts_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ts_inext",
            )
            .map_err(llvm_err)?;
        let ts_at_end = self
            .builder
            .build_int_compare(IntPredicate::SGE, ts_i_next, ts_len, "at_end")
            .map_err(llvm_err)?;
        let ts_stop = self
            .builder
            .build_or(
                ts_at_end,
                self.builder
                    .build_not(ts_is_ws, "not_ws")
                    .map_err(llvm_err)?,
                "stop",
            )
            .map_err(llvm_err)?;
        let ts_loop_block = self.builder.get_insert_block().unwrap();
        ts_i.add_incoming(&[
            (&i64.const_int(0, false), ts_entry),
            (&ts_i_next, ts_loop_block),
        ]);
        let _ = self
            .builder
            .build_conditional_branch(ts_stop, ts_done_bb, ts_loop_bb);
        self.builder.position_at_end(ts_done_bb);
        let ts_start = self.builder.build_phi(i64, "ts_start").map_err(llvm_err)?;
        ts_start.add_incoming(&[(&ts_i.as_basic_value().into_int_value(), ts_loop_block)]);
        // Use start idx as the new start; if start == len, return empty string
        let ts_new_len = self
            .builder
            .build_int_sub(
                ts_len,
                ts_start.as_basic_value().into_int_value(),
                "new_len",
            )
            .map_err(llvm_err)?;
        let ts_nptr = unsafe {
            self.builder
                .build_gep(
                    i8,
                    ts_ptr,
                    &[ts_start.as_basic_value().into_int_value()],
                    "nptr",
                )
                .map_err(llvm_err)
        }?;
        let ts_undef = str_ty.get_undef();
        let ts_r1 = self
            .builder
            .build_insert_value(ts_undef, ts_new_len, 0, "r1")
            .map_err(llvm_err)?;
        let ts_r2 = self
            .builder
            .build_insert_value(ts_r1, ts_nptr, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ts_r2));

        Ok(())
    }
}
