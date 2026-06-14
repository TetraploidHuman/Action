// Submodule: runtime_decl/define_list_xform
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_list_xform(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let _void = self.void_ty();
        let ptr = self.ptr_ty();
        let _str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let list_create_fn = self.module.get_function("action_list_create").unwrap();
        let list_push_fn = self.module.get_function("action_list_push").unwrap();
        let list_get_fn = self.module.get_function("action_list_get").unwrap();
        // ---- action_list_reverse({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let lr_fn = self.module.add_function(
            "action_list_reverse",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let lr_entry = self.context.append_basic_block(lr_fn, "entry");
        self.builder.position_at_end(lr_entry);
        let lr_list = lr_fn.get_first_param().unwrap().into_struct_value();
        let lr_len = self
            .builder
            .build_extract_value(lr_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let lr_new = self
            .builder
            .build_call(list_create_fn, &[i64.const_int(0, false).into()], "new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let lr_acc = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        self.builder.build_store(lr_acc, lr_new).map_err(llvm_err)?;
        let lr_i_a = self.builder.build_alloca(i64, "ia").map_err(llvm_err)?;
        let lr_start = self
            .builder
            .build_int_sub(lr_len, i64.const_int(1, false), "start")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lr_i_a, lr_start)
            .map_err(llvm_err)?;
        let lr_loop = self.context.append_basic_block(lr_fn, "loop");
        let lr_body = self.context.append_basic_block(lr_fn, "body");
        let lr_done = self.context.append_basic_block(lr_fn, "done");
        let _ = self.builder.build_unconditional_branch(lr_loop);
        self.builder.position_at_end(lr_loop);
        let lr_i = self
            .builder
            .build_load(i64, lr_i_a, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let lr_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, lr_i, i64.const_int(0, false), "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lr_cond, lr_body, lr_done);
        self.builder.position_at_end(lr_body);
        let lr_fv = self
            .builder
            .build_call(list_get_fn, &[lr_list.into(), lr_i.into()], "fv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?;
        let lr_cur = self
            .builder
            .build_load(self.list_type, lr_acc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let lr_pv = self
            .builder
            .build_call(list_push_fn, &[lr_cur.into(), lr_fv.into()], "pv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("push failed")?;
        self.builder.build_store(lr_acc, lr_pv).map_err(llvm_err)?;
        let lr_ni = self
            .builder
            .build_int_sub(lr_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(lr_i_a, lr_ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lr_loop);
        self.builder.position_at_end(lr_done);
        let lr_rv = self
            .builder
            .build_load(self.list_type, lr_acc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lr_rv));

        // ---- action_list_range(i64, i64) -> {ptr, i64, i64} ----
        let range_fn = self.module.add_function(
            "action_list_range",
            self.list_type.fn_type(&[i64.into(), i64.into()], false),
            None,
        );
        let rg_entry = self.context.append_basic_block(range_fn, "entry");
        self.builder.position_at_end(rg_entry);
        let rg_start = range_fn.get_first_param().unwrap().into_int_value();
        let rg_end = range_fn.get_nth_param(1).unwrap().into_int_value();
        let rg_len = self
            .builder
            .build_int_sub(rg_end, rg_start, "rg_len")
            .map_err(llvm_err)?;
        let rg_cap = self
            .builder
            .build_int_add(rg_len, i64.const_int(1, false), "rg_cap")
            .map_err(llvm_err)?;
        let rg_list = self
            .builder
            .build_call(list_create_fn, &[rg_cap.into()], "rg_list")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let rg_loop_bb = self.context.append_basic_block(range_fn, "rg_loop");
        let rg_done_bb = self.context.append_basic_block(range_fn, "rg_done");
        let rg_check = self
            .builder
            .build_int_compare(IntPredicate::SLT, rg_start, rg_end, "rg_check")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rg_check, rg_loop_bb, rg_done_bb);
        self.builder.position_at_end(rg_loop_bb);
        let rg_i = self.builder.build_phi(i64, "rg_i").map_err(llvm_err)?;
        let rg_list2 = self
            .builder
            .build_phi(self.list_type, "rg_list2")
            .map_err(llvm_err)?;
        // Create fat struct {i64 value, ptr null} for this Int
        let rg_fat_undef = self.string_type.get_undef();
        let rg_fat_val = self
            .builder
            .build_insert_value(
                rg_fat_undef,
                rg_i.as_basic_value().into_int_value(),
                0,
                "rg_fat_val",
            )
            .map_err(llvm_err)?;
        let rg_fat = self
            .builder
            .build_insert_value(rg_fat_val, self.ptr_ty().const_zero(), 1, "rg_fat")
            .map_err(llvm_err)?;
        let rg_list3 = self
            .builder
            .build_call(
                list_push_fn,
                &[
                    rg_list2.as_basic_value().into(),
                    rg_fat.as_basic_value_enum().into(),
                ],
                "rg_push",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("push failed")?;
        let rg_next = self
            .builder
            .build_int_add(
                rg_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "rg_next",
            )
            .map_err(llvm_err)?;
        let rg_done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, rg_next, rg_end, "rg_done_cond")
            .map_err(llvm_err)?;
        let rg_next_block = self.builder.get_insert_block().unwrap();
        rg_i.add_incoming(&[(&rg_start, rg_entry), (&rg_next, rg_next_block)]);
        rg_list2.add_incoming(&[(&rg_list, rg_entry), (&rg_list3, rg_next_block)]);
        let _ = self
            .builder
            .build_conditional_branch(rg_done_cond, rg_done_bb, rg_loop_bb);
        self.builder.position_at_end(rg_done_bb);
        let rg_final = self
            .builder
            .build_phi(self.list_type, "rg_final")
            .map_err(llvm_err)?;
        rg_final.add_incoming(&[(&rg_list, rg_entry), (&rg_list3, rg_next_block)]);
        let _ = self.builder.build_return(Some(&rg_final.as_basic_value()));

        // ---- action_list_take({ptr, i64, i64}, i64) -> {ptr, i64, i64} ----
        let lt_fn = self.module.add_function(
            "action_list_take",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into()], false),
            None,
        );
        let lt_entry = self.context.append_basic_block(lt_fn, "entry");
        let lt_concat = self.context.append_basic_block(lt_fn, "concat");
        let lt_normal = self.context.append_basic_block(lt_fn, "normal");
        let lt_h0 = self.context.append_basic_block(lt_fn, "h0");
        let lt_h0_dec_loop = self.context.append_basic_block(lt_fn, "h0_dec_loop");
        let lt_h0_dec_body = self.context.append_basic_block(lt_fn, "h0_dec_body");
        let lt_h0_dec_done = self.context.append_basic_block(lt_fn, "h0_dec_done");
        let lt_h0_cow = self.context.append_basic_block(lt_fn, "h0_cow");
        let lt_h0_ci_loop = self.context.append_basic_block(lt_fn, "h0_ci_loop");
        let lt_h0_ci_body = self.context.append_basic_block(lt_fn, "h0_ci_body");
        let lt_h0_ci_done = self.context.append_basic_block(lt_fn, "h0_ci_done");
        let lt_h0_reuse = self.context.append_basic_block(lt_fn, "h0_reuse");
        let lt_h0_done = self.context.append_basic_block(lt_fn, "h0_done");
        let lt_hgt0 = self.context.append_basic_block(lt_fn, "hgt0");
        let lt_loop = self.context.append_basic_block(lt_fn, "loop");
        let lt_body = self.context.append_basic_block(lt_fn, "body");
        let lt_done = self.context.append_basic_block(lt_fn, "done");
        self.builder.position_at_end(lt_entry);
        let lt_list = lt_fn.get_first_param().unwrap().into_struct_value();
        let lt_n = lt_fn.get_nth_param(1).unwrap().into_int_value();
        let lt_node = self
            .builder
            .build_extract_value(lt_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lt_len = self
            .builder
            .build_extract_value(lt_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_height = self
            .builder
            .build_extract_value(lt_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lt_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lt_is_concat, lt_concat, lt_normal);
        // ConcatNode: flatten then take
        self.builder.position_at_end(lt_concat);
        let lt_flat_fn = self.module.get_function("action_list_flatten").unwrap();
        let lt_flat = self
            .builder
            .build_call(lt_flat_fn, &[lt_list.into()], "flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lt_take_flat = self
            .builder
            .build_call(lt_fn, &[lt_flat.into(), lt_n.into()], "take_flat")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lt_take_flat));
        // Normal path: check h=0 vs h>0
        self.builder.position_at_end(lt_normal);
        let lt_is_h0 = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                lt_height,
                i64.const_int(0, false),
                "is_h0",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lt_is_h0, lt_h0, lt_hgt0);
        // === h=0: direct leaf manipulation ===
        self.builder.position_at_end(lt_h0);
        let lt_leaf_i8 = self
            .builder
            .build_pointer_cast(lt_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let lt_count_raw = self
            .builder
            .build_load(i32, lt_leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_count = self
            .builder
            .build_int_z_extend(lt_count_raw, i64, "count")
            .map_err(llvm_err)?;
        let lt_actual = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, lt_n, lt_count, "cmp")
                    .map_err(llvm_err)?,
                lt_n,
                lt_count,
                "actual",
            )
            .map_err(llvm_err)?
            .into_int_value();
        // Dec loop: rc_dec truncated elements [actual..count-1]
        let lt_dec_i = self.builder.build_alloca(i64, "dec_i").map_err(llvm_err)?;
        self.builder
            .build_store(lt_dec_i, lt_actual)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lt_h0_dec_loop);
        self.builder.position_at_end(lt_h0_dec_loop);
        let lt_di = self
            .builder
            .build_load(i64, lt_dec_i, "di")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_di_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lt_di, lt_count, "di_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lt_di_cond, lt_h0_dec_body, lt_h0_dec_done);
        self.builder.position_at_end(lt_h0_dec_body);
        let lt_rc_dec_fn = self.module.get_function("action_rc_dec").unwrap();
        let lt_eb = unsafe {
            self.builder
                .build_gep(i8, lt_leaf_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)
        }?;
        let lt_ep = unsafe {
            self.builder
                .build_gep(self.string_type, lt_eb, &[lt_di], "ep")
                .map_err(llvm_err)
        }?;
        let lt_ev = self
            .builder
            .build_load(self.string_type, lt_ep, "ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let lt_ed = self
            .builder
            .build_extract_value(lt_ev, 1, "ed")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(lt_rc_dec_fn, &[lt_ed.into()], "")
            .map_err(llvm_err)?;
        let lt_di_next = self
            .builder
            .build_int_add(lt_di, i64.const_int(1, false), "di_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lt_dec_i, lt_di_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lt_h0_dec_loop);
        // Check RC for CoW vs reuse
        self.builder.position_at_end(lt_h0_dec_done);
        let lt_node_int = self
            .builder
            .build_ptr_to_int(lt_node, i64, "node_int")
            .map_err(llvm_err)?;
        let lt_rc_addr = self
            .builder
            .build_int_sub(lt_node_int, i64.const_int(8, false), "rc_addr")
            .map_err(llvm_err)?;
        let lt_rc_ptr = self
            .builder
            .build_int_to_ptr(lt_rc_addr, ptr, "rc_ptr")
            .map_err(llvm_err)?;
        let lt_rc_val = self
            .builder
            .build_load(i64, lt_rc_ptr, "rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_need_cow = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                lt_rc_val,
                i64.const_int(1, false),
                "need_cow",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lt_need_cow, lt_h0_cow, lt_h0_reuse);
        // CoW: allocate new leaf, copy count+pad+first actual elements
        self.builder.position_at_end(lt_h0_cow);
        let leaf_ty = self.leaf_type;
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let lt_new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "new_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let lt_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let lt_copy_bytes = self
            .builder
            .build_int_mul(lt_actual, i64.const_int(16, false), "copy_bytes")
            .map_err(llvm_err)?;
        let lt_copy_total = self
            .builder
            .build_int_add(lt_copy_bytes, i64.const_int(8, false), "copy_total")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                lt_memcpy_fn,
                &[lt_new_leaf.into(), lt_node.into(), lt_copy_total.into()],
                "",
            )
            .map_err(llvm_err)?;
        // RC-inc each element in the new leaf
        let lt_ci_i = self.builder.build_alloca(i64, "ci_i").map_err(llvm_err)?;
        self.builder
            .build_store(lt_ci_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lt_h0_ci_loop);
        self.builder.position_at_end(lt_h0_ci_loop);
        let lt_ci = self
            .builder
            .build_load(i64, lt_ci_i, "ci")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_ci_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lt_ci, lt_actual, "ci_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lt_ci_cond, lt_h0_ci_body, lt_h0_ci_done);
        self.builder.position_at_end(lt_h0_ci_body);
        let lt_rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let lt_nl_i8 = self
            .builder
            .build_pointer_cast(lt_new_leaf, ptr, "nl_i8")
            .map_err(llvm_err)?;
        let lt_nl_eb = unsafe {
            self.builder
                .build_gep(i8, lt_nl_i8, &[i64.const_int(8, false)], "nl_eb")
                .map_err(llvm_err)
        }?;
        let lt_nl_ep = unsafe {
            self.builder
                .build_gep(self.string_type, lt_nl_eb, &[lt_ci], "nl_ep")
                .map_err(llvm_err)
        }?;
        let lt_nl_ev = self
            .builder
            .build_load(self.string_type, lt_nl_ep, "nl_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let lt_nl_ed = self
            .builder
            .build_extract_value(lt_nl_ev, 1, "nl_ed")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(lt_rc_inc_fn, &[lt_nl_ed.into()], "")
            .map_err(llvm_err)?;
        let lt_ci_next = self
            .builder
            .build_int_add(lt_ci, i64.const_int(1, false), "ci_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lt_ci_i, lt_ci_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lt_h0_ci_loop);
        // Set count on new leaf
        self.builder.position_at_end(lt_h0_ci_done);
        let lt_nl_count_p = self
            .builder
            .build_pointer_cast(lt_new_leaf, ptr, "nl_cp")
            .map_err(llvm_err)?;
        let lt_actual_trunc = self
            .builder
            .build_int_truncate(lt_actual, i32, "actual_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lt_nl_count_p, lt_actual_trunc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lt_h0_done);
        // Reuse: just set count on original leaf
        self.builder.position_at_end(lt_h0_reuse);
        let lt_actual_trunc2 = self
            .builder
            .build_int_truncate(lt_actual, i32, "actual_i32b")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(lt_leaf_i8, lt_actual_trunc2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lt_h0_done);
        // h0 done: build result
        self.builder.position_at_end(lt_h0_done);
        let lt_phi_leaf = self.builder.build_phi(ptr, "phi_leaf").map_err(llvm_err)?;
        lt_phi_leaf.add_incoming(&[(&lt_new_leaf, lt_h0_ci_done), (&lt_node, lt_h0_reuse)]);
        let lt_result_node = lt_phi_leaf.as_basic_value().into_pointer_value();
        let undef_take = self.list_type.get_undef();
        let lt_r1 = self
            .builder
            .build_insert_value(undef_take, lt_result_node, 0, "r1")
            .map_err(llvm_err)?;
        let lt_r2 = self
            .builder
            .build_insert_value(lt_r1, lt_actual, 1, "r2")
            .map_err(llvm_err)?;
        let lt_r3 = self
            .builder
            .build_insert_value(lt_r2, i64.const_int(0, false), 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lt_r3));
        // === h>0: per-element loop ===
        self.builder.position_at_end(lt_hgt0);
        let lt_actual2 = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, lt_n, lt_len, "cmp2")
                    .map_err(llvm_err)?,
                lt_n,
                lt_len,
                "actual2",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let lt_new = self
            .builder
            .build_call(list_create_fn, &[i64.const_int(0, false).into()], "new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let lt_acc = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        self.builder.build_store(lt_acc, lt_new).map_err(llvm_err)?;
        let lt_i_a = self.builder.build_alloca(i64, "ia").map_err(llvm_err)?;
        self.builder
            .build_store(lt_i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lt_loop);
        self.builder.position_at_end(lt_loop);
        let lt_i = self
            .builder
            .build_load(i64, lt_i_a, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lt_i, lt_actual2, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lt_cond, lt_body, lt_done);
        self.builder.position_at_end(lt_body);
        let lt_get_fn = self.module.get_function("action_list_get").unwrap();
        let lt_fv = self
            .builder
            .build_call(lt_get_fn, &[lt_list.into(), lt_i.into()], "fv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?;
        let lt_fv_data = self
            .builder
            .build_extract_value(lt_fv.into_struct_value(), 1, "fv_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lt_rc_inc_fn2 = self.module.get_function("action_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(lt_rc_inc_fn2, &[lt_fv_data.into()], "")
            .map_err(llvm_err)?;
        let lt_push_fn = self.module.get_function("action_list_push").unwrap();
        let lt_cur = self
            .builder
            .build_load(self.list_type, lt_acc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let lt_pv = self
            .builder
            .build_call(lt_push_fn, &[lt_cur.into(), lt_fv.into()], "pv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("push failed")?;
        self.builder.build_store(lt_acc, lt_pv).map_err(llvm_err)?;
        let lt_ni = self
            .builder
            .build_int_add(lt_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(lt_i_a, lt_ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lt_loop);
        self.builder.position_at_end(lt_done);
        let lt_rv = self
            .builder
            .build_load(self.list_type, lt_acc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lt_rv));

        // ---- action_list_drop({ptr, i64, i64}, i64) -> {ptr, i64, i64} ----
        let ld_fn = self.module.add_function(
            "action_list_drop",
            self.list_type
                .fn_type(&[self.list_type.into(), i64.into()], false),
            None,
        );
        let ld_entry = self.context.append_basic_block(ld_fn, "entry");
        self.builder.position_at_end(ld_entry);
        let ld_list = ld_fn.get_first_param().unwrap().into_struct_value();
        let ld_n = ld_fn.get_nth_param(1).unwrap().into_int_value();
        let ld_len = self
            .builder
            .build_extract_value(ld_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let ld_start = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, ld_n, ld_len, "cmp")
                    .map_err(llvm_err)?,
                ld_n,
                ld_len,
                "start",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let ld_new = self
            .builder
            .build_call(list_create_fn, &[i64.const_int(0, false).into()], "new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let ld_acc = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        self.builder.build_store(ld_acc, ld_new).map_err(llvm_err)?;
        let ld_i_a = self.builder.build_alloca(i64, "ia").map_err(llvm_err)?;
        self.builder
            .build_store(ld_i_a, ld_start)
            .map_err(llvm_err)?;
        let ld_loop = self.context.append_basic_block(ld_fn, "loop");
        let ld_body = self.context.append_basic_block(ld_fn, "body");
        let ld_done = self.context.append_basic_block(ld_fn, "done");
        let _ = self.builder.build_unconditional_branch(ld_loop);
        self.builder.position_at_end(ld_loop);
        let ld_i = self
            .builder
            .build_load(i64, ld_i_a, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let ld_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ld_i, ld_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ld_cond, ld_body, ld_done);
        self.builder.position_at_end(ld_body);
        let ld_fv = self
            .builder
            .build_call(list_get_fn, &[ld_list.into(), ld_i.into()], "fv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?;
        let ld_fv_data = self
            .builder
            .build_extract_value(ld_fv.into_struct_value(), 1, "fv_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ld_rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(ld_rc_inc_fn, &[ld_fv_data.into()], "")
            .map_err(llvm_err)?;
        let ld_cur = self
            .builder
            .build_load(self.list_type, ld_acc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let ld_pv = self
            .builder
            .build_call(list_push_fn, &[ld_cur.into(), ld_fv.into()], "pv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("push failed")?;
        self.builder.build_store(ld_acc, ld_pv).map_err(llvm_err)?;
        let ld_ni = self
            .builder
            .build_int_add(ld_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(ld_i_a, ld_ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ld_loop);
        self.builder.position_at_end(ld_done);
        let ld_rv = self
            .builder
            .build_load(self.list_type, ld_acc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ld_rv));

        // ---- abs(i64) -> i64 ----
        let abs_fn = self
            .module
            .add_function("abs", i64.fn_type(&[i64.into()], false), None);
        let entry = self.context.append_basic_block(abs_fn, "entry");
        self.builder.position_at_end(entry);
        let x = abs_fn.get_first_param().unwrap().into_int_value();
        let neg = self.builder.build_int_neg(x, "neg").map_err(llvm_err)?;
        let is_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, x, i64.const_int(0, false), "is_neg")
            .map_err(llvm_err)?;
        let result = self
            .builder
            .build_select(is_neg, neg, x, "abs_result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&result.into_int_value()));

        // ---- min(i64, i64) -> i64 ----
        let min_fn =
            self.module
                .add_function("min", i64.fn_type(&[i64.into(), i64.into()], false), None);
        let entry = self.context.append_basic_block(min_fn, "entry");
        self.builder.position_at_end(entry);
        let a = min_fn.get_first_param().unwrap().into_int_value();
        let b = min_fn.get_nth_param(1).unwrap().into_int_value();
        let lt = self
            .builder
            .build_int_compare(IntPredicate::SLT, a, b, "lt")
            .map_err(llvm_err)?;
        let min_result = self
            .builder
            .build_select(lt, a, b, "min_result")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_return(Some(&min_result.into_int_value()));

        // ---- max(i64, i64) -> i64 ----
        let max_fn =
            self.module
                .add_function("max", i64.fn_type(&[i64.into(), i64.into()], false), None);
        let entry = self.context.append_basic_block(max_fn, "entry");
        self.builder.position_at_end(entry);
        let ma = max_fn.get_first_param().unwrap().into_int_value();
        let mb = max_fn.get_nth_param(1).unwrap().into_int_value();
        let gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, ma, mb, "gt")
            .map_err(llvm_err)?;
        let max_result = self
            .builder
            .build_select(gt, ma, mb, "max_result")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_return(Some(&max_result.into_int_value()));

        Ok(())
    }
}
