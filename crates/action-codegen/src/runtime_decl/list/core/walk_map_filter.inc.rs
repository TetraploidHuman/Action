// ---- action_list_map_filter_walk_rec(ptr node, i64 height, ptr map_fn, ptr filter_fn, ptr acc, ptr buf_p, ptr buf_pos_p) -> void ----
        // Fused map+filter: in-order B-tree walk that applies map_fn to each element,
        // then calls filter_fn on the mapped value, pushing only passing elements.
        let mfw_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let mfw_rec_fn = self.module.add_function(
            "action_list_map_filter_walk_rec",
            void.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                ],
                false,
            ),
            None,
        );
        let mfwr_entry = self.context.append_basic_block(mfw_rec_fn, "entry");
        let mfwr_leaf_hdr = self.context.append_basic_block(mfw_rec_fn, "leaf_hdr");
        let mfwr_leaf_bdy = self.context.append_basic_block(mfw_rec_fn, "leaf_bdy");
        let mfwr_leaf_chk = self.context.append_basic_block(mfw_rec_fn, "leaf_chk");
        let mfwr_leaf_push = self.context.append_basic_block(mfw_rec_fn, "leaf_push");
        let mfwr_leaf_flush = self.context.append_basic_block(mfw_rec_fn, "leaf_flush");
        let mfwr_leaf_next = self.context.append_basic_block(mfw_rec_fn, "leaf_next");
        let mfwr_leaf_done = self.context.append_basic_block(mfw_rec_fn, "leaf_done");
        let mfwr_int_hdr = self.context.append_basic_block(mfw_rec_fn, "int_hdr");
        let mfwr_int_bdy = self.context.append_basic_block(mfw_rec_fn, "int_bdy");
        let mfwr_int_child = self.context.append_basic_block(mfw_rec_fn, "int_child");
        let mfwr_int_next = self.context.append_basic_block(mfw_rec_fn, "int_next");
        let mfwr_concat = self.context.append_basic_block(mfw_rec_fn, "concat");
        let mfwr_normal = self.context.append_basic_block(mfw_rec_fn, "normal");
        self.builder.position_at_end(mfwr_entry);
        let mfwr_node = mfw_rec_fn.get_first_param().unwrap().into_pointer_value();
        let mfwr_height = mfw_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let mfwr_map_fn = mfw_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let mfwr_filter_fn = mfw_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let mfwr_acc = mfw_rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let mfwr_buf_p = mfw_rec_fn.get_nth_param(5).unwrap().into_pointer_value();
        let mfwr_buf_pos_p = mfw_rec_fn.get_nth_param(6).unwrap().into_pointer_value();
        let mfwr_neg1 = i64.const_int(-1i64 as u64, true);
        let mfwr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, mfwr_height, mfwr_neg1, "mfwr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mfwr_is_concat, mfwr_concat, mfwr_normal);
        self.builder.position_at_end(mfwr_concat);
        let mfwr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, mfwr_node, &[i64.const_int(2, false)], "mfwr_ln_p")
                .map_err(llvm_err)
        }?;
        let mfwr_left_node = self
            .builder
            .build_load(ptr, mfwr_ln_p, "mfwr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfwr_lh_p = unsafe {
            self.builder
                .build_gep(i64, mfwr_node, &[i64.const_int(4, false)], "mfwr_lh_p")
                .map_err(llvm_err)
        }?;
        let mfwr_left_h = self
            .builder
            .build_load(i64, mfwr_lh_p, "mfwr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let mfwr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, mfwr_node, &[i64.const_int(5, false)], "mfwr_rn_p")
                .map_err(llvm_err)
        }?;
        let mfwr_right_node = self
            .builder
            .build_load(ptr, mfwr_rn_p, "mfwr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfwr_rh_p = unsafe {
            self.builder
                .build_gep(i64, mfwr_node, &[i64.const_int(7, false)], "mfwr_rh_p")
                .map_err(llvm_err)
        }?;
        let mfwr_right_h = self
            .builder
            .build_load(i64, mfwr_rh_p, "mfwr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                mfw_rec_fn,
                &[
                    mfwr_left_node.into(),
                    mfwr_left_h.into(),
                    mfwr_map_fn.into(),
                    mfwr_filter_fn.into(),
                    mfwr_acc.into(),
                    mfwr_buf_p.into(),
                    mfwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                mfw_rec_fn,
                &[
                    mfwr_right_node.into(),
                    mfwr_right_h.into(),
                    mfwr_map_fn.into(),
                    mfwr_filter_fn.into(),
                    mfwr_acc.into(),
                    mfwr_buf_p.into(),
                    mfwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(mfwr_normal);
        let mfwr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, mfwr_height, zero, "mfwr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mfwr_is_leaf, mfwr_leaf_hdr, mfwr_int_hdr);

        // Leaf scan: apply map, then filter, push only passing elements
        self.builder.position_at_end(mfwr_leaf_hdr);
        let mfwr_leaf_i8 = self
            .builder
            .build_pointer_cast(mfwr_node, ptr, "mfwr_leaf_i8")
            .map_err(llvm_err)?;
        let mfwr_count_raw = self
            .builder
            .build_load(i32, mfwr_leaf_i8, "mfwr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mfwr_count = self
            .builder
            .build_int_z_extend(mfwr_count_raw, i64, "mfwr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfwr_leaf_bdy);
        self.builder.position_at_end(mfwr_leaf_bdy);
        let mfwr_i = self.builder.build_phi(i64, "mfwr_i").map_err(llvm_err)?;
        let mfwr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mfwr_i.as_basic_value().into_int_value(),
                mfwr_count,
                "mfwr_done",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(mfwr_done_leaf, mfwr_leaf_done, mfwr_leaf_chk);
        self.builder.position_at_end(mfwr_leaf_chk);
        let mfwr_eb = unsafe {
            self.builder
                .build_gep(i8, mfwr_leaf_i8, &[i64.const_int(8, false)], "mfwr_eb")
                .map_err(llvm_err)?
        };
        let mfwr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    mfwr_eb,
                    &[mfwr_i.as_basic_value().into_int_value()],
                    "mfwr_ep",
                )
                .map_err(llvm_err)?
        };
        let mfwr_elem = self
            .builder
            .build_load(self.string_type, mfwr_ep, "mfwr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let mfwr_elem_tag = self
            .builder
            .build_extract_value(mfwr_elem, 0, "mfwr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        // Apply map function
        let mfwr_mapped_call = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                mfwr_map_fn,
                &[mfwr_elem_tag.into()],
                "mfwr_map",
            )
            .map_err(llvm_err)?;
        let mfwr_mapped_bv = mfwr_mapped_call
            .try_as_basic_value()
            .basic()
            .ok_or("map_filter_walk map call failed")?;
        // Extract tag from mapped value for filter predicate
        let mfwr_mapped_struct = mfwr_mapped_bv.into_struct_value();
        let mfwr_mapped_tag = self
            .builder
            .build_extract_value(mfwr_mapped_struct, 0, "mfwr_mt")
            .map_err(llvm_err)?
            .into_int_value();
        // Apply filter function on mapped value
        let mfwr_pred = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                mfwr_filter_fn,
                &[mfwr_mapped_tag.into()],
                "mfwr_pred",
            )
            .map_err(llvm_err)?;
        let mfwr_pred_bv = mfwr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("map_filter_walk filter call failed")?;
        let mfwr_pred_val = if mfwr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(mfwr_pred_bv.into_struct_value(), 0, "mfwr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            mfwr_pred_bv.into_int_value()
        };
        let mfwr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, mfwr_pred_val, zero, "mfwr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mfwr_is_true, mfwr_leaf_push, mfwr_leaf_next);
        // Push mapped value to buffer
        self.builder.position_at_end(mfwr_leaf_push);
        let mfwr_buf = self
            .builder
            .build_load(ptr, mfwr_buf_p, "mfwr_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfwr_pos = self
            .builder
            .build_load(i64, mfwr_buf_pos_p, "mfwr_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mfwr_buf_i8 = self
            .builder
            .build_pointer_cast(mfwr_buf, ptr, "mfwr_buf_i8")
            .map_err(llvm_err)?;
        let mfwr_buf_eb = unsafe {
            self.builder
                .build_gep(i8, mfwr_buf_i8, &[i64.const_int(8, false)], "mfwr_buf_eb")
                .map_err(llvm_err)?
        };
        let mfwr_buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, mfwr_buf_eb, &[mfwr_pos], "mfwr_buf_ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(mfwr_buf_ep, mfwr_mapped_bv)
            .map_err(llvm_err)?;
        let mfwr_pos_inc = self
            .builder
            .build_int_add(mfwr_pos, i64.const_int(1, false), "mfwr_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfwr_buf_pos_p, mfwr_pos_inc)
            .map_err(llvm_err)?;
        let mfwr_buf_full = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                mfwr_pos_inc,
                i64.const_int(64, false),
                "mfwr_buf_full",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(mfwr_buf_full, mfwr_leaf_flush, mfwr_leaf_next);

        self.builder.position_at_end(mfwr_leaf_flush);
        let mfwr_flush_cnt = i32.const_int(64, false);
        let _ = self
            .builder
            .build_store(mfwr_buf_i8, mfwr_flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mfwr_acc.into(), mfwr_buf.into()], "")
            .map_err(llvm_err)?;
        let mfwr_new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[mfw_leaf_sz.into()], "mfwr_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mfwr_new_buf_i8 = self
            .builder
            .build_pointer_cast(mfwr_new_buf, ptr, "mfwr_new_buf_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mfwr_new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfwr_buf_p, mfwr_new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfwr_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfwr_leaf_next);

        self.builder.position_at_end(mfwr_leaf_next);
        let mfwr_next_i = self
            .builder
            .build_int_add(
                mfwr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mfwr_ni",
            )
            .map_err(llvm_err)?;
        let mfwr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        mfwr_i.add_incoming(&[(&zero, mfwr_leaf_hdr), (&mfwr_next_i, mfwr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mfwr_leaf_bdy);
        self.builder.position_at_end(mfwr_leaf_done);
        let _ = self.builder.build_return(None);

        // Internal node: recurse into each child in order
        self.builder.position_at_end(mfwr_int_hdr);
        let mfwr_int_i8 = self
            .builder
            .build_pointer_cast(mfwr_node, ptr, "mfwr_int_i8")
            .map_err(llvm_err)?;
        let mfwr_child_count_raw = self
            .builder
            .build_load(i32, mfwr_int_i8, "mfwr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mfwr_child_count = self
            .builder
            .build_int_z_extend(mfwr_child_count_raw, i64, "mfwr_cc")
            .map_err(llvm_err)?;
        let mfwr_child_h = self
            .builder
            .build_int_sub(mfwr_height, i64.const_int(1, false), "mfwr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfwr_int_bdy);
        self.builder.position_at_end(mfwr_int_bdy);
        let mfwr_ci = self.builder.build_phi(i64, "mfwr_ci").map_err(llvm_err)?;
        let mfwr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mfwr_ci.as_basic_value().into_int_value(),
                mfwr_child_count,
                "mfwr_done_int",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(mfwr_done_int, mfwr_leaf_done, mfwr_int_child);
        self.builder.position_at_end(mfwr_int_child);
        let mfwr_children_base = unsafe {
            self.builder
                .build_gep(i8, mfwr_int_i8, &[i64.const_int(16, false)], "mfwr_cb")
                .map_err(llvm_err)?
        };
        let mfwr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    mfwr_children_base,
                    &[mfwr_ci.as_basic_value().into_int_value()],
                    "mfwr_cep",
                )
                .map_err(llvm_err)?
        };
        let mfwr_child_entry = self
            .builder
            .build_load(self.child_entry_type, mfwr_child_ep, "mfwr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let mfwr_child_ptr = self
            .builder
            .build_extract_value(mfwr_child_entry, 0, "mfwr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                mfw_rec_fn,
                &[
                    mfwr_child_ptr.into(),
                    mfwr_child_h.into(),
                    mfwr_map_fn.into(),
                    mfwr_filter_fn.into(),
                    mfwr_acc.into(),
                    mfwr_buf_p.into(),
                    mfwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfwr_int_next);
        self.builder.position_at_end(mfwr_int_next);
        let mfwr_next_ci = self
            .builder
            .build_int_add(
                mfwr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mfwr_nci",
            )
            .map_err(llvm_err)?;
        let mfwr_int_next_bb = self.builder.get_insert_block().unwrap();
        mfwr_ci.add_incoming(&[(&zero, mfwr_int_hdr), (&mfwr_next_ci, mfwr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mfwr_int_bdy);

        // ---- action_list_map_filter_walk({ptr,i64,i64} list, ptr map_fn, ptr filter_fn) -> {ptr,i64,i64} ----
        let mfw_fn = self.module.add_function(
            "action_list_map_filter_walk",
            self.list_type
                .fn_type(&[self.list_type.into(), ptr.into(), ptr.into()], false),
            None,
        );
        let mfw_entry = self.context.append_basic_block(mfw_fn, "entry");
        let mfw_walk = self.context.append_basic_block(mfw_fn, "walk");
        let mfw_flush = self.context.append_basic_block(mfw_fn, "flush");
        let mfw_done = self.context.append_basic_block(mfw_fn, "done");
        self.builder.position_at_end(mfw_entry);
        let mfw_list = mfw_fn.get_first_param().unwrap().into_struct_value();
        let mfw_map_fn_ptr = mfw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let mfw_filter_fn_ptr = mfw_fn.get_nth_param(2).unwrap().into_pointer_value();
        let mfw_node = self
            .builder
            .build_extract_value(mfw_list, 0, "mfw_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfw_len = self
            .builder
            .build_extract_value(mfw_list, 1, "mfw_len")
            .map_err(llvm_err)?
            .into_int_value();
        let mfw_height = self
            .builder
            .build_extract_value(mfw_list, 2, "mfw_height")
            .map_err(llvm_err)?
            .into_int_value();
        let mfw_acc = self
            .builder
            .build_alloca(self.list_type, "mfw_acc")
            .map_err(llvm_err)?;
        let mfw_buf_p = self
            .builder
            .build_alloca(ptr, "mfw_buf_p")
            .map_err(llvm_err)?;
        let mfw_buf_pos_p = self
            .builder
            .build_alloca(i64, "mfw_buf_pos_p")
            .map_err(llvm_err)?;
        let mfw_init = self
            .builder
            .build_call(create_fn, &[mfw_len.into()], "mfw_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(mfw_acc, mfw_init)
            .map_err(llvm_err)?;
        let mfw_buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[mfw_leaf_sz.into()], "mfw_buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mfw_buf_init_i8 = self
            .builder
            .build_pointer_cast(mfw_buf_init, ptr, "mfw_buf_init_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mfw_buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfw_buf_p, mfw_buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfw_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfw_walk);
        self.builder.position_at_end(mfw_walk);
        let _ = self
            .builder
            .build_call(
                mfw_rec_fn,
                &[
                    mfw_node.into(),
                    mfw_height.into(),
                    mfw_map_fn_ptr.into(),
                    mfw_filter_fn_ptr.into(),
                    mfw_acc.into(),
                    mfw_buf_p.into(),
                    mfw_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let mfw_rem_pos = self
            .builder
            .build_load(i64, mfw_buf_pos_p, "mfw_rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mfw_has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, mfw_rem_pos, zero, "mfw_has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mfw_has_rem, mfw_flush, mfw_done);
        self.builder.position_at_end(mfw_flush);
        let mfw_rem_buf = self
            .builder
            .build_load(ptr, mfw_buf_p, "mfw_rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfw_rem_buf_i8 = self
            .builder
            .build_pointer_cast(mfw_rem_buf, ptr, "mfw_rem_buf_i8")
            .map_err(llvm_err)?;
        let mfw_rem_cnt = self
            .builder
            .build_int_truncate(mfw_rem_pos, i32, "mfw_rem_cnt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mfw_rem_buf_i8, mfw_rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mfw_acc.into(), mfw_rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfw_done);
        self.builder.position_at_end(mfw_done);
        let mfw_res = self
            .builder
            .build_load(self.list_type, mfw_acc, "mfw_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mfw_res));
