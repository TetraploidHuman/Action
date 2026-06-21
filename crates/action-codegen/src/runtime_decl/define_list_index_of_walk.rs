// B-tree / Concat in-order indexOf walk (avoids per-index action_list_get).

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_index_of_walk(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let void = self.context.void_type();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let neg1 = i64.const_int(-1i64 as u64, true);
        let neg1_out = i64.const_int(-1i64 as u64, true);

        let rec_fn = self.module.add_function(
            "action_list_index_of_walk_rec",
            void.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    self.string_type.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                ],
                false,
            ),
            None,
        );
        let entry = self.context.append_basic_block(rec_fn, "entry");
        let concat = self.context.append_basic_block(rec_fn, "concat");
        let concat_right = self.context.append_basic_block(rec_fn, "concat_right");
        let normal = self.context.append_basic_block(rec_fn, "normal");
        let leaf_hdr = self.context.append_basic_block(rec_fn, "leaf_hdr");
        let leaf_bdy = self.context.append_basic_block(rec_fn, "leaf_bdy");
        let leaf_chk = self.context.append_basic_block(rec_fn, "leaf_chk");
        let leaf_found = self.context.append_basic_block(rec_fn, "leaf_found");
        let leaf_next = self.context.append_basic_block(rec_fn, "leaf_next");
        let leaf_done = self.context.append_basic_block(rec_fn, "leaf_done");
        let int_hdr = self.context.append_basic_block(rec_fn, "int_hdr");
        let int_bdy = self.context.append_basic_block(rec_fn, "int_bdy");
        let int_child = self.context.append_basic_block(rec_fn, "int_child");
        let int_child_body = self.context.append_basic_block(rec_fn, "int_child_body");
        let int_after = self.context.append_basic_block(rec_fn, "int_after");
        let int_next = self.context.append_basic_block(rec_fn, "int_next");

        self.builder.position_at_end(entry);
        let node = rec_fn.get_first_param().unwrap().into_pointer_value();
        let height = rec_fn.get_nth_param(1).unwrap().into_int_value();
        let tgt = rec_fn.get_nth_param(2).unwrap().into_struct_value();
        let idx_p = rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let res_p = rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let done_p = rec_fn.get_nth_param(5).unwrap().into_pointer_value();

        let is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, height, neg1, "is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_concat, concat, normal);

        self.builder.position_at_end(concat);
        let ln_p = unsafe {
            self.builder
                .build_gep(ptr, node, &[i64.const_int(2, false)], "ln_p")
                .map_err(llvm_err)
        }?;
        let left_node = self
            .builder
            .build_load(ptr, ln_p, "ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lh_p = unsafe {
            self.builder
                .build_gep(i64, node, &[i64.const_int(4, false)], "lh_p")
                .map_err(llvm_err)
        }?;
        let left_h = self
            .builder
            .build_load(i64, lh_p, "lh")
            .map_err(llvm_err)?
            .into_int_value();
        let ll_p = unsafe {
            self.builder
                .build_gep(i64, node, &[i64.const_int(3, false)], "ll_p")
                .map_err(llvm_err)
        }?;
        let left_len = self
            .builder
            .build_load(i64, ll_p, "ll")
            .map_err(llvm_err)?
            .into_int_value();
        let rn_p = unsafe {
            self.builder
                .build_gep(ptr, node, &[i64.const_int(5, false)], "rn_p")
                .map_err(llvm_err)
        }?;
        let right_node = self
            .builder
            .build_load(ptr, rn_p, "rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rh_p = unsafe {
            self.builder
                .build_gep(i64, node, &[i64.const_int(7, false)], "rh_p")
                .map_err(llvm_err)
        }?;
        let right_h = self
            .builder
            .build_load(i64, rh_p, "rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_call(
            rec_fn,
            &[
                left_node.into(),
                left_h.into(),
                tgt.into(),
                idx_p.into(),
                res_p.into(),
                done_p.into(),
            ],
            "",
        ).map_err(llvm_err)?;
        let found = self
            .builder
            .build_load(i64, done_p, "found_l")
            .map_err(llvm_err)?
            .into_int_value();
        let found_set = self
            .builder
            .build_int_compare(IntPredicate::NE, found, zero, "found_set")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(found_set, leaf_done, concat_right);
        self.builder.position_at_end(concat_right);
        let cur_idx = self
            .builder
            .build_load(i64, idx_p, "cur_idx")
            .map_err(llvm_err)?
            .into_int_value();
        let new_idx = self
            .builder
            .build_int_add(cur_idx, left_len, "new_idx")
            .map_err(llvm_err)?;
        self.builder.build_store(idx_p, new_idx).map_err(llvm_err)?;
        let _ = self.builder.build_call(
            rec_fn,
            &[
                right_node.into(),
                right_h.into(),
                tgt.into(),
                idx_p.into(),
                res_p.into(),
                done_p.into(),
            ],
            "",
        ).map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(normal);
        let is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_leaf, leaf_hdr, int_hdr);

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
        let li = self.builder.build_phi(i64, "li").map_err(llvm_err)?;
        let done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                li.as_basic_value().into_int_value(),
                count,
                "done_leaf",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_leaf, leaf_done, leaf_chk);
        self.builder.position_at_end(leaf_chk);
        let base_idx = self
            .builder
            .build_load(i64, idx_p, "base_idx")
            .map_err(llvm_err)?
            .into_int_value();
        let local_i = li.as_basic_value().into_int_value();
        let global_idx = self
            .builder
            .build_int_add(base_idx, local_i, "global_idx")
            .map_err(llvm_err)?;
        let eb = unsafe {
            self.builder
                .build_gep(i8, leaf_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)
        }?;
        let ep = unsafe {
            self.builder
                .build_gep(self.string_type, eb, &[local_i], "ep")
                .map_err(llvm_err)
        }?;
        let elem = self
            .builder
            .build_load(self.string_type, ep, "elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let eq = self
            .call_rt(
                "action_string_eq",
                &[elem.into(), tgt.into()],
            )?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(eq, leaf_found, leaf_next);
        self.builder.position_at_end(leaf_found);
        self.builder.build_store(res_p, global_idx).map_err(llvm_err)?;
        self.builder
            .build_store(done_p, one)
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(leaf_next);
        let next_i = self
            .builder
            .build_int_add(local_i, one, "next_i")
            .map_err(llvm_err)?;
        let leaf_next_bb = self.builder.get_insert_block().unwrap();
        li.add_incoming(&[(&zero, leaf_hdr), (&next_i, leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(leaf_bdy);
        self.builder.position_at_end(leaf_done);
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(int_hdr);
        let int_i8 = self
            .builder
            .build_pointer_cast(node, ptr, "int_i8")
            .map_err(llvm_err)?;
        let cc_raw = self
            .builder
            .build_load(i32, int_i8, "cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let child_count = self
            .builder
            .build_int_z_extend(cc_raw, i64, "cc")
            .map_err(llvm_err)?;
        let child_h = self
            .builder
            .build_int_sub(height, one, "ch")
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
        let found_int = self
            .builder
            .build_load(i64, done_p, "found_int")
            .map_err(llvm_err)?
            .into_int_value();
        let int_already = self
            .builder
            .build_int_compare(IntPredicate::NE, found_int, zero, "int_already")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(int_already, leaf_done, int_child_body);
        self.builder.position_at_end(int_child_body);
        let cb = unsafe {
            self.builder
                .build_gep(i8, int_i8, &[i64.const_int(16, false)], "cb")
                .map_err(llvm_err)
        }?;
        let cep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    cb,
                    &[ci.as_basic_value().into_int_value()],
                    "cep",
                )
                .map_err(llvm_err)
        }?;
        let ce = self
            .builder
            .build_load(self.child_entry_type, cep, "ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let cp = self
            .builder
            .build_extract_value(ce, 0, "cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let st = self
            .builder
            .build_extract_value(ce, 1, "st")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_call(
            rec_fn,
            &[
                cp.into(),
                child_h.into(),
                tgt.into(),
                idx_p.into(),
                res_p.into(),
                done_p.into(),
            ],
            "",
        ).map_err(llvm_err)?;
        let after_child = self
            .builder
            .build_load(i64, done_p, "after_child")
            .map_err(llvm_err)?
            .into_int_value();
        let child_found = self
            .builder
            .build_int_compare(IntPredicate::NE, after_child, zero, "child_found")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(child_found, leaf_done, int_after);
        self.builder.position_at_end(int_after);
        let cur = self
            .builder
            .build_load(i64, idx_p, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let cur_add = self
            .builder
            .build_int_add(cur, st, "cur_add")
            .map_err(llvm_err)?;
        self.builder.build_store(idx_p, cur_add).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(int_next);
        self.builder.position_at_end(int_next);
        let next_ci = self
            .builder
            .build_int_add(ci.as_basic_value().into_int_value(), one, "next_ci")
            .map_err(llvm_err)?;
        let int_next_bb = self.builder.get_insert_block().unwrap();
        ci.add_incoming(&[(&zero, int_hdr), (&next_ci, int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(int_bdy);

        // ---- action_list_index_of_walk(list, target) -> i64 ----
        let walk_fn = self.module.add_function(
            "action_list_index_of_walk",
            i64.fn_type(&[self.list_type.into(), self.string_type.into()], false),
            None,
        );
        let w_entry = self.context.append_basic_block(walk_fn, "entry");
        let w_done = self.context.append_basic_block(walk_fn, "done");
        self.builder.position_at_end(w_entry);
        let lst = walk_fn.get_first_param().unwrap().into_struct_value();
        let target = walk_fn.get_nth_param(1).unwrap().into_struct_value();
        let node0 = self
            .builder
            .build_extract_value(lst, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let h0 = self
            .builder
            .build_extract_value(lst, 2, "h")
            .map_err(llvm_err)?
            .into_int_value();
        let idx_a = self.builder.build_alloca(i64, "idx_a").map_err(llvm_err)?;
        let res_a = self.builder.build_alloca(i64, "res_a").map_err(llvm_err)?;
        let done_a = self.builder.build_alloca(i64, "done_a").map_err(llvm_err)?;
        self.builder.build_store(idx_a, zero).map_err(llvm_err)?;
        self.builder.build_store(res_a, neg1_out).map_err(llvm_err)?;
        self.builder.build_store(done_a, zero).map_err(llvm_err)?;
        let _ = self.builder.build_call(
            rec_fn,
            &[
                node0.into(),
                h0.into(),
                target.into(),
                idx_a.into(),
                res_a.into(),
                done_a.into(),
            ],
            "",
        ).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(w_done);
        self.builder.position_at_end(w_done);
        let out = self
            .builder
            .build_load(i64, res_a, "out")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&out));

        Ok(())
    }
}
