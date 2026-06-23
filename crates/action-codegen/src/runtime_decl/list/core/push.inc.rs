// ---- action_list_push({ptr, i64, i64}, {i64, ptr}) -> {ptr, i64, i64} ----
        // Block-based B-tree push. Supports height=0 (single leaf, common case).
        // Height>0 (internal node) will be added in follow-up.
        let list_push_fn = self.module.add_function(
            "action_list_push",
            self.list_type
                .fn_type(&[self.list_type.into(), self.string_type.into()], false),
            None,
        );
        let lp_entry = self.context.append_basic_block(list_push_fn, "entry");
        let lp_concat_append = self
            .context
            .append_basic_block(list_push_fn, "concat_append");
        let lp_normal = self.context.append_basic_block(list_push_fn, "normal");
        let lp_h0 = self.context.append_basic_block(list_push_fn, "h0");
        let lp_h0_cow = self.context.append_basic_block(list_push_fn, "h0_cow");
        let lp_h0_room = self.context.append_basic_block(list_push_fn, "h0_room");
        let lp_h0_full = self.context.append_basic_block(list_push_fn, "h0_full");
        let lp_h0_done = self.context.append_basic_block(list_push_fn, "h0_done");
        let lp_hgt0 = self.context.append_basic_block(list_push_fn, "hgt0");
        self.builder.position_at_end(lp_entry);
        let list = list_push_fn.get_first_param().unwrap().into_struct_value();
        let elem = list_push_fn.get_nth_param(1).unwrap().into_struct_value();
        let node_ptr = self
            .builder
            .build_extract_value(list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let total_len = self
            .builder
            .build_extract_value(list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let height = self
            .builder
            .build_extract_value(list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        // Check if ConcatNode — lazy append via concat(list, singleton(elem))
        let lp_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                height,
                i64.const_int(-1i64 as u64, true),
                "lp_ic",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lp_is_concat, lp_concat_append, lp_normal);
        // ConcatNode: lazy concat append (same as insert at index == len)
        self.builder.position_at_end(lp_concat_append);
        let lp_create_fn = self.module.get_function("action_list_create").unwrap();
        let lp_concat_fn = self.module.get_function("action_list_concat").unwrap();
        let lp_empty = self
            .builder
            .build_call(lp_create_fn, &[zero.into()], "lp_empty")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lp_sing = self
            .builder
            .build_call(list_push_fn, &[lp_empty.into(), elem.into()], "lp_sing")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lp_appended = self
            .builder
            .build_call(lp_concat_fn, &[list.into(), lp_sing.into()], "lp_appended")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lp_appended));
        // Normal (non-ConcatNode) path
        self.builder.position_at_end(lp_normal);
        let _lp_node2 = self
            .builder
            .build_extract_value(list, 0, "lp_n2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _lp_total2 = self
            .builder
            .build_extract_value(list, 1, "lp_t2")
            .map_err(llvm_err)?
            .into_int_value();
        let lp_h2 = self
            .builder
            .build_extract_value(list, 2, "lp_h2")
            .map_err(llvm_err)?
            .into_int_value();
        let is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, lp_h2, zero, "is_h0")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(is_h0, lp_h0, lp_hgt0);

        // === Height == 0: single leaf ===
        self.builder.position_at_end(lp_h0);
        let leaf_ty = self.leaf_type;
        let leaf_size_val = leaf_ty.size_of().ok_or("leaf size")?;
        // CoW check: read rc at leaf_ptr - 8
        let node_int = self
            .builder
            .build_ptr_to_int(node_ptr, i64, "node_int")
            .map_err(llvm_err)?;
        let rc_addr = self
            .builder
            .build_int_sub(node_int, i64.const_int(8, false), "rc_addr")
            .map_err(llvm_err)?;
        let rc_ptr = self
            .builder
            .build_int_to_ptr(rc_addr, ptr, "rc_ptr")
            .map_err(llvm_err)?;
        let rc_val = self
            .builder
            .build_load(i64, rc_ptr, "rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let need_cow = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                rc_val,
                i64.const_int(1, false),
                "need_cow",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(need_cow, lp_h0_cow, lp_h0_room);

        // CoW: copy leaf (do NOT decrement old RC — caller scope cleanup handles that)
        self.builder.position_at_end(lp_h0_cow);
        let new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size_val.into()], "new_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let cow_memcpy = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                cow_memcpy,
                &[new_leaf.into(), node_ptr.into(), leaf_size_val.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_h0_room);

        // Check if leaf has room: phi for leaf pointer
        self.builder.position_at_end(lp_h0_room);
        let phi_leaf = self.builder.build_phi(ptr, "phi_leaf").map_err(llvm_err)?;
        phi_leaf.add_incoming(&[(&node_ptr, lp_h0), (&new_leaf, lp_h0_cow)]);
        let leaf = phi_leaf.as_basic_value().into_pointer_value();
        // Read count at offset 0 of leaf (i32)
        let leaf_i8 = self
            .builder
            .build_pointer_cast(leaf, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let count_raw = self
            .builder
            .build_load(i32, leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let count_load = self
            .builder
            .build_int_z_extend(count_raw, i64, "count_val")
            .map_err(llvm_err)?;
        let is_full = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                count_load,
                i64.const_int(64, false),
                "is_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_full, lp_h0_full, lp_h0_done);

        // Leaf is full (64 elements): split into two leaves + create internal node
        self.builder.position_at_end(lp_h0_full);
        // Allocate new leaf for second half
        let new_leaf2 = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size_val.into()], "nl2")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Copy elements[32..64] from old leaf to new_leaf[0..32]
        // elements start at offset 8 in leaf struct
        let src_base = unsafe {
            self.builder
                .build_gep(i8, leaf_i8, &[i64.const_int(8, false)], "src_base")
                .map_err(llvm_err)
        }?;
        let src_elem32 = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    src_base,
                    &[i64.const_int(32, false)],
                    "src32",
                )
                .map_err(llvm_err)?
        };
        let nl2_i8 = self
            .builder
            .build_pointer_cast(new_leaf2, ptr, "nl2_i8")
            .map_err(llvm_err)?;
        let dst_base = unsafe {
            self.builder
                .build_gep(i8, nl2_i8, &[i64.const_int(8, false)], "dst_base")
                .map_err(llvm_err)
        }?;
        let dst_elem0 = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    dst_base,
                    &[i64.const_int(0, false)],
                    "dst0",
                )
                .map_err(llvm_err)?
        };
        let half_size = i64.const_int(32 * 16, false); // 32 elements * 16 bytes
        let _ = self
            .builder
            .build_call(
                cow_memcpy,
                &[dst_elem0.into(), src_elem32.into(), half_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Store new element at new_leaf[32]
        let nl2b = self
            .builder
            .build_pointer_cast(new_leaf2, ptr, "nl2b")
            .map_err(llvm_err)?;
        let nl2_elem_base = unsafe {
            self.builder
                .build_gep(i8, nl2b, &[i64.const_int(8, false)], "nl2_eb")
                .map_err(llvm_err)
        }?;
        let nl2_elem32 = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    nl2_elem_base,
                    &[i64.const_int(32, false)],
                    "nl2e32",
                )
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(nl2_elem32, elem)
            .map_err(llvm_err)?;
        // Set counts: old leaf = 32, new leaf = 33
        let _ = self
            .builder
            .build_store(leaf_i8, i64.const_int(32, false))
            .map_err(llvm_err)?;
        let nl2_count_p = self
            .builder
            .build_pointer_cast(new_leaf2, ptr, "nl2c")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2_count_p, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Create internal node with 2 children
        let internal_ty = self.internal_type;
        let internal_size = internal_ty.size_of().ok_or("internal size")?;
        let internal = self
            .builder
            .build_call(malloc_rc_fn, &[internal_size.into()], "intl")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Store count=2, total=65
        let intl_i8 = self
            .builder
            .build_pointer_cast(internal, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(intl_i8, i64.const_int(2, false))
            .map_err(llvm_err)?; // count at offset 0
                                 // total at offset 8 (after i32 count + i32 pad)
        let total_ptr = unsafe {
            self.builder
                .build_gep(i64, intl_i8, &[i64.const_int(1, false)], "total_p")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(total_ptr, i64.const_int(65, false))
            .map_err(llvm_err)?;
        // children array starts at offset 16 (after i32 count + i32 pad + i64 total)
        // child[0] = {leaf, 32}
        let children_base = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "children_base")
                .map_err(llvm_err)
        }?;
        let child0_ptr = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[i64.const_int(0, false)],
                    "c0",
                )
                .map_err(llvm_err)?
        };
        // child_entry = {ptr, i64} — store leaf ptr at offset 0, subtree_total at offset 8
        let c0_p = self
            .builder
            .build_pointer_cast(child0_ptr, ptr, "c0p")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(c0_p, leaf).map_err(llvm_err)?;
        let c0_t = unsafe {
            self.builder
                .build_gep(i64, c0_p, &[i64.const_int(1, false)], "c0t")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(c0_t, i64.const_int(32, false))
            .map_err(llvm_err)?;
        // child[1] = {new_leaf2, 33}
        let child1_ptr = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[i64.const_int(1, false)],
                    "c1",
                )
                .map_err(llvm_err)?
        };
        let c1_p = self
            .builder
            .build_pointer_cast(child1_ptr, ptr, "c1p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(c1_p, new_leaf2)
            .map_err(llvm_err)?;
        let c1_t = unsafe {
            self.builder
                .build_gep(i64, c1_p, &[i64.const_int(1, false)], "c1t")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(c1_t, i64.const_int(33, false))
            .map_err(llvm_err)?;
        // Increment RC of child[0] (old leaf or CoW copy) — internal node now references it
        // Without this, the caller's rc_dec on the old root frees a node still in the tree.
        let leaf_rc_ptr0 = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(leaf, i64, "leaf_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "leaf_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "leaf_rc_p0",
            )
            .map_err(llvm_err)?;
        let leaf_rc0 = self
            .builder
            .build_load(i64, leaf_rc_ptr0, "leaf_rc0")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_rc1 = self
            .builder
            .build_int_add(leaf_rc0, i64.const_int(1, false), "leaf_rc1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(leaf_rc_ptr0, leaf_rc1)
            .map_err(llvm_err)?;
        // Set RC of child[1] (new_leaf2) from 0 to 1 — internal node now references it
        let nl2_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(new_leaf2, i64, "nl2_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "nl2_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "nl2_rc_p",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nl2_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Return root with internal node, height=1, new total_len
        let new_total = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "new_total")
            .map_err(llvm_err)?;
        let undef2 = self.list_type.get_undef();
        let sr1 = self
            .builder
            .build_insert_value(undef2, internal, 0, "sr1")
            .map_err(llvm_err)?;
        let sr2 = self
            .builder
            .build_insert_value(sr1, new_total, 1, "sr2")
            .map_err(llvm_err)?;
        let sr3 = self
            .builder
            .build_insert_value(sr2, i64.const_int(1, false), 2, "sr3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sr3));

        // Leaf has room: store element and return
        self.builder.position_at_end(lp_h0_done);
        // Store elem at elements[count]
        // GEP: leaf + 8 (skip count+pad) = elements base, then index by count_load
        let leaf_b = self
            .builder
            .build_pointer_cast(leaf, ptr, "leaf_b")
            .map_err(llvm_err)?;
        let elem_base = unsafe {
            self.builder
                .build_gep(i8, leaf_b, &[i64.const_int(8, false)], "elem_base")
                .map_err(llvm_err)
        }?;
        let elem_gep = unsafe {
            self.builder
                .build_gep(self.string_type, elem_base, &[count_load], "elem_gep")
                .map_err(llvm_err)?
        };
        let _ = self.builder.build_store(elem_gep, elem).map_err(llvm_err)?;
        // Increment count
        let new_count = self
            .builder
            .build_int_add(count_load, i64.const_int(1, false), "new_count")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(leaf_i8, new_count)
            .map_err(llvm_err)?;
        // Return updated root (height=0, same leaf)
        let new_total_h0 = self
            .builder
            .build_int_add(total_len, i64.const_int(1, false), "nt_h0")
            .map_err(llvm_err)?;
        let undef_h0 = self.list_type.get_undef();
        let h0r1 = self
            .builder
            .build_insert_value(undef_h0, leaf, 0, "h0r1")
            .map_err(llvm_err)?;
        let h0r2 = self
            .builder
            .build_insert_value(h0r1, new_total_h0, 1, "h0r2")
            .map_err(llvm_err)?;
        let h0r3 = self
            .builder
            .build_insert_value(h0r2, zero, 2, "h0r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&h0r3));

        // === Height > 0: descend to rightmost internal node at h=1 ===
        self.builder.position_at_end(lp_hgt0);
        // Allocate variables for descent + parent tracking
        let lp_cur_node = self
            .builder
            .build_alloca(ptr, "lp_cur_node")
            .map_err(llvm_err)?;
        let lp_cur_h = self
            .builder
            .build_alloca(i64, "lp_cur_h")
            .map_err(llvm_err)?;
        let lp_parent_ptr = self
            .builder
            .build_alloca(ptr, "lp_parent_ptr")
            .map_err(llvm_err)?;
        let lp_parent_node = self
            .builder
            .build_alloca(ptr, "lp_parent_node")
            .map_err(llvm_err)?;
        let null_ptr = ptr.const_null();
        self.builder
            .build_store(lp_cur_node, node_ptr)
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_cur_h, height)
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_parent_ptr, null_ptr)
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_parent_node, null_ptr)
            .map_err(llvm_err)?;
        let lp_descend_loop = self
            .context
            .append_basic_block(list_push_fn, "descend_loop");
        let lp_descend_body = self
            .context
            .append_basic_block(list_push_fn, "descend_body");
        let lp_at_h1 = self.context.append_basic_block(list_push_fn, "at_h1");
        let _ = self.builder.build_unconditional_branch(lp_descend_loop);

        // descend_loop: iterate through internal nodes until we reach h=1
        self.builder.position_at_end(lp_descend_loop);
        let lp_ch = self
            .builder
            .build_load(i64, lp_cur_h, "ch")
            .map_err(llvm_err)?
            .into_int_value();
        let lp_ch_gt_1 = self
            .builder
            .build_int_compare(IntPredicate::SGT, lp_ch, i64.const_int(1, false), "ch_gt_1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lp_ch_gt_1, lp_descend_body, lp_at_h1);

        // descend_body: save parent info, move to rightmost child, decrease height
        self.builder.position_at_end(lp_descend_body);
        let lp_cn = self
            .builder
            .build_load(ptr, lp_cur_node, "cn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lp_cn_i8 = self
            .builder
            .build_pointer_cast(lp_cn, ptr, "cn_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_parent_node, lp_cn_i8)
            .map_err(llvm_err)?;
        let lp_dcnt_raw = self
            .builder
            .build_load(i32, lp_cn_i8, "dcnt_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lp_dcnt = self
            .builder
            .build_int_z_extend(lp_dcnt_raw, i64, "dcnt")
            .map_err(llvm_err)?;
        let lp_dlast = self
            .builder
            .build_int_sub(lp_dcnt, i64.const_int(1, false), "dlast")
            .map_err(llvm_err)?;
        let lp_dchildren = unsafe {
            self.builder
                .build_gep(i8, lp_cn_i8, &[i64.const_int(16, false)], "dchildren")
                .map_err(llvm_err)
        }?;
        let lp_dslot = unsafe {
            self.builder
                .build_gep(self.child_entry_type, lp_dchildren, &[lp_dlast], "dslot")
                .map_err(llvm_err)
        }?;
        let lp_st_slot = unsafe {
            self.builder
                .build_gep(i64, lp_dslot, &[i64.const_int(1, false)], "st_slot")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(lp_parent_ptr, lp_st_slot)
            .map_err(llvm_err)?;
        let lp_dchild = self
            .builder
            .build_load(ptr, lp_dslot, "dchild")
            .map_err(llvm_err)?
            .into_pointer_value();
        self.builder
            .build_store(lp_cur_node, lp_dchild)
            .map_err(llvm_err)?;
        let lp_ch_new = self
            .builder
            .build_int_sub(lp_ch, i64.const_int(1, false), "ch_new")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lp_cur_h, lp_ch_new)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_descend_loop);

        // At h=1: internal node whose children are leaves
        self.builder.position_at_end(lp_at_h1);
        let intl_base = self
            .builder
            .build_load(ptr, lp_cur_node, "intl_base")
            .map_err(llvm_err)?
            .into_pointer_value();
        let intl_base_i8 = self
            .builder
            .build_pointer_cast(intl_base, ptr, "intl_base_i8")
            .map_err(llvm_err)?;
        let intl_count_raw = self
            .builder
            .build_load(i32, intl_base_i8, "intl_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let intl_count = self
            .builder
            .build_int_z_extend(intl_count_raw, i64, "intl_count")
            .map_err(llvm_err)?;
        // Last child index = count - 1
        let last_idx = self
            .builder
            .build_int_sub(intl_count, i64.const_int(1, false), "last_idx")
            .map_err(llvm_err)?;
        // children array at offset 16, child entry = {ptr, i64} = 16 bytes
        let children_base = unsafe {
            self.builder
                .build_gep(
                    i8,
                    intl_base_i8,
                    &[i64.const_int(16, false)],
                    "intl_children",
                )
                .map_err(llvm_err)
        }?;
        let last_child_slot = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[last_idx],
                    "last_child_slot",
                )
                .map_err(llvm_err)
        }?;
        let last_child_ptr = self
            .builder
            .build_load(ptr, last_child_slot, "last_child")
            .map_err(llvm_err)?
            .into_pointer_value();
        let subtree_total_ptr = unsafe {
            self.builder
                .build_gep(i64, last_child_slot, &[i64.const_int(1, false)], "st_ptr")
                .map_err(llvm_err)
        }?;
        let subtree_total = self
            .builder
            .build_load(i64, subtree_total_ptr, "st")
            .map_err(llvm_err)?
            .into_int_value();
        // Check RC of leaf, copy if needed
        let leaf_int = self
            .builder
            .build_ptr_to_int(last_child_ptr, i64, "leaf_int")
            .map_err(llvm_err)?;
        let leaf_rc_addr = self
            .builder
            .build_int_sub(leaf_int, i64.const_int(8, false), "leaf_rc_addr")
            .map_err(llvm_err)?;
        let leaf_rc_ptr = self
            .builder
            .build_int_to_ptr(leaf_rc_addr, ptr, "leaf_rc_ptr")
            .map_err(llvm_err)?;
        let leaf_rc = self
            .builder
            .build_load(i64, leaf_rc_ptr, "leaf_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_shared = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                leaf_rc,
                i64.const_int(1, false),
                "leaf_shared",
            )
            .map_err(llvm_err)?;
        let lp_cow_leaf = self.context.append_basic_block(list_push_fn, "lp_cow_leaf");
        let lp_leaf_ready = self
            .context
            .append_basic_block(list_push_fn, "lp_leaf_ready");
        let _ = self
            .builder
            .build_conditional_branch(leaf_shared, lp_cow_leaf, lp_leaf_ready);
        self.builder.position_at_end(lp_cow_leaf);
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let copied_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "copied_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                self.module.get_function("memcpy").unwrap(),
                &[copied_leaf.into(), last_child_ptr.into(), leaf_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Update child pointer in internal node
        let _ = self
            .builder
            .build_store(last_child_slot, copied_leaf)
            .map_err(llvm_err)?;
        // Set RC of copied_leaf to 1 — internal node now references it
        let copied_rc_ptr = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(copied_leaf, i64, "cop_i")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "cop_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "cop_rc_p",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(copied_rc_ptr, i64.const_int(1, false))
            .map_err(llvm_err)?;
        // Decrement RC of old leaf — internal node no longer references it
        let old_rc_p = self
            .builder
            .build_int_to_ptr(leaf_rc_addr, ptr, "old_rc_p")
            .map_err(llvm_err)?;
        let old_rc = self
            .builder
            .build_load(i64, old_rc_p, "old_rc_v")
            .map_err(llvm_err)?
            .into_int_value();
        let new_old_rc = self
            .builder
            .build_int_sub(old_rc, i64.const_int(1, false), "new_old_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(old_rc_p, new_old_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_leaf_ready);
        self.builder.position_at_end(lp_leaf_ready);
        let phi_leaf = self.builder.build_phi(ptr, "phi_leaf").map_err(llvm_err)?;
        phi_leaf.add_incoming(&[(&last_child_ptr, lp_at_h1), (&copied_leaf, lp_cow_leaf)]);
        let target_leaf = phi_leaf.as_basic_value().into_pointer_value();
        // Read leaf count (i32)
        let leaf_bytes = self
            .builder
            .build_pointer_cast(target_leaf, ptr, "leaf_bytes")
            .map_err(llvm_err)?;
        let leaf_count_raw = self
            .builder
            .build_load(i32, leaf_bytes, "leaf_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_count = self
            .builder
            .build_int_z_extend(leaf_count_raw, i64, "leaf_count")
            .map_err(llvm_err)?;
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
