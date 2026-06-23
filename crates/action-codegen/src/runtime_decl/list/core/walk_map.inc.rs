// ---- action_list_map_walk_rec(ptr node, i64 height, ptr fn, ptr acc, ptr buf_p, ptr buf_pos_p) -> void ----
        // In-order B-tree scan: apply callback to each element, batch into leaf buffer.
        let lambda_fn_ty = self.string_type.fn_type(&[i64.into()], false);
        let push_leaf_fn = self.module.get_function("action_list_push_leaf").unwrap();
        let mw_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let mw_rec_fn = self.module.add_function(
            "action_list_map_walk_rec",
            void.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                ],
                false,
            ),
            None,
        );
        let mwr_entry = self.context.append_basic_block(mw_rec_fn, "entry");
        let mwr_leaf_hdr = self.context.append_basic_block(mw_rec_fn, "leaf_hdr");
        let mwr_leaf_bdy = self.context.append_basic_block(mw_rec_fn, "leaf_bdy");
        let mwr_leaf_chk = self.context.append_basic_block(mw_rec_fn, "leaf_chk");
        let mwr_leaf_flush = self.context.append_basic_block(mw_rec_fn, "leaf_flush");
        let mwr_leaf_next = self.context.append_basic_block(mw_rec_fn, "leaf_next");
        let mwr_leaf_done = self.context.append_basic_block(mw_rec_fn, "leaf_done");
        let mwr_int_hdr = self.context.append_basic_block(mw_rec_fn, "int_hdr");
        let mwr_int_bdy = self.context.append_basic_block(mw_rec_fn, "int_bdy");
        let mwr_int_child = self.context.append_basic_block(mw_rec_fn, "int_child");
        let mwr_int_next = self.context.append_basic_block(mw_rec_fn, "int_next");
        let mwr_concat = self.context.append_basic_block(mw_rec_fn, "concat");
        let mwr_normal = self.context.append_basic_block(mw_rec_fn, "normal");
        self.builder.position_at_end(mwr_entry);
        let mwr_node = mw_rec_fn.get_first_param().unwrap().into_pointer_value();
        let mwr_height = mw_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let mwr_fn = mw_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let mwr_acc = mw_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let mwr_buf_p = mw_rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let mwr_buf_pos_p = mw_rec_fn.get_nth_param(5).unwrap().into_pointer_value();
        let mwr_neg1 = i64.const_int(-1i64 as u64, true);
        let mwr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, mwr_height, mwr_neg1, "mwr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_is_concat, mwr_concat, mwr_normal);
        self.builder.position_at_end(mwr_concat);
        let mwr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, mwr_node, &[i64.const_int(2, false)], "mwr_ln_p")
                .map_err(llvm_err)
        }?;
        let mwr_left_node = self
            .builder
            .build_load(ptr, mwr_ln_p, "mwr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mwr_lh_p = unsafe {
            self.builder
                .build_gep(i64, mwr_node, &[i64.const_int(4, false)], "mwr_lh_p")
                .map_err(llvm_err)
        }?;
        let mwr_left_h = self
            .builder
            .build_load(i64, mwr_lh_p, "mwr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, mwr_node, &[i64.const_int(5, false)], "mwr_rn_p")
                .map_err(llvm_err)
        }?;
        let mwr_right_node = self
            .builder
            .build_load(ptr, mwr_rn_p, "mwr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mwr_rh_p = unsafe {
            self.builder
                .build_gep(i64, mwr_node, &[i64.const_int(7, false)], "mwr_rh_p")
                .map_err(llvm_err)
        }?;
        let mwr_right_h = self
            .builder
            .build_load(i64, mwr_rh_p, "mwr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                mw_rec_fn,
                &[
                    mwr_left_node.into(),
                    mwr_left_h.into(),
                    mwr_fn.into(),
                    mwr_acc.into(),
                    mwr_buf_p.into(),
                    mwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                mw_rec_fn,
                &[
                    mwr_right_node.into(),
                    mwr_right_h.into(),
                    mwr_fn.into(),
                    mwr_acc.into(),
                    mwr_buf_p.into(),
                    mwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(mwr_normal);
        let mwr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, mwr_height, zero, "mwr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_is_leaf, mwr_leaf_hdr, mwr_int_hdr);

        // Leaf scan
        self.builder.position_at_end(mwr_leaf_hdr);
        let mwr_leaf_i8 = self
            .builder
            .build_pointer_cast(mwr_node, ptr, "mwr_leaf_i8")
            .map_err(llvm_err)?;
        let mwr_count_raw = self
            .builder
            .build_load(i32, mwr_leaf_i8, "mwr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_count = self
            .builder
            .build_int_z_extend(mwr_count_raw, i64, "mwr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mwr_leaf_bdy);
        self.builder.position_at_end(mwr_leaf_bdy);
        let mwr_i = self.builder.build_phi(i64, "mwr_i").map_err(llvm_err)?;
        let mwr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mwr_i.as_basic_value().into_int_value(),
                mwr_count,
                "mwr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_done_leaf, mwr_leaf_done, mwr_leaf_chk);
        self.builder.position_at_end(mwr_leaf_chk);
        let mwr_eb = unsafe {
            self.builder
                .build_gep(i8, mwr_leaf_i8, &[i64.const_int(8, false)], "mwr_eb")
                .map_err(llvm_err)?
        };
        let mwr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    mwr_eb,
                    &[mwr_i.as_basic_value().into_int_value()],
                    "mwr_ep",
                )
                .map_err(llvm_err)?
        };
        let mwr_elem = self
            .builder
            .build_load(self.string_type, mwr_ep, "mwr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let mwr_elem_tag = self
            .builder
            .build_extract_value(mwr_elem, 0, "mwr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_mapped = self
            .builder
            .build_indirect_call(lambda_fn_ty, mwr_fn, &[mwr_elem_tag.into()], "mwr_mapped")
            .map_err(llvm_err)?;
        let mwr_mapped_bv = mwr_mapped
            .try_as_basic_value()
            .basic()
            .ok_or("map_walk indirect call failed")?;
        let mwr_buf = self
            .builder
            .build_load(ptr, mwr_buf_p, "mwr_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mwr_pos = self
            .builder
            .build_load(i64, mwr_buf_pos_p, "mwr_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_buf_i8 = self
            .builder
            .build_pointer_cast(mwr_buf, ptr, "mwr_buf_i8")
            .map_err(llvm_err)?;
        let mwr_buf_eb = unsafe {
            self.builder
                .build_gep(i8, mwr_buf_i8, &[i64.const_int(8, false)], "mwr_buf_eb")
                .map_err(llvm_err)?
        };
        let mwr_buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, mwr_buf_eb, &[mwr_pos], "mwr_buf_ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(mwr_buf_ep, mwr_mapped_bv)
            .map_err(llvm_err)?;
        let mwr_pos_inc = self
            .builder
            .build_int_add(mwr_pos, i64.const_int(1, false), "mwr_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(mwr_buf_pos_p, mwr_pos_inc)
            .map_err(llvm_err)?;
        let mwr_buf_full = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                mwr_pos_inc,
                i64.const_int(64, false),
                "mwr_buf_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_buf_full, mwr_leaf_flush, mwr_leaf_next);

        self.builder.position_at_end(mwr_leaf_flush);
        let mwr_flush_cnt = i32.const_int(64, false);
        let _ = self
            .builder
            .build_store(mwr_buf_i8, mwr_flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mwr_acc.into(), mwr_buf.into()], "")
            .map_err(llvm_err)?;
        let mwr_new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[mw_leaf_sz.into()], "mwr_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mwr_new_buf_i8 = self
            .builder
            .build_pointer_cast(mwr_new_buf, ptr, "mwr_new_buf_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mwr_new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mwr_buf_p, mwr_new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mwr_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mwr_leaf_next);

        self.builder.position_at_end(mwr_leaf_next);
        let mwr_next_i = self
            .builder
            .build_int_add(
                mwr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mwr_ni",
            )
            .map_err(llvm_err)?;
        let mwr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        mwr_i.add_incoming(&[(&zero, mwr_leaf_hdr), (&mwr_next_i, mwr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mwr_leaf_bdy);
        self.builder.position_at_end(mwr_leaf_done);
        let _ = self.builder.build_return(None);

        // Internal node: recurse into each child in order
        self.builder.position_at_end(mwr_int_hdr);
        let mwr_int_i8 = self
            .builder
            .build_pointer_cast(mwr_node, ptr, "mwr_int_i8")
            .map_err(llvm_err)?;
        let mwr_child_count_raw = self
            .builder
            .build_load(i32, mwr_int_i8, "mwr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mwr_child_count = self
            .builder
            .build_int_z_extend(mwr_child_count_raw, i64, "mwr_cc")
            .map_err(llvm_err)?;
        let mwr_child_h = self
            .builder
            .build_int_sub(mwr_height, i64.const_int(1, false), "mwr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mwr_int_bdy);
        self.builder.position_at_end(mwr_int_bdy);
        let mwr_ci = self.builder.build_phi(i64, "mwr_ci").map_err(llvm_err)?;
        let mwr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mwr_ci.as_basic_value().into_int_value(),
                mwr_child_count,
                "mwr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mwr_done_int, mwr_leaf_done, mwr_int_child);
        self.builder.position_at_end(mwr_int_child);
        let mwr_children_base = unsafe {
            self.builder
                .build_gep(i8, mwr_int_i8, &[i64.const_int(16, false)], "mwr_cb")
                .map_err(llvm_err)?
        };
        let mwr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    mwr_children_base,
                    &[mwr_ci.as_basic_value().into_int_value()],
                    "mwr_cep",
                )
                .map_err(llvm_err)?
        };
        let mwr_child_entry = self
            .builder
            .build_load(self.child_entry_type, mwr_child_ep, "mwr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let mwr_child_ptr = self
            .builder
            .build_extract_value(mwr_child_entry, 0, "mwr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                mw_rec_fn,
                &[
                    mwr_child_ptr.into(),
                    mwr_child_h.into(),
                    mwr_fn.into(),
                    mwr_acc.into(),
                    mwr_buf_p.into(),
                    mwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mwr_int_next);
        self.builder.position_at_end(mwr_int_next);
        let mwr_next_ci = self
            .builder
            .build_int_add(
                mwr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mwr_nci",
            )
            .map_err(llvm_err)?;
        let mwr_int_next_bb = self.builder.get_insert_block().unwrap();
        mwr_ci.add_incoming(&[(&zero, mwr_int_hdr), (&mwr_next_ci, mwr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mwr_int_bdy);

        // ---- action_list_map_walk({ptr,i64,i64} list, ptr fn) -> {ptr,i64,i64} ----
        let create_fn = self.module.get_function("action_list_create").unwrap();
        let mw_fn = self.module.add_function(
            "action_list_map_walk",
            self.list_type
                .fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let mw_entry = self.context.append_basic_block(mw_fn, "entry");
        let mw_walk = self.context.append_basic_block(mw_fn, "walk");
        let mw_flush = self.context.append_basic_block(mw_fn, "flush");
        let mw_done = self.context.append_basic_block(mw_fn, "done");
        self.builder.position_at_end(mw_entry);
        let mw_list = mw_fn.get_first_param().unwrap().into_struct_value();
        let mw_fn_ptr = mw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let mw_node = self
            .builder
            .build_extract_value(mw_list, 0, "mw_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mw_len = self
            .builder
            .build_extract_value(mw_list, 1, "mw_len")
            .map_err(llvm_err)?
            .into_int_value();
        let mw_height = self
            .builder
            .build_extract_value(mw_list, 2, "mw_height")
            .map_err(llvm_err)?
            .into_int_value();
        let mw_acc = self
            .builder
            .build_alloca(self.list_type, "mw_acc")
            .map_err(llvm_err)?;
        let mw_buf_p = self
            .builder
            .build_alloca(ptr, "mw_buf_p")
            .map_err(llvm_err)?;
        let mw_buf_pos_p = self
            .builder
            .build_alloca(i64, "mw_buf_pos_p")
            .map_err(llvm_err)?;
        let mw_init = self
            .builder
            .build_call(create_fn, &[mw_len.into()], "mw_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(mw_acc, mw_init)
            .map_err(llvm_err)?;
        let mw_buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[mw_leaf_sz.into()], "mw_buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mw_buf_init_i8 = self
            .builder
            .build_pointer_cast(mw_buf_init, ptr, "mw_buf_init_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mw_buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mw_buf_p, mw_buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mw_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mw_walk);
        self.builder.position_at_end(mw_walk);
        let _ = self
            .builder
            .build_call(
                mw_rec_fn,
                &[
                    mw_node.into(),
                    mw_height.into(),
                    mw_fn_ptr.into(),
                    mw_acc.into(),
                    mw_buf_p.into(),
                    mw_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let mw_rem_pos = self
            .builder
            .build_load(i64, mw_buf_pos_p, "mw_rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mw_has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, mw_rem_pos, zero, "mw_has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mw_has_rem, mw_flush, mw_done);
        self.builder.position_at_end(mw_flush);
        let mw_rem_buf = self
            .builder
            .build_load(ptr, mw_buf_p, "mw_rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mw_rem_buf_i8 = self
            .builder
            .build_pointer_cast(mw_rem_buf, ptr, "mw_rem_buf_i8")
            .map_err(llvm_err)?;
        let mw_rem_cnt = self
            .builder
            .build_int_truncate(mw_rem_pos, i32, "mw_rem_cnt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mw_rem_buf_i8, mw_rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mw_acc.into(), mw_rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mw_done);
        self.builder.position_at_end(mw_done);
        let mw_res = self
            .builder
            .build_load(self.list_type, mw_acc, "mw_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mw_res));

        // ---- action_list_filter_walk_rec(ptr node, i64 height, ptr fn, ptr acc, ptr buf_p, ptr buf_pos_p) -> void ----
        let fw_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let fw_rec_fn = self.module.add_function(
            "action_list_filter_walk_rec",
            void.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                ],
                false,
            ),
            None,
        );
        let fwr_entry = self.context.append_basic_block(fw_rec_fn, "entry");
        let fwr_leaf_hdr = self.context.append_basic_block(fw_rec_fn, "leaf_hdr");
        let fwr_leaf_bdy = self.context.append_basic_block(fw_rec_fn, "leaf_bdy");
        let fwr_leaf_chk = self.context.append_basic_block(fw_rec_fn, "leaf_chk");
        let fwr_leaf_push = self.context.append_basic_block(fw_rec_fn, "leaf_push");
        let fwr_leaf_flush = self.context.append_basic_block(fw_rec_fn, "leaf_flush");
        let fwr_leaf_next = self.context.append_basic_block(fw_rec_fn, "leaf_next");
        let fwr_leaf_done = self.context.append_basic_block(fw_rec_fn, "leaf_done");
        let fwr_int_hdr = self.context.append_basic_block(fw_rec_fn, "int_hdr");
        let fwr_int_bdy = self.context.append_basic_block(fw_rec_fn, "int_bdy");
        let fwr_int_child = self.context.append_basic_block(fw_rec_fn, "int_child");
        let fwr_int_next = self.context.append_basic_block(fw_rec_fn, "int_next");
        let fwr_concat = self.context.append_basic_block(fw_rec_fn, "concat");
        let fwr_normal = self.context.append_basic_block(fw_rec_fn, "normal");
        self.builder.position_at_end(fwr_entry);
        let fwr_node = fw_rec_fn.get_first_param().unwrap().into_pointer_value();
        let fwr_height = fw_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let fwr_fn = fw_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let fwr_acc = fw_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let fwr_buf_p = fw_rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let fwr_buf_pos_p = fw_rec_fn.get_nth_param(5).unwrap().into_pointer_value();
        let fwr_neg1 = i64.const_int(-1i64 as u64, true);
        let fwr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, fwr_height, fwr_neg1, "fwr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_is_concat, fwr_concat, fwr_normal);
        self.builder.position_at_end(fwr_concat);
        let fwr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, fwr_node, &[i64.const_int(2, false)], "fwr_ln_p")
                .map_err(llvm_err)
        }?;
        let fwr_left_node = self
            .builder
            .build_load(ptr, fwr_ln_p, "fwr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fwr_lh_p = unsafe {
            self.builder
                .build_gep(i64, fwr_node, &[i64.const_int(4, false)], "fwr_lh_p")
                .map_err(llvm_err)
        }?;
        let fwr_left_h = self
            .builder
            .build_load(i64, fwr_lh_p, "fwr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, fwr_node, &[i64.const_int(5, false)], "fwr_rn_p")
                .map_err(llvm_err)
        }?;
        let fwr_right_node = self
            .builder
            .build_load(ptr, fwr_rn_p, "fwr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fwr_rh_p = unsafe {
            self.builder
                .build_gep(i64, fwr_node, &[i64.const_int(7, false)], "fwr_rh_p")
                .map_err(llvm_err)
        }?;
        let fwr_right_h = self
            .builder
            .build_load(i64, fwr_rh_p, "fwr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                fw_rec_fn,
                &[
                    fwr_left_node.into(),
                    fwr_left_h.into(),
                    fwr_fn.into(),
                    fwr_acc.into(),
                    fwr_buf_p.into(),
                    fwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                fw_rec_fn,
                &[
                    fwr_right_node.into(),
                    fwr_right_h.into(),
                    fwr_fn.into(),
                    fwr_acc.into(),
                    fwr_buf_p.into(),
                    fwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(fwr_normal);
        let fwr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, fwr_height, zero, "fwr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_is_leaf, fwr_leaf_hdr, fwr_int_hdr);

        self.builder.position_at_end(fwr_leaf_hdr);
        let fwr_leaf_i8 = self
            .builder
            .build_pointer_cast(fwr_node, ptr, "fwr_leaf_i8")
            .map_err(llvm_err)?;
        let fwr_count_raw = self
            .builder
            .build_load(i32, fwr_leaf_i8, "fwr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_count = self
            .builder
            .build_int_z_extend(fwr_count_raw, i64, "fwr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fwr_leaf_bdy);
        self.builder.position_at_end(fwr_leaf_bdy);
        let fwr_i = self.builder.build_phi(i64, "fwr_i").map_err(llvm_err)?;
        let fwr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fwr_i.as_basic_value().into_int_value(),
                fwr_count,
                "fwr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_done_leaf, fwr_leaf_done, fwr_leaf_chk);
        self.builder.position_at_end(fwr_leaf_chk);
        let fwr_eb = unsafe {
            self.builder
                .build_gep(i8, fwr_leaf_i8, &[i64.const_int(8, false)], "fwr_eb")
                .map_err(llvm_err)?
        };
        let fwr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    fwr_eb,
                    &[fwr_i.as_basic_value().into_int_value()],
                    "fwr_ep",
                )
                .map_err(llvm_err)?
        };
        let fwr_elem = self
            .builder
            .build_load(self.string_type, fwr_ep, "fwr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let fwr_elem_tag = self
            .builder
            .build_extract_value(fwr_elem, 0, "fwr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_pred = self
            .builder
            .build_indirect_call(lambda_fn_ty, fwr_fn, &[fwr_elem_tag.into()], "fwr_pred")
            .map_err(llvm_err)?;
        let fwr_pred_bv = fwr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("filter_walk indirect call failed")?;
        let fwr_pred_val = if fwr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(fwr_pred_bv.into_struct_value(), 0, "fwr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            fwr_pred_bv.into_int_value()
        };
        let fwr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, fwr_pred_val, zero, "fwr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_is_true, fwr_leaf_push, fwr_leaf_next);
        self.builder.position_at_end(fwr_leaf_push);
        let fwr_buf = self
            .builder
            .build_load(ptr, fwr_buf_p, "fwr_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fwr_pos = self
            .builder
            .build_load(i64, fwr_buf_pos_p, "fwr_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_buf_i8 = self
            .builder
            .build_pointer_cast(fwr_buf, ptr, "fwr_buf_i8")
            .map_err(llvm_err)?;
        let fwr_buf_eb = unsafe {
            self.builder
                .build_gep(i8, fwr_buf_i8, &[i64.const_int(8, false)], "fwr_buf_eb")
                .map_err(llvm_err)?
        };
        let fwr_buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, fwr_buf_eb, &[fwr_pos], "fwr_buf_ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(fwr_buf_ep, fwr_elem)
            .map_err(llvm_err)?;
        let fwr_pos_inc = self
            .builder
            .build_int_add(fwr_pos, i64.const_int(1, false), "fwr_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(fwr_buf_pos_p, fwr_pos_inc)
            .map_err(llvm_err)?;
        let fwr_buf_full = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                fwr_pos_inc,
                i64.const_int(64, false),
                "fwr_buf_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_buf_full, fwr_leaf_flush, fwr_leaf_next);

        self.builder.position_at_end(fwr_leaf_flush);
        let fwr_flush_cnt = i32.const_int(64, false);
        let _ = self
            .builder
            .build_store(fwr_buf_i8, fwr_flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[fwr_acc.into(), fwr_buf.into()], "")
            .map_err(llvm_err)?;
        let fwr_new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[fw_leaf_sz.into()], "fwr_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fwr_new_buf_i8 = self
            .builder
            .build_pointer_cast(fwr_new_buf, ptr, "fwr_new_buf_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fwr_new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fwr_buf_p, fwr_new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fwr_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fwr_leaf_next);

        self.builder.position_at_end(fwr_leaf_next);
        let fwr_next_i = self
            .builder
            .build_int_add(
                fwr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fwr_ni",
            )
            .map_err(llvm_err)?;
        let fwr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        fwr_i.add_incoming(&[(&zero, fwr_leaf_hdr), (&fwr_next_i, fwr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fwr_leaf_bdy);
        self.builder.position_at_end(fwr_leaf_done);
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(fwr_int_hdr);
        let fwr_int_i8 = self
            .builder
            .build_pointer_cast(fwr_node, ptr, "fwr_int_i8")
            .map_err(llvm_err)?;
        let fwr_child_count_raw = self
            .builder
            .build_load(i32, fwr_int_i8, "fwr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fwr_child_count = self
            .builder
            .build_int_z_extend(fwr_child_count_raw, i64, "fwr_cc")
            .map_err(llvm_err)?;
        let fwr_child_h = self
            .builder
            .build_int_sub(fwr_height, i64.const_int(1, false), "fwr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fwr_int_bdy);
        self.builder.position_at_end(fwr_int_bdy);
        let fwr_ci = self.builder.build_phi(i64, "fwr_ci").map_err(llvm_err)?;
        let fwr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fwr_ci.as_basic_value().into_int_value(),
                fwr_child_count,
                "fwr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fwr_done_int, fwr_leaf_done, fwr_int_child);
        self.builder.position_at_end(fwr_int_child);
        let fwr_children_base = unsafe {
            self.builder
                .build_gep(i8, fwr_int_i8, &[i64.const_int(16, false)], "fwr_cb")
                .map_err(llvm_err)?
        };
        let fwr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    fwr_children_base,
                    &[fwr_ci.as_basic_value().into_int_value()],
                    "fwr_cep",
                )
                .map_err(llvm_err)?
        };
        let fwr_child_entry = self
            .builder
            .build_load(self.child_entry_type, fwr_child_ep, "fwr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let fwr_child_ptr = self
            .builder
            .build_extract_value(fwr_child_entry, 0, "fwr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                fw_rec_fn,
                &[
                    fwr_child_ptr.into(),
                    fwr_child_h.into(),
                    fwr_fn.into(),
                    fwr_acc.into(),
                    fwr_buf_p.into(),
                    fwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fwr_int_next);
        self.builder.position_at_end(fwr_int_next);
        let fwr_next_ci = self
            .builder
            .build_int_add(
                fwr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fwr_nci",
            )
            .map_err(llvm_err)?;
        let fwr_int_next_bb = self.builder.get_insert_block().unwrap();
        fwr_ci.add_incoming(&[(&zero, fwr_int_hdr), (&fwr_next_ci, fwr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fwr_int_bdy);

        // ---- action_list_filter_walk({ptr,i64,i64} list, ptr fn) -> {ptr,i64,i64} ----
        let fw_fn = self.module.add_function(
            "action_list_filter_walk",
            self.list_type
                .fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let fw_entry = self.context.append_basic_block(fw_fn, "entry");
        let fw_walk = self.context.append_basic_block(fw_fn, "walk");
        let fw_flush = self.context.append_basic_block(fw_fn, "flush");
        let fw_done = self.context.append_basic_block(fw_fn, "done");
        self.builder.position_at_end(fw_entry);
        let fw_list = fw_fn.get_first_param().unwrap().into_struct_value();
        let fw_fn_ptr = fw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let fw_node = self
            .builder
            .build_extract_value(fw_list, 0, "fw_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fw_len = self
            .builder
            .build_extract_value(fw_list, 1, "fw_len")
            .map_err(llvm_err)?
            .into_int_value();
        let fw_height = self
            .builder
            .build_extract_value(fw_list, 2, "fw_height")
            .map_err(llvm_err)?
            .into_int_value();
        let fw_acc = self
            .builder
            .build_alloca(self.list_type, "fw_acc")
            .map_err(llvm_err)?;
        let fw_buf_p = self
            .builder
            .build_alloca(ptr, "fw_buf_p")
            .map_err(llvm_err)?;
        let fw_buf_pos_p = self
            .builder
            .build_alloca(i64, "fw_buf_pos_p")
            .map_err(llvm_err)?;
        let fw_init = self
            .builder
            .build_call(create_fn, &[fw_len.into()], "fw_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(fw_acc, fw_init)
            .map_err(llvm_err)?;
        let fw_buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[fw_leaf_sz.into()], "fw_buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fw_buf_init_i8 = self
            .builder
            .build_pointer_cast(fw_buf_init, ptr, "fw_buf_init_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fw_buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fw_buf_p, fw_buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fw_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fw_walk);
        self.builder.position_at_end(fw_walk);
        let _ = self
            .builder
            .build_call(
                fw_rec_fn,
                &[
                    fw_node.into(),
                    fw_height.into(),
                    fw_fn_ptr.into(),
                    fw_acc.into(),
                    fw_buf_p.into(),
                    fw_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let fw_rem_pos = self
            .builder
            .build_load(i64, fw_buf_pos_p, "fw_rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let fw_has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, fw_rem_pos, zero, "fw_has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fw_has_rem, fw_flush, fw_done);
        self.builder.position_at_end(fw_flush);
        let fw_rem_buf = self
            .builder
            .build_load(ptr, fw_buf_p, "fw_rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fw_rem_buf_i8 = self
            .builder
            .build_pointer_cast(fw_rem_buf, ptr, "fw_rem_buf_i8")
            .map_err(llvm_err)?;
        let fw_rem_cnt = self
            .builder
            .build_int_truncate(fw_rem_pos, i32, "fw_rem_cnt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fw_rem_buf_i8, fw_rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[fw_acc.into(), fw_rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fw_done);
        self.builder.position_at_end(fw_done);
        let fw_res = self
            .builder
            .build_load(self.list_type, fw_acc, "fw_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fw_res));
