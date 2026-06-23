        let elem_base_x = unsafe {
            self.builder
                .build_gep(i8, leaf_bytes, &[i64.const_int(8, false)], "elem_base")
                .map_err(llvm_err)
        }?;
        let intl_total_ptr = unsafe {
            self.builder
                .build_gep(i64, intl_base_i8, &[i64.const_int(1, false)], "intl_total")
                .map_err(llvm_err)
        }?;
        let intl_old_total = self
            .builder
            .build_load(i64, intl_total_ptr, "old_total")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_full = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                leaf_count,
                i64.const_int(64, false),
                "leaf_full",
            )
            .map_err(llvm_err)?;
        let lp_store_leaf = self
            .context
            .append_basic_block(list_push_fn, "lp_store_leaf");
        let lp_split_leaf = self
            .context
            .append_basic_block(list_push_fn, "lp_split_leaf");
        let _ = self
            .builder
            .build_conditional_branch(leaf_full, lp_split_leaf, lp_store_leaf);
        // Store element in leaf (has room)
        self.builder.position_at_end(lp_store_leaf);
        let elem_slot = unsafe {
            self.builder
                .build_gep(self.string_type, elem_base_x, &[leaf_count], "elem_slot")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(elem_slot, elem)
            .map_err(llvm_err)?;
        let new_leaf_count = self
            .builder
            .build_int_add(leaf_count, i64.const_int(1, false), "new_lc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(leaf_bytes, new_leaf_count)
            .map_err(llvm_err)?;
        // Update subtree_total
        let new_st = self
            .builder
            .build_int_add(subtree_total, i64.const_int(1, false), "new_st")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(subtree_total_ptr, new_st)
            .map_err(llvm_err)?;
        // Update internal total
        let intl_new_total = self
            .builder
            .build_int_add(intl_old_total, i64.const_int(1, false), "new_total")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_total_ptr, intl_new_total)
            .map_err(llvm_err)?;
        // Update parent if we descended from height > 1
        let lp_st_slot_val = self
            .builder
            .build_load(ptr, lp_parent_ptr, "st_slot_val")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lp_has_parent = self
            .builder
            .build_int_compare(IntPredicate::NE, lp_st_slot_val, null_ptr, "has_parent")
            .map_err(llvm_err)?;
        let lp_do_parent = self
            .context
            .append_basic_block(list_push_fn, "lp_do_parent");
        let lp_parent_done = self
            .context
            .append_basic_block(list_push_fn, "lp_parent_done");
        let _ = self
            .builder
            .build_conditional_branch(lp_has_parent, lp_do_parent, lp_parent_done);
        self.builder.position_at_end(lp_do_parent);
        let st_cur = self
            .builder
            .build_load(i64, lp_st_slot_val, "st_cur")
            .map_err(llvm_err)?
            .into_int_value();
        let st_new = self
            .builder
            .build_int_add(st_cur, i64.const_int(1, false), "st_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lp_st_slot_val, st_new)
            .map_err(llvm_err)?;
        let pn_val = self
            .builder
            .build_load(ptr, lp_parent_node, "pn_val")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pn_tp = unsafe {
            self.builder
                .build_gep(i64, pn_val, &[i64.const_int(1, false)], "pn_tp")
                .map_err(llvm_err)
        }?;
        let pn_tot = self
            .builder
            .build_load(i64, pn_tp, "pn_tot")
            .map_err(llvm_err)?
            .into_int_value();
        let pn_tot_new = self
            .builder
            .build_int_add(pn_tot, i64.const_int(1, false), "pn_tot_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pn_tp, pn_tot_new)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_parent_done);
        self.builder.position_at_end(lp_parent_done);
        let new_list_len = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_len")
            .map_err(llvm_err)?;
        let undef_hgt0 = self.list_type.get_undef();
        let r_hgt0_1 = self
            .builder
            .build_insert_value(undef_hgt0, node_ptr, 0, "r1")
            .map_err(llvm_err)?;
        let r_hgt0_2 = self
            .builder
            .build_insert_value(r_hgt0_1, new_list_len, 1, "r2")
            .map_err(llvm_err)?;
        let r_hgt0_3 = self
            .builder
            .build_insert_value(r_hgt0_2, height, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r_hgt0_3));
        // Leaf full: split rightmost leaf, handle internal overflow by creating new root
        self.builder.position_at_end(lp_split_leaf);
        let leaf_size_val2 = leaf_ty.size_of().ok_or("leaf size2")?;
        let new_leaf2_gt = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size_val2.into()], "nl2_gt")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Copy elements[32..64] to new leaf
        let src_elem32_gt = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    elem_base_x,
                    &[i64.const_int(32, false)],
                    "src32_gt",
                )
                .map_err(llvm_err)
        }?;
        let nl2_bytes = self
            .builder
            .build_pointer_cast(new_leaf2_gt, ptr, "nl2_bytes")
            .map_err(llvm_err)?;
        let dst_elem_base = unsafe {
            self.builder
                .build_gep(i8, nl2_bytes, &[i64.const_int(8, false)], "dst_base_gt")
                .map_err(llvm_err)
        }?;
        let dst_elem0_gt = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    dst_elem_base,
                    &[i64.const_int(0, false)],
                    "dst0_gt",
                )
                .map_err(llvm_err)
        }?;
        let half_sz = i64.const_int(32 * 16, false);
        let _ = self
            .builder
            .build_call(
                self.module.get_function("memcpy").unwrap(),
                &[dst_elem0_gt.into(), src_elem32_gt.into(), half_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Store new element at new_leaf[32]
        let nl2_elem32_gt = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    dst_elem_base,
                    &[i64.const_int(32, false)],
                    "nl2e32_gt",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nl2_elem32_gt, elem)
            .map_err(llvm_err)?;
        // Set counts
        let _ = self
            .builder
            .build_store(leaf_bytes, i64.const_int(32, false))
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2_bytes, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Update original child's subtree_total to 32
        let _ = self
            .builder
            .build_store(subtree_total_ptr, i64.const_int(32, false))
            .map_err(llvm_err)?;
        // Set RC of new_leaf2_gt to 1
        let nl2g_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(new_leaf2_gt, i64, "nl2g_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "nl2g_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "nl2g_rc_p",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2g_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Check if internal node is full (count >= 64)
        let intl_full = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                intl_count,
                i64.const_int(64, false),
                "intl_full",
            )
            .map_err(llvm_err)?;
        let lp_add_child = self
            .context
            .append_basic_block(list_push_fn, "lp_add_child");
        let lp_split_intl = self
            .context
            .append_basic_block(list_push_fn, "lp_split_intl");
        let _ = self
            .builder
            .build_conditional_branch(intl_full, lp_split_intl, lp_add_child);

        // Internal node has room: add new child normally
        self.builder.position_at_end(lp_add_child);
        let new_child_idx = intl_count;
        let new_child_slot = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[new_child_idx],
                    "new_child",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_child_slot, new_leaf2_gt)
            .map_err(llvm_err)?;
        let nc_st_ptr = unsafe {
            self.builder
                .build_gep(i64, new_child_slot, &[i64.const_int(1, false)], "nc_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nc_st_ptr, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // RC-inc new_leaf2_gt (internal node now references it, one more reference)
        let nl2g_rc2 = self
            .builder
            .build_load(i64, nl2g_rc_ptr, "nl2g_rc2")
            .map_err(llvm_err)?
            .into_int_value();
        let nl2g_rc3 = self
            .builder
            .build_int_add(nl2g_rc2, i64.const_int(1, false), "nl2g_rc3")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2g_rc_ptr, nl2g_rc3)
            .map_err(llvm_err)?;
        let new_intl_count = self
            .builder
            .build_int_add(intl_count, i64.const_int(1, false), "new_intl_count")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_base_i8, new_intl_count)
            .map_err(llvm_err)?;
        // Update internal total
        let new_intl_total = self
            .builder
            .build_int_add(intl_old_total, i64.const_int(1, false), "new_intl_total")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_total_ptr, new_intl_total)
            .map_err(llvm_err)?;
        // Update parent if we descended from height > 1
        let lp_st_slot_val2 = self
            .builder
            .build_load(ptr, lp_parent_ptr, "st_slot_val2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lp_has_parent2 = self
            .builder
            .build_int_compare(IntPredicate::NE, lp_st_slot_val2, null_ptr, "has_parent2")
            .map_err(llvm_err)?;
        let lp_do_parent2 = self
            .context
            .append_basic_block(list_push_fn, "lp_do_parent2");
        let lp_parent_done2 = self
            .context
            .append_basic_block(list_push_fn, "lp_parent_done2");
        let _ =
            self.builder
                .build_conditional_branch(lp_has_parent2, lp_do_parent2, lp_parent_done2);
        self.builder.position_at_end(lp_do_parent2);
        let st_cur2 = self
            .builder
            .build_load(i64, lp_st_slot_val2, "st_cur2")
            .map_err(llvm_err)?
            .into_int_value();
        let st_new2 = self
            .builder
            .build_int_add(st_cur2, i64.const_int(1, false), "st_new2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lp_st_slot_val2, st_new2)
            .map_err(llvm_err)?;
        let pn_val2 = self
            .builder
            .build_load(ptr, lp_parent_node, "pn_val2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pn_tp2 = unsafe {
            self.builder
                .build_gep(i64, pn_val2, &[i64.const_int(1, false)], "pn_tp2")
                .map_err(llvm_err)
        }?;
        let pn_tot2 = self
            .builder
            .build_load(i64, pn_tp2, "pn_tot2")
            .map_err(llvm_err)?
            .into_int_value();
        let pn_tot_new2 = self
            .builder
            .build_int_add(pn_tot2, i64.const_int(1, false), "pn_tot_new2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pn_tp2, pn_tot_new2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_parent_done2);
        self.builder.position_at_end(lp_parent_done2);
        let new_list_len2 = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_len2")
            .map_err(llvm_err)?;
        let undef_hgt0b = self.list_type.get_undef();
        let r_hgt0b_1 = self
            .builder
            .build_insert_value(undef_hgt0b, node_ptr, 0, "rb1")
            .map_err(llvm_err)?;
        let r_hgt0b_2 = self
            .builder
            .build_insert_value(r_hgt0b_1, new_list_len2, 1, "rb2")
            .map_err(llvm_err)?;
        let r_hgt0b_3 = self
            .builder
            .build_insert_value(r_hgt0b_2, height, 2, "rb3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r_hgt0b_3));

        // Internal node is full: create new internal sibling or new root
        self.builder.position_at_end(lp_split_intl);
        // The rightmost leaf's subtree_total changed from subtree_total to 32.
        // Fix intl_base's total: intl_old_total - subtree_total + 32
        let thirty2 = i64.const_int(32, false);
        let intl_st_delta = self
            .builder
            .build_int_sub(subtree_total, thirty2, "st_delta")
            .map_err(llvm_err)?;
        let intl_corrected_total = self
            .builder
            .build_int_sub(intl_old_total, intl_st_delta, "corrected_total")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_total_ptr, intl_corrected_total)
            .map_err(llvm_err)?;
        // Allocate new internal node for the split-off right side
        let internal_size = self.internal_type.size_of().ok_or("internal size")?;
        let new_intl = self
            .builder
            .build_call(malloc_rc_fn, &[internal_size.into()], "new_intl")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let new_intl_i8 = self
            .builder
            .build_pointer_cast(new_intl, ptr, "new_intl_i8")
            .map_err(llvm_err)?;
        // Set new_intl count = 1
        let _ = self
            .builder
            .build_store(new_intl_i8, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Set new_intl total = 33
        let new_intl_tp = unsafe {
            self.builder
                .build_gep(i64, new_intl_i8, &[i64.const_int(1, false)], "nitp")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_intl_tp, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Set new_intl children[0] = {new_leaf2_gt, 33}
        let new_intl_cbase = unsafe {
            self.builder
                .build_gep(i8, new_intl_i8, &[i64.const_int(16, false)], "nicbase")
                .map_err(llvm_err)
        }?;
        let new_intl_c0 = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    new_intl_cbase,
                    &[i64.const_int(0, false)],
                    "nic0",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_intl_c0, new_leaf2_gt)
            .map_err(llvm_err)?;
        let nic0_st = unsafe {
            self.builder
                .build_gep(i64, new_intl_c0, &[i64.const_int(1, false)], "nic0_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nic0_st, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // RC-inc new_leaf2_gt once more (new internal node references it)
        let nl2g_rc_v = self
            .builder
            .build_load(i64, nl2g_rc_ptr, "nl2g_rc_v")
            .map_err(llvm_err)?
            .into_int_value();
        let nl2g_rc_new = self
            .builder
            .build_int_add(nl2g_rc_v, i64.const_int(1, false), "nl2g_rc_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2g_rc_ptr, nl2g_rc_new)
            .map_err(llvm_err)?;
        // Compute RC pointers for later use
        let new_intl_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(new_intl, i64, "ni_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "ni_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "ni_rc_p",
            )
            .map_err(llvm_err)?;
        let intl_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(intl_base, i64, "intl_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "intl_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "intl_rc_p",
            )
            .map_err(llvm_err)?;
        // Check if we have a parent (original height > 1)
        let lp_st_slot_val3 = self
            .builder
            .build_load(ptr, lp_parent_ptr, "st_slot_val3")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lp_has_parent3 = self
            .builder
            .build_int_compare(IntPredicate::NE, lp_st_slot_val3, null_ptr, "has_parent3")
            .map_err(llvm_err)?;
        let lp_split_has_parent = self
            .context
            .append_basic_block(list_push_fn, "split_has_parent");
        let lp_split_no_parent = self
            .context
            .append_basic_block(list_push_fn, "split_no_parent");
        let _ = self.builder.build_conditional_branch(
            lp_has_parent3,
            lp_split_has_parent,
            lp_split_no_parent,
        );

        // Has parent: add new_intl as a new sibling child in the parent
        // This avoids creating new_mid and keeps tree heights consistent.
        self.builder.position_at_end(lp_split_has_parent);
        // Update parent's subtree_total for intl_base to corrected_total
        // (it changed because the rightmost leaf split: 64 -> 32)
        let _ = self
            .builder
            .build_store(lp_st_slot_val3, intl_corrected_total)
            .map_err(llvm_err)?;
        // Set RC of new_intl to 1 (parent will reference it)
        let _ = self
            .builder
            .build_store(new_intl_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Load parent node
        let pn_val3 = self
            .builder
            .build_load(ptr, lp_parent_node, "pn_val3")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pn_pc_raw = self
            .builder
            .build_load(i32, pn_val3, "pn_pc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let pn_count = self
            .builder
            .build_int_z_extend(pn_pc_raw, i64, "pn_count")
            .map_err(llvm_err)?;
        // Parent children array at offset 16
        let pn_cbase = unsafe {
            self.builder
                .build_gep(i8, pn_val3, &[i64.const_int(16, false)], "pn_cbase")
                .map_err(llvm_err)
        }?;
        // New child slot at children[pn_count]
        let pn_new_child = unsafe {
            self.builder
                .build_gep(self.child_entry_type, pn_cbase, &[pn_count], "pn_nc")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(pn_new_child, new_intl)
            .map_err(llvm_err)?;
        let pn_nc_st = unsafe {
            self.builder
                .build_gep(i64, pn_new_child, &[i64.const_int(1, false)], "pn_nc_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(pn_nc_st, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Update parent count
        let pn_new_count = self
            .builder
            .build_int_add(pn_count, i64.const_int(1, false), "pn_new_count")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pn_val3, pn_new_count)
            .map_err(llvm_err)?;
        // Update parent total += 1
        let pn_tp3 = unsafe {
            self.builder
                .build_gep(i64, pn_val3, &[i64.const_int(1, false)], "pn_tp3")
                .map_err(llvm_err)
        }?;
        let pn_tot3 = self
            .builder
            .build_load(i64, pn_tp3, "pn_tot3")
            .map_err(llvm_err)?
            .into_int_value();
        let pn_tot_new3 = self
            .builder
            .build_int_add(pn_tot3, i64.const_int(1, false), "pn_tot_new3")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pn_tp3, pn_tot_new3)
            .map_err(llvm_err)?;
        let new_list_len3 = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_len3")
            .map_err(llvm_err)?;
        let undef_split_p = self.list_type.get_undef();
        let r_split_p_1 = self
            .builder
            .build_insert_value(undef_split_p, node_ptr, 0, "rsp1")
            .map_err(llvm_err)?;
        let r_split_p_2 = self
            .builder
            .build_insert_value(r_split_p_1, new_list_len3, 1, "rsp2")
            .map_err(llvm_err)?;
        let r_split_p_3 = self
            .builder
            .build_insert_value(r_split_p_2, height, 2, "rsp3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r_split_p_3));

        // No parent (original height == 1): create new_mid as new root
        self.builder.position_at_end(lp_split_no_parent);
        // Set RC of new_intl to 1 — new_mid will reference it
        let _ = self
            .builder
            .build_store(new_intl_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        let new_mid = self
            .builder
            .build_call(malloc_rc_fn, &[internal_size.into()], "new_mid")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let new_mid_i8 = self
            .builder
            .build_pointer_cast(new_mid, ptr, "new_mid_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(new_mid_i8, i64.const_int(2, false))
            .map_err(llvm_err)?;
        let new_mid_tp = unsafe {
            self.builder
                .build_gep(i64, new_mid_i8, &[i64.const_int(1, false)], "nmid_tp")
                .map_err(llvm_err)
        }?;
        let thirty3 = i64.const_int(33, false);
        let new_mid_total = self
            .builder
            .build_int_add(intl_corrected_total, thirty3, "new_mid_total")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(new_mid_tp, new_mid_total)
            .map_err(llvm_err)?;
        let new_mid_cbase = unsafe {
            self.builder
                .build_gep(i8, new_mid_i8, &[i64.const_int(16, false)], "nmid_cbase")
                .map_err(llvm_err)
        }?;
        let new_mid_c0 = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    new_mid_cbase,
                    &[i64.const_int(0, false)],
                    "nmid_c0",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_mid_c0, intl_base)
            .map_err(llvm_err)?;
        let nmid_c0_st = unsafe {
            self.builder
                .build_gep(i64, new_mid_c0, &[i64.const_int(1, false)], "nmid_c0_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nmid_c0_st, intl_corrected_total)
            .map_err(llvm_err)?;
        // RC-inc intl_base (new_mid now references it)
        let intl_rc_v = self
            .builder
            .build_load(i64, intl_rc_ptr, "intl_rc_v")
            .map_err(llvm_err)?
            .into_int_value();
        let intl_rc_new = self
            .builder
            .build_int_add(intl_rc_v, i64.const_int(1, false), "intl_rc_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_rc_ptr, intl_rc_new)
            .map_err(llvm_err)?;
        let new_mid_c1 = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    new_mid_cbase,
                    &[i64.const_int(1, false)],
                    "nmid_c1",
                )
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(new_mid_c1, new_intl)
            .map_err(llvm_err)?;
        let nmid_c1_st = unsafe {
            self.builder
                .build_gep(i64, new_mid_c1, &[i64.const_int(1, false)], "nmid_c1_st")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(nmid_c1_st, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // RC-inc new_intl (new_mid references it, adds to the 1 already set)
        let ni_rc_np = self
            .builder
            .build_load(i64, new_intl_rc_ptr, "ni_rc_np")
            .map_err(llvm_err)?
            .into_int_value();
        let ni_rc_new = self
            .builder
            .build_int_add(ni_rc_np, i64.const_int(1, false), "ni_rc_new")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(new_intl_rc_ptr, ni_rc_new)
            .map_err(llvm_err)?;
        let new_h = self
            .builder
            .build_int_add(height, i64.const_int(1, false), "new_h")
            .map_err(llvm_err)?;
        let new_list_len4 = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_len4")
            .map_err(llvm_err)?;
        let undef_split = self.list_type.get_undef();
        let r_split_1 = self
            .builder
            .build_insert_value(undef_split, new_mid, 0, "rs1")
            .map_err(llvm_err)?;
        let r_split_2 = self
            .builder
            .build_insert_value(r_split_1, new_list_len4, 1, "rs2")
            .map_err(llvm_err)?;
        let r_split_3 = self
            .builder
            .build_insert_value(r_split_2, new_h, 2, "rs3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r_split_3));
