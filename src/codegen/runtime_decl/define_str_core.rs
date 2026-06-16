// Submodule: runtime_decl/define_str_core
//
// String slice sharing helpers and RC wrappers.
// Owned strings: {len, ptr} where ptr points at RC-managed char data.
// Slice strings: {len, ptr} where ptr points at a 24-byte RC-managed header:
//   { i64 tag, i64 parent_storage, i64 data_offset }

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_core(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let i8 = self.context.i8_type();
        let rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let rc_dec_fn = self.module.get_function("action_rc_dec").unwrap();
        let free_fn = self.module.get_function("free").unwrap();

        // SLICE_TAG = 0xAC710001 (unlikely in normal UTF-8 text)
        let slice_tag = i64.const_int(0xAC710001, false);
        let eight = i64.const_int(8, false);
        let one = i64.const_int(1, false);
        let zero = i64.const_int(0, false);

        // ---- action_string_is_slice(i8* p) -> i1 ----
        let is_slice_fn = self.module.add_function(
            "action_string_is_slice",
            b1.fn_type(&[ptr.into()], false),
            None,
        );
        let is_entry = self.context.append_basic_block(is_slice_fn, "entry");
        let is_null_bb = self.context.append_basic_block(is_slice_fn, "null");
        let is_check_bb = self.context.append_basic_block(is_slice_fn, "check");
        let is_done = self.context.append_basic_block(is_slice_fn, "done");
        self.builder.position_at_end(is_entry);
        let is_p = is_slice_fn.get_first_param().unwrap().into_pointer_value();
        let is_null = self
            .builder
            .build_is_null(is_p, "is_null")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_null, is_null_bb, is_check_bb);
        self.builder.position_at_end(is_null_bb);
        let _ = self.builder.build_unconditional_branch(is_done);
        self.builder.position_at_end(is_check_bb);
        let is_i64p = self
            .builder
            .build_pointer_cast(is_p, ptr, "tag_p")
            .map_err(llvm_err)?;
        let is_tag = self
            .builder
            .build_load(i64, is_i64p, "tag")
            .map_err(llvm_err)?
            .into_int_value();
        let is_match = self
            .builder
            .build_int_compare(IntPredicate::EQ, is_tag, slice_tag, "is_slice")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(is_done);
        self.builder.position_at_end(is_done);
        let is_phi = self.builder.build_phi(b1, "r").map_err(llvm_err)?;
        is_phi.add_incoming(&[
            (&b1.const_int(0, false), is_null_bb),
            (&is_match, is_check_bb),
        ]);
        let _ = self.builder.build_return(Some(&is_phi.as_basic_value()));

        // ---- action_string_data_ptr(i8* p) -> i8* ----
        // Recursive: resolve slice headers to actual char data.
        let data_ptr_fn = self.module.add_function(
            "action_string_data_ptr",
            ptr.fn_type(&[ptr.into()], false),
            None,
        );
        let dp_entry = self.context.append_basic_block(data_ptr_fn, "entry");
        let dp_null = self.context.append_basic_block(data_ptr_fn, "null");
        let dp_check = self.context.append_basic_block(data_ptr_fn, "check");
        let dp_ret_owned = self.context.append_basic_block(data_ptr_fn, "ret_owned");
        let dp_slice = self.context.append_basic_block(data_ptr_fn, "slice");
        self.builder.position_at_end(dp_entry);
        let dp_p = data_ptr_fn.get_first_param().unwrap().into_pointer_value();
        let dp_is_null = self
            .builder
            .build_is_null(dp_p, "is_null")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(dp_is_null, dp_null, dp_check);
        self.builder.position_at_end(dp_null);
        let _ = self.builder.build_return(Some(&ptr.const_zero()));
        self.builder.position_at_end(dp_check);
        let dp_is_slice_cc = self
            .builder
            .build_call(is_slice_fn, &[dp_p.into()], "chk")
            .map_err(llvm_err)?;
        let dp_is_slice = dp_is_slice_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(dp_is_slice, dp_slice, dp_ret_owned);
        self.builder.position_at_end(dp_ret_owned);
        let _ = self.builder.build_return(Some(&dp_p));
        self.builder.position_at_end(dp_slice);
        let dp_hdr = self
            .builder
            .build_pointer_cast(dp_p, ptr, "hdr")
            .map_err(llvm_err)?;
        let dp_parent_p = unsafe {
            self.builder
                .build_gep(i8, dp_hdr, &[eight], "parent_p")
                .map_err(llvm_err)
        }?;
        let dp_parent_i64p = self
            .builder
            .build_pointer_cast(dp_parent_p, ptr, "parent_i64p")
            .map_err(llvm_err)?;
        let dp_parent_i64 = self
            .builder
            .build_load(i64, dp_parent_i64p, "parent_i64")
            .map_err(llvm_err)?
            .into_int_value();
        let dp_parent = self
            .builder
            .build_int_to_ptr(dp_parent_i64, ptr, "parent")
            .map_err(llvm_err)?;
        let dp_off_p = unsafe {
            self.builder
                .build_gep(i8, dp_hdr, &[i64.const_int(16, false)], "off_p")
                .map_err(llvm_err)
        }?;
        let dp_off_i64p = self
            .builder
            .build_pointer_cast(dp_off_p, ptr, "off_i64p")
            .map_err(llvm_err)?;
        let dp_offset = self
            .builder
            .build_load(i64, dp_off_i64p, "offset")
            .map_err(llvm_err)?
            .into_int_value();
        let dp_base_cc = self
            .builder
            .build_call(data_ptr_fn, &[dp_parent.into()], "base")
            .map_err(llvm_err)?;
        let dp_base = dp_base_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let dp_result = unsafe {
            self.builder
                .build_gep(i8, dp_base, &[dp_offset], "result")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_return(Some(&dp_result));

        // ---- action_string_data({i64, ptr}) -> i8* ----
        let data_fn = self.module.add_function(
            "action_string_data",
            ptr.fn_type(&[str_ty.into()], false),
            None,
        );
        let d_entry = self.context.append_basic_block(data_fn, "entry");
        self.builder.position_at_end(d_entry);
        let d_s = data_fn.get_first_param().unwrap().into_struct_value();
        let d_p = self
            .builder
            .build_extract_value(d_s, 1, "p")
            .map_err(llvm_err)?
            .into_pointer_value();
        let d_cc = self
            .builder
            .build_call(data_ptr_fn, &[d_p.into()], "dp")
            .map_err(llvm_err)?;
        let d_result = d_cc.try_as_basic_value().unwrap_basic();
        let _ = self.builder.build_return(Some(&d_result));

        // ---- action_string_rc_inc({i64, ptr}) -> void ----
        let str_rc_inc_fn = self.module.add_function(
            "action_string_rc_inc",
            void.fn_type(&[str_ty.into()], false),
            None,
        );
        let ri_entry = self.context.append_basic_block(str_rc_inc_fn, "entry");
        let ri_done = self.context.append_basic_block(str_rc_inc_fn, "done");
        self.builder.position_at_end(ri_entry);
        let ri_s = str_rc_inc_fn.get_first_param().unwrap().into_struct_value();
        let ri_p = self
            .builder
            .build_extract_value(ri_s, 1, "p")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ri_null = self.builder.build_is_null(ri_p, "null").map_err(llvm_err)?;
        let ri_do = self.context.append_basic_block(str_rc_inc_fn, "do_inc");
        let _ = self
            .builder
            .build_conditional_branch(ri_null, ri_done, ri_do);
        self.builder.position_at_end(ri_do);
        let _ = self
            .builder
            .build_call(rc_inc_fn, &[ri_p.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ri_done);
        self.builder.position_at_end(ri_done);
        let _ = self.builder.build_return(None);

        // ---- action_string_rc_dec({i64, ptr}) -> void ----
        let str_rc_dec_fn = self.module.add_function(
            "action_string_rc_dec",
            void.fn_type(&[str_ty.into()], false),
            None,
        );
        let rd_entry = self.context.append_basic_block(str_rc_dec_fn, "entry");
        let rd_done = self.context.append_basic_block(str_rc_dec_fn, "done");
        let rd_null = self.context.append_basic_block(str_rc_dec_fn, "null");
        let rd_check = self.context.append_basic_block(str_rc_dec_fn, "check");
        let rd_owned_dec = self.context.append_basic_block(str_rc_dec_fn, "owned_dec");
        let rd_slice = self.context.append_basic_block(str_rc_dec_fn, "slice");
        let rd_slice_free = self.context.append_basic_block(str_rc_dec_fn, "slice_free");
        self.builder.position_at_end(rd_entry);
        let rd_s = str_rc_dec_fn.get_first_param().unwrap().into_struct_value();
        let rd_p = self
            .builder
            .build_extract_value(rd_s, 1, "p")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rd_is_null = self.builder.build_is_null(rd_p, "null").map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rd_is_null, rd_null, rd_check);
        self.builder.position_at_end(rd_null);
        let _ = self.builder.build_unconditional_branch(rd_done);
        self.builder.position_at_end(rd_check);
        let rd_is_slice_cc = self
            .builder
            .build_call(is_slice_fn, &[rd_p.into()], "chk")
            .map_err(llvm_err)?;
        let rd_is_slice = rd_is_slice_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(rd_is_slice, rd_slice, rd_owned_dec);
        self.builder.position_at_end(rd_owned_dec);
        let _ = self
            .builder
            .build_call(rc_dec_fn, &[rd_p.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rd_done);
        self.builder.position_at_end(rd_slice);
        // Load parent before decrementing slice header
        let rd_hdr = self
            .builder
            .build_pointer_cast(rd_p, ptr, "hdr")
            .map_err(llvm_err)?;
        let rd_parent_p = unsafe {
            self.builder
                .build_gep(i8, rd_hdr, &[eight], "parent_p")
                .map_err(llvm_err)
        }?;
        let rd_parent_i64p = self
            .builder
            .build_pointer_cast(rd_parent_p, ptr, "parent_i64p")
            .map_err(llvm_err)?;
        let rd_parent_i64 = self
            .builder
            .build_load(i64, rd_parent_i64p, "parent_i64")
            .map_err(llvm_err)?
            .into_int_value();
        let rd_parent = self
            .builder
            .build_int_to_ptr(rd_parent_i64, ptr, "parent")
            .map_err(llvm_err)?;
        // Manual RC dec on slice header
        let rd_p_i64 = self
            .builder
            .build_ptr_to_int(rd_p, i64, "p_i64")
            .map_err(llvm_err)?;
        let rd_rc_addr = self
            .builder
            .build_int_sub(rd_p_i64, eight, "rc_addr")
            .map_err(llvm_err)?;
        let rd_rc_p = self
            .builder
            .build_int_to_ptr(rd_rc_addr, ptr, "rc_p")
            .map_err(llvm_err)?;
        let rd_rc = self
            .builder
            .build_load(i64, rd_rc_p, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let rd_new_rc = self
            .builder
            .build_int_sub(rd_rc, one, "new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(rd_rc_p, rd_new_rc)
            .map_err(llvm_err)?;
        let rd_is_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, rd_new_rc, zero, "zero")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rd_is_zero, rd_slice_free, rd_done);
        self.builder.position_at_end(rd_slice_free);
        let rd_free_ptr = self
            .builder
            .build_int_to_ptr(rd_rc_addr, ptr, "free_ptr")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(free_fn, &[rd_free_ptr.into()], "")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(rc_dec_fn, &[rd_parent.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rd_done);
        self.builder.position_at_end(rd_done);
        let _ = self.builder.build_return(None);

        Ok(())
    }
}
