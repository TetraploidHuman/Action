// Submodule: runtime_decl/str_adv/join (R6)

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_join(&self) -> Result<(), String> {
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

        // ---- action_string_join({ptr, i64, i64}, {i64, ptr}) -> {i64, ptr} ----
        // Tree-based: uses action_list_get for element access.
        let jn_fn = self.module.add_function(
            "action_string_join",
            str_ty.fn_type(&[self.list_type.into(), str_ty.into()], false),
            None,
        );
        let jn_entry = self.context.append_basic_block(jn_fn, "entry");
        self.builder.position_at_end(jn_entry);
        let jn_list = jn_fn.get_first_param().unwrap().into_struct_value();
        let jn_delim = jn_fn.get_nth_param(1).unwrap().into_struct_value();
        let jn_llen = self
            .builder
            .build_extract_value(jn_list, 1, "llen")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_dlen = self
            .builder
            .build_extract_value(jn_delim, 0, "dlen")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_ddata_cc = self
            .builder
            .build_call(str_data_fn, &[jn_delim.into()], "dd")
            .map_err(llvm_err)?;
        let jn_ddata = jn_ddata_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let jn_get_fn = self.module.get_function("action_list_get").unwrap();
        let one = i64.const_int(1, false);
        let zero = i64.const_int(0, false);
        let _sixteen = i64.const_int(16, false);
        let _eight = i64.const_int(8, false);

        // Compute total size
        let jn_total = self.builder.build_alloca(i64, "total").map_err(llvm_err)?;
        self.builder.build_store(jn_total, zero).map_err(llvm_err)?;
        let jn_ji = self.builder.build_alloca(i64, "ji").map_err(llvm_err)?;
        self.builder.build_store(jn_ji, zero).map_err(llvm_err)?;

        let jn_hdr = self.context.append_basic_block(jn_fn, "hdr");
        let jn_body = self.context.append_basic_block(jn_fn, "body");
        let jn_after = self.context.append_basic_block(jn_fn, "after");
        let _ = self.builder.build_unconditional_branch(jn_hdr);

        // Sum all string lengths + delimiter lengths (via action_list_get)
        self.builder.position_at_end(jn_hdr);
        let jn_iv = self
            .builder
            .build_load(i64, jn_ji, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_more = self
            .builder
            .build_int_compare(IntPredicate::ULT, jn_iv, jn_llen, "more")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(jn_more, jn_body, jn_after);

        self.builder.position_at_end(jn_body);
        let jn_ge_cc = self
            .builder
            .build_call(jn_get_fn, &[jn_list.into(), jn_iv.into()], "ge")
            .map_err(llvm_err)?;
        let jn_ge = jn_ge_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let jn_sslen = self
            .builder
            .build_extract_value(jn_ge, 0, "sslen")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cur = self
            .builder
            .build_load(i64, jn_total, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        // Add delimiter length if not last element
        let jn_ivp1 = self
            .builder
            .build_int_add(jn_iv, one, "ivp1")
            .map_err(llvm_err)?;
        let jn_is_last = self
            .builder
            .build_int_compare(IntPredicate::EQ, jn_ivp1, jn_llen, "is_last")
            .map_err(llvm_err)?;
        let jn_with_delim = self
            .builder
            .build_int_add(jn_sslen, jn_dlen, "with_delim")
            .map_err(llvm_err)?;
        let jn_delta_sv = self
            .builder
            .build_select(jn_is_last, jn_sslen, jn_with_delim, "delta")
            .map_err(llvm_err)?;
        let jn_new_total = self
            .builder
            .build_int_add(jn_cur, jn_delta_sv.into_int_value(), "new_total")
            .map_err(llvm_err)?;
        self.builder
            .build_store(jn_total, jn_new_total)
            .map_err(llvm_err)?;
        let jn_niv = self
            .builder
            .build_int_add(jn_iv, one, "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(jn_ji, jn_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(jn_hdr);

        // Allocate and copy
        self.builder.position_at_end(jn_after);
        let jn_final_total = self
            .builder
            .build_load(i64, jn_total, "final_total")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_jalc = self
            .builder
            .build_int_add(jn_final_total, one, "jalc")
            .map_err(llvm_err)?;
        let jn_buf = self
            .builder
            .build_call(malloc_rc_fn, &[jn_jalc.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for this newly allocated buffer (malloc_rc starts at 0)
        let jn_buf_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(jn_buf, i64, "jn_buf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "jn_buf_rc_addr",
            )
            .map_err(llvm_err)?;
        let jn_buf_rc_p = self
            .builder
            .build_int_to_ptr(jn_buf_rc_addr, ptr, "jn_buf_rc_p")
            .map_err(llvm_err)?;
        self.builder
            .build_store(jn_buf_rc_p, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Reset i, reset write cursor
        let jn_wpos = self.builder.build_alloca(i64, "wpos").map_err(llvm_err)?;
        self.builder.build_store(jn_ji, zero).map_err(llvm_err)?;
        self.builder.build_store(jn_wpos, zero).map_err(llvm_err)?;

        let jn_chdr = self.context.append_basic_block(jn_fn, "chdr");
        let jn_cbody = self.context.append_basic_block(jn_fn, "cbody");
        let jn_cdone = self.context.append_basic_block(jn_fn, "cdone");
        let _ = self.builder.build_unconditional_branch(jn_chdr);

        self.builder.position_at_end(jn_chdr);
        let jn_civ = self
            .builder
            .build_load(i64, jn_ji, "civ")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cmore = self
            .builder
            .build_int_compare(IntPredicate::ULT, jn_civ, jn_llen, "cmore")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(jn_cmore, jn_cbody, jn_cdone);

        self.builder.position_at_end(jn_cbody);
        let jn_cge_cc = self
            .builder
            .build_call(jn_get_fn, &[jn_list.into(), jn_civ.into()], "cge")
            .map_err(llvm_err)?;
        let jn_cge = jn_cge_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let jn_csslen = self
            .builder
            .build_extract_value(jn_cge, 0, "csslen")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cp_cc = self
            .builder
            .build_call(str_data_fn, &[jn_cge.into()], "cp")
            .map_err(llvm_err)?;
        let jn_cp = jn_cp_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Copy string data to output at wpos
        let jn_cwp = self
            .builder
            .build_load(i64, jn_wpos, "cwp")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cdst = unsafe {
            self.builder
                .build_gep(i8, jn_buf, &[jn_cwp], "cdst")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[jn_cdst.into(), jn_cp.into(), jn_csslen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let jn_nwp = self
            .builder
            .build_int_add(jn_cwp, jn_csslen, "nwp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(jn_wpos, jn_nwp)
            .map_err(llvm_err)?;
        // Copy delimiter if not last
        let jn_civp1 = self
            .builder
            .build_int_add(jn_civ, one, "civp1")
            .map_err(llvm_err)?;
        let jn_cis_last = self
            .builder
            .build_int_compare(IntPredicate::EQ, jn_civp1, jn_llen, "cis_last")
            .map_err(llvm_err)?;
        let jn_cdel_bb = self.context.append_basic_block(jn_fn, "cdel");
        let jn_cnext_bb = self.context.append_basic_block(jn_fn, "cnext");
        let _ = self
            .builder
            .build_conditional_branch(jn_cis_last, jn_cnext_bb, jn_cdel_bb);

        self.builder.position_at_end(jn_cdel_bb);
        let jn_cwp2 = self
            .builder
            .build_load(i64, jn_wpos, "cwp2")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cdst2 = unsafe {
            self.builder
                .build_gep(i8, jn_buf, &[jn_cwp2], "cdst2")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[jn_cdst2.into(), jn_ddata.into(), jn_dlen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let jn_nwp2 = self
            .builder
            .build_int_add(jn_cwp2, jn_dlen, "nwp2")
            .map_err(llvm_err)?;
        self.builder
            .build_store(jn_wpos, jn_nwp2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(jn_cnext_bb);

        self.builder.position_at_end(jn_cnext_bb);
        let jn_cniv = self
            .builder
            .build_int_add(jn_civ, one, "cniv")
            .map_err(llvm_err)?;
        self.builder.build_store(jn_ji, jn_cniv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(jn_chdr);

        // Done: null-terminate and return
        self.builder.position_at_end(jn_cdone);
        let jn_fwp = self
            .builder
            .build_load(i64, jn_wpos, "fwp")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_nullp = unsafe {
            self.builder
                .build_gep(i8, jn_buf, &[jn_fwp], "nullp")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(jn_nullp, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let jn_und = str_ty.get_undef();
        let jn_r1 = self
            .builder
            .build_insert_value(jn_und, jn_fwp, 0, "r1")
            .map_err(llvm_err)?;
        let jn_r2 = self
            .builder
            .build_insert_value(jn_r1, jn_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&jn_r2));

        Ok(())
    }
}
