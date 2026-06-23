// ---- action_list_fold_walk_rec / action_list_fold_walk ----
        // Int accumulator fast path: (i64, i64) -> i64 direct call, no fat-struct load/return.
        let fold_fn_ty = i64.fn_type(&[i64.into(), i64.into()], false);
        let fd_rec_fn = self.module.add_function(
            "action_list_fold_walk_rec",
            void.fn_type(&[ptr.into(), i64.into(), ptr.into(), ptr.into()], false),
            None,
        );
        let fdr_entry = self.context.append_basic_block(fd_rec_fn, "entry");
        let fdr_leaf_hdr = self.context.append_basic_block(fd_rec_fn, "leaf_hdr");
        let fdr_leaf_bdy = self.context.append_basic_block(fd_rec_fn, "leaf_bdy");
        let fdr_leaf_chk = self.context.append_basic_block(fd_rec_fn, "leaf_chk");
        let fdr_leaf_next = self.context.append_basic_block(fd_rec_fn, "leaf_next");
        let fdr_leaf_done = self.context.append_basic_block(fd_rec_fn, "leaf_done");
        let fdr_int_hdr = self.context.append_basic_block(fd_rec_fn, "int_hdr");
        let fdr_int_bdy = self.context.append_basic_block(fd_rec_fn, "int_bdy");
        let fdr_int_child = self.context.append_basic_block(fd_rec_fn, "int_child");
        let fdr_int_next = self.context.append_basic_block(fd_rec_fn, "int_next");
        let fdr_concat = self.context.append_basic_block(fd_rec_fn, "concat");
        let fdr_normal = self.context.append_basic_block(fd_rec_fn, "normal");
        self.builder.position_at_end(fdr_entry);
        let fdr_node = fd_rec_fn.get_first_param().unwrap().into_pointer_value();
        let fdr_height = fd_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let fdr_fn = fd_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let fdr_acc = fd_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let fdr_neg1 = i64.const_int(-1i64 as u64, true);
        let fdr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, fdr_height, fdr_neg1, "fdr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_is_concat, fdr_concat, fdr_normal);
        self.builder.position_at_end(fdr_concat);
        let fdr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, fdr_node, &[i64.const_int(2, false)], "fdr_ln_p")
                .map_err(llvm_err)
        }?;
        let fdr_left_node = self
            .builder
            .build_load(ptr, fdr_ln_p, "fdr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fdr_lh_p = unsafe {
            self.builder
                .build_gep(i64, fdr_node, &[i64.const_int(4, false)], "fdr_lh_p")
                .map_err(llvm_err)
        }?;
        let fdr_left_h = self
            .builder
            .build_load(i64, fdr_lh_p, "fdr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, fdr_node, &[i64.const_int(5, false)], "fdr_rn_p")
                .map_err(llvm_err)
        }?;
        let fdr_right_node = self
            .builder
            .build_load(ptr, fdr_rn_p, "fdr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fdr_rh_p = unsafe {
            self.builder
                .build_gep(i64, fdr_node, &[i64.const_int(7, false)], "fdr_rh_p")
                .map_err(llvm_err)
        }?;
        let fdr_right_h = self
            .builder
            .build_load(i64, fdr_rh_p, "fdr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                fd_rec_fn,
                &[
                    fdr_left_node.into(),
                    fdr_left_h.into(),
                    fdr_fn.into(),
                    fdr_acc.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                fd_rec_fn,
                &[
                    fdr_right_node.into(),
                    fdr_right_h.into(),
                    fdr_fn.into(),
                    fdr_acc.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(fdr_normal);
        let fdr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, fdr_height, zero, "fdr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_is_leaf, fdr_leaf_hdr, fdr_int_hdr);
        self.builder.position_at_end(fdr_leaf_hdr);
        let fdr_leaf_i8 = self
            .builder
            .build_pointer_cast(fdr_node, ptr, "fdr_leaf_i8")
            .map_err(llvm_err)?;
        let fdr_count_raw = self
            .builder
            .build_load(i32, fdr_leaf_i8, "fdr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_count = self
            .builder
            .build_int_z_extend(fdr_count_raw, i64, "fdr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fdr_leaf_bdy);
        self.builder.position_at_end(fdr_leaf_bdy);
        let fdr_i = self.builder.build_phi(i64, "fdr_i").map_err(llvm_err)?;
        let fdr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fdr_i.as_basic_value().into_int_value(),
                fdr_count,
                "fdr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_done_leaf, fdr_leaf_done, fdr_leaf_chk);
        self.builder.position_at_end(fdr_leaf_chk);
        let fdr_eb = unsafe {
            self.builder
                .build_gep(i8, fdr_leaf_i8, &[i64.const_int(8, false)], "fdr_eb")
                .map_err(llvm_err)?
        };
        let fdr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    fdr_eb,
                    &[fdr_i.as_basic_value().into_int_value()],
                    "fdr_ep",
                )
                .map_err(llvm_err)?
        };
        let fdr_elem_tag = self
            .builder
            .build_load(i64, fdr_ep, "fdr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_cur_acc = self
            .builder
            .build_load(i64, fdr_acc, "fdr_acc")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_new_acc = self
            .builder
            .build_indirect_call(
                fold_fn_ty,
                fdr_fn,
                &[fdr_cur_acc.into(), fdr_elem_tag.into()],
                "fdr_folded",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("fold_walk indirect call failed")?
            .into_int_value();
        self.builder
            .build_store(fdr_acc, fdr_new_acc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fdr_leaf_next);
        self.builder.position_at_end(fdr_leaf_next);
        let fdr_next_i = self
            .builder
            .build_int_add(
                fdr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fdr_ni",
            )
            .map_err(llvm_err)?;
        let fdr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        fdr_i.add_incoming(&[(&zero, fdr_leaf_hdr), (&fdr_next_i, fdr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fdr_leaf_bdy);
        self.builder.position_at_end(fdr_leaf_done);
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(fdr_int_hdr);
        let fdr_int_i8 = self
            .builder
            .build_pointer_cast(fdr_node, ptr, "fdr_int_i8")
            .map_err(llvm_err)?;
        let fdr_child_count_raw = self
            .builder
            .build_load(i32, fdr_int_i8, "fdr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_child_count = self
            .builder
            .build_int_z_extend(fdr_child_count_raw, i64, "fdr_cc")
            .map_err(llvm_err)?;
        let fdr_child_h = self
            .builder
            .build_int_sub(fdr_height, i64.const_int(1, false), "fdr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fdr_int_bdy);
        self.builder.position_at_end(fdr_int_bdy);
        let fdr_ci = self.builder.build_phi(i64, "fdr_ci").map_err(llvm_err)?;
        let fdr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                fdr_ci.as_basic_value().into_int_value(),
                fdr_child_count,
                "fdr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_done_int, fdr_leaf_done, fdr_int_child);
        self.builder.position_at_end(fdr_int_child);
        let fdr_children_base = unsafe {
            self.builder
                .build_gep(i8, fdr_int_i8, &[i64.const_int(16, false)], "fdr_cb")
                .map_err(llvm_err)?
        };
        let fdr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    fdr_children_base,
                    &[fdr_ci.as_basic_value().into_int_value()],
                    "fdr_cep",
                )
                .map_err(llvm_err)?
        };
        let fdr_child_entry = self
            .builder
            .build_load(self.child_entry_type, fdr_child_ep, "fdr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let fdr_child_ptr = self
            .builder
            .build_extract_value(fdr_child_entry, 0, "fdr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                fd_rec_fn,
                &[
                    fdr_child_ptr.into(),
                    fdr_child_h.into(),
                    fdr_fn.into(),
                    fdr_acc.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fdr_int_next);
        self.builder.position_at_end(fdr_int_next);
        let fdr_next_ci = self
            .builder
            .build_int_add(
                fdr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "fdr_nci",
            )
            .map_err(llvm_err)?;
        let fdr_int_next_bb = self.builder.get_insert_block().unwrap();
        fdr_ci.add_incoming(&[(&zero, fdr_int_hdr), (&fdr_next_ci, fdr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(fdr_int_bdy);

        let fd_fn = self.module.add_function(
            "action_list_fold_walk",
            i64.fn_type(&[self.list_type.into(), ptr.into(), i64.into()], false),
            None,
        );
        let fd_entry = self.context.append_basic_block(fd_fn, "entry");
        let fd_walk = self.context.append_basic_block(fd_fn, "walk");
        self.builder.position_at_end(fd_entry);
        let fd_list = fd_fn.get_first_param().unwrap().into_struct_value();
        let fd_fn_ptr = fd_fn.get_nth_param(1).unwrap().into_pointer_value();
        let fd_init = fd_fn.get_nth_param(2).unwrap().into_int_value();
        let fd_node = self
            .builder
            .build_extract_value(fd_list, 0, "fd_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fd_height = self
            .builder
            .build_extract_value(fd_list, 2, "fd_height")
            .map_err(llvm_err)?
            .into_int_value();
        let fd_acc = self.builder.build_alloca(i64, "fd_acc").map_err(llvm_err)?;
        self.builder
            .build_store(fd_acc, fd_init)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fd_walk);
        self.builder.position_at_end(fd_walk);
        let _ = self
            .builder
            .build_call(
                fd_rec_fn,
                &[
                    fd_node.into(),
                    fd_height.into(),
                    fd_fn_ptr.into(),
                    fd_acc.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let fd_res = self
            .builder
            .build_load(i64, fd_acc, "fd_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fd_res));
