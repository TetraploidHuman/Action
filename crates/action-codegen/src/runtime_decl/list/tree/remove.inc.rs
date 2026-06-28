// ---- action_list_remove({ptr, i64, i64}, i64 index) -> {ptr, i64, i64} ----
        let lrm_fn = self.module.add_function(
            "action_list_remove",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into()], false),
            None,
        );
        let lrm_entry = self.context.append_basic_block(lrm_fn, "entry");
        let lrm_concat = self.context.append_basic_block(lrm_fn, "concat");
        let lrm_concat_rm_left = self.context.append_basic_block(lrm_fn, "concat_rm_left");
        let lrm_concat_rm_right = self.context.append_basic_block(lrm_fn, "concat_rm_right");
        let lrm_normal = self.context.append_basic_block(lrm_fn, "normal");
        let lrm_h0 = self.context.append_basic_block(lrm_fn, "h0");
        let lrm_h0_cow = self.context.append_basic_block(lrm_fn, "h0_cow");
        let lrm_h0_cow_copy = self.context.append_basic_block(lrm_fn, "h0_cow_copy");
        let lrm_h0_ready = self.context.append_basic_block(lrm_fn, "h0_ready");
        let lrm_h0_shift_loop = self.context.append_basic_block(lrm_fn, "h0_shift_loop");
        let lrm_h0_shift_body = self.context.append_basic_block(lrm_fn, "h0_shift_body");
        let lrm_h0_done = self.context.append_basic_block(lrm_fn, "h0_done");
        let lrm_hgt0 = self.context.append_basic_block(lrm_fn, "hgt0");
        let lrm_empty_bb = self.context.append_basic_block(lrm_fn, "empty");
        let lrm_drop0_entry = self.context.append_basic_block(lrm_fn, "drop0_entry");
        let lrm_after_drop0 = self.context.append_basic_block(lrm_fn, "after_drop0");
        self.builder.position_at_end(lrm_entry);
        let lrm_list = lrm_fn.get_first_param().unwrap().into_struct_value();
        let lrm_index = lrm_fn.get_nth_param(1).unwrap().into_int_value();
        let lrm_node = self
            .builder
            .build_extract_value(lrm_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lrm_total_len = self
            .builder
            .build_extract_value(lrm_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_height = self
            .builder
            .build_extract_value(lrm_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_z = i64.const_int(0, false);
        let lrm_one = i64.const_int(1, false);
        // remove(0) on any height (incl. ConcatNode): drop(1) via range walk
        let lrm_idx_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrm_index, lrm_z, "idx_neg")
            .map_err(llvm_err)?;
        let lrm_idx_clamp = self
            .builder
            .build_select(lrm_idx_neg, lrm_z, lrm_index, "idx_clamp")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_last = self
            .builder
            .build_int_sub(lrm_total_len, lrm_one, "last")
            .map_err(llvm_err)?;
        let lrm_idx_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, lrm_idx_clamp, lrm_last, "idx_gt")
            .map_err(llvm_err)?;
        let lrm_idx_final = self
            .builder
            .build_select(lrm_idx_gt, lrm_last, lrm_idx_clamp, "idx_final")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_is_idx0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, lrm_idx_final, lrm_z, "is_idx0")
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lrm_is_idx0, lrm_drop0_entry, lrm_after_drop0);
        self.builder.position_at_end(lrm_drop0_entry);
        let lrm_drop_fn_e = self.module.get_function("action_list_drop").unwrap();
        let lrm_drop0_e = self
            .builder
            .build_call(lrm_drop_fn_e, &[lrm_list.into(), lrm_one.into()], "drop0")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lrm_drop0_e));
        self.builder.position_at_end(lrm_after_drop0);
        let lrm_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lrm_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_is_concat, lrm_concat, lrm_normal);
        // ConcatNode: lazy dispatch — remove in left/right subtree, rebuild via concat
        self.builder.position_at_end(lrm_concat);
        let lrm_cn_ln_p = unsafe {
            self.builder
                .build_gep(ptr, lrm_node, &[i64.const_int(2, false)], "cn_ln_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_left_node = self
            .builder
            .build_load(ptr, lrm_cn_ln_p, "cn_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lrm_cn_ll_p = unsafe {
            self.builder
                .build_gep(i64, lrm_node, &[i64.const_int(3, false)], "cn_ll_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_left_len = self
            .builder
            .build_load(i64, lrm_cn_ll_p, "cn_ll")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_cn_lh_p = unsafe {
            self.builder
                .build_gep(i64, lrm_node, &[i64.const_int(4, false)], "cn_lh_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_left_h = self
            .builder
            .build_load(i64, lrm_cn_lh_p, "cn_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_cn_l_undef = self.list_type.get_undef();
        let lrm_cn_l1 = self
            .builder
            .build_insert_value(lrm_cn_l_undef, lrm_cn_left_node, 0, "cn_l1")
            .map_err(llvm_err)?;
        let lrm_cn_l2 = self
            .builder
            .build_insert_value(lrm_cn_l1, lrm_cn_left_len, 1, "cn_l2")
            .map_err(llvm_err)?;
        let lrm_cn_left = self
            .builder
            .build_insert_value(lrm_cn_l2, lrm_cn_left_h, 2, "cn_left")
            .map_err(llvm_err)?
            .into_struct_value();
        let lrm_cn_rn_p = unsafe {
            self.builder
                .build_gep(ptr, lrm_node, &[i64.const_int(5, false)], "cn_rn_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_right_node = self
            .builder
            .build_load(ptr, lrm_cn_rn_p, "cn_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lrm_cn_rl_p = unsafe {
            self.builder
                .build_gep(i64, lrm_node, &[i64.const_int(6, false)], "cn_rl_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_right_len = self
            .builder
            .build_load(i64, lrm_cn_rl_p, "cn_rl")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_cn_rh_p = unsafe {
            self.builder
                .build_gep(i64, lrm_node, &[i64.const_int(7, false)], "cn_rh_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_right_h = self
            .builder
            .build_load(i64, lrm_cn_rh_p, "cn_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_cn_r_undef = self.list_type.get_undef();
        let lrm_cn_r1 = self
            .builder
            .build_insert_value(lrm_cn_r_undef, lrm_cn_right_node, 0, "cn_r1")
            .map_err(llvm_err)?;
        let lrm_cn_r2 = self
            .builder
            .build_insert_value(lrm_cn_r1, lrm_cn_right_len, 1, "cn_r2")
            .map_err(llvm_err)?;
        let lrm_cn_right = self
            .builder
            .build_insert_value(lrm_cn_r2, lrm_cn_right_h, 2, "cn_right")
            .map_err(llvm_err)?
            .into_struct_value();
        let lrm_cn_lt = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrm_idx_final, lrm_cn_left_len, "cn_lt")
            .map_err(llvm_err)?;
        let lrm_cn_chk_right = self.context.append_basic_block(lrm_fn, "concat_chk_right");
        let _ =
            self.builder
                .build_conditional_branch(lrm_cn_lt, lrm_concat_rm_left, lrm_cn_chk_right);
        self.builder.position_at_end(lrm_cn_chk_right);
        let _ = self.builder.build_unconditional_branch(lrm_concat_rm_right);

        let lrm_concat_fn = self.module.get_function("action_list_concat").unwrap();

        self.builder.position_at_end(lrm_concat_rm_left);
        let lrm_cn_new_left = self
            .builder
            .build_call(lrm_fn, &[lrm_cn_left.into(), lrm_idx_final.into()], "cn_nl")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lrm_cn_rl_res = self
            .builder
            .build_call(
                lrm_concat_fn,
                &[lrm_cn_new_left.into(), lrm_cn_right.into()],
                "cn_rl_res",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lrm_cn_rl_res));

        self.builder.position_at_end(lrm_concat_rm_right);
        let lrm_cn_new_idx = self
            .builder
            .build_int_sub(lrm_idx_final, lrm_cn_left_len, "cn_ni")
            .map_err(llvm_err)?;
        let lrm_cn_new_right = self
            .builder
            .build_call(
                lrm_fn,
                &[lrm_cn_right.into(), lrm_cn_new_idx.into()],
                "cn_nr",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lrm_cn_rr_res = self
            .builder
            .build_call(
                lrm_concat_fn,
                &[lrm_cn_left.into(), lrm_cn_new_right.into()],
                "cn_rr_res",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lrm_cn_rr_res));
        // Normal path: check h=0 vs h>0
        self.builder.position_at_end(lrm_normal);
        let zr = i64.const_int(0, false);
        let oner = i64.const_int(1, false);
        let lrm_is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, lrm_height, zr, "is_h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_is_h0, lrm_h0, lrm_hgt0);
        // === h=0: direct leaf manipulation ===
        self.builder.position_at_end(lrm_h0);
        let lrm_leaf_i8 = self
            .builder
            .build_pointer_cast(lrm_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let lrm_count_raw = self
            .builder
            .build_load(i32, lrm_leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_count = self
            .builder
            .build_int_z_extend(lrm_count_raw, i64, "count")
            .map_err(llvm_err)?;
        // If count==0 return unchanged
        let lrm_count_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, lrm_count, zr, "count_zero")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_count_zero, lrm_empty_bb, lrm_h0_cow);
        // CoW check
        self.builder.position_at_end(lrm_h0_cow);
        let lrm_last = self
            .builder
            .build_int_sub(lrm_count, oner, "last")
            .map_err(llvm_err)?;
        let lrm_idx_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrm_index, zr, "idx_neg")
            .map_err(llvm_err)?;
        let lrm_idx1 = self
            .builder
            .build_select(lrm_idx_neg, zr, lrm_index, "idx1")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_idx_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, lrm_idx1, lrm_last, "idx_gt")
            .map_err(llvm_err)?;
        let lrm_idx = self
            .builder
            .build_select(lrm_idx_gt, lrm_last, lrm_idx1, "idx")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_node_int = self
            .builder
            .build_ptr_to_int(lrm_node, i64, "node_int")
            .map_err(llvm_err)?;
        let lrm_rc_addr = self
            .builder
            .build_int_sub(lrm_node_int, i64.const_int(8, false), "rc_addr")
            .map_err(llvm_err)?;
        let lrm_rc_ptr = self
            .builder
            .build_int_to_ptr(lrm_rc_addr, ptr, "rc_ptr")
            .map_err(llvm_err)?;
        let lrm_rc_val = self
            .builder
            .build_load(i64, lrm_rc_ptr, "rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_need_cow = self
            .builder
            .build_int_compare(IntPredicate::SGT, lrm_rc_val, oner, "need_cow")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_need_cow, lrm_h0_cow_copy, lrm_h0_ready);
        self.builder.position_at_end(lrm_h0_cow_copy);
        let leaf_ty = self.leaf_type;
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let lrm_cow_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "cow_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let lrm_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                lrm_memcpy_fn,
                &[lrm_cow_leaf.into(), lrm_node.into(), leaf_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lrm_h0_ready);
        self.builder.position_at_end(lrm_h0_ready);
        let lrm_leaf_phi = self.builder.build_phi(ptr, "leaf_phi").map_err(llvm_err)?;
        lrm_leaf_phi.add_incoming(&[(&lrm_node, lrm_h0_cow), (&lrm_cow_leaf, lrm_h0_cow_copy)]);
        let lrm_leaf = lrm_leaf_phi.as_basic_value().into_pointer_value();
        // RC-dec the removed element's data_ptr
        let lrm_leaf2_i8 = self
            .builder
            .build_pointer_cast(lrm_leaf, ptr, "leaf2_i8")
            .map_err(llvm_err)?;
        let lrm_eb = unsafe {
            self.builder
                .build_gep(i8, lrm_leaf2_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)
        }?;
        let lrm_rm_ep = unsafe {
            self.builder
                .build_gep(self.string_type, lrm_eb, &[lrm_idx], "rm_ep")
                .map_err(llvm_err)
        }?;
        let lrm_rm_ev = self
            .builder
            .build_load(self.string_type, lrm_rm_ep, "rm_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let lrm_str_rc_dec_fn = self.module.get_function("action_string_rc_dec").unwrap();
        let _ = self
            .builder
            .build_call(lrm_str_rc_dec_fn, &[lrm_rm_ev.into()], "")
            .map_err(llvm_err)?;
        // Shift elements [idx+1..count-1] left by 1
        let lrm_si_val = self
            .builder
            .build_int_add(lrm_idx, oner, "si_start")
            .map_err(llvm_err)?;
        let lrm_si = self.builder.build_alloca(i64, "si").map_err(llvm_err)?;
        self.builder
            .build_store(lrm_si, lrm_si_val)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lrm_h0_shift_loop);
        self.builder.position_at_end(lrm_h0_shift_loop);
        let lrm_siv = self
            .builder
            .build_load(i64, lrm_si, "siv")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_si_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrm_siv, lrm_count, "si_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_si_cond, lrm_h0_shift_body, lrm_h0_done);
        self.builder.position_at_end(lrm_h0_shift_body);
        let lrm_src = unsafe {
            self.builder
                .build_gep(self.string_type, lrm_eb, &[lrm_siv], "src")
                .map_err(llvm_err)
        }?;
        let lrm_sv = self
            .builder
            .build_load(self.string_type, lrm_src, "sv")
            .map_err(llvm_err)?;
        let lrm_siv_minus1 = self
            .builder
            .build_int_sub(lrm_siv, oner, "siv_m1")
            .map_err(llvm_err)?;
        let lrm_dst = unsafe {
            self.builder
                .build_gep(self.string_type, lrm_eb, &[lrm_siv_minus1], "dst")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(lrm_dst, lrm_sv)
            .map_err(llvm_err)?;
        let lrm_siv_plus1 = self
            .builder
            .build_int_add(lrm_siv, oner, "siv_p1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lrm_si, lrm_siv_plus1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lrm_h0_shift_loop);
        // Decrement count and return
        self.builder.position_at_end(lrm_h0_done);
        let lrm_new_count = self
            .builder
            .build_int_sub(lrm_count, oner, "new_count")
            .map_err(llvm_err)?;
        let lrm_new_count_i32 = self
            .builder
            .build_int_truncate(lrm_new_count, i32, "new_count_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lrm_leaf2_i8, lrm_new_count_i32)
            .map_err(llvm_err)?;
        let lrm_new_total = self
            .builder
            .build_int_sub(lrm_total_len, oner, "new_total")
            .map_err(llvm_err)?;
        let undef_rem = self.list_type.get_undef();
        let lrm_r1 = self
            .builder
            .build_insert_value(undef_rem, lrm_leaf, 0, "r1")
            .map_err(llvm_err)?;
        let lrm_r2 = self
            .builder
            .build_insert_value(lrm_r1, lrm_new_total, 1, "r2")
            .map_err(llvm_err)?;
        let lrm_r3 = self
            .builder
            .build_insert_value(lrm_r2, zr, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lrm_r3));
        // Empty: return original list unchanged
        self.builder.position_at_end(lrm_empty_bb);
        let _ = self.builder.build_return(Some(&lrm_list));
        // === h>0: take+drop+concat ===
        self.builder.position_at_end(lrm_hgt0);
        let lrm_len2 = self
            .builder
            .build_extract_value(lrm_list, 1, "lrm_len2")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_len_zero2 = self
            .builder
            .build_int_compare(IntPredicate::EQ, lrm_len2, zr, "len_zero2")
            .map_err(llvm_err)?;
        let lrm_hgt0_empty = self.context.append_basic_block(lrm_fn, "hgt0_empty");
        let lrm_hgt0_body = self.context.append_basic_block(lrm_fn, "hgt0_body");
        let _ = self
            .builder
            .build_conditional_branch(lrm_len_zero2, lrm_hgt0_empty, lrm_hgt0_body);
        self.builder.position_at_end(lrm_hgt0_empty);
        let _ = self.builder.build_return(Some(&lrm_list));
        self.builder.position_at_end(lrm_hgt0_body);
        let lrm_last2 = self
            .builder
            .build_int_sub(lrm_len2, oner, "last2")
            .map_err(llvm_err)?;
        let lrm_idx2_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrm_index, zr, "idx2_neg")
            .map_err(llvm_err)?;
        let lrm_idx2a = self
            .builder
            .build_select(lrm_idx2_neg, zr, lrm_index, "idx2a")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_idx2_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, lrm_idx2a, lrm_last2, "idx2_gt")
            .map_err(llvm_err)?;
        let lrm_idx2 = self
            .builder
            .build_select(lrm_idx2_gt, lrm_last2, lrm_idx2a, "idx2")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_is_idx0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, lrm_idx2, zr, "is_idx0")
            .map_err(llvm_err)?;
        let lrm_drop0_bb = self.context.append_basic_block(lrm_fn, "hgt0_drop0");
        let lrm_tdc_bb = self.context.append_basic_block(lrm_fn, "hgt0_tdc");
        let _ = self
            .builder
            .build_conditional_branch(lrm_is_idx0, lrm_drop0_bb, lrm_tdc_bb);
        self.builder.position_at_end(lrm_drop0_bb);
        let lrm_drop_fn0 = self.module.get_function("action_list_drop").unwrap();
        let lrm_drop0_rv = self
            .builder
            .build_call(lrm_drop_fn0, &[lrm_list.into(), oner.into()], "drop0")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lrm_drop0_rv));
        self.builder.position_at_end(lrm_tdc_bb);
        let lrm_remove_rec_fn = self.module.get_function("action_list_remove_rec").unwrap();
        let lrm_root_rc_p = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(
                                self.builder
                                    .build_extract_value(lrm_list, 0, "rm_node")
                                    .map_err(llvm_err)?
                                    .into_pointer_value(),
                                i64,
                                "rm_root_pi",
                            )
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "rm_root_ra",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "rm_root_rp",
            )
            .map_err(llvm_err)?;
        let lrm_root_rc = self
            .builder
            .build_load(i64, lrm_root_rc_p, "rm_root_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_rec_height_out = self
            .builder
            .build_alloca(i64, "rm_rec_height_out")
            .map_err(llvm_err)?;
        let lrm_rec_node = self
            .builder
            .build_extract_value(lrm_list, 0, "rm_rec_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lrm_rec_h = self
            .builder
            .build_extract_value(lrm_list, 2, "rm_rec_h")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_rec_root = self
            .builder
            .build_call(
                lrm_remove_rec_fn,
                &[
                    lrm_rec_node.into(),
                    lrm_rec_h.into(),
                    lrm_idx2.into(),
                    lrm_root_rc.into(),
                    lrm_rec_height_out.into(),
                ],
                "rec_rm_root",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let lrm_rec_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lrm_rec_root,
                ptr.const_null(),
                "rm_rec_null",
            )
            .map_err(llvm_err)?;
        let lrm_rec_ok_bb = self.context.append_basic_block(lrm_fn, "rm_rec_ok");
        let lrm_rec_fallback_bb = self.context.append_basic_block(lrm_fn, "rm_rec_fallback");
        let _ =
            self.builder
                .build_conditional_branch(lrm_rec_null, lrm_rec_fallback_bb, lrm_rec_ok_bb);
        self.builder.position_at_end(lrm_rec_ok_bb);
        let lrm_rec_new_len = self
            .builder
            .build_int_sub(lrm_len2, oner, "rm_rec_new_len")
            .map_err(llvm_err)?;
        let lrm_rec_height = self
            .builder
            .build_load(i64, lrm_rec_height_out, "rm_rec_height")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_rec_r1 = self
            .builder
            .build_insert_value(self.list_type.get_undef(), lrm_rec_root, 0, "rm_r1")
            .map_err(llvm_err)?;
        let lrm_rec_r2 = self
            .builder
            .build_insert_value(lrm_rec_r1, lrm_rec_new_len, 1, "rm_r2")
            .map_err(llvm_err)?;
        let lrm_rec_r3 = self
            .builder
            .build_insert_value(lrm_rec_r2, lrm_rec_height, 2, "rm_r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lrm_rec_r3));
        self.builder.position_at_end(lrm_rec_fallback_bb);
        let lrm_take_fn = self.module.get_function("action_list_take").unwrap();
        let lrm_drop_fn = self.module.get_function("action_list_drop").unwrap();
        let lrm_concat_fn = self.module.get_function("action_list_concat").unwrap();
        let lrm_left = self
            .builder
            .build_call(lrm_take_fn, &[lrm_list.into(), lrm_idx2.into()], "left")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lrm_idx2p1 = self
            .builder
            .build_int_add(lrm_idx2, oner, "idx2p1")
            .map_err(llvm_err)?;
        let lrm_right = self
            .builder
            .build_call(lrm_drop_fn, &[lrm_list.into(), lrm_idx2p1.into()], "right")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lrm_result = self
            .builder
            .build_call(
                lrm_concat_fn,
                &[lrm_left.into(), lrm_right.into()],
                "result",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lrm_result));

