// ---- action_list_head({ptr, i64, i64}) -> {i64, ptr} ----
        // Delegates to get(0), which handles ConcatNodes.
        let list_head_fn = self.module.add_function(
            "action_list_head",
            self.string_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(list_head_fn, "entry");
        self.builder.position_at_end(entry);
        let lh_list = list_head_fn.get_first_param().unwrap().into_struct_value();
        let lh_len = self
            .builder
            .build_extract_value(lh_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let lh_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, lh_len, i64.const_int(0, false), "empty")
            .map_err(llvm_err)?;
        let lh_has = self.context.append_basic_block(list_head_fn, "has");
        let lh_none = self.context.append_basic_block(list_head_fn, "none");
        let _ = self
            .builder
            .build_conditional_branch(lh_empty, lh_none, lh_has);
        self.builder.position_at_end(lh_none);
        let lh_none_val = self.string_type.const_zero();
        let _ = self.builder.build_return(Some(&lh_none_val));
        self.builder.position_at_end(lh_has);
        // For ConcatNode: get(0) delegates through ConcatNode chain
        let lh_get_fn = self.module.get_function("action_list_get").unwrap();
        let lh_val = self
            .builder
            .build_call(
                lh_get_fn,
                &[lh_list.into(), i64.const_int(0, false).into()],
                "val",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lh_val));

        // ---- action_list_len({ptr, i64, i64}) -> i64 ----
        let list_len_fn = self.module.add_function(
            "action_list_len",
            i64.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(list_len_fn, "entry");
        self.builder.position_at_end(entry);
        let list = list_len_fn.get_first_param().unwrap().into_struct_value();
        let len = self
            .builder
            .build_extract_value(list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_return(Some(&len));

        // ---- action_list_contains_walk(ptr node, i64 height, {i64,ptr} key) -> i1 ----
        // In-order B-tree scan: reads leaf slots directly instead of index-based get().
        let lc_walk_fn = self.module.add_function(
            "action_list_contains_walk",
            b1.fn_type(&[ptr.into(), i64.into(), self.string_type.into()], false),
            None,
        );
        let lw_entry = self.context.append_basic_block(lc_walk_fn, "entry");
        let lw_leaf_hdr = self.context.append_basic_block(lc_walk_fn, "leaf_hdr");
        let lw_leaf_bdy = self.context.append_basic_block(lc_walk_fn, "leaf_bdy");
        let lw_leaf_next = self.context.append_basic_block(lc_walk_fn, "leaf_next");
        let lw_leaf_chk = self.context.append_basic_block(lc_walk_fn, "leaf_chk");
        let lw_leaf_found = self.context.append_basic_block(lc_walk_fn, "leaf_found");
        let lw_leaf_content = self.context.append_basic_block(lc_walk_fn, "leaf_content");
        let lw_leaf_str_gate = self.context.append_basic_block(lc_walk_fn, "leaf_str_gate");
        let lw_leaf_str_cmp = self.context.append_basic_block(lc_walk_fn, "leaf_str_cmp");
        let lw_leaf_str_found = self
            .context
            .append_basic_block(lc_walk_fn, "leaf_str_found");
        let lw_int_hdr = self.context.append_basic_block(lc_walk_fn, "int_hdr");
        let lw_int_bdy = self.context.append_basic_block(lc_walk_fn, "int_bdy");
        let lw_int_next = self.context.append_basic_block(lc_walk_fn, "int_next");
        let lw_int_found = self.context.append_basic_block(lc_walk_fn, "int_found");
        let lw_miss = self.context.append_basic_block(lc_walk_fn, "miss");
        let lw_concat = self.context.append_basic_block(lc_walk_fn, "concat");
        let lw_concat_found = self.context.append_basic_block(lc_walk_fn, "concat_found");
        let lw_concat_right = self.context.append_basic_block(lc_walk_fn, "concat_right");
        let lw_tree = self.context.append_basic_block(lc_walk_fn, "tree");
        self.builder.position_at_end(lw_entry);
        let lw_node = lc_walk_fn.get_first_param().unwrap().into_pointer_value();
        let lw_height = lc_walk_fn.get_nth_param(1).unwrap().into_int_value();
        let lw_key = lc_walk_fn.get_nth_param(2).unwrap().into_struct_value();
        let lw_key_tag = self
            .builder
            .build_extract_value(lw_key, 0, "lw_ktag")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_key_data = self
            .builder
            .build_extract_value(lw_key, 1, "lw_kdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lw_neg1 = i64.const_int(-1i64 as u64, true);
        let lw_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, lw_height, lw_neg1, "lw_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_is_concat, lw_concat, lw_tree);

        // ConcatNode: walk left then right without flattening
        self.builder.position_at_end(lw_concat);
        let lw_cn_ln_p = unsafe {
            self.builder
                .build_gep(ptr, lw_node, &[i64.const_int(2, false)], "cn_ln_p")
                .map_err(llvm_err)
        }?;
        let lw_cn_left = self
            .builder
            .build_load(ptr, lw_cn_ln_p, "cn_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lw_cn_lh_p = unsafe {
            self.builder
                .build_gep(i64, lw_node, &[i64.const_int(4, false)], "cn_lh_p")
                .map_err(llvm_err)
        }?;
        let lw_cn_lh = self
            .builder
            .build_load(i64, lw_cn_lh_p, "cn_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_cn_rn_p = unsafe {
            self.builder
                .build_gep(ptr, lw_node, &[i64.const_int(5, false)], "cn_rn_p")
                .map_err(llvm_err)
        }?;
        let lw_cn_right = self
            .builder
            .build_load(ptr, lw_cn_rn_p, "cn_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lw_cn_rh_p = unsafe {
            self.builder
                .build_gep(i64, lw_node, &[i64.const_int(7, false)], "cn_rh_p")
                .map_err(llvm_err)
        }?;
        let lw_cn_rh = self
            .builder
            .build_load(i64, lw_cn_rh_p, "cn_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_left_hit = self
            .builder
            .build_call(
                lc_walk_fn,
                &[lw_cn_left.into(), lw_cn_lh.into(), lw_key.into()],
                "lw_lhit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let lw_left_ok = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lw_left_hit,
                b1.const_int(1, false),
                "lw_lok",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_left_ok, lw_concat_found, lw_concat_right);
        self.builder.position_at_end(lw_concat_found);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(lw_concat_right);
        let lw_right_hit = self
            .builder
            .build_call(
                lc_walk_fn,
                &[lw_cn_right.into(), lw_cn_rh.into(), lw_key.into()],
                "lw_rhit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&lw_right_hit));

        self.builder.position_at_end(lw_tree);
        let lw_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, lw_height, zero, "lw_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_is_leaf, lw_leaf_hdr, lw_int_hdr);

        // Leaf scan
        self.builder.position_at_end(lw_leaf_hdr);
        let lw_leaf_i8 = self
            .builder
            .build_pointer_cast(lw_node, ptr, "lw_leaf_i8")
            .map_err(llvm_err)?;
        let lw_count_raw = self
            .builder
            .build_load(i32, lw_leaf_i8, "lw_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_count = self
            .builder
            .build_int_z_extend(lw_count_raw, i64, "lw_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lw_leaf_bdy);
        self.builder.position_at_end(lw_leaf_bdy);
        let lw_i = self.builder.build_phi(i64, "lw_i").map_err(llvm_err)?;
        let lw_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                lw_i.as_basic_value().into_int_value(),
                lw_count,
                "lw_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_done_leaf, lw_miss, lw_leaf_chk);
        self.builder.position_at_end(lw_leaf_chk);
        let lw_eb = unsafe {
            self.builder
                .build_gep(i8, lw_leaf_i8, &[i64.const_int(8, false)], "lw_eb")
                .map_err(llvm_err)?
        };
        let lw_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    lw_eb,
                    &[lw_i.as_basic_value().into_int_value()],
                    "lw_ep",
                )
                .map_err(llvm_err)?
        };
        let lw_elem = self
            .builder
            .build_load(self.string_type, lw_ep, "lw_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let lw_elem_tag = self
            .builder
            .build_extract_value(lw_elem, 0, "lw_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_elem_data = self
            .builder
            .build_extract_value(lw_elem, 1, "lw_edata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lw_tag_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, lw_elem_tag, lw_key_tag, "lw_teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_tag_eq, lw_leaf_content, lw_leaf_next);
        self.builder.position_at_end(lw_leaf_content);
        let lw_null = self.ptr_ty().const_zero();
        let lw_ed_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, lw_elem_data, lw_null, "lw_ed_null")
            .map_err(llvm_err)?;
        let lw_kd_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, lw_key_data, lw_null, "lw_kd_null")
            .map_err(llvm_err)?;
        let lw_both_null = self
            .builder
            .build_and(lw_ed_null, lw_kd_null, "lw_both_null")
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lw_both_null, lw_leaf_found, lw_leaf_str_gate);
        self.builder.position_at_end(lw_leaf_str_gate);
        let lw_ed_nn = self
            .builder
            .build_not(lw_ed_null, "lw_ed_nn")
            .map_err(llvm_err)?;
        let lw_kd_nn = self
            .builder
            .build_not(lw_kd_null, "lw_kd_nn")
            .map_err(llvm_err)?;
        let lw_both_nn = self
            .builder
            .build_and(lw_ed_nn, lw_kd_nn, "lw_both_nn")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_both_nn, lw_leaf_str_cmp, lw_leaf_next);
        self.builder.position_at_end(lw_leaf_str_cmp);
        let lw_str_eq = self
            .call_rt(
                "action_string_eq",
                &[
                    lw_elem.as_basic_value_enum().into(),
                    lw_key.as_basic_value_enum().into(),
                ],
            )?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(lw_str_eq, lw_leaf_str_found, lw_leaf_next);
        self.builder.position_at_end(lw_leaf_found);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(lw_leaf_str_found);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(lw_leaf_next);
        let lw_next_i = self
            .builder
            .build_int_add(
                lw_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "lw_ni",
            )
            .map_err(llvm_err)?;
        let lw_leaf_next_bb = self.builder.get_insert_block().unwrap();
        lw_i.add_incoming(&[(&zero, lw_leaf_hdr), (&lw_next_i, lw_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(lw_leaf_bdy);

        // Internal node: recurse into each child in order
        self.builder.position_at_end(lw_int_hdr);
        let lw_int_i8 = self
            .builder
            .build_pointer_cast(lw_node, ptr, "lw_int_i8")
            .map_err(llvm_err)?;
        let lw_child_count_raw = self
            .builder
            .build_load(i32, lw_int_i8, "lw_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lw_child_count = self
            .builder
            .build_int_z_extend(lw_child_count_raw, i64, "lw_cc")
            .map_err(llvm_err)?;
        let lw_child_h = self
            .builder
            .build_int_sub(lw_height, i64.const_int(1, false), "lw_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lw_int_bdy);
        self.builder.position_at_end(lw_int_bdy);
        let lw_ci = self.builder.build_phi(i64, "lw_ci").map_err(llvm_err)?;
        let lw_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                lw_ci.as_basic_value().into_int_value(),
                lw_child_count,
                "lw_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lw_done_int, lw_miss, lw_int_found);
        self.builder.position_at_end(lw_int_found);
        let lw_children_base = unsafe {
            self.builder
                .build_gep(i8, lw_int_i8, &[i64.const_int(16, false)], "lw_cb")
                .map_err(llvm_err)?
        };
        let lw_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    lw_children_base,
                    &[lw_ci.as_basic_value().into_int_value()],
                    "lw_cep",
                )
                .map_err(llvm_err)?
        };
        let lw_child_entry = self
            .builder
            .build_load(self.child_entry_type, lw_child_ep, "lw_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let lw_child_ptr = self
            .builder
            .build_extract_value(lw_child_entry, 0, "lw_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lw_child_hit = self
            .builder
            .build_call(
                lc_walk_fn,
                &[lw_child_ptr.into(), lw_child_h.into(), lw_key.into()],
                "lw_hit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(lw_child_hit, lw_leaf_found, lw_int_next);
        self.builder.position_at_end(lw_int_next);
        let lw_next_ci = self
            .builder
            .build_int_add(
                lw_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "lw_nci",
            )
            .map_err(llvm_err)?;
        let lw_int_next_bb = self.builder.get_insert_block().unwrap();
        lw_ci.add_incoming(&[(&zero, lw_int_hdr), (&lw_next_ci, lw_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(lw_int_bdy);

        self.builder.position_at_end(lw_miss);
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));
