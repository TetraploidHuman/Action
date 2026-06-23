// ---- action_list_slice({ptr, i64, i64}, i64 start, i64 end) -> {ptr, i64, i64} ----
        let slc_fn = self.module.add_function(
            "action_list_slice",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into(), i64.into()], false),
            None,
        );
        let slc_entry = self.context.append_basic_block(slc_fn, "entry");
        let slc_concat = self.context.append_basic_block(slc_fn, "concat");
        let slc_normal = self.context.append_basic_block(slc_fn, "normal");
        let slc_h0 = self.context.append_basic_block(slc_fn, "h0");
        let slc_h0_ci_loop = self.context.append_basic_block(slc_fn, "h0_ci_loop");
        let slc_h0_ci_body = self.context.append_basic_block(slc_fn, "h0_ci_body");
        let slc_h0_done = self.context.append_basic_block(slc_fn, "h0_done");
        let slc_hgt0 = self.context.append_basic_block(slc_fn, "hgt0");
        self.builder.position_at_end(slc_entry);
        let slc_list = slc_fn.get_first_param().unwrap().into_struct_value();
        let slc_start = slc_fn.get_nth_param(1).unwrap().into_int_value();
        let slc_end = slc_fn.get_nth_param(2).unwrap().into_int_value();
        let slc_node = self
            .builder
            .build_extract_value(slc_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let slc_len = self
            .builder
            .build_extract_value(slc_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_height = self
            .builder
            .build_extract_value(slc_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                slc_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(slc_is_concat, slc_concat, slc_normal);
        // ConcatNode: drop(start) + take(count) via range walk (no flatten)
        self.builder.position_at_end(slc_concat);
        let slc_c_s_neg = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                slc_start,
                i64.const_int(0, false),
                "csneg",
            )
            .map_err(llvm_err)?;
        let slc_c_s_clamp = self
            .builder
            .build_select(slc_c_s_neg, i64.const_int(0, false), slc_start, "csclamp")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_c_s_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_c_s_clamp, slc_len, "csgt")
            .map_err(llvm_err)?;
        let slc_c_s_final = self
            .builder
            .build_select(slc_c_s_gt, slc_len, slc_c_s_clamp, "csfinal")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_c_e_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_end, i64.const_int(0, false), "ceneg")
            .map_err(llvm_err)?;
        let slc_c_e_clamp = self
            .builder
            .build_select(slc_c_e_neg, i64.const_int(0, false), slc_end, "ceclamp")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_c_e_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_c_e_clamp, slc_len, "cegt")
            .map_err(llvm_err)?;
        let slc_c_e_final = self
            .builder
            .build_select(slc_c_e_gt, slc_len, slc_c_e_clamp, "cefinal")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_c_rlen = self
            .builder
            .build_int_sub(slc_c_e_final, slc_c_s_final, "crlen")
            .map_err(llvm_err)?;
        let slc_c_rlen_neg = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                slc_c_rlen,
                i64.const_int(0, false),
                "crlen_neg",
            )
            .map_err(llvm_err)?;
        let slc_c_rlen_final = self
            .builder
            .build_select(
                slc_c_rlen_neg,
                i64.const_int(0, false),
                slc_c_rlen,
                "crlenf",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let slc_drop_fn = self.module.get_function("action_list_drop").unwrap();
        let slc_take_fn = self.module.get_function("action_list_take").unwrap();
        let slc_drop_r = self
            .builder
            .build_call(
                slc_drop_fn,
                &[slc_list.into(), slc_c_s_final.into()],
                "drop_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let slc_take_r = self
            .builder
            .build_call(
                slc_take_fn,
                &[slc_drop_r.into(), slc_c_rlen_final.into()],
                "take_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&slc_take_r));
        // Normal path: check h=0 vs h>0
        self.builder.position_at_end(slc_normal);
        let slc_is_h0 = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                slc_height,
                i64.const_int(0, false),
                "is_h0",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(slc_is_h0, slc_h0, slc_hgt0);
        // === h=0: direct leaf manipulation ===
        self.builder.position_at_end(slc_h0);
        let slc_leaf_i8 = self
            .builder
            .build_pointer_cast(slc_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let slc_count_raw = self
            .builder
            .build_load(i32, slc_leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_count = self
            .builder
            .build_int_z_extend(slc_count_raw, i64, "count")
            .map_err(llvm_err)?;
        let z = i64.const_int(0, false);
        // Clamp start to [0, count]
        let slc_s_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_start, z, "s_neg")
            .map_err(llvm_err)?;
        let slc_s_clamp = self
            .builder
            .build_select(slc_s_neg, z, slc_start, "s_clamp")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_s_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_s_clamp, slc_count, "s_gt")
            .map_err(llvm_err)?;
        let slc_s_final = self
            .builder
            .build_select(slc_s_gt, slc_count, slc_s_clamp, "s_final")
            .map_err(llvm_err)?
            .into_int_value();
        // Clamp end to [0, count]
        let slc_e_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_end, z, "e_neg")
            .map_err(llvm_err)?;
        let slc_e_clamp = self
            .builder
            .build_select(slc_e_neg, z, slc_end, "e_clamp")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_e_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_e_clamp, slc_count, "e_gt")
            .map_err(llvm_err)?;
        let slc_e_final = self
            .builder
            .build_select(slc_e_gt, slc_count, slc_e_clamp, "e_final")
            .map_err(llvm_err)?
            .into_int_value();
        // Compute result length
        let slc_rlen = self
            .builder
            .build_int_sub(slc_e_final, slc_s_final, "rlen")
            .map_err(llvm_err)?;
        let slc_rlen_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_rlen, z, "rlen_neg")
            .map_err(llvm_err)?;
        let slc_new_count = self
            .builder
            .build_select(slc_rlen_neg, z, slc_rlen, "new_count")
            .map_err(llvm_err)?
            .into_int_value();
        // Allocate new leaf
        let leaf_ty = self.leaf_type;
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let slc_new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "new_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Copy elements[start..end] from old leaf to new_leaf[0..new_count]
        let slc_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let slc_old_eb = unsafe {
            self.builder
                .build_gep(i8, slc_leaf_i8, &[i64.const_int(8, false)], "old_eb")
                .map_err(llvm_err)
        }?;
        let slc_src = unsafe {
            self.builder
                .build_gep(self.string_type, slc_old_eb, &[slc_s_final], "src")
                .map_err(llvm_err)
        }?;
        let slc_new_i8 = self
            .builder
            .build_pointer_cast(slc_new_leaf, ptr, "new_i8")
            .map_err(llvm_err)?;
        let slc_new_eb = unsafe {
            self.builder
                .build_gep(i8, slc_new_i8, &[i64.const_int(8, false)], "new_eb")
                .map_err(llvm_err)
        }?;
        let slc_dst = unsafe {
            self.builder
                .build_gep(self.string_type, slc_new_eb, &[z], "dst")
                .map_err(llvm_err)
        }?;
        let slc_copy_bytes = self
            .builder
            .build_int_mul(slc_new_count, i64.const_int(16, false), "copy_bytes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                slc_memcpy_fn,
                &[slc_dst.into(), slc_src.into(), slc_copy_bytes.into()],
                "",
            )
            .map_err(llvm_err)?;
        // RC-inc each element in the new leaf
        let slc_ci_i = self.builder.build_alloca(i64, "ci_i").map_err(llvm_err)?;
        self.builder.build_store(slc_ci_i, z).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(slc_h0_ci_loop);
        self.builder.position_at_end(slc_h0_ci_loop);
        let slc_ci = self
            .builder
            .build_load(i64, slc_ci_i, "ci")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_ci_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_ci, slc_new_count, "ci_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(slc_ci_cond, slc_h0_ci_body, slc_h0_done);
        self.builder.position_at_end(slc_h0_ci_body);
        let slc_str_rc_inc_fn = self.module.get_function("action_string_rc_inc").unwrap();
        let slc_ci_ep = unsafe {
            self.builder
                .build_gep(self.string_type, slc_new_eb, &[slc_ci], "ci_ep")
                .map_err(llvm_err)
        }?;
        let slc_ci_ev = self
            .builder
            .build_load(self.string_type, slc_ci_ep, "ci_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let _ = self
            .builder
            .build_call(slc_str_rc_inc_fn, &[slc_ci_ev.into()], "")
            .map_err(llvm_err)?;
        let slc_ci_next = self
            .builder
            .build_int_add(slc_ci, i64.const_int(1, false), "ci_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slc_ci_i, slc_ci_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(slc_h0_ci_loop);
        // Set count on new leaf and return
        self.builder.position_at_end(slc_h0_done);
        let slc_new_count_i32 = self
            .builder
            .build_int_truncate(slc_new_count, i32, "new_count_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(slc_new_i8, slc_new_count_i32)
            .map_err(llvm_err)?;
        let undef_slc = self.list_type.get_undef();
        let slc_r1 = self
            .builder
            .build_insert_value(undef_slc, slc_new_leaf, 0, "r1")
            .map_err(llvm_err)?;
        let slc_r2 = self
            .builder
            .build_insert_value(slc_r1, slc_new_count, 1, "r2")
            .map_err(llvm_err)?;
        let slc_r3 = self
            .builder
            .build_insert_value(slc_r2, i64.const_int(0, false), 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&slc_r3));
        // === h>0: per-element loop ===
        self.builder.position_at_end(slc_hgt0);
        let slc_s_neg2 = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                slc_start,
                i64.const_int(0, false),
                "sneg2",
            )
            .map_err(llvm_err)?;
        let slc_s_clamp2 = self
            .builder
            .build_select(slc_s_neg2, i64.const_int(0, false), slc_start, "sclamp2")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_s_gt2 = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_s_clamp2, slc_len, "sgt2")
            .map_err(llvm_err)?;
        let slc_s_final2 = self
            .builder
            .build_select(slc_s_gt2, slc_len, slc_s_clamp2, "sfinal2")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_e_neg2 = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_end, i64.const_int(0, false), "eneg2")
            .map_err(llvm_err)?;
        let slc_e_clamp2 = self
            .builder
            .build_select(slc_e_neg2, i64.const_int(0, false), slc_end, "eclamp2")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_e_gt2 = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_e_clamp2, slc_len, "egt2")
            .map_err(llvm_err)?;
        let slc_e_final2 = self
            .builder
            .build_select(slc_e_gt2, slc_len, slc_e_clamp2, "efinal2")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_rlen2 = self
            .builder
            .build_int_sub(slc_e_final2, slc_s_final2, "rlen2")
            .map_err(llvm_err)?;
        let slc_rlen_neg2 = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                slc_rlen2,
                i64.const_int(0, false),
                "rneg2",
            )
            .map_err(llvm_err)?;
        let slc_rlen_final2 = self
            .builder
            .build_select(slc_rlen_neg2, i64.const_int(0, false), slc_rlen2, "rlenf2")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_drop_fn2 = self.module.get_function("action_list_drop").unwrap();
        let slc_take_fn2 = self.module.get_function("action_list_take").unwrap();
        let slc_drop_r2 = self
            .builder
            .build_call(
                slc_drop_fn2,
                &[slc_list.into(), slc_s_final2.into()],
                "drop_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let slc_take_r2 = self
            .builder
            .build_call(
                slc_take_fn2,
                &[slc_drop_r2.into(), slc_rlen_final2.into()],
                "take_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&slc_take_r2));
