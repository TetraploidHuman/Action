// String runtime (moved from list/tree/remove.inc.rs — R7)

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_split_lines(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i8 = self.context.i8_type();
        let str_ty = self.string_type;
        let ptr = self.ptr_ty();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        // ---- action_string_split_lines({i64, ptr}) -> {ptr, i64, i64} ----
        let sl_fn = self.module.add_function(
            "action_string_split_lines",
            self.list_type.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(sl_fn, "entry");
        self.builder.position_at_end(entry);
        let sl_s = sl_fn.get_first_param().unwrap().into_struct_value();
        let sl_len = self
            .builder
            .build_extract_value(sl_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_ptr = self
            .builder
            .build_extract_value(sl_s, 1, "sptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc4 = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let sl_list_init = cc4.try_as_basic_value().unwrap_basic().into_struct_value();
        // Use alloca to accumulate list across loop iterations
        let sl_list_alloc = self
            .builder
            .build_alloca(self.list_type, "list_acc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_list_alloc, sl_list_init)
            .map_err(llvm_err)?;
        // Scan through string, splitting on '\n'
        let sl_start_alloc = self.builder.build_alloca(i64, "start").map_err(llvm_err)?;
        let sl_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(sl_start_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sl_loop = self.context.append_basic_block(sl_fn, "loop");
        let sl_body_bb = self.context.append_basic_block(sl_fn, "body");
        let sl_done = self.context.append_basic_block(sl_fn, "done");
        let _ = self.builder.build_unconditional_branch(sl_loop);
        self.builder.position_at_end(sl_loop);
        let sl_i = self
            .builder
            .build_load(i64, sl_i_alloc, "sl_i")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_cond = self
            .builder
            .build_int_compare(IntPredicate::SLE, sl_i, sl_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sl_cond, sl_body_bb, sl_done);
        self.builder.position_at_end(sl_body_bb);
        // Check if at end or char is '\n'
        let sl_at_end = self
            .builder
            .build_int_compare(IntPredicate::EQ, sl_i, sl_len, "atend")
            .map_err(llvm_err)?;
        let sl_cp = unsafe {
            self.builder
                .build_gep(i8, sl_ptr, &[sl_i], "cp")
                .map_err(llvm_err)
        }?;
        let sl_c = self
            .builder
            .build_load(i8, sl_cp, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_is_nl = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sl_c,
                i8.const_int(b'\n' as u64, false),
                "isnl",
            )
            .map_err(llvm_err)?;
        let sl_cr = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sl_c,
                i8.const_int(b'\r' as u64, false),
                "iscr",
            )
            .map_err(llvm_err)?;
        let sl_split = self
            .builder
            .build_or(
                sl_at_end,
                self.builder
                    .build_or(sl_is_nl, sl_cr, "")
                    .map_err(llvm_err)?,
                "split",
            )
            .map_err(llvm_err)?;
        let sl_cont = self.context.append_basic_block(sl_fn, "cont");
        let sl_extract = self.context.append_basic_block(sl_fn, "extract");
        let _ = self
            .builder
            .build_conditional_branch(sl_split, sl_extract, sl_cont);
        // Extract line from start to i
        self.builder.position_at_end(sl_extract);
        let sl_start = self
            .builder
            .build_load(i64, sl_start_alloc, "slstart")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_seg_len = self
            .builder
            .build_int_sub(sl_i, sl_start, "seg_len")
            .map_err(llvm_err)?;
        let sl_seg_data = unsafe {
            self.builder
                .build_gep(i8, sl_ptr, &[sl_start], "segp")
                .map_err(llvm_err)
        }?;
        // Skip \r if next char is \n
        let sl_next_i = self
            .builder
            .build_int_add(sl_i, i64.const_int(1, false), "nexti")
            .map_err(llvm_err)?;
        // Create string for this segment: malloc + memcpy
        let sl_salloc = self
            .builder
            .build_call(malloc_rc_fn, &[sl_seg_len.into()], "seg")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for newly allocated segment
        let sl_sa_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(sl_salloc, i64, "sl_sa_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "sl_sa_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(sl_sa_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sl_salloc.into(), sl_seg_data.into(), sl_seg_len.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sl_fat = self.string_type.get_undef();
        let sl_fat_tag = self
            .builder
            .build_insert_value(sl_fat, self.i64_ty().const_int(1, false), 0, "tag")
            .map_err(llvm_err)?;
        let sl_fat_val = self
            .builder
            .build_insert_value(sl_fat_tag, sl_salloc, 1, "data")
            .map_err(llvm_err)?;
        let sl_cur_list = self
            .builder
            .build_load(self.list_type, sl_list_alloc, "cur_list")
            .map_err(llvm_err)?
            .into_struct_value();
        let sl_push_cc = self.call_rt(
            "action_list_push",
            &[sl_cur_list.into(), sl_fat_val.into_struct_value().into()],
        )?;
        let sl_new_list = sl_push_cc.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(sl_list_alloc, sl_new_list)
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_start_alloc, sl_next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sl_cont);
        // Continue scanning
        self.builder.position_at_end(sl_cont);
        let sl_i2 = self
            .builder
            .build_load(i64, sl_i_alloc, "i2")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_i_next = self
            .builder
            .build_int_add(sl_i2, i64.const_int(1, false), "inext")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_i_alloc, sl_i_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sl_loop);
        self.builder.position_at_end(sl_done);
        let sl_result = self
            .builder
            .build_load(self.list_type, sl_list_alloc, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sl_result));

                Ok(())
    }
}
