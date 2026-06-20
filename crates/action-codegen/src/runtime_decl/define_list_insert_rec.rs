// B-tree path-copy insert (CoW when rc > 1 or list root is shared). Returns null if leaf full.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_insert_rec(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let null_ptr = ptr.const_null();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        let rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();

        let lir_fn = self.module.add_function(
            "action_list_insert_rec",
            ptr.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    i64.into(),
                    self.string_type.into(),
                    i64.into(),
                ],
                false,
            ),
            None,
        );

        let entry = self.context.append_basic_block(lir_fn, "entry");
        let leaf = self.context.append_basic_block(lir_fn, "leaf");
        let leaf_full = self.context.append_basic_block(lir_fn, "leaf_full");
        let leaf_cow = self.context.append_basic_block(lir_fn, "leaf_cow");
        let leaf_cow_copy = self.context.append_basic_block(lir_fn, "leaf_cow_copy");
        let leaf_xfer = self.context.append_basic_block(lir_fn, "leaf_xfer");
        let leaf_no_xfer = self.context.append_basic_block(lir_fn, "leaf_no_xfer");
        let leaf_ready = self.context.append_basic_block(lir_fn, "leaf_ready");
        let leaf_shift_loop = self.context.append_basic_block(lir_fn, "leaf_shift_loop");
        let leaf_shift_body = self.context.append_basic_block(lir_fn, "leaf_shift_body");
        let leaf_shift_done = self.context.append_basic_block(lir_fn, "leaf_shift_done");
        let int_scan_loop = self.context.append_basic_block(lir_fn, "int_scan_loop");
        let int_scan_body = self.context.append_basic_block(lir_fn, "int_scan_body");
        let int_scan_found = self.context.append_basic_block(lir_fn, "int_scan_found");
        let int_scan_next = self.context.append_basic_block(lir_fn, "int_scan_next");
        let int_cow = self.context.append_basic_block(lir_fn, "int_cow");
        let int_cow_copy = self.context.append_basic_block(lir_fn, "int_cow_copy");
        let int_xfer = self.context.append_basic_block(lir_fn, "int_xfer");
        let int_no_xfer = self.context.append_basic_block(lir_fn, "int_no_xfer");
        let int_inc_loop = self.context.append_basic_block(lir_fn, "int_inc_loop");
        let int_inc_body = self.context.append_basic_block(lir_fn, "int_inc_body");
        let int_inc_done = self.context.append_basic_block(lir_fn, "int_inc_done");
        let int_prep_recurse = self.context.append_basic_block(lir_fn, "int_prep_recurse");
        let int_update = self.context.append_basic_block(lir_fn, "int_update");
        let int_ret = self.context.append_basic_block(lir_fn, "int_ret");
        let dec_old = self.context.append_basic_block(lir_fn, "dec_old");
        let store_child = self.context.append_basic_block(lir_fn, "store_child");
        let fail_ret = self.context.append_basic_block(lir_fn, "fail_ret");

        self.builder.position_at_end(entry);
        let lir_node = lir_fn.get_first_param().unwrap().into_pointer_value();
        let lir_height = lir_fn.get_nth_param(1).unwrap().into_int_value();
        let lir_idx = lir_fn.get_nth_param(2).unwrap().into_int_value();
        let lir_val = lir_fn.get_nth_param(3).unwrap().into_struct_value();
        let list_root_rc = lir_fn.get_nth_param(4).unwrap().into_int_value();
        let is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, lir_height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_leaf, leaf, int_scan_loop);

        // ---- leaf: CoW, shift, insert ----
        self.builder.position_at_end(leaf);
        let leaf_i8 = self
            .builder
            .build_pointer_cast(lir_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let count_raw = self
            .builder
            .build_load(self.context.i32_type(), leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let count = self
            .builder
            .build_int_z_extend(count_raw, i64, "count")
            .map_err(llvm_err)?;
        let is_full = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                count,
                i64.const_int(64, false),
                "is_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_full, leaf_full, leaf_cow);

        self.builder.position_at_end(leaf_full);
        let _ = self.builder.build_return(Some(&null_ptr));

        self.builder.position_at_end(leaf_cow);
        let leaf_int = self
            .builder
            .build_ptr_to_int(lir_node, i64, "leaf_int")
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
        let new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "new_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[new_leaf.into(), lir_node.into(), leaf_sz.into()],
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
        let new_leaf_rc = self
            .builder
            .build_int_sub(leaf_rc, one, "new_leaf_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(leaf_rc_p, new_leaf_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(leaf_ready);
        self.builder.position_at_end(leaf_no_xfer);
        let _ = self.builder.build_unconditional_branch(leaf_ready);

        self.builder.position_at_end(leaf_ready);
        let leaf_phi = self.builder.build_phi(ptr, "leaf_phi").map_err(llvm_err)?;
        leaf_phi.add_incoming(&[
            (&lir_node, leaf_cow),
            (&new_leaf, leaf_xfer),
            (&new_leaf, leaf_no_xfer),
        ]);
        let work_leaf = leaf_phi.as_basic_value().into_pointer_value();
        let work_i8 = self
            .builder
            .build_pointer_cast(work_leaf, ptr, "work_i8")
            .map_err(llvm_err)?;
        let eb = unsafe {
            self.builder
                .build_gep(i8, work_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)
        }?;
        let idx_clamped = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, lir_idx, zero, "idx_neg")
                    .map_err(llvm_err)?,
                zero,
                lir_idx,
                "idx0",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let idx = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SGT, idx_clamped, count, "idx_gt")
                    .map_err(llvm_err)?,
                count,
                idx_clamped,
                "idx",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let si = self.builder.build_alloca(i64, "si").map_err(llvm_err)?;
        let count_m1 = self
            .builder
            .build_int_sub(count, one, "cm1")
            .map_err(llvm_err)?;
        self.builder.build_store(si, count_m1).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(leaf_shift_loop);

        self.builder.position_at_end(leaf_shift_loop);
        let siv = self
            .builder
            .build_load(i64, si, "siv")
            .map_err(llvm_err)?
            .into_int_value();
        let si_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, siv, idx, "si_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(si_cond, leaf_shift_body, leaf_shift_done);

        self.builder.position_at_end(leaf_shift_body);
        let src = unsafe {
            self.builder
                .build_gep(self.string_type, eb, &[siv], "src")
                .map_err(llvm_err)
        }?;
        let sv = self
            .builder
            .build_load(self.string_type, src, "sv")
            .map_err(llvm_err)?;
        let siv_p1 = self
            .builder
            .build_int_add(siv, one, "siv_p1")
            .map_err(llvm_err)?;
        let dst = unsafe {
            self.builder
                .build_gep(self.string_type, eb, &[siv_p1], "dst")
                .map_err(llvm_err)
        }?;
        self.builder.build_store(dst, sv).map_err(llvm_err)?;
        let siv_m1 = self
            .builder
            .build_int_sub(siv, one, "siv_m1")
            .map_err(llvm_err)?;
        self.builder.build_store(si, siv_m1).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(leaf_shift_loop);

        self.builder.position_at_end(leaf_shift_done);
        let ins_dst = unsafe {
            self.builder
                .build_gep(self.string_type, eb, &[idx], "ins_dst")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(ins_dst, lir_val)
            .map_err(llvm_err)?;
        let new_count = self
            .builder
            .build_int_add(count, one, "new_count")
            .map_err(llvm_err)?;
        let new_count_i32 = self
            .builder
            .build_int_truncate(new_count, self.context.i32_type(), "new_count_i32")
            .map_err(llvm_err)?;
        self.builder
            .build_store(work_i8, new_count_i32)
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&work_leaf));

        // ---- internal: scan, path-copy, recurse, update ----
        self.builder.position_at_end(int_scan_loop);
        let phi_i = self.builder.build_phi(i64, "phi_i").map_err(llvm_err)?;
        let phi_acc = self.builder.build_phi(i64, "phi_acc").map_err(llvm_err)?;
        phi_i.add_incoming(&[(&zero, entry)]);
        phi_acc.add_incoming(&[(&zero, entry)]);
        let scan_i = phi_i.as_basic_value().into_int_value();
        let scan_acc = phi_acc.as_basic_value().into_int_value();
        let int_i8 = self
            .builder
            .build_pointer_cast(lir_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let int_count_raw = self
            .builder
            .build_load(self.context.i32_type(), int_i8, "intl_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let int_count = self
            .builder
            .build_int_z_extend(int_count_raw, i64, "intl_count")
            .map_err(llvm_err)?;
        let done_scan = self
            .builder
            .build_int_compare(IntPredicate::SGE, scan_i, int_count, "done_scan")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_scan, int_scan_found, int_scan_body);

        self.builder.position_at_end(int_scan_body);
        let children_base = unsafe {
            self.builder
                .build_gep(i8, int_i8, &[i64.const_int(16, false)], "scb")
                .map_err(llvm_err)
        }?;
        let child_ep = unsafe {
            self.builder
                .build_gep(self.child_entry_type, children_base, &[scan_i], "cep")
                .map_err(llvm_err)
        }?;
        let child_total = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, child_ep, "ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                1,
                "ct",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let new_acc = self
            .builder
            .build_int_add(scan_acc, child_total, "new_acc")
            .map_err(llvm_err)?;
        let found_child = self
            .builder
            .build_int_compare(IntPredicate::SLT, lir_idx, new_acc, "found_child")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(found_child, int_scan_found, int_scan_next);

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
        phi_found_i.add_incoming(&[(&scan_i, int_scan_body), (&scan_i, int_scan_loop)]);
        phi_found_acc.add_incoming(&[(&scan_acc, int_scan_body), (&scan_acc, int_scan_loop)]);
        let found_i = phi_found_i.as_basic_value().into_int_value();
        let offset_before = phi_found_acc.as_basic_value().into_int_value();
        let local_idx = self
            .builder
            .build_int_sub(lir_idx, offset_before, "local_idx")
            .map_err(llvm_err)?;
        let child_h = self
            .builder
            .build_int_sub(lir_height, one, "child_h")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_cow);

        self.builder.position_at_end(int_cow);
        let int_int = self
            .builder
            .build_ptr_to_int(lir_node, i64, "int_int")
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
                &[new_int.into(), lir_node.into(), int_sz.into()],
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
        let new_int_rc = self
            .builder
            .build_int_sub(int_rc, one, "new_int_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(int_rc_p, new_int_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_inc_loop);
        self.builder.position_at_end(int_no_xfer);
        let _ = self.builder.build_unconditional_branch(int_inc_loop);

        // memcpy duplicates child pointers — bump each child's RC in the copy.
        self.builder.position_at_end(int_inc_loop);
        let inc_i_phi = self.builder.build_phi(i64, "inc_i").map_err(llvm_err)?;
        inc_i_phi.add_incoming(&[(&zero, int_xfer), (&zero, int_no_xfer)]);
        let inc_i = inc_i_phi.as_basic_value().into_int_value();
        let new_int_i8 = self
            .builder
            .build_pointer_cast(new_int, ptr, "ni_i8")
            .map_err(llvm_err)?;
        let new_int_count_raw = self
            .builder
            .build_load(self.context.i32_type(), new_int_i8, "ni_count")
            .map_err(llvm_err)?
            .into_int_value();
        let new_int_count = self
            .builder
            .build_int_z_extend(new_int_count_raw, i64, "ni_cnt")
            .map_err(llvm_err)?;
        let inc_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, inc_i, new_int_count, "inc_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(inc_done, int_inc_done, int_inc_body);

        self.builder.position_at_end(int_inc_body);
        let inc_cb = unsafe {
            self.builder
                .build_gep(i8, new_int_i8, &[i64.const_int(16, false)], "inc_cb")
                .map_err(llvm_err)
        }?;
        let inc_cep = unsafe {
            self.builder
                .build_gep(self.child_entry_type, inc_cb, &[inc_i], "inc_cep")
                .map_err(llvm_err)
        }?;
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
        work_phi.add_incoming(&[(&lir_node, int_cow), (&new_int, int_inc_done)]);
        let work_node = work_phi.as_basic_value().into_pointer_value();
        let prep_i8 = self
            .builder
            .build_pointer_cast(work_node, ptr, "prep_i8")
            .map_err(llvm_err)?;
        let prep_ce_base = unsafe {
            self.builder
                .build_gep(i8, prep_i8, &[i64.const_int(16, false)], "prep_ceb")
                .map_err(llvm_err)
        }?;
        let prep_ce_ptr = unsafe {
            self.builder
                .build_gep(self.child_entry_type, prep_ce_base, &[found_i], "prep_cep")
                .map_err(llvm_err)
        }?;
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
        let new_child = self
            .builder
            .build_call(
                lir_fn,
                &[
                    work_child.into(),
                    child_h.into(),
                    local_idx.into(),
                    lir_val.into(),
                    list_root_rc.into(),
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
            .build_conditional_branch(child_failed, fail_ret, int_update);
        self.builder.position_at_end(fail_ret);
        let _ = self.builder.build_return(Some(&null_ptr));

        self.builder.position_at_end(int_update);
        let work_i8 = self
            .builder
            .build_pointer_cast(work_node, ptr, "work_i8")
            .map_err(llvm_err)?;
        let upd_ce_base = unsafe {
            self.builder
                .build_gep(i8, work_i8, &[i64.const_int(16, false)], "upb")
                .map_err(llvm_err)
        }?;
        let upd_ce_ptr = unsafe {
            self.builder
                .build_gep(self.child_entry_type, upd_ce_base, &[found_i], "upcep")
                .map_err(llvm_err)
        }?;
        let child_slot = self
            .builder
            .build_pointer_cast(upd_ce_ptr, ptr, "child_slot")
            .map_err(llvm_err)?;
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
        let old_child_rc_dec = self
            .builder
            .build_int_sub(old_child_rc, one, "oc_rc_dec")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(old_child_rc_p, old_child_rc_dec)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(store_child);

        self.builder.position_at_end(store_child);
        self.builder
            .build_store(child_slot, new_child)
            .map_err(llvm_err)?;
        let inc_child_bb = self.context.append_basic_block(lir_fn, "inc_child");
        let after_inc_bb = self.context.append_basic_block(lir_fn, "after_inc");
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
        let st_ptr = unsafe {
            self.builder
                .build_gep(i64, upd_ce_ptr, &[i64.const_int(1, false)], "st_ptr")
                .map_err(llvm_err)
        }?;
        let st_val = self
            .builder
            .build_load(i64, st_ptr, "st_val")
            .map_err(llvm_err)?
            .into_int_value();
        let st_new = self
            .builder
            .build_int_add(st_val, one, "st_new")
            .map_err(llvm_err)?;
        self.builder.build_store(st_ptr, st_new).map_err(llvm_err)?;
        let total_ptr = unsafe {
            self.builder
                .build_gep(i64, work_i8, &[i64.const_int(1, false)], "total_ptr")
                .map_err(llvm_err)
        }?;
        let total_val = self
            .builder
            .build_load(i64, total_ptr, "total_val")
            .map_err(llvm_err)?
            .into_int_value();
        let total_new = self
            .builder
            .build_int_add(total_val, one, "total_new")
            .map_err(llvm_err)?;
        self.builder
            .build_store(total_ptr, total_new)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_ret);

        self.builder.position_at_end(int_ret);
        let _ = self.builder.build_return(Some(&work_node));

        Ok(())
    }
}
