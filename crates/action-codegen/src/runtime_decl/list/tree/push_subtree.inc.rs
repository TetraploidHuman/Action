// ---- action_list_push_leaf(ptr acc, ptr leaf) -> void ----
        // Bulk-push all elements from a leaf into the accumulator.
        // Uses memcpy+rc_inc when accumulator's last leaf has room; falls back to per-element push.
        let pl_fn = self.module.get_function("action_list_push_leaf").unwrap();
        let pl_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let pl_push_fn = self.module.get_function("action_list_push").unwrap();
        let string_ty = self.string_type;
        let leaf_ty = self.leaf_type;
        let pl_entry = self.context.append_basic_block(pl_fn, "entry");
        let pl_loop_bb = self.context.append_basic_block(pl_fn, "lp");
        let pl_body_bb = self.context.append_basic_block(pl_fn, "body");
        let pl_fb_bb = self.context.append_basic_block(pl_fn, "fb");
        let pl_bulk_bb = self.context.append_basic_block(pl_fn, "bulk");
        let pl_fallback_bb = self.context.append_basic_block(pl_fn, "fallback");
        let pl_memcpy_bb = self.context.append_basic_block(pl_fn, "memcpy");
        let pl_rc_loop = self.context.append_basic_block(pl_fn, "rc_lp");
        let pl_rc_body = self.context.append_basic_block(pl_fn, "rc_body");
        let pl_rc_done = self.context.append_basic_block(pl_fn, "rc_done");
        let pl_done = self.context.append_basic_block(pl_fn, "done");
        self.builder.position_at_end(pl_entry);
        let pl_acc = pl_fn.get_first_param().unwrap().into_pointer_value();
        let pl_leaf = pl_fn.get_nth_param(1).unwrap().into_pointer_value();
        let pl_leaf_i8 = self
            .builder
            .build_pointer_cast(pl_leaf, ptr, "lf_i8")
            .map_err(llvm_err)?;
        let pl_leaf_cnt_r = self
            .builder
            .build_load(i32, pl_leaf_i8, "lf_cnt")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_leaf_cnt = self
            .builder
            .build_int_z_extend(pl_leaf_cnt_r, i64, "cnt64")
            .map_err(llvm_err)?;
        let pl_pos = self.builder.build_alloca(i64, "pos").map_err(llvm_err)?;
        let _ = self.builder.build_store(pl_pos, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_loop_bb);
        // Loop header
        self.builder.position_at_end(pl_loop_bb);
        let pl_pos_v = self
            .builder
            .build_load(i64, pl_pos, "pos_v")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_cmp = self
            .builder
            .build_int_compare(IntPredicate::SLT, pl_pos_v, pl_leaf_cnt, "cmp")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pl_cmp, pl_body_bb, pl_fb_bb);
        // Loop body: try to bulk-push remaining elements
        self.builder.position_at_end(pl_body_bb);
        let pl_cur = self
            .builder
            .build_load(self.list_type, pl_acc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let pl_cur_node = self
            .builder
            .build_extract_value(pl_cur, 0, "cur_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pl_cur_total = self
            .builder
            .build_extract_value(pl_cur, 1, "cur_total")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_cur_h = self
            .builder
            .build_extract_value(pl_cur, 2, "cur_h")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_cur_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, pl_cur_h, zero, "cur_h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pl_cur_h0, pl_bulk_bb, pl_fallback_bb);
        // Bulk path: result is h=0 (single leaf)
        self.builder.position_at_end(pl_bulk_bb);
        let pl_lst_lf = pl_cur_node;
        let pl_lst_i8 = self
            .builder
            .build_pointer_cast(pl_lst_lf, ptr, "lst_i8")
            .map_err(llvm_err)?;
        let pl_lst_cnt_r = self
            .builder
            .build_load(i32, pl_lst_i8, "lst_cnt")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_lst_cnt = self
            .builder
            .build_int_z_extend(pl_lst_cnt_r, i64, "lst_cnt64")
            .map_err(llvm_err)?;
        let pl_room = self
            .builder
            .build_int_sub(i64.const_int(64, false), pl_lst_cnt, "room")
            .map_err(llvm_err)?;
        let pl_rem = self
            .builder
            .build_int_sub(pl_leaf_cnt, pl_pos_v, "rem")
            .map_err(llvm_err)?;
        let pl_batch = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, pl_rem, pl_room, "use_rem")
                    .map_err(llvm_err)?,
                pl_rem,
                pl_room,
                "batch",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let pl_batch_z = self
            .builder
            .build_int_compare(IntPredicate::EQ, pl_batch, zero, "batch_z")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pl_batch_z, pl_fallback_bb, pl_memcpy_bb);
        // memcpy block
        self.builder.position_at_end(pl_memcpy_bb);
        let pl_lf_int = self
            .builder
            .build_ptr_to_int(pl_lst_lf, i64, "lf_int")
            .map_err(llvm_err)?;
        let pl_rc_a = self
            .builder
            .build_int_sub(pl_lf_int, i64.const_int(8, false), "rc_a")
            .map_err(llvm_err)?;
        let pl_rc_p = self
            .builder
            .build_int_to_ptr(pl_rc_a, ptr, "rc_p")
            .map_err(llvm_err)?;
        let pl_rc_v = self
            .builder
            .build_load(i64, pl_rc_p, "rc_v")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_need_cow = self
            .builder
            .build_int_compare(IntPredicate::SGT, pl_rc_v, one, "need_cow")
            .map_err(llvm_err)?;
        let pl_leaf_sz = leaf_ty.size_of().ok_or("leaf size")?;
        let pl_cow_lf = self
            .builder
            .build_call(malloc_rc_fn, &[pl_leaf_sz.into()], "cow_lf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                pl_memcpy_fn,
                &[pl_cow_lf.into(), pl_lst_lf.into(), pl_leaf_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let pl_use_lf = self
            .builder
            .build_select(pl_need_cow, pl_cow_lf, pl_lst_lf, "use_lf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pl_use_lf_i8 = self
            .builder
            .build_pointer_cast(pl_use_lf, ptr, "use_i8")
            .map_err(llvm_err)?;
        let pl_dst_off = self
            .builder
            .build_int_add(
                i64.const_int(8, false),
                self.builder
                    .build_int_mul(pl_lst_cnt, i64.const_int(16, false), "dstoff_mul")
                    .map_err(llvm_err)?,
                "dstoff",
            )
            .map_err(llvm_err)?;
        let pl_dst = unsafe {
            self.builder
                .build_gep(i8, pl_use_lf_i8, &[pl_dst_off], "dst")
                .map_err(llvm_err)
        }?;
        let pl_src_off = self
            .builder
            .build_int_add(
                i64.const_int(8, false),
                self.builder
                    .build_int_mul(pl_pos_v, i64.const_int(16, false), "srcoff_mul")
                    .map_err(llvm_err)?,
                "srcoff",
            )
            .map_err(llvm_err)?;
        let pl_src = unsafe {
            self.builder
                .build_gep(i8, pl_leaf_i8, &[pl_src_off], "src")
                .map_err(llvm_err)
        }?;
        let pl_cpy_sz = self
            .builder
            .build_int_mul(pl_batch, i64.const_int(16, false), "cpy_sz")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                pl_memcpy_fn,
                &[pl_dst.into(), pl_src.into(), pl_cpy_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        // rc_inc each copied element
        let pl_rc_i = self.builder.build_alloca(i64, "rc_i").map_err(llvm_err)?;
        let _ = self.builder.build_store(pl_rc_i, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_rc_loop);
        self.builder.position_at_end(pl_rc_loop);
        let pl_rc_iv = self
            .builder
            .build_load(i64, pl_rc_i, "rc_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_rc_cmp = self
            .builder
            .build_int_compare(IntPredicate::SLT, pl_rc_iv, pl_batch, "rc_cmp")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pl_rc_cmp, pl_rc_body, pl_rc_done);
        self.builder.position_at_end(pl_rc_body);
        let pl_el_off = self
            .builder
            .build_int_add(
                i64.const_int(8, false),
                self.builder
                    .build_int_mul(
                        self.builder
                            .build_int_add(pl_pos_v, pl_rc_iv, "el_idx")
                            .map_err(llvm_err)?,
                        i64.const_int(16, false),
                        "el_off_mul",
                    )
                    .map_err(llvm_err)?,
                "el_off",
            )
            .map_err(llvm_err)?;
        let pl_el_p = unsafe {
            self.builder
                .build_gep(i8, pl_leaf_i8, &[pl_el_off], "el_p")
                .map_err(llvm_err)
        }?;
        let pl_el_ev = self
            .builder
            .build_load(string_ty, pl_el_p, "el_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let pl_str_rc_inc_fn = self.module.get_function("action_string_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(pl_str_rc_inc_fn, &[pl_el_ev.into()], "")
            .map_err(llvm_err)?;
        let pl_rc_next = self
            .builder
            .build_int_add(pl_rc_iv, one, "rc_next")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pl_rc_i, pl_rc_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_rc_loop);
        // Update leaf count and accumulator
        self.builder.position_at_end(pl_rc_done);
        let pl_new_lc = self
            .builder
            .build_int_add(pl_lst_cnt, pl_batch, "new_lc")
            .map_err(llvm_err)?;
        let pl_new_lc_i32 = self
            .builder
            .build_int_truncate(pl_new_lc, i32, "new_lc_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pl_use_lf_i8, pl_new_lc_i32)
            .map_err(llvm_err)?;
        let pl_new_total = self
            .builder
            .build_int_add(pl_cur_total, pl_batch, "new_total")
            .map_err(llvm_err)?;
        let pl_undef = self.list_type.get_undef();
        let pl_v1 = self
            .builder
            .build_insert_value(pl_undef, pl_use_lf, 0, "v1")
            .map_err(llvm_err)?;
        let pl_v2 = self
            .builder
            .build_insert_value(pl_v1, pl_new_total, 1, "v2")
            .map_err(llvm_err)?;
        let pl_v3 = self
            .builder
            .build_insert_value(pl_v2, zero, 2, "v3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(pl_acc, pl_v3).map_err(llvm_err)?;
        let pl_nxt = self
            .builder
            .build_int_add(pl_pos_v, pl_batch, "nxt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(pl_pos, pl_nxt).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_loop_bb);
        // Fallback: push one element via action_list_push
        self.builder.position_at_end(pl_fallback_bb);
        let pl_fb_off = self
            .builder
            .build_int_add(
                i64.const_int(8, false),
                self.builder
                    .build_int_mul(pl_pos_v, i64.const_int(16, false), "fb_off_m")
                    .map_err(llvm_err)?,
                "fb_off",
            )
            .map_err(llvm_err)?;
        let pl_fb_ep = unsafe {
            self.builder
                .build_gep(i8, pl_leaf_i8, &[pl_fb_off], "fb_ep")
                .map_err(llvm_err)
        }?;
        let pl_fb_ev = self
            .builder
            .build_load(string_ty, pl_fb_ep, "fb_ev")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(pl_str_rc_inc_fn, &[pl_fb_ev.into_struct_value().into()], "")
            .map_err(llvm_err)?;
        let pl_fb_cur = self
            .builder
            .build_load(self.list_type, pl_acc, "fb_cur")
            .map_err(llvm_err)?;
        let pl_fb_new = self
            .builder
            .build_call(
                pl_push_fn,
                &[pl_fb_cur.into(), pl_fb_ev.as_basic_value_enum().into()],
                "fb_new",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self
            .builder
            .build_store(pl_acc, pl_fb_new)
            .map_err(llvm_err)?;
        let pl_fb_next = self
            .builder
            .build_int_add(pl_pos_v, one, "fb_next")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pl_pos, pl_fb_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_loop_bb);
        // Final branch
        self.builder.position_at_end(pl_fb_bb);
        let _ = self.builder.build_unconditional_branch(pl_done);
        self.builder.position_at_end(pl_done);
        let _ = self.builder.build_return(None);

        // ---- action_list_push_subtree(ptr acc, ptr node, i64 height) -> void ----
        // Pushes all elements from a B-tree subtree (height >= 0) or ConcatNode DAG into acc.
        // ConcatNode (height == -1): walk left/right without flattening the entire DAG.
        let ps_fn = self
            .module
            .get_function("action_list_push_subtree")
            .unwrap();
        let child_entry_ty = self.child_entry_type;
        let ps_entry = self.context.append_basic_block(ps_fn, "entry");
        let ps_concat_walk = self.context.append_basic_block(ps_fn, "concat_walk");
        let ps_h0_leaf = self.context.append_basic_block(ps_fn, "h0_leaf");
        let ps_h1_intl = self.context.append_basic_block(ps_fn, "h1_intl");
        let ps_hgt1_recurse = self.context.append_basic_block(ps_fn, "hgt1");
        let ps_done = self.context.append_basic_block(ps_fn, "done");
        self.builder.position_at_end(ps_entry);
        let ps_acc = ps_fn.get_first_param().unwrap().into_pointer_value();
        let ps_node = ps_fn.get_nth_param(1).unwrap().into_pointer_value();
        let ps_height = ps_fn.get_nth_param(2).unwrap().into_int_value();
        let ps_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ps_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let ps_not_concat = self.context.append_basic_block(ps_fn, "not_concat");
        let _ = self
            .builder
            .build_conditional_branch(ps_is_concat, ps_concat_walk, ps_not_concat);
        // ConcatNode: recursively push left then right subtrees (lazy concat-tree walk)
        self.builder.position_at_end(ps_concat_walk);
        let ps_cn_i8 = self
            .builder
            .build_pointer_cast(ps_node, ptr, "cn_i8")
            .map_err(llvm_err)?;
        let ps_left_ptr = unsafe {
            self.builder
                .build_gep(i8, ps_cn_i8, &[i64.const_int(16, false)], "left_ptr")
                .map_err(llvm_err)
        }?;
        let ps_left = self
            .builder
            .build_load(self.list_type, ps_left_ptr, "left")
            .map_err(llvm_err)?
            .into_struct_value();
        let ps_right_ptr = unsafe {
            self.builder
                .build_gep(i8, ps_cn_i8, &[i64.const_int(40, false)], "right_ptr")
                .map_err(llvm_err)
        }?;
        let ps_right = self
            .builder
            .build_load(self.list_type, ps_right_ptr, "right")
            .map_err(llvm_err)?
            .into_struct_value();
        let ps_l_node = self
            .builder
            .build_extract_value(ps_left, 0, "l_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ps_l_h = self
            .builder
            .build_extract_value(ps_left, 2, "l_fh")
            .map_err(llvm_err)?
            .into_int_value();
        let ps_r_node = self
            .builder
            .build_extract_value(ps_right, 0, "r_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ps_r_h = self
            .builder
            .build_extract_value(ps_right, 2, "r_fh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(ps_fn, &[ps_acc.into(), ps_l_node.into(), ps_l_h.into()], "")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(ps_fn, &[ps_acc.into(), ps_r_node.into(), ps_r_h.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ps_done);
        // Three-way dispatch: h==0, h==1, h>=2
        self.builder.position_at_end(ps_not_concat);
        let ps_is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, ps_height, zero, "is_h0")
            .map_err(llvm_err)?;
        let ps_not_h0 = self.context.append_basic_block(ps_fn, "not_h0");
        let _ = self
            .builder
            .build_conditional_branch(ps_is_h0, ps_h0_leaf, ps_not_h0);
        self.builder.position_at_end(ps_not_h0);
        let ps_is_h1 = self
            .builder
            .build_int_compare(IntPredicate::EQ, ps_height, one, "is_h1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ps_is_h1, ps_h1_intl, ps_hgt1_recurse);
        // === ps_h0_leaf: delegate to action_list_push_leaf ===
        self.builder.position_at_end(ps_h0_leaf);
        let ps_leaf_fn = self.module.get_function("action_list_push_leaf").unwrap();
        let _ = self
            .builder
            .build_call(ps_leaf_fn, &[ps_acc.into(), ps_node.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ps_done);
        // === ps_h1_intl: internal node with leaf children ===
        self.builder.position_at_end(ps_h1_intl);
        let ps_intl_i8 = self
            .builder
            .build_pointer_cast(ps_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let ps_intl_cnt_r = self
            .builder
            .build_load(i32, ps_intl_i8, "intl_cnt")
            .map_err(llvm_err)?
            .into_int_value();
        let ps_intl_cnt = self
            .builder
            .build_int_z_extend(ps_intl_cnt_r, i64, "intl_cnt64")
            .map_err(llvm_err)?;
        let ps_ci = self.builder.build_alloca(i64, "ci").map_err(llvm_err)?;
        let _ = self.builder.build_store(ps_ci, zero).map_err(llvm_err)?;
        let ps_cloop = self.context.append_basic_block(ps_fn, "clp");
        let ps_cbody = self.context.append_basic_block(ps_fn, "cbody");
        let ps_cdone = self.context.append_basic_block(ps_fn, "cdone");
        let _ = self.builder.build_unconditional_branch(ps_cloop);
        self.builder.position_at_end(ps_cloop);
        let ps_civ = self
            .builder
            .build_load(i64, ps_ci, "civ")
            .map_err(llvm_err)?
            .into_int_value();
        let ps_ccmp = self
            .builder
            .build_int_compare(IntPredicate::SLT, ps_civ, ps_intl_cnt, "ccmp")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ps_ccmp, ps_cbody, ps_cdone);
        self.builder.position_at_end(ps_cbody);
        // Load child entry: node+16 + ci*16
        let ps_ce_off = self
            .builder
            .build_int_add(
                i64.const_int(16, false),
                self.builder
                    .build_int_mul(ps_civ, i64.const_int(16, false), "ce_off_m")
                    .map_err(llvm_err)?,
                "ce_off",
            )
            .map_err(llvm_err)?;
        let ps_ce_p = unsafe {
            self.builder
                .build_gep(i8, ps_intl_i8, &[ps_ce_off], "ce_p")
                .map_err(llvm_err)
        }?;
        let ps_ce = self
            .builder
            .build_load(child_entry_ty, ps_ce_p, "ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let ps_child = self
            .builder
            .build_extract_value(ps_ce, 0, "child")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Recursively push this child (it's a leaf, h=0)
        let _ = self
            .builder
            .build_call(ps_fn, &[ps_acc.into(), ps_child.into(), zero.into()], "")
            .map_err(llvm_err)?;
        let ps_cnext = self
            .builder
            .build_int_add(ps_civ, one, "cnext")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(ps_ci, ps_cnext)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ps_cloop);
        self.builder.position_at_end(ps_cdone);
        let _ = self.builder.build_unconditional_branch(ps_done);
        // === ps_hgt1_recurse: deep internal node — recurse into children ===
        self.builder.position_at_end(ps_hgt1_recurse);
        let ps_d_intl_i8 = self
            .builder
            .build_pointer_cast(ps_node, ptr, "dintl_i8")
            .map_err(llvm_err)?;
        let ps_d_cnt_r = self
            .builder
            .build_load(i32, ps_d_intl_i8, "dcnt")
            .map_err(llvm_err)?
            .into_int_value();
        let ps_d_cnt = self
            .builder
            .build_int_z_extend(ps_d_cnt_r, i64, "dcnt64")
            .map_err(llvm_err)?;
        let ps_di = self.builder.build_alloca(i64, "di").map_err(llvm_err)?;
        let _ = self.builder.build_store(ps_di, zero).map_err(llvm_err)?;
        let ps_dloop = self.context.append_basic_block(ps_fn, "dlp");
        let ps_dbody = self.context.append_basic_block(ps_fn, "dbody");
        let ps_ddone = self.context.append_basic_block(ps_fn, "ddone");
        let _ = self.builder.build_unconditional_branch(ps_dloop);
        self.builder.position_at_end(ps_dloop);
        let ps_div = self
            .builder
            .build_load(i64, ps_di, "div")
            .map_err(llvm_err)?
            .into_int_value();
        let ps_dcmp = self
            .builder
            .build_int_compare(IntPredicate::SLT, ps_div, ps_d_cnt, "dcmp")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ps_dcmp, ps_dbody, ps_ddone);
        self.builder.position_at_end(ps_dbody);
        let ps_dce_off = self
            .builder
            .build_int_add(
                i64.const_int(16, false),
                self.builder
                    .build_int_mul(ps_div, i64.const_int(16, false), "dce_off_m")
                    .map_err(llvm_err)?,
                "dce_off",
            )
            .map_err(llvm_err)?;
        let ps_dce_p = unsafe {
            self.builder
                .build_gep(i8, ps_d_intl_i8, &[ps_dce_off], "dce_p")
                .map_err(llvm_err)
        }?;
        let ps_dce = self
            .builder
            .build_load(child_entry_ty, ps_dce_p, "dce")
            .map_err(llvm_err)?
            .into_struct_value();
        let ps_dchild = self
            .builder
            .build_extract_value(ps_dce, 0, "dchild")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ps_dh = self
            .builder
            .build_int_sub(ps_height, one, "dh")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(ps_fn, &[ps_acc.into(), ps_dchild.into(), ps_dh.into()], "")
            .map_err(llvm_err)?;
        let ps_dnext = self
            .builder
            .build_int_add(ps_div, one, "dnext")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(ps_di, ps_dnext)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ps_dloop);
        self.builder.position_at_end(ps_ddone);
        let _ = self.builder.build_unconditional_branch(ps_done);
        // Done: return
        self.builder.position_at_end(ps_done);
        let _ = self.builder.build_return(None);

        self.define_list_range_walk_rec()?;
