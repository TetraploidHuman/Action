// ---- action_list_flatten({ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Converts a ConcatNode DAG into a flat B-tree list.
        // Recursively flattens nested ConcatNode children before merging materialized subtrees.
        let fl_fn = self.module.get_function("action_list_flatten").unwrap();
        let fl_entry = self.context.append_basic_block(fl_fn, "entry");
        let fl_not_concat = self.context.append_basic_block(fl_fn, "not_concat");
        let fl_concat = self.context.append_basic_block(fl_fn, "concat");
        self.builder.position_at_end(fl_entry);
        let fl_input = fl_fn.get_first_param().unwrap().into_struct_value();
        let fl_height = self
            .builder
            .build_extract_value(fl_input, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                fl_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fl_is_concat, fl_concat, fl_not_concat);
        // Not concat: return input unchanged
        self.builder.position_at_end(fl_not_concat);
        let _ = self.builder.build_return(Some(&fl_input));
        // Concat: recursively flatten nested ConcatNode children, then merge flat subtrees
        self.builder.position_at_end(fl_concat);
        let fl_node = self
            .builder
            .build_extract_value(fl_input, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_node_i8 = self
            .builder
            .build_pointer_cast(fl_node, ptr, "node_i8")
            .map_err(llvm_err)?;
        let fl_left_ptr = unsafe {
            self.builder
                .build_gep(i8, fl_node_i8, &[i64.const_int(16, false)], "left_ptr")
                .map_err(llvm_err)
        }?;
        let fl_left = self
            .builder
            .build_load(self.list_type, fl_left_ptr, "left")
            .map_err(llvm_err)?
            .into_struct_value();
        let fl_left_h = self
            .builder
            .build_extract_value(fl_left, 2, "lh")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_right_ptr = unsafe {
            self.builder
                .build_gep(i8, fl_node_i8, &[i64.const_int(40, false)], "right_ptr")
                .map_err(llvm_err)
        }?;
        let fl_right = self
            .builder
            .build_load(self.list_type, fl_right_ptr, "right")
            .map_err(llvm_err)?
            .into_struct_value();
        let fl_right_h = self
            .builder
            .build_extract_value(fl_right, 2, "rh")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_neg1 = i64.const_int(-1i64 as u64, true);

        let fl_l_is_c = self
            .builder
            .build_int_compare(IntPredicate::EQ, fl_left_h, fl_neg1, "l_is_c")
            .map_err(llvm_err)?;
        let fl_l_flat_bb = self.context.append_basic_block(fl_fn, "l_flat");
        let fl_l_done_bb = self.context.append_basic_block(fl_fn, "l_done");
        let _ = self
            .builder
            .build_conditional_branch(fl_l_is_c, fl_l_flat_bb, fl_l_done_bb);
        self.builder.position_at_end(fl_l_flat_bb);
        let fl_l_flat_v = self
            .builder
            .build_call(fl_fn, &[fl_left.into()], "l_flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let _ = self.builder.build_unconditional_branch(fl_l_done_bb);
        self.builder.position_at_end(fl_l_done_bb);
        let fl_l_phi = self
            .builder
            .build_phi(self.list_type, "l_phi")
            .map_err(llvm_err)?;
        fl_l_phi.add_incoming(&[(&fl_left, fl_concat)]);
        fl_l_phi.add_incoming(&[(&fl_l_flat_v, fl_l_flat_bb)]);
        let fl_l_final = fl_l_phi.as_basic_value().into_struct_value();
        let fl_l_node = self
            .builder
            .build_extract_value(fl_l_final, 0, "l_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_l_h = self
            .builder
            .build_extract_value(fl_l_final, 2, "l_fh")
            .map_err(llvm_err)?
            .into_int_value();

        let fl_r_is_c = self
            .builder
            .build_int_compare(IntPredicate::EQ, fl_right_h, fl_neg1, "r_is_c")
            .map_err(llvm_err)?;
        let fl_r_flat_bb = self.context.append_basic_block(fl_fn, "r_flat");
        let fl_r_done_bb = self.context.append_basic_block(fl_fn, "r_done");
        let _ = self
            .builder
            .build_conditional_branch(fl_r_is_c, fl_r_flat_bb, fl_r_done_bb);
        self.builder.position_at_end(fl_r_flat_bb);
        let fl_r_flat_v = self
            .builder
            .build_call(fl_fn, &[fl_right.into()], "r_flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let _ = self.builder.build_unconditional_branch(fl_r_done_bb);
        self.builder.position_at_end(fl_r_done_bb);
        let fl_r_phi = self
            .builder
            .build_phi(self.list_type, "r_phi")
            .map_err(llvm_err)?;
        fl_r_phi.add_incoming(&[(&fl_right, fl_l_done_bb)]);
        fl_r_phi.add_incoming(&[(&fl_r_flat_v, fl_r_flat_bb)]);
        let fl_r_final = fl_r_phi.as_basic_value().into_struct_value();
        let fl_r_node = self
            .builder
            .build_extract_value(fl_r_final, 0, "r_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_r_h = self
            .builder
            .build_extract_value(fl_r_final, 2, "r_fh")
            .map_err(llvm_err)?
            .into_int_value();

        let fl_empty_cc = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let fl_empty = fl_empty_cc.try_as_basic_value().unwrap_basic();
        let fl_acc = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fl_acc, fl_empty)
            .map_err(llvm_err)?;
        let fl_ps_fn = self
            .module
            .get_function("action_list_push_subtree")
            .unwrap();
        let _ = self
            .builder
            .build_call(
                fl_ps_fn,
                &[fl_acc.into(), fl_l_node.into(), fl_l_h.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                fl_ps_fn,
                &[fl_acc.into(), fl_r_node.into(), fl_r_h.into()],
                "",
            )
            .map_err(llvm_err)?;
        let fl_result = self
            .builder
            .build_load(self.list_type, fl_acc, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fl_result));
