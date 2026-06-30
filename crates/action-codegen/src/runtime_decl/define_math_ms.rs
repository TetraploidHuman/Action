// Submodule: runtime_decl/define_math_ms
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::values::BasicValue;
use inkwell::{FloatPredicate, IntPredicate};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_math_ms(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let f64 = self.f64_ty();
        let _void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let _i32 = self.context.i32_type();
        let _i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let _list_create_fn = self.module.get_function("action_list_create").unwrap();
        let _list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        // ---- action_max_tree_height(ptr node, i64 height) -> i64 ----
        // Returns the maximum real tree height in a ConcatNode DAG.
        // Recursive: walks ConcatNode chain, returns max of left/right subtree heights.
        let mth_fn = self.module.get_function("action_max_tree_height").unwrap();
        let mth_entry = self.context.append_basic_block(mth_fn, "entry");
        let mth_concat = self.context.append_basic_block(mth_fn, "concat");
        let mth_ret = self.context.append_basic_block(mth_fn, "ret");
        self.builder.position_at_end(mth_entry);
        let mth_node = mth_fn.get_first_param().unwrap().into_pointer_value();
        let mth_h = mth_fn.get_nth_param(1).unwrap().into_int_value();
        let mth_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                mth_h,
                i64.const_int(-1i64 as u64, true),
                "mth_ic",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mth_is_concat, mth_concat, mth_ret);
        self.builder.position_at_end(mth_concat);
        // Load left: offset 16 = node, offset 32 = height
        let mth_ln = unsafe {
            self.builder
                .build_gep(ptr, mth_node, &[i64.const_int(2, false)], "mth_ln")
                .map_err(llvm_err)
        }?;
        let mth_ln_v = self
            .builder
            .build_load(ptr, mth_ln, "mth_lnv")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mth_lh = unsafe {
            self.builder
                .build_gep(i64, mth_node, &[i64.const_int(4, false)], "mth_lh")
                .map_err(llvm_err)
        }?;
        let mth_lh_v = self
            .builder
            .build_load(i64, mth_lh, "mth_lhv")
            .map_err(llvm_err)?
            .into_int_value();
        let mth_l = self
            .builder
            .build_call(mth_fn, &[mth_ln_v.into(), mth_lh_v.into()], "mth_l")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        // Load right: offset 40 = node, offset 56 = height
        let mth_rn = unsafe {
            self.builder
                .build_gep(ptr, mth_node, &[i64.const_int(5, false)], "mth_rn")
                .map_err(llvm_err)
        }?;
        let mth_rn_v = self
            .builder
            .build_load(ptr, mth_rn, "mth_rnv")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mth_rh = unsafe {
            self.builder
                .build_gep(i64, mth_node, &[i64.const_int(7, false)], "mth_rh")
                .map_err(llvm_err)
        }?;
        let mth_rh_v = self
            .builder
            .build_load(i64, mth_rh, "mth_rhv")
            .map_err(llvm_err)?
            .into_int_value();
        let mth_r = self
            .builder
            .build_call(mth_fn, &[mth_rn_v.into(), mth_rh_v.into()], "mth_r")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let mth_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, mth_l, mth_r, "mth_gt")
            .map_err(llvm_err)?;
        let mth_max = self
            .builder
            .build_select(mth_gt, mth_l, mth_r, "mth_max")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_return(Some(&mth_max));
        self.builder.position_at_end(mth_ret);
        let _ = self.builder.build_return(Some(&mth_h));

        // ---- action_abs_f(f64) -> f64 ----
        let af_fn =
            self.module
                .add_function("action_abs_f", f64.fn_type(&[f64.into()], false), None);
        let af_entry = self.context.append_basic_block(af_fn, "entry");
        self.builder.position_at_end(af_entry);
        let af_val = af_fn.get_first_param().unwrap().into_float_value();
        let af_zero = f64.const_zero();
        let af_neg = self
            .builder
            .build_float_neg(af_val, "neg")
            .map_err(llvm_err)?;
        let af_cmp = self
            .builder
            .build_float_compare(FloatPredicate::OLT, af_val, af_zero, "cmp")
            .map_err(llvm_err)?;
        let af_r = self
            .builder
            .build_select(af_cmp, af_neg, af_val, "r")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&af_r));

        // ---- action_map_keys({ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Open-addressing: scan i=0..cap-1, skip empty/tombstone slots.
        let mk_fn = self.module.add_function(
            "action_map_keys",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let mk_entry = self.context.append_basic_block(mk_fn, "entry");
        self.builder.position_at_end(mk_entry);
        let mk_in = mk_fn.get_first_param().unwrap().into_struct_value();
        let mk_data = self
            .builder
            .build_extract_value(mk_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mk_cap = self
            .builder
            .build_extract_value(mk_in, 2, "cap")
            .map_err(llvm_err)?
            .into_int_value();
        let mk_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let mk_resv = mk_res.try_as_basic_value().unwrap_basic();
        let mk_ra = self
            .builder
            .build_alloca(self.list_type, "mk_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(mk_ra, mk_resv).map_err(llvm_err)?;
        let mk_i = self.builder.build_alloca(i64, "mk_i").map_err(llvm_err)?;
        self.builder
            .build_store(mk_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let mk_loop = self.context.append_basic_block(mk_fn, "loop");
        let mk_chk = self.context.append_basic_block(mk_fn, "chk");
        let mk_body = self.context.append_basic_block(mk_fn, "body");
        let mk_skip = self.context.append_basic_block(mk_fn, "skip");
        let mk_done = self.context.append_basic_block(mk_fn, "done");
        let _ = self.builder.build_unconditional_branch(mk_loop);
        self.builder.position_at_end(mk_loop);
        let mk_iv = self
            .builder
            .build_load(i64, mk_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mk_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mk_iv, mk_cap, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mk_cond, mk_chk, mk_done);
        self.builder.position_at_end(mk_chk);
        self.ht_branch_if_slot_active(mk_data, mk_iv, mk_body, mk_skip)?;
        self.builder.position_at_end(mk_body);
        let mk_key = self.ht_key_fat_at(mk_data, mk_iv)?;
        let mk_cl = self
            .builder
            .build_load(self.list_type, mk_ra, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let mk_ps = self.call_rt("action_list_push", &[mk_cl.into(), mk_key.into()])?;
        self.builder
            .build_store(mk_ra, mk_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mk_skip);
        self.builder.position_at_end(mk_skip);
        let mk_inc2 = self
            .builder
            .build_int_add(mk_iv, i64.const_int(1, false), "inc2")
            .map_err(llvm_err)?;
        self.builder.build_store(mk_i, mk_inc2).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mk_loop);
        self.builder.position_at_end(mk_done);
        let mk_rt = self
            .builder
            .build_load(self.list_type, mk_ra, "mk_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mk_rt));

        // ---- action_map_values({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let mv_fn = self.module.add_function(
            "action_map_values",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let mv_entry = self.context.append_basic_block(mv_fn, "entry");
        self.builder.position_at_end(mv_entry);
        let mv_in = mv_fn.get_first_param().unwrap().into_struct_value();
        let mv_data = self
            .builder
            .build_extract_value(mv_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mv_cap = self
            .builder
            .build_extract_value(mv_in, 2, "cap")
            .map_err(llvm_err)?
            .into_int_value();
        let mv_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let mv_resv = mv_res.try_as_basic_value().unwrap_basic();
        let mv_ra = self
            .builder
            .build_alloca(self.list_type, "mv_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(mv_ra, mv_resv).map_err(llvm_err)?;
        let mv_i = self.builder.build_alloca(i64, "mv_i").map_err(llvm_err)?;
        self.builder
            .build_store(mv_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let mv_loop = self.context.append_basic_block(mv_fn, "loop");
        let mv_chk = self.context.append_basic_block(mv_fn, "chk");
        let mv_body = self.context.append_basic_block(mv_fn, "body");
        let mv_skip = self.context.append_basic_block(mv_fn, "skip");
        let mv_done = self.context.append_basic_block(mv_fn, "done");
        let _ = self.builder.build_unconditional_branch(mv_loop);
        self.builder.position_at_end(mv_loop);
        let mv_iv = self
            .builder
            .build_load(i64, mv_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mv_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mv_iv, mv_cap, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mv_cond, mv_chk, mv_done);
        self.builder.position_at_end(mv_chk);
        self.ht_branch_if_slot_active(mv_data, mv_iv, mv_body, mv_skip)?;
        self.builder.position_at_end(mv_body);
        let mv_val = self.ht_val_fat_at(mv_data, mv_iv)?;
        let mv_cl = self
            .builder
            .build_load(self.list_type, mv_ra, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let mv_ps = self.call_rt("action_list_push", &[mv_cl.into(), mv_val.into()])?;
        self.builder
            .build_store(mv_ra, mv_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mv_skip);
        self.builder.position_at_end(mv_skip);
        let mv_inc = self
            .builder
            .build_int_add(mv_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(mv_i, mv_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mv_loop);
        self.builder.position_at_end(mv_done);
        let mv_rt = self
            .builder
            .build_load(self.list_type, mv_ra, "mv_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mv_rt));

        // ---- action_map_entries({ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Flat table: key+value from slot i.
        let me_fn = self.module.add_function(
            "action_map_entries",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let me_entry = self.context.append_basic_block(me_fn, "entry");
        self.builder.position_at_end(me_entry);
        let me_in = me_fn.get_first_param().unwrap().into_struct_value();
        let me_data = self
            .builder
            .build_extract_value(me_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let me_cap = self
            .builder
            .build_extract_value(me_in, 2, "cap")
            .map_err(llvm_err)?
            .into_int_value();
        let me_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let me_resv = me_res.try_as_basic_value().unwrap_basic();
        let me_ra = self
            .builder
            .build_alloca(self.list_type, "me_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(me_ra, me_resv).map_err(llvm_err)?;
        let me_i = self.builder.build_alloca(i64, "me_i").map_err(llvm_err)?;
        self.builder
            .build_store(me_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let me_loop = self.context.append_basic_block(me_fn, "loop");
        let me_chk = self.context.append_basic_block(me_fn, "chk");
        let me_body = self.context.append_basic_block(me_fn, "body");
        let me_skip = self.context.append_basic_block(me_fn, "skip");
        let me_done = self.context.append_basic_block(me_fn, "done");
        let _ = self.builder.build_unconditional_branch(me_loop);
        self.builder.position_at_end(me_loop);
        let me_iv = self
            .builder
            .build_load(i64, me_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let me_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, me_iv, me_cap, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(me_cond, me_chk, me_done);
        self.builder.position_at_end(me_chk);
        self.ht_branch_if_slot_active(me_data, me_iv, me_body, me_skip)?;
        self.builder.position_at_end(me_body);
        let me_key = self.ht_key_fat_at(me_data, me_iv)?;
        let me_val = self.ht_val_fat_at(me_data, me_iv)?;
        // Build tuple: allocate 2 fat structs and point to them
        let me_tuple_ty = self
            .context
            .struct_type(&[self.string_type.into(), self.string_type.into()], false);
        let me_tuple_ptr = self
            .builder
            .build_call(malloc_rc_fn, &[i64.const_int(32, false).into()], "tup")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for newly allocated tuple
        let me_tup_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(me_tuple_ptr, i64, "me_tup_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "me_tup_rc_addr",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(
                self.builder
                    .build_int_to_ptr(me_tup_rc_addr, ptr, "")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
            )
            .map_err(llvm_err)?;
        let me_tup_a = self
            .builder
            .build_struct_gep(me_tuple_ty, me_tuple_ptr, 0, "ta")
            .map_err(llvm_err)?;
        let me_tup_b = self
            .builder
            .build_struct_gep(me_tuple_ty, me_tuple_ptr, 1, "tb")
            .map_err(llvm_err)?;
        self.builder
            .build_store(me_tup_a, me_key)
            .map_err(llvm_err)?;
        self.builder
            .build_store(me_tup_b, me_val)
            .map_err(llvm_err)?;
        // Wrap in a fat struct: tag=5 (Struct), data=tuple_ptr
        let me_fat_undef = self.string_type.get_undef();
        let me_fat1 = self
            .builder
            .build_insert_value(me_fat_undef, i64.const_int(5, false), 0, "ftag")
            .map_err(llvm_err)?;
        let me_fat2 = self
            .builder
            .build_insert_value(me_fat1, me_tuple_ptr, 1, "fdata")
            .map_err(llvm_err)?;
        let me_cl = self
            .builder
            .build_load(self.list_type, me_ra, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let me_ps = self.call_rt(
            "action_list_push",
            &[me_cl.into(), me_fat2.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(me_ra, me_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(me_skip);
        self.builder.position_at_end(me_skip);
        let me_inc = self
            .builder
            .build_int_add(me_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(me_i, me_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(me_loop);
        self.builder.position_at_end(me_done);
        let me_rt = self
            .builder
            .build_load(self.list_type, me_ra, "me_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&me_rt));

        // ---- action_set_union({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Sets use map layout (4×i64 per entry). Result must be in map format.
        let su_fn = self.module.add_function(
            "action_set_union",
            self.list_type
                .fn_type(&[self.list_type.into(), self.list_type.into()], false),
            None,
        );
        let su_entry = self.context.append_basic_block(su_fn, "entry");
        self.builder.position_at_end(su_entry);
        let su_a = su_fn.get_first_param().unwrap().into_struct_value();
        let su_b = su_fn.get_nth_param(1).unwrap().into_struct_value();
        let su_adata = self
            .builder
            .build_extract_value(su_a, 0, "adata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let su_acap = self
            .builder
            .build_extract_value(su_a, 2, "acap")
            .map_err(llvm_err)?
            .into_int_value();
        let su_bdata = self
            .builder
            .build_extract_value(su_b, 0, "bdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let su_blen = self
            .builder
            .build_extract_value(su_b, 1, "blen")
            .map_err(llvm_err)?
            .into_int_value();
        let su_bcap = self
            .builder
            .build_extract_value(su_b, 2, "bcap")
            .map_err(llvm_err)?
            .into_int_value();
        let su_alen = self
            .builder
            .build_extract_value(su_a, 1, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let su_cap = self
            .builder
            .build_int_add(su_alen, su_blen, "cap")
            .map_err(llvm_err)?;
        let su_cap4 = self
            .builder
            .build_int_add(su_cap, i64.const_int(4, false), "cap4")
            .map_err(llvm_err)?;
        let map_create_fn = self.module.get_function("action_map_create").unwrap();
        let bulk_fn = self
            .module
            .get_function("action_ht_bulk_copy_active_slots")
            .unwrap();
        let su_res = self
            .builder
            .build_call(map_create_fn, &[su_cap4.into()], "res")
            .map_err(llvm_err)?;
        let su_resv = su_res.try_as_basic_value().unwrap_basic();
        let su_ra = self
            .builder
            .build_alloca(self.list_type, "su_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(su_ra, su_resv).map_err(llvm_err)?;
        let su_loaded = self
            .builder
            .build_load(self.list_type, su_ra, "su_loaded")
            .map_err(llvm_err)?
            .into_struct_value();
        let su_dest_data = self
            .builder
            .build_extract_value(su_loaded, 0, "dest_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let su_dest_cap = self
            .builder
            .build_extract_value(su_loaded, 2, "dest_cap")
            .map_err(llvm_err)?
            .into_int_value();
        let su_len_p = self
            .builder
            .build_struct_gep(self.list_type, su_ra, 1, "len_p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                bulk_fn,
                &[
                    su_dest_data.into(),
                    su_dest_cap.into(),
                    su_len_p.into(),
                    su_adata.into(),
                    su_acap.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                bulk_fn,
                &[
                    su_dest_data.into(),
                    su_dest_cap.into(),
                    su_len_p.into(),
                    su_bdata.into(),
                    su_bcap.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let su_rt = self
            .builder
            .build_load(self.list_type, su_ra, "su_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&su_rt));

        // ---- action_set_intersection({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Sets use map layout (4×i64 per entry). Result must be in map format.
        let si_fn = self.module.add_function(
            "action_set_intersection",
            self.list_type
                .fn_type(&[self.list_type.into(), self.list_type.into()], false),
            None,
        );
        let si_entry = self.context.append_basic_block(si_fn, "entry");
        self.builder.position_at_end(si_entry);
        let si_a = si_fn.get_first_param().unwrap().into_struct_value();
        let si_b = si_fn.get_nth_param(1).unwrap().into_struct_value();
        let si_adata = self
            .builder
            .build_extract_value(si_a, 0, "adata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let si_acap = self
            .builder
            .build_extract_value(si_a, 2, "acap")
            .map_err(llvm_err)?
            .into_int_value();
        let si_alen = self
            .builder
            .build_extract_value(si_a, 1, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let si_blen = self
            .builder
            .build_extract_value(si_b, 1, "blen")
            .map_err(llvm_err)?
            .into_int_value();
        let si_min_len = self
            .builder
            .build_int_compare(IntPredicate::SLT, si_alen, si_blen, "min_cmp")
            .map_err(llvm_err)?;
        let si_est = self
            .builder
            .build_select(si_min_len, si_alen, si_blen, "est")
            .map_err(llvm_err)?
            .into_int_value();
        let si_cap4 = self
            .builder
            .build_int_add(si_est, i64.const_int(4, false), "cap4")
            .map_err(llvm_err)?;
        let map_create_fn = self.module.get_function("action_map_create").unwrap();
        let ht_insert_fn = self.module.get_function("action_ht_insert").unwrap();
        let mc_fn = self.module.get_function("action_map_contains").unwrap();
        let si_res = self
            .builder
            .build_call(map_create_fn, &[si_cap4.into()], "res")
            .map_err(llvm_err)?;
        let si_resv = si_res.try_as_basic_value().unwrap_basic();
        let si_ra = self
            .builder
            .build_alloca(self.list_type, "si_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(si_ra, si_resv).map_err(llvm_err)?;
        let si_null = {
            let u = str_ty.get_undef();
            let u1 = self
                .builder
                .build_insert_value(u, i64.const_int(0, false), 0, "n0")
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(u1, self.ptr_ty().const_zero(), 1, "n1")
                .map_err(llvm_err)?
        };
        let si_i = self.builder.build_alloca(i64, "si_i").map_err(llvm_err)?;
        self.builder
            .build_store(si_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let si_loop = self.context.append_basic_block(si_fn, "loop");
        let si_chk = self.context.append_basic_block(si_fn, "chk");
        let si_body = self.context.append_basic_block(si_fn, "body");
        let si_skip_slot = self.context.append_basic_block(si_fn, "skip_slot");
        let si_done = self.context.append_basic_block(si_fn, "done");
        let _ = self.builder.build_unconditional_branch(si_loop);
        self.builder.position_at_end(si_loop);
        let si_iv = self
            .builder
            .build_load(i64, si_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let si_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, si_iv, si_acap, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(si_cond, si_chk, si_done);
        self.builder.position_at_end(si_chk);
        self.ht_branch_if_slot_active(si_adata, si_iv, si_body, si_skip_slot)?;
        self.builder.position_at_end(si_body);
        let si_key = self.ht_key_fat_at(si_adata, si_iv)?;
        // Check if element is in B (use map_contains for correct layout)
        let si_contains = self
            .builder
            .build_call(
                mc_fn,
                &[si_b.as_basic_value_enum().into(), si_key.into()],
                "cont",
            )
            .map_err(llvm_err)?;
        let si_found = si_contains
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let si_add = self.context.append_basic_block(si_fn, "add");
        let si_skip_miss = self.context.append_basic_block(si_fn, "skip_miss");
        let _ = self
            .builder
            .build_conditional_branch(si_found, si_add, si_skip_miss);
        self.builder.position_at_end(si_add);
        let si_cl2 = self
            .builder
            .build_load(self.list_type, si_ra, "cl2")
            .map_err(llvm_err)?
            .into_struct_value();
        let si_ins = self
            .builder
            .build_call(
                ht_insert_fn,
                &[
                    si_cl2.into(),
                    si_key.into(),
                    si_null.as_basic_value_enum().into(),
                ],
                "ins",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(si_ra, si_ins.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(si_skip_slot);
        self.builder.position_at_end(si_skip_miss);
        let _ = self.builder.build_unconditional_branch(si_skip_slot);
        self.builder.position_at_end(si_skip_slot);
        let si_inc = self
            .builder
            .build_int_add(si_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(si_i, si_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(si_loop);
        self.builder.position_at_end(si_done);
        let si_rt = self
            .builder
            .build_load(self.list_type, si_ra, "si_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&si_rt));

        // ---- action_set_difference({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Sets use map layout (4×i64 per entry). Result must be in map format.
        let sd_fn = self.module.add_function(
            "action_set_difference",
            self.list_type
                .fn_type(&[self.list_type.into(), self.list_type.into()], false),
            None,
        );
        let sd_entry = self.context.append_basic_block(sd_fn, "entry");
        self.builder.position_at_end(sd_entry);
        let sd_a = sd_fn.get_first_param().unwrap().into_struct_value();
        let sd_b = sd_fn.get_nth_param(1).unwrap().into_struct_value();
        let sd_adata = self
            .builder
            .build_extract_value(sd_a, 0, "adata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sd_acap = self
            .builder
            .build_extract_value(sd_a, 2, "acap")
            .map_err(llvm_err)?
            .into_int_value();
        let sd_cap4 = self
            .builder
            .build_int_add(
                self.builder
                    .build_extract_value(sd_a, 1, "alen")
                    .map_err(llvm_err)?
                    .into_int_value(),
                i64.const_int(4, false),
                "cap4",
            )
            .map_err(llvm_err)?;
        let map_create_fn = self.module.get_function("action_map_create").unwrap();
        let mi_fn = self.module.get_function("action_map_insert").unwrap();
        let mc_fn = self.module.get_function("action_map_contains").unwrap();
        let sd_res = self
            .builder
            .build_call(map_create_fn, &[sd_cap4.into()], "res")
            .map_err(llvm_err)?;
        let sd_resv = sd_res.try_as_basic_value().unwrap_basic();
        let sd_ra = self
            .builder
            .build_alloca(self.list_type, "sd_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(sd_ra, sd_resv).map_err(llvm_err)?;
        let sd_null = {
            let u = str_ty.get_undef();
            let u1 = self
                .builder
                .build_insert_value(u, i64.const_int(0, false), 0, "n0")
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(u1, self.ptr_ty().const_zero(), 1, "n1")
                .map_err(llvm_err)?
        };
        let sd_i = self.builder.build_alloca(i64, "sd_i").map_err(llvm_err)?;
        self.builder
            .build_store(sd_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sd_loop = self.context.append_basic_block(sd_fn, "loop");
        let sd_chk = self.context.append_basic_block(sd_fn, "chk");
        let sd_body = self.context.append_basic_block(sd_fn, "body");
        let sd_skip_slot = self.context.append_basic_block(sd_fn, "skip_slot");
        let sd_done = self.context.append_basic_block(sd_fn, "done");
        let _ = self.builder.build_unconditional_branch(sd_loop);
        self.builder.position_at_end(sd_loop);
        let sd_iv = self
            .builder
            .build_load(i64, sd_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let sd_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, sd_iv, sd_acap, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sd_cond, sd_chk, sd_done);
        self.builder.position_at_end(sd_chk);
        self.ht_branch_if_slot_active(sd_adata, sd_iv, sd_body, sd_skip_slot)?;
        self.builder.position_at_end(sd_body);
        let sd_key = self.ht_key_fat_at(sd_adata, sd_iv)?;
        // Check if element is NOT in B (use map_contains for correct layout)
        let sd_contains = self
            .builder
            .build_call(
                mc_fn,
                &[sd_b.as_basic_value_enum().into(), sd_key.into()],
                "cont",
            )
            .map_err(llvm_err)?;
        let sd_not_cont = self
            .builder
            .build_not(
                sd_contains
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value(),
                "nc",
            )
            .map_err(llvm_err)?;
        let sd_add = self.context.append_basic_block(sd_fn, "add");
        let sd_skip_in = self.context.append_basic_block(sd_fn, "skip_in");
        let _ = self
            .builder
            .build_conditional_branch(sd_not_cont, sd_add, sd_skip_in);
        self.builder.position_at_end(sd_add);
        let sd_cl2 = self
            .builder
            .build_load(self.list_type, sd_ra, "cl2")
            .map_err(llvm_err)?
            .into_struct_value();
        let sd_ins = self
            .builder
            .build_call(
                mi_fn,
                &[
                    sd_cl2.into(),
                    sd_key.into(),
                    sd_null.as_basic_value_enum().into(),
                ],
                "ins",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(sd_ra, sd_ins.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sd_skip_slot);
        self.builder.position_at_end(sd_skip_in);
        let _ = self.builder.build_unconditional_branch(sd_skip_slot);
        self.builder.position_at_end(sd_skip_slot);
        let sd_inc = self
            .builder
            .build_int_add(sd_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(sd_i, sd_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sd_loop);
        self.builder.position_at_end(sd_done);
        let sd_rt = self
            .builder
            .build_load(self.list_type, sd_ra, "sd_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sd_rt));
        // ---- action_set_is_subset({ptr, i64, i64}, {ptr, i64, i64}) -> i1 ----
        // Open-addressing: scan slots 0..cap-1 in A.
        let ss_fn = self.module.add_function(
            "action_set_is_subset",
            self.context
                .bool_type()
                .fn_type(&[self.list_type.into(), self.list_type.into()], false),
            None,
        );
        let ss_entry = self.context.append_basic_block(ss_fn, "entry");
        self.builder.position_at_end(ss_entry);
        let ss_a = ss_fn.get_first_param().unwrap().into_struct_value();
        let ss_b = ss_fn.get_nth_param(1).unwrap().into_struct_value();
        let ss_acap = self
            .builder
            .build_extract_value(ss_a, 2, "acap")
            .map_err(llvm_err)?
            .into_int_value();
        let ss_adata = self
            .builder
            .build_extract_value(ss_a, 0, "ad")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ss_ht_contains = self.module.get_function("action_ht_contains").unwrap();
        let ss_i = self.builder.build_alloca(i64, "ss_i").map_err(llvm_err)?;
        self.builder
            .build_store(ss_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let ss_loop = self.context.append_basic_block(ss_fn, "loop");
        let ss_chk = self.context.append_basic_block(ss_fn, "chk");
        let ss_body = self.context.append_basic_block(ss_fn, "body");
        let ss_skip = self.context.append_basic_block(ss_fn, "skip");
        let ss_fail = self.context.append_basic_block(ss_fn, "fail");
        let ss_ok = self.context.append_basic_block(ss_fn, "ok");
        let _ = self.builder.build_unconditional_branch(ss_loop);
        self.builder.position_at_end(ss_loop);
        let ss_iv = self
            .builder
            .build_load(i64, ss_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let ss_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ss_iv, ss_acap, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ss_cond, ss_chk, ss_ok);
        self.builder.position_at_end(ss_chk);
        self.ht_branch_if_slot_active(ss_adata, ss_iv, ss_body, ss_skip)?;
        self.builder.position_at_end(ss_body);
        let ss_key = self.ht_key_fat_at(ss_adata, ss_iv)?;
        let ss_cont = self
            .builder
            .build_call(ss_ht_contains, &[ss_b.into(), ss_key.into()], "cont")
            .map_err(llvm_err)?;
        let ss_found = ss_cont.try_as_basic_value().unwrap_basic().into_int_value();
        let ss_next = self.context.append_basic_block(ss_fn, "next");
        let _ = self
            .builder
            .build_conditional_branch(ss_found, ss_next, ss_fail);
        self.builder.position_at_end(ss_next);
        let _ = self.builder.build_unconditional_branch(ss_skip);
        self.builder.position_at_end(ss_skip);
        let ss_inc = self
            .builder
            .build_int_add(ss_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(ss_i, ss_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ss_loop);
        self.builder.position_at_end(ss_fail);
        let _ = self
            .builder
            .build_return(Some(&self.context.bool_type().const_int(0, false)));
        self.builder.position_at_end(ss_ok);
        let _ = self
            .builder
            .build_return(Some(&self.context.bool_type().const_int(1, false)));

        // ---- action_rand_shuffle({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let rs_fn = self.module.add_function(
            "action_rand_shuffle",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let rs_entry = self.context.append_basic_block(rs_fn, "entry");
        self.builder.position_at_end(rs_entry);
        let rs_in = rs_fn.get_first_param().unwrap().into_struct_value();
        let rs_len = self
            .builder
            .build_extract_value(rs_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        // Copy input list
        let rs_copy = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let rs_copyv = rs_copy.try_as_basic_value().unwrap_basic();
        let rs_ra = self
            .builder
            .build_alloca(self.list_type, "rs_ra")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rs_ra, rs_copyv)
            .map_err(llvm_err)?;
        // Copy all elements
        let rs_ci = self.builder.build_alloca(i64, "rs_ci").map_err(llvm_err)?;
        self.builder
            .build_store(rs_ci, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let rs_cloop = self.context.append_basic_block(rs_fn, "cloop");
        let rs_cbody = self.context.append_basic_block(rs_fn, "cbody");
        let rs_cdone = self.context.append_basic_block(rs_fn, "cdone");
        let _ = self.builder.build_unconditional_branch(rs_cloop);
        self.builder.position_at_end(rs_cloop);
        let rs_civ = self
            .builder
            .build_load(i64, rs_ci, "civ")
            .map_err(llvm_err)?
            .into_int_value();
        let rs_ccond = self
            .builder
            .build_int_compare(IntPredicate::SLT, rs_civ, rs_len, "ccond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rs_ccond, rs_cbody, rs_cdone);
        self.builder.position_at_end(rs_cbody);
        let rs_get_fn = self.module.get_function("action_list_get").unwrap();
        let rs_cev = self
            .builder
            .build_call(rs_get_fn, &[rs_in.into(), rs_civ.into()], "cev")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let rs_ccl = self
            .builder
            .build_load(self.list_type, rs_ra, "ccl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rs_cps = self.call_rt(
            "action_list_push",
            &[rs_ccl.into(), rs_cev.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(rs_ra, rs_cps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let rs_cinc = self
            .builder
            .build_int_add(rs_civ, i64.const_int(1, false), "cinc")
            .map_err(llvm_err)?;
        self.builder.build_store(rs_ci, rs_cinc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rs_cloop);
        self.builder.position_at_end(rs_cdone);
        // Fisher-Yates shuffle: iterate from end to start
        let rs_i = self.builder.build_alloca(i64, "rs_i").map_err(llvm_err)?;
        let rs_len1 = self
            .builder
            .build_int_sub(rs_len, i64.const_int(1, false), "len1")
            .map_err(llvm_err)?;
        self.builder.build_store(rs_i, rs_len1).map_err(llvm_err)?;
        let rs_floop = self.context.append_basic_block(rs_fn, "floop");
        let rs_fbody = self.context.append_basic_block(rs_fn, "fbody");
        let rs_fdone = self.context.append_basic_block(rs_fn, "fdone");
        let _ = self.builder.build_unconditional_branch(rs_floop);
        self.builder.position_at_end(rs_floop);
        let rs_iv = self
            .builder
            .build_load(i64, rs_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let rs_fcond = self
            .builder
            .build_int_compare(IntPredicate::SGT, rs_iv, i64.const_int(0, false), "fcond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rs_fcond, rs_fbody, rs_fdone);
        self.builder.position_at_end(rs_fbody);
        // Generate random index [0, i]
        let rs_rand = self.call_rt(
            "action_rand_int",
            &[i64.const_int(0, false).into(), rs_iv.into()],
        )?;
        let rs_j = rs_rand.try_as_basic_value().unwrap_basic().into_int_value();
        // Swap elements at i and j using tree-aware get/set
        let rs_cur = self
            .builder
            .build_load(self.list_type, rs_ra, "cur_list")
            .map_err(llvm_err)?
            .into_struct_value();
        let rs_get_fn2 = self.module.get_function("action_list_get").unwrap();
        let rs_ei = self
            .builder
            .build_call(rs_get_fn2, &[rs_cur.into(), rs_iv.into()], "ei")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let rs_ej = self
            .builder
            .build_call(rs_get_fn2, &[rs_cur.into(), rs_j.into()], "ej")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let rs_set_fn = self.module.get_function("action_list_set").unwrap();
        let rs_after_j = self
            .builder
            .build_call(
                rs_set_fn,
                &[rs_cur.into(), rs_iv.into(), rs_ej.into()],
                "after_j",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(rs_ra, rs_after_j)
            .map_err(llvm_err)?;
        let rs_cur2 = self
            .builder
            .build_load(self.list_type, rs_ra, "cur2")
            .map_err(llvm_err)?
            .into_struct_value();
        let rs_after_i = self
            .builder
            .build_call(
                rs_set_fn,
                &[rs_cur2.into(), rs_j.into(), rs_ei.into()],
                "after_i",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(rs_ra, rs_after_i)
            .map_err(llvm_err)?;
        let rs_dec = self
            .builder
            .build_int_sub(rs_iv, i64.const_int(1, false), "dec")
            .map_err(llvm_err)?;
        self.builder.build_store(rs_i, rs_dec).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rs_floop);
        self.builder.position_at_end(rs_fdone);
        let rs_rt = self
            .builder
            .build_load(self.list_type, rs_ra, "rs_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&rs_rt));

        Ok(())
    }
}
