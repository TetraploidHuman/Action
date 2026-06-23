// ---- action_list_index_of({ptr, i64, i64}, {i64, ptr}) -> i64 ----
        let lio_fn = self.module.add_function(
            "action_list_index_of",
            i64.fn_type(&[self.list_type.into(), str_ty.into()], false),
            None,
        );
        let lio_entry = self.context.append_basic_block(lio_fn, "entry");
        self.builder.position_at_end(lio_entry);
        let lio_lst = lio_fn.get_first_param().unwrap().into_struct_value();
        let lio_tgt = lio_fn.get_nth_param(1).unwrap().into_struct_value();
        let lio_walk = self
            .module
            .get_function("action_list_index_of_walk")
            .unwrap();
        let lio_cc = self
            .builder
            .build_call(lio_walk, &[lio_lst.into(), lio_tgt.into()], "idx")
            .map_err(llvm_err)?;
        let lio_r = lio_cc
            .try_as_basic_value()
            .basic()
            .ok_or("index_of_walk failed")?;
        let _ = self.builder.build_return(Some(&lio_r));
