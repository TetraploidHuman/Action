// insert_rec fallback: split a full leaf child and insert a sibling under an internal node.

use super::{llvm_err, CodeGen};
use inkwell::values::FunctionValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_insert_split_child(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let one = i64.const_int(1, false);
        let half = i64.const_int(32, false);
        let half_sz = i64.const_int(32 * 16, false);
        let fanout = i64.const_int(64, false);
        let null_ptr = ptr.const_null();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        let rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();

        let split_fn = self.module.add_function(
            "action_list_insert_rec_split_child",
            ptr.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    ptr.into(),
                    i64.into(),
                    self.string_type.into(),
                    i64.into(),
                ],
                false,
            ),
            None,
        );

        let entry = self.context.append_basic_block(split_fn, "entry");
        let fail = self.context.append_basic_block(split_fn, "fail");
        let leaf_cow = self.context.append_basic_block(split_fn, "leaf_cow");
        let leaf_cow_copy = self.context.append_basic_block(split_fn, "leaf_cow_copy");
        let leaf_ready = self.context.append_basic_block(split_fn, "leaf_ready");
        let do_split = self.context.append_basic_block(split_fn, "do_split");
        let ins_left = self.context.append_basic_block(split_fn, "ins_left");
        let ins_right = self.context.append_basic_block(split_fn, "ins_right");
        let shift_loop = self.context.append_basic_block(split_fn, "shift_loop");
        let shift_entry = self.context.append_basic_block(split_fn, "shift_entry");
        let shift_body = self.context.append_basic_block(split_fn, "shift_body");
        let shift_done = self.context.append_basic_block(split_fn, "shift_done");
        let left_shift_loop = self.context.append_basic_block(split_fn, "left_shift_loop");
        let left_shift_entry = self
            .context
            .append_basic_block(split_fn, "left_shift_entry");
        let left_shift_body = self.context.append_basic_block(split_fn, "left_shift_body");
        let left_shift_done = self.context.append_basic_block(split_fn, "left_shift_done");
        let right_shift_loop = self
            .context
            .append_basic_block(split_fn, "right_shift_loop");
        let right_shift_entry = self
            .context
            .append_basic_block(split_fn, "right_shift_entry");
        let right_shift_body = self
            .context
            .append_basic_block(split_fn, "right_shift_body");
        let right_shift_done = self
            .context
            .append_basic_block(split_fn, "right_shift_done");
        let store_sibling = self.context.append_basic_block(split_fn, "store_sibling");
        let ok_ret = self.context.append_basic_block(split_fn, "ok_ret");

        self.builder.position_at_end(entry);
        let intl_node = split_fn.get_first_param().unwrap().into_pointer_value();
        let found_i = split_fn.get_nth_param(1).unwrap().into_int_value();
        let leaf_node = split_fn.get_nth_param(2).unwrap().into_pointer_value();
        let local_idx = split_fn.get_nth_param(3).unwrap().into_int_value();
        let ins_val = split_fn.get_nth_param(4).unwrap().into_struct_value();
        let list_root_rc = split_fn.get_nth_param(5).unwrap().into_int_value();

        let intl_i8 = self
            .builder
            .build_pointer_cast(intl_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let int_count_raw = self
            .builder
            .build_load(i32, intl_i8, "int_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let int_count = self
            .builder
            .build_int_z_extend(int_count_raw, i64, "int_count")
            .map_err(llvm_err)?;
        let int_full = self
            .builder
            .build_int_compare(IntPredicate::SGE, int_count, fanout, "int_full")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(int_full, fail, leaf_cow);

        // CoW leaf before split
        self.builder.position_at_end(leaf_cow);
        let leaf_int = self
            .builder
            .build_ptr_to_int(leaf_node, i64, "leaf_int")
            .map_err(llvm_err)?;
        let leaf_rc_a = self
            .builder
            .build_int_sub(leaf_int, i64.const_int(8, false), "leaf_rc_a")
            .map_err(llvm_err)?;
        let leaf_rc_p = self
            .builder
            .build_int_to_ptr(leaf_rc_a, ptr, "leaf_rc_p")
            .map_err(llvm_err)?;
        let leaf_rc = self
            .builder
            .build_load(i64, leaf_rc_p, "leaf_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_shared = self
            .builder
            .build_or(
                self.builder
                    .build_int_compare(IntPredicate::SGT, leaf_rc, one, "leaf_sh_rc")
                    .map_err(llvm_err)?,
                self.builder
                    .build_int_compare(IntPredicate::SGT, list_root_rc, one, "leaf_sh_root")
                    .map_err(llvm_err)?,
                "leaf_shared",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(leaf_shared, leaf_cow_copy, leaf_ready);

        self.builder.position_at_end(leaf_cow_copy);
        let leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let cow_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "cow_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[cow_leaf.into(), leaf_node.into(), leaf_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let leaf_do_xfer = self
            .builder
            .build_int_compare(IntPredicate::SGT, leaf_rc, one, "leaf_do_xfer")
            .map_err(llvm_err)?;
        let leaf_xfer = self.context.append_basic_block(split_fn, "leaf_xfer");
        let leaf_no_xfer = self.context.append_basic_block(split_fn, "leaf_no_xfer");
        let _ = self
            .builder
            .build_conditional_branch(leaf_do_xfer, leaf_xfer, leaf_no_xfer);
        self.builder.position_at_end(leaf_xfer);
        let _ = self.builder.build_store(
            leaf_rc_p,
            self.builder
                .build_int_sub(leaf_rc, one, "leaf_rc_dec")
                .map_err(llvm_err)?,
        );
        let _ = self.builder.build_unconditional_branch(leaf_ready);
        self.builder.position_at_end(leaf_no_xfer);
        let _ = self.builder.build_unconditional_branch(leaf_ready);

        self.builder.position_at_end(leaf_ready);
        let work_leaf_phi = self.builder.build_phi(ptr, "work_leaf").map_err(llvm_err)?;
        work_leaf_phi.add_incoming(&[
            (&leaf_node, leaf_cow),
            (&cow_leaf, leaf_xfer),
            (&cow_leaf, leaf_no_xfer),
        ]);
        let work_leaf = work_leaf_phi.as_basic_value().into_pointer_value();
        let _ = self.builder.build_unconditional_branch(do_split);

        // Split work_leaf: left [0..32], right [32..64] copied to new leaf
        self.builder.position_at_end(do_split);
        let wl_i8 = self
            .builder
            .build_pointer_cast(work_leaf, ptr, "wl_i8")
            .map_err(llvm_err)?;
        let wl_eb = unsafe {
            self.builder
                .build_gep(i8, wl_i8, &[i64.const_int(8, false)], "wl_eb")
                .map_err(llvm_err)?
        };
        let right_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "right_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let rl_i8 = self
            .builder
            .build_pointer_cast(right_leaf, ptr, "rl_i8")
            .map_err(llvm_err)?;
        let rl_eb = unsafe {
            self.builder
                .build_gep(i8, rl_i8, &[i64.const_int(8, false)], "rl_eb")
                .map_err(llvm_err)?
        };
        let src32 = unsafe {
            self.builder
                .build_gep(self.string_type, wl_eb, &[half], "src32")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_call(memcpy_fn, &[rl_eb.into(), src32.into(), half_sz.into()], "")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(wl_i8, i32.const_int(32, false))
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(rl_i8, i32.const_int(32, false))
            .map_err(llvm_err)?;

        let idx_in_left = self
            .builder
            .build_int_compare(IntPredicate::SLE, local_idx, half, "idx_in_left")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(idx_in_left, ins_left, ins_right);

        // Insert into left half [0..32)
        self.builder.position_at_end(ins_left);
        let left_idx = local_idx;
        let _ = self.builder.build_unconditional_branch(left_shift_entry);
        self.builder.position_at_end(left_shift_entry);
        let _ = self.builder.build_unconditional_branch(left_shift_loop);
        self.builder.position_at_end(left_shift_loop);
        let ls_i = self.builder.build_phi(i64, "ls_i").map_err(llvm_err)?;
        let ls_done = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                ls_i.as_basic_value().into_int_value(),
                left_idx,
                "ls_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ls_done, left_shift_done, left_shift_body);
        self.builder.position_at_end(left_shift_body);
        let ls_cur = ls_i.as_basic_value().into_int_value();
        let ls_from = self
            .builder
            .build_int_sub(half, one, "ls_from")
            .map_err(llvm_err)?;
        let ls_src = self
            .builder
            .build_int_sub(ls_from, ls_cur, "ls_src")
            .map_err(llvm_err)?;
        let ls_dst = self
            .builder
            .build_int_sub(half, ls_cur, "ls_dst")
            .map_err(llvm_err)?;
        let ls_sp = unsafe {
            self.builder
                .build_gep(self.string_type, wl_eb, &[ls_src], "ls_sp")
                .map_err(llvm_err)?
        };
        let ls_dp = unsafe {
            self.builder
                .build_gep(self.string_type, wl_eb, &[ls_dst], "ls_dp")
                .map_err(llvm_err)?
        };
        let ls_v = self
            .builder
            .build_load(self.string_type, ls_sp, "ls_v")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(ls_dp, ls_v).map_err(llvm_err)?;
        let ls_next = self
            .builder
            .build_int_add(ls_cur, one, "ls_next")
            .map_err(llvm_err)?;
        let ls_body_bb = self.builder.get_insert_block().unwrap();
        ls_i.add_incoming(&[(&half, left_shift_entry), (&ls_next, ls_body_bb)]);
        let _ = self.builder.build_unconditional_branch(left_shift_loop);
        self.builder.position_at_end(left_shift_done);
        let ls_ins_p = unsafe {
            self.builder
                .build_gep(self.string_type, wl_eb, &[left_idx], "ls_ins_p")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(ls_ins_p, ins_val)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(wl_i8, i32.const_int(33, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(store_sibling);

        // Insert into right half
        self.builder.position_at_end(ins_right);
        let right_idx = self
            .builder
            .build_int_sub(local_idx, half, "right_idx")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(right_shift_entry);
        self.builder.position_at_end(right_shift_entry);
        let _ = self.builder.build_unconditional_branch(right_shift_loop);
        self.builder.position_at_end(right_shift_loop);
        let rs_i = self.builder.build_phi(i64, "rs_i").map_err(llvm_err)?;
        let rs_done = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                rs_i.as_basic_value().into_int_value(),
                right_idx,
                "rs_done",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rs_done, right_shift_done, right_shift_body);
        self.builder.position_at_end(right_shift_body);
        let rs_cur = rs_i.as_basic_value().into_int_value();
        let rs_from = self
            .builder
            .build_int_sub(half, one, "rs_from")
            .map_err(llvm_err)?;
        let rs_src = self
            .builder
            .build_int_sub(rs_from, rs_cur, "rs_src")
            .map_err(llvm_err)?;
        let rs_dst = self
            .builder
            .build_int_sub(half, rs_cur, "rs_dst")
            .map_err(llvm_err)?;
        let rs_sp = unsafe {
            self.builder
                .build_gep(self.string_type, rl_eb, &[rs_src], "rs_sp")
                .map_err(llvm_err)?
        };
        let rs_dp = unsafe {
            self.builder
                .build_gep(self.string_type, rl_eb, &[rs_dst], "rs_dp")
                .map_err(llvm_err)?
        };
        let rs_v = self
            .builder
            .build_load(self.string_type, rs_sp, "rs_v")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(rs_dp, rs_v).map_err(llvm_err)?;
        let rs_next = self
            .builder
            .build_int_add(rs_cur, one, "rs_next")
            .map_err(llvm_err)?;
        let rs_body_bb = self.builder.get_insert_block().unwrap();
        rs_i.add_incoming(&[(&half, right_shift_entry), (&rs_next, rs_body_bb)]);
        let _ = self.builder.build_unconditional_branch(right_shift_loop);
        self.builder.position_at_end(right_shift_done);
        let rs_ins_p = unsafe {
            self.builder
                .build_gep(self.string_type, rl_eb, &[right_idx], "rs_ins_p")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(rs_ins_p, ins_val)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(rl_i8, i32.const_int(33, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(store_sibling);

        // store_sibling: update left child, shift siblings, insert right
        self.builder.position_at_end(store_sibling);
        let left_count_phi = self
            .builder
            .build_phi(i64, "left_count")
            .map_err(llvm_err)?;
        left_count_phi.add_incoming(&[
            (&i64.const_int(33, false), left_shift_done),
            (&half, right_shift_done),
        ]);
        let right_count_phi = self
            .builder
            .build_phi(i64, "right_count")
            .map_err(llvm_err)?;
        right_count_phi.add_incoming(&[
            (&half, left_shift_done),
            (&i64.const_int(33, false), right_shift_done),
        ]);
        let lc = left_count_phi.as_basic_value().into_int_value();
        let rc = right_count_phi.as_basic_value().into_int_value();

        let children_base = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "cb")
                .map_err(llvm_err)?
        };
        let left_slot = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[found_i],
                    "left_slot",
                )
                .map_err(llvm_err)?
        };
        let left_p = self
            .builder
            .build_pointer_cast(left_slot, ptr, "left_p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(left_p, work_leaf)
            .map_err(llvm_err)?;
        let left_st_p = unsafe {
            self.builder
                .build_gep(i64, left_p, &[one], "left_st_p")
                .map_err(llvm_err)?
        };
        let _ = self.builder.build_store(left_st_p, lc).map_err(llvm_err)?;

        let sibling_i = self
            .builder
            .build_int_add(found_i, one, "sibling_i")
            .map_err(llvm_err)?;
        let last_child = self
            .builder
            .build_int_sub(int_count, one, "last_child")
            .map_err(llvm_err)?;
        let need_shift = self
            .builder
            .build_int_compare(IntPredicate::SLE, sibling_i, last_child, "need_shift")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(need_shift, shift_entry, shift_done);

        self.builder.position_at_end(shift_entry);
        let _ = self.builder.build_unconditional_branch(shift_loop);

        self.builder.position_at_end(shift_loop);
        let sh_i = self.builder.build_phi(i64, "sh_i").map_err(llvm_err)?;
        let sh_cur = sh_i.as_basic_value().into_int_value();
        let sh_done = self
            .builder
            .build_int_compare(IntPredicate::SLT, sh_cur, last_child, "sh_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sh_done, shift_body, shift_done);
        self.builder.position_at_end(shift_body);
        let sh_src_i = sh_cur;
        let sh_dst_i = self
            .builder
            .build_int_add(sh_cur, one, "sh_dst_i")
            .map_err(llvm_err)?;
        let sh_src = unsafe {
            self.builder
                .build_gep(self.child_entry_type, children_base, &[sh_src_i], "sh_src")
                .map_err(llvm_err)?
        };
        let sh_dst = unsafe {
            self.builder
                .build_gep(self.child_entry_type, children_base, &[sh_dst_i], "sh_dst")
                .map_err(llvm_err)?
        };
        let sh_v = self
            .builder
            .build_load(self.child_entry_type, sh_src, "sh_v")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(sh_dst, sh_v).map_err(llvm_err)?;
        let sh_next = self
            .builder
            .build_int_add(sh_cur, one, "sh_next")
            .map_err(llvm_err)?;
        let sh_body_bb = self.builder.get_insert_block().unwrap();
        sh_i.add_incoming(&[(&last_child, shift_entry), (&sh_next, sh_body_bb)]);
        let _ = self.builder.build_unconditional_branch(shift_loop);

        self.builder.position_at_end(shift_done);
        let right_slot = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[sibling_i],
                    "right_slot",
                )
                .map_err(llvm_err)?
        };
        let right_p = self
            .builder
            .build_pointer_cast(right_slot, ptr, "right_p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(right_p, right_leaf)
            .map_err(llvm_err)?;
        let right_st_p = unsafe {
            self.builder
                .build_gep(i64, right_p, &[one], "right_st_p")
                .map_err(llvm_err)?
        };
        let _ = self.builder.build_store(right_st_p, rc).map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(rc_inc_fn, &[right_leaf.into()], "")
            .map_err(llvm_err)?;

        let new_count = self
            .builder
            .build_int_add(int_count, one, "new_count")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(
                intl_i8,
                self.builder
                    .build_int_truncate(new_count, i32, "new_count32")
                    .map_err(llvm_err)?,
            )
            .map_err(llvm_err)?;
        let total_p = unsafe {
            self.builder
                .build_gep(i64, intl_i8, &[one], "total_p")
                .map_err(llvm_err)?
        };
        let total_v = self
            .builder
            .build_load(i64, total_p, "total_v")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_store(
                total_p,
                self.builder
                    .build_int_add(total_v, one, "total_new")
                    .map_err(llvm_err)?,
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ok_ret);

        self.builder.position_at_end(ok_ret);
        let _ = self.builder.build_return(Some(&intl_node));

        self.builder.position_at_end(fail);
        let _ = self.builder.build_return(Some(&null_ptr));

        let _split_fn: FunctionValue<'ctx> = split_fn;
        Ok(())
    }
}
