// B-tree path-copy remove (CoW when rc > 1 or list root is shared). Returns null on failure.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_remove_rec(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let null_ptr = ptr.const_null();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        let rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let str_rc_dec_fn = self.module.get_function("action_string_rc_dec").unwrap();

        let lrr_fn = self.module.add_function(
            "action_list_remove_rec",
            ptr.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    i64.into(),
                    i64.into(),
                    ptr.into(),
                ],
                false,
            ),
            None,
        );

        let entry = self.context.append_basic_block(lrr_fn, "entry");
        let leaf = self.context.append_basic_block(lrr_fn, "leaf");
        let leaf_cow = self.context.append_basic_block(lrr_fn, "leaf_cow");
        let leaf_cow_copy = self.context.append_basic_block(lrr_fn, "leaf_cow_copy");
        let leaf_xfer = self.context.append_basic_block(lrr_fn, "leaf_xfer");
        let leaf_no_xfer = self.context.append_basic_block(lrr_fn, "leaf_no_xfer");
        let leaf_ready = self.context.append_basic_block(lrr_fn, "leaf_ready");
        let leaf_shift_loop = self.context.append_basic_block(lrr_fn, "leaf_shift_loop");
        let leaf_shift_body = self.context.append_basic_block(lrr_fn, "leaf_shift_body");
        let leaf_shift_done = self.context.append_basic_block(lrr_fn, "leaf_shift_done");
        let int_scan_loop = self.context.append_basic_block(lrr_fn, "int_scan_loop");
        let int_scan_body = self.context.append_basic_block(lrr_fn, "int_scan_body");
        let int_scan_found = self.context.append_basic_block(lrr_fn, "int_scan_found");
        let int_scan_next = self.context.append_basic_block(lrr_fn, "int_scan_next");
        let int_cow = self.context.append_basic_block(lrr_fn, "int_cow");
        let int_cow_copy = self.context.append_basic_block(lrr_fn, "int_cow_copy");
        let int_xfer = self.context.append_basic_block(lrr_fn, "int_xfer");
        let int_no_xfer = self.context.append_basic_block(lrr_fn, "int_no_xfer");
        let int_inc_loop = self.context.append_basic_block(lrr_fn, "int_inc_loop");
        let int_inc_body = self.context.append_basic_block(lrr_fn, "int_inc_body");
        let int_inc_done = self.context.append_basic_block(lrr_fn, "int_inc_done");
        let int_prep_recurse = self.context.append_basic_block(lrr_fn, "int_prep_recurse");
        let int_update = self.context.append_basic_block(lrr_fn, "int_update");
        let dec_old = self.context.append_basic_block(lrr_fn, "dec_old");
        let store_child = self.context.append_basic_block(lrr_fn, "store_child");
        let int_ret = self.context.append_basic_block(lrr_fn, "int_ret");
        let fail = self.context.append_basic_block(lrr_fn, "fail");

        self.builder.position_at_end(entry);
        let lrr_node = lrr_fn.get_first_param().unwrap().into_pointer_value();
        let lrr_height = lrr_fn.get_nth_param(1).unwrap().into_int_value();
        let lrr_idx = lrr_fn.get_nth_param(2).unwrap().into_int_value();
        let list_root_rc = lrr_fn.get_nth_param(3).unwrap().into_int_value();
        let out_height = lrr_fn.get_nth_param(4).unwrap().into_pointer_value();
        let _ = self.builder.build_store(out_height, lrr_height).map_err(llvm_err)?;
        let is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, lrr_height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_leaf, leaf, int_scan_loop);

        // ---- leaf remove ----
        self.builder.position_at_end(leaf);
        let leaf_i8 = self
            .builder
            .build_pointer_cast(lrr_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let count_raw = self
            .builder
            .build_load(i32, leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let count = self
            .builder
            .build_int_z_extend(count_raw, i64, "count")
            .map_err(llvm_err)?;
        let count_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, count, zero, "count_zero")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(count_zero, fail, leaf_cow);

        self.builder.position_at_end(leaf_cow);
        let leaf_int = self
            .builder
            .build_ptr_to_int(lrr_node, i64, "leaf_int")
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
                &[cow_leaf.into(), lrr_node.into(), leaf_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let leaf_do_xfer = self
            .builder
            .build_int_compare(IntPredicate::SGT, leaf_rc, one, "leaf_do_xfer")
            .map_err(llvm_err)?;
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
            (&lrr_node, leaf_cow),
            (&cow_leaf, leaf_xfer),
            (&cow_leaf, leaf_no_xfer),
        ]);
        let work_leaf = work_leaf_phi.as_basic_value().into_pointer_value();
        let wl_i8 = self
            .builder
            .build_pointer_cast(work_leaf, ptr, "wl_i8")
            .map_err(llvm_err)?;
        let wl_eb = unsafe {
            self.builder
                .build_gep(i8, wl_i8, &[i64.const_int(8, false)], "wl_eb")
                .map_err(llvm_err)?
        };
        let idx = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, lrr_idx, zero, "idx_neg")
                    .map_err(llvm_err)?,
                zero,
                lrr_idx,
                "idx0",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let last = self
            .builder
            .build_int_sub(count, one, "last")
            .map_err(llvm_err)?;
        let idx_c = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SGT, idx, last, "idx_gt")
                    .map_err(llvm_err)?,
                last,
                idx,
                "idx_c",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let rm_ep = unsafe {
            self.builder
                .build_gep(self.string_type, wl_eb, &[idx_c], "rm_ep")
                .map_err(llvm_err)?
        };
        let rm_ev = self
            .builder
            .build_load(self.string_type, rm_ep, "rm_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let _ = self
            .builder
            .build_call(str_rc_dec_fn, &[rm_ev.into()], "")
            .map_err(llvm_err)?;
        let si = self.builder.build_alloca(i64, "si").map_err(llvm_err)?;
        let si_start = self
            .builder
            .build_int_add(idx_c, one, "si_start")
            .map_err(llvm_err)?;
        self.builder.build_store(si, si_start).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(leaf_shift_loop);

        self.builder.position_at_end(leaf_shift_loop);
        let siv = self
            .builder
            .build_load(i64, si, "siv")
            .map_err(llvm_err)?
            .into_int_value();
        let si_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, siv, count, "si_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(si_done, leaf_shift_done, leaf_shift_body);

        self.builder.position_at_end(leaf_shift_body);
        let src = unsafe {
            self.builder
                .build_gep(self.string_type, wl_eb, &[siv], "src")
                .map_err(llvm_err)?
        };
        let sv = self
            .builder
            .build_load(self.string_type, src, "sv")
            .map_err(llvm_err)?;
        let dst_i = self
            .builder
            .build_int_sub(siv, one, "dst_i")
            .map_err(llvm_err)?;
        let dst = unsafe {
            self.builder
                .build_gep(self.string_type, wl_eb, &[dst_i], "dst")
                .map_err(llvm_err)?
        };
        let _ = self.builder.build_store(dst, sv).map_err(llvm_err)?;
        let siv_p1 = self
            .builder
            .build_int_add(siv, one, "siv_p1")
            .map_err(llvm_err)?;
        self.builder.build_store(si, siv_p1).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(leaf_shift_loop);

        self.builder.position_at_end(leaf_shift_done);
        let new_count = self
            .builder
            .build_int_sub(count, one, "new_count")
            .map_err(llvm_err)?;
        let new_count_i32 = self
            .builder
            .build_int_truncate(new_count, i32, "new_count_i32")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(wl_i8, new_count_i32).map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&work_leaf));

        // ---- internal: scan children ----
        self.builder.position_at_end(int_scan_loop);
        let phi_i = self.builder.build_phi(i64, "phi_i").map_err(llvm_err)?;
        let phi_acc = self.builder.build_phi(i64, "phi_acc").map_err(llvm_err)?;
        phi_i.add_incoming(&[(&zero, entry)]);
        phi_acc.add_incoming(&[(&zero, entry)]);
        let scan_i = phi_i.as_basic_value().into_int_value();
        let scan_acc = phi_acc.as_basic_value().into_int_value();
        let int_i8 = self
            .builder
            .build_pointer_cast(lrr_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let int_count_raw = self
            .builder
            .build_load(i32, int_i8, "intl_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let int_count = self
            .builder
            .build_int_z_extend(int_count_raw, i64, "int_count")
            .map_err(llvm_err)?;
        let scan_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, scan_i, int_count, "scan_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(scan_done, fail, int_scan_body);

        self.builder.position_at_end(int_scan_body);
        let ce_base = unsafe {
            self.builder
                .build_gep(i8, int_i8, &[i64.const_int(16, false)], "cb")
                .map_err(llvm_err)?
        };
        let ce_ptr = unsafe {
            self.builder
                .build_gep(self.child_entry_type, ce_base, &[scan_i], "cep")
                .map_err(llvm_err)?
        };
        let ce = self
            .builder
            .build_load(self.child_entry_type, ce_ptr, "ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let st = self
            .builder
            .build_extract_value(ce, 1, "st")
            .map_err(llvm_err)?
            .into_int_value();
        let new_acc = self
            .builder
            .build_int_add(scan_acc, st, "new_acc")
            .map_err(llvm_err)?;
        let idx_in_child = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrr_idx, new_acc, "in_child")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(idx_in_child, int_scan_found, int_scan_next);

        self.builder.position_at_end(int_scan_next);
        let next_i = self
            .builder
            .build_int_add(scan_i, one, "next_i")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_scan_loop);
        phi_i.add_incoming(&[(&next_i, int_scan_next)]);
        phi_acc.add_incoming(&[(&new_acc, int_scan_next)]);

        self.builder.position_at_end(int_scan_found);
        let phi_found_i = self.builder.build_phi(i64, "phi_fi").map_err(llvm_err)?;
        let phi_found_acc = self.builder.build_phi(i64, "phi_fa").map_err(llvm_err)?;
        phi_found_i.add_incoming(&[(&scan_i, int_scan_body)]);
        phi_found_acc.add_incoming(&[(&scan_acc, int_scan_body)]);
        let found_i = phi_found_i.as_basic_value().into_int_value();
        let offset_before = phi_found_acc.as_basic_value().into_int_value();
        let local_idx = self
            .builder
            .build_int_sub(lrr_idx, offset_before, "local_idx")
            .map_err(llvm_err)?;
        let child_h = self
            .builder
            .build_int_sub(lrr_height, one, "child_h")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_cow);

        self.builder.position_at_end(int_cow);
        let int_int = self
            .builder
            .build_ptr_to_int(lrr_node, i64, "int_int")
            .map_err(llvm_err)?;
        let int_rc_a = self
            .builder
            .build_int_sub(int_int, i64.const_int(8, false), "int_rc_a")
            .map_err(llvm_err)?;
        let int_rc_p = self
            .builder
            .build_int_to_ptr(int_rc_a, ptr, "int_rc_p")
            .map_err(llvm_err)?;
        let int_rc = self
            .builder
            .build_load(i64, int_rc_p, "int_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let int_shared = self
            .builder
            .build_or(
                self.builder
                    .build_int_compare(IntPredicate::SGT, int_rc, one, "int_sh_rc")
                    .map_err(llvm_err)?,
                self.builder
                    .build_int_compare(IntPredicate::SGT, list_root_rc, one, "int_sh_root")
                    .map_err(llvm_err)?,
                "int_shared",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(int_shared, int_cow_copy, int_prep_recurse);

        self.builder.position_at_end(int_cow_copy);
        let int_sz = self.internal_type.size_of().ok_or("internal size")?;
        let new_int = self
            .builder
            .build_call(malloc_rc_fn, &[int_sz.into()], "new_int")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[new_int.into(), lrr_node.into(), int_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let int_do_xfer = self
            .builder
            .build_int_compare(IntPredicate::SGT, int_rc, one, "int_do_xfer")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(int_do_xfer, int_xfer, int_no_xfer);
        self.builder.position_at_end(int_xfer);
        let _ = self.builder.build_store(
            int_rc_p,
            self.builder
                .build_int_sub(int_rc, one, "int_rc_dec")
                .map_err(llvm_err)?,
        );
        let _ = self.builder.build_unconditional_branch(int_inc_loop);
        self.builder.position_at_end(int_no_xfer);
        let _ = self.builder.build_unconditional_branch(int_inc_loop);

        self.builder.position_at_end(int_inc_loop);
        let inc_i_phi = self.builder.build_phi(i64, "inc_i").map_err(llvm_err)?;
        inc_i_phi.add_incoming(&[(&zero, int_xfer), (&zero, int_no_xfer)]);
        let inc_i = inc_i_phi.as_basic_value().into_int_value();
        let ni_i8 = self
            .builder
            .build_pointer_cast(new_int, ptr, "ni_i8")
            .map_err(llvm_err)?;
        let ni_count_raw = self
            .builder
            .build_load(i32, ni_i8, "ni_count")
            .map_err(llvm_err)?
            .into_int_value();
        let ni_count = self
            .builder
            .build_int_z_extend(ni_count_raw, i64, "ni_cnt")
            .map_err(llvm_err)?;
        let inc_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, inc_i, ni_count, "inc_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(inc_done, int_inc_done, int_inc_body);

        self.builder.position_at_end(int_inc_body);
        let inc_cb = unsafe {
            self.builder
                .build_gep(i8, ni_i8, &[i64.const_int(16, false)], "inc_cb")
                .map_err(llvm_err)?
        };
        let inc_cep = unsafe {
            self.builder
                .build_gep(self.child_entry_type, inc_cb, &[inc_i], "inc_cep")
                .map_err(llvm_err)?
        };
        let inc_child = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, inc_cep, "inc_ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                0,
                "inc_ch",
            )
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(rc_inc_fn, &[inc_child.into()], "")
            .map_err(llvm_err)?;
        let inc_next = self
            .builder
            .build_int_add(inc_i, one, "inc_next")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_inc_loop);
        inc_i_phi.add_incoming(&[(&inc_next, int_inc_body)]);

        self.builder.position_at_end(int_inc_done);
        let _ = self.builder.build_unconditional_branch(int_prep_recurse);

        self.builder.position_at_end(int_prep_recurse);
        let work_phi = self.builder.build_phi(ptr, "work_phi").map_err(llvm_err)?;
        work_phi.add_incoming(&[(&lrr_node, int_cow), (&new_int, int_inc_done)]);
        let work_node = work_phi.as_basic_value().into_pointer_value();
        let prep_i8 = self
            .builder
            .build_pointer_cast(work_node, ptr, "prep_i8")
            .map_err(llvm_err)?;
        let prep_ce_base = unsafe {
            self.builder
                .build_gep(i8, prep_i8, &[i64.const_int(16, false)], "prep_ceb")
                .map_err(llvm_err)?
        };
        let prep_ce_ptr = unsafe {
            self.builder
                .build_gep(self.child_entry_type, prep_ce_base, &[found_i], "prep_cep")
                .map_err(llvm_err)?
        };
        let work_child = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, prep_ce_ptr, "prep_ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                0,
                "work_child",
            )
            .map_err(llvm_err)?
            .into_pointer_value();
        let child_height_out = self
            .builder
            .build_alloca(i64, "child_height_out")
            .map_err(llvm_err)?;
        self.builder
            .build_store(child_height_out, child_h)
            .map_err(llvm_err)?;
        let new_child = self
            .builder
            .build_call(
                lrr_fn,
                &[
                    work_child.into(),
                    child_h.into(),
                    local_idx.into(),
                    list_root_rc.into(),
                    child_height_out.into(),
                ],
                "new_child",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let child_failed = self
            .builder
            .build_int_compare(IntPredicate::EQ, new_child, null_ptr, "child_fail")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(child_failed, fail, int_update);

        self.builder.position_at_end(int_update);
        let child_changed = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                self.builder
                    .build_ptr_to_int(new_child, i64, "nc_i")
                    .map_err(llvm_err)?,
                self.builder
                    .build_ptr_to_int(work_child, i64, "wc_i")
                    .map_err(llvm_err)?,
                "child_changed",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(child_changed, dec_old, store_child);

        self.builder.position_at_end(dec_old);
        let old_child_rc_a = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(work_child, i64, "wc_int")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "oc_rc_a",
            )
            .map_err(llvm_err)?;
        let old_child_rc_p = self
            .builder
            .build_int_to_ptr(old_child_rc_a, ptr, "oc_rc_p")
            .map_err(llvm_err)?;
        let old_child_rc = self
            .builder
            .build_load(i64, old_child_rc_p, "oc_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_store(
            old_child_rc_p,
            self.builder
                .build_int_sub(old_child_rc, one, "oc_rc_dec")
                .map_err(llvm_err)?,
        );
        let _ = self.builder.build_unconditional_branch(store_child);

        self.builder.position_at_end(store_child);
        let inc_child_bb = self.context.append_basic_block(lrr_fn, "inc_child");
        let after_inc_bb = self.context.append_basic_block(lrr_fn, "after_inc");
        let _ = self
            .builder
            .build_conditional_branch(child_changed, inc_child_bb, after_inc_bb);
        self.builder.position_at_end(inc_child_bb);
        let _ = self
            .builder
            .build_call(rc_inc_fn, &[new_child.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(after_inc_bb);
        self.builder.position_at_end(after_inc_bb);

        let old_st = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, prep_ce_ptr, "old_ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                1,
                "old_st",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let new_st = self
            .builder
            .build_int_sub(old_st, one, "new_st")
            .map_err(llvm_err)?;
        let new_entry = self
            .builder
            .build_insert_value(self.child_entry_type.get_undef(), new_child, 0, "ne0")
            .map_err(llvm_err)?;
        let new_entry2 = self
            .builder
            .build_insert_value(new_entry, new_st, 1, "ne1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(prep_ce_ptr, new_entry2)
            .map_err(llvm_err)?;
        let int_total_p = unsafe {
            self.builder
                .build_gep(i64, prep_i8, &[one], "int_total_p")
                .map_err(llvm_err)?
        };
        let old_total = self
            .builder
            .build_load(i64, int_total_p, "old_total")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_store(
                int_total_p,
                self.builder
                    .build_int_sub(old_total, one, "new_total")
                    .map_err(llvm_err)?,
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_ret);

        self.builder.position_at_end(int_ret);
        let _ = self.builder.build_return(Some(&work_node));

        self.builder.position_at_end(fail);
        let _ = self.builder.build_return(Some(&null_ptr));

        Ok(())
    }
}
