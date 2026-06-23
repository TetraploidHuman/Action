// Submodule: runtime_decl/str_adv/contains (R6)

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_contains(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let _ptr = self.ptr_ty();
        let b1 = self.bool_ty();
        let _i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let _malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let _memcmp_fn = self.module.get_function("memcmp").unwrap();
        let _memcpy_fn = self.module.get_function("memcpy").unwrap();
        let str_data_fn = self.module.get_function("action_string_data").unwrap();

        // ---- action_string_contains({i64, ptr}, {i64, ptr}) -> i1 ----
        let sc_fn = self.module.add_function(
            "action_string_contains",
            b1.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let sc_entry = self.context.append_basic_block(sc_fn, "entry");
        self.builder.position_at_end(sc_entry);
        let sc_haystack = sc_fn.get_first_param().unwrap().into_struct_value();
        let sc_needle = sc_fn.get_nth_param(1).unwrap().into_struct_value();
        let sc_hlen = self
            .builder
            .build_extract_value(sc_haystack, 0, "hlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sc_hptr_cc = self
            .builder
            .build_call(str_data_fn, &[sc_haystack.into()], "hp")
            .map_err(llvm_err)?;
        let sc_hptr = sc_hptr_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sc_nlen = self
            .builder
            .build_extract_value(sc_needle, 0, "nlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sc_nptr_cc = self
            .builder
            .build_call(str_data_fn, &[sc_needle.into()], "np")
            .map_err(llvm_err)?;
        let sc_nptr = sc_nptr_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // If needle is empty, return true
        let sc_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, sc_nlen, i64.const_int(0, false), "nempty")
            .map_err(llvm_err)?;
        let sc_len_ok = self
            .builder
            .build_int_compare(IntPredicate::SLE, sc_nlen, sc_hlen, "lenok")
            .map_err(llvm_err)?;
        let _sc_can_search = self
            .builder
            .build_and(
                sc_len_ok,
                self.builder
                    .build_not(sc_empty, "not_empty")
                    .map_err(llvm_err)?,
                "can_search",
            )
            .map_err(llvm_err)?;
        // Brute-force search
        let sc_max = self
            .builder
            .build_int_sub(sc_hlen, sc_nlen, "max")
            .map_err(llvm_err)?;
        let sc_loop_bb = self.context.append_basic_block(sc_fn, "sc_loop");
        let sc_found_bb = self.context.append_basic_block(sc_fn, "sc_found");
        let sc_notfound_bb = self.context.append_basic_block(sc_fn, "sc_notfound");
        let _ = self.builder.build_unconditional_branch(sc_loop_bb);
        self.builder.position_at_end(sc_loop_bb);
        let sc_i = self.builder.build_phi(i64, "sc_i").map_err(llvm_err)?;
        // Compare character by character
        let sc_j_loop_bb = self.context.append_basic_block(sc_fn, "sc_jloop");
        let sc_match_bb = self.context.append_basic_block(sc_fn, "sc_match");
        let sc_mismatch_bb = self.context.append_basic_block(sc_fn, "sc_mismatch");
        let _ = self.builder.build_unconditional_branch(sc_j_loop_bb);
        self.builder.position_at_end(sc_j_loop_bb);
        let sc_j = self.builder.build_phi(i64, "sc_j").map_err(llvm_err)?;
        let sc_hidx = self
            .builder
            .build_int_add(
                sc_i.as_basic_value().into_int_value(),
                sc_j.as_basic_value().into_int_value(),
                "hidx",
            )
            .map_err(llvm_err)?;
        let sc_hp = unsafe {
            self.builder
                .build_gep(i8, sc_hptr, &[sc_hidx], "hp")
                .map_err(llvm_err)
        }?;
        let sc_hc = self
            .builder
            .build_load(i8, sc_hp, "hc")
            .map_err(llvm_err)?
            .into_int_value();
        let sc_np = unsafe {
            self.builder
                .build_gep(i8, sc_nptr, &[sc_j.as_basic_value().into_int_value()], "np")
                .map_err(llvm_err)
        }?;
        let sc_nc = self
            .builder
            .build_load(i8, sc_np, "nc")
            .map_err(llvm_err)?
            .into_int_value();
        let sc_char_match = self
            .builder
            .build_int_compare(IntPredicate::EQ, sc_hc, sc_nc, "char_match")
            .map_err(llvm_err)?;
        let sc_j_next = self
            .builder
            .build_int_add(
                sc_j.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "jnext",
            )
            .map_err(llvm_err)?;
        let sc_j_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, sc_j_next, sc_nlen, "jdone")
            .map_err(llvm_err)?;
        sc_j.add_incoming(&[(&i64.const_int(0, false), sc_loop_bb)]);
        let _ = self
            .builder
            .build_conditional_branch(sc_char_match, sc_match_bb, sc_mismatch_bb);
        self.builder.position_at_end(sc_match_bb);
        sc_j.add_incoming(&[(&sc_j_next, sc_match_bb)]);
        let _ = self
            .builder
            .build_conditional_branch(sc_j_done, sc_found_bb, sc_j_loop_bb);
        self.builder.position_at_end(sc_mismatch_bb);
        let sc_i_next = self
            .builder
            .build_int_add(
                sc_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "inext",
            )
            .map_err(llvm_err)?;
        let sc_i_done = self
            .builder
            .build_int_compare(IntPredicate::SGT, sc_i_next, sc_max, "idone")
            .map_err(llvm_err)?;
        let sc_i_block = self.builder.get_insert_block().unwrap();
        sc_i.add_incoming(&[
            (&i64.const_int(0, false), sc_entry),
            (&sc_i_next, sc_i_block),
        ]);
        let _ = self
            .builder
            .build_conditional_branch(sc_i_done, sc_notfound_bb, sc_loop_bb);
        self.builder.position_at_end(sc_found_bb);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(sc_notfound_bb);
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        Ok(())
    }
}
