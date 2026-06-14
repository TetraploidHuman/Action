// Submodule: runtime_decl/define_str_extra
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_extra(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let _void = self.void_ty();
        let _ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();

        let memcmp_fn = self.module.get_function("memcmp").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();

        let _list_create_fn = self.module.get_function("action_list_create").unwrap();
        let _list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        // ---- action_string_starts_with({i64, ptr}, {i64, ptr}) -> i1 ----
        let sw_fn = self.module.add_function(
            "action_string_starts_with",
            self.bool_ty()
                .fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let sw_entry = self.context.append_basic_block(sw_fn, "entry");
        self.builder.position_at_end(sw_entry);
        let sw_s = sw_fn.get_first_param().unwrap().into_struct_value();
        let sw_pre = sw_fn.get_nth_param(1).unwrap().into_struct_value();
        let sw_slen = self
            .builder
            .build_extract_value(sw_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sw_plen = self
            .builder
            .build_extract_value(sw_pre, 0, "plen")
            .map_err(llvm_err)?
            .into_int_value();
        let sw_sdata = self
            .builder
            .build_extract_value(sw_s, 1, "sdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sw_pdata = self
            .builder
            .build_extract_value(sw_pre, 1, "pdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sw_len_ok = self
            .builder
            .build_int_compare(IntPredicate::UGE, sw_slen, sw_plen, "len_ok")
            .map_err(llvm_err)?;
        let sw_check = self.context.append_basic_block(sw_fn, "check");
        let sw_cmp = self.context.append_basic_block(sw_fn, "cmp");
        let sw_false = self.context.append_basic_block(sw_fn, "false");
        let sw_done = self.context.append_basic_block(sw_fn, "done");
        let _ = self
            .builder
            .build_conditional_branch(sw_len_ok, sw_check, sw_false);
        // check: empty prefix → true, else → cmp
        self.builder.position_at_end(sw_check);
        let sw_pz = self
            .builder
            .build_int_compare(IntPredicate::EQ, sw_plen, i64.const_int(0, false), "pz")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sw_pz, sw_done, sw_cmp);
        // cmp: memcmp
        self.builder.position_at_end(sw_cmp);
        let sw_mc = self
            .builder
            .build_call(
                memcmp_fn,
                &[sw_sdata.into(), sw_pdata.into(), sw_plen.into()],
                "mc",
            )
            .map_err(llvm_err)?;
        let sw_mcr = sw_mc.try_as_basic_value().unwrap_basic().into_int_value();
        let sw_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, sw_mcr, i32.const_int(0, false), "eq")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sw_done);
        // false
        self.builder.position_at_end(sw_false);
        let _ = self.builder.build_unconditional_branch(sw_done);
        // done: phi [pz from check, eq from cmp, false from false]
        self.builder.position_at_end(sw_done);
        let sw_phi = self
            .builder
            .build_phi(self.bool_ty(), "sw_result")
            .map_err(llvm_err)?;
        sw_phi.add_incoming(&[
            (&sw_pz, sw_check),
            (&sw_eq, sw_cmp),
            (&self.bool_ty().const_int(0, false), sw_false),
        ]);
        let _ = self.builder.build_return(Some(&sw_phi.as_basic_value()));

        // ---- action_string_ends_with({i64, ptr}, {i64, ptr}) -> i1 ----
        let ew_fn = self.module.add_function(
            "action_string_ends_with",
            self.bool_ty()
                .fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let ew_entry = self.context.append_basic_block(ew_fn, "entry");
        self.builder.position_at_end(ew_entry);
        let ew_s = ew_fn.get_first_param().unwrap().into_struct_value();
        let ew_suf = ew_fn.get_nth_param(1).unwrap().into_struct_value();
        let ew_slen = self
            .builder
            .build_extract_value(ew_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let ew_suflen = self
            .builder
            .build_extract_value(ew_suf, 0, "suflen")
            .map_err(llvm_err)?
            .into_int_value();
        let ew_sdata = self
            .builder
            .build_extract_value(ew_s, 1, "sdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ew_sufdata = self
            .builder
            .build_extract_value(ew_suf, 1, "sufdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ew_len_ok = self
            .builder
            .build_int_compare(IntPredicate::UGE, ew_slen, ew_suflen, "len_ok")
            .map_err(llvm_err)?;
        let ew_check = self.context.append_basic_block(ew_fn, "check");
        let ew_cmp = self.context.append_basic_block(ew_fn, "cmp");
        let ew_false = self.context.append_basic_block(ew_fn, "false");
        let ew_done = self.context.append_basic_block(ew_fn, "done");
        let _ = self
            .builder
            .build_conditional_branch(ew_len_ok, ew_check, ew_false);
        // check: empty suffix → true, else → cmp
        self.builder.position_at_end(ew_check);
        let ew_sufz = self
            .builder
            .build_int_compare(IntPredicate::EQ, ew_suflen, i64.const_int(0, false), "sufz")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ew_sufz, ew_done, ew_cmp);
        // cmp: memcmp from offset len-suffixlen
        self.builder.position_at_end(ew_cmp);
        let ew_off = self
            .builder
            .build_int_sub(ew_slen, ew_suflen, "off")
            .map_err(llvm_err)?;
        let ew_sp = unsafe {
            self.builder
                .build_gep(i8, ew_sdata, &[ew_off], "sp")
                .map_err(llvm_err)
        }?;
        let ew_mc = self
            .builder
            .build_call(
                memcmp_fn,
                &[ew_sp.into(), ew_sufdata.into(), ew_suflen.into()],
                "mc",
            )
            .map_err(llvm_err)?;
        let ew_mcr = ew_mc.try_as_basic_value().unwrap_basic().into_int_value();
        let ew_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, ew_mcr, i32.const_int(0, false), "eq")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ew_done);
        // false
        self.builder.position_at_end(ew_false);
        let _ = self.builder.build_unconditional_branch(ew_done);
        // done: phi [sufz from check, eq from cmp, false from false]
        self.builder.position_at_end(ew_done);
        let ew_phi = self
            .builder
            .build_phi(self.bool_ty(), "ew_result")
            .map_err(llvm_err)?;
        ew_phi.add_incoming(&[
            (&ew_sufz, ew_check),
            (&ew_eq, ew_cmp),
            (&self.bool_ty().const_int(0, false), ew_false),
        ]);
        let _ = self.builder.build_return(Some(&ew_phi.as_basic_value()));

        // ---- action_string_substring({i64, ptr}, i64 start, i64 len) -> {i64, ptr} ----
        let sub_fn = self.module.add_function(
            "action_string_substring",
            str_ty.fn_type(&[str_ty.into(), i64.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(sub_fn, "entry");
        self.builder.position_at_end(entry);
        let sub_s = sub_fn.get_first_param().unwrap().into_struct_value();
        let sub_start = sub_fn.get_nth_param(1).unwrap().into_int_value();
        let sub_len = sub_fn.get_nth_param(2).unwrap().into_int_value();
        let sub_slen = self
            .builder
            .build_extract_value(sub_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sub_sdata = self
            .builder
            .build_extract_value(sub_s, 1, "sdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Clamp: if start >= slen, return empty string
        let sub_start_ok = self
            .builder
            .build_int_compare(IntPredicate::ULT, sub_start, sub_slen, "start_ok")
            .map_err(llvm_err)?;
        let sub_end = self
            .builder
            .build_int_add(sub_start, sub_len, "end")
            .map_err(llvm_err)?;
        let sub_end_ok = self
            .builder
            .build_int_compare(IntPredicate::ULE, sub_end, sub_slen, "end_ok")
            .map_err(llvm_err)?;
        let sub_clamped_end = self
            .builder
            .build_select(sub_end_ok, sub_end, sub_slen, "clamped_end")
            .map_err(llvm_err)?
            .into_int_value();
        let sub_actual_len = self
            .builder
            .build_int_sub(sub_clamped_end, sub_start, "actual_len")
            .map_err(llvm_err)?;
        let sub_clamped_start = self
            .builder
            .build_select(sub_start_ok, sub_start, sub_slen, "clamped_start")
            .map_err(llvm_err)?
            .into_int_value();
        let _sub_zero_len = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sub_actual_len,
                i64.const_int(0, false),
                "zero_len",
            )
            .map_err(llvm_err)?;
        // Allocate and copy
        let sub_alc = self
            .builder
            .build_int_add(sub_actual_len, i64.const_int(1, false), "alc")
            .map_err(llvm_err)?;
        let sub_buf = self
            .builder
            .build_call(malloc_rc_fn, &[sub_alc.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sub_src = unsafe {
            self.builder
                .build_gep(i8, sub_sdata, &[sub_clamped_start], "src")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sub_buf.into(), sub_src.into(), sub_actual_len.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sub_null = unsafe {
            self.builder
                .build_gep(i8, sub_buf, &[sub_actual_len], "null")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(sub_null, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let sub_undef = str_ty.get_undef();
        let sub_r1 = self
            .builder
            .build_insert_value(sub_undef, sub_actual_len, 0, "r1")
            .map_err(llvm_err)?;
        let sub_r2 = self
            .builder
            .build_insert_value(sub_r1, sub_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sub_r2));

        Ok(())
    }
}
