// Fused filter+fold: single B-tree walk (filter survivors, fold into acc).

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(in crate::runtime_decl) fn define_list_filter_fold(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let void = self.context.void_type();
        let zero = i64.const_int(0, false);
        let lambda_fn_ty = self.string_type.fn_type(&[i64.into()], false);
        let fold_fn_ty = i64.fn_type(&[i64.into(), i64.into()], false);

        let rec_fn = self.module.add_function(
            "action_list_filter_fold_walk_rec",
            void.fn_type(
                &[ptr.into(), i64.into(), ptr.into(), ptr.into(), ptr.into()],
                false,
            ),
            None,
        );
        let entry = self.context.append_basic_block(rec_fn, "entry");
        let leaf_hdr = self.context.append_basic_block(rec_fn, "leaf_hdr");
        let leaf_bdy = self.context.append_basic_block(rec_fn, "leaf_bdy");
        let leaf_chk = self.context.append_basic_block(rec_fn, "leaf_chk");
        let leaf_fold = self.context.append_basic_block(rec_fn, "leaf_fold");
        let leaf_next = self.context.append_basic_block(rec_fn, "leaf_next");
        let leaf_done = self.context.append_basic_block(rec_fn, "leaf_done");
        let int_hdr = self.context.append_basic_block(rec_fn, "int_hdr");
        let int_bdy = self.context.append_basic_block(rec_fn, "int_bdy");
        let int_child = self.context.append_basic_block(rec_fn, "int_child");
        let int_next = self.context.append_basic_block(rec_fn, "int_next");
        let concat_bb = self.context.append_basic_block(rec_fn, "concat");
        let normal_bb = self.context.append_basic_block(rec_fn, "normal");

        self.builder.position_at_end(entry);
        let node = rec_fn.get_first_param().unwrap().into_pointer_value();
        let height = rec_fn.get_nth_param(1).unwrap().into_int_value();
        let filter_fn = rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let fold_fn = rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let acc_p = rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let neg1 = i64.const_int(-1i64 as u64, true);
        let is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, height, neg1, "is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_concat, concat_bb, normal_bb);

        // ConcatNode: walk left then right
        self.builder.position_at_end(concat_bb);
        let ln_p = unsafe {
            self.builder
                .build_gep(ptr, node, &[i64.const_int(2, false)], "ln_p")
                .map_err(llvm_err)?
        };
        let left_node = self
            .builder
            .build_load(ptr, ln_p, "ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lh_p = unsafe {
            self.builder
                .build_gep(i64, node, &[i64.const_int(4, false)], "lh_p")
                .map_err(llvm_err)?
        };
        let left_h = self
            .builder
            .build_load(i64, lh_p, "lh")
            .map_err(llvm_err)?
            .into_int_value();
        let rn_p = unsafe {
            self.builder
                .build_gep(ptr, node, &[i64.const_int(5, false)], "rn_p")
                .map_err(llvm_err)?
        };
        let right_node = self
            .builder
            .build_load(ptr, rn_p, "rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rh_p = unsafe {
            self.builder
                .build_gep(i64, node, &[i64.const_int(7, false)], "rh_p")
                .map_err(llvm_err)?
        };
        let right_h = self
            .builder
            .build_load(i64, rh_p, "rh")
            .map_err(llvm_err)?
            .into_int_value();
        let rec_args = |n: inkwell::values::PointerValue<'ctx>,
                        h: inkwell::values::IntValue<'ctx>|
         -> Vec<inkwell::values::BasicMetadataValueEnum<'ctx>> {
            vec![
                n.into(),
                h.into(),
                filter_fn.into(),
                fold_fn.into(),
                acc_p.into(),
            ]
        };
        let _ = self
            .builder
            .build_call(rec_fn, &rec_args(left_node, left_h), "")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(rec_fn, &rec_args(right_node, right_h), "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(normal_bb);
        let is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_leaf, leaf_hdr, int_hdr);

        // ---- leaf walk ----
        self.builder.position_at_end(leaf_hdr);
        let leaf_i8 = self
            .builder
            .build_pointer_cast(node, ptr, "leaf_i8")
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
        let _ = self.builder.build_unconditional_branch(leaf_bdy);

        self.builder.position_at_end(leaf_bdy);
        let i_phi = self.builder.build_phi(i64, "i").map_err(llvm_err)?;
        let done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                i_phi.as_basic_value().into_int_value(),
                count,
                "done_leaf",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_leaf, leaf_done, leaf_chk);

        self.builder.position_at_end(leaf_chk);
        let eb = unsafe {
            self.builder
                .build_gep(i8, leaf_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)?
        };
        let ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    eb,
                    &[i_phi.as_basic_value().into_int_value()],
                    "ep",
                )
                .map_err(llvm_err)?
        };
        let elem = self
            .builder
            .build_load(self.string_type, ep, "elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let elem_tag = self
            .builder
            .build_extract_value(elem, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let pred_call = self
            .builder
            .build_indirect_call(lambda_fn_ty, filter_fn, &[elem_tag.into()], "pred")
            .map_err(llvm_err)?;
        let pred_bv = pred_call
            .try_as_basic_value()
            .basic()
            .ok_or("filter_fold_walk filter failed")?;
        let pred_val = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred_val, zero, "is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_true, leaf_fold, leaf_next);

        self.builder.position_at_end(leaf_fold);
        let cur_acc = self
            .builder
            .build_load(i64, acc_p, "acc")
            .map_err(llvm_err)?
            .into_int_value();
        let new_acc = self
            .builder
            .build_indirect_call(
                fold_fn_ty,
                fold_fn,
                &[cur_acc.into(), elem_tag.into()],
                "folded",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("filter_fold_walk fold failed")?
            .into_int_value();
        self.builder.build_store(acc_p, new_acc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(leaf_next);

        self.builder.position_at_end(leaf_next);
        let next_i = self
            .builder
            .build_int_add(
                i_phi.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ni",
            )
            .map_err(llvm_err)?;
        let leaf_next_bb = self.builder.get_insert_block().unwrap();
        i_phi.add_incoming(&[(&zero, leaf_hdr), (&next_i, leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(leaf_bdy);

        self.builder.position_at_end(leaf_done);
        let _ = self.builder.build_return(None);

        // ---- internal walk ----
        self.builder.position_at_end(int_hdr);
        let int_i8 = self
            .builder
            .build_pointer_cast(node, ptr, "int_i8")
            .map_err(llvm_err)?;
        let child_count_raw = self
            .builder
            .build_load(i32, int_i8, "cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let child_count = self
            .builder
            .build_int_z_extend(child_count_raw, i64, "cc")
            .map_err(llvm_err)?;
        let child_h = self
            .builder
            .build_int_sub(height, i64.const_int(1, false), "ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_bdy);

        self.builder.position_at_end(int_bdy);
        let ci = self.builder.build_phi(i64, "ci").map_err(llvm_err)?;
        let done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                ci.as_basic_value().into_int_value(),
                child_count,
                "done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_int, leaf_done, int_child);

        self.builder.position_at_end(int_child);
        let children_base = unsafe {
            self.builder
                .build_gep(i8, int_i8, &[i64.const_int(16, false)], "cb")
                .map_err(llvm_err)?
        };
        let child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[ci.as_basic_value().into_int_value()],
                    "cep",
                )
                .map_err(llvm_err)?
        };
        let child_entry = self
            .builder
            .build_load(self.child_entry_type, child_ep, "ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let child_ptr = self
            .builder
            .build_extract_value(child_entry, 0, "cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    child_ptr.into(),
                    child_h.into(),
                    filter_fn.into(),
                    fold_fn.into(),
                    acc_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_next);

        self.builder.position_at_end(int_next);
        let next_ci = self
            .builder
            .build_int_add(
                ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "nci",
            )
            .map_err(llvm_err)?;
        let int_next_bb = self.builder.get_insert_block().unwrap();
        ci.add_incoming(&[(&zero, int_hdr), (&next_ci, int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(int_bdy);

        // ---- outer wrapper ----
        let outer_fn = self.module.add_function(
            "action_list_filter_fold_walk",
            i64.fn_type(
                &[self.list_type.into(), ptr.into(), ptr.into(), i64.into()],
                false,
            ),
            None,
        );
        let o_entry = self.context.append_basic_block(outer_fn, "entry");
        let o_walk = self.context.append_basic_block(outer_fn, "walk");
        self.builder.position_at_end(o_entry);
        let list = outer_fn.get_first_param().unwrap().into_struct_value();
        let o_filter = outer_fn.get_nth_param(1).unwrap().into_pointer_value();
        let o_fold = outer_fn.get_nth_param(2).unwrap().into_pointer_value();
        let o_init = outer_fn.get_nth_param(3).unwrap().into_int_value();
        let o_node = self
            .builder
            .build_extract_value(list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let o_height = self
            .builder
            .build_extract_value(list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let o_acc = self.builder.build_alloca(i64, "acc").map_err(llvm_err)?;
        self.builder.build_store(o_acc, o_init).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(o_walk);
        self.builder.position_at_end(o_walk);
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    o_node.into(),
                    o_height.into(),
                    o_filter.into(),
                    o_fold.into(),
                    o_acc.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let res = self
            .builder
            .build_load(i64, o_acc, "res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&res));

        let _void: inkwell::types::VoidType<'ctx> = void;
        Ok(())
    }
}
