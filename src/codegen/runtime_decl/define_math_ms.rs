// Submodule: runtime_decl/define_math_ms
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::{FloatPredicate, IntPredicate};
use inkwell::values::BasicValue;

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
            // Tree-based: keys are at even indices, step by 2.
            let mk_fn = self.module.add_function(
                "action_map_keys",
                self.list_type.fn_type(&[self.list_type.into()], false),
                None,
            );
            let mk_entry = self.context.append_basic_block(mk_fn, "entry");
            self.builder.position_at_end(mk_entry);
            let mk_in = mk_fn.get_first_param().unwrap().into_struct_value();
            let mk_len = self
                .builder
                .build_extract_value(mk_in, 1, "len")
                .map_err(llvm_err)?
                .into_int_value();
            let mk_res = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
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
            let mk_body = self.context.append_basic_block(mk_fn, "body");
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
                .build_int_compare(IntPredicate::SLT, mk_iv, mk_len, "cond")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(mk_cond, mk_body, mk_done);
            self.builder.position_at_end(mk_body);
            // Get key at even index via action_list_get (returns fat struct directly)
            let mk_get_fn = self.module.get_function("action_list_get").unwrap();
            let mk_key = self
                .builder
                .build_call(mk_get_fn, &[mk_in.into(), mk_iv.into()], "key")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .basic()
                .ok_or("get key failed")?;
            let mk_cl = self
                .builder
                .build_load(self.list_type, mk_ra, "cl")
                .map_err(llvm_err)?
                .into_struct_value();
            let mk_ps = self.call_rt("action_list_push", &[mk_cl.into(), mk_key.into()])?;
            self.builder
                .build_store(mk_ra, mk_ps.try_as_basic_value().unwrap_basic())
                .map_err(llvm_err)?;
            let mk_inc = self
                .builder
                .build_int_add(mk_iv, i64.const_int(2, false), "inc")
                .map_err(llvm_err)?;
            self.builder.build_store(mk_i, mk_inc).map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(mk_loop);
            self.builder.position_at_end(mk_done);
            let mk_rt = self
                .builder
                .build_load(self.list_type, mk_ra, "mk_rt")
                .map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&mk_rt));

            // ---- action_map_values({ptr, i64, i64}) -> {ptr, i64, i64} ----
            // Tree-based: values are at odd indices (1, 3, 5, ...), step by 2.
            let mv_fn = self.module.add_function(
                "action_map_values",
                self.list_type.fn_type(&[self.list_type.into()], false),
                None,
            );
            let mv_entry = self.context.append_basic_block(mv_fn, "entry");
            self.builder.position_at_end(mv_entry);
            let mv_in = mv_fn.get_first_param().unwrap().into_struct_value();
            let mv_len = self
                .builder
                .build_extract_value(mv_in, 1, "len")
                .map_err(llvm_err)?
                .into_int_value();
            let mv_res = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
            let mv_resv = mv_res.try_as_basic_value().unwrap_basic();
            let mv_ra = self
                .builder
                .build_alloca(self.list_type, "mv_ra")
                .map_err(llvm_err)?;
            self.builder.build_store(mv_ra, mv_resv).map_err(llvm_err)?;
            let mv_i = self.builder.build_alloca(i64, "mv_i").map_err(llvm_err)?;
            self.builder
                .build_store(mv_i, i64.const_int(1, false))
                .map_err(llvm_err)?;
            let mv_loop = self.context.append_basic_block(mv_fn, "loop");
            let mv_body = self.context.append_basic_block(mv_fn, "body");
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
                .build_int_compare(IntPredicate::SLT, mv_iv, mv_len, "cond")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(mv_cond, mv_body, mv_done);
            self.builder.position_at_end(mv_body);
            let mv_get_fn = self.module.get_function("action_list_get").unwrap();
            let mv_val = self
                .builder
                .build_call(mv_get_fn, &[mv_in.into(), mv_iv.into()], "val")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .basic()
                .ok_or("get val failed")?;
            let mv_cl = self
                .builder
                .build_load(self.list_type, mv_ra, "cl")
                .map_err(llvm_err)?
                .into_struct_value();
            let mv_ps = self.call_rt("action_list_push", &[mv_cl.into(), mv_val.into()])?;
            self.builder
                .build_store(mv_ra, mv_ps.try_as_basic_value().unwrap_basic())
                .map_err(llvm_err)?;
            let mv_inc = self
                .builder
                .build_int_add(mv_iv, i64.const_int(2, false), "inc")
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
            // Tree-based: step by 2, get key at i and value at i+1.
            let me_fn = self.module.add_function(
                "action_map_entries",
                self.list_type.fn_type(&[self.list_type.into()], false),
                None,
            );
            let me_entry = self.context.append_basic_block(me_fn, "entry");
            self.builder.position_at_end(me_entry);
            let me_in = me_fn.get_first_param().unwrap().into_struct_value();
            let me_len = self
                .builder
                .build_extract_value(me_in, 1, "len")
                .map_err(llvm_err)?
                .into_int_value();
            let me_res = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
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
            let me_body = self.context.append_basic_block(me_fn, "body");
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
                .build_int_compare(IntPredicate::SLT, me_iv, me_len, "cond")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(me_cond, me_body, me_done);
            self.builder.position_at_end(me_body);
            let me_get_fn = self.module.get_function("action_list_get").unwrap();
            let me_key = self
                .builder
                .build_call(me_get_fn, &[me_in.into(), me_iv.into()], "key")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .basic()
                .ok_or("get key failed")?;
            let me_vp1 = self
                .builder
                .build_int_add(me_iv, i64.const_int(1, false), "vp1")
                .map_err(llvm_err)?;
            let me_val = self
                .builder
                .build_call(me_get_fn, &[me_in.into(), me_vp1.into()], "val")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .basic()
                .ok_or("get val failed")?;
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
            let me_inc = self
                .builder
                .build_int_add(me_iv, i64.const_int(2, false), "inc")
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
                self.list_type.fn_type(&[self.list_type.into(), self.list_type.into()], false),
                None,
            );
            let su_entry = self.context.append_basic_block(su_fn, "entry");
            self.builder.position_at_end(su_entry);
            let su_a = su_fn.get_first_param().unwrap().into_struct_value();
            let su_b = su_fn.get_nth_param(1).unwrap().into_struct_value();
            let su_alen = self
                .builder
                .build_extract_value(su_a, 1, "alen")
                .map_err(llvm_err)?
                .into_int_value();
            let su_blen = self
                .builder
                .build_extract_value(su_b, 1, "blen")
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
            let mi_fn = self.module.get_function("action_map_insert").unwrap();
            let mc_fn = self.module.get_function("action_map_contains").unwrap();
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
            let su_null = {
                let u = str_ty.get_undef();
                let u1 = self
                    .builder
                    .build_insert_value(u, i64.const_int(0, false), 0, "n0")
                    .map_err(llvm_err)?;
                self.builder
                    .build_insert_value(u1, self.ptr_ty().const_zero(), 1, "n1")
                    .map_err(llvm_err)?
            };
            let su_get_fn = self.module.get_function("action_list_get").unwrap();
            // Add all from A (each set entry occupies 2 list elements: key + null)
            // su_alen = total list elements = 2 * num_entries
            let su_npairs1 = self
                .builder
                .build_int_signed_div(su_alen, i64.const_int(2, false), "npairs1")
                .map_err(llvm_err)?;
            let su_i = self.builder.build_alloca(i64, "su_i").map_err(llvm_err)?;
            self.builder
                .build_store(su_i, i64.const_int(0, false))
                .map_err(llvm_err)?;
            let su_loop1 = self.context.append_basic_block(su_fn, "loop1");
            let su_body1 = self.context.append_basic_block(su_fn, "body1");
            let su_done1 = self.context.append_basic_block(su_fn, "done1");
            let _ = self.builder.build_unconditional_branch(su_loop1);
            self.builder.position_at_end(su_loop1);
            let su_iv = self
                .builder
                .build_load(i64, su_i, "iv")
                .map_err(llvm_err)?
                .into_int_value();
            let su_c1 = self
                .builder
                .build_int_compare(IntPredicate::SLT, su_iv, su_npairs1, "c1")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(su_c1, su_body1, su_done1);
            self.builder.position_at_end(su_body1);
            let su_kidx = self
                .builder
                .build_int_mul(su_iv, i64.const_int(2, false), "kidx")
                .map_err(llvm_err)?;
            let su_key = self
                .builder
                .build_call(su_get_fn, &[su_a.into(), su_kidx.into()], "key")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic();
            let su_cl1 = self
                .builder
                .build_load(self.list_type, su_ra, "cl1")
                .map_err(llvm_err)?
                .into_struct_value();
            let su_ins = self
                .builder
                .build_call(
                    mi_fn,
                    &[
                        su_cl1.into(),
                        su_key.into(),
                        su_null.as_basic_value_enum().into(),
                    ],
                    "ins",
                )
                .map_err(llvm_err)?;
            self.builder
                .build_store(su_ra, su_ins.try_as_basic_value().unwrap_basic())
                .map_err(llvm_err)?;
            let su_inc = self
                .builder
                .build_int_add(su_iv, i64.const_int(1, false), "inc")
                .map_err(llvm_err)?;
            self.builder.build_store(su_i, su_inc).map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(su_loop1);
            // Add from B only if not already in result
            self.builder.position_at_end(su_done1);
            // Add from B only if not already in result
            let su_npairs2 = self
                .builder
                .build_int_signed_div(su_blen, i64.const_int(2, false), "npairs2")
                .map_err(llvm_err)?;
            let su_j = self.builder.build_alloca(i64, "su_j").map_err(llvm_err)?;
            self.builder
                .build_store(su_j, i64.const_int(0, false))
                .map_err(llvm_err)?;
            let su_loop2 = self.context.append_basic_block(su_fn, "loop2");
            let su_body2 = self.context.append_basic_block(su_fn, "body2");
            let su_done2 = self.context.append_basic_block(su_fn, "done2");
            let _ = self.builder.build_unconditional_branch(su_loop2);
            self.builder.position_at_end(su_loop2);
            let su_jv = self
                .builder
                .build_load(i64, su_j, "jv")
                .map_err(llvm_err)?
                .into_int_value();
            let su_c2 = self
                .builder
                .build_int_compare(IntPredicate::SLT, su_jv, su_npairs2, "c2")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(su_c2, su_body2, su_done2);
            self.builder.position_at_end(su_body2);
            let su_kidx2 = self
                .builder
                .build_int_mul(su_jv, i64.const_int(2, false), "kidx2")
                .map_err(llvm_err)?;
            let su_key2 = self
                .builder
                .build_call(su_get_fn, &[su_b.into(), su_kidx2.into()], "key2")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic();
            let su_cl2 = self
                .builder
                .build_load(self.list_type, su_ra, "cl2")
                .map_err(llvm_err)?
                .into_struct_value();
            let su_contains = self
                .builder
                .build_call(
                    mc_fn,
                    &[su_cl2.into(), su_key2.as_basic_value_enum().into()],
                    "cont",
                )
                .map_err(llvm_err)?;
            let su_not_cont = self
                .builder
                .build_not(
                    su_contains
                        .try_as_basic_value()
                        .unwrap_basic()
                        .into_int_value(),
                    "nc",
                )
                .map_err(llvm_err)?;
            let su_add = self.context.append_basic_block(su_fn, "add");
            let su_skip = self.context.append_basic_block(su_fn, "skip");
            let _ = self
                .builder
                .build_conditional_branch(su_not_cont, su_add, su_skip);
            self.builder.position_at_end(su_add);
            let su_cl3 = self
                .builder
                .build_load(self.list_type, su_ra, "cl3")
                .map_err(llvm_err)?
                .into_struct_value();
            let su_ins2 = self
                .builder
                .build_call(
                    mi_fn,
                    &[
                        su_cl3.into(),
                        su_key2.into(),
                        su_null.as_basic_value_enum().into(),
                    ],
                    "ins2",
                )
                .map_err(llvm_err)?;
            self.builder
                .build_store(su_ra, su_ins2.try_as_basic_value().unwrap_basic())
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(su_skip);
            self.builder.position_at_end(su_skip);
            let su_inc2 = self
                .builder
                .build_int_add(su_jv, i64.const_int(1, false), "inc2")
                .map_err(llvm_err)?;
            self.builder.build_store(su_j, su_inc2).map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(su_loop2);
            self.builder.position_at_end(su_done2);
            let su_rt = self
                .builder
                .build_load(self.list_type, su_ra, "su_rt")
                .map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&su_rt));

            // ---- action_set_intersection({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
            // Sets use map layout (4×i64 per entry). Result must be in map format.
            let si_fn = self.module.add_function(
                "action_set_intersection",
                self.list_type.fn_type(&[self.list_type.into(), self.list_type.into()], false),
                None,
            );
            let si_entry = self.context.append_basic_block(si_fn, "entry");
            self.builder.position_at_end(si_entry);
            let si_a = si_fn.get_first_param().unwrap().into_struct_value();
            let si_b = si_fn.get_nth_param(1).unwrap().into_struct_value();
            let si_alen = self
                .builder
                .build_extract_value(si_a, 1, "alen")
                .map_err(llvm_err)?
                .into_int_value();
            let si_cap4 = self
                .builder
                .build_int_add(si_alen, i64.const_int(4, false), "cap4")
                .map_err(llvm_err)?;
            let map_create_fn = self.module.get_function("action_map_create").unwrap();
            let mi_fn = self.module.get_function("action_map_insert").unwrap();
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
            let si_get_fn = self.module.get_function("action_list_get").unwrap();
            // Each set entry occupies 2 list elements; iterate num_entries = alen/2
            let si_npairs = self
                .builder
                .build_int_signed_div(si_alen, i64.const_int(2, false), "si_np")
                .map_err(llvm_err)?;
            let si_i = self.builder.build_alloca(i64, "si_i").map_err(llvm_err)?;
            self.builder
                .build_store(si_i, i64.const_int(0, false))
                .map_err(llvm_err)?;
            let si_loop = self.context.append_basic_block(si_fn, "loop");
            let si_body = self.context.append_basic_block(si_fn, "body");
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
                .build_int_compare(IntPredicate::SLT, si_iv, si_npairs, "cond")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(si_cond, si_body, si_done);
            self.builder.position_at_end(si_body);
            let si_kidx = self
                .builder
                .build_int_mul(si_iv, i64.const_int(2, false), "kidx")
                .map_err(llvm_err)?;
            let si_key = self
                .builder
                .build_call(si_get_fn, &[si_a.into(), si_kidx.into()], "key")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic();
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
            let si_skip = self.context.append_basic_block(si_fn, "skip");
            let _ = self
                .builder
                .build_conditional_branch(si_found, si_add, si_skip);
            self.builder.position_at_end(si_add);
            let si_cl2 = self
                .builder
                .build_load(self.list_type, si_ra, "cl2")
                .map_err(llvm_err)?
                .into_struct_value();
            let si_ins = self
                .builder
                .build_call(
                    mi_fn,
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
            let _ = self.builder.build_unconditional_branch(si_skip);
            self.builder.position_at_end(si_skip);
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
                self.list_type.fn_type(&[self.list_type.into(), self.list_type.into()], false),
                None,
            );
            let sd_entry = self.context.append_basic_block(sd_fn, "entry");
            self.builder.position_at_end(sd_entry);
            let sd_a = sd_fn.get_first_param().unwrap().into_struct_value();
            let sd_b = sd_fn.get_nth_param(1).unwrap().into_struct_value();
            let sd_alen = self
                .builder
                .build_extract_value(sd_a, 1, "alen")
                .map_err(llvm_err)?
                .into_int_value();
            let sd_cap4 = self
                .builder
                .build_int_add(sd_alen, i64.const_int(4, false), "cap4")
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
            let sd_get_fn = self.module.get_function("action_list_get").unwrap();
            // Each set entry occupies 2 list elements; iterate num_entries = alen/2
            let sd_npairs = self
                .builder
                .build_int_signed_div(sd_alen, i64.const_int(2, false), "sd_np")
                .map_err(llvm_err)?;
            let sd_i = self.builder.build_alloca(i64, "sd_i").map_err(llvm_err)?;
            self.builder
                .build_store(sd_i, i64.const_int(0, false))
                .map_err(llvm_err)?;
            let sd_loop = self.context.append_basic_block(sd_fn, "loop");
            let sd_body = self.context.append_basic_block(sd_fn, "body");
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
                .build_int_compare(IntPredicate::SLT, sd_iv, sd_npairs, "cond")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(sd_cond, sd_body, sd_done);
            self.builder.position_at_end(sd_body);
            let sd_kidx = self
                .builder
                .build_int_mul(sd_iv, i64.const_int(2, false), "kidx")
                .map_err(llvm_err)?;
            let sd_key = self
                .builder
                .build_call(sd_get_fn, &[sd_a.into(), sd_kidx.into()], "key")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic();
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
            let sd_skip = self.context.append_basic_block(sd_fn, "skip");
            let _ = self
                .builder
                .build_conditional_branch(sd_not_cont, sd_add, sd_skip);
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
            let _ = self.builder.build_unconditional_branch(sd_skip);
            self.builder.position_at_end(sd_skip);
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
            // Sets use map layout: each entry = 4×i64 (key_tag, key_ptr_i64, val_tag, val_ptr_i64).
            // Compare only keys (offsets 0 and 1), skip values (offsets 2 and 3).
            let ss_fn = self.module.add_function(
                "action_set_is_subset",
                self.context
                    .bool_type()
                    .fn_type(&[self.list_type.into(), self.list_type.into()], false),
                None,
            );
            let ss_entry = self.context.append_basic_block(ss_fn, "entry");
            self.builder.position_at_end(ss_entry);
            let a = ss_fn.get_first_param().unwrap().into_struct_value();
            let b = ss_fn.get_nth_param(1).unwrap().into_struct_value();
            let alen = self
                .builder
                .build_extract_value(a, 1, "al")
                .map_err(llvm_err)?
                .into_int_value();
            let blen = self
                .builder
                .build_extract_value(b, 1, "bl")
                .map_err(llvm_err)?
                .into_int_value();
            let two = i64.const_int(2, false);
            let npairs_a = self
                .builder
                .build_int_signed_div(alen, two, "npairs_a")
                .map_err(llvm_err)?;
            let npairs_b = self
                .builder
                .build_int_signed_div(blen, two, "npairs_b")
                .map_err(llvm_err)?;
            let ss_get_fn = self.module.get_function("action_list_get").unwrap();

            // Outer loop counter
            let oi = self.builder.build_alloca(i64, "oi").map_err(llvm_err)?;
            self.builder
                .build_store(oi, i64.const_int(0, false))
                .map_err(llvm_err)?;
            let oloop = self.context.append_basic_block(ss_fn, "oloop");
            let obody = self.context.append_basic_block(ss_fn, "obody");
            let ofound = self.context.append_basic_block(ss_fn, "ofound");
            let oinc = self.context.append_basic_block(ss_fn, "oinc");
            let rtrue = self.context.append_basic_block(ss_fn, "rtrue");
            let rfalse = self.context.append_basic_block(ss_fn, "rfalse");
            let _ = self.builder.build_unconditional_branch(oloop);

            // Outer loop
            self.builder.position_at_end(oloop);
            let oiv = self
                .builder
                .build_load(i64, oi, "oiv")
                .map_err(llvm_err)?
                .into_int_value();
            let ocond = self
                .builder
                .build_int_compare(IntPredicate::SLT, oiv, npairs_a, "ocond")
                .map_err(llvm_err)?;
            let _ = self.builder.build_conditional_branch(ocond, obody, rtrue);

            // Outer body: load A key at index oiv*2 (tree-based map: keys at even indices)
            self.builder.position_at_end(obody);
            let a_kidx = self
                .builder
                .build_int_mul(oiv, i64.const_int(2, false), "a_kidx")
                .map_err(llvm_err)?;
            let a_key = self
                .builder
                .build_call(ss_get_fn, &[a.into(), a_kidx.into()], "a_key")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            let a_tag = self
                .builder
                .build_extract_value(a_key, 0, "a_tag")
                .map_err(llvm_err)?
                .into_int_value();
            let a_ptr = self
                .builder
                .build_extract_value(a_key, 1, "a_ptr")
                .map_err(llvm_err)?
                .into_pointer_value();
            let a_ptr_i64 = self
                .builder
                .build_ptr_to_int(a_ptr, i64, "a_pi")
                .map_err(llvm_err)?;
            let a_is_null = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    a_ptr_i64,
                    i64.const_int(0, false),
                    "a_is_null",
                )
                .map_err(llvm_err)?;

            // Inner loop counter
            let ij = self.builder.build_alloca(i64, "ij").map_err(llvm_err)?;
            self.builder
                .build_store(ij, i64.const_int(0, false))
                .map_err(llvm_err)?;
            let iloop = self.context.append_basic_block(ss_fn, "iloop");
            let ibody = self.context.append_basic_block(ss_fn, "ibody");
            let inext = self.context.append_basic_block(ss_fn, "inext");
            let inotfound = self.context.append_basic_block(ss_fn, "inotfound");
            let _ = self.builder.build_unconditional_branch(iloop);

            // Inner loop
            self.builder.position_at_end(iloop);
            let ijv = self
                .builder
                .build_load(i64, ij, "ijv")
                .map_err(llvm_err)?
                .into_int_value();
            let icond = self
                .builder
                .build_int_compare(IntPredicate::SLT, ijv, npairs_b, "icond")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(icond, ibody, inotfound);

            // Inner body: load B key at index ijv*2, compare with A key
            self.builder.position_at_end(ibody);
            let b_kidx = self
                .builder
                .build_int_mul(ijv, i64.const_int(2, false), "b_kidx")
                .map_err(llvm_err)?;
            let b_key = self
                .builder
                .build_call(ss_get_fn, &[b.into(), b_kidx.into()], "b_key")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            let b_tag = self
                .builder
                .build_extract_value(b_key, 0, "b_tag")
                .map_err(llvm_err)?
                .into_int_value();
            let b_ptr = self
                .builder
                .build_extract_value(b_key, 1, "b_ptr")
                .map_err(llvm_err)?
                .into_pointer_value();
            let b_ptr_i64 = self
                .builder
                .build_ptr_to_int(b_ptr, i64, "b_pi")
                .map_err(llvm_err)?;
            let tag_eq = self
                .builder
                .build_int_compare(IntPredicate::EQ, a_tag, b_tag, "tag_eq")
                .map_err(llvm_err)?;
            let icontent = self.context.append_basic_block(ss_fn, "icontent");
            let _ = self
                .builder
                .build_conditional_branch(tag_eq, icontent, inext);

            // Tags match: check pointer for null vs content
            self.builder.position_at_end(icontent);
            let b_is_null = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    b_ptr_i64,
                    i64.const_int(0, false),
                    "b_is_null",
                )
                .map_err(llvm_err)?;
            let both_null = self
                .builder
                .build_and(a_is_null, b_is_null, "both_null")
                .map_err(llvm_err)?;
            let ifound_bb = self.context.append_basic_block(ss_fn, "ifound_bb");
            let istr_bb = self.context.append_basic_block(ss_fn, "istr_bb");
            let _ = self
                .builder
                .build_conditional_branch(both_null, ifound_bb, istr_bb);
            // Both null: int/None match
            self.builder.position_at_end(ifound_bb);
            let _ = self.builder.build_unconditional_branch(ofound);
            // At least one pointer non-null: both must be non-null for string compare
            self.builder.position_at_end(istr_bb);
            let a_nn = self
                .builder
                .build_not(a_is_null, "a_nn")
                .map_err(llvm_err)?;
            let b_nn = self
                .builder
                .build_not(b_is_null, "b_nn")
                .map_err(llvm_err)?;
            let both_nn = self
                .builder
                .build_and(a_nn, b_nn, "both_nn")
                .map_err(llvm_err)?;
            let istr_eq = self.context.append_basic_block(ss_fn, "istr_eq");
            let _ = self
                .builder
                .build_conditional_branch(both_nn, istr_eq, inext);
            // Build fat structs for string_eq call
            self.builder.position_at_end(istr_eq);
            let a_fat_undef = str_ty.get_undef();
            let a_fat1 = self
                .builder
                .build_insert_value(a_fat_undef, a_tag, 0, "af1")
                .map_err(llvm_err)?;
            let a_ptr_val = self
                .builder
                .build_int_to_ptr(a_ptr_i64, ptr, "a_ptr")
                .map_err(llvm_err)?;
            let a_fat2 = self
                .builder
                .build_insert_value(a_fat1, a_ptr_val, 1, "af2")
                .map_err(llvm_err)?;
            let b_fat_undef = str_ty.get_undef();
            let b_fat1 = self
                .builder
                .build_insert_value(b_fat_undef, b_tag, 0, "bf1")
                .map_err(llvm_err)?;
            let b_ptr_val = self
                .builder
                .build_int_to_ptr(b_ptr_i64, ptr, "b_ptr")
                .map_err(llvm_err)?;
            let b_fat2 = self
                .builder
                .build_insert_value(b_fat1, b_ptr_val, 1, "bf2")
                .map_err(llvm_err)?;
            let sseq_fn = self.module.get_function("action_string_eq").unwrap();
            let sseq = self
                .builder
                .build_call(
                    sseq_fn,
                    &[
                        a_fat2.as_basic_value_enum().into(),
                        b_fat2.as_basic_value_enum().into(),
                    ],
                    "sseq",
                )
                .map_err(llvm_err)?;
            let seq_val = sseq.try_as_basic_value().unwrap_basic().into_int_value();
            let istr_found = self.context.append_basic_block(ss_fn, "istr_found");
            let _ = self
                .builder
                .build_conditional_branch(seq_val, istr_found, inext);
            self.builder.position_at_end(istr_found);
            let _ = self.builder.build_unconditional_branch(ofound);

            // Increment inner loop
            self.builder.position_at_end(inext);
            let nij = self
                .builder
                .build_int_add(ijv, i64.const_int(1, false), "nij")
                .map_err(llvm_err)?;
            self.builder.build_store(ij, nij).map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(iloop);

            // Element NOT found in B
            self.builder.position_at_end(inotfound);
            let _ = self.builder.build_unconditional_branch(rfalse);

            // Element found in B: increment outer loop
            self.builder.position_at_end(ofound);
            let _ = self.builder.build_unconditional_branch(oinc);
            self.builder.position_at_end(oinc);
            let noi = self
                .builder
                .build_int_add(oiv, i64.const_int(1, false), "noi")
                .map_err(llvm_err)?;
            self.builder.build_store(oi, noi).map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(oloop);

            // Results
            self.builder.position_at_end(rfalse);
            let _ = self
                .builder
                .build_return(Some(&self.context.bool_type().const_int(0, false)));
            self.builder.position_at_end(rtrue);
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
