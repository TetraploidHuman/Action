// ---- action_list_take_while_walk_rec(ptr node, i64 height, ptr fn, ptr acc, ptr buf_p, ptr buf_pos_p) -> void ----
        let fw_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let tw_rec_fn = self.module.add_function(
            "action_list_take_while_walk_rec",
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
        let twr_entry = self.context.append_basic_block(tw_rec_fn, "entry");
        let twr_leaf_hdr = self.context.append_basic_block(tw_rec_fn, "leaf_hdr");
        let twr_leaf_bdy = self.context.append_basic_block(tw_rec_fn, "leaf_bdy");
        let twr_leaf_chk = self.context.append_basic_block(tw_rec_fn, "leaf_chk");
        let twr_leaf_push = self.context.append_basic_block(tw_rec_fn, "leaf_push");
        let twr_leaf_stop = self.context.append_basic_block(tw_rec_fn, "leaf_stop");
        let twr_leaf_flush = self.context.append_basic_block(tw_rec_fn, "leaf_flush");
        let twr_leaf_next = self.context.append_basic_block(tw_rec_fn, "leaf_next");
        let twr_leaf_done = self.context.append_basic_block(tw_rec_fn, "leaf_done");
        let twr_int_hdr = self.context.append_basic_block(tw_rec_fn, "int_hdr");
        let twr_int_bdy = self.context.append_basic_block(tw_rec_fn, "int_bdy");
        let twr_int_child = self.context.append_basic_block(tw_rec_fn, "int_child");
        let twr_int_next = self.context.append_basic_block(tw_rec_fn, "int_next");
        let twr_concat = self.context.append_basic_block(tw_rec_fn, "concat");
        let twr_normal = self.context.append_basic_block(tw_rec_fn, "normal");
        self.builder.position_at_end(twr_entry);
        let twr_node = tw_rec_fn.get_first_param().unwrap().into_pointer_value();
        let twr_height = tw_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let twr_fn = tw_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let twr_acc = tw_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let twr_buf_p = tw_rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let twr_buf_pos_p = tw_rec_fn.get_nth_param(5).unwrap().into_pointer_value();
        let twr_stopped_p = tw_rec_fn.get_nth_param(6).unwrap().into_pointer_value();
        let twr_neg1 = i64.const_int(-1i64 as u64, true);
        let twr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, twr_height, twr_neg1, "twr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(twr_is_concat, twr_concat, twr_normal);
        self.builder.position_at_end(twr_concat);
        let twr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, twr_node, &[i64.const_int(2, false)], "twr_ln_p")
                .map_err(llvm_err)
        }?;
        let twr_left_node = self
            .builder
            .build_load(ptr, twr_ln_p, "twr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let twr_lh_p = unsafe {
            self.builder
                .build_gep(i64, twr_node, &[i64.const_int(4, false)], "twr_lh_p")
                .map_err(llvm_err)
        }?;
        let twr_left_h = self
            .builder
            .build_load(i64, twr_lh_p, "twr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let twr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, twr_node, &[i64.const_int(5, false)], "twr_rn_p")
                .map_err(llvm_err)
        }?;
        let twr_right_node = self
            .builder
            .build_load(ptr, twr_rn_p, "twr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let twr_rh_p = unsafe {
            self.builder
                .build_gep(i64, twr_node, &[i64.const_int(7, false)], "twr_rh_p")
                .map_err(llvm_err)
        }?;
        let twr_right_h = self
            .builder
            .build_load(i64, twr_rh_p, "twr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                tw_rec_fn,
                &[
                    twr_left_node.into(),
                    twr_left_h.into(),
                    twr_fn.into(),
                    twr_acc.into(),
                    twr_buf_p.into(),
                    twr_buf_pos_p.into(),
                    twr_stopped_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                tw_rec_fn,
                &[
                    twr_right_node.into(),
                    twr_right_h.into(),
                    twr_fn.into(),
                    twr_acc.into(),
                    twr_buf_p.into(),
                    twr_buf_pos_p.into(),
                    twr_stopped_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(twr_normal);
        let twr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, twr_height, zero, "twr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(twr_is_leaf, twr_leaf_hdr, twr_int_hdr);

        self.builder.position_at_end(twr_leaf_hdr);
        let twr_leaf_i8 = self
            .builder
            .build_pointer_cast(twr_node, ptr, "twr_leaf_i8")
            .map_err(llvm_err)?;
        let twr_count_raw = self
            .builder
            .build_load(i32, twr_leaf_i8, "twr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let twr_count = self
            .builder
            .build_int_z_extend(twr_count_raw, i64, "twr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(twr_leaf_bdy);
        self.builder.position_at_end(twr_leaf_bdy);
        let twr_i = self.builder.build_phi(i64, "twr_i").map_err(llvm_err)?;
        let twr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                twr_i.as_basic_value().into_int_value(),
                twr_count,
                "twr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(twr_done_leaf, twr_leaf_done, twr_leaf_chk);
        self.builder.position_at_end(twr_leaf_chk);
        let twr_eb = unsafe {
            self.builder
                .build_gep(i8, twr_leaf_i8, &[i64.const_int(8, false)], "twr_eb")
                .map_err(llvm_err)?
        };
        let twr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    twr_eb,
                    &[twr_i.as_basic_value().into_int_value()],
                    "twr_ep",
                )
                .map_err(llvm_err)?
        };
        let twr_elem = self
            .builder
            .build_load(self.string_type, twr_ep, "twr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let twr_elem_tag = self
            .builder
            .build_extract_value(twr_elem, 0, "twr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let twr_pred = self
            .builder
            .build_indirect_call(lambda_fn_ty, twr_fn, &[twr_elem_tag.into()], "twr_pred")
            .map_err(llvm_err)?;
        let twr_pred_bv = twr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("filter_walk indirect call failed")?;
        let twr_pred_val = if twr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(twr_pred_bv.into_struct_value(), 0, "twr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            twr_pred_bv.into_int_value()
        };
        let twr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, twr_pred_val, zero, "twr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(twr_is_true, twr_leaf_push, twr_leaf_stop);
        self.builder.position_at_end(twr_leaf_push);
        let twr_buf = self
            .builder
            .build_load(ptr, twr_buf_p, "twr_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let twr_pos = self
            .builder
            .build_load(i64, twr_buf_pos_p, "twr_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let twr_buf_i8 = self
            .builder
            .build_pointer_cast(twr_buf, ptr, "twr_buf_i8")
            .map_err(llvm_err)?;
        let twr_buf_eb = unsafe {
            self.builder
                .build_gep(i8, twr_buf_i8, &[i64.const_int(8, false)], "twr_buf_eb")
                .map_err(llvm_err)?
        };
        let twr_buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, twr_buf_eb, &[twr_pos], "twr_buf_ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(twr_buf_ep, twr_elem)
            .map_err(llvm_err)?;
        let twr_pos_inc = self
            .builder
            .build_int_add(twr_pos, i64.const_int(1, false), "twr_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(twr_buf_pos_p, twr_pos_inc)
            .map_err(llvm_err)?;
        let twr_buf_full = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                twr_pos_inc,
                i64.const_int(64, false),
                "twr_buf_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(twr_buf_full, twr_leaf_flush, twr_leaf_next);

        self.builder.position_at_end(twr_leaf_flush);
        let twr_flush_cnt = i32.const_int(64, false);
        let _ = self
            .builder
            .build_store(twr_buf_i8, twr_flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[twr_acc.into(), twr_buf.into()], "")
            .map_err(llvm_err)?;
        let twr_new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[fw_leaf_sz.into()], "twr_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let twr_new_buf_i8 = self
            .builder
            .build_pointer_cast(twr_new_buf, ptr, "twr_new_buf_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(twr_new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(twr_buf_p, twr_new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(twr_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(twr_leaf_next);

        self.builder.position_at_end(twr_leaf_stop);
        let _ = self
            .builder
            .build_store(twr_stopped_p, i64.const_int(1, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(twr_leaf_next);
        let twr_next_i = self
            .builder
            .build_int_add(
                twr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "twr_ni",
            )
            .map_err(llvm_err)?;
        let twr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        twr_i.add_incoming(&[(&zero, twr_leaf_hdr), (&twr_next_i, twr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(twr_leaf_bdy);
        self.builder.position_at_end(twr_leaf_done);
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(twr_int_hdr);
        let twr_int_i8 = self
            .builder
            .build_pointer_cast(twr_node, ptr, "twr_int_i8")
            .map_err(llvm_err)?;
        let twr_child_count_raw = self
            .builder
            .build_load(i32, twr_int_i8, "twr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let twr_child_count = self
            .builder
            .build_int_z_extend(twr_child_count_raw, i64, "twr_cc")
            .map_err(llvm_err)?;
        let twr_child_h = self
            .builder
            .build_int_sub(twr_height, i64.const_int(1, false), "twr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(twr_int_bdy);
        self.builder.position_at_end(twr_int_bdy);
        let twr_ci = self.builder.build_phi(i64, "twr_ci").map_err(llvm_err)?;
        let twr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                twr_ci.as_basic_value().into_int_value(),
                twr_child_count,
                "twr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(twr_done_int, twr_leaf_done, twr_int_child);
        self.builder.position_at_end(twr_int_child);
        let twr_children_base = unsafe {
            self.builder
                .build_gep(i8, twr_int_i8, &[i64.const_int(16, false)], "twr_cb")
                .map_err(llvm_err)?
        };
        let twr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    twr_children_base,
                    &[twr_ci.as_basic_value().into_int_value()],
                    "twr_cep",
                )
                .map_err(llvm_err)?
        };
        let twr_child_entry = self
            .builder
            .build_load(self.child_entry_type, twr_child_ep, "twr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let twr_child_ptr = self
            .builder
            .build_extract_value(twr_child_entry, 0, "twr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                tw_rec_fn,
                &[
                    twr_child_ptr.into(),
                    twr_child_h.into(),
                    twr_fn.into(),
                    twr_acc.into(),
                    twr_buf_p.into(),
                    twr_buf_pos_p.into(),
                    twr_stopped_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(twr_int_next);
        self.builder.position_at_end(twr_int_next);
        let twr_next_ci = self
            .builder
            .build_int_add(
                twr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "twr_nci",
            )
            .map_err(llvm_err)?;
        let twr_int_next_bb = self.builder.get_insert_block().unwrap();
        twr_ci.add_incoming(&[(&zero, twr_int_hdr), (&twr_next_ci, twr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(twr_int_bdy);

        // ---- action_list_take_while_walk({ptr,i64,i64} list, ptr fn) -> {ptr,i64,i64} ----
        let tw_fn = self.module.add_function(
            "action_list_take_while_walk",
            self.list_type
                .fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let tw_entry = self.context.append_basic_block(tw_fn, "entry");
        let tw_walk = self.context.append_basic_block(tw_fn, "walk");
        let tw_flush = self.context.append_basic_block(tw_fn, "flush");
        let tw_done = self.context.append_basic_block(tw_fn, "done");
        self.builder.position_at_end(tw_entry);
        let tw_list = tw_fn.get_first_param().unwrap().into_struct_value();
        let tw_fn_ptr = tw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let tw_node = self
            .builder
            .build_extract_value(tw_list, 0, "tw_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let tw_len = self
            .builder
            .build_extract_value(tw_list, 1, "tw_len")
            .map_err(llvm_err)?
            .into_int_value();
        let tw_height = self
            .builder
            .build_extract_value(tw_list, 2, "tw_height")
            .map_err(llvm_err)?
            .into_int_value();
        let tw_acc = self
            .builder
            .build_alloca(self.list_type, "tw_acc")
            .map_err(llvm_err)?;
        let tw_buf_p = self
            .builder
            .build_alloca(ptr, "tw_buf_p")
            .map_err(llvm_err)?;
        let tw_buf_pos_p = self
            .builder
            .build_alloca(i64, "tw_buf_pos_p")
            .map_err(llvm_err)?;
        let tw_stopped_p = self
            .builder
            .build_alloca(i64, "tw_stopped_p")
            .map_err(llvm_err)?;
        let tw_init = self
            .builder
            .build_call(create_fn, &[tw_len.into()], "tw_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(tw_acc, tw_init)
            .map_err(llvm_err)?;
        let tw_buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[fw_leaf_sz.into()], "tw_buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let tw_buf_init_i8 = self
            .builder
            .build_pointer_cast(tw_buf_init, ptr, "tw_buf_init_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(tw_buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(tw_buf_p, tw_buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(tw_buf_pos_p, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(tw_stopped_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(tw_walk);
        self.builder.position_at_end(tw_walk);
        let _ = self
            .builder
            .build_call(
                tw_rec_fn,
                &[
                    tw_node.into(),
                    tw_height.into(),
                    tw_fn_ptr.into(),
                    tw_acc.into(),
                    tw_buf_p.into(),
                    tw_buf_pos_p.into(),
                    tw_stopped_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let tw_rem_pos = self
            .builder
            .build_load(i64, tw_buf_pos_p, "tw_rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let tw_has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, tw_rem_pos, zero, "tw_has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(tw_has_rem, tw_flush, tw_done);
        self.builder.position_at_end(tw_flush);
        let tw_rem_buf = self
            .builder
            .build_load(ptr, tw_buf_p, "tw_rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let tw_rem_buf_i8 = self
            .builder
            .build_pointer_cast(tw_rem_buf, ptr, "tw_rem_buf_i8")
            .map_err(llvm_err)?;
        let tw_rem_cnt = self
            .builder
            .build_int_truncate(tw_rem_pos, i32, "tw_rem_cnt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(tw_rem_buf_i8, tw_rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[tw_acc.into(), tw_rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(tw_done);
        self.builder.position_at_end(tw_done);
        let tw_res = self
            .builder
            .build_load(self.list_type, tw_acc, "tw_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&tw_res));
