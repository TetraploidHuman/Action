// ---- action_list_get({ptr, i64, i64}, i64) -> {i64, ptr} ----
        // Block-based: traverse tree to find element at index.
        let list_get_fn = self.module.get_function("action_list_get").unwrap();
        let lg_entry = self.context.append_basic_block(list_get_fn, "entry");
        let lg_concat_loop = self.context.append_basic_block(list_get_fn, "concat_loop");
        let lg_h0 = self.context.append_basic_block(list_get_fn, "h0");
        let lg_h0_body = self.context.append_basic_block(list_get_fn, "h0_body");
        let lg_hgt0 = self.context.append_basic_block(list_get_fn, "hgt0");
        let lg_hgt0_loop = self.context.append_basic_block(list_get_fn, "hgt0_loop");
        let lg_hgt0_found = self.context.append_basic_block(list_get_fn, "hgt0_found");
        let lg_hgt0_next = self.context.append_basic_block(list_get_fn, "hgt0_next");
        let lg_ret = self.context.append_basic_block(list_get_fn, "ret");
        self.builder.position_at_end(lg_entry);
        let list = list_get_fn.get_first_param().unwrap().into_struct_value();
        let idx = list_get_fn.get_nth_param(1).unwrap().into_int_value();
        let node_ptr = self
            .builder
            .build_extract_value(list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let height = self
            .builder
            .build_extract_value(list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        // Check if ConcatNode (height == -1) — delegate through ConcatNode chain
        let is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_concat, lg_concat_loop, lg_h0);

        // ConcatNode delegation loop: use cached left_len in ConcatNode, descend in O(depth)
        self.builder.position_at_end(lg_concat_loop);
        let lg_phi_node = self.builder.build_phi(ptr, "lg_phi_n").map_err(llvm_err)?;
        let lg_phi_idx = self.builder.build_phi(i64, "lg_phi_i").map_err(llvm_err)?;
        lg_phi_node.add_incoming(&[(&node_ptr, lg_entry)]);
        lg_phi_idx.add_incoming(&[(&idx, lg_entry)]);
        let cc_node = lg_phi_node.as_basic_value().into_pointer_value();
        let cc_idx = lg_phi_idx.as_basic_value().into_int_value();
        // Cached left subtree size at ConcatNode offset 3 (left list len field)
        let cc_left_len_p = unsafe {
            self.builder
                .build_gep(i64, cc_node, &[i64.const_int(3, false)], "cc_llp")
                .map_err(llvm_err)
        }?;
        let cc_left_len = self
            .builder
            .build_load(i64, cc_left_len_p, "cc_ll")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_go_left = self
            .builder
            .build_int_compare(IntPredicate::SLT, cc_idx, cc_left_len, "cc_gl")
            .map_err(llvm_err)?;
        let cc_left_node_p = unsafe {
            self.builder
                .build_gep(ptr, cc_node, &[i64.const_int(2, false)], "cc_lnp")
                .map_err(llvm_err)
        }?;
        let cc_left_node = self
            .builder
            .build_load(ptr, cc_left_node_p, "cc_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc_left_h_p = unsafe {
            self.builder
                .build_gep(i64, cc_node, &[i64.const_int(4, false)], "cc_lhp")
                .map_err(llvm_err)
        }?;
        let cc_left_h = self
            .builder
            .build_load(i64, cc_left_h_p, "cc_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_right_node_p = unsafe {
            self.builder
                .build_gep(ptr, cc_node, &[i64.const_int(5, false)], "cc_rnp")
                .map_err(llvm_err)
        }?;
        let cc_right_node = self
            .builder
            .build_load(ptr, cc_right_node_p, "cc_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc_right_h_p = unsafe {
            self.builder
                .build_gep(i64, cc_node, &[i64.const_int(7, false)], "cc_rhp")
                .map_err(llvm_err)
        }?;
        let cc_right_h = self
            .builder
            .build_load(i64, cc_right_h_p, "cc_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_right_idx = self
            .builder
            .build_int_sub(cc_idx, cc_left_len, "cc_ni")
            .map_err(llvm_err)?;
        let cc_next_node = self
            .builder
            .build_select(cc_go_left, cc_left_node, cc_right_node, "cc_nn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc_next_h = self
            .builder
            .build_select(cc_go_left, cc_left_h, cc_right_h, "cc_nh")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_next_idx = self
            .builder
            .build_select(cc_go_left, cc_idx, cc_right_idx, "cc_ni2")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_neg1 = i64.const_int(-1i64 as u64, true);
        let cc_child_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, cc_next_h, cc_neg1, "cc_cic")
            .map_err(llvm_err)?;
        lg_phi_node.add_incoming(&[(&cc_next_node, lg_concat_loop)]);
        lg_phi_idx.add_incoming(&[(&cc_next_idx, lg_concat_loop)]);
        let _ = self
            .builder
            .build_conditional_branch(cc_child_is_concat, lg_concat_loop, lg_h0);
        let zero = i64.const_int(0, false);

        // Height == 0: single leaf, direct access
        // Phi nodes for resolved node, height, idx from entry and concat descent
        self.builder.position_at_end(lg_h0);
        let lg_resolved_node = self.builder.build_phi(ptr, "lg_rn").map_err(llvm_err)?;
        let lg_resolved_h = self.builder.build_phi(i64, "lg_rh").map_err(llvm_err)?;
        let lg_resolved_idx = self.builder.build_phi(i64, "lg_ri").map_err(llvm_err)?;
        lg_resolved_node.add_incoming(&[(&node_ptr, lg_entry)]);
        lg_resolved_h.add_incoming(&[(&height, lg_entry)]);
        lg_resolved_idx.add_incoming(&[(&idx, lg_entry)]);
        lg_resolved_node.add_incoming(&[(&cc_next_node, lg_concat_loop)]);
        lg_resolved_h.add_incoming(&[(&cc_next_h, lg_concat_loop)]);
        lg_resolved_idx.add_incoming(&[(&cc_next_idx, lg_concat_loop)]);
        let rn = lg_resolved_node.as_basic_value().into_pointer_value();
        let rh = lg_resolved_h.as_basic_value().into_int_value();
        let ri = lg_resolved_idx.as_basic_value().into_int_value();

        let is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, rh, zero, "is_h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_h0, lg_h0_body, lg_hgt0);

        // h=0 body
        self.builder.position_at_end(lg_h0_body);
        let leaf_i8 = self
            .builder
            .build_pointer_cast(rn, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let elem_base = unsafe {
            self.builder
                .build_gep(i8, leaf_i8, &[i64.const_int(8, false)], "elem_base")
                .map_err(llvm_err)?
        };
        let elem_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, elem_base, &[ri], "elem_ptr")
                .map_err(llvm_err)?
        };
        let elem_val = self
            .builder
            .build_load(self.string_type, elem_ptr, "elem")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lg_ret);

        // Height > 0: traverse internal nodes
        // current_node = rn; remaining_height = rh; remaining_idx = ri
        self.builder.position_at_end(lg_hgt0);
        let _ = self.builder.build_unconditional_branch(lg_hgt0_loop);

        // Loop: iterate through internal nodes using subtree_total
        self.builder.position_at_end(lg_hgt0_loop);
        // Phi: {current_node, remaining_height, remaining_idx}
        let phi_node = self.builder.build_phi(ptr, "phi_node").map_err(llvm_err)?;
        let phi_height = self
            .builder
            .build_phi(i64, "phi_height")
            .map_err(llvm_err)?;
        let phi_idx = self.builder.build_phi(i64, "phi_idx").map_err(llvm_err)?;
        phi_node.add_incoming(&[(&rn, lg_hgt0)]);
        phi_height.add_incoming(&[(&rh, lg_hgt0)]);
        phi_idx.add_incoming(&[(&ri, lg_hgt0)]);
        let cur_node = phi_node.as_basic_value().into_pointer_value();
        let cur_height = phi_height.as_basic_value().into_int_value();
        let cur_idx = phi_idx.as_basic_value().into_int_value();
        // If height == 0, we've reached a leaf
        let is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, cur_height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_leaf, lg_hgt0_found, lg_hgt0_next);

        // Found leaf: load element
        self.builder.position_at_end(lg_hgt0_found);
        let found_leaf_i8 = self
            .builder
            .build_pointer_cast(cur_node, ptr, "fl_i8")
            .map_err(llvm_err)?;
        let found_elem_base = unsafe {
            self.builder
                .build_gep(i8, found_leaf_i8, &[i64.const_int(8, false)], "feb")
                .map_err(llvm_err)?
        };
        let found_elem_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, found_elem_base, &[cur_idx], "fe_p")
                .map_err(llvm_err)?
        };
        let found_elem = self
            .builder
            .build_load(self.string_type, found_elem_ptr, "fe")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lg_ret);

        // Internal node: find which child contains the index
        // children array at offset 16 (after i32 count + i32 pad + i64 total)
        // child_entry = {ptr child, i64 subtree_total}
        self.builder.position_at_end(lg_hgt0_next);
        let intl_i8 = self
            .builder
            .build_pointer_cast(cur_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let intl_count_raw = self
            .builder
            .build_load(i32, intl_i8, "intl_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let intl_count = self
            .builder
            .build_int_z_extend(intl_count_raw, i64, "intl_count")
            .map_err(llvm_err)?;
        // Iterate children: for i in 0..count, check if idx < child[i].subtree_total
        // For simplicity, scan linearly (B=64, so at most 64 iterations)
        // Use a loop or just unrolled scan
        // Store result: (child_ptr, child_subtree_total, child_idx)
        // For now: simple linear scan in a loop
        let scan_loop = self.context.append_basic_block(list_get_fn, "scan_loop");
        let scan_body = self.context.append_basic_block(list_get_fn, "scan_body");
        let scan_found = self.context.append_basic_block(list_get_fn, "scan_found");
        let scan_next = self.context.append_basic_block(list_get_fn, "scan_next");
        let _ = self.builder.build_unconditional_branch(scan_loop);
        self.builder.position_at_end(scan_loop);
        let phi_i = self.builder.build_phi(i64, "phi_i").map_err(llvm_err)?;
        let phi_acc = self.builder.build_phi(i64, "phi_acc").map_err(llvm_err)?;
        phi_i.add_incoming(&[(&zero, lg_hgt0_next)]);
        phi_acc.add_incoming(&[(&zero, lg_hgt0_next)]);
        let scan_i = phi_i.as_basic_value().into_int_value();
        let scan_acc = phi_acc.as_basic_value().into_int_value();
        let done_scan = self
            .builder
            .build_int_compare(IntPredicate::SGE, scan_i, intl_count, "done_scan")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_scan, scan_found, scan_body);

        self.builder.position_at_end(scan_body);
        // Load child[scan_i].subtree_total
        let scan_children_base = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "scb")
                .map_err(llvm_err)?
        };
        let child_entry_ptr = unsafe {
            self.builder
                .build_gep(self.child_entry_type, scan_children_base, &[scan_i], "cep")
                .map_err(llvm_err)?
        };
        let child_total = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, child_entry_ptr, "ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                1,
                "ct",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let new_acc = self
            .builder
            .build_int_add(scan_acc, child_total, "new_acc")
            .map_err(llvm_err)?;
        let found_child = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur_idx, new_acc, "found_child")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(found_child, scan_found, scan_next);

        self.builder.position_at_end(scan_next);
        let next_i = self
            .builder
            .build_int_add(scan_i, i64.const_int(1, false), "next_i")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(scan_loop);
        phi_i.add_incoming(&[(&next_i, scan_next)]);
        phi_acc.add_incoming(&[(&new_acc, scan_next)]);

        self.builder.position_at_end(scan_found);
        // phi for the found child index and accumulated offset before this child
        let phi_found_i = self.builder.build_phi(i64, "phi_fi").map_err(llvm_err)?;
        let phi_found_acc = self.builder.build_phi(i64, "phi_fa").map_err(llvm_err)?;
        phi_found_i.add_incoming(&[(&scan_i, scan_body), (&scan_i, scan_loop)]);
        // The accumulated offset before this child is scan_acc (not new_acc)
        phi_found_acc.add_incoming(&[(&scan_acc, scan_body), (&scan_acc, scan_loop)]);
        let found_i = phi_found_i.as_basic_value().into_int_value();
        let offset_before = phi_found_acc.as_basic_value().into_int_value();
        // Load child[found_i].ptr
        let found_ce_base = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "fceb")
                .map_err(llvm_err)?
        };
        let found_ce_ptr = unsafe {
            self.builder
                .build_gep(self.child_entry_type, found_ce_base, &[found_i], "fcep")
                .map_err(llvm_err)?
        };
        let found_ce = self
            .builder
            .build_load(self.child_entry_type, found_ce_ptr, "fce")
            .map_err(llvm_err)?
            .into_struct_value();
        let child_ptr = self
            .builder
            .build_extract_value(found_ce, 0, "child_p")
            .map_err(llvm_err)?
            .into_pointer_value();
        let new_idx = self
            .builder
            .build_int_sub(cur_idx, offset_before, "new_idx")
            .map_err(llvm_err)?;
        let new_height = self
            .builder
            .build_int_sub(cur_height, i64.const_int(1, false), "new_h")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lg_hgt0_loop);
        phi_node.add_incoming(&[(&child_ptr, scan_found)]);
        phi_height.add_incoming(&[(&new_height, scan_found)]);
        phi_idx.add_incoming(&[(&new_idx, scan_found)]);

        // Return
        self.builder.position_at_end(lg_ret);
        let phi_ret = self
            .builder
            .build_phi(self.string_type, "phi_ret")
            .map_err(llvm_err)?;
        phi_ret.add_incoming(&[(&elem_val, lg_h0_body), (&found_elem, lg_hgt0_found)]);
        let _ = self.builder.build_return(Some(&phi_ret.as_basic_value()));
