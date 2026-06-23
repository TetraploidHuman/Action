// ---- action_list_any_walk_rec / action_list_any_walk ----
        let ay_rec_fn = self.module.add_function(
            "action_list_any_walk_rec",
            b1.fn_type(&[ptr.into(), i64.into(), ptr.into()], false),
            None,
        );
        let ayr_entry = self.context.append_basic_block(ay_rec_fn, "entry");
        let ayr_true = self.context.append_basic_block(ay_rec_fn, "any_true");
        let ayr_false = self.context.append_basic_block(ay_rec_fn, "any_false");
        let ayr_leaf_hdr = self.context.append_basic_block(ay_rec_fn, "leaf_hdr");
        let ayr_leaf_bdy = self.context.append_basic_block(ay_rec_fn, "leaf_bdy");
        let ayr_leaf_chk = self.context.append_basic_block(ay_rec_fn, "leaf_chk");
        let ayr_leaf_next = self.context.append_basic_block(ay_rec_fn, "leaf_next");
        let ayr_int_hdr = self.context.append_basic_block(ay_rec_fn, "int_hdr");
        let ayr_int_bdy = self.context.append_basic_block(ay_rec_fn, "int_bdy");
        let ayr_int_child = self.context.append_basic_block(ay_rec_fn, "int_child");
        let ayr_int_next = self.context.append_basic_block(ay_rec_fn, "int_next");
        let ayr_concat = self.context.append_basic_block(ay_rec_fn, "concat");
        let ayr_concat_right = self.context.append_basic_block(ay_rec_fn, "concat_right");
        let ayr_normal = self.context.append_basic_block(ay_rec_fn, "normal");
        self.builder.position_at_end(ayr_entry);
        let ayr_node = ay_rec_fn.get_first_param().unwrap().into_pointer_value();
        let ayr_height = ay_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let ayr_fn = ay_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let ayr_neg1 = i64.const_int(-1i64 as u64, true);
        let ayr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, ayr_height, ayr_neg1, "ayr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_is_concat, ayr_concat, ayr_normal);
        self.builder.position_at_end(ayr_concat);
        let ayr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, ayr_node, &[i64.const_int(2, false)], "ayr_ln_p")
                .map_err(llvm_err)
        }?;
        let ayr_left_node = self
            .builder
            .build_load(ptr, ayr_ln_p, "ayr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ayr_lh_p = unsafe {
            self.builder
                .build_gep(i64, ayr_node, &[i64.const_int(4, false)], "ayr_lh_p")
                .map_err(llvm_err)
        }?;
        let ayr_left_h = self
            .builder
            .build_load(i64, ayr_lh_p, "ayr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, ayr_node, &[i64.const_int(5, false)], "ayr_rn_p")
                .map_err(llvm_err)
        }?;
        let ayr_right_node = self
            .builder
            .build_load(ptr, ayr_rn_p, "ayr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ayr_rh_p = unsafe {
            self.builder
                .build_gep(i64, ayr_node, &[i64.const_int(7, false)], "ayr_rh_p")
                .map_err(llvm_err)
        }?;
        let ayr_right_h = self
            .builder
            .build_load(i64, ayr_rh_p, "ayr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_lhit = self
            .builder
            .build_call(
                ay_rec_fn,
                &[ayr_left_node.into(), ayr_left_h.into(), ayr_fn.into()],
                "ayr_lhit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(ayr_lhit, ayr_true, ayr_concat_right);
        self.builder.position_at_end(ayr_concat_right);
        let ayr_rhit = self
            .builder
            .build_call(
                ay_rec_fn,
                &[ayr_right_node.into(), ayr_right_h.into(), ayr_fn.into()],
                "ayr_rhit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&ayr_rhit));
        self.builder.position_at_end(ayr_normal);
        let ayr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, ayr_height, zero, "ayr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_is_leaf, ayr_leaf_hdr, ayr_int_hdr);
        self.builder.position_at_end(ayr_leaf_hdr);
        let ayr_leaf_i8 = self
            .builder
            .build_pointer_cast(ayr_node, ptr, "ayr_leaf_i8")
            .map_err(llvm_err)?;
        let ayr_count_raw = self
            .builder
            .build_load(i32, ayr_leaf_i8, "ayr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_count = self
            .builder
            .build_int_z_extend(ayr_count_raw, i64, "ayr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ayr_leaf_bdy);
        self.builder.position_at_end(ayr_leaf_bdy);
        let ayr_i = self.builder.build_phi(i64, "ayr_i").map_err(llvm_err)?;
        let ayr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                ayr_i.as_basic_value().into_int_value(),
                ayr_count,
                "ayr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_done_leaf, ayr_false, ayr_leaf_chk);
        self.builder.position_at_end(ayr_leaf_chk);
        let ayr_eb = unsafe {
            self.builder
                .build_gep(i8, ayr_leaf_i8, &[i64.const_int(8, false)], "ayr_eb")
                .map_err(llvm_err)?
        };
        let ayr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    ayr_eb,
                    &[ayr_i.as_basic_value().into_int_value()],
                    "ayr_ep",
                )
                .map_err(llvm_err)?
        };
        let ayr_elem = self
            .builder
            .build_load(self.string_type, ayr_ep, "ayr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let ayr_elem_tag = self
            .builder
            .build_extract_value(ayr_elem, 0, "ayr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_pred = self
            .builder
            .build_indirect_call(lambda_fn_ty, ayr_fn, &[ayr_elem_tag.into()], "ayr_pred")
            .map_err(llvm_err)?;
        let ayr_pred_bv = ayr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("any_walk indirect call failed")?;
        let ayr_pred_val = if ayr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(ayr_pred_bv.into_struct_value(), 0, "ayr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            ayr_pred_bv.into_int_value()
        };
        let ayr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, ayr_pred_val, zero, "ayr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_is_true, ayr_true, ayr_leaf_next);
        self.builder.position_at_end(ayr_leaf_next);
        let ayr_next_i = self
            .builder
            .build_int_add(
                ayr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ayr_ni",
            )
            .map_err(llvm_err)?;
        let ayr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        ayr_i.add_incoming(&[(&zero, ayr_leaf_hdr), (&ayr_next_i, ayr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(ayr_leaf_bdy);
        self.builder.position_at_end(ayr_int_hdr);
        let ayr_int_i8 = self
            .builder
            .build_pointer_cast(ayr_node, ptr, "ayr_int_i8")
            .map_err(llvm_err)?;
        let ayr_child_count_raw = self
            .builder
            .build_load(i32, ayr_int_i8, "ayr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let ayr_child_count = self
            .builder
            .build_int_z_extend(ayr_child_count_raw, i64, "ayr_cc")
            .map_err(llvm_err)?;
        let ayr_child_h = self
            .builder
            .build_int_sub(ayr_height, i64.const_int(1, false), "ayr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ayr_int_bdy);
        self.builder.position_at_end(ayr_int_bdy);
        let ayr_ci = self.builder.build_phi(i64, "ayr_ci").map_err(llvm_err)?;
        let ayr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                ayr_ci.as_basic_value().into_int_value(),
                ayr_child_count,
                "ayr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ayr_done_int, ayr_false, ayr_int_child);
        self.builder.position_at_end(ayr_int_child);
        let ayr_children_base = unsafe {
            self.builder
                .build_gep(i8, ayr_int_i8, &[i64.const_int(16, false)], "ayr_cb")
                .map_err(llvm_err)?
        };
        let ayr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    ayr_children_base,
                    &[ayr_ci.as_basic_value().into_int_value()],
                    "ayr_cep",
                )
                .map_err(llvm_err)?
        };
        let ayr_child_entry = self
            .builder
            .build_load(self.child_entry_type, ayr_child_ep, "ayr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let ayr_child_ptr = self
            .builder
            .build_extract_value(ayr_child_entry, 0, "ayr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ayr_child_hit = self
            .builder
            .build_call(
                ay_rec_fn,
                &[ayr_child_ptr.into(), ayr_child_h.into(), ayr_fn.into()],
                "ayr_hit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(ayr_child_hit, ayr_true, ayr_int_next);
        self.builder.position_at_end(ayr_int_next);
        let ayr_next_ci = self
            .builder
            .build_int_add(
                ayr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ayr_nci",
            )
            .map_err(llvm_err)?;
        let ayr_int_next_bb = self.builder.get_insert_block().unwrap();
        ayr_ci.add_incoming(&[(&zero, ayr_int_hdr), (&ayr_next_ci, ayr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(ayr_int_bdy);
        self.builder.position_at_end(ayr_true);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(ayr_false);
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        let ay_fn = self.module.add_function(
            "action_list_any_walk",
            b1.fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let ay_entry = self.context.append_basic_block(ay_fn, "entry");
        let ay_walk = self.context.append_basic_block(ay_fn, "walk");
        self.builder.position_at_end(ay_entry);
        let ay_list = ay_fn.get_first_param().unwrap().into_struct_value();
        let ay_fn_ptr = ay_fn.get_nth_param(1).unwrap().into_pointer_value();
        let ay_node = self
            .builder
            .build_extract_value(ay_list, 0, "ay_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ay_height = self
            .builder
            .build_extract_value(ay_list, 2, "ay_height")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(ay_walk);
        self.builder.position_at_end(ay_walk);
        let ay_hit = self
            .builder
            .build_call(
                ay_rec_fn,
                &[ay_node.into(), ay_height.into(), ay_fn_ptr.into()],
                "ay_hit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&ay_hit));

        // ---- action_list_all_walk_rec / action_list_all_walk ----
        let al_rec_fn = self.module.add_function(
            "action_list_all_walk_rec",
            b1.fn_type(&[ptr.into(), i64.into(), ptr.into()], false),
            None,
        );
        let alr_entry = self.context.append_basic_block(al_rec_fn, "entry");
        let alr_true = self.context.append_basic_block(al_rec_fn, "all_true");
        let alr_false = self.context.append_basic_block(al_rec_fn, "all_false");
        let alr_leaf_hdr = self.context.append_basic_block(al_rec_fn, "leaf_hdr");
        let alr_leaf_bdy = self.context.append_basic_block(al_rec_fn, "leaf_bdy");
        let alr_leaf_chk = self.context.append_basic_block(al_rec_fn, "leaf_chk");
        let alr_leaf_next = self.context.append_basic_block(al_rec_fn, "leaf_next");
        let alr_int_hdr = self.context.append_basic_block(al_rec_fn, "int_hdr");
        let alr_int_bdy = self.context.append_basic_block(al_rec_fn, "int_bdy");
        let alr_int_child = self.context.append_basic_block(al_rec_fn, "int_child");
        let alr_int_next = self.context.append_basic_block(al_rec_fn, "int_next");
        let alr_concat = self.context.append_basic_block(al_rec_fn, "concat");
        let alr_concat_right = self.context.append_basic_block(al_rec_fn, "concat_right");
        let alr_normal = self.context.append_basic_block(al_rec_fn, "normal");
        self.builder.position_at_end(alr_entry);
        let alr_node = al_rec_fn.get_first_param().unwrap().into_pointer_value();
        let alr_height = al_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let alr_fn = al_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let alr_neg1 = i64.const_int(-1i64 as u64, true);
        let alr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, alr_height, alr_neg1, "alr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_is_concat, alr_concat, alr_normal);
        self.builder.position_at_end(alr_concat);
        let alr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, alr_node, &[i64.const_int(2, false)], "alr_ln_p")
                .map_err(llvm_err)
        }?;
        let alr_left_node = self
            .builder
            .build_load(ptr, alr_ln_p, "alr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alr_lh_p = unsafe {
            self.builder
                .build_gep(i64, alr_node, &[i64.const_int(4, false)], "alr_lh_p")
                .map_err(llvm_err)
        }?;
        let alr_left_h = self
            .builder
            .build_load(i64, alr_lh_p, "alr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, alr_node, &[i64.const_int(5, false)], "alr_rn_p")
                .map_err(llvm_err)
        }?;
        let alr_right_node = self
            .builder
            .build_load(ptr, alr_rn_p, "alr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alr_rh_p = unsafe {
            self.builder
                .build_gep(i64, alr_node, &[i64.const_int(7, false)], "alr_rh_p")
                .map_err(llvm_err)
        }?;
        let alr_right_h = self
            .builder
            .build_load(i64, alr_rh_p, "alr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_lok = self
            .builder
            .build_call(
                al_rec_fn,
                &[alr_left_node.into(), alr_left_h.into(), alr_fn.into()],
                "alr_lok",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(alr_lok, alr_concat_right, alr_false);
        self.builder.position_at_end(alr_concat_right);
        let alr_rok = self
            .builder
            .build_call(
                al_rec_fn,
                &[alr_right_node.into(), alr_right_h.into(), alr_fn.into()],
                "alr_rok",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&alr_rok));
        self.builder.position_at_end(alr_normal);
        let alr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, alr_height, zero, "alr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_is_leaf, alr_leaf_hdr, alr_int_hdr);
        self.builder.position_at_end(alr_leaf_hdr);
        let alr_leaf_i8 = self
            .builder
            .build_pointer_cast(alr_node, ptr, "alr_leaf_i8")
            .map_err(llvm_err)?;
        let alr_count_raw = self
            .builder
            .build_load(i32, alr_leaf_i8, "alr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_count = self
            .builder
            .build_int_z_extend(alr_count_raw, i64, "alr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(alr_leaf_bdy);
        self.builder.position_at_end(alr_leaf_bdy);
        let alr_i = self.builder.build_phi(i64, "alr_i").map_err(llvm_err)?;
        let alr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                alr_i.as_basic_value().into_int_value(),
                alr_count,
                "alr_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_done_leaf, alr_true, alr_leaf_chk);
        self.builder.position_at_end(alr_leaf_chk);
        let alr_eb = unsafe {
            self.builder
                .build_gep(i8, alr_leaf_i8, &[i64.const_int(8, false)], "alr_eb")
                .map_err(llvm_err)?
        };
        let alr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    alr_eb,
                    &[alr_i.as_basic_value().into_int_value()],
                    "alr_ep",
                )
                .map_err(llvm_err)?
        };
        let alr_elem = self
            .builder
            .build_load(self.string_type, alr_ep, "alr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let alr_elem_tag = self
            .builder
            .build_extract_value(alr_elem, 0, "alr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_pred = self
            .builder
            .build_indirect_call(lambda_fn_ty, alr_fn, &[alr_elem_tag.into()], "alr_pred")
            .map_err(llvm_err)?;
        let alr_pred_bv = alr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("all_walk indirect call failed")?;
        let alr_pred_val = if alr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(alr_pred_bv.into_struct_value(), 0, "alr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            alr_pred_bv.into_int_value()
        };
        let alr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, alr_pred_val, zero, "alr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_is_true, alr_leaf_next, alr_false);
        self.builder.position_at_end(alr_leaf_next);
        let alr_next_i = self
            .builder
            .build_int_add(
                alr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "alr_ni",
            )
            .map_err(llvm_err)?;
        let alr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        alr_i.add_incoming(&[(&zero, alr_leaf_hdr), (&alr_next_i, alr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(alr_leaf_bdy);
        self.builder.position_at_end(alr_int_hdr);
        let alr_int_i8 = self
            .builder
            .build_pointer_cast(alr_node, ptr, "alr_int_i8")
            .map_err(llvm_err)?;
        let alr_child_count_raw = self
            .builder
            .build_load(i32, alr_int_i8, "alr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let alr_child_count = self
            .builder
            .build_int_z_extend(alr_child_count_raw, i64, "alr_cc")
            .map_err(llvm_err)?;
        let alr_child_h = self
            .builder
            .build_int_sub(alr_height, i64.const_int(1, false), "alr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(alr_int_bdy);
        self.builder.position_at_end(alr_int_bdy);
        let alr_ci = self.builder.build_phi(i64, "alr_ci").map_err(llvm_err)?;
        let alr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                alr_ci.as_basic_value().into_int_value(),
                alr_child_count,
                "alr_done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(alr_done_int, alr_true, alr_int_child);
        self.builder.position_at_end(alr_int_child);
        let alr_children_base = unsafe {
            self.builder
                .build_gep(i8, alr_int_i8, &[i64.const_int(16, false)], "alr_cb")
                .map_err(llvm_err)?
        };
        let alr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    alr_children_base,
                    &[alr_ci.as_basic_value().into_int_value()],
                    "alr_cep",
                )
                .map_err(llvm_err)?
        };
        let alr_child_entry = self
            .builder
            .build_load(self.child_entry_type, alr_child_ep, "alr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let alr_child_ptr = self
            .builder
            .build_extract_value(alr_child_entry, 0, "alr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alr_child_ok = self
            .builder
            .build_call(
                al_rec_fn,
                &[alr_child_ptr.into(), alr_child_h.into(), alr_fn.into()],
                "alr_ok",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(alr_child_ok, alr_int_next, alr_false);
        self.builder.position_at_end(alr_int_next);
        let alr_next_ci = self
            .builder
            .build_int_add(
                alr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "alr_nci",
            )
            .map_err(llvm_err)?;
        let alr_int_next_bb = self.builder.get_insert_block().unwrap();
        alr_ci.add_incoming(&[(&zero, alr_int_hdr), (&alr_next_ci, alr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(alr_int_bdy);
        self.builder.position_at_end(alr_true);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(alr_false);
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        let al_fn = self.module.add_function(
            "action_list_all_walk",
            b1.fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let al_entry = self.context.append_basic_block(al_fn, "entry");
        let al_walk = self.context.append_basic_block(al_fn, "walk");
        self.builder.position_at_end(al_entry);
        let al_list = al_fn.get_first_param().unwrap().into_struct_value();
        let al_fn_ptr = al_fn.get_nth_param(1).unwrap().into_pointer_value();
        let al_node = self
            .builder
            .build_extract_value(al_list, 0, "al_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let al_height = self
            .builder
            .build_extract_value(al_list, 2, "al_height")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(al_walk);
        self.builder.position_at_end(al_walk);
        let al_ok = self
            .builder
            .build_call(
                al_rec_fn,
                &[al_node.into(), al_height.into(), al_fn_ptr.into()],
                "al_ok",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&al_ok));
