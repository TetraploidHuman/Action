// ---- action_list_filter_map_walk_rec(ptr node, i64 height, ptr filter_fn, ptr map_fn, ptr acc, ptr buf_p, ptr buf_pos_p) -> void ----
        // Fused filter+map: in-order B-tree walk that filters each element, then maps survivors.
        let fmw_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let fmw_rec_fn = self.module.add_function(
            "action_list_filter_map_walk_rec",
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
        let fmwr_entry = self.context.append_basic_block(fmw_rec_fn, "entry");
        let fmwr_leaf_hdr = self.context.append_basic_block(fmw_rec_fn, "leaf_hdr");
        let fmwr_leaf_bdy = self.context.append_basic_block(fmw_rec_fn, "leaf_bdy");
        let fmwr_leaf_chk = self.context.append_basic_block(fmw_rec_fn, "leaf_chk");
        let fmwr_leaf_map = self.context.append_basic_block(fmw_rec_fn, "leaf_map");
        let fmwr_leaf_push = self.context.append_basic_block(fmw_rec_fn, "leaf_push");
        let fmwr_leaf_flush = self.context.append_basic_block(fmw_rec_fn, "leaf_flush");
        let fmwr_leaf_next = self.context.append_basic_block(fmw_rec_fn, "leaf_next");
        let fmwr_leaf_done = self.context.append_basic_block(fmw_rec_fn, "leaf_done");
        let fmwr_int_hdr = self.context.append_basic_block(fmw_rec_fn, "int_hdr");
        let fmwr_int_bdy = self.context.append_basic_block(fmw_rec_fn, "int_bdy");
        let fmwr_int_child = self.context.append_basic_block(fmw_rec_fn, "int_child");
        let fmwr_int_next = self.context.append_basic_block(fmw_rec_fn, "int_next");
        let fmwr_concat = self.context.append_basic_block(fmw_rec_fn, "concat");
        let fmwr_normal = self.context.append_basic_block(fmw_rec_fn, "normal");
        self.builder.position_at_end(fmwr_entry);
        let fmwr_node = fmw_rec_fn.get_first_param().unwrap().into_pointer_value();
        let fmwr_height = fmw_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let fmwr_filter_fn = fmw_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let fmwr_map_fn = fmw_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let fmwr_acc = fmw_rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let fmwr_buf_p = fmw_rec_fn.get_nth_param(5).unwrap().into_pointer_value();
        let fmwr_buf_pos_p = fmw_rec_fn.get_nth_param(6).unwrap().into_pointer_value();
        let fmwr_neg1 = i64.const_int(-1i64 as u64, true);
        let fmwr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, fmwr_height, fmwr_neg1, "fmwr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fmwr_is_concat, fmwr_concat, fmwr_normal);
        self.builder.position_at_end(fmwr_concat);
        let fmwr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, fmwr_node, &[i64.const_int(2, false)], "fmwr_ln_p")
                .map_err(llvm_err)
        }?;
        let fmwr_left_node = self
            .builder
            .build_load(ptr, fmwr_ln_p, "fmwr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fmwr_lh_p = unsafe {
            self.builder
                .build_gep(i64, fmwr_node, &[i64.const_int(4, false)], "fmwr_lh_p")
                .map_err(llvm_err)
        }?;
        let fmwr_left_h = self
            .builder
            .build_load(i64, fmwr_lh_p, "fmwr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let fmwr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, fmwr_node, &[i64.const_int(5, false)], "fmwr_rn_p")
                .map_err(llvm_err)
        }?;
        let fmwr_right_node = self
            .builder
            .build_load(ptr, fmwr_rn_p, "fmwr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fmwr_rh_p = unsafe {
            self.builder
                .build_gep(i64, fmwr_node, &[i64.const_int(7, false)], "fmwr_rh_p")
                .map_err(llvm_err)
        }?;
        let fmwr_right_h = self
            .builder
            .build_load(i64, fmwr_rh_p, "fmwr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                fmw_rec_fn,
                &[
                    fmwr_left_node.into(),
                    fmwr_left_h.into(),
                    fmwr_filter_fn.into(),
                    fmwr_map_fn.into(),
                    fmwr_acc.into(),
                    fmwr_buf_p.into(),
                    fmwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                fmw_rec_fn,
                &[
                    fmwr_right_node.into(),
                    fmwr_right_h.into(),
                    fmwr_filter_fn.into(),
                    fmwr_map_fn.into(),
                    fmwr_acc.into(),
                    fmwr_buf_p.into(),
                    fmwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(fmwr_normal);
        let fmwr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, fmwr_height, zero, "fmwr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fmwr_is_leaf, fmwr_leaf_hdr, fmwr_int_hdr);

        self.builder.position_at_end(fmwr_leaf_hdr);
        let fmwr_leaf_i8 = self
            .builder
            .build_pointer_cast(fmwr_node, ptr, "fmwr_leaf_i8")
            .map_err(llvm_err)?;
        let fmwr_count_raw = self
            .builder
            .build_load(i32, fmwr_leaf_i8, "fmwr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fmwr_count = self
            .builder
            .build_int_z_extend(fmwr_count_raw, i64, "fmwr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fmwr_leaf_bdy);
        self.builder.position_at_end(fmwr_leaf_bdy);
        let fmwr_i = self.builder.build_phi(i64, "fmwr_i").map_err(llvm_err)?;
        let fmwr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fmwr_i.as_basic_value().into_int_value(),
                fmwr_count,
                "fmwr_done",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(fmwr_done_leaf, fmwr_leaf_done, fmwr_leaf_chk);
        self.builder.position_at_end(fmwr_leaf_chk);
        let fmwr_eb = unsafe {
            self.builder
                .build_gep(i8, fmwr_leaf_i8, &[i64.const_int(8, false)], "fmwr_eb")
                .map_err(llvm_err)?
        };
        let fmwr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    fmwr_eb,
                    &[fmwr_i.as_basic_value().into_int_value()],
                    "fmwr_ep",
                )
                .map_err(llvm_err)?
        };
        let fmwr_elem = self
            .builder
            .build_load(self.string_type, fmwr_ep, "fmwr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let fmwr_elem_tag = self
            .builder
            .build_extract_value(fmwr_elem, 0, "fmwr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let fmwr_pred = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                fmwr_filter_fn,
                &[fmwr_elem_tag.into()],
                "fmwr_pred",
            )
            .map_err(llvm_err)?;
        let fmwr_pred_bv = fmwr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("filter_map_walk filter call failed")?;
        let fmwr_pred_val = if fmwr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(fmwr_pred_bv.into_struct_value(), 0, "fmwr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            fmwr_pred_bv.into_int_value()
        };
        let fmwr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, fmwr_pred_val, zero, "fmwr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fmwr_is_true, fmwr_leaf_map, fmwr_leaf_next);
        self.builder.position_at_end(fmwr_leaf_map);
        let fmwr_mapped_call = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                fmwr_map_fn,
                &[fmwr_elem_tag.into()],
                "fmwr_map",
            )
            .map_err(llvm_err)?;
        let fmwr_mapped_bv = fmwr_mapped_call
            .try_as_basic_value()
            .basic()
            .ok_or("filter_map_walk map call failed")?;
        let _ = self.builder.build_unconditional_branch(fmwr_leaf_push);
        self.builder.position_at_end(fmwr_leaf_push);
        let fmwr_buf = self
            .builder
            .build_load(ptr, fmwr_buf_p, "fmwr_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fmwr_pos = self
            .builder
            .build_load(i64, fmwr_buf_pos_p, "fmwr_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let fmwr_buf_i8 = self
            .builder
            .build_pointer_cast(fmwr_buf, ptr, "fmwr_buf_i8")
            .map_err(llvm_err)?;
        let fmwr_buf_eb = unsafe {
            self.builder
                .build_gep(i8, fmwr_buf_i8, &[i64.const_int(8, false)], "fmwr_buf_eb")
                .map_err(llvm_err)?
        };
        let fmwr_buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, fmwr_buf_eb, &[fmwr_pos], "fmwr_buf_ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(fmwr_buf_ep, fmwr_mapped_bv)
            .map_err(llvm_err)?;
        let fmwr_pos_inc = self
            .builder
            .build_int_add(fmwr_pos, i64.const_int(1, false), "fmwr_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(fmwr_buf_pos_p, fmwr_pos_inc)
            .map_err(llvm_err)?;
        let fmwr_buf_full = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                fmwr_pos_inc,
                i64.const_int(64, false),
                "fmwr_buf_full",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(fmwr_buf_full, fmwr_leaf_flush, fmwr_leaf_next);

        self.builder.position_at_end(fmwr_leaf_flush);
        let fmwr_flush_cnt = i32.const_int(64, false);
        let _ = self
            .builder
            .build_store(fmwr_buf_i8, fmwr_flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[fmwr_acc.into(), fmwr_buf.into()], "")
            .map_err(llvm_err)?;
        let fmwr_new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[fmw_leaf_sz.into()], "fmwr_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fmwr_new_buf_i8 = self
            .builder
            .build_pointer_cast(fmwr_new_buf, ptr, "fmwr_new_buf_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fmwr_new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fmwr_buf_p, fmwr_new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fmwr_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fmwr_leaf_next);

        self.builder.position_at_end(fmwr_leaf_next);
        let fmwr_next_i = self
            .builder
            .build_int_add(
                fmwr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fmwr_ni",
            )
            .map_err(llvm_err)?;
        let fmwr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        fmwr_i.add_incoming(&[(&zero, fmwr_leaf_hdr), (&fmwr_next_i, fmwr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fmwr_leaf_bdy);
        self.builder.position_at_end(fmwr_leaf_done);
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(fmwr_int_hdr);
        let fmwr_int_i8 = self
            .builder
            .build_pointer_cast(fmwr_node, ptr, "fmwr_int_i8")
            .map_err(llvm_err)?;
        let fmwr_child_count_raw = self
            .builder
            .build_load(i32, fmwr_int_i8, "fmwr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fmwr_child_count = self
            .builder
            .build_int_z_extend(fmwr_child_count_raw, i64, "fmwr_cc")
            .map_err(llvm_err)?;
        let fmwr_child_h = self
            .builder
            .build_int_sub(fmwr_height, i64.const_int(1, false), "fmwr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fmwr_int_bdy);
        self.builder.position_at_end(fmwr_int_bdy);
        let fmwr_ci = self.builder.build_phi(i64, "fmwr_ci").map_err(llvm_err)?;
        let fmwr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fmwr_ci.as_basic_value().into_int_value(),
                fmwr_child_count,
                "fmwr_done_int",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(fmwr_done_int, fmwr_leaf_done, fmwr_int_child);
        self.builder.position_at_end(fmwr_int_child);
        let fmwr_children_base = unsafe {
            self.builder
                .build_gep(i8, fmwr_int_i8, &[i64.const_int(16, false)], "fmwr_cb")
                .map_err(llvm_err)?
        };
        let fmwr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    fmwr_children_base,
                    &[fmwr_ci.as_basic_value().into_int_value()],
                    "fmwr_cep",
                )
                .map_err(llvm_err)?
        };
        let fmwr_child_entry = self
            .builder
            .build_load(self.child_entry_type, fmwr_child_ep, "fmwr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let fmwr_child_ptr = self
            .builder
            .build_extract_value(fmwr_child_entry, 0, "fmwr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                fmw_rec_fn,
                &[
                    fmwr_child_ptr.into(),
                    fmwr_child_h.into(),
                    fmwr_filter_fn.into(),
                    fmwr_map_fn.into(),
                    fmwr_acc.into(),
                    fmwr_buf_p.into(),
                    fmwr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fmwr_int_next);
        self.builder.position_at_end(fmwr_int_next);
        let fmwr_next_ci = self
            .builder
            .build_int_add(
                fmwr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fmwr_nci",
            )
            .map_err(llvm_err)?;
        let fmwr_int_next_bb = self.builder.get_insert_block().unwrap();
        fmwr_ci.add_incoming(&[(&zero, fmwr_int_hdr), (&fmwr_next_ci, fmwr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fmwr_int_bdy);

        // ---- action_list_filter_map_walk({ptr,i64,i64} list, ptr filter_fn, ptr map_fn) -> {ptr,i64,i64} ----
        let fmw_fn = self.module.add_function(
            "action_list_filter_map_walk",
            self.list_type
                .fn_type(&[self.list_type.into(), ptr.into(), ptr.into()], false),
            None,
        );
        let fmw_entry = self.context.append_basic_block(fmw_fn, "entry");
        let fmw_walk = self.context.append_basic_block(fmw_fn, "walk");
        let fmw_flush = self.context.append_basic_block(fmw_fn, "flush");
        let fmw_done = self.context.append_basic_block(fmw_fn, "done");
        self.builder.position_at_end(fmw_entry);
        let fmw_list = fmw_fn.get_first_param().unwrap().into_struct_value();
        let fmw_filter_fn_ptr = fmw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let fmw_map_fn_ptr = fmw_fn.get_nth_param(2).unwrap().into_pointer_value();
        let fmw_node = self
            .builder
            .build_extract_value(fmw_list, 0, "fmw_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fmw_len = self
            .builder
            .build_extract_value(fmw_list, 1, "fmw_len")
            .map_err(llvm_err)?
            .into_int_value();
        let fmw_height = self
            .builder
            .build_extract_value(fmw_list, 2, "fmw_height")
            .map_err(llvm_err)?
            .into_int_value();
        let fmw_acc = self
            .builder
            .build_alloca(self.list_type, "fmw_acc")
            .map_err(llvm_err)?;
        let fmw_buf_p = self
            .builder
            .build_alloca(ptr, "fmw_buf_p")
            .map_err(llvm_err)?;
        let fmw_buf_pos_p = self
            .builder
            .build_alloca(i64, "fmw_buf_pos_p")
            .map_err(llvm_err)?;
        let fmw_init = self
            .builder
            .build_call(create_fn, &[fmw_len.into()], "fmw_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(fmw_acc, fmw_init)
            .map_err(llvm_err)?;
        let fmw_buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[fmw_leaf_sz.into()], "fmw_buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fmw_buf_init_i8 = self
            .builder
            .build_pointer_cast(fmw_buf_init, ptr, "fmw_buf_init_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fmw_buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fmw_buf_p, fmw_buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(fmw_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fmw_walk);
        self.builder.position_at_end(fmw_walk);
        let _ = self
            .builder
            .build_call(
                fmw_rec_fn,
                &[
                    fmw_node.into(),
                    fmw_height.into(),
                    fmw_filter_fn_ptr.into(),
                    fmw_map_fn_ptr.into(),
                    fmw_acc.into(),
                    fmw_buf_p.into(),
                    fmw_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let fmw_rem_pos = self
            .builder
            .build_load(i64, fmw_buf_pos_p, "fmw_rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let fmw_has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, fmw_rem_pos, zero, "fmw_has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fmw_has_rem, fmw_flush, fmw_done);
        self.builder.position_at_end(fmw_flush);
        let fmw_rem_buf = self
            .builder
            .build_load(ptr, fmw_buf_p, "fmw_rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fmw_rem_buf_i8 = self
            .builder
            .build_pointer_cast(fmw_rem_buf, ptr, "fmw_rem_buf_i8")
            .map_err(llvm_err)?;
        let fmw_rem_cnt = self
            .builder
            .build_int_truncate(fmw_rem_pos, i32, "fmw_rem_cnt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fmw_rem_buf_i8, fmw_rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[fmw_acc.into(), fmw_rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fmw_done);
        self.builder.position_at_end(fmw_done);
        let fmw_res = self
            .builder
            .build_load(self.list_type, fmw_acc, "fmw_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fmw_res));

        // ---- Phase 3: take_while / map_take_while B-tree walks ----
