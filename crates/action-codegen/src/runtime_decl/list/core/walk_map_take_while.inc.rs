// ---- action_list_map_take_while_walk_rec(ptr node, i64 height, ptr map_fn, ptr filter_fn, ptr acc, ptr buf_p, ptr buf_pos_p) -> void ----
        // Fused map+filter: in-order B-tree walk that applies map_fn to each element,
        // then calls filter_fn on the mapped value, pushing only passing elements.
        let mfw_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let mtw_rec_fn = self.module.add_function(
            "action_list_map_take_while_walk_rec",
            void.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    ptr.into(),
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
        let mtwr_entry = self.context.append_basic_block(mtw_rec_fn, "entry");
        let mtwr_leaf_hdr = self.context.append_basic_block(mtw_rec_fn, "leaf_hdr");
        let mtwr_leaf_bdy = self.context.append_basic_block(mtw_rec_fn, "leaf_bdy");
        let mtwr_leaf_chk = self.context.append_basic_block(mtw_rec_fn, "leaf_chk");
        let mtwr_leaf_push = self.context.append_basic_block(mtw_rec_fn, "leaf_push");
        let mtwr_leaf_stop = self.context.append_basic_block(mtw_rec_fn, "leaf_stop");
        let mtwr_leaf_flush = self.context.append_basic_block(mtw_rec_fn, "leaf_flush");
        let mtwr_leaf_next = self.context.append_basic_block(mtw_rec_fn, "leaf_next");
        let mtwr_leaf_done = self.context.append_basic_block(mtw_rec_fn, "leaf_done");
        let mtwr_int_hdr = self.context.append_basic_block(mtw_rec_fn, "int_hdr");
        let mtwr_int_bdy = self.context.append_basic_block(mtw_rec_fn, "int_bdy");
        let mtwr_int_child = self.context.append_basic_block(mtw_rec_fn, "int_child");
        let mtwr_int_next = self.context.append_basic_block(mtw_rec_fn, "int_next");
        let mtwr_concat = self.context.append_basic_block(mtw_rec_fn, "concat");
        let mtwr_normal = self.context.append_basic_block(mtw_rec_fn, "normal");
        self.builder.position_at_end(mtwr_entry);
        let mtwr_node = mtw_rec_fn.get_first_param().unwrap().into_pointer_value();
        let mtwr_height = mtw_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let mtwr_map_fn = mtw_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let mtwr_filter_fn = mtw_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let mtwr_acc = mtw_rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let mtwr_buf_p = mtw_rec_fn.get_nth_param(5).unwrap().into_pointer_value();
        let mtwr_buf_pos_p = mtw_rec_fn.get_nth_param(6).unwrap().into_pointer_value();
        let mtwr_stopped_p = mtw_rec_fn.get_nth_param(7).unwrap().into_pointer_value();
        let mtwr_neg1 = i64.const_int(-1i64 as u64, true);
        let mtwr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, mtwr_height, mtwr_neg1, "mtwr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mtwr_is_concat, mtwr_concat, mtwr_normal);
        self.builder.position_at_end(mtwr_concat);
        let mtwr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, mtwr_node, &[i64.const_int(2, false)], "mtwr_ln_p")
                .map_err(llvm_err)
        }?;
        let mtwr_left_node = self
            .builder
            .build_load(ptr, mtwr_ln_p, "mtwr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mtwr_lh_p = unsafe {
            self.builder
                .build_gep(i64, mtwr_node, &[i64.const_int(4, false)], "mtwr_lh_p")
                .map_err(llvm_err)
        }?;
        let mtwr_left_h = self
            .builder
            .build_load(i64, mtwr_lh_p, "mtwr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let mtwr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, mtwr_node, &[i64.const_int(5, false)], "mtwr_rn_p")
                .map_err(llvm_err)
        }?;
        let mtwr_right_node = self
            .builder
            .build_load(ptr, mtwr_rn_p, "mtwr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mtwr_rh_p = unsafe {
            self.builder
                .build_gep(i64, mtwr_node, &[i64.const_int(7, false)], "mtwr_rh_p")
                .map_err(llvm_err)
        }?;
        let mtwr_right_h = self
            .builder
            .build_load(i64, mtwr_rh_p, "mtwr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                mtw_rec_fn,
                &[
                    mtwr_left_node.into(),
                    mtwr_left_h.into(),
                    mtwr_map_fn.into(),
                    mtwr_filter_fn.into(),
                    mtwr_acc.into(),
                    mtwr_buf_p.into(),
                    mtwr_buf_pos_p.into(),
                    mtwr_stopped_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                mtw_rec_fn,
                &[
                    mtwr_right_node.into(),
                    mtwr_right_h.into(),
                    mtwr_map_fn.into(),
                    mtwr_filter_fn.into(),
                    mtwr_acc.into(),
                    mtwr_buf_p.into(),
                    mtwr_buf_pos_p.into(),
                    mtwr_stopped_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(mtwr_normal);
        let mtwr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, mtwr_height, zero, "mtwr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mtwr_is_leaf, mtwr_leaf_hdr, mtwr_int_hdr);

        // Leaf scan: apply map, then filter, push only passing elements
        self.builder.position_at_end(mtwr_leaf_hdr);
        let mtwr_leaf_i8 = self
            .builder
            .build_pointer_cast(mtwr_node, ptr, "mtwr_leaf_i8")
            .map_err(llvm_err)?;
        let mtwr_count_raw = self
            .builder
            .build_load(i32, mtwr_leaf_i8, "mtwr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mtwr_count = self
            .builder
            .build_int_z_extend(mtwr_count_raw, i64, "mtwr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mtwr_leaf_bdy);
        self.builder.position_at_end(mtwr_leaf_bdy);
        let mtwr_i = self.builder.build_phi(i64, "mtwr_i").map_err(llvm_err)?;
        let mtwr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mtwr_i.as_basic_value().into_int_value(),
                mtwr_count,
                "mtwr_done",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(mtwr_done_leaf, mtwr_leaf_done, mtwr_leaf_chk);
        self.builder.position_at_end(mtwr_leaf_chk);
        let mtwr_eb = unsafe {
            self.builder
                .build_gep(i8, mtwr_leaf_i8, &[i64.const_int(8, false)], "mtwr_eb")
                .map_err(llvm_err)?
        };
        let mtwr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    mtwr_eb,
                    &[mtwr_i.as_basic_value().into_int_value()],
                    "mtwr_ep",
                )
                .map_err(llvm_err)?
        };
        let mtwr_elem = self
            .builder
            .build_load(self.string_type, mtwr_ep, "mtwr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let mtwr_elem_tag = self
            .builder
            .build_extract_value(mtwr_elem, 0, "mtwr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        // Apply map function
        let mtwr_mapped_call = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                mtwr_map_fn,
                &[mtwr_elem_tag.into()],
                "mtwr_map",
            )
            .map_err(llvm_err)?;
        let mtwr_mapped_bv = mtwr_mapped_call
            .try_as_basic_value()
            .basic()
            .ok_or("map_filter_walk map call failed")?;
        // Extract tag from mapped value for filter predicate
        let mtwr_mapped_struct = mtwr_mapped_bv.into_struct_value();
        let mtwr_mapped_tag = self
            .builder
            .build_extract_value(mtwr_mapped_struct, 0, "mtwr_mt")
            .map_err(llvm_err)?
            .into_int_value();
        // Apply filter function on mapped value
        let mtwr_pred = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                mtwr_filter_fn,
                &[mtwr_mapped_tag.into()],
                "mtwr_pred",
            )
            .map_err(llvm_err)?;
        let mtwr_pred_bv = mtwr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("map_filter_walk filter call failed")?;
        let mtwr_pred_val = if mtwr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(mtwr_pred_bv.into_struct_value(), 0, "mtwr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            mtwr_pred_bv.into_int_value()
        };
        let mtwr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, mtwr_pred_val, zero, "mtwr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mtwr_is_true, mtwr_leaf_push, mtwr_leaf_stop);
        // Push mapped value to buffer
        self.builder.position_at_end(mtwr_leaf_push);
        let mtwr_buf = self
            .builder
            .build_load(ptr, mtwr_buf_p, "mtwr_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mtwr_pos = self
            .builder
            .build_load(i64, mtwr_buf_pos_p, "mtwr_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mtwr_buf_i8 = self
            .builder
            .build_pointer_cast(mtwr_buf, ptr, "mtwr_buf_i8")
            .map_err(llvm_err)?;
        let mtwr_buf_eb = unsafe {
            self.builder
                .build_gep(i8, mtwr_buf_i8, &[i64.const_int(8, false)], "mtwr_buf_eb")
                .map_err(llvm_err)?
        };
        let mtwr_buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, mtwr_buf_eb, &[mtwr_pos], "mtwr_buf_ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(mtwr_buf_ep, mtwr_mapped_bv)
            .map_err(llvm_err)?;
        let mtwr_pos_inc = self
            .builder
            .build_int_add(mtwr_pos, i64.const_int(1, false), "mtwr_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(mtwr_buf_pos_p, mtwr_pos_inc)
            .map_err(llvm_err)?;
        let mtwr_buf_full = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                mtwr_pos_inc,
                i64.const_int(64, false),
                "mtwr_buf_full",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(mtwr_buf_full, mtwr_leaf_flush, mtwr_leaf_next);

        self.builder.position_at_end(mtwr_leaf_flush);
        let mtwr_flush_cnt = i32.const_int(64, false);
        let _ = self
            .builder
            .build_store(mtwr_buf_i8, mtwr_flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mtwr_acc.into(), mtwr_buf.into()], "")
            .map_err(llvm_err)?;
        let mtwr_new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[mfw_leaf_sz.into()], "mtwr_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mtwr_new_buf_i8 = self
            .builder
            .build_pointer_cast(mtwr_new_buf, ptr, "mtwr_new_buf_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mtwr_new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mtwr_buf_p, mtwr_new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mtwr_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mtwr_leaf_next);

        self.builder.position_at_end(mtwr_leaf_stop);
        let _ = self
            .builder
            .build_store(mtwr_stopped_p, i64.const_int(1, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(mtwr_leaf_next);
        let mtwr_next_i = self
            .builder
            .build_int_add(
                mtwr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mtwr_ni",
            )
            .map_err(llvm_err)?;
        let mtwr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        mtwr_i.add_incoming(&[(&zero, mtwr_leaf_hdr), (&mtwr_next_i, mtwr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mtwr_leaf_bdy);
        self.builder.position_at_end(mtwr_leaf_done);
        let _ = self.builder.build_return(None);

        // Internal node: recurse into each child in order
        self.builder.position_at_end(mtwr_int_hdr);
        let mtwr_int_i8 = self
            .builder
            .build_pointer_cast(mtwr_node, ptr, "mtwr_int_i8")
            .map_err(llvm_err)?;
        let mtwr_child_count_raw = self
            .builder
            .build_load(i32, mtwr_int_i8, "mtwr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mtwr_child_count = self
            .builder
            .build_int_z_extend(mtwr_child_count_raw, i64, "mtwr_cc")
            .map_err(llvm_err)?;
        let mtwr_child_h = self
            .builder
            .build_int_sub(mtwr_height, i64.const_int(1, false), "mtwr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mtwr_int_bdy);
        self.builder.position_at_end(mtwr_int_bdy);
        let mtwr_ci = self.builder.build_phi(i64, "mtwr_ci").map_err(llvm_err)?;
        let mtwr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mtwr_ci.as_basic_value().into_int_value(),
                mtwr_child_count,
                "mtwr_done_int",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(mtwr_done_int, mtwr_leaf_done, mtwr_int_child);
        self.builder.position_at_end(mtwr_int_child);
        let mtwr_children_base = unsafe {
            self.builder
                .build_gep(i8, mtwr_int_i8, &[i64.const_int(16, false)], "mtwr_cb")
                .map_err(llvm_err)?
        };
        let mtwr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    mtwr_children_base,
                    &[mtwr_ci.as_basic_value().into_int_value()],
                    "mtwr_cep",
                )
                .map_err(llvm_err)?
        };
        let mtwr_child_entry = self
            .builder
            .build_load(self.child_entry_type, mtwr_child_ep, "mtwr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let mtwr_child_ptr = self
            .builder
            .build_extract_value(mtwr_child_entry, 0, "mtwr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                mtw_rec_fn,
                &[
                    mtwr_child_ptr.into(),
                    mtwr_child_h.into(),
                    mtwr_map_fn.into(),
                    mtwr_filter_fn.into(),
                    mtwr_acc.into(),
                    mtwr_buf_p.into(),
                    mtwr_buf_pos_p.into(),
                    mtwr_stopped_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mtwr_int_next);
        self.builder.position_at_end(mtwr_int_next);
        let mtwr_next_ci = self
            .builder
            .build_int_add(
                mtwr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mtwr_nci",
            )
            .map_err(llvm_err)?;
        let mtwr_int_next_bb = self.builder.get_insert_block().unwrap();
        mtwr_ci.add_incoming(&[(&zero, mtwr_int_hdr), (&mtwr_next_ci, mtwr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mtwr_int_bdy);

        // ---- action_list_map_take_while_walk({ptr,i64,i64} list, ptr map_fn, ptr filter_fn) -> {ptr,i64,i64} ----
        let mtw_fn = self.module.add_function(
            "action_list_map_take_while_walk",
            self.list_type
                .fn_type(&[self.list_type.into(), ptr.into(), ptr.into()], false),
            None,
        );
        let mtw_entry = self.context.append_basic_block(mtw_fn, "entry");
        let mtw_walk = self.context.append_basic_block(mtw_fn, "walk");
        let mtw_flush = self.context.append_basic_block(mtw_fn, "flush");
        let mtw_done = self.context.append_basic_block(mtw_fn, "done");
        self.builder.position_at_end(mtw_entry);
        let mtw_list = mtw_fn.get_first_param().unwrap().into_struct_value();
        let mtw_map_fn_ptr = mtw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let mtw_filter_fn_ptr = mtw_fn.get_nth_param(2).unwrap().into_pointer_value();
        let mtw_node = self
            .builder
            .build_extract_value(mtw_list, 0, "mtw_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mtw_len = self
            .builder
            .build_extract_value(mtw_list, 1, "mtw_len")
            .map_err(llvm_err)?
            .into_int_value();
        let mtw_height = self
            .builder
            .build_extract_value(mtw_list, 2, "mtw_height")
            .map_err(llvm_err)?
            .into_int_value();
        let mtw_acc = self
            .builder
            .build_alloca(self.list_type, "mtw_acc")
            .map_err(llvm_err)?;
        let mtw_buf_p = self
            .builder
            .build_alloca(ptr, "mtw_buf_p")
            .map_err(llvm_err)?;
        let mtw_stopped_p = self
            .builder
            .build_alloca(i64, "mtw_stopped_p")
            .map_err(llvm_err)?;
        let mtw_buf_pos_p = self
            .builder
            .build_alloca(i64, "mtw_buf_pos_p")
            .map_err(llvm_err)?;
        let mtw_init = self
            .builder
            .build_call(create_fn, &[mtw_len.into()], "mtw_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(mtw_acc, mtw_init)
            .map_err(llvm_err)?;
        let mtw_buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[mfw_leaf_sz.into()], "mtw_buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mtw_buf_init_i8 = self
            .builder
            .build_pointer_cast(mtw_buf_init, ptr, "mtw_buf_init_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mtw_buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mtw_buf_p, mtw_buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mtw_buf_pos_p, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mtw_stopped_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mtw_walk);
        self.builder.position_at_end(mtw_walk);
        let _ = self
            .builder
            .build_call(
                mtw_rec_fn,
                &[
                    mtw_node.into(),
                    mtw_height.into(),
                    mtw_map_fn_ptr.into(),
                    mtw_filter_fn_ptr.into(),
                    mtw_acc.into(),
                    mtw_buf_p.into(),
                    mtw_buf_pos_p.into(),
                    mtw_stopped_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let mtw_rem_pos = self
            .builder
            .build_load(i64, mtw_buf_pos_p, "mtw_rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mtw_has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, mtw_rem_pos, zero, "mtw_has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mtw_has_rem, mtw_flush, mtw_done);
        self.builder.position_at_end(mtw_flush);
        let mtw_rem_buf = self
            .builder
            .build_load(ptr, mtw_buf_p, "mtw_rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mtw_rem_buf_i8 = self
            .builder
            .build_pointer_cast(mtw_rem_buf, ptr, "mtw_rem_buf_i8")
            .map_err(llvm_err)?;
        let mtw_rem_cnt = self
            .builder
            .build_int_truncate(mtw_rem_pos, i32, "mtw_rem_cnt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mtw_rem_buf_i8, mtw_rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mtw_acc.into(), mtw_rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mtw_done);
        self.builder.position_at_end(mtw_done);
        let mtw_res = self
            .builder
            .build_load(self.list_type, mtw_acc, "mtw_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mtw_res));
