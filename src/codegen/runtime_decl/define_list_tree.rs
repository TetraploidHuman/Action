// Submodule: runtime_decl/define_list_tree
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_tree(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let _void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let zero = self.i64_ty().const_int(0, false);
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();

        let memcmp_fn = self.module.get_function("memcmp").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();

        let _list_create_fn = self.module.get_function("action_list_create").unwrap();
        let _list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        // ---- action_list_slice({ptr, i64, i64}, i64 start, i64 end) -> {ptr, i64, i64} ----
        let slc_fn = self.module.add_function(
            "action_list_slice",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into(), i64.into()], false),
            None,
        );
        let slc_entry = self.context.append_basic_block(slc_fn, "entry");
        let slc_concat = self.context.append_basic_block(slc_fn, "concat");
        let slc_normal = self.context.append_basic_block(slc_fn, "normal");
        let slc_h0 = self.context.append_basic_block(slc_fn, "h0");
        let slc_h0_ci_loop = self.context.append_basic_block(slc_fn, "h0_ci_loop");
        let slc_h0_ci_body = self.context.append_basic_block(slc_fn, "h0_ci_body");
        let slc_h0_done = self.context.append_basic_block(slc_fn, "h0_done");
        let slc_hgt0 = self.context.append_basic_block(slc_fn, "hgt0");
        self.builder.position_at_end(slc_entry);
        let slc_list = slc_fn.get_first_param().unwrap().into_struct_value();
        let slc_start = slc_fn.get_nth_param(1).unwrap().into_int_value();
        let slc_end = slc_fn.get_nth_param(2).unwrap().into_int_value();
        let slc_node = self
            .builder
            .build_extract_value(slc_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let slc_len = self
            .builder
            .build_extract_value(slc_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_height = self
            .builder
            .build_extract_value(slc_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                slc_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(slc_is_concat, slc_concat, slc_normal);
        // ConcatNode: flatten then slice
        self.builder.position_at_end(slc_concat);
        let slc_flat_fn = self.module.get_function("action_list_flatten").unwrap();
        let slc_flat = self
            .builder
            .build_call(slc_flat_fn, &[slc_list.into()], "flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let slc_flat_rv = self
            .builder
            .build_call(
                slc_fn,
                &[slc_flat.into(), slc_start.into(), slc_end.into()],
                "slc_flat",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&slc_flat_rv));
        // Normal path: check h=0 vs h>0
        self.builder.position_at_end(slc_normal);
        let slc_is_h0 = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                slc_height,
                i64.const_int(0, false),
                "is_h0",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(slc_is_h0, slc_h0, slc_hgt0);
        // === h=0: direct leaf manipulation ===
        self.builder.position_at_end(slc_h0);
        let slc_leaf_i8 = self
            .builder
            .build_pointer_cast(slc_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let slc_count_raw = self
            .builder
            .build_load(i32, slc_leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_count = self
            .builder
            .build_int_z_extend(slc_count_raw, i64, "count")
            .map_err(llvm_err)?;
        let z = i64.const_int(0, false);
        // Clamp start to [0, count]
        let slc_s_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_start, z, "s_neg")
            .map_err(llvm_err)?;
        let slc_s_clamp = self
            .builder
            .build_select(slc_s_neg, z, slc_start, "s_clamp")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_s_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_s_clamp, slc_count, "s_gt")
            .map_err(llvm_err)?;
        let slc_s_final = self
            .builder
            .build_select(slc_s_gt, slc_count, slc_s_clamp, "s_final")
            .map_err(llvm_err)?
            .into_int_value();
        // Clamp end to [0, count]
        let slc_e_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_end, z, "e_neg")
            .map_err(llvm_err)?;
        let slc_e_clamp = self
            .builder
            .build_select(slc_e_neg, z, slc_end, "e_clamp")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_e_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_e_clamp, slc_count, "e_gt")
            .map_err(llvm_err)?;
        let slc_e_final = self
            .builder
            .build_select(slc_e_gt, slc_count, slc_e_clamp, "e_final")
            .map_err(llvm_err)?
            .into_int_value();
        // Compute result length
        let slc_rlen = self
            .builder
            .build_int_sub(slc_e_final, slc_s_final, "rlen")
            .map_err(llvm_err)?;
        let slc_rlen_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_rlen, z, "rlen_neg")
            .map_err(llvm_err)?;
        let slc_new_count = self
            .builder
            .build_select(slc_rlen_neg, z, slc_rlen, "new_count")
            .map_err(llvm_err)?
            .into_int_value();
        // Allocate new leaf
        let leaf_ty = self.leaf_type;
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let slc_new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "new_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Copy elements[start..end] from old leaf to new_leaf[0..new_count]
        let slc_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let slc_old_eb = unsafe {
            self.builder
                .build_gep(i8, slc_leaf_i8, &[i64.const_int(8, false)], "old_eb")
                .map_err(llvm_err)
        }?;
        let slc_src = unsafe {
            self.builder
                .build_gep(self.string_type, slc_old_eb, &[slc_s_final], "src")
                .map_err(llvm_err)
        }?;
        let slc_new_i8 = self
            .builder
            .build_pointer_cast(slc_new_leaf, ptr, "new_i8")
            .map_err(llvm_err)?;
        let slc_new_eb = unsafe {
            self.builder
                .build_gep(i8, slc_new_i8, &[i64.const_int(8, false)], "new_eb")
                .map_err(llvm_err)
        }?;
        let slc_dst = unsafe {
            self.builder
                .build_gep(self.string_type, slc_new_eb, &[z], "dst")
                .map_err(llvm_err)
        }?;
        let slc_copy_bytes = self
            .builder
            .build_int_mul(slc_new_count, i64.const_int(16, false), "copy_bytes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                slc_memcpy_fn,
                &[slc_dst.into(), slc_src.into(), slc_copy_bytes.into()],
                "",
            )
            .map_err(llvm_err)?;
        // RC-inc each element in the new leaf
        let slc_ci_i = self.builder.build_alloca(i64, "ci_i").map_err(llvm_err)?;
        self.builder.build_store(slc_ci_i, z).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(slc_h0_ci_loop);
        self.builder.position_at_end(slc_h0_ci_loop);
        let slc_ci = self
            .builder
            .build_load(i64, slc_ci_i, "ci")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_ci_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_ci, slc_new_count, "ci_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(slc_ci_cond, slc_h0_ci_body, slc_h0_done);
        self.builder.position_at_end(slc_h0_ci_body);
        let slc_rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let slc_ci_ep = unsafe {
            self.builder
                .build_gep(self.string_type, slc_new_eb, &[slc_ci], "ci_ep")
                .map_err(llvm_err)
        }?;
        let slc_ci_ev = self
            .builder
            .build_load(self.string_type, slc_ci_ep, "ci_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let slc_ci_ed = self
            .builder
            .build_extract_value(slc_ci_ev, 1, "ci_ed")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(slc_rc_inc_fn, &[slc_ci_ed.into()], "")
            .map_err(llvm_err)?;
        let slc_ci_next = self
            .builder
            .build_int_add(slc_ci, i64.const_int(1, false), "ci_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slc_ci_i, slc_ci_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(slc_h0_ci_loop);
        // Set count on new leaf and return
        self.builder.position_at_end(slc_h0_done);
        let slc_new_count_i32 = self
            .builder
            .build_int_truncate(slc_new_count, i32, "new_count_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(slc_new_i8, slc_new_count_i32)
            .map_err(llvm_err)?;
        let undef_slc = self.list_type.get_undef();
        let slc_r1 = self
            .builder
            .build_insert_value(undef_slc, slc_new_leaf, 0, "r1")
            .map_err(llvm_err)?;
        let slc_r2 = self
            .builder
            .build_insert_value(slc_r1, slc_new_count, 1, "r2")
            .map_err(llvm_err)?;
        let slc_r3 = self
            .builder
            .build_insert_value(slc_r2, i64.const_int(0, false), 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&slc_r3));
        // === h>0: per-element loop ===
        self.builder.position_at_end(slc_hgt0);
        let slc_s_neg2 = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                slc_start,
                i64.const_int(0, false),
                "sneg2",
            )
            .map_err(llvm_err)?;
        let slc_s_clamp2 = self
            .builder
            .build_select(slc_s_neg2, i64.const_int(0, false), slc_start, "sclamp2")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_s_gt2 = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_s_clamp2, slc_len, "sgt2")
            .map_err(llvm_err)?;
        let slc_s_final2 = self
            .builder
            .build_select(slc_s_gt2, slc_len, slc_s_clamp2, "sfinal2")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_e_neg2 = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_end, i64.const_int(0, false), "eneg2")
            .map_err(llvm_err)?;
        let slc_e_clamp2 = self
            .builder
            .build_select(slc_e_neg2, i64.const_int(0, false), slc_end, "eclamp2")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_e_gt2 = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_e_clamp2, slc_len, "egt2")
            .map_err(llvm_err)?;
        let slc_e_final2 = self
            .builder
            .build_select(slc_e_gt2, slc_len, slc_e_clamp2, "efinal2")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_rlen2 = self
            .builder
            .build_int_sub(slc_e_final2, slc_s_final2, "rlen2")
            .map_err(llvm_err)?;
        let slc_rlen_neg2 = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                slc_rlen2,
                i64.const_int(0, false),
                "rneg2",
            )
            .map_err(llvm_err)?;
        let slc_rlen_final2 = self
            .builder
            .build_select(slc_rlen_neg2, i64.const_int(0, false), slc_rlen2, "rlenf2")
            .map_err(llvm_err)?
            .into_int_value();
        let cc6 = self.call_rt("action_list_create", &[slc_rlen_final2.into()])?;
        let slc_new_init = cc6.try_as_basic_value().unwrap_basic().into_struct_value();
        let slc_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slc_new_alloc, slc_new_init)
            .map_err(llvm_err)?;
        let slc_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(slc_i_alloc, slc_s_final2)
            .map_err(llvm_err)?;
        let slc_loop = self.context.append_basic_block(slc_fn, "loop");
        let slc_body = self.context.append_basic_block(slc_fn, "body");
        let slc_done = self.context.append_basic_block(slc_fn, "done");
        let _ = self.builder.build_unconditional_branch(slc_loop);
        self.builder.position_at_end(slc_loop);
        let slc_i = self
            .builder
            .build_load(i64, slc_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_i, slc_e_final2, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(slc_cond, slc_body, slc_done);
        self.builder.position_at_end(slc_body);
        let slc_get_fn = self.module.get_function("action_list_get").unwrap();
        let slc_ev = self
            .builder
            .build_call(slc_get_fn, &[slc_list.into(), slc_i.into()], "ev")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let slc_ev_data = self
            .builder
            .build_extract_value(slc_ev.into_struct_value(), 1, "ev_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let slc_rc_inc_fn2 = self.module.get_function("action_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(slc_rc_inc_fn2, &[slc_ev_data.into()], "")
            .map_err(llvm_err)?;
        let slc_cur = self
            .builder
            .build_load(self.list_type, slc_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let cc7 = self.call_rt("action_list_push", &[slc_cur.into(), slc_ev.into()])?;
        let slc_nv = cc7.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(slc_new_alloc, slc_nv)
            .map_err(llvm_err)?;
        let slc_ni = self
            .builder
            .build_int_add(slc_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slc_i_alloc, slc_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(slc_loop);
        self.builder.position_at_end(slc_done);
        let slc_rv = self
            .builder
            .build_load(self.list_type, slc_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&slc_rv));

        // ---- action_list_insert({ptr, i64, i64}, i64 index, {i64, ptr}) -> {ptr, i64, i64} ----
        let li_fn = self.module.add_function(
            "action_list_insert",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into(), str_ty.into()], false),
            None,
        );
        let li_entry = self.context.append_basic_block(li_fn, "entry");
        let li_concat = self.context.append_basic_block(li_fn, "concat");
        let li_concat_append = self.context.append_basic_block(li_fn, "concat_append");
        let li_concat_chk_prepend = self.context.append_basic_block(li_fn, "concat_chk_pre");
        let li_concat_prepend = self.context.append_basic_block(li_fn, "concat_prepend");
        let li_concat_dispatch = self.context.append_basic_block(li_fn, "concat_dispatch");
        let li_concat_ins_left = self.context.append_basic_block(li_fn, "concat_ins_left");
        let li_concat_ins_right = self.context.append_basic_block(li_fn, "concat_ins_right");
        let li_concat_boundary = self.context.append_basic_block(li_fn, "concat_boundary");
        let li_concat_route = self.context.append_basic_block(li_fn, "concat_route");
        let li_normal = self.context.append_basic_block(li_fn, "normal");
        let li_h0 = self.context.append_basic_block(li_fn, "h0");
        let li_h0_cow = self.context.append_basic_block(li_fn, "h0_cow");
        let li_h0_cow_copy = self.context.append_basic_block(li_fn, "h0_cow_copy");
        let li_h0_ready = self.context.append_basic_block(li_fn, "h0_ready");
        let li_h0_shift_loop = self.context.append_basic_block(li_fn, "h0_shift_loop");
        let li_h0_shift_body = self.context.append_basic_block(li_fn, "h0_shift_body");
        let li_h0_shift_done = self.context.append_basic_block(li_fn, "h0_shift_done");
        let li_h0_done = self.context.append_basic_block(li_fn, "h0_done");
        let li_hgt0 = self.context.append_basic_block(li_fn, "hgt0");
        self.builder.position_at_end(li_entry);
        let li_list = li_fn.get_first_param().unwrap().into_struct_value();
        let li_index = li_fn.get_nth_param(1).unwrap().into_int_value();
        let li_elem = li_fn.get_nth_param(2).unwrap().into_struct_value();
        let li_node = self
            .builder
            .build_extract_value(li_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let li_total_len = self
            .builder
            .build_extract_value(li_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let li_height = self
            .builder
            .build_extract_value(li_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let li_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                li_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(li_is_concat, li_concat, li_normal);
        // ConcatNode: lazy concat insert (append/prepend/middle dispatch)
        self.builder.position_at_end(li_concat);
        let li_create_fn = self.module.get_function("action_list_create").unwrap();
        let li_push_fn = self.module.get_function("action_list_push").unwrap();
        let li_concat_fn = self.module.get_function("action_list_concat").unwrap();
        // append: index == len -> lazy concat(list, singleton(elem))
        let li_cc_is_append = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_index, li_total_len, "cc_app")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            li_cc_is_append,
            li_concat_append,
            li_concat_chk_prepend,
        );
        self.builder.position_at_end(li_concat_append);
        let li_cc_empty = self
            .builder
            .build_call(li_create_fn, &[zero.into()], "cc_empty")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_sing_a = self
            .builder
            .build_call(
                li_push_fn,
                &[li_cc_empty.into(), li_elem.into()],
                "cc_sing_a",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_app_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_list.into(), li_cc_sing_a.into()],
                "cc_app_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_app_r));
        // prepend: index == 0 -> lazy concat(singleton(elem), list)
        self.builder.position_at_end(li_concat_chk_prepend);
        let li_cc_is_prepend = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_index, zero, "cc_pre")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            li_cc_is_prepend,
            li_concat_prepend,
            li_concat_dispatch,
        );
        self.builder.position_at_end(li_concat_prepend);
        let li_cc_empty2 = self
            .builder
            .build_call(li_create_fn, &[zero.into()], "cc_empty2")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_sing_p = self
            .builder
            .build_call(
                li_push_fn,
                &[li_cc_empty2.into(), li_elem.into()],
                "cc_sing_p",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_pre_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_sing_p.into(), li_list.into()],
                "cc_pre_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_pre_r));
        // middle insert: load child lists before dispatch branches (SSA)
        self.builder.position_at_end(li_concat_dispatch);
        let li_cc_ln_p = unsafe {
            self.builder
                .build_gep(ptr, li_node, &[i64.const_int(2, false)], "cc_ln_p")
                .map_err(llvm_err)
        }?;
        let li_cc_left_node = self
            .builder
            .build_load(ptr, li_cc_ln_p, "cc_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let li_cc_ll_p = unsafe {
            self.builder
                .build_gep(i64, li_node, &[i64.const_int(3, false)], "cc_ll_p")
                .map_err(llvm_err)
        }?;
        let li_cc_left_len = self
            .builder
            .build_load(i64, li_cc_ll_p, "cc_ll")
            .map_err(llvm_err)?
            .into_int_value();
        let li_cc_lh_p = unsafe {
            self.builder
                .build_gep(i64, li_node, &[i64.const_int(4, false)], "cc_lh_p")
                .map_err(llvm_err)
        }?;
        let li_cc_left_h = self
            .builder
            .build_load(i64, li_cc_lh_p, "cc_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let li_cc_undef = self.list_type.get_undef();
        let li_cc_l1 = self
            .builder
            .build_insert_value(li_cc_undef, li_cc_left_node, 0, "cc_l1")
            .map_err(llvm_err)?;
        let li_cc_l2 = self
            .builder
            .build_insert_value(li_cc_l1, li_cc_left_len, 1, "cc_l2")
            .map_err(llvm_err)?;
        let li_cc_left = self
            .builder
            .build_insert_value(li_cc_l2, li_cc_left_h, 2, "cc_left")
            .map_err(llvm_err)?
            .into_struct_value();
        let li_cc_rn_p = unsafe {
            self.builder
                .build_gep(ptr, li_node, &[i64.const_int(5, false)], "cc_rn_p")
                .map_err(llvm_err)
        }?;
        let li_cc_right_node = self
            .builder
            .build_load(ptr, li_cc_rn_p, "cc_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let li_cc_rl_p = unsafe {
            self.builder
                .build_gep(i64, li_node, &[i64.const_int(6, false)], "cc_rl_p")
                .map_err(llvm_err)
        }?;
        let li_cc_right_len = self
            .builder
            .build_load(i64, li_cc_rl_p, "cc_rl")
            .map_err(llvm_err)?
            .into_int_value();
        let li_cc_rh_p = unsafe {
            self.builder
                .build_gep(i64, li_node, &[i64.const_int(7, false)], "cc_rh_p")
                .map_err(llvm_err)
        }?;
        let li_cc_right_h = self
            .builder
            .build_load(i64, li_cc_rh_p, "cc_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let li_cc_rundef = self.list_type.get_undef();
        let li_cc_r1 = self
            .builder
            .build_insert_value(li_cc_rundef, li_cc_right_node, 0, "cc_r1")
            .map_err(llvm_err)?;
        let li_cc_r2 = self
            .builder
            .build_insert_value(li_cc_r1, li_cc_right_len, 1, "cc_r2")
            .map_err(llvm_err)?;
        let li_cc_right = self
            .builder
            .build_insert_value(li_cc_r2, li_cc_right_h, 2, "cc_right")
            .map_err(llvm_err)?
            .into_struct_value();
        let li_cc_lt_left = self
            .builder
            .build_int_compare(IntPredicate::SLT, li_index, li_cc_left_len, "cc_lt_l")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            li_cc_lt_left,
            li_concat_ins_left,
            li_concat_route,
        );
        // idx >= left.len: boundary vs insert-right
        self.builder.position_at_end(li_concat_route);
        let li_cc_is_boundary = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_index, li_cc_left_len, "cc_bnd")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            li_cc_is_boundary,
            li_concat_boundary,
            li_concat_ins_right,
        );
        // idx < left.len: insert(left), concat(result, right)
        self.builder.position_at_end(li_concat_ins_left);
        let li_cc_il = self
            .builder
            .build_call(
                li_fn,
                &[li_cc_left.into(), li_index.into(), li_elem.into()],
                "cc_il",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_il_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_il.into(), li_cc_right.into()],
                "cc_il_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_il_r));
        // idx == left.len: boundary concat(left, singleton(elem), right)
        self.builder.position_at_end(li_concat_boundary);
        let li_cc_empty3 = self
            .builder
            .build_call(li_create_fn, &[zero.into()], "cc_empty3")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_sing_b = self
            .builder
            .build_call(
                li_push_fn,
                &[li_cc_empty3.into(), li_elem.into()],
                "cc_sing_b",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_mid = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_left.into(), li_cc_sing_b.into()],
                "cc_mid",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_bnd_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_mid.into(), li_cc_right.into()],
                "cc_bnd_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_bnd_r));
        // idx > left.len: insert(right, idx-left.len), concat(left, result)
        self.builder.position_at_end(li_concat_ins_right);
        let li_cc_ri = self
            .builder
            .build_int_sub(li_index, li_cc_left_len, "cc_ri")
            .map_err(llvm_err)?;
        let li_cc_ir = self
            .builder
            .build_call(
                li_fn,
                &[li_cc_right.into(), li_cc_ri.into(), li_elem.into()],
                "cc_ir",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_cc_ir_r = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_cc_left.into(), li_cc_ir.into()],
                "cc_ir_r",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_cc_ir_r));
        // Normal path: check h=0 vs h>0
        self.builder.position_at_end(li_normal);
        let li_is_h0 = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                li_height,
                i64.const_int(0, false),
                "is_h0",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(li_is_h0, li_h0, li_hgt0);
        // === h=0: direct leaf manipulation (with room) ===
        self.builder.position_at_end(li_h0);
        let li_leaf_i8 = self
            .builder
            .build_pointer_cast(li_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let li_count_raw = self
            .builder
            .build_load(i32, li_leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let li_count = self
            .builder
            .build_int_z_extend(li_count_raw, i64, "count")
            .map_err(llvm_err)?;
        let z = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let li_idx0 = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, li_index, z, "idx_neg")
                    .map_err(llvm_err)?,
                z,
                li_index,
                "idx0",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let li_idx = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SGT, li_idx0, li_count, "idx_gt")
                    .map_err(llvm_err)?,
                li_count,
                li_idx0,
                "idx",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let li_is_full = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                li_count,
                i64.const_int(64, false),
                "is_full",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(li_is_full, li_hgt0, li_h0_cow);
        // CoW check
        self.builder.position_at_end(li_h0_cow);
        let li_node_int = self
            .builder
            .build_ptr_to_int(li_node, i64, "node_int")
            .map_err(llvm_err)?;
        let li_rc_addr = self
            .builder
            .build_int_sub(li_node_int, i64.const_int(8, false), "rc_addr")
            .map_err(llvm_err)?;
        let li_rc_ptr = self
            .builder
            .build_int_to_ptr(li_rc_addr, ptr, "rc_ptr")
            .map_err(llvm_err)?;
        let li_rc_val = self
            .builder
            .build_load(i64, li_rc_ptr, "rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let li_need_cow = self
            .builder
            .build_int_compare(IntPredicate::SGT, li_rc_val, one, "need_cow")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(li_need_cow, li_h0_cow_copy, li_h0_ready);
        // CoW: copy leaf only when shared (rc > 1)
        self.builder.position_at_end(li_h0_cow_copy);
        let leaf_ty = self.leaf_type;
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let li_cow_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "cow_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let li_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                li_memcpy_fn,
                &[li_cow_leaf.into(), li_node.into(), leaf_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(li_h0_ready);
        // Unique reference (rc == 1): mutate leaf in place; shared: use copied leaf
        self.builder.position_at_end(li_h0_ready);
        let li_leaf_phi = self.builder.build_phi(ptr, "leaf_phi").map_err(llvm_err)?;
        li_leaf_phi.add_incoming(&[(&li_node, li_h0_cow), (&li_cow_leaf, li_h0_cow_copy)]);
        let li_leaf = li_leaf_phi.as_basic_value().into_pointer_value();
        let li_leaf2_i8 = self
            .builder
            .build_pointer_cast(li_leaf, ptr, "leaf2_i8")
            .map_err(llvm_err)?;
        let li_eb = unsafe {
            self.builder
                .build_gep(i8, li_leaf2_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)
        }?;
        // Shift elements [idx..count-1] right by 1 (reverse loop)
        let li_si = self.builder.build_alloca(i64, "si").map_err(llvm_err)?;
        let li_count_minus1 = self
            .builder
            .build_int_sub(li_count, one, "cm1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(li_si, li_count_minus1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(li_h0_shift_loop);
        self.builder.position_at_end(li_h0_shift_loop);
        let li_siv = self
            .builder
            .build_load(i64, li_si, "siv")
            .map_err(llvm_err)?
            .into_int_value();
        let li_si_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, li_siv, li_idx, "si_cond")
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(li_si_cond, li_h0_shift_body, li_h0_shift_done);
        self.builder.position_at_end(li_h0_shift_body);
        let li_src = unsafe {
            self.builder
                .build_gep(self.string_type, li_eb, &[li_siv], "src")
                .map_err(llvm_err)
        }?;
        let li_sv = self
            .builder
            .build_load(self.string_type, li_src, "sv")
            .map_err(llvm_err)?;
        let li_siv_plus1 = self
            .builder
            .build_int_add(li_siv, one, "siv_p1")
            .map_err(llvm_err)?;
        let li_dst = unsafe {
            self.builder
                .build_gep(self.string_type, li_eb, &[li_siv_plus1], "dst")
                .map_err(llvm_err)
        }?;
        self.builder.build_store(li_dst, li_sv).map_err(llvm_err)?;
        let li_siv_minus1 = self
            .builder
            .build_int_sub(li_siv, one, "siv_m1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(li_si, li_siv_minus1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(li_h0_shift_loop);
        // Insert new element and increment count
        self.builder.position_at_end(li_h0_shift_done);
        let li_ins_dst = unsafe {
            self.builder
                .build_gep(self.string_type, li_eb, &[li_idx], "ins_dst")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(li_ins_dst, li_elem)
            .map_err(llvm_err)?;
        let li_new_count = self
            .builder
            .build_int_add(li_count, one, "new_count")
            .map_err(llvm_err)?;
        let li_new_count_i32 = self
            .builder
            .build_int_truncate(li_new_count, i32, "new_count_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(li_leaf2_i8, li_new_count_i32)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(li_h0_done);
        self.builder.position_at_end(li_h0_done);
        let li_new_total = self
            .builder
            .build_int_add(li_total_len, one, "new_total")
            .map_err(llvm_err)?;
        let undef_ins = self.list_type.get_undef();
        let li_r1 = self
            .builder
            .build_insert_value(undef_ins, li_leaf, 0, "r1")
            .map_err(llvm_err)?;
        let li_r2 = self
            .builder
            .build_insert_value(li_r1, li_new_total, 1, "r2")
            .map_err(llvm_err)?;
        let li_r3 = self
            .builder
            .build_insert_value(li_r2, i64.const_int(0, false), 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&li_r3));
        // === h>0 (or h=0 full): take+push+drop+concat ===
        self.builder.position_at_end(li_hgt0);
        let li_len = self
            .builder
            .build_extract_value(li_list, 1, "li_len")
            .map_err(llvm_err)?
            .into_int_value();
        let li_idx2 = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, li_index, z, "idx_neg2")
                    .map_err(llvm_err)?,
                z,
                li_index,
                "idx_clamped2",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let li_idx3 = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SGT, li_idx2, li_len, "idx_gt2")
                    .map_err(llvm_err)?,
                li_len,
                li_idx2,
                "idx2",
            )
            .map_err(llvm_err)?
            .into_int_value();
        // If appending to end, just push
        let li_is_append = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_idx3, li_len, "is_append")
            .map_err(llvm_err)?;
        let li_append_bb = self.context.append_basic_block(li_fn, "append");
        let li_split_bb = self.context.append_basic_block(li_fn, "split");
        let _ = self
            .builder
            .build_conditional_branch(li_is_append, li_append_bb, li_split_bb);
        // Append: just push
        self.builder.position_at_end(li_append_bb);
        let li_push_fn = self.module.get_function("action_list_push").unwrap();
        let li_push_rv = self
            .builder
            .build_call(li_push_fn, &[li_list.into(), li_elem.into()], "push_rv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_push_rv));
        // Split: take + push + drop + concat
        self.builder.position_at_end(li_split_bb);
        let li_take_fn = self.module.get_function("action_list_take").unwrap();
        let li_drop_fn = self.module.get_function("action_list_drop").unwrap();
        let li_concat_fn = self.module.get_function("action_list_concat").unwrap();
        let li_left = self
            .builder
            .build_call(li_take_fn, &[li_list.into(), li_idx3.into()], "left")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_right = self
            .builder
            .build_call(li_drop_fn, &[li_list.into(), li_idx3.into()], "right")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let li_left_with = self
            .builder
            .build_call(li_push_fn, &[li_left.into(), li_elem.into()], "left_with")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let li_result = self
            .builder
            .build_call(
                li_concat_fn,
                &[li_left_with.into(), li_right.into()],
                "result",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&li_result));

        // ---- action_list_remove({ptr, i64, i64}, i64 index) -> {ptr, i64, i64} ----
        let lrm_fn = self.module.add_function(
            "action_list_remove",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into()], false),
            None,
        );
        let lrm_entry = self.context.append_basic_block(lrm_fn, "entry");
        let lrm_concat = self.context.append_basic_block(lrm_fn, "concat");
        let lrm_concat_rm_left = self.context.append_basic_block(lrm_fn, "concat_rm_left");
        let lrm_concat_rm_right = self.context.append_basic_block(lrm_fn, "concat_rm_right");
        let lrm_normal = self.context.append_basic_block(lrm_fn, "normal");
        let lrm_h0 = self.context.append_basic_block(lrm_fn, "h0");
        let lrm_h0_cow = self.context.append_basic_block(lrm_fn, "h0_cow");
        let lrm_h0_cow_copy = self.context.append_basic_block(lrm_fn, "h0_cow_copy");
        let lrm_h0_ready = self.context.append_basic_block(lrm_fn, "h0_ready");
        let lrm_h0_shift_loop = self.context.append_basic_block(lrm_fn, "h0_shift_loop");
        let lrm_h0_shift_body = self.context.append_basic_block(lrm_fn, "h0_shift_body");
        let lrm_h0_done = self.context.append_basic_block(lrm_fn, "h0_done");
        let lrm_hgt0 = self.context.append_basic_block(lrm_fn, "hgt0");
        let lrm_empty_bb = self.context.append_basic_block(lrm_fn, "empty");
        self.builder.position_at_end(lrm_entry);
        let lrm_list = lrm_fn.get_first_param().unwrap().into_struct_value();
        let lrm_index = lrm_fn.get_nth_param(1).unwrap().into_int_value();
        let lrm_node = self
            .builder
            .build_extract_value(lrm_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lrm_total_len = self
            .builder
            .build_extract_value(lrm_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_height = self
            .builder
            .build_extract_value(lrm_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lrm_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_is_concat, lrm_concat, lrm_normal);
        // ConcatNode: lazy dispatch — remove in left/right subtree, rebuild via concat
        self.builder.position_at_end(lrm_concat);
        let lrm_cn_ln_p = unsafe {
            self.builder
                .build_gep(ptr, lrm_node, &[i64.const_int(2, false)], "cn_ln_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_left_node = self
            .builder
            .build_load(ptr, lrm_cn_ln_p, "cn_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lrm_cn_ll_p = unsafe {
            self.builder
                .build_gep(i64, lrm_node, &[i64.const_int(3, false)], "cn_ll_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_left_len = self
            .builder
            .build_load(i64, lrm_cn_ll_p, "cn_ll")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_cn_lh_p = unsafe {
            self.builder
                .build_gep(i64, lrm_node, &[i64.const_int(4, false)], "cn_lh_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_left_h = self
            .builder
            .build_load(i64, lrm_cn_lh_p, "cn_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_cn_l_undef = self.list_type.get_undef();
        let lrm_cn_l1 = self
            .builder
            .build_insert_value(lrm_cn_l_undef, lrm_cn_left_node, 0, "cn_l1")
            .map_err(llvm_err)?;
        let lrm_cn_l2 = self
            .builder
            .build_insert_value(lrm_cn_l1, lrm_cn_left_len, 1, "cn_l2")
            .map_err(llvm_err)?;
        let lrm_cn_left = self
            .builder
            .build_insert_value(lrm_cn_l2, lrm_cn_left_h, 2, "cn_left")
            .map_err(llvm_err)?
            .into_struct_value();
        let lrm_cn_rn_p = unsafe {
            self.builder
                .build_gep(ptr, lrm_node, &[i64.const_int(5, false)], "cn_rn_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_right_node = self
            .builder
            .build_load(ptr, lrm_cn_rn_p, "cn_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lrm_cn_rl_p = unsafe {
            self.builder
                .build_gep(i64, lrm_node, &[i64.const_int(6, false)], "cn_rl_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_right_len = self
            .builder
            .build_load(i64, lrm_cn_rl_p, "cn_rl")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_cn_rh_p = unsafe {
            self.builder
                .build_gep(i64, lrm_node, &[i64.const_int(7, false)], "cn_rh_p")
                .map_err(llvm_err)
        }?;
        let lrm_cn_right_h = self
            .builder
            .build_load(i64, lrm_cn_rh_p, "cn_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_cn_r_undef = self.list_type.get_undef();
        let lrm_cn_r1 = self
            .builder
            .build_insert_value(lrm_cn_r_undef, lrm_cn_right_node, 0, "cn_r1")
            .map_err(llvm_err)?;
        let lrm_cn_r2 = self
            .builder
            .build_insert_value(lrm_cn_r1, lrm_cn_right_len, 1, "cn_r2")
            .map_err(llvm_err)?;
        let lrm_cn_right = self
            .builder
            .build_insert_value(lrm_cn_r2, lrm_cn_right_h, 2, "cn_right")
            .map_err(llvm_err)?
            .into_struct_value();
        let lrm_cn_lt = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrm_index, lrm_cn_left_len, "cn_lt")
            .map_err(llvm_err)?;
        let lrm_cn_chk_right = self.context.append_basic_block(lrm_fn, "concat_chk_right");
        let _ =
            self.builder
                .build_conditional_branch(lrm_cn_lt, lrm_concat_rm_left, lrm_cn_chk_right);
        self.builder.position_at_end(lrm_cn_chk_right);
        let _ = self.builder.build_unconditional_branch(lrm_concat_rm_right);

        let lrm_concat_fn = self.module.get_function("action_list_concat").unwrap();

        self.builder.position_at_end(lrm_concat_rm_left);
        let lrm_cn_new_left = self
            .builder
            .build_call(lrm_fn, &[lrm_cn_left.into(), lrm_index.into()], "cn_nl")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lrm_cn_rl_res = self
            .builder
            .build_call(
                lrm_concat_fn,
                &[lrm_cn_new_left.into(), lrm_cn_right.into()],
                "cn_rl_res",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lrm_cn_rl_res));

        self.builder.position_at_end(lrm_concat_rm_right);
        let lrm_cn_new_idx = self
            .builder
            .build_int_sub(lrm_index, lrm_cn_left_len, "cn_ni")
            .map_err(llvm_err)?;
        let lrm_cn_new_right = self
            .builder
            .build_call(
                lrm_fn,
                &[lrm_cn_right.into(), lrm_cn_new_idx.into()],
                "cn_nr",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lrm_cn_rr_res = self
            .builder
            .build_call(
                lrm_concat_fn,
                &[lrm_cn_left.into(), lrm_cn_new_right.into()],
                "cn_rr_res",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lrm_cn_rr_res));

        // Normal path: check h=0 vs h>0
        self.builder.position_at_end(lrm_normal);
        let zr = i64.const_int(0, false);
        let oner = i64.const_int(1, false);
        let lrm_is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, lrm_height, zr, "is_h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_is_h0, lrm_h0, lrm_hgt0);
        // === h=0: direct leaf manipulation ===
        self.builder.position_at_end(lrm_h0);
        let lrm_leaf_i8 = self
            .builder
            .build_pointer_cast(lrm_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let lrm_count_raw = self
            .builder
            .build_load(i32, lrm_leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_count = self
            .builder
            .build_int_z_extend(lrm_count_raw, i64, "count")
            .map_err(llvm_err)?;
        // If count==0 return unchanged
        let lrm_count_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, lrm_count, zr, "count_zero")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_count_zero, lrm_empty_bb, lrm_h0_cow);
        // CoW check
        self.builder.position_at_end(lrm_h0_cow);
        let lrm_last = self
            .builder
            .build_int_sub(lrm_count, oner, "last")
            .map_err(llvm_err)?;
        let lrm_idx_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrm_index, zr, "idx_neg")
            .map_err(llvm_err)?;
        let lrm_idx1 = self
            .builder
            .build_select(lrm_idx_neg, zr, lrm_index, "idx1")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_idx_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, lrm_idx1, lrm_last, "idx_gt")
            .map_err(llvm_err)?;
        let lrm_idx = self
            .builder
            .build_select(lrm_idx_gt, lrm_last, lrm_idx1, "idx")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_node_int = self
            .builder
            .build_ptr_to_int(lrm_node, i64, "node_int")
            .map_err(llvm_err)?;
        let lrm_rc_addr = self
            .builder
            .build_int_sub(lrm_node_int, i64.const_int(8, false), "rc_addr")
            .map_err(llvm_err)?;
        let lrm_rc_ptr = self
            .builder
            .build_int_to_ptr(lrm_rc_addr, ptr, "rc_ptr")
            .map_err(llvm_err)?;
        let lrm_rc_val = self
            .builder
            .build_load(i64, lrm_rc_ptr, "rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_need_cow = self
            .builder
            .build_int_compare(IntPredicate::SGT, lrm_rc_val, oner, "need_cow")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_need_cow, lrm_h0_cow_copy, lrm_h0_ready);
        self.builder.position_at_end(lrm_h0_cow_copy);
        let leaf_ty = self.leaf_type;
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let lrm_cow_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "cow_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let lrm_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                lrm_memcpy_fn,
                &[lrm_cow_leaf.into(), lrm_node.into(), leaf_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lrm_h0_ready);
        self.builder.position_at_end(lrm_h0_ready);
        let lrm_leaf_phi = self.builder.build_phi(ptr, "leaf_phi").map_err(llvm_err)?;
        lrm_leaf_phi.add_incoming(&[(&lrm_node, lrm_h0_cow), (&lrm_cow_leaf, lrm_h0_cow_copy)]);
        let lrm_leaf = lrm_leaf_phi.as_basic_value().into_pointer_value();
        // RC-dec the removed element's data_ptr
        let lrm_leaf2_i8 = self
            .builder
            .build_pointer_cast(lrm_leaf, ptr, "leaf2_i8")
            .map_err(llvm_err)?;
        let lrm_eb = unsafe {
            self.builder
                .build_gep(i8, lrm_leaf2_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)
        }?;
        let lrm_rm_ep = unsafe {
            self.builder
                .build_gep(self.string_type, lrm_eb, &[lrm_idx], "rm_ep")
                .map_err(llvm_err)
        }?;
        let lrm_rm_ev = self
            .builder
            .build_load(self.string_type, lrm_rm_ep, "rm_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let lrm_rm_ed = self
            .builder
            .build_extract_value(lrm_rm_ev, 1, "rm_ed")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lrm_rc_dec_fn = self.module.get_function("action_rc_dec").unwrap();
        let _ = self
            .builder
            .build_call(lrm_rc_dec_fn, &[lrm_rm_ed.into()], "")
            .map_err(llvm_err)?;
        // Shift elements [idx+1..count-1] left by 1
        let lrm_si_val = self
            .builder
            .build_int_add(lrm_idx, oner, "si_start")
            .map_err(llvm_err)?;
        let lrm_si = self.builder.build_alloca(i64, "si").map_err(llvm_err)?;
        self.builder
            .build_store(lrm_si, lrm_si_val)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lrm_h0_shift_loop);
        self.builder.position_at_end(lrm_h0_shift_loop);
        let lrm_siv = self
            .builder
            .build_load(i64, lrm_si, "siv")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_si_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrm_siv, lrm_count, "si_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lrm_si_cond, lrm_h0_shift_body, lrm_h0_done);
        self.builder.position_at_end(lrm_h0_shift_body);
        let lrm_src = unsafe {
            self.builder
                .build_gep(self.string_type, lrm_eb, &[lrm_siv], "src")
                .map_err(llvm_err)
        }?;
        let lrm_sv = self
            .builder
            .build_load(self.string_type, lrm_src, "sv")
            .map_err(llvm_err)?;
        let lrm_siv_minus1 = self
            .builder
            .build_int_sub(lrm_siv, oner, "siv_m1")
            .map_err(llvm_err)?;
        let lrm_dst = unsafe {
            self.builder
                .build_gep(self.string_type, lrm_eb, &[lrm_siv_minus1], "dst")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(lrm_dst, lrm_sv)
            .map_err(llvm_err)?;
        let lrm_siv_plus1 = self
            .builder
            .build_int_add(lrm_siv, oner, "siv_p1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lrm_si, lrm_siv_plus1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lrm_h0_shift_loop);
        // Decrement count and return
        self.builder.position_at_end(lrm_h0_done);
        let lrm_new_count = self
            .builder
            .build_int_sub(lrm_count, oner, "new_count")
            .map_err(llvm_err)?;
        let lrm_new_count_i32 = self
            .builder
            .build_int_truncate(lrm_new_count, i32, "new_count_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lrm_leaf2_i8, lrm_new_count_i32)
            .map_err(llvm_err)?;
        let lrm_new_total = self
            .builder
            .build_int_sub(lrm_total_len, oner, "new_total")
            .map_err(llvm_err)?;
        let undef_rem = self.list_type.get_undef();
        let lrm_r1 = self
            .builder
            .build_insert_value(undef_rem, lrm_leaf, 0, "r1")
            .map_err(llvm_err)?;
        let lrm_r2 = self
            .builder
            .build_insert_value(lrm_r1, lrm_new_total, 1, "r2")
            .map_err(llvm_err)?;
        let lrm_r3 = self
            .builder
            .build_insert_value(lrm_r2, zr, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lrm_r3));
        // Empty: return original list unchanged
        self.builder.position_at_end(lrm_empty_bb);
        let _ = self.builder.build_return(Some(&lrm_list));
        // === h>0: take+drop+concat ===
        self.builder.position_at_end(lrm_hgt0);
        let lrm_len2 = self
            .builder
            .build_extract_value(lrm_list, 1, "lrm_len2")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_len_zero2 = self
            .builder
            .build_int_compare(IntPredicate::EQ, lrm_len2, zr, "len_zero2")
            .map_err(llvm_err)?;
        let lrm_hgt0_empty = self.context.append_basic_block(lrm_fn, "hgt0_empty");
        let lrm_hgt0_body = self.context.append_basic_block(lrm_fn, "hgt0_body");
        let _ = self
            .builder
            .build_conditional_branch(lrm_len_zero2, lrm_hgt0_empty, lrm_hgt0_body);
        self.builder.position_at_end(lrm_hgt0_empty);
        let _ = self.builder.build_return(Some(&lrm_list));
        self.builder.position_at_end(lrm_hgt0_body);
        let lrm_last2 = self
            .builder
            .build_int_sub(lrm_len2, oner, "last2")
            .map_err(llvm_err)?;
        let lrm_idx2_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, lrm_index, zr, "idx2_neg")
            .map_err(llvm_err)?;
        let lrm_idx2a = self
            .builder
            .build_select(lrm_idx2_neg, zr, lrm_index, "idx2a")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_idx2_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, lrm_idx2a, lrm_last2, "idx2_gt")
            .map_err(llvm_err)?;
        let lrm_idx2 = self
            .builder
            .build_select(lrm_idx2_gt, lrm_last2, lrm_idx2a, "idx2")
            .map_err(llvm_err)?
            .into_int_value();
        let lrm_take_fn = self.module.get_function("action_list_take").unwrap();
        let lrm_drop_fn = self.module.get_function("action_list_drop").unwrap();
        let lrm_concat_fn = self.module.get_function("action_list_concat").unwrap();
        let lrm_left = self
            .builder
            .build_call(lrm_take_fn, &[lrm_list.into(), lrm_idx2.into()], "left")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lrm_idx2p1 = self
            .builder
            .build_int_add(lrm_idx2, oner, "idx2p1")
            .map_err(llvm_err)?;
        let lrm_right = self
            .builder
            .build_call(lrm_drop_fn, &[lrm_list.into(), lrm_idx2p1.into()], "right")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lrm_result = self
            .builder
            .build_call(
                lrm_concat_fn,
                &[lrm_left.into(), lrm_right.into()],
                "result",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lrm_result));

        // ---- action_string_split_lines({i64, ptr}) -> {ptr, i64, i64} ----
        let sl_fn = self.module.add_function(
            "action_string_split_lines",
            self.list_type.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(sl_fn, "entry");
        self.builder.position_at_end(entry);
        let sl_s = sl_fn.get_first_param().unwrap().into_struct_value();
        let sl_len = self
            .builder
            .build_extract_value(sl_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_ptr = self
            .builder
            .build_extract_value(sl_s, 1, "sptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc4 = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let sl_list_init = cc4.try_as_basic_value().unwrap_basic().into_struct_value();
        // Use alloca to accumulate list across loop iterations
        let sl_list_alloc = self
            .builder
            .build_alloca(self.list_type, "list_acc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_list_alloc, sl_list_init)
            .map_err(llvm_err)?;
        // Scan through string, splitting on '\n'
        let sl_start_alloc = self.builder.build_alloca(i64, "start").map_err(llvm_err)?;
        let sl_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(sl_start_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sl_loop = self.context.append_basic_block(sl_fn, "loop");
        let sl_body_bb = self.context.append_basic_block(sl_fn, "body");
        let sl_done = self.context.append_basic_block(sl_fn, "done");
        let _ = self.builder.build_unconditional_branch(sl_loop);
        self.builder.position_at_end(sl_loop);
        let sl_i = self
            .builder
            .build_load(i64, sl_i_alloc, "sl_i")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_cond = self
            .builder
            .build_int_compare(IntPredicate::SLE, sl_i, sl_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sl_cond, sl_body_bb, sl_done);
        self.builder.position_at_end(sl_body_bb);
        // Check if at end or char is '\n'
        let sl_at_end = self
            .builder
            .build_int_compare(IntPredicate::EQ, sl_i, sl_len, "atend")
            .map_err(llvm_err)?;
        let sl_cp = unsafe {
            self.builder
                .build_gep(i8, sl_ptr, &[sl_i], "cp")
                .map_err(llvm_err)
        }?;
        let sl_c = self
            .builder
            .build_load(i8, sl_cp, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_is_nl = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sl_c,
                i8.const_int(b'\n' as u64, false),
                "isnl",
            )
            .map_err(llvm_err)?;
        let sl_cr = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sl_c,
                i8.const_int(b'\r' as u64, false),
                "iscr",
            )
            .map_err(llvm_err)?;
        let sl_split = self
            .builder
            .build_or(
                sl_at_end,
                self.builder
                    .build_or(sl_is_nl, sl_cr, "")
                    .map_err(llvm_err)?,
                "split",
            )
            .map_err(llvm_err)?;
        let sl_cont = self.context.append_basic_block(sl_fn, "cont");
        let sl_extract = self.context.append_basic_block(sl_fn, "extract");
        let _ = self
            .builder
            .build_conditional_branch(sl_split, sl_extract, sl_cont);
        // Extract line from start to i
        self.builder.position_at_end(sl_extract);
        let sl_start = self
            .builder
            .build_load(i64, sl_start_alloc, "slstart")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_seg_len = self
            .builder
            .build_int_sub(sl_i, sl_start, "seg_len")
            .map_err(llvm_err)?;
        let sl_seg_data = unsafe {
            self.builder
                .build_gep(i8, sl_ptr, &[sl_start], "segp")
                .map_err(llvm_err)
        }?;
        // Skip \r if next char is \n
        let sl_next_i = self
            .builder
            .build_int_add(sl_i, i64.const_int(1, false), "nexti")
            .map_err(llvm_err)?;
        // Create string for this segment: malloc + memcpy
        let sl_salloc = self
            .builder
            .build_call(malloc_rc_fn, &[sl_seg_len.into()], "seg")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for newly allocated segment
        let sl_sa_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(sl_salloc, i64, "sl_sa_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "sl_sa_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(sl_sa_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sl_salloc.into(), sl_seg_data.into(), sl_seg_len.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sl_fat = self.string_type.get_undef();
        let sl_fat_tag = self
            .builder
            .build_insert_value(sl_fat, self.i64_ty().const_int(1, false), 0, "tag")
            .map_err(llvm_err)?;
        let sl_fat_val = self
            .builder
            .build_insert_value(sl_fat_tag, sl_salloc, 1, "data")
            .map_err(llvm_err)?;
        let sl_cur_list = self
            .builder
            .build_load(self.list_type, sl_list_alloc, "cur_list")
            .map_err(llvm_err)?
            .into_struct_value();
        let sl_push_cc = self.call_rt(
            "action_list_push",
            &[sl_cur_list.into(), sl_fat_val.as_basic_value_enum().into()],
        )?;
        let sl_new_list = sl_push_cc.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(sl_list_alloc, sl_new_list)
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_start_alloc, sl_next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sl_cont);
        // Continue scanning
        self.builder.position_at_end(sl_cont);
        let sl_i2 = self
            .builder
            .build_load(i64, sl_i_alloc, "i2")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_i_next = self
            .builder
            .build_int_add(sl_i2, i64.const_int(1, false), "inext")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_i_alloc, sl_i_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sl_loop);
        self.builder.position_at_end(sl_done);
        let sl_result = self
            .builder
            .build_load(self.list_type, sl_list_alloc, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sl_result));

        // ---- action_string_index_of({i64, ptr}, {i64, ptr}) -> i64 (returns -1 if not found) ----
        let sio_fn = self.module.add_function(
            "action_string_index_of",
            i64.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(sio_fn, "entry");
        self.builder.position_at_end(entry);
        let sio_hay = sio_fn.get_first_param().unwrap().into_struct_value();
        let sio_nee = sio_fn.get_nth_param(1).unwrap().into_struct_value();
        let sio_hlen = self
            .builder
            .build_extract_value(sio_hay, 0, "hlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sio_hptr = self
            .builder
            .build_extract_value(sio_hay, 1, "hptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sio_nlen = self
            .builder
            .build_extract_value(sio_nee, 0, "nlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sio_nptr = self
            .builder
            .build_extract_value(sio_nee, 1, "nptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        // If needle empty, return 0
        let sio_nempty = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sio_nlen,
                i64.const_int(0, false),
                "nempty",
            )
            .map_err(llvm_err)?;
        let sio_nok = self
            .builder
            .build_int_compare(IntPredicate::SLE, sio_nlen, sio_hlen, "nok")
            .map_err(llvm_err)?;
        let _sio_can = self
            .builder
            .build_and(
                sio_nok,
                self.builder.build_not(sio_nempty, "").map_err(llvm_err)?,
                "",
            )
            .map_err(llvm_err)?;
        let sio_max = self
            .builder
            .build_int_sub(sio_hlen, sio_nlen, "max")
            .map_err(llvm_err)?;
        // Outer loop
        let sio_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(sio_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sio_oloop = self.context.append_basic_block(sio_fn, "oloop");
        let sio_obody = self.context.append_basic_block(sio_fn, "obody");
        let sio_notfound = self.context.append_basic_block(sio_fn, "notfound");
        let _ = self.builder.build_unconditional_branch(sio_oloop);
        self.builder.position_at_end(sio_oloop);
        let sio_iv = self
            .builder
            .build_load(i64, sio_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let sio_cond = self
            .builder
            .build_int_compare(IntPredicate::SLE, sio_iv, sio_max, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sio_cond, sio_obody, sio_notfound);
        self.builder.position_at_end(sio_obody);
        let sio_hp = unsafe {
            self.builder
                .build_gep(i8, sio_hptr, &[sio_iv], "hp")
                .map_err(llvm_err)
        }?;
        let sio_eq = self
            .builder
            .build_call(
                memcmp_fn,
                &[sio_hp.into(), sio_nptr.into(), sio_nlen.into()],
                "eq",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let sio_match = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sio_eq,
                self.i32_ty().const_int(0, false),
                "match",
            )
            .map_err(llvm_err)?;
        let sio_match_bb = self.context.append_basic_block(sio_fn, "match");
        let sio_next_bb = self.context.append_basic_block(sio_fn, "next");
        let _ = self
            .builder
            .build_conditional_branch(sio_match, sio_match_bb, sio_next_bb);
        self.builder.position_at_end(sio_match_bb);
        let _ = self.builder.build_return(Some(&sio_iv));
        self.builder.position_at_end(sio_next_bb);
        let sio_next_i = self
            .builder
            .build_int_add(sio_iv, i64.const_int(1, false), "nexti")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sio_i, sio_next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sio_oloop);
        self.builder.position_at_end(sio_notfound);
        let _ = self
            .builder
            .build_return(Some(&i64.const_int(-1i64 as u64, true)));

        // ---- action_list_flatten({ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Converts a ConcatNode DAG into a flat B-tree list.
        // Recursively flattens nested ConcatNode children before merging materialized subtrees.
        let fl_fn = self.module.get_function("action_list_flatten").unwrap();
        let fl_entry = self.context.append_basic_block(fl_fn, "entry");
        let fl_not_concat = self.context.append_basic_block(fl_fn, "not_concat");
        let fl_concat = self.context.append_basic_block(fl_fn, "concat");
        self.builder.position_at_end(fl_entry);
        let fl_input = fl_fn.get_first_param().unwrap().into_struct_value();
        let fl_height = self
            .builder
            .build_extract_value(fl_input, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                fl_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fl_is_concat, fl_concat, fl_not_concat);
        // Not concat: return input unchanged
        self.builder.position_at_end(fl_not_concat);
        let _ = self.builder.build_return(Some(&fl_input));
        // Concat: recursively flatten nested ConcatNode children, then merge flat subtrees
        self.builder.position_at_end(fl_concat);
        let fl_node = self
            .builder
            .build_extract_value(fl_input, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_node_i8 = self
            .builder
            .build_pointer_cast(fl_node, ptr, "node_i8")
            .map_err(llvm_err)?;
        let fl_left_ptr = unsafe {
            self.builder
                .build_gep(i8, fl_node_i8, &[i64.const_int(16, false)], "left_ptr")
                .map_err(llvm_err)
        }?;
        let fl_left = self
            .builder
            .build_load(self.list_type, fl_left_ptr, "left")
            .map_err(llvm_err)?
            .into_struct_value();
        let fl_left_h = self
            .builder
            .build_extract_value(fl_left, 2, "lh")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_right_ptr = unsafe {
            self.builder
                .build_gep(i8, fl_node_i8, &[i64.const_int(40, false)], "right_ptr")
                .map_err(llvm_err)
        }?;
        let fl_right = self
            .builder
            .build_load(self.list_type, fl_right_ptr, "right")
            .map_err(llvm_err)?
            .into_struct_value();
        let fl_right_h = self
            .builder
            .build_extract_value(fl_right, 2, "rh")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_neg1 = i64.const_int(-1i64 as u64, true);

        let fl_l_is_c = self
            .builder
            .build_int_compare(IntPredicate::EQ, fl_left_h, fl_neg1, "l_is_c")
            .map_err(llvm_err)?;
        let fl_l_flat_bb = self.context.append_basic_block(fl_fn, "l_flat");
        let fl_l_done_bb = self.context.append_basic_block(fl_fn, "l_done");
        let _ = self
            .builder
            .build_conditional_branch(fl_l_is_c, fl_l_flat_bb, fl_l_done_bb);
        self.builder.position_at_end(fl_l_flat_bb);
        let fl_l_flat_v = self
            .builder
            .build_call(fl_fn, &[fl_left.into()], "l_flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let _ = self.builder.build_unconditional_branch(fl_l_done_bb);
        self.builder.position_at_end(fl_l_done_bb);
        let fl_l_phi = self
            .builder
            .build_phi(self.list_type, "l_phi")
            .map_err(llvm_err)?;
        fl_l_phi.add_incoming(&[(&fl_left, fl_concat)]);
        fl_l_phi.add_incoming(&[(&fl_l_flat_v, fl_l_flat_bb)]);
        let fl_l_final = fl_l_phi.as_basic_value().into_struct_value();
        let fl_l_node = self
            .builder
            .build_extract_value(fl_l_final, 0, "l_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_l_h = self
            .builder
            .build_extract_value(fl_l_final, 2, "l_fh")
            .map_err(llvm_err)?
            .into_int_value();

        let fl_r_is_c = self
            .builder
            .build_int_compare(IntPredicate::EQ, fl_right_h, fl_neg1, "r_is_c")
            .map_err(llvm_err)?;
        let fl_r_flat_bb = self.context.append_basic_block(fl_fn, "r_flat");
        let fl_r_done_bb = self.context.append_basic_block(fl_fn, "r_done");
        let _ = self
            .builder
            .build_conditional_branch(fl_r_is_c, fl_r_flat_bb, fl_r_done_bb);
        self.builder.position_at_end(fl_r_flat_bb);
        let fl_r_flat_v = self
            .builder
            .build_call(fl_fn, &[fl_right.into()], "r_flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let _ = self.builder.build_unconditional_branch(fl_r_done_bb);
        self.builder.position_at_end(fl_r_done_bb);
        let fl_r_phi = self
            .builder
            .build_phi(self.list_type, "r_phi")
            .map_err(llvm_err)?;
        fl_r_phi.add_incoming(&[(&fl_right, fl_l_done_bb)]);
        fl_r_phi.add_incoming(&[(&fl_r_flat_v, fl_r_flat_bb)]);
        let fl_r_final = fl_r_phi.as_basic_value().into_struct_value();
        let fl_r_node = self
            .builder
            .build_extract_value(fl_r_final, 0, "r_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_r_h = self
            .builder
            .build_extract_value(fl_r_final, 2, "r_fh")
            .map_err(llvm_err)?
            .into_int_value();

        let fl_empty_cc = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let fl_empty = fl_empty_cc.try_as_basic_value().unwrap_basic();
        let fl_acc = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(fl_acc, fl_empty)
            .map_err(llvm_err)?;
        let fl_ps_fn = self
            .module
            .get_function("action_list_push_subtree")
            .unwrap();
        let _ = self
            .builder
            .build_call(
                fl_ps_fn,
                &[fl_acc.into(), fl_l_node.into(), fl_l_h.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                fl_ps_fn,
                &[fl_acc.into(), fl_r_node.into(), fl_r_h.into()],
                "",
            )
            .map_err(llvm_err)?;
        let fl_result = self
            .builder
            .build_load(self.list_type, fl_acc, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fl_result));

        // ---- action_list_push_leaf(ptr acc, ptr leaf) -> void ----
        // Bulk-push all elements from a leaf into the accumulator.
        // Uses memcpy+rc_inc when accumulator's last leaf has room; falls back to per-element push.
        let pl_fn = self.module.get_function("action_list_push_leaf").unwrap();
        let pl_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let pl_rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let pl_push_fn = self.module.get_function("action_list_push").unwrap();
        let string_ty = self.string_type;
        let leaf_ty = self.leaf_type;
        let pl_entry = self.context.append_basic_block(pl_fn, "entry");
        let pl_loop_bb = self.context.append_basic_block(pl_fn, "lp");
        let pl_body_bb = self.context.append_basic_block(pl_fn, "body");
        let pl_fb_bb = self.context.append_basic_block(pl_fn, "fb");
        let pl_bulk_bb = self.context.append_basic_block(pl_fn, "bulk");
        let pl_fallback_bb = self.context.append_basic_block(pl_fn, "fallback");
        let pl_memcpy_bb = self.context.append_basic_block(pl_fn, "memcpy");
        let pl_rc_loop = self.context.append_basic_block(pl_fn, "rc_lp");
        let pl_rc_body = self.context.append_basic_block(pl_fn, "rc_body");
        let pl_rc_done = self.context.append_basic_block(pl_fn, "rc_done");
        let pl_done = self.context.append_basic_block(pl_fn, "done");
        self.builder.position_at_end(pl_entry);
        let pl_acc = pl_fn.get_first_param().unwrap().into_pointer_value();
        let pl_leaf = pl_fn.get_nth_param(1).unwrap().into_pointer_value();
        let pl_leaf_i8 = self
            .builder
            .build_pointer_cast(pl_leaf, ptr, "lf_i8")
            .map_err(llvm_err)?;
        let pl_leaf_cnt_r = self
            .builder
            .build_load(i32, pl_leaf_i8, "lf_cnt")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_leaf_cnt = self
            .builder
            .build_int_z_extend(pl_leaf_cnt_r, i64, "cnt64")
            .map_err(llvm_err)?;
        let pl_pos = self.builder.build_alloca(i64, "pos").map_err(llvm_err)?;
        let _ = self.builder.build_store(pl_pos, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_loop_bb);
        // Loop header
        self.builder.position_at_end(pl_loop_bb);
        let pl_pos_v = self
            .builder
            .build_load(i64, pl_pos, "pos_v")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_cmp = self
            .builder
            .build_int_compare(IntPredicate::SLT, pl_pos_v, pl_leaf_cnt, "cmp")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pl_cmp, pl_body_bb, pl_fb_bb);
        // Loop body: try to bulk-push remaining elements
        self.builder.position_at_end(pl_body_bb);
        let pl_cur = self
            .builder
            .build_load(self.list_type, pl_acc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let pl_cur_node = self
            .builder
            .build_extract_value(pl_cur, 0, "cur_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pl_cur_total = self
            .builder
            .build_extract_value(pl_cur, 1, "cur_total")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_cur_h = self
            .builder
            .build_extract_value(pl_cur, 2, "cur_h")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_cur_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, pl_cur_h, zero, "cur_h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pl_cur_h0, pl_bulk_bb, pl_fallback_bb);
        // Bulk path: result is h=0 (single leaf)
        self.builder.position_at_end(pl_bulk_bb);
        let pl_lst_lf = pl_cur_node;
        let pl_lst_i8 = self
            .builder
            .build_pointer_cast(pl_lst_lf, ptr, "lst_i8")
            .map_err(llvm_err)?;
        let pl_lst_cnt_r = self
            .builder
            .build_load(i32, pl_lst_i8, "lst_cnt")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_lst_cnt = self
            .builder
            .build_int_z_extend(pl_lst_cnt_r, i64, "lst_cnt64")
            .map_err(llvm_err)?;
        let pl_room = self
            .builder
            .build_int_sub(i64.const_int(64, false), pl_lst_cnt, "room")
            .map_err(llvm_err)?;
        let pl_rem = self
            .builder
            .build_int_sub(pl_leaf_cnt, pl_pos_v, "rem")
            .map_err(llvm_err)?;
        let pl_batch = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, pl_rem, pl_room, "use_rem")
                    .map_err(llvm_err)?,
                pl_rem,
                pl_room,
                "batch",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let pl_batch_z = self
            .builder
            .build_int_compare(IntPredicate::EQ, pl_batch, zero, "batch_z")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pl_batch_z, pl_fallback_bb, pl_memcpy_bb);
        // memcpy block
        self.builder.position_at_end(pl_memcpy_bb);
        let pl_lf_int = self
            .builder
            .build_ptr_to_int(pl_lst_lf, i64, "lf_int")
            .map_err(llvm_err)?;
        let pl_rc_a = self
            .builder
            .build_int_sub(pl_lf_int, i64.const_int(8, false), "rc_a")
            .map_err(llvm_err)?;
        let pl_rc_p = self
            .builder
            .build_int_to_ptr(pl_rc_a, ptr, "rc_p")
            .map_err(llvm_err)?;
        let pl_rc_v = self
            .builder
            .build_load(i64, pl_rc_p, "rc_v")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_need_cow = self
            .builder
            .build_int_compare(IntPredicate::SGT, pl_rc_v, one, "need_cow")
            .map_err(llvm_err)?;
        let pl_leaf_sz = leaf_ty.size_of().ok_or("leaf size")?;
        let pl_cow_lf = self
            .builder
            .build_call(malloc_rc_fn, &[pl_leaf_sz.into()], "cow_lf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                pl_memcpy_fn,
                &[pl_cow_lf.into(), pl_lst_lf.into(), pl_leaf_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        let pl_use_lf = self
            .builder
            .build_select(pl_need_cow, pl_cow_lf, pl_lst_lf, "use_lf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pl_use_lf_i8 = self
            .builder
            .build_pointer_cast(pl_use_lf, ptr, "use_i8")
            .map_err(llvm_err)?;
        let pl_dst_off = self
            .builder
            .build_int_add(
                i64.const_int(8, false),
                self.builder
                    .build_int_mul(pl_lst_cnt, i64.const_int(16, false), "dstoff_mul")
                    .map_err(llvm_err)?,
                "dstoff",
            )
            .map_err(llvm_err)?;
        let pl_dst = unsafe {
            self.builder
                .build_gep(i8, pl_use_lf_i8, &[pl_dst_off], "dst")
                .map_err(llvm_err)
        }?;
        let pl_src_off = self
            .builder
            .build_int_add(
                i64.const_int(8, false),
                self.builder
                    .build_int_mul(pl_pos_v, i64.const_int(16, false), "srcoff_mul")
                    .map_err(llvm_err)?,
                "srcoff",
            )
            .map_err(llvm_err)?;
        let pl_src = unsafe {
            self.builder
                .build_gep(i8, pl_leaf_i8, &[pl_src_off], "src")
                .map_err(llvm_err)
        }?;
        let pl_cpy_sz = self
            .builder
            .build_int_mul(pl_batch, i64.const_int(16, false), "cpy_sz")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                pl_memcpy_fn,
                &[pl_dst.into(), pl_src.into(), pl_cpy_sz.into()],
                "",
            )
            .map_err(llvm_err)?;
        // rc_inc each copied element
        let pl_rc_i = self.builder.build_alloca(i64, "rc_i").map_err(llvm_err)?;
        let _ = self.builder.build_store(pl_rc_i, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_rc_loop);
        self.builder.position_at_end(pl_rc_loop);
        let pl_rc_iv = self
            .builder
            .build_load(i64, pl_rc_i, "rc_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let pl_rc_cmp = self
            .builder
            .build_int_compare(IntPredicate::SLT, pl_rc_iv, pl_batch, "rc_cmp")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pl_rc_cmp, pl_rc_body, pl_rc_done);
        self.builder.position_at_end(pl_rc_body);
        let pl_el_off = self
            .builder
            .build_int_add(
                i64.const_int(8, false),
                self.builder
                    .build_int_mul(
                        self.builder
                            .build_int_add(pl_pos_v, pl_rc_iv, "el_idx")
                            .map_err(llvm_err)?,
                        i64.const_int(16, false),
                        "el_off_mul",
                    )
                    .map_err(llvm_err)?,
                "el_off",
            )
            .map_err(llvm_err)?;
        let pl_el_p = unsafe {
            self.builder
                .build_gep(i8, pl_leaf_i8, &[pl_el_off], "el_p")
                .map_err(llvm_err)
        }?;
        let pl_el_ev = self
            .builder
            .build_load(string_ty, pl_el_p, "el_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let pl_el_dp = self
            .builder
            .build_extract_value(pl_el_ev, 1, "el_dp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(pl_rc_inc_fn, &[pl_el_dp.into()], "")
            .map_err(llvm_err)?;
        let pl_rc_next = self
            .builder
            .build_int_add(pl_rc_iv, one, "rc_next")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pl_rc_i, pl_rc_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_rc_loop);
        // Update leaf count and accumulator
        self.builder.position_at_end(pl_rc_done);
        let pl_new_lc = self
            .builder
            .build_int_add(pl_lst_cnt, pl_batch, "new_lc")
            .map_err(llvm_err)?;
        let pl_new_lc_i32 = self
            .builder
            .build_int_truncate(pl_new_lc, i32, "new_lc_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pl_use_lf_i8, pl_new_lc_i32)
            .map_err(llvm_err)?;
        let pl_new_total = self
            .builder
            .build_int_add(pl_cur_total, pl_batch, "new_total")
            .map_err(llvm_err)?;
        let pl_undef = self.list_type.get_undef();
        let pl_v1 = self
            .builder
            .build_insert_value(pl_undef, pl_use_lf, 0, "v1")
            .map_err(llvm_err)?;
        let pl_v2 = self
            .builder
            .build_insert_value(pl_v1, pl_new_total, 1, "v2")
            .map_err(llvm_err)?;
        let pl_v3 = self
            .builder
            .build_insert_value(pl_v2, zero, 2, "v3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(pl_acc, pl_v3).map_err(llvm_err)?;
        let pl_nxt = self
            .builder
            .build_int_add(pl_pos_v, pl_batch, "nxt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(pl_pos, pl_nxt).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_loop_bb);
        // Fallback: push one element via action_list_push
        self.builder.position_at_end(pl_fallback_bb);
        let pl_fb_off = self
            .builder
            .build_int_add(
                i64.const_int(8, false),
                self.builder
                    .build_int_mul(pl_pos_v, i64.const_int(16, false), "fb_off_m")
                    .map_err(llvm_err)?,
                "fb_off",
            )
            .map_err(llvm_err)?;
        let pl_fb_ep = unsafe {
            self.builder
                .build_gep(i8, pl_leaf_i8, &[pl_fb_off], "fb_ep")
                .map_err(llvm_err)
        }?;
        let pl_fb_ev = self
            .builder
            .build_load(string_ty, pl_fb_ep, "fb_ev")
            .map_err(llvm_err)?;
        let pl_fb_ed = self
            .builder
            .build_extract_value(pl_fb_ev.into_struct_value(), 1, "fb_ed")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(pl_rc_inc_fn, &[pl_fb_ed.into()], "")
            .map_err(llvm_err)?;
        let pl_fb_cur = self
            .builder
            .build_load(self.list_type, pl_acc, "fb_cur")
            .map_err(llvm_err)?;
        let pl_fb_new = self
            .builder
            .build_call(
                pl_push_fn,
                &[pl_fb_cur.into(), pl_fb_ev.as_basic_value_enum().into()],
                "fb_new",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self
            .builder
            .build_store(pl_acc, pl_fb_new)
            .map_err(llvm_err)?;
        let pl_fb_next = self
            .builder
            .build_int_add(pl_pos_v, one, "fb_next")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(pl_pos, pl_fb_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pl_loop_bb);
        // Final branch
        self.builder.position_at_end(pl_fb_bb);
        let _ = self.builder.build_unconditional_branch(pl_done);
        self.builder.position_at_end(pl_done);
        let _ = self.builder.build_return(None);

        // ---- action_list_push_subtree(ptr acc, ptr node, i64 height) -> void ----
        // Pushes all elements from a materialized B-tree subtree (height >= 0) into acc.
        // ConcatNode (height == -1) must be flattened first — delegate to action_list_flatten.
        let ps_fn = self
            .module
            .get_function("action_list_push_subtree")
            .unwrap();
        let child_entry_ty = self.child_entry_type;
        let ps_entry = self.context.append_basic_block(ps_fn, "entry");
        let ps_flatten_push = self.context.append_basic_block(ps_fn, "flatten_push");
        let ps_h0_leaf = self.context.append_basic_block(ps_fn, "h0_leaf");
        let ps_h1_intl = self.context.append_basic_block(ps_fn, "h1_intl");
        let ps_hgt1_recurse = self.context.append_basic_block(ps_fn, "hgt1");
        let ps_done = self.context.append_basic_block(ps_fn, "done");
        self.builder.position_at_end(ps_entry);
        let ps_acc = ps_fn.get_first_param().unwrap().into_pointer_value();
        let ps_node = ps_fn.get_nth_param(1).unwrap().into_pointer_value();
        let ps_height = ps_fn.get_nth_param(2).unwrap().into_int_value();
        let ps_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ps_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let ps_not_concat = self.context.append_basic_block(ps_fn, "not_concat");
        let _ = self
            .builder
            .build_conditional_branch(ps_is_concat, ps_flatten_push, ps_not_concat);
        // ConcatNode: flatten to materialized tree, then push (never walk concat in-place)
        self.builder.position_at_end(ps_flatten_push);
        let ps_cn_i8 = self
            .builder
            .build_pointer_cast(ps_node, ptr, "cn_i8")
            .map_err(llvm_err)?;
        let ps_len_p = unsafe {
            self.builder
                .build_gep(i64, ps_cn_i8, &[i64.const_int(1, false)], "len_p")
                .map_err(llvm_err)
        }?;
        let ps_len_v = self
            .builder
            .build_load(i64, ps_len_p, "len_v")
            .map_err(llvm_err)?;
        let ps_neg1 = i64.const_int(-1i64 as u64, true);
        let ps_cn_list_undef = self.list_type.get_undef();
        let ps_cn_list_v = self
            .builder
            .build_insert_value(ps_cn_list_undef, ps_node, 0, "cn_l0")
            .map_err(llvm_err)?;
        let ps_cn_list_v = self
            .builder
            .build_insert_value(ps_cn_list_v, ps_len_v, 1, "cn_l1")
            .map_err(llvm_err)?;
        let ps_cn_list = self
            .builder
            .build_insert_value(ps_cn_list_v, ps_neg1, 2, "cn_l2")
            .map_err(llvm_err)?
            .into_struct_value();
        let ps_flat_fn = self.module.get_function("action_list_flatten").unwrap();
        let ps_flat_v = self
            .builder
            .build_call(ps_flat_fn, &[ps_cn_list.into()], "flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let ps_flat_node = self
            .builder
            .build_extract_value(ps_flat_v, 0, "flat_n")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ps_flat_h = self
            .builder
            .build_extract_value(ps_flat_v, 2, "flat_h")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(
                ps_fn,
                &[ps_acc.into(), ps_flat_node.into(), ps_flat_h.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ps_done);
        // Three-way dispatch: h==0, h==1, h>=2
        self.builder.position_at_end(ps_not_concat);
        let ps_is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, ps_height, zero, "is_h0")
            .map_err(llvm_err)?;
        let ps_not_h0 = self.context.append_basic_block(ps_fn, "not_h0");
        let _ = self
            .builder
            .build_conditional_branch(ps_is_h0, ps_h0_leaf, ps_not_h0);
        self.builder.position_at_end(ps_not_h0);
        let ps_is_h1 = self
            .builder
            .build_int_compare(IntPredicate::EQ, ps_height, one, "is_h1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ps_is_h1, ps_h1_intl, ps_hgt1_recurse);
        // === ps_h0_leaf: delegate to action_list_push_leaf ===
        self.builder.position_at_end(ps_h0_leaf);
        let ps_leaf_fn = self.module.get_function("action_list_push_leaf").unwrap();
        let _ = self
            .builder
            .build_call(ps_leaf_fn, &[ps_acc.into(), ps_node.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ps_done);
        // === ps_h1_intl: internal node with leaf children ===
        self.builder.position_at_end(ps_h1_intl);
        let ps_intl_i8 = self
            .builder
            .build_pointer_cast(ps_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let ps_intl_cnt_r = self
            .builder
            .build_load(i32, ps_intl_i8, "intl_cnt")
            .map_err(llvm_err)?
            .into_int_value();
        let ps_intl_cnt = self
            .builder
            .build_int_z_extend(ps_intl_cnt_r, i64, "intl_cnt64")
            .map_err(llvm_err)?;
        let ps_ci = self.builder.build_alloca(i64, "ci").map_err(llvm_err)?;
        let _ = self.builder.build_store(ps_ci, zero).map_err(llvm_err)?;
        let ps_cloop = self.context.append_basic_block(ps_fn, "clp");
        let ps_cbody = self.context.append_basic_block(ps_fn, "cbody");
        let ps_cdone = self.context.append_basic_block(ps_fn, "cdone");
        let _ = self.builder.build_unconditional_branch(ps_cloop);
        self.builder.position_at_end(ps_cloop);
        let ps_civ = self
            .builder
            .build_load(i64, ps_ci, "civ")
            .map_err(llvm_err)?
            .into_int_value();
        let ps_ccmp = self
            .builder
            .build_int_compare(IntPredicate::SLT, ps_civ, ps_intl_cnt, "ccmp")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ps_ccmp, ps_cbody, ps_cdone);
        self.builder.position_at_end(ps_cbody);
        // Load child entry: node+16 + ci*16
        let ps_ce_off = self
            .builder
            .build_int_add(
                i64.const_int(16, false),
                self.builder
                    .build_int_mul(ps_civ, i64.const_int(16, false), "ce_off_m")
                    .map_err(llvm_err)?,
                "ce_off",
            )
            .map_err(llvm_err)?;
        let ps_ce_p = unsafe {
            self.builder
                .build_gep(i8, ps_intl_i8, &[ps_ce_off], "ce_p")
                .map_err(llvm_err)
        }?;
        let ps_ce = self
            .builder
            .build_load(child_entry_ty, ps_ce_p, "ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let ps_child = self
            .builder
            .build_extract_value(ps_ce, 0, "child")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Recursively push this child (it's a leaf, h=0)
        let _ = self
            .builder
            .build_call(ps_fn, &[ps_acc.into(), ps_child.into(), zero.into()], "")
            .map_err(llvm_err)?;
        let ps_cnext = self
            .builder
            .build_int_add(ps_civ, one, "cnext")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(ps_ci, ps_cnext)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ps_cloop);
        self.builder.position_at_end(ps_cdone);
        let _ = self.builder.build_unconditional_branch(ps_done);
        // === ps_hgt1_recurse: deep internal node — recurse into children ===
        self.builder.position_at_end(ps_hgt1_recurse);
        let ps_d_intl_i8 = self
            .builder
            .build_pointer_cast(ps_node, ptr, "dintl_i8")
            .map_err(llvm_err)?;
        let ps_d_cnt_r = self
            .builder
            .build_load(i32, ps_d_intl_i8, "dcnt")
            .map_err(llvm_err)?
            .into_int_value();
        let ps_d_cnt = self
            .builder
            .build_int_z_extend(ps_d_cnt_r, i64, "dcnt64")
            .map_err(llvm_err)?;
        let ps_di = self.builder.build_alloca(i64, "di").map_err(llvm_err)?;
        let _ = self.builder.build_store(ps_di, zero).map_err(llvm_err)?;
        let ps_dloop = self.context.append_basic_block(ps_fn, "dlp");
        let ps_dbody = self.context.append_basic_block(ps_fn, "dbody");
        let ps_ddone = self.context.append_basic_block(ps_fn, "ddone");
        let _ = self.builder.build_unconditional_branch(ps_dloop);
        self.builder.position_at_end(ps_dloop);
        let ps_div = self
            .builder
            .build_load(i64, ps_di, "div")
            .map_err(llvm_err)?
            .into_int_value();
        let ps_dcmp = self
            .builder
            .build_int_compare(IntPredicate::SLT, ps_div, ps_d_cnt, "dcmp")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ps_dcmp, ps_dbody, ps_ddone);
        self.builder.position_at_end(ps_dbody);
        let ps_dce_off = self
            .builder
            .build_int_add(
                i64.const_int(16, false),
                self.builder
                    .build_int_mul(ps_div, i64.const_int(16, false), "dce_off_m")
                    .map_err(llvm_err)?,
                "dce_off",
            )
            .map_err(llvm_err)?;
        let ps_dce_p = unsafe {
            self.builder
                .build_gep(i8, ps_d_intl_i8, &[ps_dce_off], "dce_p")
                .map_err(llvm_err)
        }?;
        let ps_dce = self
            .builder
            .build_load(child_entry_ty, ps_dce_p, "dce")
            .map_err(llvm_err)?
            .into_struct_value();
        let ps_dchild = self
            .builder
            .build_extract_value(ps_dce, 0, "dchild")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ps_dh = self
            .builder
            .build_int_sub(ps_height, one, "dh")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(ps_fn, &[ps_acc.into(), ps_dchild.into(), ps_dh.into()], "")
            .map_err(llvm_err)?;
        let ps_dnext = self
            .builder
            .build_int_add(ps_div, one, "dnext")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(ps_di, ps_dnext)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ps_dloop);
        self.builder.position_at_end(ps_ddone);
        let _ = self.builder.build_unconditional_branch(ps_done);
        // Done: return
        self.builder.position_at_end(ps_done);
        let _ = self.builder.build_return(None);

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
        let sa_r1 = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let sa_r1v = sa_r1.try_as_basic_value().unwrap_basic();
        let sa_a1 = self
            .builder
            .build_alloca(self.list_type, "sa_a1")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_a1, sa_r1v).map_err(llvm_err)?;
        let sa_r2 = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let sa_r2v = sa_r2.try_as_basic_value().unwrap_basic();
        let sa_a2 = self
            .builder
            .build_alloca(self.list_type, "sa_a2")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_a2, sa_r2v).map_err(llvm_err)?;
        let sa_i = self.builder.build_alloca(i64, "sa_i").map_err(llvm_err)?;
        self.builder
            .build_store(sa_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sa_loop = self.context.append_basic_block(sa_fn, "loop");
        let sa_body = self.context.append_basic_block(sa_fn, "body");
        let sa_done = self.context.append_basic_block(sa_fn, "done");
        let _ = self.builder.build_unconditional_branch(sa_loop);
        self.builder.position_at_end(sa_loop);
        let sa_iv = self
            .builder
            .build_load(i64, sa_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let sa_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, sa_iv, sa_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sa_cond, sa_body, sa_done);
        self.builder.position_at_end(sa_body);
        let sa_get_fn = self.module.get_function("action_list_get").unwrap();
        let sa_ev = self
            .builder
            .build_call(sa_get_fn, &[sa_in.into(), sa_iv.into()], "ev")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?
            .into_struct_value();
        let sa_before = self
            .builder
            .build_int_compare(IntPredicate::SLT, sa_iv, sa_idx_safe, "before")
            .map_err(llvm_err)?;
        let sa_l1 = self
            .builder
            .build_load(self.list_type, sa_a1, "l1")
            .map_err(llvm_err)?
            .into_struct_value();
        let sa_l2 = self
            .builder
            .build_load(self.list_type, sa_a2, "l2")
            .map_err(llvm_err)?
            .into_struct_value();
        let sa_ps1 = self.call_rt(
            "action_list_push",
            &[sa_l1.into(), sa_ev.as_basic_value_enum().into()],
        )?;
        let sa_ps2 = self.call_rt(
            "action_list_push",
            &[sa_l2.into(), sa_ev.as_basic_value_enum().into()],
        )?;
        let sa_l1_sel = self
            .builder
            .build_select(
                sa_before,
                sa_ps1.try_as_basic_value().unwrap_basic(),
                sa_l1.into(),
                "l1s",
            )
            .map_err(llvm_err)?;
        let sa_l2_sel = self
            .builder
            .build_select(
                sa_before,
                sa_l2.into(),
                sa_ps2.try_as_basic_value().unwrap_basic(),
                "l2s",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_a1, sa_l1_sel)
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_a2, sa_l2_sel)
            .map_err(llvm_err)?;
        let sa_inc = self
            .builder
            .build_int_add(sa_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_i, sa_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sa_loop);
        self.builder.position_at_end(sa_done);
        // Return as list of 2 lists
        let sa_malloc = self
            .builder
            .build_call(malloc_rc_fn, &[i64.const_int(16, false).into()], "sa_m")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for newly allocated buffer
        let sa_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(sa_malloc, i64, "sa_m_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "sa_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(sa_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        let sa_l1f = self
            .builder
            .build_load(self.list_type, sa_a1, "l1f")
            .map_err(llvm_err)?
            .into_struct_value();
        let sa_fat1 = self.string_type.get_undef();
        let sa_fat1t = self
            .builder
            .build_insert_value(sa_fat1, i64.const_int(6, false), 0, "t1")
            .map_err(llvm_err)?;
        let sa_l1p = self
            .builder
            .build_alloca(self.list_type, "l1p")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_l1p, sa_l1f).map_err(llvm_err)?;
        let sa_fat1v = self
            .builder
            .build_insert_value(sa_fat1t, sa_l1p, 1, "v1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_malloc, sa_fat1v)
            .map_err(llvm_err)?;
        let sa_slot2 = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    sa_malloc,
                    &[i64.const_int(1, false)],
                    "s2",
                )
                .map_err(llvm_err)
        }?;
        let sa_l2f = self
            .builder
            .build_load(self.list_type, sa_a2, "l2f")
            .map_err(llvm_err)?
            .into_struct_value();
        let sa_fat2 = self.string_type.get_undef();
        let sa_fat2t = self
            .builder
            .build_insert_value(sa_fat2, i64.const_int(6, false), 0, "t2")
            .map_err(llvm_err)?;
        let sa_l2p = self
            .builder
            .build_alloca(self.list_type, "l2p")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_l2p, sa_l2f).map_err(llvm_err)?;
        let sa_fat2v = self
            .builder
            .build_insert_value(sa_fat2t, sa_l2p, 1, "v2")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_slot2, sa_fat2v)
            .map_err(llvm_err)?;
        let sa_rt = self.list_type.get_undef();
        let sa_rtd = self
            .builder
            .build_insert_value(sa_rt, sa_malloc, 0, "d")
            .map_err(llvm_err)?;
        let sa_rtl = self
            .builder
            .build_insert_value(sa_rtd, i64.const_int(2, false), 1, "l")
            .map_err(llvm_err)?;
        let sa_rtc = self
            .builder
            .build_insert_value(sa_rtl, i64.const_int(2, false), 2, "c")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sa_rtc));

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
        let ch_subl = self.call_rt("action_list_create", &[ch_csafe.into()])?;
        let ch_sublv = ch_subl.try_as_basic_value().unwrap_basic();
        let ch_sa = self
            .builder
            .build_alloca(self.list_type, "ch_sa")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_sa, ch_sublv)
            .map_err(llvm_err)?;
        let ch_j = self.builder.build_alloca(i64, "ch_j").map_err(llvm_err)?;
        self.builder
            .build_store(ch_j, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let ch_iloop = self.context.append_basic_block(ch_fn, "iloop");
        let ch_ibody = self.context.append_basic_block(ch_fn, "ibody");
        let ch_idone = self.context.append_basic_block(ch_fn, "idone");
        let _ = self.builder.build_unconditional_branch(ch_iloop);
        self.builder.position_at_end(ch_iloop);
        let ch_jv = self
            .builder
            .build_load(i64, ch_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_jc = self
            .builder
            .build_int_compare(IntPredicate::SLT, ch_jv, ch_csafe, "jc")
            .map_err(llvm_err)?;
        let ch_end = self
            .builder
            .build_int_compare(IntPredicate::SGE, ch_iv, ch_len, "end")
            .map_err(llvm_err)?;
        let ch_ic = self
            .builder
            .build_and(
                ch_jc,
                self.builder.build_not(ch_end, "").map_err(llvm_err)?,
                "ic",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ch_ic, ch_ibody, ch_idone);
        self.builder.position_at_end(ch_ibody);
        let ch_cur_i = self
            .builder
            .build_load(i64, ch_i, "cur_i")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_get_fn = self.module.get_function("action_list_get").unwrap();
        let ch_ev = self
            .builder
            .build_call(ch_get_fn, &[ch_in.into(), ch_cur_i.into()], "ev")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?
            .into_struct_value();
        let ch_cl = self
            .builder
            .build_load(self.list_type, ch_sa, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let ch_ps = self.call_rt(
            "action_list_push",
            &[ch_cl.into(), ch_ev.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(ch_sa, ch_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let ch_ivi = self
            .builder
            .build_int_add(ch_cur_i, i64.const_int(1, false), "ivi")
            .map_err(llvm_err)?;
        self.builder.build_store(ch_i, ch_ivi).map_err(llvm_err)?;
        let ch_jvi = self
            .builder
            .build_int_add(ch_jv, i64.const_int(1, false), "jvi")
            .map_err(llvm_err)?;
        self.builder.build_store(ch_j, ch_jvi).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ch_iloop);
        self.builder.position_at_end(ch_idone);
        let ch_subl_fat = self.string_type.get_undef();
        let ch_sublft = self
            .builder
            .build_insert_value(ch_subl_fat, i64.const_int(6, false), 0, "st")
            .map_err(llvm_err)?;
        let ch_subl_l = self
            .builder
            .build_load(self.list_type, ch_sa, "sl")
            .map_err(llvm_err)?
            .into_struct_value();
        let ch_sp = self
            .builder
            .build_alloca(self.list_type, "ch_sp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_sp, ch_subl_l)
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
        let wn_subl = self.call_rt("action_list_create", &[wn_wsafe.into()])?;
        let wn_sublv = wn_subl.try_as_basic_value().unwrap_basic();
        let wn_sa = self
            .builder
            .build_alloca(self.list_type, "wn_sa")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wn_sa, wn_sublv)
            .map_err(llvm_err)?;
        let wn_j = self.builder.build_alloca(i64, "wn_j").map_err(llvm_err)?;
        self.builder
            .build_store(wn_j, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let wn_iloop = self.context.append_basic_block(wn_fn, "iloop");
        let wn_ibody = self.context.append_basic_block(wn_fn, "ibody");
        let wn_idone = self.context.append_basic_block(wn_fn, "idone");
        let _ = self.builder.build_unconditional_branch(wn_iloop);
        self.builder.position_at_end(wn_iloop);
        let wn_jv = self
            .builder
            .build_load(i64, wn_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_jc = self
            .builder
            .build_int_compare(IntPredicate::SLT, wn_jv, wn_wsafe, "jc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(wn_jc, wn_ibody, wn_idone);
        self.builder.position_at_end(wn_ibody);
        let wn_ep_idx = self
            .builder
            .build_int_add(wn_iv, wn_jv, "epi")
            .map_err(llvm_err)?;
        let wn_get_fn = self.module.get_function("action_list_get").unwrap();
        let wn_ev = self
            .builder
            .build_call(wn_get_fn, &[wn_in.into(), wn_ep_idx.into()], "ev")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?
            .into_struct_value();
        let wn_cl = self
            .builder
            .build_load(self.list_type, wn_sa, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let wn_ps = self.call_rt(
            "action_list_push",
            &[wn_cl.into(), wn_ev.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(wn_sa, wn_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let wn_jvi = self
            .builder
            .build_int_add(wn_jv, i64.const_int(1, false), "jvi")
            .map_err(llvm_err)?;
        self.builder.build_store(wn_j, wn_jvi).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(wn_iloop);
        self.builder.position_at_end(wn_idone);
        let wn_fat = self.string_type.get_undef();
        let wn_ft = self
            .builder
            .build_insert_value(wn_fat, i64.const_int(6, false), 0, "ft")
            .map_err(llvm_err)?;
        let wn_sl = self
            .builder
            .build_load(self.list_type, wn_sa, "sl")
            .map_err(llvm_err)?
            .into_struct_value();
        let wn_sp = self
            .builder
            .build_alloca(self.list_type, "wn_sp")
            .map_err(llvm_err)?;
        self.builder.build_store(wn_sp, wn_sl).map_err(llvm_err)?;
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

        // ---- action_list_index_of({ptr, i64, i64}, {i64, ptr}) -> i64 ----
        let lio_fn = self.module.add_function(
            "action_list_index_of",
            i64.fn_type(&[self.list_type.into(), str_ty.into()], false),
            None,
        );
        let lio_entry = self.context.append_basic_block(lio_fn, "entry");
        self.builder.position_at_end(lio_entry);
        let lio_lst = lio_fn.get_first_param().unwrap().into_struct_value();
        let lio_tgt = lio_fn.get_nth_param(1).unwrap().into_struct_value();

        let lio_len = self
            .builder
            .build_extract_value(lio_lst, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let lio_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(lio_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let lio_loop = self.context.append_basic_block(lio_fn, "loop");
        let lio_body = self.context.append_basic_block(lio_fn, "body");
        let lio_nf = self.context.append_basic_block(lio_fn, "notfound");
        let _ = self.builder.build_unconditional_branch(lio_loop);
        self.builder.position_at_end(lio_loop);
        let lio_iv = self
            .builder
            .build_load(i64, lio_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let lio_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lio_iv, lio_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lio_cond, lio_body, lio_nf);
        self.builder.position_at_end(lio_body);
        // Load element via action_list_get (tree-aware)
        let lio_get_fn = self.module.get_function("action_list_get").unwrap();
        let lio_get_cc = self
            .builder
            .build_call(lio_get_fn, &[lio_lst.into(), lio_iv.into()], "lio_get")
            .map_err(llvm_err)?;
        let lio_ev = lio_get_cc
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?
            .into_struct_value();
        let lio_etag = self
            .builder
            .build_extract_value(lio_ev, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let lio_ttag = self
            .builder
            .build_extract_value(lio_tgt, 0, "ttag")
            .map_err(llvm_err)?
            .into_int_value();
        let lio_teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, lio_etag, lio_ttag, "teq")
            .map_err(llvm_err)?;
        let lio_eptr = self
            .builder
            .build_extract_value(lio_ev, 1, "eptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lio_tptr = self
            .builder
            .build_extract_value(lio_tgt, 1, "tptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lio_ptr_match = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_ptr_to_int(lio_eptr, i64, "")
                    .map_err(llvm_err)?,
                self.builder
                    .build_ptr_to_int(lio_tptr, i64, "")
                    .map_err(llvm_err)?,
                "scm",
            )
            .map_err(llvm_err)?;
        let lio_match = self
            .builder
            .build_and(lio_teq, lio_ptr_match, "match")
            .map_err(llvm_err)?;
        let lio_ret_match = self.context.append_basic_block(lio_fn, "ret_match");
        let lio_next = self.context.append_basic_block(lio_fn, "next");
        let _ = self
            .builder
            .build_conditional_branch(lio_match, lio_ret_match, lio_next);
        self.builder.position_at_end(lio_ret_match);
        let _ = self.builder.build_return(Some(&lio_iv));
        self.builder.position_at_end(lio_next);
        let lio_inc = self
            .builder
            .build_int_add(lio_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(lio_i, lio_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lio_loop);
        self.builder.position_at_end(lio_nf);
        let _ = self
            .builder
            .build_return(Some(&i64.const_int(-1i64 as u64, true)));

        // ---- action_list_concat({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Block-based: walks source tree leaves, pushes elements in batches.
        // Special-cases height-0 + height-0 with total <= 64 for O(1) single-leaf merge.
        let concat_fn = self.module.get_function("action_list_concat").unwrap();
        let concat_entry = self.context.append_basic_block(concat_fn, "entry");
        self.builder.position_at_end(concat_entry);
        // Allocate result slot in entry (must dominate all paths)
        let _concat_ra = self
            .builder
            .build_alloca(self.list_type, "concat_ra")
            .map_err(llvm_err)?;
        let concat_a = concat_fn.get_first_param().unwrap().into_struct_value();
        let concat_b = concat_fn.get_nth_param(1).unwrap().into_struct_value();
        let a_len = self
            .builder
            .build_extract_value(concat_a, 1, "a_len")
            .map_err(llvm_err)?
            .into_int_value();
        let b_len = self
            .builder
            .build_extract_value(concat_b, 1, "b_len")
            .map_err(llvm_err)?
            .into_int_value();
        let a_height = self
            .builder
            .build_extract_value(concat_a, 2, "a_h")
            .map_err(llvm_err)?
            .into_int_value();
        let b_height = self
            .builder
            .build_extract_value(concat_b, 2, "b_h")
            .map_err(llvm_err)?
            .into_int_value();
        let a_node = self
            .builder
            .build_extract_value(concat_a, 0, "a_n")
            .map_err(llvm_err)?
            .into_pointer_value();
        let b_node = self
            .builder
            .build_extract_value(concat_b, 0, "b_n")
            .map_err(llvm_err)?
            .into_pointer_value();
        let total = self
            .builder
            .build_int_add(a_len, b_len, "total")
            .map_err(llvm_err)?;
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let b64 = i64.const_int(64, false);
        let elem_sz = i64.const_int(16, false); // string_type = {i64, ptr}
        let leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;
        let _concat_push_fn = self.module.get_function("action_list_push").unwrap();
        let _concat_create_fn = self.module.get_function("action_list_create").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();

        // === Edge cases: empty list sharing ===
        let cc_empty_a = self.context.append_basic_block(concat_fn, "empty_a");
        let cc_empty_b = self.context.append_basic_block(concat_fn, "empty_b");
        let cc_share_ret = self.context.append_basic_block(concat_fn, "share_ret");
        let cc_small_merge = self.context.append_basic_block(concat_fn, "small_merge");
        let cc_lazy_concat = self.context.append_basic_block(concat_fn, "lazy_concat");

        let b_is_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, b_len, zero, "b_z")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(b_is_zero, cc_empty_b, cc_empty_a);

        // A is empty (B non-empty): share B
        self.builder.position_at_end(cc_empty_a);
        let a_is_zero = self
            .builder
            .build_int_compare(IntPredicate::EQ, a_len, zero, "a_z")
            .map_err(llvm_err)?;
        // Check special merge case: both height=0 && total <= 64
        let _ = self
            .builder
            .build_conditional_branch(a_is_zero, cc_share_ret, cc_small_merge);

        // B is empty: share A
        self.builder.position_at_end(cc_empty_b);
        let _ = self.builder.build_unconditional_branch(cc_share_ret);

        // share_ret: rc_inc the non-empty node and return it
        self.builder.position_at_end(cc_share_ret);
        // Phi for which list to return (A when B empty, B when A empty)
        let share_phi_list = self
            .builder
            .build_phi(self.list_type, "share_phi")
            .map_err(llvm_err)?;
        share_phi_list.add_incoming(&[(&concat_a, cc_empty_b)]);
        share_phi_list.add_incoming(&[(&concat_b, cc_empty_a)]);
        let share_list = share_phi_list.as_basic_value().into_struct_value();
        let share_node = self
            .builder
            .build_extract_value(share_list, 0, "share_n")
            .map_err(llvm_err)?
            .into_pointer_value();
        // rc_inc the shared node to account for the new reference
        let share_rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(share_rc_inc_fn, &[share_node.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&share_list));

        // === Special case: both height=0, total <= 64 → single leaf merge ===
        self.builder.position_at_end(cc_small_merge);
        let bh0_cond = self
            .builder
            .build_int_compare(IntPredicate::EQ, b_height, zero, "bh0")
            .map_err(llvm_err)?;
        let total_small = self
            .builder
            .build_int_compare(IntPredicate::SLE, total, b64, "tsmall")
            .map_err(llvm_err)?;
        let can_merge = self
            .builder
            .build_and(bh0_cond, total_small, "can_merge")
            .map_err(llvm_err)?;
        let cc_do_merge = self.context.append_basic_block(concat_fn, "do_merge");
        let _ = self
            .builder
            .build_conditional_branch(can_merge, cc_do_merge, cc_lazy_concat);

        // Perform single-leaf merge
        self.builder.position_at_end(cc_do_merge);
        let new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "merged")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let a_leaf_i8 = self
            .builder
            .build_pointer_cast(a_node, ptr, "ali8")
            .map_err(llvm_err)?;
        let b_leaf_i8 = self
            .builder
            .build_pointer_cast(b_node, ptr, "bli8")
            .map_err(llvm_err)?;
        let nl_i8 = self
            .builder
            .build_pointer_cast(new_leaf, ptr, "nli8")
            .map_err(llvm_err)?;
        // Copy a's elements: dst = new_leaf+8, src = a_leaf+8, size = a_len*16
        let a_src = unsafe {
            self.builder
                .build_gep(i8, a_leaf_i8, &[i64.const_int(8, false)], "a_src")
                .map_err(llvm_err)
        }?;
        let nl_dst = unsafe {
            self.builder
                .build_gep(i8, nl_i8, &[i64.const_int(8, false)], "nl_dst")
                .map_err(llvm_err)
        }?;
        let a_bytes = self
            .builder
            .build_int_mul(a_len, elem_sz, "a_bytes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[nl_dst.into(), a_src.into(), a_bytes.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Copy b's elements: dst = new_leaf+8 + a_len*16, src = b_leaf+8, size = b_len*16
        let nl_dst2 = unsafe {
            self.builder
                .build_gep(self.string_type, nl_dst, &[a_len], "nl_dst2")
                .map_err(llvm_err)
        }?;
        let b_src = unsafe {
            self.builder
                .build_gep(i8, b_leaf_i8, &[i64.const_int(8, false)], "b_src")
                .map_err(llvm_err)
        }?;
        let b_bytes = self
            .builder
            .build_int_mul(b_len, elem_sz, "b_bytes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[nl_dst2.into(), b_src.into(), b_bytes.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Set leaf count
        let _ = self.builder.build_store(nl_i8, total).map_err(llvm_err)?;
        // Return {new_leaf, total, 0}
        let sm_undef = self.list_type.get_undef();
        let sm_r1 = self
            .builder
            .build_insert_value(sm_undef, new_leaf, 0, "sm_r1")
            .map_err(llvm_err)?;
        let sm_r2 = self
            .builder
            .build_insert_value(sm_r1, total, 1, "sm_r2")
            .map_err(llvm_err)?;
        let sm_r3 = self
            .builder
            .build_insert_value(sm_r2, zero, 2, "sm_r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sm_r3)).map_err(llvm_err)?;

        // === Lazy concat: create ConcatNode instead of flattening immediately ===
        self.builder.position_at_end(cc_lazy_concat);
        // Compute ConcatNode depth: max(existing depth of A/B, 0) + 1
        // Check if A is already a ConcatNode (height == -1)
        let a_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                a_height,
                i64.const_int(-1i64 as u64, true),
                "a_is_concat",
            )
            .map_err(llvm_err)?;
        let a_depth_load_bb = self.context.append_basic_block(concat_fn, "a_depth_load");
        let a_depth_done_bb = self.context.append_basic_block(concat_fn, "a_depth_done");
        let _ =
            self.builder
                .build_conditional_branch(a_is_concat, a_depth_load_bb, a_depth_done_bb);
        self.builder.position_at_end(a_depth_load_bb);
        let a_depth_ptr = unsafe {
            self.builder
                .build_gep(i64, a_node, &[i64.const_int(0, false)], "a_depth_p")
                .map_err(llvm_err)
        }?;
        let a_depth_val = self
            .builder
            .build_load(i64, a_depth_ptr, "a_depth_v")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(a_depth_done_bb);
        self.builder.position_at_end(a_depth_done_bb);
        let a_depth_phi = self
            .builder
            .build_phi(i64, "a_depth_phi")
            .map_err(llvm_err)?;
        a_depth_phi.add_incoming(&[(&zero, cc_lazy_concat)]); // flat tree (a_is_concat == false)
        a_depth_phi.add_incoming(&[(&a_depth_val, a_depth_load_bb)]); // ConcatNode (loaded depth)
        let a_depth = a_depth_phi.as_basic_value().into_int_value();

        // Check if B is already a ConcatNode (height == -1)
        let b_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                b_height,
                i64.const_int(-1i64 as u64, true),
                "b_is_concat",
            )
            .map_err(llvm_err)?;
        let b_depth_load_bb = self.context.append_basic_block(concat_fn, "b_depth_load");
        let b_depth_done_bb = self.context.append_basic_block(concat_fn, "b_depth_done");
        let _ =
            self.builder
                .build_conditional_branch(b_is_concat, b_depth_load_bb, b_depth_done_bb);
        self.builder.position_at_end(b_depth_load_bb);
        let b_depth_ptr = unsafe {
            self.builder
                .build_gep(i64, b_node, &[i64.const_int(0, false)], "b_depth_p")
                .map_err(llvm_err)
        }?;
        let b_depth_val = self
            .builder
            .build_load(i64, b_depth_ptr, "b_depth_v")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(b_depth_done_bb);
        self.builder.position_at_end(b_depth_done_bb);
        let b_depth_phi = self
            .builder
            .build_phi(i64, "b_depth_phi")
            .map_err(llvm_err)?;
        b_depth_phi.add_incoming(&[(&zero, a_depth_done_bb)]); // flat tree (b_is_concat == false)
        b_depth_phi.add_incoming(&[(&b_depth_val, b_depth_load_bb)]); // ConcatNode (loaded depth)
        let b_depth = b_depth_phi.as_basic_value().into_int_value();

        // new_depth = max(a_depth, b_depth) + 1
        let a_gt_b = self
            .builder
            .build_int_compare(IntPredicate::SGT, a_depth, b_depth, "a_gt_b")
            .map_err(llvm_err)?;
        let max_depth = self
            .builder
            .build_select(a_gt_b, a_depth, b_depth, "max_depth")
            .map_err(llvm_err)?
            .into_int_value();
        let new_depth = self
            .builder
            .build_int_add(max_depth, one, "new_depth")
            .map_err(llvm_err)?;

        // Allocate ConcatNode: {i64 depth, i64 total_len, self.list_typepe left, self.list_typepe right} = 80 bytes
        let concat_node_size = i64.const_int(80, false);
        let concat_node = self
            .builder
            .build_call(malloc_rc_fn, &[concat_node_size.into()], "concat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let cn_i8 = self
            .builder
            .build_pointer_cast(concat_node, ptr, "cn_i8")
            .map_err(llvm_err)?;

        // Store depth at offset 0
        let _ = self
            .builder
            .build_store(cn_i8, new_depth)
            .map_err(llvm_err)?;
        // Store total_len at offset 8
        let cn_tl = unsafe {
            self.builder
                .build_gep(i64, cn_i8, &[i64.const_int(1, false)], "cn_tl")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(cn_tl, total).map_err(llvm_err)?;
        // Store left list at offset 16 (2 * 8 bytes)
        let cn_left = unsafe {
            self.builder
                .build_gep(i64, cn_i8, &[i64.const_int(2, false)], "cn_left")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(cn_left, concat_a)
            .map_err(llvm_err)?;
        // Store right list at offset 40 (5 * 8 bytes)
        let cn_right = unsafe {
            self.builder
                .build_gep(i64, cn_i8, &[i64.const_int(5, false)], "cn_right")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_store(cn_right, concat_b)
            .map_err(llvm_err)?;

        // rc_inc both children's nodes (they're now referenced by the ConcatNode)
        let cc_rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(cc_rc_inc_fn, &[a_node.into()], "")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(cc_rc_inc_fn, &[b_node.into()], "")
            .map_err(llvm_err)?;

        // Return {concat_node, total, -1}
        let lc_undef = self.list_type.get_undef();
        let lc_r1 = self
            .builder
            .build_insert_value(lc_undef, concat_node, 0, "lc_r1")
            .map_err(llvm_err)?;
        let lc_r2 = self
            .builder
            .build_insert_value(lc_r1, total, 1, "lc_r2")
            .map_err(llvm_err)?;
        let lc_r3 = self
            .builder
            .build_insert_value(lc_r2, i64.const_int(-1i64 as u64, true), 2, "lc_r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lc_r3)).map_err(llvm_err)?;

        Ok(())
    }
}
