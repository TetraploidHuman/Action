// String runtime (moved from list/tree/remove.inc.rs — R7)

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_index_of(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i8 = self.context.i8_type();
        let str_ty = self.string_type;
        let memcmp_fn = self.module.get_function("memcmp").unwrap();
        // ---- action_string_index_of({i64, ptr}, {i64, ptr}) -> i64 (returns -1 if not found) ----
        let sio_fn = self.module.add_function(
            "action_string_index_of",
            i64.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(sio_fn, "entry");
        self.builder.position_at_end(entry);
        let sio_hay = sio_fn.get_first_param().unwrap().into_struct_value();
        let sio_nee = sio_fn.get_nth_param(1).unwrap().into_struct_value();
        let sio_hlen = self
            .builder
            .build_extract_value(sio_hay, 0, "hlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sio_hptr = self
            .builder
            .build_extract_value(sio_hay, 1, "hptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sio_nlen = self
            .builder
            .build_extract_value(sio_nee, 0, "nlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sio_nptr = self
            .builder
            .build_extract_value(sio_nee, 1, "nptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        // If needle empty, return 0
        let sio_nempty = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sio_nlen,
                i64.const_int(0, false),
                "nempty",
            )
            .map_err(llvm_err)?;
        let sio_nok = self
            .builder
            .build_int_compare(IntPredicate::SLE, sio_nlen, sio_hlen, "nok")
            .map_err(llvm_err)?;
        let _sio_can = self
            .builder
            .build_and(
                sio_nok,
                self.builder.build_not(sio_nempty, "").map_err(llvm_err)?,
                "",
            )
            .map_err(llvm_err)?;
        let sio_max = self
            .builder
            .build_int_sub(sio_hlen, sio_nlen, "max")
            .map_err(llvm_err)?;
        // Outer loop
        let sio_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(sio_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sio_oloop = self.context.append_basic_block(sio_fn, "oloop");
        let sio_obody = self.context.append_basic_block(sio_fn, "obody");
        let sio_notfound = self.context.append_basic_block(sio_fn, "notfound");
        let _ = self.builder.build_unconditional_branch(sio_oloop);
        self.builder.position_at_end(sio_oloop);
        let sio_iv = self
            .builder
            .build_load(i64, sio_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let sio_cond = self
            .builder
            .build_int_compare(IntPredicate::SLE, sio_iv, sio_max, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sio_cond, sio_obody, sio_notfound);
        self.builder.position_at_end(sio_obody);
        let sio_hp = unsafe {
            self.builder
                .build_gep(i8, sio_hptr, &[sio_iv], "hp")
                .map_err(llvm_err)
        }?;
        let sio_eq = self
            .builder
            .build_call(
                memcmp_fn,
                &[sio_hp.into(), sio_nptr.into(), sio_nlen.into()],
                "eq",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let sio_match = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sio_eq,
                self.i32_ty().const_int(0, false),
                "match",
            )
            .map_err(llvm_err)?;
        let sio_match_bb = self.context.append_basic_block(sio_fn, "match");
        let sio_next_bb = self.context.append_basic_block(sio_fn, "next");
        let _ = self
            .builder
            .build_conditional_branch(sio_match, sio_match_bb, sio_next_bb);
        self.builder.position_at_end(sio_match_bb);
        let _ = self.builder.build_return(Some(&sio_iv));
        self.builder.position_at_end(sio_next_bb);
        let sio_next_i = self
            .builder
            .build_int_add(sio_iv, i64.const_int(1, false), "nexti")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sio_i, sio_next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sio_oloop);
        self.builder.position_at_end(sio_notfound);
        let _ = self
            .builder
            .build_return(Some(&i64.const_int(-1i64 as u64, true)));
        Ok(())
    }
}
