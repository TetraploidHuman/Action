// ---- action_list_insert({ptr, i64, i64}, i64 index, {i64, ptr}) -> {ptr, i64, i64} ----
        let li_fn = self.module.add_function(
            "action_list_insert",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into(), str_ty.into()], false),
            None,
        );
        let li_entry = self.context.append_basic_block(li_fn, "entry");
        let li_concat = self.context.append_basic_block(li_fn, "concat");
        let li_concat_append = self.context.append_basic_block(li_fn, "concat_append");
        let li_concat_chk_prepend = self.context.append_basic_block(li_fn, "concat_chk_pre");
        let li_concat_prepend = self.context.append_basic_block(li_fn, "concat_prepend");
        let li_concat_dispatch = self.context.append_basic_block(li_fn, "concat_dispatch");
        let li_concat_ins_left = self.context.append_basic_block(li_fn, "concat_ins_left");
        let li_concat_ins_right = self.context.append_basic_block(li_fn, "concat_ins_right");
        let li_concat_boundary = self.context.append_basic_block(li_fn, "concat_boundary");
        let li_concat_route = self.context.append_basic_block(li_fn, "concat_route");
        let li_normal = self.context.append_basic_block(li_fn, "normal");
        let li_h0 = self.context.append_basic_block(li_fn, "h0");
        let li_h0_cow = self.context.append_basic_block(li_fn, "h0_cow");
        let li_h0_cow_copy = self.context.append_basic_block(li_fn, "h0_cow_copy");
        let li_h0_ready = self.context.append_basic_block(li_fn, "h0_ready");
        let li_h0_shift_loop = self.context.append_basic_block(li_fn, "h0_shift_loop");
        let li_h0_shift_body = self.context.append_basic_block(li_fn, "h0_shift_body");
        let li_h0_shift_done = self.context.append_basic_block(li_fn, "h0_shift_done");
        let li_h0_done = self.context.append_basic_block(li_fn, "h0_done");
        let li_hgt0 = self.context.append_basic_block(li_fn, "hgt0");
        self.builder.position_at_end(li_entry);
        let li_list = li_fn.get_first_param().unwrap().into_struct_value();
        let li_index = li_fn.get_nth_param(1).unwrap().into_int_value();
        let li_elem = li_fn.get_nth_param(2).unwrap().into_struct_value();
        let li_node = self
            .builder
            .build_extract_value(li_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let li_total_len = self
            .builder
            .build_extract_value(li_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let li_height = self
            .builder
            .build_extract_value(li_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let li_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                li_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(li_is_concat, li_concat, li_normal);
        // ConcatNode: lazy concat insert (append/prepend/middle dispatch)
        self.builder.position_at_end(li_concat);
        let li_create_fn = self.module.get_function("action_list_create").unwrap();
        let li_push_fn = self.module.get_function("action_list_push").unwrap();
        let li_concat_fn = self.module.get_function("action_list_concat").unwrap();
        // append: index == len -> lazy concat(list, singleton(elem))
        let li_cc_is_append = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_index, li_total_len, "cc_app")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            li_cc_is_append,
            li_concat_append,
            li_concat_chk_prepend,
        );
        self.builder.position_at_end(li_concat_append);
        let li_cc_empty = self
            .builder
            .build_call(li_create_fn, &[zero.into()], "cc_empty")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_sing_a = self
            .builder
            .build_call(
                li_push_fn,
                &[li_cc_empty.into(), li_elem.into()],
                "cc_sing_a",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_app_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_list.into(), li_cc_sing_a.into()],
                "cc_app_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_app_r));
        // prepend: index == 0 -> lazy concat(singleton(elem), list)
        self.builder.position_at_end(li_concat_chk_prepend);
        let li_cc_is_prepend = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_index, zero, "cc_pre")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            li_cc_is_prepend,
            li_concat_prepend,
            li_concat_dispatch,
        );
        self.builder.position_at_end(li_concat_prepend);
        let li_cc_empty2 = self
            .builder
            .build_call(li_create_fn, &[zero.into()], "cc_empty2")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_sing_p = self
            .builder
            .build_call(
                li_push_fn,
                &[li_cc_empty2.into(), li_elem.into()],
                "cc_sing_p",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_pre_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_sing_p.into(), li_list.into()],
                "cc_pre_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_pre_r));
        // middle insert: lazy left/right dispatch on ConcatNode
        self.builder.position_at_end(li_concat_dispatch);
        let li_cc_ln_p = unsafe {
            self.builder
                .build_gep(ptr, li_node, &[i64.const_int(2, false)], "cc_ln_p")
                .map_err(llvm_err)
        }?;
        let li_cc_left_node = self
            .builder
            .build_load(ptr, li_cc_ln_p, "cc_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let li_cc_ll_p = unsafe {
            self.builder
                .build_gep(i64, li_node, &[i64.const_int(3, false)], "cc_ll_p")
                .map_err(llvm_err)
        }?;
        let li_cc_left_len = self
            .builder
            .build_load(i64, li_cc_ll_p, "cc_ll")
            .map_err(llvm_err)?
            .into_int_value();
        let li_cc_lh_p = unsafe {
            self.builder
                .build_gep(i64, li_node, &[i64.const_int(4, false)], "cc_lh_p")
                .map_err(llvm_err)
        }?;
        let li_cc_left_h = self
            .builder
            .build_load(i64, li_cc_lh_p, "cc_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let li_cc_undef = self.list_type.get_undef();
        let li_cc_l1 = self
            .builder
            .build_insert_value(li_cc_undef, li_cc_left_node, 0, "cc_l1")
            .map_err(llvm_err)?;
        let li_cc_l2 = self
            .builder
            .build_insert_value(li_cc_l1, li_cc_left_len, 1, "cc_l2")
            .map_err(llvm_err)?;
        let li_cc_left = self
            .builder
            .build_insert_value(li_cc_l2, li_cc_left_h, 2, "cc_left")
            .map_err(llvm_err)?
            .into_struct_value();
        let li_cc_rn_p = unsafe {
            self.builder
                .build_gep(ptr, li_node, &[i64.const_int(5, false)], "cc_rn_p")
                .map_err(llvm_err)
        }?;
        let li_cc_right_node = self
            .builder
            .build_load(ptr, li_cc_rn_p, "cc_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let li_cc_rl_p = unsafe {
            self.builder
                .build_gep(i64, li_node, &[i64.const_int(6, false)], "cc_rl_p")
                .map_err(llvm_err)
        }?;
        let li_cc_right_len = self
            .builder
            .build_load(i64, li_cc_rl_p, "cc_rl")
            .map_err(llvm_err)?
            .into_int_value();
        let li_cc_rh_p = unsafe {
            self.builder
                .build_gep(i64, li_node, &[i64.const_int(7, false)], "cc_rh_p")
                .map_err(llvm_err)
        }?;
        let li_cc_right_h = self
            .builder
            .build_load(i64, li_cc_rh_p, "cc_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let li_cc_rundef = self.list_type.get_undef();
        let li_cc_r1 = self
            .builder
            .build_insert_value(li_cc_rundef, li_cc_right_node, 0, "cc_r1")
            .map_err(llvm_err)?;
        let li_cc_r2 = self
            .builder
            .build_insert_value(li_cc_r1, li_cc_right_len, 1, "cc_r2")
            .map_err(llvm_err)?;
        let li_cc_right = self
            .builder
            .build_insert_value(li_cc_r2, li_cc_right_h, 2, "cc_right")
            .map_err(llvm_err)?
            .into_struct_value();
        let li_cc_lt_left = self
            .builder
            .build_int_compare(IntPredicate::SLT, li_index, li_cc_left_len, "cc_lt_l")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            li_cc_lt_left,
            li_concat_ins_left,
            li_concat_route,
        );
        self.builder.position_at_end(li_concat_route);
        let li_cc_is_boundary = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_index, li_cc_left_len, "cc_bnd")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            li_cc_is_boundary,
            li_concat_boundary,
            li_concat_ins_right,
        );
        self.builder.position_at_end(li_concat_ins_left);
        let li_cc_il = self
            .builder
            .build_call(
                li_fn,
                &[li_cc_left.into(), li_index.into(), li_elem.into()],
                "cc_il",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_il_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_il.into(), li_cc_right.into()],
                "cc_il_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_il_r));
        self.builder.position_at_end(li_concat_boundary);
        let li_cc_empty3 = self
            .builder
            .build_call(li_create_fn, &[zero.into()], "cc_empty3")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_sing_b = self
            .builder
            .build_call(
                li_push_fn,
                &[li_cc_empty3.into(), li_elem.into()],
                "cc_sing_b",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_mid = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_left.into(), li_cc_sing_b.into()],
                "cc_mid",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_bnd_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_mid.into(), li_cc_right.into()],
                "cc_bnd_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_bnd_r));
        self.builder.position_at_end(li_concat_ins_right);
        let li_cc_ri = self
            .builder
            .build_int_sub(li_index, li_cc_left_len, "cc_ri")
            .map_err(llvm_err)?;
        let li_cc_ir = self
            .builder
            .build_call(
                li_fn,
                &[li_cc_right.into(), li_cc_ri.into(), li_elem.into()],
                "cc_ir",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_ir_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_left.into(), li_cc_ir.into()],
                "cc_ir_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_ir_r));
        // Normal path: check h=0 vs h>0
        self.builder.position_at_end(li_normal);
        let li_is_h0 = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                li_height,
                i64.const_int(0, false),
                "is_h0",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(li_is_h0, li_h0, li_hgt0);
        // === h=0: direct leaf manipulation (with room) ===
        self.builder.position_at_end(li_h0);
        let li_leaf_i8 = self
            .builder
            .build_pointer_cast(li_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let li_count_raw = self
            .builder
            .build_load(i32, li_leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let li_count = self
            .builder
            .build_int_z_extend(li_count_raw, i64, "count")
            .map_err(llvm_err)?;
        let z = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let li_idx0 = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, li_index, z, "idx_neg")
                    .map_err(llvm_err)?,
                z,
                li_index,
                "idx0",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let li_idx = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SGT, li_idx0, li_count, "idx_gt")
                    .map_err(llvm_err)?,
                li_count,
                li_idx0,
                "idx",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let li_is_full = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                li_count,
                i64.const_int(64, false),
                "is_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(li_is_full, li_hgt0, li_h0_cow);
        // CoW check
        self.builder.position_at_end(li_h0_cow);
        let li_node_int = self
            .builder
            .build_ptr_to_int(li_node, i64, "node_int")
            .map_err(llvm_err)?;
        let li_rc_addr = self
            .builder
            .build_int_sub(li_node_int, i64.const_int(8, false), "rc_addr")
            .map_err(llvm_err)?;
        let li_rc_ptr = self
            .builder
            .build_int_to_ptr(li_rc_addr, ptr, "rc_ptr")
            .map_err(llvm_err)?;
        let li_rc_val = self
            .builder
            .build_load(i64, li_rc_ptr, "rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let li_need_cow = self
            .builder
            .build_int_compare(IntPredicate::SGT, li_rc_val, one, "need_cow")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(li_need_cow, li_h0_cow_copy, li_h0_ready);
        // CoW: copy leaf only when shared (rc > 1)
        self.builder.position_at_end(li_h0_cow_copy);
        let leaf_ty = self.leaf_type;
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let li_cow_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "cow_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let li_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                li_memcpy_fn,
                &[li_cow_leaf.into(), li_node.into(), leaf_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(li_h0_ready);
        // Unique reference (rc == 1): mutate leaf in place; shared: use copied leaf
        self.builder.position_at_end(li_h0_ready);
        let li_leaf_phi = self.builder.build_phi(ptr, "leaf_phi").map_err(llvm_err)?;
        li_leaf_phi.add_incoming(&[(&li_node, li_h0_cow), (&li_cow_leaf, li_h0_cow_copy)]);
        let li_leaf = li_leaf_phi.as_basic_value().into_pointer_value();
        let li_leaf2_i8 = self
            .builder
            .build_pointer_cast(li_leaf, ptr, "leaf2_i8")
            .map_err(llvm_err)?;
        let li_eb = unsafe {
            self.builder
                .build_gep(i8, li_leaf2_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)
        }?;
        // Shift elements [idx..count-1] right by 1 (reverse loop)
        let li_si = self.builder.build_alloca(i64, "si").map_err(llvm_err)?;
        let li_count_minus1 = self
            .builder
            .build_int_sub(li_count, one, "cm1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(li_si, li_count_minus1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(li_h0_shift_loop);
        self.builder.position_at_end(li_h0_shift_loop);
        let li_siv = self
            .builder
            .build_load(i64, li_si, "siv")
            .map_err(llvm_err)?
            .into_int_value();
        let li_si_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, li_siv, li_idx, "si_cond")
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(li_si_cond, li_h0_shift_body, li_h0_shift_done);
        self.builder.position_at_end(li_h0_shift_body);
        let li_src = unsafe {
            self.builder
                .build_gep(self.string_type, li_eb, &[li_siv], "src")
                .map_err(llvm_err)
        }?;
        let li_sv = self
            .builder
            .build_load(self.string_type, li_src, "sv")
            .map_err(llvm_err)?;
        let li_siv_plus1 = self
            .builder
            .build_int_add(li_siv, one, "siv_p1")
            .map_err(llvm_err)?;
        let li_dst = unsafe {
            self.builder
                .build_gep(self.string_type, li_eb, &[li_siv_plus1], "dst")
                .map_err(llvm_err)
        }?;
        self.builder.build_store(li_dst, li_sv).map_err(llvm_err)?;
        let li_siv_minus1 = self
            .builder
            .build_int_sub(li_siv, one, "siv_m1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(li_si, li_siv_minus1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(li_h0_shift_loop);
        // Insert new element and increment count
        self.builder.position_at_end(li_h0_shift_done);
        let li_ins_dst = unsafe {
            self.builder
                .build_gep(self.string_type, li_eb, &[li_idx], "ins_dst")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(li_ins_dst, li_elem)
            .map_err(llvm_err)?;
        let li_new_count = self
            .builder
            .build_int_add(li_count, one, "new_count")
            .map_err(llvm_err)?;
        let li_new_count_i32 = self
            .builder
            .build_int_truncate(li_new_count, i32, "new_count_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(li_leaf2_i8, li_new_count_i32)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(li_h0_done);
        self.builder.position_at_end(li_h0_done);
        let li_new_total = self
            .builder
            .build_int_add(li_total_len, one, "new_total")
            .map_err(llvm_err)?;
        let undef_ins = self.list_type.get_undef();
        let li_r1 = self
            .builder
            .build_insert_value(undef_ins, li_leaf, 0, "r1")
            .map_err(llvm_err)?;
        let li_r2 = self
            .builder
            .build_insert_value(li_r1, li_new_total, 1, "r2")
            .map_err(llvm_err)?;
        let li_r3 = self
            .builder
            .build_insert_value(li_r2, i64.const_int(0, false), 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&li_r3));
        // Split: insert_rec B-tree path first; null → take+push+drop+concat fallback (rare)
        self.builder.position_at_end(li_hgt0);
        let li_len = self
            .builder
            .build_extract_value(li_list, 1, "li_len")
            .map_err(llvm_err)?
            .into_int_value();
        let li_idx2 = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, li_index, z, "idx_neg2")
                    .map_err(llvm_err)?,
                z,
                li_index,
                "idx_clamped2",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let li_idx3 = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SGT, li_idx2, li_len, "idx_gt2")
                    .map_err(llvm_err)?,
                li_len,
                li_idx2,
                "idx2",
            )
            .map_err(llvm_err)?
            .into_int_value();
        // If appending to end, just push
        let li_is_append = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_idx3, li_len, "is_append")
            .map_err(llvm_err)?;
        let li_append_bb = self.context.append_basic_block(li_fn, "append");
        let li_prepend_bb = self.context.append_basic_block(li_fn, "prepend");
        let li_split_bb = self.context.append_basic_block(li_fn, "split");
        let _ = self
            .builder
            .build_conditional_branch(li_is_append, li_append_bb, li_prepend_bb);
        // Prepend (index==0): concat(singleton(elem), list) — avoids take/drop split
        self.builder.position_at_end(li_prepend_bb);
        let li_is_prepend = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_idx3, z, "is_prepend")
            .map_err(llvm_err)?;
        let li_prepend_body = self.context.append_basic_block(li_fn, "prepend_body");
        let _ = self
            .builder
            .build_conditional_branch(li_is_prepend, li_prepend_body, li_split_bb);
        self.builder.position_at_end(li_prepend_body);
        let li_create_fn = self.module.get_function("action_list_create").unwrap();
        let li_push_fn = self.module.get_function("action_list_push").unwrap();
        let li_concat_fn = self.module.get_function("action_list_concat").unwrap();
        let li_pre_empty = self
            .builder
            .build_call(li_create_fn, &[z.into()], "pre_empty")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_pre_sing = self
            .builder
            .build_call(
                li_push_fn,
                &[li_pre_empty.into(), li_elem.into()],
                "pre_sing",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_pre_rv = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_pre_sing.into(), li_list.into()],
                "pre_rv",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_pre_rv));
        // Append: just push
        self.builder.position_at_end(li_append_bb);
        let li_push_fn = self.module.get_function("action_list_push").unwrap();
        let li_push_rv = self
            .builder
            .build_call(li_push_fn, &[li_list.into(), li_elem.into()], "push_rv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_push_rv));
        // Split: insert_rec B-tree path first; null → take+push+drop+concat fallback
        self.builder.position_at_end(li_split_bb);
        let li_rec_start_bb = self.context.append_basic_block(li_fn, "rec_start");
        let _ = self.builder.build_unconditional_branch(li_rec_start_bb);
        self.builder.position_at_end(li_rec_start_bb);
        let li_insert_rec_fn = self.module.get_function("action_list_insert_rec").unwrap();
        let li_root_rc_p = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(li_node, self.i64_ty(), "li_root_pi")
                            .map_err(llvm_err)?,
                        self.i64_ty().const_int(8, false),
                        "li_root_ra",
                    )
                    .map_err(llvm_err)?,
                self.ptr_ty(),
                "li_root_rp",
            )
            .map_err(llvm_err)?;
        let li_root_rc = self
            .builder
            .build_load(self.i64_ty(), li_root_rc_p, "li_root_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let li_rec_height_out = self
            .builder
            .build_alloca(self.i64_ty(), "rec_height_out")
            .map_err(llvm_err)?;
        let li_rec_root = self
            .builder
            .build_call(
                li_insert_rec_fn,
                &[
                    li_node.into(),
                    li_height.into(),
                    li_idx3.into(),
                    li_elem.into(),
                    li_root_rc.into(),
                    li_rec_height_out.into(),
                ],
                "rec_root",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let li_rec_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_rec_root, ptr.const_null(), "rec_null")
            .map_err(llvm_err)?;
        let li_rec_ok_bb = self.context.append_basic_block(li_fn, "rec_ok");
        // insert_rec null → take+push+drop+concat (rare after split_intl/out_height fixes).
        let li_rec_fallback_bb = self.context.append_basic_block(li_fn, "rec_fallback");
        let _ =
            self.builder
                .build_conditional_branch(li_rec_null, li_rec_fallback_bb, li_rec_ok_bb);
        self.builder.position_at_end(li_rec_ok_bb);
        let li_rec_new_len = self
            .builder
            .build_int_add(li_len, one, "rec_new_len")
            .map_err(llvm_err)?;
        let li_rec_r1 = self
            .builder
            .build_insert_value(self.list_type.get_undef(), li_rec_root, 0, "rec_r1")
            .map_err(llvm_err)?;
        let li_rec_r2 = self
            .builder
            .build_insert_value(li_rec_r1, li_rec_new_len, 1, "rec_r2")
            .map_err(llvm_err)?;
        let li_rec_height = self
            .builder
            .build_load(self.i64_ty(), li_rec_height_out, "rec_height")
            .map_err(llvm_err)?
            .into_int_value();
        let li_rec_r3 = self
            .builder
            .build_insert_value(li_rec_r2, li_rec_height, 2, "rec_r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&li_rec_r3));
        self.builder.position_at_end(li_rec_fallback_bb);
        let li_take_fn = self.module.get_function("action_list_take").unwrap();
        let li_drop_fn = self.module.get_function("action_list_drop").unwrap();
        let li_concat_fn = self.module.get_function("action_list_concat").unwrap();
        let li_left = self
            .builder
            .build_call(li_take_fn, &[li_list.into(), li_idx3.into()], "left")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_right = self
            .builder
            .build_call(li_drop_fn, &[li_list.into(), li_idx3.into()], "right")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_left_with = self
            .builder
            .build_call(li_push_fn, &[li_left.into(), li_elem.into()], "left_with")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let li_result = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_left_with.into(), li_right.into()],
                "result",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_result));
