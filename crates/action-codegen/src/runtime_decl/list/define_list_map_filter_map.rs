// Fused map+filter+map: single B-tree walk (map_inner, filter, map_outer).

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(in crate::runtime_decl) fn define_list_map_filter_map(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let void = self.context.void_type();
        let zero = i64.const_int(0, false);
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let lambda_fn_ty = self.string_type.fn_type(&[i64.into()], false);
        let push_leaf_fn = self.module.get_function("action_list_push_leaf").unwrap();
        let create_fn = self.module.get_function("action_list_create").unwrap();

        // ---- action_list_map_filter_map_walk_rec(ptr node, i64 height, ptr map_fn, ptr filter_fn, ptr acc, ptr buf_p, ptr buf_pos_p) -> void ----
        // Fused map+filter: in-order B-tree walk that applies map_fn to each element,
        // then calls filter_fn on the mapped value, pushing only passing elements.
        let mfmw_leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let mfmr_rec_fn = self.module.add_function(
            "action_list_map_filter_map_walk_rec",
            void.fn_type(
                &[
                    ptr.into(),
                    i64.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                    ptr.into(),
                ],
                false,
            ),
            None,
        );
        let mfmr_entry = self.context.append_basic_block(mfmr_rec_fn, "entry");
        let mfmr_leaf_hdr = self.context.append_basic_block(mfmr_rec_fn, "leaf_hdr");
        let mfmr_leaf_bdy = self.context.append_basic_block(mfmr_rec_fn, "leaf_bdy");
        let mfmr_leaf_chk = self.context.append_basic_block(mfmr_rec_fn, "leaf_chk");
        let mfmr_leaf_push = self.context.append_basic_block(mfmr_rec_fn, "leaf_push");
        let mfmr_leaf_flush = self.context.append_basic_block(mfmr_rec_fn, "leaf_flush");
        let mfmr_leaf_next = self.context.append_basic_block(mfmr_rec_fn, "leaf_next");
        let mfmr_leaf_done = self.context.append_basic_block(mfmr_rec_fn, "leaf_done");
        let mfmr_int_hdr = self.context.append_basic_block(mfmr_rec_fn, "int_hdr");
        let mfmr_int_bdy = self.context.append_basic_block(mfmr_rec_fn, "int_bdy");
        let mfmr_int_child = self.context.append_basic_block(mfmr_rec_fn, "int_child");
        let mfmr_int_next = self.context.append_basic_block(mfmr_rec_fn, "int_next");
        let mfmr_concat = self.context.append_basic_block(mfmr_rec_fn, "concat");
        let mfmr_normal = self.context.append_basic_block(mfmr_rec_fn, "normal");
        self.builder.position_at_end(mfmr_entry);
        let mfmr_node = mfmr_rec_fn.get_first_param().unwrap().into_pointer_value();
        let mfmr_height = mfmr_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let mfmr_map_fn = mfmr_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let mfmr_filter_fn = mfmr_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let mfmr_map_outer_fn = mfmr_rec_fn.get_nth_param(4).unwrap().into_pointer_value();
        let mfmr_acc = mfmr_rec_fn.get_nth_param(5).unwrap().into_pointer_value();
        let mfmr_buf_p = mfmr_rec_fn.get_nth_param(6).unwrap().into_pointer_value();
        let mfmr_buf_pos_p = mfmr_rec_fn.get_nth_param(7).unwrap().into_pointer_value();
        let mfmr_neg1 = i64.const_int(-1i64 as u64, true);
        let mfmr_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, mfmr_height, mfmr_neg1, "mfmr_is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mfmr_is_concat, mfmr_concat, mfmr_normal);
        self.builder.position_at_end(mfmr_concat);
        let mfmr_ln_p = unsafe {
            self.builder
                .build_gep(ptr, mfmr_node, &[i64.const_int(2, false)], "mfmr_ln_p")
                .map_err(llvm_err)
        }?;
        let mfmr_left_node = self
            .builder
            .build_load(ptr, mfmr_ln_p, "mfmr_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfmr_lh_p = unsafe {
            self.builder
                .build_gep(i64, mfmr_node, &[i64.const_int(4, false)], "mfmr_lh_p")
                .map_err(llvm_err)
        }?;
        let mfmr_left_h = self
            .builder
            .build_load(i64, mfmr_lh_p, "mfmr_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let mfmr_rn_p = unsafe {
            self.builder
                .build_gep(ptr, mfmr_node, &[i64.const_int(5, false)], "mfmr_rn_p")
                .map_err(llvm_err)
        }?;
        let mfmr_right_node = self
            .builder
            .build_load(ptr, mfmr_rn_p, "mfmr_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfmr_rh_p = unsafe {
            self.builder
                .build_gep(i64, mfmr_node, &[i64.const_int(7, false)], "mfmr_rh_p")
                .map_err(llvm_err)
        }?;
        let mfmr_right_h = self
            .builder
            .build_load(i64, mfmr_rh_p, "mfmr_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                mfmr_rec_fn,
                &[
                    mfmr_left_node.into(),
                    mfmr_left_h.into(),
                    mfmr_map_fn.into(),
                    mfmr_filter_fn.into(),
                    mfmr_map_outer_fn.into(),
                    mfmr_acc.into(),
                    mfmr_buf_p.into(),
                    mfmr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                mfmr_rec_fn,
                &[
                    mfmr_right_node.into(),
                    mfmr_right_h.into(),
                    mfmr_map_fn.into(),
                    mfmr_filter_fn.into(),
                    mfmr_map_outer_fn.into(),
                    mfmr_acc.into(),
                    mfmr_buf_p.into(),
                    mfmr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(mfmr_normal);
        let mfmr_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, mfmr_height, zero, "mfmr_is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mfmr_is_leaf, mfmr_leaf_hdr, mfmr_int_hdr);

        // Leaf scan: apply map, then filter, push only passing elements
        self.builder.position_at_end(mfmr_leaf_hdr);
        let mfmr_leaf_i8 = self
            .builder
            .build_pointer_cast(mfmr_node, ptr, "mfmr_leaf_i8")
            .map_err(llvm_err)?;
        let mfmr_count_raw = self
            .builder
            .build_load(i32, mfmr_leaf_i8, "mfmr_count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mfmr_count = self
            .builder
            .build_int_z_extend(mfmr_count_raw, i64, "mfmr_count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfmr_leaf_bdy);
        self.builder.position_at_end(mfmr_leaf_bdy);
        let mfmr_i = self.builder.build_phi(i64, "mfmr_i").map_err(llvm_err)?;
        let mfmr_done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mfmr_i.as_basic_value().into_int_value(),
                mfmr_count,
                "mfmr_done",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(mfmr_done_leaf, mfmr_leaf_done, mfmr_leaf_chk);
        self.builder.position_at_end(mfmr_leaf_chk);
        let mfmr_eb = unsafe {
            self.builder
                .build_gep(i8, mfmr_leaf_i8, &[i64.const_int(8, false)], "mfmr_eb")
                .map_err(llvm_err)?
        };
        let mfmr_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    mfmr_eb,
                    &[mfmr_i.as_basic_value().into_int_value()],
                    "mfmr_ep",
                )
                .map_err(llvm_err)?
        };
        let mfmr_elem = self
            .builder
            .build_load(self.string_type, mfmr_ep, "mfmr_elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let mfmr_elem_tag = self
            .builder
            .build_extract_value(mfmr_elem, 0, "mfmr_etag")
            .map_err(llvm_err)?
            .into_int_value();
        // Apply map function
        let mfmr_mapped_call = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                mfmr_map_fn,
                &[mfmr_elem_tag.into()],
                "mfmr_map",
            )
            .map_err(llvm_err)?;
        let mfmr_mapped_bv = mfmr_mapped_call
            .try_as_basic_value()
            .basic()
            .ok_or("map_filter_walk map call failed")?;
        // Extract tag from mapped value for filter predicate
        let mfmr_mapped_struct = mfmr_mapped_bv.into_struct_value();
        let mfmr_mapped_tag = self
            .builder
            .build_extract_value(mfmr_mapped_struct, 0, "mfmr_mt")
            .map_err(llvm_err)?
            .into_int_value();
        // Apply filter function on mapped value
        let mfmr_pred = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                mfmr_filter_fn,
                &[mfmr_mapped_tag.into()],
                "mfmr_pred",
            )
            .map_err(llvm_err)?;
        let mfmr_pred_bv = mfmr_pred
            .try_as_basic_value()
            .basic()
            .ok_or("map_filter_walk filter call failed")?;
        let mfmr_pred_val = if mfmr_pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(mfmr_pred_bv.into_struct_value(), 0, "mfmr_pv")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            mfmr_pred_bv.into_int_value()
        };
        let mfmr_is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, mfmr_pred_val, zero, "mfmr_is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mfmr_is_true, mfmr_leaf_push, mfmr_leaf_next);
        // Push outer-mapped value to buffer
        self.builder.position_at_end(mfmr_leaf_push);

        // Apply outer map function on filtered value
        let mfmr_outer_tag = self
            .builder
            .build_extract_value(mfmr_mapped_struct, 0, "mfmr_ot")
            .map_err(llvm_err)?
            .into_int_value();
        let mfmr_outer_call = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                mfmr_map_outer_fn,
                &[mfmr_outer_tag.into()],
                "mfmr_outer",
            )
            .map_err(llvm_err)?;
        let mfmr_outer_bv = mfmr_outer_call
            .try_as_basic_value()
            .basic()
            .ok_or("map_filter_map_walk outer map call failed")?;
        let mfmr_buf = self
            .builder
            .build_load(ptr, mfmr_buf_p, "mfmr_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfmr_pos = self
            .builder
            .build_load(i64, mfmr_buf_pos_p, "mfmr_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mfmr_buf_i8 = self
            .builder
            .build_pointer_cast(mfmr_buf, ptr, "mfmr_buf_i8")
            .map_err(llvm_err)?;
        let mfmr_buf_eb = unsafe {
            self.builder
                .build_gep(i8, mfmr_buf_i8, &[i64.const_int(8, false)], "mfmr_buf_eb")
                .map_err(llvm_err)?
        };
        let mfmr_buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, mfmr_buf_eb, &[mfmr_pos], "mfmr_buf_ep")
                .map_err(llvm_err)?
        };
        let _ = self
            .builder
            .build_store(mfmr_buf_ep, mfmr_outer_bv)
            .map_err(llvm_err)?;
        let mfmr_pos_inc = self
            .builder
            .build_int_add(mfmr_pos, i64.const_int(1, false), "mfmr_pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfmr_buf_pos_p, mfmr_pos_inc)
            .map_err(llvm_err)?;
        let mfmr_buf_full = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                mfmr_pos_inc,
                i64.const_int(64, false),
                "mfmr_buf_full",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(mfmr_buf_full, mfmr_leaf_flush, mfmr_leaf_next);

        self.builder.position_at_end(mfmr_leaf_flush);
        let mfmr_flush_cnt = i32.const_int(64, false);
        let _ = self
            .builder
            .build_store(mfmr_buf_i8, mfmr_flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mfmr_acc.into(), mfmr_buf.into()], "")
            .map_err(llvm_err)?;
        let mfmr_new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[mfmw_leaf_sz.into()], "mfmr_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mfmr_new_buf_i8 = self
            .builder
            .build_pointer_cast(mfmr_new_buf, ptr, "mfmr_new_buf_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mfmr_new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfmr_buf_p, mfmr_new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfmr_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfmr_leaf_next);

        self.builder.position_at_end(mfmr_leaf_next);
        let mfmr_next_i = self
            .builder
            .build_int_add(
                mfmr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mfmr_ni",
            )
            .map_err(llvm_err)?;
        let mfmr_leaf_next_bb = self.builder.get_insert_block().unwrap();
        mfmr_i.add_incoming(&[(&zero, mfmr_leaf_hdr), (&mfmr_next_i, mfmr_leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mfmr_leaf_bdy);
        self.builder.position_at_end(mfmr_leaf_done);
        let _ = self.builder.build_return(None);

        // Internal node: recurse into each child in order
        self.builder.position_at_end(mfmr_int_hdr);
        let mfmr_int_i8 = self
            .builder
            .build_pointer_cast(mfmr_node, ptr, "mfmr_int_i8")
            .map_err(llvm_err)?;
        let mfmr_child_count_raw = self
            .builder
            .build_load(i32, mfmr_int_i8, "mfmr_cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let mfmr_child_count = self
            .builder
            .build_int_z_extend(mfmr_child_count_raw, i64, "mfmr_cc")
            .map_err(llvm_err)?;
        let mfmr_child_h = self
            .builder
            .build_int_sub(mfmr_height, i64.const_int(1, false), "mfmr_ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfmr_int_bdy);
        self.builder.position_at_end(mfmr_int_bdy);
        let mfmr_ci = self.builder.build_phi(i64, "mfmr_ci").map_err(llvm_err)?;
        let mfmr_done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                mfmr_ci.as_basic_value().into_int_value(),
                mfmr_child_count,
                "mfmr_done_int",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(mfmr_done_int, mfmr_leaf_done, mfmr_int_child);
        self.builder.position_at_end(mfmr_int_child);
        let mfmr_children_base = unsafe {
            self.builder
                .build_gep(i8, mfmr_int_i8, &[i64.const_int(16, false)], "mfmr_cb")
                .map_err(llvm_err)?
        };
        let mfmr_child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    mfmr_children_base,
                    &[mfmr_ci.as_basic_value().into_int_value()],
                    "mfmr_cep",
                )
                .map_err(llvm_err)?
        };
        let mfmr_child_entry = self
            .builder
            .build_load(self.child_entry_type, mfmr_child_ep, "mfmr_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let mfmr_child_ptr = self
            .builder
            .build_extract_value(mfmr_child_entry, 0, "mfmr_cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                mfmr_rec_fn,
                &[
                    mfmr_child_ptr.into(),
                    mfmr_child_h.into(),
                    mfmr_map_fn.into(),
                    mfmr_filter_fn.into(),
                    mfmr_map_outer_fn.into(),
                    mfmr_acc.into(),
                    mfmr_buf_p.into(),
                    mfmr_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfmr_int_next);
        self.builder.position_at_end(mfmr_int_next);
        let mfmr_next_ci = self
            .builder
            .build_int_add(
                mfmr_ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "mfmr_nci",
            )
            .map_err(llvm_err)?;
        let mfmr_int_next_bb = self.builder.get_insert_block().unwrap();
        mfmr_ci.add_incoming(&[(&zero, mfmr_int_hdr), (&mfmr_next_ci, mfmr_int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(mfmr_int_bdy);

        // ---- action_list_map_filter_map_walk({ptr,i64,i64} list, ptr map_fn, ptr filter_fn) -> {ptr,i64,i64} ----
        let mfmw_fn = self.module.add_function(
            "action_list_map_filter_map_walk",
            self.list_type.fn_type(
                &[self.list_type.into(), ptr.into(), ptr.into(), ptr.into()],
                false,
            ),
            None,
        );
        let mfmw_entry = self.context.append_basic_block(mfmw_fn, "entry");
        let mfmw_walk = self.context.append_basic_block(mfmw_fn, "walk");
        let mfmw_flush = self.context.append_basic_block(mfmw_fn, "flush");
        let mfmw_done = self.context.append_basic_block(mfmw_fn, "done");
        self.builder.position_at_end(mfmw_entry);
        let mfmw_list = mfmw_fn.get_first_param().unwrap().into_struct_value();
        let mfmw_map_fn_ptr = mfmw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let mfmw_filter_fn_ptr = mfmw_fn.get_nth_param(2).unwrap().into_pointer_value();
        let mfmw_map_outer_fn_ptr = mfmw_fn.get_nth_param(3).unwrap().into_pointer_value();
        let mfmw_node = self
            .builder
            .build_extract_value(mfmw_list, 0, "mfmw_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfmw_len = self
            .builder
            .build_extract_value(mfmw_list, 1, "mfmw_len")
            .map_err(llvm_err)?
            .into_int_value();
        let mfmw_height = self
            .builder
            .build_extract_value(mfmw_list, 2, "mfmw_height")
            .map_err(llvm_err)?
            .into_int_value();
        let mfmw_acc = self
            .builder
            .build_alloca(self.list_type, "mfmw_acc")
            .map_err(llvm_err)?;
        let mfmw_buf_p = self
            .builder
            .build_alloca(ptr, "mfmw_buf_p")
            .map_err(llvm_err)?;
        let mfmw_buf_pos_p = self
            .builder
            .build_alloca(i64, "mfmw_buf_pos_p")
            .map_err(llvm_err)?;
        let mfmw_init = self
            .builder
            .build_call(create_fn, &[mfmw_len.into()], "mfmw_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(mfmw_acc, mfmw_init)
            .map_err(llvm_err)?;
        let mfmw_buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[mfmw_leaf_sz.into()], "mfmw_buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mfmw_buf_init_i8 = self
            .builder
            .build_pointer_cast(mfmw_buf_init, ptr, "mfmw_buf_init_i8")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mfmw_buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfmw_buf_p, mfmw_buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(mfmw_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfmw_walk);
        self.builder.position_at_end(mfmw_walk);
        let _ = self
            .builder
            .build_call(
                mfmr_rec_fn,
                &[
                    mfmw_node.into(),
                    mfmw_height.into(),
                    mfmw_map_fn_ptr.into(),
                    mfmw_filter_fn_ptr.into(),
                    mfmw_map_outer_fn_ptr.into(),
                    mfmw_acc.into(),
                    mfmw_buf_p.into(),
                    mfmw_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let mfmw_rem_pos = self
            .builder
            .build_load(i64, mfmw_buf_pos_p, "mfmw_rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let mfmw_has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, mfmw_rem_pos, zero, "mfmw_has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mfmw_has_rem, mfmw_flush, mfmw_done);
        self.builder.position_at_end(mfmw_flush);
        let mfmw_rem_buf = self
            .builder
            .build_load(ptr, mfmw_buf_p, "mfmw_rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mfmw_rem_buf_i8 = self
            .builder
            .build_pointer_cast(mfmw_rem_buf, ptr, "mfmw_rem_buf_i8")
            .map_err(llvm_err)?;
        let mfmw_rem_cnt = self
            .builder
            .build_int_truncate(mfmw_rem_pos, i32, "mfmw_rem_cnt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mfmw_rem_buf_i8, mfmw_rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[mfmw_acc.into(), mfmw_rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mfmw_done);
        self.builder.position_at_end(mfmw_done);
        let mfmw_res = self
            .builder
            .build_load(self.list_type, mfmw_acc, "mfmw_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mfmw_res));
        Ok(())
    }
}
