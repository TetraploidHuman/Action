// ---- action_list_print({ptr, i64, i64}) ----
        let list_print_fn = self.module.add_function(
            "action_list_print",
            void.fn_type(&[self.list_type.into()], false),
            None,
        );
        let lp_entry = self.context.append_basic_block(list_print_fn, "entry");
        self.builder.position_at_end(lp_entry);
        let lp_list = list_print_fn.get_first_param().unwrap().into_struct_value();
        let lp_len = self
            .builder
            .build_extract_value(lp_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        // Print "["
        let _ = self.builder.build_call(printf_fn, &[fmt_lb_ptr.into()], "");
        let lp_i = self.builder.build_alloca(i64, "lpi").map_err(llvm_err)?;
        self.builder
            .build_store(lp_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let lp_hdr = self.context.append_basic_block(list_print_fn, "lphdr");
        let lp_bdy = self.context.append_basic_block(list_print_fn, "lpbdy");
        let lp_ext = self.context.append_basic_block(list_print_fn, "lpext");
        let _ = self.builder.build_unconditional_branch(lp_hdr);
        self.builder.position_at_end(lp_hdr);
        let lp_iv = self
            .builder
            .build_load(i64, lp_i, "lpiv")
            .map_err(llvm_err)?
            .into_int_value();
        let lp_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lp_iv, lp_len, "lpcond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lp_cond, lp_bdy, lp_ext);
        self.builder.position_at_end(lp_bdy);
        // Print ", " if not first
        let lp_is_first = self
            .builder
            .build_int_compare(IntPredicate::EQ, lp_iv, i64.const_int(0, false), "is_first")
            .map_err(llvm_err)?;
        let lp_sep_bb = self.context.append_basic_block(list_print_fn, "lpsep");
        let lp_val_bb = self.context.append_basic_block(list_print_fn, "lpval");
        let _ = self
            .builder
            .build_conditional_branch(lp_is_first, lp_val_bb, lp_sep_bb);
        self.builder.position_at_end(lp_sep_bb);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_sep_ptr.into()], "");
        let _ = self.builder.build_unconditional_branch(lp_val_bb);
        self.builder.position_at_end(lp_val_bb);
        let lp_elem_val = self
            .builder
            .build_call(list_get_fn, &[lp_list.into(), lp_iv.into()], "lpe")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?;
        let lp_elem = lp_elem_val.into_struct_value();
        let lp_tag = self
            .builder
            .build_extract_value(lp_elem, 0, "lptag")
            .map_err(llvm_err)?
            .into_int_value();
        // Print integer tag for now
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), lp_tag.into()], "");
        // Next
        let lp_next = self
            .builder
            .build_int_add(lp_iv, i64.const_int(1, false), "lpnext")
            .map_err(llvm_err)?;
        self.builder.build_store(lp_i, lp_next).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_hdr);
        self.builder.position_at_end(lp_ext);
        let _ = self.builder.build_call(printf_fn, &[fmt_rb_ptr.into()], "");
        let _ = self.builder.build_return(None);
