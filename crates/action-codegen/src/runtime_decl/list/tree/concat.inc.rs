// ---- action_list_concat({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Block-based: walks source tree leaves, pushes elements in batches.
        // Special-cases height-0 + height-0 with total <= 64 for O(1) single-leaf merge.
        let concat_fn = self.module.get_function("action_list_concat").unwrap();
        let concat_entry = self.context.append_basic_block(concat_fn, "entry");
        self.builder.position_at_end(concat_entry);
        // Allocate result slot in entry (must dominate all paths)
        let _concat_ra = self
            .builder
            .build_alloca(self.list_type, "concat_ra")
            .map_err(llvm_err)?;
        let concat_a = concat_fn.get_first_param().unwrap().into_struct_value();
        let concat_b = concat_fn.get_nth_param(1).unwrap().into_struct_value();
        let a_len = self
            .builder
            .build_extract_value(concat_a, 1, "a_len")
            .map_err(llvm_err)?
            .into_int_value();
        let b_len = self
            .builder
            .build_extract_value(concat_b, 1, "b_len")
            .map_err(llvm_err)?
            .into_int_value();
        let a_height = self
            .builder
            .build_extract_value(concat_a, 2, "a_h")
            .map_err(llvm_err)?
            .into_int_value();
        let b_height = self
            .builder
            .build_extract_value(concat_b, 2, "b_h")
            .map_err(llvm_err)?
            .into_int_value();
        let a_node = self
            .builder
            .build_extract_value(concat_a, 0, "a_n")
            .map_err(llvm_err)?
            .into_pointer_value();
        let b_node = self
            .builder
            .build_extract_value(concat_b, 0, "b_n")
            .map_err(llvm_err)?
            .into_pointer_value();
        let total = self
            .builder
            .build_int_add(a_len, b_len, "total")
            .map_err(llvm_err)?;
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let b64 = i64.const_int(64, false);
        let elem_sz = i64.const_int(16, false); // string_type = {i64, ptr}
        let leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let _concat_push_fn = self.module.get_function("action_list_push").unwrap();
        let _concat_create_fn = self.module.get_function("action_list_create").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();

        // === Edge cases: empty list sharing ===
        let cc_empty_a = self.context.append_basic_block(concat_fn, "empty_a");
        let cc_empty_b = self.context.append_basic_block(concat_fn, "empty_b");
        let cc_share_ret = self.context.append_basic_block(concat_fn, "share_ret");
        let cc_small_merge = self.context.append_basic_block(concat_fn, "small_merge");
        let cc_lazy_concat = self.context.append_basic_block(concat_fn, "lazy_concat");

        let b_is_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, b_len, zero, "b_z")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(b_is_zero, cc_empty_b, cc_empty_a);

        // A is empty (B non-empty): share B
        self.builder.position_at_end(cc_empty_a);
        let a_is_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, a_len, zero, "a_z")
            .map_err(llvm_err)?;
        // Check special merge case: both height=0 && total <= 64
        let _ = self
            .builder
            .build_conditional_branch(a_is_zero, cc_share_ret, cc_small_merge);

        // B is empty: share A
        self.builder.position_at_end(cc_empty_b);
        let _ = self.builder.build_unconditional_branch(cc_share_ret);

        // share_ret: rc_inc the non-empty node and return it
        self.builder.position_at_end(cc_share_ret);
        // Phi for which list to return (A when B empty, B when A empty)
        let share_phi_list = self
            .builder
            .build_phi(self.list_type, "share_phi")
            .map_err(llvm_err)?;
        share_phi_list.add_incoming(&[(&concat_a, cc_empty_b)]);
        share_phi_list.add_incoming(&[(&concat_b, cc_empty_a)]);
        let share_list = share_phi_list.as_basic_value().into_struct_value();
        let share_node = self
            .builder
            .build_extract_value(share_list, 0, "share_n")
            .map_err(llvm_err)?
            .into_pointer_value();
        // rc_inc the shared node to account for the new reference
        let share_rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(share_rc_inc_fn, &[share_node.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&share_list));

        // === Special case: both height=0, total <= 64 → single leaf merge ===
        self.builder.position_at_end(cc_small_merge);
        let ah0_cond = self
            .builder
            .build_int_compare(IntPredicate::EQ, a_height, zero, "ah0")
            .map_err(llvm_err)?;
        let bh0_cond = self
            .builder
            .build_int_compare(IntPredicate::EQ, b_height, zero, "bh0")
            .map_err(llvm_err)?;
        let both_h0 = self
            .builder
            .build_and(ah0_cond, bh0_cond, "both_h0")
            .map_err(llvm_err)?;
        let total_small = self
            .builder
            .build_int_compare(IntPredicate::SLE, total, b64, "tsmall")
            .map_err(llvm_err)?;
        let can_merge = self
            .builder
            .build_and(both_h0, total_small, "can_merge")
            .map_err(llvm_err)?;
        let cc_do_merge = self.context.append_basic_block(concat_fn, "do_merge");
        let cc_sm_ci_loop = self.context.append_basic_block(concat_fn, "sm_ci_loop");
        let cc_sm_ci_body = self.context.append_basic_block(concat_fn, "sm_ci_body");
        let cc_sm_ci_done = self.context.append_basic_block(concat_fn, "sm_ci_done");
        let _ = self
            .builder
            .build_conditional_branch(can_merge, cc_do_merge, cc_lazy_concat);

        // Perform single-leaf merge
        self.builder.position_at_end(cc_do_merge);
        let new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "merged")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let a_leaf_i8 = self
            .builder
            .build_pointer_cast(a_node, ptr, "ali8")
            .map_err(llvm_err)?;
        let b_leaf_i8 = self
            .builder
            .build_pointer_cast(b_node, ptr, "bli8")
            .map_err(llvm_err)?;
        let nl_i8 = self
            .builder
            .build_pointer_cast(new_leaf, ptr, "nli8")
            .map_err(llvm_err)?;
        // Copy a's elements: dst = new_leaf+8, src = a_leaf+8, size = a_len*16
        let a_src = unsafe {
            self.builder
                .build_gep(i8, a_leaf_i8, &[i64.const_int(8, false)], "a_src")
                .map_err(llvm_err)
        }?;
        let nl_dst = unsafe {
            self.builder
                .build_gep(i8, nl_i8, &[i64.const_int(8, false)], "nl_dst")
                .map_err(llvm_err)
        }?;
        let a_bytes = self
            .builder
            .build_int_mul(a_len, elem_sz, "a_bytes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[nl_dst.into(), a_src.into(), a_bytes.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Copy b's elements: dst = new_leaf+8 + a_len*16, src = b_leaf+8, size = b_len*16
        let nl_dst2 = unsafe {
            self.builder
                .build_gep(self.string_type, nl_dst, &[a_len], "nl_dst2")
                .map_err(llvm_err)
        }?;
        let b_src = unsafe {
            self.builder
                .build_gep(i8, b_leaf_i8, &[i64.const_int(8, false)], "b_src")
                .map_err(llvm_err)
        }?;
        let b_bytes = self
            .builder
            .build_int_mul(b_len, elem_sz, "b_bytes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[nl_dst2.into(), b_src.into(), b_bytes.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Set leaf count (i32 at offset 0)
        let sm_count = self
            .builder
            .build_int_truncate(total, i32, "sm_count")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl_i8, sm_count)
            .map_err(llvm_err)?;
        // RC-inc each element copied into the merged leaf (shared string refs)
        let _ = self.builder.build_unconditional_branch(cc_sm_ci_loop);
        self.builder.position_at_end(cc_sm_ci_loop);
        let sm_ci_i = self.builder.build_phi(i64, "sm_ci_i").map_err(llvm_err)?;
        let sm_ci_cur = sm_ci_i.as_basic_value().into_int_value();
        let sm_ci_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, sm_ci_cur, total, "sm_ci_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sm_ci_done, cc_sm_ci_done, cc_sm_ci_body);
        self.builder.position_at_end(cc_sm_ci_body);
        let sm_str_rc_inc_fn = self.module.get_function("action_string_rc_inc").unwrap();
        let sm_ci_ep = unsafe {
            self.builder
                .build_gep(self.string_type, nl_dst, &[sm_ci_cur], "sm_ci_ep")
                .map_err(llvm_err)?
        };
        let sm_ci_ev = self
            .builder
            .build_load(self.string_type, sm_ci_ep, "sm_ci_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let _ = self
            .builder
            .build_call(sm_str_rc_inc_fn, &[sm_ci_ev.into()], "")
            .map_err(llvm_err)?;
        let sm_ci_next = self
            .builder
            .build_int_add(sm_ci_cur, one, "sm_ci_next")
            .map_err(llvm_err)?;
        let sm_ci_body_bb = self.builder.get_insert_block().unwrap();
        sm_ci_i.add_incoming(&[(&zero, cc_do_merge), (&sm_ci_next, sm_ci_body_bb)]);
        let _ = self.builder.build_unconditional_branch(cc_sm_ci_loop);

        self.builder.position_at_end(cc_sm_ci_done);
        // Return {new_leaf, total, 0}
        let sm_undef = self.list_type.get_undef();
        let sm_r1 = self
            .builder
            .build_insert_value(sm_undef, new_leaf, 0, "sm_r1")
            .map_err(llvm_err)?;
        let sm_r2 = self
            .builder
            .build_insert_value(sm_r1, total, 1, "sm_r2")
            .map_err(llvm_err)?;
        let sm_r3 = self
            .builder
            .build_insert_value(sm_r2, zero, 2, "sm_r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sm_r3)).map_err(llvm_err)?;

        // === Lazy concat: create ConcatNode instead of flattening immediately ===
        self.builder.position_at_end(cc_lazy_concat);
        // Compute ConcatNode depth: max(existing depth of A/B, 0) + 1
        // Check if A is already a ConcatNode (height == -1)
        let a_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                a_height,
                i64.const_int(-1i64 as u64, true),
                "a_is_concat",
            )
            .map_err(llvm_err)?;
        let a_depth_load_bb = self.context.append_basic_block(concat_fn, "a_depth_load");
        let a_depth_done_bb = self.context.append_basic_block(concat_fn, "a_depth_done");
        let _ =
            self.builder
                .build_conditional_branch(a_is_concat, a_depth_load_bb, a_depth_done_bb);
        self.builder.position_at_end(a_depth_load_bb);
        let a_depth_ptr = unsafe {
            self.builder
                .build_gep(i64, a_node, &[i64.const_int(0, false)], "a_depth_p")
                .map_err(llvm_err)
        }?;
        let a_depth_val = self
            .builder
            .build_load(i64, a_depth_ptr, "a_depth_v")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(a_depth_done_bb);
        self.builder.position_at_end(a_depth_done_bb);
        let a_depth_phi = self
            .builder
            .build_phi(i64, "a_depth_phi")
            .map_err(llvm_err)?;
        a_depth_phi.add_incoming(&[(&zero, cc_lazy_concat)]); // flat tree (a_is_concat == false)
        a_depth_phi.add_incoming(&[(&a_depth_val, a_depth_load_bb)]); // ConcatNode (loaded depth)
        let a_depth = a_depth_phi.as_basic_value().into_int_value();

        // Check if B is already a ConcatNode (height == -1)
        let b_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                b_height,
                i64.const_int(-1i64 as u64, true),
                "b_is_concat",
            )
            .map_err(llvm_err)?;
        let b_depth_load_bb = self.context.append_basic_block(concat_fn, "b_depth_load");
        let b_depth_done_bb = self.context.append_basic_block(concat_fn, "b_depth_done");
        let _ =
            self.builder
                .build_conditional_branch(b_is_concat, b_depth_load_bb, b_depth_done_bb);
        self.builder.position_at_end(b_depth_load_bb);
        let b_depth_ptr = unsafe {
            self.builder
                .build_gep(i64, b_node, &[i64.const_int(0, false)], "b_depth_p")
                .map_err(llvm_err)
        }?;
        let b_depth_val = self
            .builder
            .build_load(i64, b_depth_ptr, "b_depth_v")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(b_depth_done_bb);
        self.builder.position_at_end(b_depth_done_bb);
        let b_depth_phi = self
            .builder
            .build_phi(i64, "b_depth_phi")
            .map_err(llvm_err)?;
        b_depth_phi.add_incoming(&[(&zero, a_depth_done_bb)]); // flat tree (b_is_concat == false)
        b_depth_phi.add_incoming(&[(&b_depth_val, b_depth_load_bb)]); // ConcatNode (loaded depth)
        let b_depth = b_depth_phi.as_basic_value().into_int_value();

        // new_depth = max(a_depth, b_depth) + 1
        let a_gt_b = self
            .builder
            .build_int_compare(IntPredicate::SGT, a_depth, b_depth, "a_gt_b")
            .map_err(llvm_err)?;
        let max_depth = self
            .builder
            .build_select(a_gt_b, a_depth, b_depth, "max_depth")
            .map_err(llvm_err)?
            .into_int_value();
        let new_depth = self
            .builder
            .build_int_add(max_depth, one, "new_depth")
            .map_err(llvm_err)?;

        // Allocate ConcatNode: {i64 depth, i64 total_len, self.list_typepe left, self.list_typepe right} = 80 bytes
        let concat_node_size = i64.const_int(80, false);
        let concat_node = self
            .builder
            .build_call(malloc_rc_fn, &[concat_node_size.into()], "concat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let cn_i8 = self
            .builder
            .build_pointer_cast(concat_node, ptr, "cn_i8")
            .map_err(llvm_err)?;

        // Store depth at offset 0
        let _ = self
            .builder
            .build_store(cn_i8, new_depth)
            .map_err(llvm_err)?;
        // Store total_len at offset 8
        let cn_tl = unsafe {
            self.builder
                .build_gep(i64, cn_i8, &[i64.const_int(1, false)], "cn_tl")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(cn_tl, total).map_err(llvm_err)?;
        // Store left list at offset 16 (2 * 8 bytes)
        let cn_left = unsafe {
            self.builder
                .build_gep(i64, cn_i8, &[i64.const_int(2, false)], "cn_left")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(cn_left, concat_a)
            .map_err(llvm_err)?;
        // Store right list at offset 40 (5 * 8 bytes)
        let cn_right = unsafe {
            self.builder
                .build_gep(i64, cn_i8, &[i64.const_int(5, false)], "cn_right")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(cn_right, concat_b)
            .map_err(llvm_err)?;

        // rc_inc both children's nodes (they're now referenced by the ConcatNode)
        let cc_rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(cc_rc_inc_fn, &[a_node.into()], "")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(cc_rc_inc_fn, &[b_node.into()], "")
            .map_err(llvm_err)?;

        // Return {concat_node, total, -1} — balance if ConcatNode spine too deep
        let lc_undef = self.list_type.get_undef();
        let lc_r1 = self
            .builder
            .build_insert_value(lc_undef, concat_node, 0, "lc_r1")
            .map_err(llvm_err)?;
        let lc_r2 = self
            .builder
            .build_insert_value(lc_r1, total, 1, "lc_r2")
            .map_err(llvm_err)?;
        let lc_r3 = self
            .builder
            .build_insert_value(lc_r2, i64.const_int(-1i64 as u64, true), 2, "lc_r3")
            .map_err(llvm_err)?
            .into_struct_value();

        let cc_balance_chk = self.context.append_basic_block(concat_fn, "balance_chk");
        let cc_return_lazy = self.context.append_basic_block(concat_fn, "return_lazy");
        let cc_flatten_bal = self.context.append_basic_block(concat_fn, "flatten_bal");
        let _ = self.builder.build_unconditional_branch(cc_balance_chk);

        self.builder.position_at_end(cc_balance_chk);
        let max_concat_depth = i64.const_int(32, false);
        let needs_balance = self
            .builder
            .build_int_compare(IntPredicate::SGT, new_depth, max_concat_depth, "needs_bal")
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(needs_balance, cc_flatten_bal, cc_return_lazy);

        self.builder.position_at_end(cc_return_lazy);
        let _ = self.builder.build_return(Some(&lc_r3));

        self.builder.position_at_end(cc_flatten_bal);
        let cc_flatten_fn = self.module.get_function("action_list_flatten").unwrap();
        let cc_balanced = self
            .builder
            .build_call(cc_flatten_fn, &[lc_r3.into()], "balanced")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&cc_balanced));
