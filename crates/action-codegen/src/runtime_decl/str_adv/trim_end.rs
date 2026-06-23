// Submodule: runtime_decl/str_adv/trim_end (R6)

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_trim_end(&self) -> Result<(), String> {
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

        // ---- action_string_trim_end({i64, ptr}) -> {i64, ptr} ----
        let te_fn = self.module.add_function(
            "action_string_trim_end",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let te_entry = self.context.append_basic_block(te_fn, "entry");
        self.builder.position_at_end(te_entry);
        let te_str = te_fn.get_first_param().unwrap().into_struct_value();
        let te_len = self
            .builder
            .build_extract_value(te_str, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let te_ptr_cc = self
            .builder
            .build_call(str_data_fn, &[te_str.into()], "dp")
            .map_err(llvm_err)?;
        let te_ptr = te_ptr_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Start from len-1 and go backwards
        let te_start = self
            .builder
            .build_int_sub(te_len, i64.const_int(1, false), "last")
            .map_err(llvm_err)?;
        let te_loop_bb = self.context.append_basic_block(te_fn, "te_loop");
        let te_done_bb = self.context.append_basic_block(te_fn, "te_done");
        let _ = self.builder.build_unconditional_branch(te_loop_bb);
        self.builder.position_at_end(te_loop_bb);
        let te_i = self.builder.build_phi(i64, "te_i").map_err(llvm_err)?;
        let te_cp = unsafe {
            self.builder
                .build_gep(i8, te_ptr, &[te_i.as_basic_value().into_int_value()], "cp")
                .map_err(llvm_err)
        }?;
        let te_c = self
            .builder
            .build_load(i8, te_cp, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let te_is_space = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                te_c,
                i8.const_int(0x20, false),
                "is_space",
            )
            .map_err(llvm_err)?;
        let te_is_tab = self
            .builder
            .build_int_compare(IntPredicate::EQ, te_c, i8.const_int(0x09, false), "is_tab")
            .map_err(llvm_err)?;
        let te_is_nl = self
            .builder
            .build_int_compare(IntPredicate::EQ, te_c, i8.const_int(0x0a, false), "is_nl")
            .map_err(llvm_err)?;
        let te_is_cr = self
            .builder
            .build_int_compare(IntPredicate::EQ, te_c, i8.const_int(0x0d, false), "is_cr")
            .map_err(llvm_err)?;
        let te_is_ws1 = self
            .builder
            .build_or(te_is_space, te_is_tab, "ws1")
            .map_err(llvm_err)?;
        let te_is_ws2 = self
            .builder
            .build_or(te_is_nl, te_is_cr, "ws2")
            .map_err(llvm_err)?;
        let te_is_ws = self
            .builder
            .build_or(te_is_ws1, te_is_ws2, "is_ws")
            .map_err(llvm_err)?;
        let te_i_next = self
            .builder
            .build_int_sub(
                te_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "te_inext",
            )
            .map_err(llvm_err)?;
        let te_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, te_i_next, i64.const_int(0, false), "neg")
            .map_err(llvm_err)?;
        let te_stop = self
            .builder
            .build_or(
                te_neg,
                self.builder
                    .build_not(te_is_ws, "not_ws")
                    .map_err(llvm_err)?,
                "stop",
            )
            .map_err(llvm_err)?;
        let te_loop_block = self.builder.get_insert_block().unwrap();
        te_i.add_incoming(&[(&te_start, te_entry), (&te_i_next, te_loop_block)]);
        let _ = self
            .builder
            .build_conditional_branch(te_stop, te_done_bb, te_loop_bb);
        self.builder.position_at_end(te_done_bb);
        // te_i is the index of the character we just checked.
        // If it was not whitespace, new_len = te_i + 1.
        // If te_neg was true (all whitespace), te_i = 0 but we need new_len = 0.
        // Check te_neg by checking if te_i_next < 0
        let _te_neg_check = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                te_i.as_basic_value().into_int_value(),
                i64.const_int(0, false),
                "neg_check",
            )
            .map_err(llvm_err)?;
        // Re-check: was the character at te_i whitespace?
        // Easier: just re-load and check
        let te_final_cp = unsafe {
            self.builder
                .build_gep(i8, te_ptr, &[te_i.as_basic_value().into_int_value()], "fcp")
                .map_err(llvm_err)
        }?;
        let te_final_c = self
            .builder
            .build_load(i8, te_final_cp, "fc")
            .map_err(llvm_err)?
            .into_int_value();
        let te_final_ws1 = self
            .builder
            .build_or(
                self.builder
                    .build_int_compare(IntPredicate::EQ, te_final_c, i8.const_int(0x20, false), "")
                    .map_err(llvm_err)?,
                self.builder
                    .build_int_compare(IntPredicate::EQ, te_final_c, i8.const_int(0x09, false), "")
                    .map_err(llvm_err)?,
                "",
            )
            .map_err(llvm_err)?;
        let te_final_ws2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_compare(IntPredicate::EQ, te_final_c, i8.const_int(0x0a, false), "")
                    .map_err(llvm_err)?,
                self.builder
                    .build_int_compare(IntPredicate::EQ, te_final_c, i8.const_int(0x0d, false), "")
                    .map_err(llvm_err)?,
                "",
            )
            .map_err(llvm_err)?;
        let te_final_ws = self
            .builder
            .build_or(te_final_ws1, te_final_ws2, "fws")
            .map_err(llvm_err)?;
        let te_zero_len = i64.const_int(0, false);
        let te_plus1 = self
            .builder
            .build_int_add(
                te_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "plus1",
            )
            .map_err(llvm_err)?;
        let te_new_len = self
            .builder
            .build_select(te_final_ws, te_zero_len, te_plus1, "new_len")
            .map_err(llvm_err)?
            .into_int_value();
        let te_undef = str_ty.get_undef();
        let te_r1 = self
            .builder
            .build_insert_value(te_undef, te_new_len, 0, "r1")
            .map_err(llvm_err)?;
        let te_r2 = self
            .builder
            .build_insert_value(te_r1, te_ptr, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&te_r2));

        Ok(())
    }
}
