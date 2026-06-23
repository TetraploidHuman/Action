// Submodule: runtime_decl/str_adv/split (R6)

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_split(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let ptr = self.ptr_ty();
        let _b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let memcmp_fn = self.module.get_function("memcmp").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        let str_data_fn = self.module.get_function("action_string_data").unwrap();

        // ---- action_string_split({i64, ptr}, {i64, ptr}) -> {ptr, i64, i64} ----
        // Tree-based: uses action_list_create + action_list_push for result list.
        let sp_fn = self.module.add_function(
            "action_string_split",
            self.list_type
                .fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let sp_entry = self.context.append_basic_block(sp_fn, "entry");
        self.builder.position_at_end(sp_entry);
        let sp_s = sp_fn.get_first_param().unwrap().into_struct_value();
        let sp_delim = sp_fn.get_nth_param(1).unwrap().into_struct_value();
        let sp_slen = self
            .builder
            .build_extract_value(sp_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_sdata_cc = self
            .builder
            .build_call(str_data_fn, &[sp_s.into()], "sd")
            .map_err(llvm_err)?;
        let sp_sdata = sp_sdata_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sp_dlen = self
            .builder
            .build_extract_value(sp_delim, 0, "dlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_ddata_cc = self
            .builder
            .build_call(str_data_fn, &[sp_delim.into()], "dd")
            .map_err(llvm_err)?;
        let sp_ddata = sp_ddata_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let one = i64.const_int(1, false);
        let zero = i64.const_int(0, false);

        // Create result list via action_list_create
        let sp_list = self.call_rt("action_list_create", &[zero.into()])?;
        let sp_list_bv = sp_list.try_as_basic_value().unwrap_basic();
        let sp_list_ptr = self
            .builder
            .build_alloca(self.list_type, "list_ptr")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_list_ptr, sp_list_bv)
            .map_err(llvm_err)?;

        // Need to check dlen > 0 to avoid infinite loops
        let sp_dzero = self
            .builder
            .build_int_compare(IntPredicate::EQ, sp_dlen, zero, "dzero")
            .map_err(llvm_err)?;
        let sp_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        let sp_last = self.builder.build_alloca(i64, "last").map_err(llvm_err)?;
        self.builder.build_store(sp_i, zero).map_err(llvm_err)?;
        self.builder.build_store(sp_last, zero).map_err(llvm_err)?;
        let sp_fill_hdr = self.context.append_basic_block(sp_fn, "fill_hdr");
        let sp_fill_body = self.context.append_basic_block(sp_fn, "fill_body");
        let sp_fill_push = self.context.append_basic_block(sp_fn, "fill_push");
        let sp_fill_next = self.context.append_basic_block(sp_fn, "fill_next");
        let sp_fill_last = self.context.append_basic_block(sp_fn, "fill_last");
        let sp_fill_done = self.context.append_basic_block(sp_fn, "fill_done");
        let _ = self
            .builder
            .build_conditional_branch(sp_dzero, sp_fill_last, sp_fill_hdr);

        // fill_hdr: while i + dlen <= slen
        self.builder.position_at_end(sp_fill_hdr);
        let sp_iv = self
            .builder
            .build_load(i64, sp_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_end = self
            .builder
            .build_int_add(sp_iv, sp_dlen, "end")
            .map_err(llvm_err)?;
        let sp_in_range = self
            .builder
            .build_int_compare(IntPredicate::ULE, sp_end, sp_slen, "in_range")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sp_in_range, sp_fill_body, sp_fill_last);

        self.builder.position_at_end(sp_fill_body);
        let sp_src = unsafe {
            self.builder
                .build_gep(i8, sp_sdata, &[sp_iv], "src")
                .map_err(llvm_err)
        }?;
        let sp_mc = self
            .builder
            .build_call(
                memcmp_fn,
                &[sp_src.into(), sp_ddata.into(), sp_dlen.into()],
                "mc",
            )
            .map_err(llvm_err)?;
        let sp_mcr = sp_mc.try_as_basic_value().unwrap_basic().into_int_value();
        let sp_match = self
            .builder
            .build_int_compare(IntPredicate::EQ, sp_mcr, i32.const_int(0, false), "match")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sp_match, sp_fill_push, sp_fill_next);

        // fill_push: create substring [last, i) and push to list
        self.builder.position_at_end(sp_fill_push);
        let sp_last_v = self
            .builder
            .build_load(i64, sp_last, "last_v")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_seg_len = self
            .builder
            .build_int_sub(sp_iv, sp_last_v, "seg_len")
            .map_err(llvm_err)?;
        let sp_salc = self
            .builder
            .build_int_add(sp_seg_len, one, "salc")
            .map_err(llvm_err)?;
        let sp_sbuf = self
            .builder
            .build_call(malloc_rc_fn, &[sp_salc.into()], "sbuf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for this newly allocated buffer (malloc_rc starts at 0)
        let sp_sbuf_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(sp_sbuf, i64, "sp_sbuf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "sp_sbuf_rc_addr",
            )
            .map_err(llvm_err)?;
        let sp_sbuf_rc_p = self
            .builder
            .build_int_to_ptr(sp_sbuf_rc_addr, ptr, "sp_sbuf_rc_p")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_sbuf_rc_p, i64.const_int(1, false))
            .map_err(llvm_err)?;
        let sp_ssrc = unsafe {
            self.builder
                .build_gep(i8, sp_sdata, &[sp_last_v], "ssrc")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sp_sbuf.into(), sp_ssrc.into(), sp_seg_len.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sp_snull = unsafe {
            self.builder
                .build_gep(i8, sp_sbuf, &[sp_seg_len], "snull")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(sp_snull, i8.const_int(0, false))
            .map_err(llvm_err)?;
        // Build fat struct {seg_len, sbuf}
        let sp_fat_undef = str_ty.get_undef();
        let sp_fat = self
            .builder
            .build_insert_value(sp_fat_undef, sp_seg_len, 0, "fat1")
            .map_err(llvm_err)?;
        let sp_fat = self
            .builder
            .build_insert_value(sp_fat, sp_sbuf, 1, "fat2")
            .map_err(llvm_err)?;
        // Push via action_list_push
        let sp_cur_list = self
            .builder
            .build_load(self.list_type, sp_list_ptr, "cur_list")
            .map_err(llvm_err)?
            .into_struct_value();
        let sp_pushed = self.call_rt(
            "action_list_push",
            &[sp_cur_list.into(), sp_fat.into_struct_value().into()],
        )?;
        self.builder
            .build_store(sp_list_ptr, sp_pushed.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        // Update last = i + dlen, i = i + dlen
        let sp_nlast = self
            .builder
            .build_int_add(sp_iv, sp_dlen, "nlast")
            .map_err(llvm_err)?;
        self.builder.build_store(sp_i, sp_nlast).map_err(llvm_err)?;
        self.builder
            .build_store(sp_last, sp_nlast)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sp_fill_hdr);

        // fill_next: i += 1
        self.builder.position_at_end(sp_fill_next);
        let sp_ni = self
            .builder
            .build_int_add(sp_iv, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(sp_i, sp_ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sp_fill_hdr);

        // fill_last: push remaining segment from last to slen
        self.builder.position_at_end(sp_fill_last);
        let sp_last_v2 = self
            .builder
            .build_load(i64, sp_last, "last_v2")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_seg_len2 = self
            .builder
            .build_int_sub(sp_slen, sp_last_v2, "seg_len2")
            .map_err(llvm_err)?;
        let sp_salc2 = self
            .builder
            .build_int_add(sp_seg_len2, one, "salc2")
            .map_err(llvm_err)?;
        let sp_sbuf2 = self
            .builder
            .build_call(malloc_rc_fn, &[sp_salc2.into()], "sbuf2")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for this newly allocated buffer (malloc_rc starts at 0)
        let sp_sbuf2_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(sp_sbuf2, i64, "sp_sbuf2_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "sp_sbuf2_rc_addr",
            )
            .map_err(llvm_err)?;
        let sp_sbuf2_rc_p = self
            .builder
            .build_int_to_ptr(sp_sbuf2_rc_addr, ptr, "sp_sbuf2_rc_p")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_sbuf2_rc_p, i64.const_int(1, false))
            .map_err(llvm_err)?;
        let sp_ssrc2 = unsafe {
            self.builder
                .build_gep(i8, sp_sdata, &[sp_last_v2], "ssrc2")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sp_sbuf2.into(), sp_ssrc2.into(), sp_seg_len2.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sp_snull2 = unsafe {
            self.builder
                .build_gep(i8, sp_sbuf2, &[sp_seg_len2], "snull2")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(sp_snull2, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let sp_fat_undef2 = str_ty.get_undef();
        let sp_fat2 = self
            .builder
            .build_insert_value(sp_fat_undef2, sp_seg_len2, 0, "fat1b")
            .map_err(llvm_err)?;
        let sp_fat2 = self
            .builder
            .build_insert_value(sp_fat2, sp_sbuf2, 1, "fat2b")
            .map_err(llvm_err)?;
        let sp_cur_list2 = self
            .builder
            .build_load(self.list_type, sp_list_ptr, "cur_list2")
            .map_err(llvm_err)?
            .into_struct_value();
        let sp_pushed2 = self.call_rt(
            "action_list_push",
            &[sp_cur_list2.into(), sp_fat2.into_struct_value().into()],
        )?;
        self.builder
            .build_store(sp_list_ptr, sp_pushed2.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sp_fill_done);

        // fill_done: return list
        self.builder.position_at_end(sp_fill_done);
        let sp_result = self
            .builder
            .build_load(self.list_type, sp_list_ptr, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sp_result));

        Ok(())
    }
}
