// ---- action_list_find_walk_rec(ptr node, i64 height, ptr fn, ptr found_p, ptr found_flag_p) -> void ----
        let fd_find_rec_fn = self.module.add_function(
            "action_list_find_walk_rec",
            void.fn_type(
                &[ptr.into(), i64.into(), ptr.into(), ptr.into(), ptr.into()],
                false,
            ),
            None,
        );
        let fdr_entry = self.context.append_basic_block(fd_find_rec_fn, "entry");
        let fdr_leaf_hdr = self.context.append_basic_block(fd_find_rec_fn, "leaf_hdr");
        let fdr_leaf_bdy = self.context.append_basic_block(fd_find_rec_fn, "leaf_bdy");
        let fdr_leaf_chk = self.context.append_basic_block(fd_find_rec_fn, "leaf_chk");
        let fdr_leaf_found = self
            .context
            .append_basic_block(fd_find_rec_fn, "leaf_found");
        let fdr_leaf_next = self.context.append_basic_block(fd_find_rec_fn, "leaf_next");
        let fdr_leaf_done = self.context.append_basic_block(fd_find_rec_fn, "leaf_done");
        let fdr_int_hdr = self.context.append_basic_block(fd_find_rec_fn, "int_hdr");
        let fdr_int_bdy = self.context.append_basic_block(fd_find_rec_fn, "int_bdy");
        let fdr_int_child = self.context.append_basic_block(fd_find_rec_fn, "int_child");
        let fdr_int_child_body = self
            .context
            .append_basic_block(fd_find_rec_fn, "int_child_body");
        let fdr_int_next = self.context.append_basic_block(fd_find_rec_fn, "int_next");
        let fdr_concat = self.context.append_basic_block(fd_find_rec_fn, "concat");
        let fdr_concat_right = self
            .context
            .append_basic_block(fd_find_rec_fn, "concat_right");
        let fdr_normal = self.context.append_basic_block(fd_find_rec_fn, "normal");
        self.builder.position_at_end(fdr_entry);
        let fdr_node = fd_find_rec_fn
            .get_first_param()
            .unwrap()
            .into_pointer_value();
        let fdr_height = fd_find_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let fdr_fn = fd_find_rec_fn
            .get_nth_param(2)
            .unwrap()
            .into_pointer_value();
        let fdr_found_p = fd_find_rec_fn
            .get_nth_param(3)
            .unwrap()
            .into_pointer_value();
        let fdr_found_flag_p = fd_find_rec_fn
            .get_nth_param(4)
            .unwrap()
            .into_pointer_value();
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
                fd_find_rec_fn,
                &[
                    fdr_left_node.into(),
                    fdr_left_h.into(),
                    fdr_fn.into(),
                    fdr_found_p.into(),
                    fdr_found_flag_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let fdr_concat_lflag = self
            .builder
            .build_load(i64, fdr_found_flag_p, "fdr_concat_lflag")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_concat_lfound = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                fdr_concat_lflag,
                zero,
                "fdr_concat_lfound",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            fdr_concat_lfound,
            fdr_leaf_done,
            fdr_concat_right,
        );
        self.builder.position_at_end(fdr_concat_right);
        let _ = self
            .builder
            .build_call(
                fd_find_rec_fn,
                &[
                    fdr_right_node.into(),
                    fdr_right_h.into(),
                    fdr_fn.into(),
                    fdr_found_p.into(),
                    fdr_found_flag_p.into(),
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
        let fdr_elem = self
            .builder
            .build_load(self.string_type, fdr_ep, "fdr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let fdr_elem_tag = self
            .builder
            .build_extract_value(fdr_elem, 0, "fdr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_pred = self
            .builder
            .build_indirect_call(lambda_fn_ty, fdr_fn, &[fdr_elem_tag.into()], "fdr_pred")
            .map_err(llvm_err)?;
        let fdr_pred_bv = fdr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("filter_walk indirect call failed")?;
        let fdr_pred_val = if fdr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(fdr_pred_bv.into_struct_value(), 0, "fdr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            fdr_pred_bv.into_int_value()
        };
        let fdr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, fdr_pred_val, zero, "fdr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_is_true, fdr_leaf_found, fdr_leaf_next);
        self.builder.position_at_end(fdr_leaf_found);
        let _ = self
            .builder
            .build_store(fdr_found_p, fdr_elem)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fdr_found_flag_p, i64.const_int(1, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

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
        let fdr_int_flag = self
            .builder
            .build_load(i64, fdr_found_flag_p, "fdr_int_flag")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_int_already = self
            .builder
            .build_int_compare(IntPredicate::NE, fdr_int_flag, zero, "fdr_int_already")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            fdr_int_already,
            fdr_leaf_done,
            fdr_int_child_body,
        );
        self.builder.position_at_end(fdr_int_child_body);
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
                fd_find_rec_fn,
                &[
                    fdr_child_ptr.into(),
                    fdr_child_h.into(),
                    fdr_fn.into(),
                    fdr_found_p.into(),
                    fdr_found_flag_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let fdr_child_flag = self
            .builder
            .build_load(i64, fdr_found_flag_p, "fdr_child_flag")
            .map_err(llvm_err)?
            .into_int_value();
        let fdr_child_found = self
            .builder
            .build_int_compare(IntPredicate::NE, fdr_child_flag, zero, "fdr_child_found")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fdr_child_found, fdr_leaf_done, fdr_int_next);
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

        // ---- action_list_find_walk({ptr,i64,i64} list, ptr fn) -> {i64,ptr} fat (tag=1 null) ----
        let fd_fn = self.module.add_function(
            "action_list_find_walk",
            self.string_type
                .fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let fd_entry = self.context.append_basic_block(fd_fn, "entry");
        let fd_walk = self.context.append_basic_block(fd_fn, "walk");
        let fd_done = self.context.append_basic_block(fd_fn, "done");
        self.builder.position_at_end(fd_entry);
        let fd_list = fd_fn.get_first_param().unwrap().into_struct_value();
        let fd_fn_ptr = fd_fn.get_nth_param(1).unwrap().into_pointer_value();
        let fd_found_p = self
            .builder
            .build_alloca(self.string_type, "fd_found")
            .map_err(llvm_err)?;
        let fd_flag_p = self
            .builder
            .build_alloca(i64, "fd_flag")
            .map_err(llvm_err)?;
        self.builder
            .build_store(fd_flag_p, zero)
            .map_err(llvm_err)?;
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
        let fd_rec = self
            .module
            .get_function("action_list_find_walk_rec")
            .unwrap();
        let _ = self.builder.build_unconditional_branch(fd_walk);
        self.builder.position_at_end(fd_walk);
        let _ = self
            .builder
            .build_call(
                fd_rec,
                &[
                    fd_node.into(),
                    fd_height.into(),
                    fd_fn_ptr.into(),
                    fd_found_p.into(),
                    fd_flag_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fd_done);
        self.builder.position_at_end(fd_done);
        let fd_flag = self
            .builder
            .build_load(i64, fd_flag_p, "fd_f")
            .map_err(llvm_err)?
            .into_int_value();
        let fd_found = self
            .builder
            .build_load(self.string_type, fd_found_p, "fd_val")
            .map_err(llvm_err)?;
        let fd_is_found = self
            .builder
            .build_int_compare(IntPredicate::NE, fd_flag, zero, "fd_ok")
            .map_err(llvm_err)?;
        let null_u = self.string_type.get_undef();
        let null1 = self
            .builder
            .build_insert_value(null_u, i64.const_int(1, false), 0, "n1")
            .map_err(llvm_err)?;
        let null_fat = self
            .builder
            .build_insert_value(null1, ptr.const_zero(), 1, "n2")
            .map_err(llvm_err)?
            .as_basic_value_enum();
        let fd_ret = self
            .builder
            .build_select(
                fd_is_found,
                fd_found,
                null_fat.as_basic_value_enum(),
                "fd_ret",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fd_ret));
