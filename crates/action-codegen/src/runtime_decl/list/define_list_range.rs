// Forward range copy: skip N elements then copy up to limit (take/drop hot path).

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(in crate::runtime_decl) fn define_list_range_walk_rec(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let i8 = self.context.i8_type();
        let i32 = self.context.i32_type();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let neg1 = i64.const_int(-1i64 as u64, true);
        let child_entry_ty = self.child_entry_type;
        let list_push_fn = self.module.get_function("action_list_push").unwrap();
        let push_subtree_fn = self
            .module
            .get_function("action_list_push_subtree")
            .unwrap();
        let str_rc_inc_fn = self.module.get_function("action_string_rc_inc").unwrap();

        let rng_fn = self
            .module
            .get_function("action_list_range_walk_rec")
            .unwrap();

        let entry = self.context.append_basic_block(rng_fn, "entry");
        let concat_bb = self.context.append_basic_block(rng_fn, "concat");
        let concat_skip_left = self.context.append_basic_block(rng_fn, "concat_skip_left");
        let concat_walk_left = self.context.append_basic_block(rng_fn, "concat_walk_left");
        let concat_walk_right = self.context.append_basic_block(rng_fn, "concat_walk_right");
        let h0_check = self.context.append_basic_block(rng_fn, "h0_check");
        let h0_leaf_check = self.context.append_basic_block(rng_fn, "h0_leaf_check");
        let h0_leaf = self.context.append_basic_block(rng_fn, "h0_leaf");
        let h0_loop = self.context.append_basic_block(rng_fn, "h0_loop");
        let h0_body = self.context.append_basic_block(rng_fn, "h0_body");
        let h0_skip_chk = self.context.append_basic_block(rng_fn, "h0_skip_chk");
        let h0_advance_skip = self.context.append_basic_block(rng_fn, "h0_advance_skip");
        let h0_push = self.context.append_basic_block(rng_fn, "h0_push");
        let h0_done = self.context.append_basic_block(rng_fn, "h0_done");
        let int_hdr = self.context.append_basic_block(rng_fn, "int_hdr");
        let int_loop = self.context.append_basic_block(rng_fn, "int_loop");
        let int_body = self.context.append_basic_block(rng_fn, "int_body");
        let int_load_child = self.context.append_basic_block(rng_fn, "int_load_child");
        let int_skip_whole = self.context.append_basic_block(rng_fn, "int_skip_whole");
        let int_skip_branch = self.context.append_basic_block(rng_fn, "int_skip_branch");
        let int_skip_part = self.context.append_basic_block(rng_fn, "int_skip_part");
        let int_copy_branch = self.context.append_basic_block(rng_fn, "int_copy_branch");
        let int_copy_whole = self.context.append_basic_block(rng_fn, "int_copy_whole");
        let int_copy_part = self.context.append_basic_block(rng_fn, "int_copy_part");
        let int_next = self.context.append_basic_block(rng_fn, "int_next");
        let int_done = self.context.append_basic_block(rng_fn, "int_done");
        let done = self.context.append_basic_block(rng_fn, "done");

        self.builder.position_at_end(entry);
        let rng_acc = rng_fn.get_first_param().unwrap().into_pointer_value();
        let rng_node = rng_fn.get_nth_param(1).unwrap().into_pointer_value();
        let rng_height = rng_fn.get_nth_param(2).unwrap().into_int_value();
        let rng_skip_p = rng_fn.get_nth_param(3).unwrap().into_pointer_value();
        let rng_limit_p = rng_fn.get_nth_param(4).unwrap().into_pointer_value();

        let rng_limit0 = self
            .builder
            .build_load(i64, rng_limit_p, "lim0")
            .map_err(llvm_err)?
            .into_int_value();
        let rng_lim_done = self
            .builder
            .build_int_compare(IntPredicate::SLE, rng_limit0, zero, "lim_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rng_lim_done, done, h0_check);

        self.builder.position_at_end(h0_check);
        let rng_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, rng_height, neg1, "is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rng_is_concat, concat_bb, h0_leaf_check);

        self.builder.position_at_end(h0_leaf_check);
        let rng_is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, rng_height, zero, "is_h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rng_is_h0, h0_leaf, int_hdr);

        // ConcatNode
        self.builder.position_at_end(concat_bb);
        let cn_ll_p = unsafe {
            self.builder
                .build_gep(i64, rng_node, &[i64.const_int(3, false)], "cn_ll_p")
                .map_err(llvm_err)
        }?;
        let cn_left_len = self
            .builder
            .build_load(i64, cn_ll_p, "cn_ll")
            .map_err(llvm_err)?
            .into_int_value();
        let cn_ln_p = unsafe {
            self.builder
                .build_gep(ptr, rng_node, &[i64.const_int(2, false)], "cn_ln_p")
                .map_err(llvm_err)
        }?;
        let cn_left_node = self
            .builder
            .build_load(ptr, cn_ln_p, "cn_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cn_lh_p = unsafe {
            self.builder
                .build_gep(i64, rng_node, &[i64.const_int(4, false)], "cn_lh_p")
                .map_err(llvm_err)
        }?;
        let cn_left_h = self
            .builder
            .build_load(i64, cn_lh_p, "cn_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let cn_rn_p = unsafe {
            self.builder
                .build_gep(ptr, rng_node, &[i64.const_int(5, false)], "cn_rn_p")
                .map_err(llvm_err)
        }?;
        let cn_right_node = self
            .builder
            .build_load(ptr, cn_rn_p, "cn_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cn_rh_p = unsafe {
            self.builder
                .build_gep(i64, rng_node, &[i64.const_int(7, false)], "cn_rh_p")
                .map_err(llvm_err)
        }?;
        let cn_right_h = self
            .builder
            .build_load(i64, cn_rh_p, "cn_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let cn_skip = self
            .builder
            .build_load(i64, rng_skip_p, "cn_skip")
            .map_err(llvm_err)?
            .into_int_value();
        let cn_skip_ge = self
            .builder
            .build_int_compare(IntPredicate::SGE, cn_skip, cn_left_len, "cn_skip_ge")
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(cn_skip_ge, concat_skip_left, concat_walk_left);

        self.builder.position_at_end(concat_skip_left);
        let cn_new_skip = self
            .builder
            .build_int_sub(cn_skip, cn_left_len, "cn_nskip")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rng_skip_p, cn_new_skip)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                rng_fn,
                &[
                    rng_acc.into(),
                    cn_right_node.into(),
                    cn_right_h.into(),
                    rng_skip_p.into(),
                    rng_limit_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(done);

        self.builder.position_at_end(concat_walk_left);
        let _ = self
            .builder
            .build_call(
                rng_fn,
                &[
                    rng_acc.into(),
                    cn_left_node.into(),
                    cn_left_h.into(),
                    rng_skip_p.into(),
                    rng_limit_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let cn_lim = self
            .builder
            .build_load(i64, rng_limit_p, "cn_lim")
            .map_err(llvm_err)?
            .into_int_value();
        let cn_lim_done = self
            .builder
            .build_int_compare(IntPredicate::SLE, cn_lim, zero, "cn_lim_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cn_lim_done, done, concat_walk_right);

        self.builder.position_at_end(concat_walk_right);
        self.builder
            .build_store(rng_skip_p, zero)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                rng_fn,
                &[
                    rng_acc.into(),
                    cn_right_node.into(),
                    cn_right_h.into(),
                    rng_skip_p.into(),
                    rng_limit_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(done);

        // h=0 leaf: initialize pos from skip so we copy [skip .. skip+limit)
        self.builder.position_at_end(h0_leaf);
        let h0_i8 = self
            .builder
            .build_pointer_cast(rng_node, ptr, "h0_i8")
            .map_err(llvm_err)?;
        let h0_cnt_r = self
            .builder
            .build_load(i32, h0_i8, "h0_cnt_r")
            .map_err(llvm_err)?
            .into_int_value();
        let h0_cnt = self
            .builder
            .build_int_z_extend(h0_cnt_r, i64, "h0_cnt")
            .map_err(llvm_err)?;
        let h0_skip_init = self
            .builder
            .build_load(i64, rng_skip_p, "h0_skip_init")
            .map_err(llvm_err)?
            .into_int_value();
        let h0_pos_a = self.builder.build_alloca(i64, "h0_pos").map_err(llvm_err)?;
        self.builder
            .build_store(h0_pos_a, h0_skip_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(rng_skip_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(h0_loop);

        self.builder.position_at_end(h0_loop);
        let h0_pos = self
            .builder
            .build_load(i64, h0_pos_a, "h0_pos_v")
            .map_err(llvm_err)?
            .into_int_value();
        let h0_pos_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, h0_pos, h0_cnt, "h0_pos_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(h0_pos_done, h0_done, h0_body);

        self.builder.position_at_end(h0_body);
        let h0_lim = self
            .builder
            .build_load(i64, rng_limit_p, "h0_lim")
            .map_err(llvm_err)?
            .into_int_value();
        let h0_lim_done = self
            .builder
            .build_int_compare(IntPredicate::SLE, h0_lim, zero, "h0_lim_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(h0_lim_done, h0_done, h0_skip_chk);

        self.builder.position_at_end(h0_skip_chk);
        let h0_skip = self
            .builder
            .build_load(i64, rng_skip_p, "h0_skip")
            .map_err(llvm_err)?
            .into_int_value();
        let h0_has_skip = self
            .builder
            .build_int_compare(IntPredicate::SGT, h0_skip, zero, "h0_has_skip")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(h0_has_skip, h0_advance_skip, h0_push);

        self.builder.position_at_end(h0_advance_skip);
        let h0_skip_dec = self
            .builder
            .build_int_sub(h0_skip, one, "h0_skip_dec")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rng_skip_p, h0_skip_dec)
            .map_err(llvm_err)?;
        let h0_pos_inc_skip = self
            .builder
            .build_int_add(h0_pos, one, "h0_pos_inc_s")
            .map_err(llvm_err)?;
        self.builder
            .build_store(h0_pos_a, h0_pos_inc_skip)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(h0_loop);

        self.builder.position_at_end(h0_push);
        let h0_eb = unsafe {
            self.builder
                .build_gep(i8, h0_i8, &[i64.const_int(8, false)], "h0_eb")
                .map_err(llvm_err)
        }?;
        let h0_ep = unsafe {
            self.builder
                .build_gep(self.string_type, h0_eb, &[h0_pos], "h0_ep")
                .map_err(llvm_err)
        }?;
        let h0_elem = self
            .builder
            .build_load(self.string_type, h0_ep, "h0_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let _ = self
            .builder
            .build_call(str_rc_inc_fn, &[h0_elem.into()], "")
            .map_err(llvm_err)?;
        let h0_cur = self
            .builder
            .build_load(self.list_type, rng_acc, "h0_cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let h0_pushed = self
            .builder
            .build_call(list_push_fn, &[h0_cur.into(), h0_elem.into()], "h0_pushed")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("h0 push failed")?;
        self.builder
            .build_store(rng_acc, h0_pushed)
            .map_err(llvm_err)?;
        let h0_lim_dec = self
            .builder
            .build_int_sub(h0_lim, one, "h0_lim_dec")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rng_limit_p, h0_lim_dec)
            .map_err(llvm_err)?;
        let h0_pos_inc = self
            .builder
            .build_int_add(h0_pos, one, "h0_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(h0_pos_a, h0_pos_inc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(h0_loop);

        self.builder.position_at_end(h0_done);
        let _ = self.builder.build_unconditional_branch(done);

        // internal node
        self.builder.position_at_end(int_hdr);
        let int_i8 = self
            .builder
            .build_pointer_cast(rng_node, ptr, "int_i8")
            .map_err(llvm_err)?;
        let int_cnt_r = self
            .builder
            .build_load(i32, int_i8, "int_cnt_r")
            .map_err(llvm_err)?
            .into_int_value();
        let int_cnt = self
            .builder
            .build_int_z_extend(int_cnt_r, i64, "int_cnt")
            .map_err(llvm_err)?;
        let int_i_a = self.builder.build_alloca(i64, "int_i").map_err(llvm_err)?;
        self.builder.build_store(int_i_a, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_loop);

        self.builder.position_at_end(int_loop);
        let int_iv = self
            .builder
            .build_load(i64, int_i_a, "int_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let int_iv_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, int_iv, int_cnt, "int_iv_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(int_iv_done, int_done, int_body);

        self.builder.position_at_end(int_body);
        let int_lim = self
            .builder
            .build_load(i64, rng_limit_p, "int_lim")
            .map_err(llvm_err)?
            .into_int_value();
        let int_lim_done = self
            .builder
            .build_int_compare(IntPredicate::SLE, int_lim, zero, "int_lim_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(int_lim_done, int_done, int_load_child);

        self.builder.position_at_end(int_load_child);
        let int_ce_off = self
            .builder
            .build_int_add(
                i64.const_int(16, false),
                self.builder
                    .build_int_mul(int_iv, i64.const_int(16, false), "int_ce_off_m")
                    .map_err(llvm_err)?,
                "int_ce_off",
            )
            .map_err(llvm_err)?;
        let int_ce_p = unsafe {
            self.builder
                .build_gep(i8, int_i8, &[int_ce_off], "int_ce_p")
                .map_err(llvm_err)
        }?;
        let int_ce = self
            .builder
            .build_load(child_entry_ty, int_ce_p, "int_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let int_child = self
            .builder
            .build_extract_value(int_ce, 0, "int_child")
            .map_err(llvm_err)?
            .into_pointer_value();
        let int_st = self
            .builder
            .build_extract_value(int_ce, 1, "int_st")
            .map_err(llvm_err)?
            .into_int_value();
        let int_child_h = self
            .builder
            .build_int_sub(rng_height, one, "int_child_h")
            .map_err(llvm_err)?;
        let int_skip = self
            .builder
            .build_load(i64, rng_skip_p, "int_skip")
            .map_err(llvm_err)?
            .into_int_value();
        let int_skip_ge = self
            .builder
            .build_int_compare(IntPredicate::SGE, int_skip, int_st, "int_skip_ge")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(int_skip_ge, int_skip_whole, int_skip_branch);

        self.builder.position_at_end(int_skip_branch);
        let int_skip_gt0 = self
            .builder
            .build_int_compare(IntPredicate::SGT, int_skip, zero, "int_skip_gt0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(int_skip_gt0, int_skip_part, int_copy_branch);

        self.builder.position_at_end(int_copy_branch);
        let int_lim_ge = self
            .builder
            .build_int_compare(IntPredicate::SGE, int_lim, int_st, "int_lim_ge")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(int_lim_ge, int_copy_whole, int_copy_part);

        self.builder.position_at_end(int_skip_whole);
        let int_skip_sub = self
            .builder
            .build_int_sub(int_skip, int_st, "int_skip_sub")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rng_skip_p, int_skip_sub)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_next);

        self.builder.position_at_end(int_skip_part);
        let _ = self
            .builder
            .build_call(
                rng_fn,
                &[
                    rng_acc.into(),
                    int_child.into(),
                    int_child_h.into(),
                    rng_skip_p.into(),
                    rng_limit_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(rng_skip_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_next);

        self.builder.position_at_end(int_copy_whole);
        let _ = self
            .builder
            .build_call(
                push_subtree_fn,
                &[rng_acc.into(), int_child.into(), int_child_h.into()],
                "",
            )
            .map_err(llvm_err)?;
        let int_lim_sub = self
            .builder
            .build_int_sub(int_lim, int_st, "int_lim_sub")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rng_limit_p, int_lim_sub)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_next);

        self.builder.position_at_end(int_copy_part);
        let _ = self
            .builder
            .build_call(
                rng_fn,
                &[
                    rng_acc.into(),
                    int_child.into(),
                    int_child_h.into(),
                    rng_skip_p.into(),
                    rng_limit_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_done);

        self.builder.position_at_end(int_next);
        let int_iv_next = self
            .builder
            .build_int_add(int_iv, one, "int_iv_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(int_i_a, int_iv_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_loop);

        self.builder.position_at_end(int_done);
        let _ = self.builder.build_unconditional_branch(done);

        self.builder.position_at_end(done);
        let _ = self.builder.build_return(None);

        Ok(())
    }
}
