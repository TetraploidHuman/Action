// insert_rec overflow: split a full internal node (64 children) and retry insert on the new root.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_insert_split_intl(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let half = i64.const_int(32, false);
        let fanout = i64.const_int(64, false);
        let null_ptr = ptr.const_null();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let insert_rec_fn = self
            .module
            .get_function("action_list_insert_rec")
            .unwrap();

        let split_fn = self
            .module
            .get_function("action_list_insert_rec_split_intl")
            .unwrap();

        let entry = self.context.append_basic_block(split_fn, "entry");
        let fail = self.context.append_basic_block(split_fn, "fail");
        let sum_left_loop = self.context.append_basic_block(split_fn, "sum_left_loop");
        let sum_left_body = self.context.append_basic_block(split_fn, "sum_left_body");
        let sum_left_done = self.context.append_basic_block(split_fn, "sum_left_done");
        let sum_right_loop = self.context.append_basic_block(split_fn, "sum_right_loop");
        let sum_right_body = self.context.append_basic_block(split_fn, "sum_right_body");
        let sum_right_done = self.context.append_basic_block(split_fn, "sum_right_done");
        let copy_right_loop = self.context.append_basic_block(split_fn, "copy_right_loop");
        let copy_right_body = self.context.append_basic_block(split_fn, "copy_right_body");
        let copy_right_done = self.context.append_basic_block(split_fn, "copy_right_done");
        let retry = self.context.append_basic_block(split_fn, "retry");

        self.builder.position_at_end(entry);
        let intl_node = split_fn.get_first_param().unwrap().into_pointer_value();
        let intl_height = split_fn.get_nth_param(1).unwrap().into_int_value();
        let ins_idx = split_fn.get_nth_param(2).unwrap().into_int_value();
        let ins_val = split_fn.get_nth_param(3).unwrap().into_struct_value();
        let list_root_rc = split_fn.get_nth_param(4).unwrap().into_int_value();

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
            .build_conditional_branch(int_full, sum_left_loop, fail);

        // Sum subtree totals for left [0..32)
        self.builder.position_at_end(sum_left_loop);
        let sl_i = self.builder.build_phi(i64, "sl_i").map_err(llvm_err)?;
        let sl_acc = self.builder.build_phi(i64, "sl_acc").map_err(llvm_err)?;
        let sl_cur = sl_i.as_basic_value().into_int_value();
        let sl_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, sl_cur, half, "sl_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sl_done, sum_left_done, sum_left_body);

        self.builder.position_at_end(sum_left_body);
        let sl_cb = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "sl_cb")
                .map_err(llvm_err)?
        };
        let sl_ce = unsafe {
            self.builder
                .build_gep(self.child_entry_type, sl_cb, &[sl_cur], "sl_ce")
                .map_err(llvm_err)?
        };
        let sl_st = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, sl_ce, "sl_ce_v")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                1,
                "sl_st",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let sl_acc_v = sl_acc.as_basic_value().into_int_value();
        let sl_next_acc = self
            .builder
            .build_int_add(sl_acc_v, sl_st, "sl_next_acc")
            .map_err(llvm_err)?;
        let sl_next_i = self
            .builder
            .build_int_add(sl_cur, one, "sl_next_i")
            .map_err(llvm_err)?;
        let sl_body_bb = self.builder.get_insert_block().unwrap();
        sl_i.add_incoming(&[(&zero, entry), (&sl_next_i, sl_body_bb)]);
        sl_acc.add_incoming(&[(&zero, entry), (&sl_next_acc, sl_body_bb)]);
        let _ = self.builder.build_unconditional_branch(sum_left_loop);

        self.builder.position_at_end(sum_left_done);
        let left_total = sl_acc.as_basic_value().into_int_value();
        let _ = self.builder.build_unconditional_branch(sum_right_loop);

        // Sum subtree totals for right [32..64)
        self.builder.position_at_end(sum_right_loop);
        let sr_i = self.builder.build_phi(i64, "sr_i").map_err(llvm_err)?;
        let sr_acc = self.builder.build_phi(i64, "sr_acc").map_err(llvm_err)?;
        let sr_cur = sr_i.as_basic_value().into_int_value();
        let sr_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, sr_cur, fanout, "sr_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sr_done, sum_right_done, sum_right_body);

        self.builder.position_at_end(sum_right_body);
        let sr_cb = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "sr_cb")
                .map_err(llvm_err)?
        };
        let sr_ce = unsafe {
            self.builder
                .build_gep(self.child_entry_type, sr_cb, &[sr_cur], "sr_ce")
                .map_err(llvm_err)?
        };
        let sr_st = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, sr_ce, "sr_ce_v")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                1,
                "sr_st",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let sr_acc_v = sr_acc.as_basic_value().into_int_value();
        let sr_next_acc = self
            .builder
            .build_int_add(sr_acc_v, sr_st, "sr_next_acc")
            .map_err(llvm_err)?;
        let sr_next_i = self
            .builder
            .build_int_add(sr_cur, one, "sr_next_i")
            .map_err(llvm_err)?;
        let sr_body_bb = self.builder.get_insert_block().unwrap();
        sr_i.add_incoming(&[(&half, sum_left_done), (&sr_next_i, sr_body_bb)]);
        sr_acc.add_incoming(&[(&zero, sum_left_done), (&sr_next_acc, sr_body_bb)]);
        let _ = self.builder.build_unconditional_branch(sum_right_loop);

        self.builder.position_at_end(sum_right_done);
        let right_total = sr_acc.as_basic_value().into_int_value();

        // Truncate left internal to 32 children
        let _ = self
            .builder
            .build_store(
                intl_i8,
                self.builder
                    .build_int_truncate(half, i32, "half32")
                    .map_err(llvm_err)?,
            )
            .map_err(llvm_err)?;
        let left_total_p = unsafe {
            self.builder
                .build_gep(i64, intl_i8, &[one], "left_total_p")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(left_total_p, left_total)
            .map_err(llvm_err)?;

        // Allocate right internal and copy children [32..64)
        let int_sz = self.internal_type.size_of().ok_or("internal size")?;
        let right_intl = self
            .builder
            .build_call(malloc_rc_fn, &[int_sz.into()], "right_intl")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let right_i8 = self
            .builder
            .build_pointer_cast(right_intl, ptr, "right_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(
                right_i8,
                self.builder
                    .build_int_truncate(half, i32, "rh32")
                    .map_err(llvm_err)?,
            )
            .map_err(llvm_err)?;
        let right_total_p = unsafe {
            self.builder
                .build_gep(i64, right_i8, &[one], "right_total_p")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(right_total_p, right_total)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(copy_right_loop);

        self.builder.position_at_end(copy_right_loop);
        let cr_i = self.builder.build_phi(i64, "cr_i").map_err(llvm_err)?;
        let cr_cur = cr_i.as_basic_value().into_int_value();
        let cr_off = self
            .builder
            .build_int_add(half, cr_cur, "cr_src_i")
            .map_err(llvm_err)?;
        let cr_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, cr_cur, half, "cr_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cr_done, copy_right_done, copy_right_body);

        self.builder.position_at_end(copy_right_body);
        let src_cb = unsafe {
            self.builder
                .build_gep(i8, intl_i8, &[i64.const_int(16, false)], "src_cb")
                .map_err(llvm_err)?
        };
        let src_ce = unsafe {
            self.builder
                .build_gep(self.child_entry_type, src_cb, &[cr_off], "src_ce")
                .map_err(llvm_err)?
        };
        let src_entry = self
            .builder
            .build_load(self.child_entry_type, src_ce, "src_entry")
            .map_err(llvm_err)?
            .into_struct_value();
        let src_child = self
            .builder
            .build_extract_value(src_entry, 0, "src_child")
            .map_err(llvm_err)?
            .into_pointer_value();
        let dst_cb = unsafe {
            self.builder
                .build_gep(i8, right_i8, &[i64.const_int(16, false)], "dst_cb")
                .map_err(llvm_err)?
        };
        let dst_ce = unsafe {
            self.builder
                .build_gep(self.child_entry_type, dst_cb, &[cr_cur], "dst_ce")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(dst_ce, src_entry)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(rc_inc_fn, &[src_child.into()], "")
            .map_err(llvm_err)?;
        let cr_next = self
            .builder
            .build_int_add(cr_cur, one, "cr_next")
            .map_err(llvm_err)?;
        let cr_body_bb = self.builder.get_insert_block().unwrap();
        cr_i.add_incoming(&[(&zero, sum_right_done), (&cr_next, cr_body_bb)]);
        let _ = self.builder.build_unconditional_branch(copy_right_loop);

        self.builder.position_at_end(copy_right_done);
        let _ = self
            .builder
            .build_call(rc_inc_fn, &[right_intl.into()], "")
            .map_err(llvm_err)?;

        // New root one level taller: {left=intl_node, right=right_intl}
        let new_root = self
            .builder
            .build_call(malloc_rc_fn, &[int_sz.into()], "new_root")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let nr_i8 = self
            .builder
            .build_pointer_cast(new_root, ptr, "nr_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(
                nr_i8,
                i32.const_int(2, false),
            )
            .map_err(llvm_err)?;
        let grand_total = self
            .builder
            .build_int_add(left_total, right_total, "grand_total")
            .map_err(llvm_err)?;
        let nr_total_p = unsafe {
            self.builder
                .build_gep(i64, nr_i8, &[one], "nr_total_p")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(nr_total_p, grand_total)
            .map_err(llvm_err)?;
        let nr_cb = unsafe {
            self.builder
                .build_gep(i8, nr_i8, &[i64.const_int(16, false)], "nr_cb")
                .map_err(llvm_err)?
        };
        let nr_c0 = unsafe {
            self.builder
                .build_gep(self.child_entry_type, nr_cb, &[zero], "nr_c0")
                .map_err(llvm_err)?
        };
        let nr_c0_p = self
            .builder
            .build_pointer_cast(nr_c0, ptr, "nr_c0_p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nr_c0_p, intl_node)
            .map_err(llvm_err)?;
        let nr_c0_st = unsafe {
            self.builder
                .build_gep(i64, nr_c0_p, &[one], "nr_c0_st")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(nr_c0_st, left_total)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(rc_inc_fn, &[intl_node.into()], "")
            .map_err(llvm_err)?;
        let nr_c1 = unsafe {
            self.builder
                .build_gep(self.child_entry_type, nr_cb, &[one], "nr_c1")
                .map_err(llvm_err)?
        };
        let nr_c1_p = self
            .builder
            .build_pointer_cast(nr_c1, ptr, "nr_c1_p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(nr_c1_p, right_intl)
            .map_err(llvm_err)?;
        let nr_c1_st = unsafe {
            self.builder
                .build_gep(i64, nr_c1_p, &[one], "nr_c1_st")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(nr_c1_st, right_total)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(retry);

        self.builder.position_at_end(retry);
        let new_height = self
            .builder
            .build_int_add(intl_height, one, "new_height")
            .map_err(llvm_err)?;
        let retry_root = self
            .builder
            .build_call(
                insert_rec_fn,
                &[
                    new_root.into(),
                    new_height.into(),
                    ins_idx.into(),
                    ins_val.into(),
                    list_root_rc.into(),
                ],
                "retry_root",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self.builder.build_return(Some(&retry_root));

        self.builder.position_at_end(fail);
        let _ = self.builder.build_return(Some(&null_ptr));

        Ok(())
    }
}
