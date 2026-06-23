// ---- action_list_set_rec(ptr node, i64 height, i64 idx, {i64,ptr} val) -> ptr ----
        // B-tree path-copy single-element update. CoW when rc > 1.
        let lsr_fn = self.module.add_function(
            "action_list_set_rec",
            ptr.fn_type(
                &[ptr.into(), i64.into(), i64.into(), self.string_type.into()],
                false,
            ),
            None,
        );
        let lsr_entry = self.context.append_basic_block(lsr_fn, "entry");
        let lsr_leaf = self.context.append_basic_block(lsr_fn, "leaf");
        let lsr_leaf_cow = self.context.append_basic_block(lsr_fn, "leaf_cow");
        let lsr_leaf_cow_copy = self.context.append_basic_block(lsr_fn, "leaf_cow_copy");
        let lsr_leaf_store = self.context.append_basic_block(lsr_fn, "leaf_store");
        let lsr_int_scan_loop = self.context.append_basic_block(lsr_fn, "int_scan_loop");
        let lsr_int_scan_body = self.context.append_basic_block(lsr_fn, "int_scan_body");
        let lsr_int_scan_found = self.context.append_basic_block(lsr_fn, "int_scan_found");
        let lsr_int_scan_next = self.context.append_basic_block(lsr_fn, "int_scan_next");
        let lsr_int_cow = self.context.append_basic_block(lsr_fn, "int_cow");
        let lsr_int_cow_copy = self.context.append_basic_block(lsr_fn, "int_cow_copy");
        let lsr_int_inc_loop = self.context.append_basic_block(lsr_fn, "int_inc_loop");
        let lsr_int_inc_body = self.context.append_basic_block(lsr_fn, "int_inc_body");
        let lsr_int_inc_done = self.context.append_basic_block(lsr_fn, "int_inc_done");
        let lsr_int_update = self.context.append_basic_block(lsr_fn, "int_update");
        let lsr_int_ret = self.context.append_basic_block(lsr_fn, "int_ret");
        self.builder.position_at_end(lsr_entry);
        let lsr_node = lsr_fn.get_first_param().unwrap().into_pointer_value();
        let lsr_height = lsr_fn.get_nth_param(1).unwrap().into_int_value();
        let lsr_idx = lsr_fn.get_nth_param(2).unwrap().into_int_value();
        let lsr_val = lsr_fn.get_nth_param(3).unwrap().into_struct_value();
        let lsr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, lsr_height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lsr_is_leaf, lsr_leaf, lsr_int_scan_loop);

        // Leaf: CoW if shared, store element at idx
        self.builder.position_at_end(lsr_leaf);
        let lsr_leaf_int = self
            .builder
            .build_ptr_to_int(lsr_node, i64, "leaf_int")
            .map_err(llvm_err)?;
        let lsr_leaf_rc_a = self
            .builder
            .build_int_sub(lsr_leaf_int, i64.const_int(8, false), "leaf_rc_a")
            .map_err(llvm_err)?;
        let lsr_leaf_rc_p = self
            .builder
            .build_int_to_ptr(lsr_leaf_rc_a, ptr, "leaf_rc_p")
            .map_err(llvm_err)?;
        let lsr_leaf_rc = self
            .builder
            .build_load(i64, lsr_leaf_rc_p, "leaf_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_leaf_shared = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                lsr_leaf_rc,
                i64.const_int(1, false),
                "leaf_shared",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lsr_leaf_shared, lsr_leaf_cow, lsr_leaf_store);

        self.builder.position_at_end(lsr_leaf_cow);
        let lsr_need_copy = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                lsr_leaf_rc,
                i64.const_int(1, false),
                "need_copy",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lsr_need_copy, lsr_leaf_cow_copy, lsr_leaf_store);

        self.builder.position_at_end(lsr_leaf_cow_copy);
        let lsr_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let lsr_new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[lsr_leaf_sz.into()], "new_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let lsr_memcpy = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                lsr_memcpy,
                &[lsr_new_leaf.into(), lsr_node.into(), lsr_leaf_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let lsr_new_leaf_rc = self
            .builder
            .build_int_sub(lsr_leaf_rc, i64.const_int(1, false), "new_leaf_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lsr_leaf_rc_p, lsr_new_leaf_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_leaf_store);

        self.builder.position_at_end(lsr_leaf_store);
        let lsr_leaf_phi = self.builder.build_phi(ptr, "leaf_phi").map_err(llvm_err)?;
        lsr_leaf_phi.add_incoming(&[
            (&lsr_node, lsr_leaf),
            (&lsr_node, lsr_leaf_cow),
            (&lsr_new_leaf, lsr_leaf_cow_copy),
        ]);
        let lsr_leaf_ptr = lsr_leaf_phi.as_basic_value().into_pointer_value();
        let lsr_leaf_i8 = self
            .builder
            .build_pointer_cast(lsr_leaf_ptr, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let lsr_eb = unsafe {
            self.builder
                .build_gep(i8, lsr_leaf_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)?
        };
        let lsr_ep = unsafe {
            self.builder
                .build_gep(self.string_type, lsr_eb, &[lsr_idx], "ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(lsr_ep, lsr_val)
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lsr_leaf_ptr));

        // Internal: scan children to find target, recurse, path-copy on way up
        self.builder.position_at_end(lsr_int_scan_loop);
        let lsr_phi_i = self.builder.build_phi(i64, "phi_i").map_err(llvm_err)?;
        let lsr_phi_acc = self.builder.build_phi(i64, "phi_acc").map_err(llvm_err)?;
        lsr_phi_i.add_incoming(&[(&zero, lsr_entry)]);
        lsr_phi_acc.add_incoming(&[(&zero, lsr_entry)]);
        let lsr_scan_i = lsr_phi_i.as_basic_value().into_int_value();
        let lsr_scan_acc = lsr_phi_acc.as_basic_value().into_int_value();
        let lsr_int_i8 = self
            .builder
            .build_pointer_cast(lsr_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let lsr_int_count_raw = self
            .builder
            .build_load(i32, lsr_int_i8, "intl_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_int_count = self
            .builder
            .build_int_z_extend(lsr_int_count_raw, i64, "intl_count")
            .map_err(llvm_err)?;
        let lsr_done_scan = self
            .builder
            .build_int_compare(IntPredicate::SGE, lsr_scan_i, lsr_int_count, "done_scan")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            lsr_done_scan,
            lsr_int_scan_found,
            lsr_int_scan_body,
        );

        self.builder.position_at_end(lsr_int_scan_body);
        let lsr_children_base = unsafe {
            self.builder
                .build_gep(i8, lsr_int_i8, &[i64.const_int(16, false)], "scb")
                .map_err(llvm_err)?
        };
        let lsr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    lsr_children_base,
                    &[lsr_scan_i],
                    "cep",
                )
                .map_err(llvm_err)?
        };
        let lsr_child_total = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, lsr_child_ep, "ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                1,
                "ct",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_new_acc = self
            .builder
            .build_int_add(lsr_scan_acc, lsr_child_total, "new_acc")
            .map_err(llvm_err)?;
        let lsr_found_child = self
            .builder
            .build_int_compare(IntPredicate::SLT, lsr_idx, lsr_new_acc, "found_child")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            lsr_found_child,
            lsr_int_scan_found,
            lsr_int_scan_next,
        );

        self.builder.position_at_end(lsr_int_scan_next);
        let lsr_next_i = self
            .builder
            .build_int_add(lsr_scan_i, i64.const_int(1, false), "next_i")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_int_scan_loop);
        lsr_phi_i.add_incoming(&[(&lsr_next_i, lsr_int_scan_next)]);
        lsr_phi_acc.add_incoming(&[(&lsr_new_acc, lsr_int_scan_next)]);

        self.builder.position_at_end(lsr_int_scan_found);
        let lsr_phi_found_i = self.builder.build_phi(i64, "phi_fi").map_err(llvm_err)?;
        let lsr_phi_found_acc = self.builder.build_phi(i64, "phi_fa").map_err(llvm_err)?;
        lsr_phi_found_i.add_incoming(&[
            (&lsr_scan_i, lsr_int_scan_body),
            (&lsr_scan_i, lsr_int_scan_loop),
        ]);
        lsr_phi_found_acc.add_incoming(&[
            (&lsr_scan_acc, lsr_int_scan_body),
            (&lsr_scan_acc, lsr_int_scan_loop),
        ]);
        let lsr_found_i = lsr_phi_found_i.as_basic_value().into_int_value();
        let lsr_offset_before = lsr_phi_found_acc.as_basic_value().into_int_value();
        let lsr_found_ce_base = unsafe {
            self.builder
                .build_gep(i8, lsr_int_i8, &[i64.const_int(16, false)], "fceb")
                .map_err(llvm_err)?
        };
        let lsr_found_ce_ptr = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    lsr_found_ce_base,
                    &[lsr_found_i],
                    "fcep",
                )
                .map_err(llvm_err)?
        };
        let lsr_old_child = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, lsr_found_ce_ptr, "fce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                0,
                "old_child",
            )
            .map_err(llvm_err)?
            .into_pointer_value();
        let lsr_local_idx = self
            .builder
            .build_int_sub(lsr_idx, lsr_offset_before, "local_idx")
            .map_err(llvm_err)?;
        let lsr_child_h = self
            .builder
            .build_int_sub(lsr_height, i64.const_int(1, false), "child_h")
            .map_err(llvm_err)?;
        let lsr_new_child = self
            .builder
            .build_call(
                lsr_fn,
                &[
                    lsr_old_child.into(),
                    lsr_child_h.into(),
                    lsr_local_idx.into(),
                    lsr_val.into(),
                ],
                "new_child",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self.builder.build_unconditional_branch(lsr_int_cow);

        // CoW internal node if shared
        self.builder.position_at_end(lsr_int_cow);
        let lsr_int_int = self
            .builder
            .build_ptr_to_int(lsr_node, i64, "int_int")
            .map_err(llvm_err)?;
        let lsr_int_rc_a = self
            .builder
            .build_int_sub(lsr_int_int, i64.const_int(8, false), "int_rc_a")
            .map_err(llvm_err)?;
        let lsr_int_rc_p = self
            .builder
            .build_int_to_ptr(lsr_int_rc_a, ptr, "int_rc_p")
            .map_err(llvm_err)?;
        let lsr_int_rc = self
            .builder
            .build_load(i64, lsr_int_rc_p, "int_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_int_shared = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                lsr_int_rc,
                i64.const_int(1, false),
                "int_shared",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lsr_int_shared, lsr_int_cow_copy, lsr_int_update);

        self.builder.position_at_end(lsr_int_cow_copy);
        let lsr_int_sz = self.internal_type.size_of().ok_or("internal size")?;
        let lsr_new_int = self
            .builder
            .build_call(malloc_rc_fn, &[lsr_int_sz.into()], "new_int")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                lsr_memcpy,
                &[lsr_new_int.into(), lsr_node.into(), lsr_int_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let lsr_new_int_rc = self
            .builder
            .build_int_sub(lsr_int_rc, i64.const_int(1, false), "new_int_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lsr_int_rc_p, lsr_new_int_rc)
            .map_err(llvm_err)?;
        let lsr_rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let _ = self.builder.build_unconditional_branch(lsr_int_inc_loop);

        self.builder.position_at_end(lsr_int_inc_loop);
        let lsr_inc_i_phi = self.builder.build_phi(i64, "inc_i").map_err(llvm_err)?;
        lsr_inc_i_phi.add_incoming(&[(&zero, lsr_int_cow_copy)]);
        let lsr_inc_i = lsr_inc_i_phi.as_basic_value().into_int_value();
        let lsr_new_int_i8 = self
            .builder
            .build_pointer_cast(lsr_new_int, ptr, "ni_i8")
            .map_err(llvm_err)?;
        let lsr_new_int_count_raw = self
            .builder
            .build_load(i32, lsr_new_int_i8, "ni_count")
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_new_int_count = self
            .builder
            .build_int_z_extend(lsr_new_int_count_raw, i64, "ni_cnt")
            .map_err(llvm_err)?;
        let lsr_inc_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, lsr_inc_i, lsr_new_int_count, "inc_done")
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(lsr_inc_done, lsr_int_inc_done, lsr_int_inc_body);

        self.builder.position_at_end(lsr_int_inc_body);
        let lsr_inc_cb = unsafe {
            self.builder
                .build_gep(i8, lsr_new_int_i8, &[i64.const_int(16, false)], "inc_cb")
                .map_err(llvm_err)?
        };
        let lsr_inc_cep = unsafe {
            self.builder
                .build_gep(self.child_entry_type, lsr_inc_cb, &[lsr_inc_i], "inc_cep")
                .map_err(llvm_err)?
        };
        let lsr_inc_child = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, lsr_inc_cep, "inc_ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                0,
                "inc_ch",
            )
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(lsr_rc_inc_fn, &[lsr_inc_child.into()], "")
            .map_err(llvm_err)?;
        let lsr_inc_next = self
            .builder
            .build_int_add(lsr_inc_i, i64.const_int(1, false), "inc_next")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_int_inc_loop);
        lsr_inc_i_phi.add_incoming(&[(&lsr_inc_next, lsr_int_inc_body)]);

        self.builder.position_at_end(lsr_int_inc_done);
        let _ = self.builder.build_unconditional_branch(lsr_int_update);

        self.builder.position_at_end(lsr_int_update);
        let lsr_work_phi = self.builder.build_phi(ptr, "work_phi").map_err(llvm_err)?;
        lsr_work_phi.add_incoming(&[(&lsr_node, lsr_int_cow), (&lsr_new_int, lsr_int_inc_done)]);
        let lsr_work_node = lsr_work_phi.as_basic_value().into_pointer_value();
        let lsr_work_i8 = self
            .builder
            .build_pointer_cast(lsr_work_node, ptr, "work_i8")
            .map_err(llvm_err)?;
        let lsr_upd_ce_base = unsafe {
            self.builder
                .build_gep(i8, lsr_work_i8, &[i64.const_int(16, false)], "upb")
                .map_err(llvm_err)?
        };
        let lsr_upd_ce_ptr = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    lsr_upd_ce_base,
                    &[lsr_found_i],
                    "upcep",
                )
                .map_err(llvm_err)?
        };
        let lsr_child_slot = self
            .builder
            .build_pointer_cast(lsr_upd_ce_ptr, ptr, "child_slot")
            .map_err(llvm_err)?;
        let lsr_child_changed = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                self.builder
                    .build_ptr_to_int(lsr_new_child, i64, "nc_i")
                    .map_err(llvm_err)?,
                self.builder
                    .build_ptr_to_int(lsr_old_child, i64, "oc_i")
                    .map_err(llvm_err)?,
                "child_changed",
            )
            .map_err(llvm_err)?;
        let lsr_dec_old = self.context.append_basic_block(lsr_fn, "dec_old");
        let lsr_store_child = self.context.append_basic_block(lsr_fn, "store_child");
        let _ =
            self.builder
                .build_conditional_branch(lsr_child_changed, lsr_dec_old, lsr_store_child);
        self.builder.position_at_end(lsr_dec_old);
        let lsr_old_child_rc_a = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(lsr_old_child, i64, "oc_int")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "oc_rc_a",
            )
            .map_err(llvm_err)?;
        let lsr_old_child_rc_p = self
            .builder
            .build_int_to_ptr(lsr_old_child_rc_a, ptr, "oc_rc_p")
            .map_err(llvm_err)?;
        let lsr_old_child_rc = self
            .builder
            .build_load(i64, lsr_old_child_rc_p, "oc_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let lsr_old_child_rc_dec = self
            .builder
            .build_int_sub(lsr_old_child_rc, i64.const_int(1, false), "oc_rc_dec")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lsr_old_child_rc_p, lsr_old_child_rc_dec)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_store_child);
        self.builder.position_at_end(lsr_store_child);
        let _ = self
            .builder
            .build_store(lsr_child_slot, lsr_new_child)
            .map_err(llvm_err)?;
        let lsr_inc_child = self.context.append_basic_block(lsr_fn, "inc_child");
        let lsr_after_inc = self.context.append_basic_block(lsr_fn, "after_inc");
        let _ =
            self.builder
                .build_conditional_branch(lsr_child_changed, lsr_inc_child, lsr_after_inc);
        self.builder.position_at_end(lsr_inc_child);
        let _ = self
            .builder
            .build_call(lsr_rc_inc_fn, &[lsr_new_child.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lsr_after_inc);
        self.builder.position_at_end(lsr_after_inc);
        let _ = self.builder.build_unconditional_branch(lsr_int_ret);

        self.builder.position_at_end(lsr_int_ret);
        let _ = self.builder.build_return(Some(&lsr_work_node));

        // ---- action_list_set({ptr, i64, i64}, i64, {i64, ptr}) -> {ptr, i64, i64} ----
        // Set element at index to value, CoW-safe. Returns new root.
        let list_set_fn = self.module.add_function(
            "action_list_set",
            self.list_type.fn_type(
                &[self.list_type.into(), i64.into(), self.string_type.into()],
                false,
            ),
            None,
        );
        let ls_entry = self.context.append_basic_block(list_set_fn, "entry");
        let ls_concat = self.context.append_basic_block(list_set_fn, "concat");
        let ls_concat_set_left = self
            .context
            .append_basic_block(list_set_fn, "concat_set_left");
        let ls_concat_set_right = self
            .context
            .append_basic_block(list_set_fn, "concat_set_right");
        let ls_normal = self.context.append_basic_block(list_set_fn, "normal");
        let ls_h0 = self.context.append_basic_block(list_set_fn, "h0");
        let ls_h0_cow = self.context.append_basic_block(list_set_fn, "h0_cow");
        let ls_h0_store = self.context.append_basic_block(list_set_fn, "h0_store");
        let ls_hgt0 = self.context.append_basic_block(list_set_fn, "hgt0");

        self.builder.position_at_end(ls_entry);
        let ls_list = list_set_fn.get_first_param().unwrap().into_struct_value();
        let ls_idx = list_set_fn.get_nth_param(1).unwrap().into_int_value();
        let ls_val = list_set_fn.get_nth_param(2).unwrap().into_struct_value();
        let ls_height = self
            .builder
            .build_extract_value(ls_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ls_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ls_is_concat, ls_concat, ls_normal);
        // ConcatNode: lazy dispatch — set in left/right subtree, rebuild via concat
        self.builder.position_at_end(ls_concat);
        let ls_node = self
            .builder
            .build_extract_value(ls_list, 0, "cn_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ls_concat_fn = self.module.get_function("action_list_concat").unwrap();
        let ls_cn_ln_p = unsafe {
            self.builder
                .build_gep(ptr, ls_node, &[i64.const_int(2, false)], "cn_ln_p")
                .map_err(llvm_err)
        }?;
        let ls_cn_left_node = self
            .builder
            .build_load(ptr, ls_cn_ln_p, "cn_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ls_cn_ll_p = unsafe {
            self.builder
                .build_gep(i64, ls_node, &[i64.const_int(3, false)], "cn_ll_p")
                .map_err(llvm_err)
        }?;
        let ls_cn_left_len = self
            .builder
            .build_load(i64, ls_cn_ll_p, "cn_ll")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_cn_lh_p = unsafe {
            self.builder
                .build_gep(i64, ls_node, &[i64.const_int(4, false)], "cn_lh_p")
                .map_err(llvm_err)
        }?;
        let ls_cn_left_h = self
            .builder
            .build_load(i64, ls_cn_lh_p, "cn_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_cn_l_undef = self.list_type.get_undef();
        let ls_cn_l1 = self
            .builder
            .build_insert_value(ls_cn_l_undef, ls_cn_left_node, 0, "cn_l1")
            .map_err(llvm_err)?;
        let ls_cn_l2 = self
            .builder
            .build_insert_value(ls_cn_l1, ls_cn_left_len, 1, "cn_l2")
            .map_err(llvm_err)?;
        let ls_cn_left = self
            .builder
            .build_insert_value(ls_cn_l2, ls_cn_left_h, 2, "cn_left")
            .map_err(llvm_err)?
            .into_struct_value();
        let ls_cn_rn_p = unsafe {
            self.builder
                .build_gep(ptr, ls_node, &[i64.const_int(5, false)], "cn_rn_p")
                .map_err(llvm_err)
        }?;
        let ls_cn_right_node = self
            .builder
            .build_load(ptr, ls_cn_rn_p, "cn_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ls_cn_rl_p = unsafe {
            self.builder
                .build_gep(i64, ls_node, &[i64.const_int(6, false)], "cn_rl_p")
                .map_err(llvm_err)
        }?;
        let ls_cn_right_len = self
            .builder
            .build_load(i64, ls_cn_rl_p, "cn_rl")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_cn_rh_p = unsafe {
            self.builder
                .build_gep(i64, ls_node, &[i64.const_int(7, false)], "cn_rh_p")
                .map_err(llvm_err)
        }?;
        let ls_cn_right_h = self
            .builder
            .build_load(i64, ls_cn_rh_p, "cn_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_cn_r_undef = self.list_type.get_undef();
        let ls_cn_r1 = self
            .builder
            .build_insert_value(ls_cn_r_undef, ls_cn_right_node, 0, "cn_r1")
            .map_err(llvm_err)?;
        let ls_cn_r2 = self
            .builder
            .build_insert_value(ls_cn_r1, ls_cn_right_len, 1, "cn_r2")
            .map_err(llvm_err)?;
        let ls_cn_right = self
            .builder
            .build_insert_value(ls_cn_r2, ls_cn_right_h, 2, "cn_right")
            .map_err(llvm_err)?
            .into_struct_value();
        let ls_cn_lt_left = self
            .builder
            .build_int_compare(IntPredicate::SLT, ls_idx, ls_cn_left_len, "cn_lt_l")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            ls_cn_lt_left,
            ls_concat_set_left,
            ls_concat_set_right,
        );
        self.builder.position_at_end(ls_concat_set_left);
        let ls_cn_new_left = self
            .builder
            .build_call(
                list_set_fn,
                &[ls_cn_left.into(), ls_idx.into(), ls_val.into()],
                "cn_nl",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let ls_cn_lr = self
            .builder
            .build_call(
                ls_concat_fn,
                &[ls_cn_new_left.into(), ls_cn_right.into()],
                "cn_lr",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&ls_cn_lr));
        self.builder.position_at_end(ls_concat_set_right);
        let ls_cn_new_idx = self
            .builder
            .build_int_sub(ls_idx, ls_cn_left_len, "cn_ni")
            .map_err(llvm_err)?;
        let ls_cn_new_right = self
            .builder
            .build_call(
                list_set_fn,
                &[ls_cn_right.into(), ls_cn_new_idx.into(), ls_val.into()],
                "cn_nr",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let ls_cn_rr = self
            .builder
            .build_call(
                ls_concat_fn,
                &[ls_cn_left.into(), ls_cn_new_right.into()],
                "cn_rr",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&ls_cn_rr));
        // Normal path
        self.builder.position_at_end(ls_normal);
        let ls_node = self
            .builder
            .build_extract_value(ls_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ls_len = self
            .builder
            .build_extract_value(ls_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_h = self
            .builder
            .build_extract_value(ls_list, 2, "h")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, ls_h, zero, "is_h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ls_is_h0, ls_h0, ls_hgt0);

        // Height == 0: direct manipulation
        self.builder.position_at_end(ls_h0);
        let ls_node_int = self
            .builder
            .build_ptr_to_int(ls_node, i64, "node_int")
            .map_err(llvm_err)?;
        let ls_rc_a = self
            .builder
            .build_int_sub(ls_node_int, i64.const_int(8, false), "rc_a")
            .map_err(llvm_err)?;
        let ls_rc_p = self
            .builder
            .build_int_to_ptr(ls_rc_a, ptr, "rc_p")
            .map_err(llvm_err)?;
        let ls_rc = self
            .builder
            .build_load(i64, ls_rc_p, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let ls_cow = self
            .builder
            .build_int_compare(IntPredicate::SGT, ls_rc, i64.const_int(1, false), "cow")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ls_cow, ls_h0_cow, ls_h0_store);

        self.builder.position_at_end(ls_h0_cow);
        let ls_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let ls_new = self
            .builder
            .build_call(malloc_rc_fn, &[ls_leaf_sz.into()], "new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let ls_cpy = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                ls_cpy,
                &[ls_new.into(), ls_node.into(), ls_leaf_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let ls_new_rc = self
            .builder
            .build_int_sub(ls_rc, i64.const_int(1, false), "new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(ls_rc_p, ls_new_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ls_h0_store);

        self.builder.position_at_end(ls_h0_store);
        let ls_phi = self.builder.build_phi(ptr, "leaf_phi").map_err(llvm_err)?;
        ls_phi.add_incoming(&[(&ls_node, ls_h0), (&ls_new, ls_h0_cow)]);
        let ls_leaf = ls_phi.as_basic_value().into_pointer_value();
        let ls_li8 = self
            .builder
            .build_pointer_cast(ls_leaf, ptr, "li8")
            .map_err(llvm_err)?;
        let ls_eb = unsafe {
            self.builder
                .build_gep(i8, ls_li8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)?
        };
        let ls_ep = unsafe {
            self.builder
                .build_gep(self.string_type, ls_eb, &[ls_idx], "ep")
                .map_err(llvm_err)?
        };
        let _ = self.builder.build_store(ls_ep, ls_val).map_err(llvm_err)?;
        let ls_undef = self.list_type.get_undef();
        let ls_r1 = self
            .builder
            .build_insert_value(ls_undef, ls_leaf, 0, "r1")
            .map_err(llvm_err)?;
        let ls_r2 = self
            .builder
            .build_insert_value(ls_r1, ls_len, 1, "r2")
            .map_err(llvm_err)?;
        let ls_r3 = self
            .builder
            .build_insert_value(ls_r2, zero, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ls_r3));

        // Height > 0: B-tree path-copy via action_list_set_rec
        self.builder.position_at_end(ls_hgt0);
        let ls_set_rec_fn = self.module.get_function("action_list_set_rec").unwrap();
        let ls_new_root = self
            .builder
            .build_call(
                ls_set_rec_fn,
                &[ls_node.into(), ls_h.into(), ls_idx.into(), ls_val.into()],
                "new_root",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let ls_undef_h = self.list_type.get_undef();
        let ls_hr1 = self
            .builder
            .build_insert_value(ls_undef_h, ls_new_root, 0, "hr1")
            .map_err(llvm_err)?;
        let ls_hr2 = self
            .builder
            .build_insert_value(ls_hr1, ls_len, 1, "hr2")
            .map_err(llvm_err)?;
        let ls_hr3 = self
            .builder
            .build_insert_value(ls_hr2, ls_h, 2, "hr3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ls_hr3));
