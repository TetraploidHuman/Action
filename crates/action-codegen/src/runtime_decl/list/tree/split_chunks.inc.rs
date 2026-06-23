// ---- action_list_split_at({ptr, i64, i64}, i64) -> {ptr, i64, i64} ----
        let sa_fn = self.module.add_function(
            "action_list_split_at",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into()], false),
            None,
        );
        let sa_entry = self.context.append_basic_block(sa_fn, "entry");
        self.builder.position_at_end(sa_entry);
        let sa_in = sa_fn.get_first_param().unwrap().into_struct_value();
        let sa_idx = sa_fn.get_nth_param(1).unwrap().into_int_value();

        let sa_len = self
            .builder
            .build_extract_value(sa_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let sa_clamped = self
            .builder
            .build_int_compare(IntPredicate::SLT, sa_idx, i64.const_int(0, false), "cl")
            .map_err(llvm_err)?;
        let sa_idx0 = self
            .builder
            .build_select(sa_clamped, i64.const_int(0, false), sa_idx, "idx0")
            .map_err(llvm_err)?
            .into_int_value();
        let sa_cl2 = self
            .builder
            .build_int_compare(IntPredicate::SGT, sa_idx0, sa_len, "cl2")
            .map_err(llvm_err)?;
        let sa_idx_safe = self
            .builder
            .build_select(sa_cl2, sa_len, sa_idx0, "idx_safe")
            .map_err(llvm_err)?
            .into_int_value();
        let sa_done = self.context.append_basic_block(sa_fn, "done");
        let sa_take_fn = self.module.get_function("action_list_take").unwrap();
        let sa_drop_fn = self.module.get_function("action_list_drop").unwrap();
        let sa_left_cc = self
            .builder
            .build_call(sa_take_fn, &[sa_in.into(), sa_idx_safe.into()], "left")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let sa_right_cc = self
            .builder
            .build_call(sa_drop_fn, &[sa_in.into(), sa_idx_safe.into()], "right")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let sa_a1 = self
            .builder
            .build_alloca(self.list_type, "sa_a1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_a1, sa_left_cc)
            .map_err(llvm_err)?;
        let sa_a2 = self
            .builder
            .build_alloca(self.list_type, "sa_a2")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_a2, sa_right_cc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sa_done);
        self.builder.position_at_end(sa_done);
        // Return as list of 2 lists (same push pattern as chunks)
        let sa_res = self.call_rt("action_list_create", &[i64.const_int(2, false).into()])?;
        let sa_resv = sa_res.try_as_basic_value().unwrap_basic();
        let sa_ra = self
            .builder
            .build_alloca(self.list_type, "sa_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_ra, sa_resv).map_err(llvm_err)?;
        for (sa_src, sa_tag) in [(sa_a1, "1"), (sa_a2, "2")] {
            let sa_sub = self
                .builder
                .build_load(self.list_type, sa_src, &format!("l{sa_tag}f"))
                .map_err(llvm_err)?
                .into_struct_value();
            let sa_fat = self.string_type.get_undef();
            let sa_fatt = self
                .builder
                .build_insert_value(sa_fat, i64.const_int(6, false), 0, &format!("t{sa_tag}"))
                .map_err(llvm_err)?;
            let sa_sp = self
                .builder
                .build_alloca(self.list_type, &format!("sp{sa_tag}"))
                .map_err(llvm_err)?;
            self.builder.build_store(sa_sp, sa_sub).map_err(llvm_err)?;
            let sa_fatv = self
                .builder
                .build_insert_value(sa_fatt, sa_sp, 1, &format!("v{sa_tag}"))
                .map_err(llvm_err)?;
            let sa_rl = self
                .builder
                .build_load(self.list_type, sa_ra, "rl")
                .map_err(llvm_err)?
                .into_struct_value();
            let sa_rps = self.call_rt(
                "action_list_push",
                &[sa_rl.into(), sa_fatv.as_basic_value_enum().into()],
            )?;
            self.builder
                .build_store(sa_ra, sa_rps.try_as_basic_value().unwrap_basic())
                .map_err(llvm_err)?;
        }
        let sa_rt = self
            .builder
            .build_load(self.list_type, sa_ra, "sa_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sa_rt));

        // ---- action_list_chunks({ptr, i64, i64}, i64 chunk_size) -> {ptr, i64, i64} ----
        let ch_fn = self.module.add_function(
            "action_list_chunks",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into()], false),
            None,
        );
        let ch_entry = self.context.append_basic_block(ch_fn, "entry");
        self.builder.position_at_end(ch_entry);
        let ch_in = ch_fn.get_first_param().unwrap().into_struct_value();
        let ch_csize = ch_fn.get_nth_param(1).unwrap().into_int_value();

        let ch_len = self
            .builder
            .build_extract_value(ch_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_cz = self
            .builder
            .build_int_compare(IntPredicate::SLT, ch_csize, i64.const_int(1, false), "cz")
            .map_err(llvm_err)?;
        let ch_csafe = self
            .builder
            .build_select(ch_cz, i64.const_int(1, false), ch_csize, "csafe")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let ch_resv = ch_res.try_as_basic_value().unwrap_basic();
        let ch_ra = self
            .builder
            .build_alloca(self.list_type, "ch_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(ch_ra, ch_resv).map_err(llvm_err)?;
        let ch_i = self.builder.build_alloca(i64, "ch_i").map_err(llvm_err)?;
        self.builder
            .build_store(ch_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let ch_loop = self.context.append_basic_block(ch_fn, "loop");
        let ch_body = self.context.append_basic_block(ch_fn, "body");
        let ch_done = self.context.append_basic_block(ch_fn, "done");
        let _ = self.builder.build_unconditional_branch(ch_loop);
        self.builder.position_at_end(ch_loop);
        let ch_iv = self
            .builder
            .build_load(i64, ch_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ch_iv, ch_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ch_cond, ch_body, ch_done);
        self.builder.position_at_end(ch_body);
        let ch_slice_fn = self.module.get_function("action_list_slice").unwrap();
        let ch_end_raw = self
            .builder
            .build_int_add(ch_iv, ch_csafe, "end_raw")
            .map_err(llvm_err)?;
        let ch_sl_cc = self
            .builder
            .build_call(
                ch_slice_fn,
                &[ch_in.into(), ch_iv.into(), ch_end_raw.into()],
                "sl",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let ch_subl_fat = self.string_type.get_undef();
        let ch_sublft = self
            .builder
            .build_insert_value(ch_subl_fat, i64.const_int(6, false), 0, "st")
            .map_err(llvm_err)?;
        let ch_sp = self
            .builder
            .build_alloca(self.list_type, "ch_sp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_sp, ch_sl_cc)
            .map_err(llvm_err)?;
        let ch_sublfv = self
            .builder
            .build_insert_value(ch_sublft, ch_sp, 1, "sv")
            .map_err(llvm_err)?;
        let ch_rl = self
            .builder
            .build_load(self.list_type, ch_ra, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let ch_rps = self.call_rt(
            "action_list_push",
            &[ch_rl.into(), ch_sublfv.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(ch_ra, ch_rps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let ch_rem = self
            .builder
            .build_int_sub(ch_len, ch_iv, "rem")
            .map_err(llvm_err)?;
        let ch_step_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, ch_rem, ch_csafe, "step_gt")
            .map_err(llvm_err)?;
        let ch_step = self
            .builder
            .build_select(ch_step_gt, ch_csafe, ch_rem, "step")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_niv = self
            .builder
            .build_int_add(ch_iv, ch_step, "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(ch_i, ch_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ch_loop);
        self.builder.position_at_end(ch_done);
        let ch_rt = self
            .builder
            .build_load(self.list_type, ch_ra, "ch_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ch_rt));

        // ---- action_list_windows({ptr, i64, i64}, i64 win_size) -> {ptr, i64, i64} ----
        let wn_fn = self.module.add_function(
            "action_list_windows",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into()], false),
            None,
        );
        let wn_entry = self.context.append_basic_block(wn_fn, "entry");
        self.builder.position_at_end(wn_entry);
        let wn_in = wn_fn.get_first_param().unwrap().into_struct_value();
        let wn_wsize = wn_fn.get_nth_param(1).unwrap().into_int_value();

        let wn_len = self
            .builder
            .build_extract_value(wn_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_wz = self
            .builder
            .build_int_compare(IntPredicate::SLT, wn_wsize, i64.const_int(1, false), "wz")
            .map_err(llvm_err)?;
        let wn_wsafe = self
            .builder
            .build_select(wn_wz, i64.const_int(1, false), wn_wsize, "wsafe")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_tmp = self
            .builder
            .build_int_sub(wn_len, wn_wsafe, "tmp")
            .map_err(llvm_err)?;
        let wn_nw1 = self
            .builder
            .build_int_add(wn_tmp, i64.const_int(1, false), "nw1")
            .map_err(llvm_err)?;
        let wn_nz = self
            .builder
            .build_int_compare(IntPredicate::SLT, wn_nw1, i64.const_int(0, false), "nz")
            .map_err(llvm_err)?;
        let wn_nwin = self
            .builder
            .build_select(wn_nz, i64.const_int(0, false), wn_nw1, "nwin")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let wn_resv = wn_res.try_as_basic_value().unwrap_basic();
        let wn_ra = self
            .builder
            .build_alloca(self.list_type, "wn_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(wn_ra, wn_resv).map_err(llvm_err)?;
        let wn_i = self.builder.build_alloca(i64, "wn_i").map_err(llvm_err)?;
        self.builder
            .build_store(wn_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let wn_loop = self.context.append_basic_block(wn_fn, "loop");
        let wn_body = self.context.append_basic_block(wn_fn, "body");
        let wn_done = self.context.append_basic_block(wn_fn, "done");
        let _ = self.builder.build_unconditional_branch(wn_loop);
        self.builder.position_at_end(wn_loop);
        let wn_iv = self
            .builder
            .build_load(i64, wn_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, wn_iv, wn_nwin, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(wn_cond, wn_body, wn_done);
        self.builder.position_at_end(wn_body);
        let wn_slice_fn = self.module.get_function("action_list_slice").unwrap();
        let wn_end = self
            .builder
            .build_int_add(wn_iv, wn_wsafe, "end")
            .map_err(llvm_err)?;
        let wn_sl_cc = self
            .builder
            .build_call(
                wn_slice_fn,
                &[wn_in.into(), wn_iv.into(), wn_end.into()],
                "sl",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let wn_fat = self.string_type.get_undef();
        let wn_ft = self
            .builder
            .build_insert_value(wn_fat, i64.const_int(6, false), 0, "ft")
            .map_err(llvm_err)?;
        let wn_sp = self
            .builder
            .build_alloca(self.list_type, "wn_sp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wn_sp, wn_sl_cc)
            .map_err(llvm_err)?;
        let wn_fv = self
            .builder
            .build_insert_value(wn_ft, wn_sp, 1, "fv")
            .map_err(llvm_err)?;
        let wn_rl = self
            .builder
            .build_load(self.list_type, wn_ra, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let wn_rps = self.call_rt(
            "action_list_push",
            &[wn_rl.into(), wn_fv.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(wn_ra, wn_rps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let wn_ivi = self
            .builder
            .build_int_add(wn_iv, i64.const_int(1, false), "ivi")
            .map_err(llvm_err)?;
        self.builder.build_store(wn_i, wn_ivi).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(wn_loop);
        self.builder.position_at_end(wn_done);
        let wn_rt = self
            .builder
            .build_load(self.list_type, wn_ra, "wn_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&wn_rt));
