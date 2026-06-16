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
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();

        let memcmp_fn = self.module.get_function("memcmp").unwrap();
        let _memcpy_fn = self.module.get_function("memcpy").unwrap();
        let str_data_fn = self.module.get_function("action_string_data").unwrap();
        let is_slice_fn = self.module.get_function("action_string_is_slice").unwrap();
        let rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let slice_tag = i64.const_int(0xAC710001, false);
        let hdr_size = i64.const_int(24, false);
        let eight = i64.const_int(8, false);
        let sixteen = i64.const_int(16, false);
        let zero = i64.const_int(0, false);

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
        let sw_sdata_cc = self
            .builder
            .build_call(str_data_fn, &[sw_s.into()], "sd")
            .map_err(llvm_err)?;
        let sw_sdata = sw_sdata_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sw_pdata_cc = self
            .builder
            .build_call(str_data_fn, &[sw_pre.into()], "pd")
            .map_err(llvm_err)?;
        let sw_pdata = sw_pdata_cc
            .try_as_basic_value()
            .unwrap_basic()
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
        let ew_sdata_cc = self
            .builder
            .build_call(str_data_fn, &[ew_s.into()], "sd")
            .map_err(llvm_err)?;
        let ew_sdata = ew_sdata_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let ew_sufdata_cc = self
            .builder
            .build_call(str_data_fn, &[ew_suf.into()], "sufd")
            .map_err(llvm_err)?;
        let ew_sufdata = ew_sufdata_cc
            .try_as_basic_value()
            .unwrap_basic()
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
        // Slice sharing: returns a slice header pointing into parent data (no copy).
        let sub_fn = self.module.add_function(
            "action_string_substring",
            str_ty.fn_type(&[str_ty.into(), i64.into(), i64.into()], false),
            None,
        );
        let sub_entry = self.context.append_basic_block(sub_fn, "entry");
        let sub_empty = self.context.append_basic_block(sub_fn, "empty");
        let sub_slice = self.context.append_basic_block(sub_fn, "slice");
        self.builder.position_at_end(sub_entry);
        let sub_s = sub_fn.get_first_param().unwrap().into_struct_value();
        let sub_start = sub_fn.get_nth_param(1).unwrap().into_int_value();
        let sub_len = sub_fn.get_nth_param(2).unwrap().into_int_value();
        let sub_slen = self
            .builder
            .build_extract_value(sub_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sub_storage = self
            .builder
            .build_extract_value(sub_s, 1, "storage")
            .map_err(llvm_err)?
            .into_pointer_value();
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
        let sub_zero_len = self
            .builder
            .build_int_compare(IntPredicate::EQ, sub_actual_len, zero, "zero_len")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sub_zero_len, sub_empty, sub_slice);
        // Empty substring: allocate owned empty buffer
        self.builder.position_at_end(sub_empty);
        let sub_ebuf = self
            .builder
            .build_call(malloc_rc_fn, &[i64.const_int(1, false).into()], "ebuf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        self.builder
            .build_store(sub_ebuf, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let sub_eundef = str_ty.get_undef();
        let sub_er1 = self
            .builder
            .build_insert_value(sub_eundef, zero, 0, "er1")
            .map_err(llvm_err)?;
        let sub_er2 = self
            .builder
            .build_insert_value(sub_er1, sub_ebuf, 1, "er2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sub_er2));
        // Non-empty: create slice header {tag, parent_storage, data_offset}
        self.builder.position_at_end(sub_slice);
        let sub_is_slice_cc = self
            .builder
            .build_call(is_slice_fn, &[sub_storage.into()], "chk")
            .map_err(llvm_err)?;
        let sub_parent_is_slice = sub_is_slice_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let sub_hdr = self
            .builder
            .build_call(malloc_rc_fn, &[hdr_size.into()], "hdr")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sub_hdr_i64p = self
            .builder
            .build_pointer_cast(sub_hdr, ptr, "hdr_i64p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(sub_hdr_i64p, slice_tag)
            .map_err(llvm_err)?;
        let sub_parent_p = unsafe {
            self.builder
                .build_gep(i8, sub_hdr, &[eight], "parent_p")
                .map_err(llvm_err)
        }?;
        let sub_parent_i64p = self
            .builder
            .build_pointer_cast(sub_parent_p, ptr, "parent_i64p")
            .map_err(llvm_err)?;
        let sub_storage_i64 = self
            .builder
            .build_ptr_to_int(sub_storage, i64, "storage_i64")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(sub_parent_i64p, sub_storage_i64)
            .map_err(llvm_err)?;
        // base_offset = parent_is_slice ? load(parent+16) : 0
        let sub_parent_off_p = unsafe {
            self.builder
                .build_gep(i8, sub_storage, &[sixteen], "parent_off_p")
                .map_err(llvm_err)
        }?;
        let sub_parent_off_i64p = self
            .builder
            .build_pointer_cast(sub_parent_off_p, ptr, "parent_off_i64p")
            .map_err(llvm_err)?;
        let sub_parent_off = self
            .builder
            .build_load(i64, sub_parent_off_i64p, "parent_off")
            .map_err(llvm_err)?
            .into_int_value();
        let sub_base_off = self
            .builder
            .build_select(sub_parent_is_slice, sub_parent_off, zero, "base_off")
            .map_err(llvm_err)?
            .into_int_value();
        let sub_abs_off = self
            .builder
            .build_int_add(sub_base_off, sub_clamped_start, "abs_off")
            .map_err(llvm_err)?;
        let sub_off_p = unsafe {
            self.builder
                .build_gep(i8, sub_hdr, &[sixteen], "off_p")
                .map_err(llvm_err)
        }?;
        let sub_off_i64p = self
            .builder
            .build_pointer_cast(sub_off_p, ptr, "off_i64p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(sub_off_i64p, sub_abs_off)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(rc_inc_fn, &[sub_storage.into()], "")
            .map_err(llvm_err)?;
        let sub_sundef = str_ty.get_undef();
        let sub_sr1 = self
            .builder
            .build_insert_value(sub_sundef, sub_actual_len, 0, "sr1")
            .map_err(llvm_err)?;
        let sub_sr2 = self
            .builder
            .build_insert_value(sub_sr1, sub_hdr, 1, "sr2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sub_sr2));

        Ok(())
    }
}
