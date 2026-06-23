// ---- action_list_contains({ptr, i64, i64}, {i64, ptr}) -> i1 ----
        let lc_fn = self.module.add_function(
            "action_list_contains",
            b1.fn_type(&[self.list_type.into(), self.string_type.into()], false),
            None,
        );
        let lc_entry = self.context.append_basic_block(lc_fn, "entry");
        self.builder.position_at_end(lc_entry);
        let lc_list = lc_fn.get_first_param().unwrap().into_struct_value();
        let lc_key = lc_fn.get_nth_param(1).unwrap().into_struct_value();
        let lc_node = self
            .builder
            .build_extract_value(lc_list, 0, "lc_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lc_height = self
            .builder
            .build_extract_value(lc_list, 2, "lc_height")
            .map_err(llvm_err)?
            .into_int_value();
        let lc_hit = self
            .builder
            .build_call(
                lc_walk_fn,
                &[lc_node.into(), lc_height.into(), lc_key.into()],
                "lc_hit",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&lc_hit));
