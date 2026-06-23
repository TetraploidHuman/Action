// Submodule: runtime_decl/str_adv/repeat (R6)

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_repeat(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let ptr = self.ptr_ty();
        let _b1 = self.bool_ty();
        let _i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let _memcmp_fn = self.module.get_function("memcmp").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        let str_data_fn = self.module.get_function("action_string_data").unwrap();

        // ---- action_string_repeat({i64, ptr}, i64) -> {i64, ptr} ----
        let sr_fn = self.module.add_function(
            "action_string_repeat",
            str_ty.fn_type(&[str_ty.into(), i64.into()], false),
            None,
        );
        let sr_entry = self.context.append_basic_block(sr_fn, "entry");
        self.builder.position_at_end(sr_entry);
        let sr_str = sr_fn.get_first_param().unwrap().into_struct_value();
        let sr_n = sr_fn.get_nth_param(1).unwrap().into_int_value();
        let sr_slen = self
            .builder
            .build_extract_value(sr_str, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sr_sptr_cc = self
            .builder
            .build_call(str_data_fn, &[sr_str.into()], "sp")
            .map_err(llvm_err)?;
        let sr_sptr = sr_sptr_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sr_total = self
            .builder
            .build_int_mul(sr_slen, sr_n, "total")
            .map_err(llvm_err)?;
        let sr_buf = self
            .builder
            .build_call(malloc_rc_fn, &[sr_total.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("malloc")?
            .into_pointer_value();
        // Set RC=1 for newly allocated buffer
        let sr_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(sr_buf, i64, "sr_buf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "sr_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(sr_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        // Loop: copy s into buffer n times
        let sr_loop_bb = self.context.append_basic_block(sr_fn, "sr_loop");
        let sr_done_bb = self.context.append_basic_block(sr_fn, "sr_done");
        let _ = self.builder.build_unconditional_branch(sr_loop_bb);
        self.builder.position_at_end(sr_loop_bb);
        let sr_i = self.builder.build_phi(i64, "sr_i").map_err(llvm_err)?;
        let sr_offset = self
            .builder
            .build_int_mul(sr_i.as_basic_value().into_int_value(), sr_slen, "offset")
            .map_err(llvm_err)?;
        let sr_dst = unsafe {
            self.builder
                .build_gep(i8, sr_buf, &[sr_offset], "dst")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sr_dst.into(), sr_sptr.into(), sr_slen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sr_i_next = self
            .builder
            .build_int_add(
                sr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "sri_next",
            )
            .map_err(llvm_err)?;
        let sr_done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, sr_i_next, sr_n, "srdone")
            .map_err(llvm_err)?;
        let sr_loop_block = self.builder.get_insert_block().unwrap();
        sr_i.add_incoming(&[
            (&i64.const_int(0, false), sr_entry),
            (&sr_i_next, sr_loop_block),
        ]);
        let _ = self
            .builder
            .build_conditional_branch(sr_done_cond, sr_done_bb, sr_loop_bb);
        self.builder.position_at_end(sr_done_bb);
        let sr_undef = str_ty.get_undef();
        let sr_r1 = self
            .builder
            .build_insert_value(sr_undef, sr_total, 0, "r1")
            .map_err(llvm_err)?;
        let sr_r2 = self
            .builder
            .build_insert_value(sr_r1, sr_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sr_r2));

        Ok(())
    }
}
