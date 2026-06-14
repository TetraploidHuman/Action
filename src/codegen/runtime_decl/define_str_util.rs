// Submodule: runtime_decl/define_str_util
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::values::IntValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_util(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let _void = self.void_ty();
        let _ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let _i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();

        let memcpy_fn = self.module.get_function("memcpy").unwrap();

        let _list_create_fn = self.module.get_function("action_list_create").unwrap();
        let _list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        // ---- action_string_to_upper({i64, ptr}) -> {i64, ptr} ----
        let to_upper_fn = self.module.add_function(
            "action_string_to_upper",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(to_upper_fn, "entry");
        self.builder.position_at_end(entry);
        let str_param = to_upper_fn.get_first_param().unwrap().into_struct_value();
        let str_len = self
            .builder
            .build_extract_value(str_param, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let str_data = self
            .builder
            .build_extract_value(str_param, 1, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alloc_len = self
            .builder
            .build_int_add(str_len, i64.const_int(1, false), "alloc_len")
            .map_err(llvm_err)?;
        let new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[alloc_len.into()], "new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Loop: for i in 0..len, copy byte, convert if lowercase
        let loop_bb = self.context.append_basic_block(to_upper_fn, "loop");
        let body_bb = self.context.append_basic_block(to_upper_fn, "body");
        let done_bb = self.context.append_basic_block(to_upper_fn, "done");
        let i_alloca = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_bb);
        self.builder.position_at_end(loop_bb);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let not_done = self
            .builder
            .build_int_compare(IntPredicate::ULT, i_val, str_len, "not_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(not_done, body_bb, done_bb);
        self.builder.position_at_end(body_bb);
        let src_ptr = unsafe {
            self.builder
                .build_gep(i8, str_data, &[i_val], "src_ptr")
                .map_err(llvm_err)
        }?;
        let c = self
            .builder
            .build_load(i8, src_ptr, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let is_lower = self
            .builder
            .build_int_compare(
                IntPredicate::UGE,
                c,
                i8.const_int('a' as u64, false),
                "ge_a",
            )
            .map_err(llvm_err)?;
        let is_lower2 = self
            .builder
            .build_int_compare(
                IntPredicate::ULE,
                c,
                i8.const_int('z' as u64, false),
                "le_z",
            )
            .map_err(llvm_err)?;
        let is_lower_final = self
            .builder
            .build_and(is_lower, is_lower2, "is_lower")
            .map_err(llvm_err)?;
        let upper_c = self
            .builder
            .build_int_sub(c, i8.const_int(32, false), "upper_c")
            .map_err(llvm_err)?;
        let conv = self
            .builder
            .build_select(is_lower_final, upper_c, c, "conv")
            .map_err(llvm_err)?
            .into_int_value();
        let dst_ptr = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[i_val], "dst_ptr")
                .map_err(llvm_err)
        }?;
        self.builder.build_store(dst_ptr, conv).map_err(llvm_err)?;
        let next_i = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "next_i")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_bb);
        self.builder.position_at_end(done_bb);
        let null_gep = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[str_len], "null_ptr")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(null_gep, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, str_len, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, new_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        // ---- action_string_to_lower({i64, ptr}) -> {i64, ptr} ----
        let to_lower_fn = self.module.add_function(
            "action_string_to_lower",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(to_lower_fn, "entry");
        self.builder.position_at_end(entry);
        let str_param = to_lower_fn.get_first_param().unwrap().into_struct_value();
        let str_len = self
            .builder
            .build_extract_value(str_param, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let str_data = self
            .builder
            .build_extract_value(str_param, 1, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alloc_len = self
            .builder
            .build_int_add(str_len, i64.const_int(1, false), "alloc_len")
            .map_err(llvm_err)?;
        let new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[alloc_len.into()], "new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let loop_bb = self.context.append_basic_block(to_lower_fn, "loop");
        let body_bb = self.context.append_basic_block(to_lower_fn, "body");
        let done_bb = self.context.append_basic_block(to_lower_fn, "done");
        let i_alloca = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_bb);
        self.builder.position_at_end(loop_bb);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let not_done = self
            .builder
            .build_int_compare(IntPredicate::ULT, i_val, str_len, "not_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(not_done, body_bb, done_bb);
        self.builder.position_at_end(body_bb);
        let src_ptr = unsafe {
            self.builder
                .build_gep(i8, str_data, &[i_val], "src_ptr")
                .map_err(llvm_err)
        }?;
        let c = self
            .builder
            .build_load(i8, src_ptr, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let is_upper = self
            .builder
            .build_int_compare(
                IntPredicate::UGE,
                c,
                i8.const_int('A' as u64, false),
                "ge_A",
            )
            .map_err(llvm_err)?;
        let is_upper2 = self
            .builder
            .build_int_compare(
                IntPredicate::ULE,
                c,
                i8.const_int('Z' as u64, false),
                "le_Z",
            )
            .map_err(llvm_err)?;
        let is_upper_final = self
            .builder
            .build_and(is_upper, is_upper2, "is_upper")
            .map_err(llvm_err)?;
        let lower_c = self
            .builder
            .build_int_add(c, i8.const_int(32, false), "lower_c")
            .map_err(llvm_err)?;
        let conv = self
            .builder
            .build_select(is_upper_final, lower_c, c, "conv")
            .map_err(llvm_err)?
            .into_int_value();
        let dst_ptr = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[i_val], "dst_ptr")
                .map_err(llvm_err)
        }?;
        self.builder.build_store(dst_ptr, conv).map_err(llvm_err)?;
        let next_i = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "next_i")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_bb);
        self.builder.position_at_end(done_bb);
        let null_gep = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[str_len], "null_ptr")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(null_gep, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, str_len, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, new_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        // ---- action_string_trim({i64, ptr}) -> {i64, ptr} ----
        let trim_fn = self.module.add_function(
            "action_string_trim",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(trim_fn, "entry");
        self.builder.position_at_end(entry);
        let str_param = trim_fn.get_first_param().unwrap().into_struct_value();
        let str_len = self
            .builder
            .build_extract_value(str_param, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let str_data = self
            .builder
            .build_extract_value(str_param, 1, "data")
            .map_err(llvm_err)?
            .into_pointer_value();

        // Helper to build is-whitespace check for a char value
        let build_is_ws = |builder: &inkwell::builder::Builder<'ctx>,
                           c: IntValue<'ctx>|
         -> Result<IntValue<'ctx>, String> {
            let is_sp = builder
                .build_int_compare(
                    IntPredicate::EQ,
                    c,
                    i8.const_int(b' ' as u64, false),
                    "is_sp",
                )
                .map_err(llvm_err)?;
            let is_tab = builder
                .build_int_compare(
                    IntPredicate::EQ,
                    c,
                    i8.const_int(b'\t' as u64, false),
                    "is_tab",
                )
                .map_err(llvm_err)?;
            let is_nl = builder
                .build_int_compare(
                    IntPredicate::EQ,
                    c,
                    i8.const_int(b'\n' as u64, false),
                    "is_nl",
                )
                .map_err(llvm_err)?;
            let is_cr = builder
                .build_int_compare(
                    IntPredicate::EQ,
                    c,
                    i8.const_int(b'\r' as u64, false),
                    "is_cr",
                )
                .map_err(llvm_err)?;
            let ws1 = builder.build_or(is_sp, is_tab, "ws1").map_err(llvm_err)?;
            let ws2 = builder.build_or(is_nl, is_cr, "ws2").map_err(llvm_err)?;
            builder.build_or(ws1, ws2, "is_ws").map_err(llvm_err)
        };

        // Find start (left trim)
        let find_start_hdr = self.context.append_basic_block(trim_fn, "find_start_hdr");
        let find_start_body = self.context.append_basic_block(trim_fn, "find_start_body");
        let start_done = self.context.append_basic_block(trim_fn, "start_done");
        let start_idx = self
            .builder
            .build_alloca(i64, "start_idx")
            .map_err(llvm_err)?;
        self.builder
            .build_store(start_idx, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(find_start_hdr);

        // find_start_hdr: while start < len
        self.builder.position_at_end(find_start_hdr);
        let si = self
            .builder
            .build_load(i64, start_idx, "si")
            .map_err(llvm_err)?
            .into_int_value();
        let si_lt_len = self
            .builder
            .build_int_compare(IntPredicate::ULT, si, str_len, "si_lt_len")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(si_lt_len, find_start_body, start_done);

        self.builder.position_at_end(find_start_body);
        let sp = unsafe {
            self.builder
                .build_gep(i8, str_data, &[si], "sp")
                .map_err(llvm_err)
        }?;
        let sc = self
            .builder
            .build_load(i8, sp, "sc")
            .map_err(llvm_err)?
            .into_int_value();
        let is_ws = build_is_ws(&self.builder, sc)?;
        let si_plus1 = self
            .builder
            .build_int_add(si, i64.const_int(1, false), "si_plus1")
            .map_err(llvm_err)?;
        let new_si = self
            .builder
            .build_select(is_ws, si_plus1, si, "new_si")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(start_idx, new_si)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_ws, find_start_hdr, start_done);

        // Find end (right trim) - similar loop going backwards
        self.builder.position_at_end(start_done);
        let find_end_hdr = self.context.append_basic_block(trim_fn, "find_end_hdr");
        let find_end_body = self.context.append_basic_block(trim_fn, "find_end_body");
        let end_done = self.context.append_basic_block(trim_fn, "end_done");
        let end_idx = self
            .builder
            .build_alloca(i64, "end_idx")
            .map_err(llvm_err)?;
        self.builder
            .build_store(end_idx, str_len)
            .map_err(llvm_err)?;
        // Load start value here so it dominates uses in end_done
        let final_si = self
            .builder
            .build_load(i64, start_idx, "final_si")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(find_end_hdr);

        // find_end_hdr: while end > start
        self.builder.position_at_end(find_end_hdr);
        let ei = self
            .builder
            .build_load(i64, end_idx, "ei")
            .map_err(llvm_err)?
            .into_int_value();
        let ei_gt_si = self
            .builder
            .build_int_compare(IntPredicate::UGT, ei, final_si, "ei_gt_si")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ei_gt_si, find_end_body, end_done);

        self.builder.position_at_end(find_end_body);
        let ei_minus1 = self
            .builder
            .build_int_sub(ei, i64.const_int(1, false), "ei_minus1")
            .map_err(llvm_err)?;
        let ep = unsafe {
            self.builder
                .build_gep(i8, str_data, &[ei_minus1], "ep")
                .map_err(llvm_err)
        }?;
        let ec = self
            .builder
            .build_load(i8, ep, "ec")
            .map_err(llvm_err)?
            .into_int_value();
        let is_ws = build_is_ws(&self.builder, ec)?;
        let new_ei = self
            .builder
            .build_select(is_ws, ei_minus1, ei, "new_ei")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(end_idx, new_ei)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_ws, find_end_hdr, end_done);

        // end_done: allocate and copy
        self.builder.position_at_end(end_done);
        // Reload end since it might have changed in the loop
        let final_ei = self
            .builder
            .build_load(i64, end_idx, "final_ei")
            .map_err(llvm_err)?
            .into_int_value();
        let new_len = self
            .builder
            .build_int_sub(final_ei, final_si, "new_len")
            .map_err(llvm_err)?;
        // Allocate new_len + 1 for null terminator
        let alloc_len = self
            .builder
            .build_int_add(new_len, i64.const_int(1, false), "alloc_len")
            .map_err(llvm_err)?;
        let new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[alloc_len.into()], "new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let src_offset = unsafe {
            self.builder
                .build_gep(i8, str_data, &[final_si], "src_offset")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[new_buf.into(), src_offset.into(), new_len.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Null terminate
        let null_gep = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[new_len], "null_ptr")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(null_gep, i8.const_int(0, false))
            .map_err(llvm_err)?;
        // Return {new_len, new_buf}
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, new_len, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, new_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        Ok(())
    }
}
