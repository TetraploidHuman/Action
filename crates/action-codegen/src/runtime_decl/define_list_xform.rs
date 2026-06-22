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
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let neg1 = i64.const_int(-1i64 as u64, true);
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let list_create_fn = self.module.get_function("action_list_create").unwrap();
        let list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        let child_entry_ty = self.child_entry_type;

        // ---- action_list_reverse_walk_rec(ptr acc, ptr node, i64 height) -> void ----
        // Reverse-order B-tree / ConcatNode walk: push elements into acc without get().
        let rw_fn = self
            .module
            .get_function("action_list_reverse_walk_rec")
            .unwrap();
        let rw_entry = self.context.append_basic_block(rw_fn, "entry");
        let rw_concat = self.context.append_basic_block(rw_fn, "concat");
        let rw_not_concat = self.context.append_basic_block(rw_fn, "not_concat");
        let rw_h0_leaf = self.context.append_basic_block(rw_fn, "h0_leaf");
        let rw_h0_loop = self.context.append_basic_block(rw_fn, "h0_loop");
        let rw_h0_body = self.context.append_basic_block(rw_fn, "h0_body");
        let rw_h0_done = self.context.append_basic_block(rw_fn, "h0_done");
        let rw_h1_intl = self.context.append_basic_block(rw_fn, "h1_intl");
        let rw_h1_loop = self.context.append_basic_block(rw_fn, "h1_loop");
        let rw_h1_body = self.context.append_basic_block(rw_fn, "h1_body");
        let rw_h1_done = self.context.append_basic_block(rw_fn, "h1_done");
        let rw_hgt1 = self.context.append_basic_block(rw_fn, "hgt1");
        let rw_hgt1_loop = self.context.append_basic_block(rw_fn, "hgt1_loop");
        let rw_hgt1_body = self.context.append_basic_block(rw_fn, "hgt1_body");
        let rw_hgt1_done = self.context.append_basic_block(rw_fn, "hgt1_done");
        let rw_done = self.context.append_basic_block(rw_fn, "done");
        self.builder.position_at_end(rw_entry);
        let rw_acc = rw_fn.get_first_param().unwrap().into_pointer_value();
        let rw_node = rw_fn.get_nth_param(1).unwrap().into_pointer_value();
        let rw_height = rw_fn.get_nth_param(2).unwrap().into_int_value();
        let rw_is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, rw_height, neg1, "is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rw_is_concat, rw_concat, rw_not_concat);
        // ConcatNode: reverse walk right then left
        self.builder.position_at_end(rw_concat);
        let rw_cn_i8 = self
            .builder
            .build_pointer_cast(rw_node, ptr, "cn_i8")
            .map_err(llvm_err)?;
        let rw_right_ptr = unsafe {
            self.builder
                .build_gep(i8, rw_cn_i8, &[i64.const_int(40, false)], "right_ptr")
                .map_err(llvm_err)
        }?;
        let rw_right = self
            .builder
            .build_load(self.list_type, rw_right_ptr, "right")
            .map_err(llvm_err)?
            .into_struct_value();
        let rw_left_ptr = unsafe {
            self.builder
                .build_gep(i8, rw_cn_i8, &[i64.const_int(16, false)], "left_ptr")
                .map_err(llvm_err)
        }?;
        let rw_left = self
            .builder
            .build_load(self.list_type, rw_left_ptr, "left")
            .map_err(llvm_err)?
            .into_struct_value();
        let rw_r_node = self
            .builder
            .build_extract_value(rw_right, 0, "r_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rw_r_h = self
            .builder
            .build_extract_value(rw_right, 2, "r_fh")
            .map_err(llvm_err)?
            .into_int_value();
        let rw_l_node = self
            .builder
            .build_extract_value(rw_left, 0, "l_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rw_l_h = self
            .builder
            .build_extract_value(rw_left, 2, "l_fh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(rw_fn, &[rw_acc.into(), rw_r_node.into(), rw_r_h.into()], "")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(rw_fn, &[rw_acc.into(), rw_l_node.into(), rw_l_h.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rw_done);
        // Three-way dispatch: h==0, h==1, h>=2
        self.builder.position_at_end(rw_not_concat);
        let rw_is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, rw_height, zero, "is_h0")
            .map_err(llvm_err)?;
        let rw_not_h0 = self.context.append_basic_block(rw_fn, "not_h0");
        let _ = self
            .builder
            .build_conditional_branch(rw_is_h0, rw_h0_leaf, rw_not_h0);
        self.builder.position_at_end(rw_not_h0);
        let rw_is_h1 = self
            .builder
            .build_int_compare(IntPredicate::EQ, rw_height, one, "is_h1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rw_is_h1, rw_h1_intl, rw_hgt1);
        // === h=0: reverse leaf scan ===
        self.builder.position_at_end(rw_h0_leaf);
        let rw_leaf_i8 = self
            .builder
            .build_pointer_cast(rw_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let rw_count_raw = self
            .builder
            .build_load(i32, rw_leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let rw_count = self
            .builder
            .build_int_z_extend(rw_count_raw, i64, "count")
            .map_err(llvm_err)?;
        let rw_h0_i = self.builder.build_alloca(i64, "h0_i").map_err(llvm_err)?;
        let rw_h0_start = self
            .builder
            .build_int_sub(rw_count, one, "h0_start")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rw_h0_i, rw_h0_start)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rw_h0_loop);
        self.builder.position_at_end(rw_h0_loop);
        let rw_h0_iv = self
            .builder
            .build_load(i64, rw_h0_i, "h0_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let rw_h0_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, rw_h0_iv, zero, "h0_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rw_h0_cond, rw_h0_body, rw_h0_done);
        self.builder.position_at_end(rw_h0_body);
        let rw_eb = unsafe {
            self.builder
                .build_gep(i8, rw_leaf_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)
        }?;
        let rw_ep = unsafe {
            self.builder
                .build_gep(self.string_type, rw_eb, &[rw_h0_iv], "ep")
                .map_err(llvm_err)
        }?;
        let rw_elem = self
            .builder
            .build_load(self.string_type, rw_ep, "elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let rw_cur = self
            .builder
            .build_load(self.list_type, rw_acc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let rw_pushed = self
            .builder
            .build_call(list_push_fn, &[rw_cur.into(), rw_elem.into()], "pushed")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("push failed")?;
        self.builder
            .build_store(rw_acc, rw_pushed)
            .map_err(llvm_err)?;
        let rw_h0_next = self
            .builder
            .build_int_sub(rw_h0_iv, one, "h0_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rw_h0_i, rw_h0_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rw_h0_loop);
        self.builder.position_at_end(rw_h0_done);
        let _ = self.builder.build_unconditional_branch(rw_done);
        // === h=1: reverse child scan (leaf children) ===
        self.builder.position_at_end(rw_h1_intl);
        let rw_intl_i8 = self
            .builder
            .build_pointer_cast(rw_node, ptr, "intl_i8")
            .map_err(llvm_err)?;
        let rw_intl_cnt_r = self
            .builder
            .build_load(i32, rw_intl_i8, "intl_cnt")
            .map_err(llvm_err)?
            .into_int_value();
        let rw_intl_cnt = self
            .builder
            .build_int_z_extend(rw_intl_cnt_r, i64, "intl_cnt64")
            .map_err(llvm_err)?;
        let rw_h1_i = self.builder.build_alloca(i64, "h1_i").map_err(llvm_err)?;
        let rw_h1_start = self
            .builder
            .build_int_sub(rw_intl_cnt, one, "h1_start")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rw_h1_i, rw_h1_start)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rw_h1_loop);
        self.builder.position_at_end(rw_h1_loop);
        let rw_h1_iv = self
            .builder
            .build_load(i64, rw_h1_i, "h1_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let rw_h1_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, rw_h1_iv, zero, "h1_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rw_h1_cond, rw_h1_body, rw_h1_done);
        self.builder.position_at_end(rw_h1_body);
        let rw_h1_ce_off = self
            .builder
            .build_int_add(
                i64.const_int(16, false),
                self.builder
                    .build_int_mul(rw_h1_iv, i64.const_int(16, false), "h1_ce_off_m")
                    .map_err(llvm_err)?,
                "h1_ce_off",
            )
            .map_err(llvm_err)?;
        let rw_h1_ce_p = unsafe {
            self.builder
                .build_gep(i8, rw_intl_i8, &[rw_h1_ce_off], "h1_ce_p")
                .map_err(llvm_err)
        }?;
        let rw_h1_ce = self
            .builder
            .build_load(child_entry_ty, rw_h1_ce_p, "h1_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let rw_h1_child = self
            .builder
            .build_extract_value(rw_h1_ce, 0, "h1_child")
            .map_err(llvm_err)?
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(rw_fn, &[rw_acc.into(), rw_h1_child.into(), zero.into()], "")
            .map_err(llvm_err)?;
        let rw_h1_next = self
            .builder
            .build_int_sub(rw_h1_iv, one, "h1_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rw_h1_i, rw_h1_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rw_h1_loop);
        self.builder.position_at_end(rw_h1_done);
        let _ = self.builder.build_unconditional_branch(rw_done);
        // === h>=2: reverse deep internal node scan ===
        self.builder.position_at_end(rw_hgt1);
        let rw_d_intl_i8 = self
            .builder
            .build_pointer_cast(rw_node, ptr, "dintl_i8")
            .map_err(llvm_err)?;
        let rw_d_cnt_r = self
            .builder
            .build_load(i32, rw_d_intl_i8, "dcnt")
            .map_err(llvm_err)?
            .into_int_value();
        let rw_d_cnt = self
            .builder
            .build_int_z_extend(rw_d_cnt_r, i64, "dcnt64")
            .map_err(llvm_err)?;
        let rw_hgt1_i = self.builder.build_alloca(i64, "hgt1_i").map_err(llvm_err)?;
        let rw_hgt1_start = self
            .builder
            .build_int_sub(rw_d_cnt, one, "hgt1_start")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rw_hgt1_i, rw_hgt1_start)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rw_hgt1_loop);
        self.builder.position_at_end(rw_hgt1_loop);
        let rw_hgt1_iv = self
            .builder
            .build_load(i64, rw_hgt1_i, "hgt1_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let rw_hgt1_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, rw_hgt1_iv, zero, "hgt1_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rw_hgt1_cond, rw_hgt1_body, rw_hgt1_done);
        self.builder.position_at_end(rw_hgt1_body);
        let rw_hgt1_ce_off = self
            .builder
            .build_int_add(
                i64.const_int(16, false),
                self.builder
                    .build_int_mul(rw_hgt1_iv, i64.const_int(16, false), "hgt1_ce_off_m")
                    .map_err(llvm_err)?,
                "hgt1_ce_off",
            )
            .map_err(llvm_err)?;
        let rw_hgt1_ce_p = unsafe {
            self.builder
                .build_gep(i8, rw_d_intl_i8, &[rw_hgt1_ce_off], "hgt1_ce_p")
                .map_err(llvm_err)
        }?;
        let rw_hgt1_ce = self
            .builder
            .build_load(child_entry_ty, rw_hgt1_ce_p, "hgt1_ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let rw_hgt1_child = self
            .builder
            .build_extract_value(rw_hgt1_ce, 0, "hgt1_child")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rw_child_h = self
            .builder
            .build_int_sub(rw_height, one, "child_h")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                rw_fn,
                &[rw_acc.into(), rw_hgt1_child.into(), rw_child_h.into()],
                "",
            )
            .map_err(llvm_err)?;
        let rw_hgt1_next = self
            .builder
            .build_int_sub(rw_hgt1_iv, one, "hgt1_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rw_hgt1_i, rw_hgt1_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rw_hgt1_loop);
        self.builder.position_at_end(rw_hgt1_done);
        let _ = self.builder.build_unconditional_branch(rw_done);
        self.builder.position_at_end(rw_done);
        let _ = self.builder.build_return(None);

        // ---- action_list_reverse({ptr, i64, i64}) -> {ptr, i64, i64} ----
        // B-tree reverse walk: O(n) tree scan instead of O(n) get+push.
        let lr_fn = self.module.add_function(
            "action_list_reverse",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let lr_entry = self.context.append_basic_block(lr_fn, "entry");
        self.builder.position_at_end(lr_entry);
        let lr_list = lr_fn.get_first_param().unwrap().into_struct_value();
        let lr_node = self
            .builder
            .build_extract_value(lr_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lr_height = self
            .builder
            .build_extract_value(lr_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let lr_new = self
            .builder
            .build_call(list_create_fn, &[zero.into()], "new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let lr_acc = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        self.builder.build_store(lr_acc, lr_new).map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                rw_fn,
                &[lr_acc.into(), lr_node.into(), lr_height.into()],
                "",
            )
            .map_err(llvm_err)?;
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
        let lt_concat_take_left = self.context.append_basic_block(lt_fn, "concat_take_left");
        let lt_concat_take_both = self.context.append_basic_block(lt_fn, "concat_take_both");
        let lt_normal = self.context.append_basic_block(lt_fn, "normal");
        let lt_h0 = self.context.append_basic_block(lt_fn, "h0");
        let lt_h0_cow = self.context.append_basic_block(lt_fn, "h0_cow");
        let lt_h0_ci_loop = self.context.append_basic_block(lt_fn, "h0_ci_loop");
        let lt_h0_ci_body = self.context.append_basic_block(lt_fn, "h0_ci_body");
        let lt_h0_ci_done = self.context.append_basic_block(lt_fn, "h0_ci_done");
        let lt_h0_done = self.context.append_basic_block(lt_fn, "h0_done");
        let lt_hgt0 = self.context.append_basic_block(lt_fn, "hgt0");
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
        // ConcatNode: lazy dispatch — take from left or concat(left, take(right))
        self.builder.position_at_end(lt_concat);
        let lt_cn_ln_p = unsafe {
            self.builder
                .build_gep(ptr, lt_node, &[i64.const_int(2, false)], "cn_ln_p")
                .map_err(llvm_err)
        }?;
        let lt_cn_left_node = self
            .builder
            .build_load(ptr, lt_cn_ln_p, "cn_ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lt_cn_ll_p = unsafe {
            self.builder
                .build_gep(i64, lt_node, &[i64.const_int(3, false)], "cn_ll_p")
                .map_err(llvm_err)
        }?;
        let lt_cn_left_len = self
            .builder
            .build_load(i64, lt_cn_ll_p, "cn_ll")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_cn_lh_p = unsafe {
            self.builder
                .build_gep(i64, lt_node, &[i64.const_int(4, false)], "cn_lh_p")
                .map_err(llvm_err)
        }?;
        let lt_cn_left_h = self
            .builder
            .build_load(i64, lt_cn_lh_p, "cn_lh")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_cn_l_undef = self.list_type.get_undef();
        let lt_cn_l1 = self
            .builder
            .build_insert_value(lt_cn_l_undef, lt_cn_left_node, 0, "cn_l1")
            .map_err(llvm_err)?;
        let lt_cn_l2 = self
            .builder
            .build_insert_value(lt_cn_l1, lt_cn_left_len, 1, "cn_l2")
            .map_err(llvm_err)?;
        let lt_cn_left = self
            .builder
            .build_insert_value(lt_cn_l2, lt_cn_left_h, 2, "cn_left")
            .map_err(llvm_err)?
            .into_struct_value();
        let lt_cn_rn_p = unsafe {
            self.builder
                .build_gep(ptr, lt_node, &[i64.const_int(5, false)], "cn_rn_p")
                .map_err(llvm_err)
        }?;
        let lt_cn_right_node = self
            .builder
            .build_load(ptr, lt_cn_rn_p, "cn_rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lt_cn_rl_p = unsafe {
            self.builder
                .build_gep(i64, lt_node, &[i64.const_int(6, false)], "cn_rl_p")
                .map_err(llvm_err)
        }?;
        let lt_cn_right_len = self
            .builder
            .build_load(i64, lt_cn_rl_p, "cn_rl")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_cn_rh_p = unsafe {
            self.builder
                .build_gep(i64, lt_node, &[i64.const_int(7, false)], "cn_rh_p")
                .map_err(llvm_err)
        }?;
        let lt_cn_right_h = self
            .builder
            .build_load(i64, lt_cn_rh_p, "cn_rh")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_cn_r_undef = self.list_type.get_undef();
        let lt_cn_r1 = self
            .builder
            .build_insert_value(lt_cn_r_undef, lt_cn_right_node, 0, "cn_r1")
            .map_err(llvm_err)?;
        let lt_cn_r2 = self
            .builder
            .build_insert_value(lt_cn_r1, lt_cn_right_len, 1, "cn_r2")
            .map_err(llvm_err)?;
        let lt_cn_right = self
            .builder
            .build_insert_value(lt_cn_r2, lt_cn_right_h, 2, "cn_right")
            .map_err(llvm_err)?
            .into_struct_value();
        let lt_cn_n_le = self
            .builder
            .build_int_compare(IntPredicate::SLE, lt_n, lt_cn_left_len, "cn_n_le")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            lt_cn_n_le,
            lt_concat_take_left,
            lt_concat_take_both,
        );
        self.builder.position_at_end(lt_concat_take_left);
        let lt_cn_tl_res = self
            .builder
            .build_call(lt_fn, &[lt_cn_left.into(), lt_n.into()], "cn_tl")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lt_cn_tl_res));
        self.builder.position_at_end(lt_concat_take_both);
        let lt_concat_fn = self.module.get_function("action_list_concat").unwrap();
        let lt_cn_rn = self
            .builder
            .build_int_sub(lt_n, lt_cn_left_len, "cn_rn_idx")
            .map_err(llvm_err)?;
        let lt_cn_tr = self
            .builder
            .build_call(lt_fn, &[lt_cn_right.into(), lt_cn_rn.into()], "cn_tr")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let lt_cn_tb_res = self
            .builder
            .build_call(lt_concat_fn, &[lt_cn_left.into(), lt_cn_tr.into()], "cn_tb")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&lt_cn_tb_res));
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
        // Persistent take: always copy prefix into a new leaf; never mutate the source leaf.
        let _ = self.builder.build_unconditional_branch(lt_h0_cow);
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
        let lt_str_rc_inc_fn = self.module.get_function("action_string_rc_inc").unwrap();
        let _ = self
            .builder
            .build_call(lt_str_rc_inc_fn, &[lt_nl_ev.into()], "")
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
        // h0 done: build result
        self.builder.position_at_end(lt_h0_done);
        let lt_result_node = lt_new_leaf;
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
        // === h>0: B-tree range walk (skip=0, limit=n) ===
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
            .build_call(list_create_fn, &[zero.into()], "new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let lt_acc = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        self.builder.build_store(lt_acc, lt_new).map_err(llvm_err)?;
        let lt_skip_a = self.builder.build_alloca(i64, "skip_a").map_err(llvm_err)?;
        self.builder
            .build_store(lt_skip_a, zero)
            .map_err(llvm_err)?;
        let lt_limit_a = self
            .builder
            .build_alloca(i64, "limit_a")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lt_limit_a, lt_actual2)
            .map_err(llvm_err)?;
        let lt_range_fn = self
            .module
            .get_function("action_list_range_walk_rec")
            .unwrap();
        let _ = self
            .builder
            .build_call(
                lt_range_fn,
                &[
                    lt_acc.into(),
                    lt_node.into(),
                    lt_height.into(),
                    lt_skip_a.into(),
                    lt_limit_a.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
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
        let ld_h0 = self.context.append_basic_block(ld_fn, "h0");
        let ld_h0_empty = self.context.append_basic_block(ld_fn, "h0_empty");
        let ld_h0_copy = self.context.append_basic_block(ld_fn, "h0_copy");
        let ld_h0_ci_loop = self.context.append_basic_block(ld_fn, "h0_ci_loop");
        let ld_h0_ci_body = self.context.append_basic_block(ld_fn, "h0_ci_body");
        let ld_h0_ci_done = self.context.append_basic_block(ld_fn, "h0_ci_done");
        let ld_h0_done = self.context.append_basic_block(ld_fn, "h0_done");
        let ld_hgt0 = self.context.append_basic_block(ld_fn, "hgt0");
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
        let ld_node = self
            .builder
            .build_extract_value(ld_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ld_height = self
            .builder
            .build_extract_value(ld_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let ld_remain = self
            .builder
            .build_int_sub(ld_len, ld_start, "remain")
            .map_err(llvm_err)?;
        let ld_is_h0 = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                ld_height,
                i64.const_int(0, false),
                "is_h0",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ld_is_h0, ld_h0, ld_hgt0);
        // === h=0: suffix copy into new leaf (symmetric with take prefix path) ===
        self.builder.position_at_end(ld_h0);
        let ld_remain_zero = self
            .builder
            .build_int_compare(IntPredicate::SLE, ld_remain, zero, "rem_zero")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ld_remain_zero, ld_h0_empty, ld_h0_copy);
        self.builder.position_at_end(ld_h0_empty);
        let ld_empty = self
            .builder
            .build_call(list_create_fn, &[zero.into()], "empty")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let _ = self.builder.build_return(Some(&ld_empty));
        self.builder.position_at_end(ld_h0_copy);
        let ld_leaf_i8 = self
            .builder
            .build_pointer_cast(ld_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let ld_count_raw = self
            .builder
            .build_load(i32, ld_leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let ld_count = self
            .builder
            .build_int_z_extend(ld_count_raw, i64, "count")
            .map_err(llvm_err)?;
        let ld_s_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, ld_start, ld_count, "s_gt")
            .map_err(llvm_err)?;
        let ld_copy_start = self
            .builder
            .build_select(ld_s_gt, ld_count, ld_start, "copy_start")
            .map_err(llvm_err)?
            .into_int_value();
        let ld_avail = self
            .builder
            .build_int_sub(ld_count, ld_copy_start, "avail")
            .map_err(llvm_err)?;
        let ld_avail_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, ld_avail, zero, "avail_neg")
            .map_err(llvm_err)?;
        let ld_avail0 = self
            .builder
            .build_select(ld_avail_neg, zero, ld_avail, "avail0")
            .map_err(llvm_err)?
            .into_int_value();
        let ld_rem_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, ld_remain, ld_avail0, "rem_gt")
            .map_err(llvm_err)?;
        let ld_new_count = self
            .builder
            .build_select(ld_rem_gt, ld_avail0, ld_remain, "new_count")
            .map_err(llvm_err)?
            .into_int_value();
        let ld_nc_zero = self
            .builder
            .build_int_compare(IntPredicate::SLE, ld_new_count, zero, "nc_zero")
            .map_err(llvm_err)?;
        let ld_h0_alloc = self.context.append_basic_block(ld_fn, "h0_alloc");
        let _ = self
            .builder
            .build_conditional_branch(ld_nc_zero, ld_h0_empty, ld_h0_alloc);
        self.builder.position_at_end(ld_h0_alloc);
        let leaf_ty = self.leaf_type;
        let leaf_size = leaf_ty.size_of().ok_or("leaf size")?;
        let ld_new_leaf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_size.into()], "new_leaf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let ld_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let ld_old_eb = unsafe {
            self.builder
                .build_gep(i8, ld_leaf_i8, &[i64.const_int(8, false)], "old_eb")
                .map_err(llvm_err)
        }?;
        let ld_src = unsafe {
            self.builder
                .build_gep(self.string_type, ld_old_eb, &[ld_copy_start], "src")
                .map_err(llvm_err)
        }?;
        let ld_new_i8 = self
            .builder
            .build_pointer_cast(ld_new_leaf, ptr, "new_i8")
            .map_err(llvm_err)?;
        let ld_new_eb = unsafe {
            self.builder
                .build_gep(i8, ld_new_i8, &[i64.const_int(8, false)], "new_eb")
                .map_err(llvm_err)
        }?;
        let ld_dst = unsafe {
            self.builder
                .build_gep(self.string_type, ld_new_eb, &[zero], "dst")
                .map_err(llvm_err)
        }?;
        let ld_copy_bytes = self
            .builder
            .build_int_mul(ld_new_count, i64.const_int(16, false), "copy_bytes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                ld_memcpy_fn,
                &[ld_dst.into(), ld_src.into(), ld_copy_bytes.into()],
                "",
            )
            .map_err(llvm_err)?;
        let ld_ci_i = self.builder.build_alloca(i64, "ci_i").map_err(llvm_err)?;
        self.builder.build_store(ld_ci_i, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ld_h0_ci_loop);
        self.builder.position_at_end(ld_h0_ci_loop);
        let ld_ci = self
            .builder
            .build_load(i64, ld_ci_i, "ci")
            .map_err(llvm_err)?
            .into_int_value();
        let ld_ci_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ld_ci, ld_new_count, "ci_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ld_ci_cond, ld_h0_ci_body, ld_h0_ci_done);
        self.builder.position_at_end(ld_h0_ci_body);
        let ld_str_rc_inc_fn = self.module.get_function("action_string_rc_inc").unwrap();
        let ld_ci_ep = unsafe {
            self.builder
                .build_gep(self.string_type, ld_new_eb, &[ld_ci], "ci_ep")
                .map_err(llvm_err)
        }?;
        let ld_ci_ev = self
            .builder
            .build_load(self.string_type, ld_ci_ep, "ci_ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let _ = self
            .builder
            .build_call(ld_str_rc_inc_fn, &[ld_ci_ev.into()], "")
            .map_err(llvm_err)?;
        let ld_ci_next = self
            .builder
            .build_int_add(ld_ci, one, "ci_next")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ld_ci_i, ld_ci_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ld_h0_ci_loop);
        self.builder.position_at_end(ld_h0_ci_done);
        let ld_new_count_i32 = self
            .builder
            .build_int_truncate(ld_new_count, i32, "new_count_i32")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(ld_new_i8, ld_new_count_i32)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ld_h0_done);
        self.builder.position_at_end(ld_h0_done);
        let undef_drop = self.list_type.get_undef();
        let ld_r1 = self
            .builder
            .build_insert_value(undef_drop, ld_new_leaf, 0, "r1")
            .map_err(llvm_err)?;
        let ld_r2 = self
            .builder
            .build_insert_value(ld_r1, ld_new_count, 1, "r2")
            .map_err(llvm_err)?;
        let ld_r3 = self
            .builder
            .build_insert_value(ld_r2, i64.const_int(0, false), 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ld_r3));
        // === h>0: B-tree range walk ===
        self.builder.position_at_end(ld_hgt0);
        let ld_new = self
            .builder
            .build_call(list_create_fn, &[zero.into()], "new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let ld_acc = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        self.builder.build_store(ld_acc, ld_new).map_err(llvm_err)?;
        let ld_skip_a = self.builder.build_alloca(i64, "skip_a").map_err(llvm_err)?;
        self.builder
            .build_store(ld_skip_a, ld_start)
            .map_err(llvm_err)?;
        let ld_limit_a = self
            .builder
            .build_alloca(i64, "limit_a")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ld_limit_a, ld_remain)
            .map_err(llvm_err)?;
        let ld_range_fn = self
            .module
            .get_function("action_list_range_walk_rec")
            .unwrap();
        let _ = self
            .builder
            .build_call(
                ld_range_fn,
                &[
                    ld_acc.into(),
                    ld_node.into(),
                    ld_height.into(),
                    ld_skip_a.into(),
                    ld_limit_a.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
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
