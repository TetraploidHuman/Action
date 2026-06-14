// Submodule: runtime_decl/define_map
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_map(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let _void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let _i32 = self.context.i32_type();
        let _i8 = self.context.i8_type();
        let zero = self.i64_ty().const_int(0, false);
        let _list_create_fn = self.module.get_function("action_list_create").unwrap();
        let _list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        // ---- action_map_create(i64 capacity) -> {ptr, i64, i64} ----
        // Delegates to action_list_create (tree-based storage).
        // Map stores key-value pairs as consecutive fat-struct elements.
        let map_create_fn = self.module.add_function(
            "action_map_create",
            self.list_type.fn_type(&[i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(map_create_fn, "entry");
        self.builder.position_at_end(entry);
        let cap = map_create_fn.get_first_param().unwrap().into_int_value();
        let list_create_fn = self.module.get_function("action_list_create").unwrap();
        let result = self
            .builder
            .build_call(list_create_fn, &[cap.into()], "r")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let _ = self.builder.build_return(Some(&result));

        // ---- action_map_insert / action_map_get / action_map_contains ----
        // Tree-based: map entries stored as consecutive fat-struct elements
        // [key0, val0, key1, val1, ...] in tree leaf/internal nodes.
        // All three delegate to action_list_get / action_list_push.

        let list_get_fn2 = self.module.get_function("action_list_get").unwrap();
        let list_push_fn2 = self.module.get_function("action_list_push").unwrap();
        let map_create_fn2 = self.module.get_function("action_map_create").unwrap();
        let seq_fn_ref = self.module.get_function("action_string_eq").unwrap();
        let sentinel = i64.const_int(i64::MAX as u64, false);

        // ---- action_map_insert({ptr,i64,i64}, {i64,ptr}, {i64,ptr}) -> {ptr,i64,i64} ----
        // Tree-based rebuild: scan old map; rebuild new map via action_list_push.
        // If key exists, its value is updated. If not, key+value are appended.
        let mi_fn = self.module.add_function(
            "action_map_insert",
            self.list_type.fn_type(
                &[self.list_type.into(), str_ty.into(), str_ty.into()],
                false,
            ),
            None,
        );
        let mi_entry = self.context.append_basic_block(mi_fn, "entry");
        let mi_search = self.context.append_basic_block(mi_fn, "search");
        let mi_body = self.context.append_basic_block(mi_fn, "body");
        let mi_ckey = self.context.append_basic_block(mi_fn, "ckey");
        let mi_found = self.context.append_basic_block(mi_fn, "found");
        let mi_nxt = self.context.append_basic_block(mi_fn, "next");
        let mi_rebuild = self.context.append_basic_block(mi_fn, "rebuild");
        let mi_rb_loop = self.context.append_basic_block(mi_fn, "rb_loop");
        let mi_rb_body = self.context.append_basic_block(mi_fn, "rb_body");
        let mi_rb_match = self.context.append_basic_block(mi_fn, "rb_match");
        let mi_rb_copy = self.context.append_basic_block(mi_fn, "rb_copy");
        let mi_rb_nxt = self.context.append_basic_block(mi_fn, "rb_next");
        let mi_rb_done = self.context.append_basic_block(mi_fn, "rb_done");
        let mi_rb_append = self.context.append_basic_block(mi_fn, "rb_append");
        let mi_rb_ret = self.context.append_basic_block(mi_fn, "rb_ret");

        // Entry
        self.builder.position_at_end(mi_entry);
        let mi_map = mi_fn.get_first_param().unwrap().into_struct_value();
        let mi_key = mi_fn.get_nth_param(1).unwrap().into_struct_value();
        let mi_val = mi_fn.get_nth_param(2).unwrap().into_struct_value();
        let mi_len = self
            .builder
            .build_extract_value(mi_map, 1, "l")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(mi_i, zero).map_err(llvm_err)?;
        let mi_match_pos = self
            .builder
            .build_alloca(i64, "match_pos")
            .map_err(llvm_err)?;
        self.builder
            .build_store(mi_match_pos, sentinel)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_search);

        // Search loop: i from 0 to len-1, step 2 (skip values)
        self.builder.position_at_end(mi_search);
        let mi_iv = self
            .builder
            .build_load(i64, mi_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mi_iv, mi_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_cond, mi_body, mi_rebuild);

        self.builder.position_at_end(mi_body);
        let mi_sk_cc = self
            .builder
            .build_call(list_get_fn2, &[mi_map.into(), mi_iv.into()], "gk")
            .map_err(llvm_err)?;
        let mi_sk = mi_sk_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let mi_sk_tag = self
            .builder
            .build_extract_value(mi_sk, 0, "skt")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_ktag = self
            .builder
            .build_extract_value(mi_key, 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_tag_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, mi_sk_tag, mi_ktag, "teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_tag_eq, mi_ckey, mi_nxt);

        self.builder.position_at_end(mi_ckey);
        let mi_kptr = self
            .builder
            .build_extract_value(mi_key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mi_kp_i64 = self
            .builder
            .build_ptr_to_int(mi_kptr, i64, "kp_i64")
            .map_err(llvm_err)?;
        let mi_kpz = self
            .builder
            .build_int_compare(IntPredicate::EQ, mi_kp_i64, zero, "kpz")
            .map_err(llvm_err)?;
        let mi_seq = self
            .builder
            .build_call(seq_fn_ref, &[mi_sk.into(), mi_key.into()], "seq")
            .map_err(llvm_err)?;
        let mi_fe = self
            .builder
            .build_select(
                mi_kpz,
                mi_tag_eq,
                mi_seq.try_as_basic_value().unwrap_basic().into_int_value(),
                "fe",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_fe.into_int_value(), mi_found, mi_nxt);

        self.builder.position_at_end(mi_found);
        self.builder
            .build_store(mi_match_pos, mi_iv)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_rebuild);

        self.builder.position_at_end(mi_nxt);
        let mi_niv = self
            .builder
            .build_int_add(mi_iv, i64.const_int(2, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(mi_i, mi_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_search);

        // Rebuild phase
        self.builder.position_at_end(mi_rebuild);
        let mi_new_cc = self
            .builder
            .build_call(map_create_fn2, &[zero.into()], "new_map")
            .map_err(llvm_err)?;
        let mi_new = mi_new_cc.try_as_basic_value().unwrap_basic();
        let mi_cur = self
            .builder
            .build_alloca(self.list_type, "cur")
            .map_err(llvm_err)?;
        self.builder.build_store(mi_cur, mi_new).map_err(llvm_err)?;
        let mi_j = self.builder.build_alloca(i64, "j").map_err(llvm_err)?;
        self.builder.build_store(mi_j, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_rb_loop);

        self.builder.position_at_end(mi_rb_loop);
        let mi_jv = self
            .builder
            .build_load(i64, mi_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_jc = self
            .builder
            .build_int_compare(IntPredicate::SLT, mi_jv, mi_len, "jc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_jc, mi_rb_body, mi_rb_done);

        self.builder.position_at_end(mi_rb_body);
        let mi_mv = self
            .builder
            .build_load(i64, mi_match_pos, "mv")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_im = self
            .builder
            .build_int_compare(IntPredicate::EQ, mi_jv, mi_mv, "im")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_im, mi_rb_match, mi_rb_copy);

        // Push new key+value for matched entry
        self.builder.position_at_end(mi_rb_match);
        let mi_s1 = self
            .builder
            .build_load(self.list_type, mi_cur, "s1")
            .map_err(llvm_err)?
            .into_struct_value();
        let mi_pk = self
            .builder
            .build_call(list_push_fn2, &[mi_s1.into(), mi_key.into()], "pk")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(mi_cur, mi_pk).map_err(llvm_err)?;
        let mi_s2 = self
            .builder
            .build_load(self.list_type, mi_cur, "s2")
            .map_err(llvm_err)?
            .into_struct_value();
        let mi_pv = self
            .builder
            .build_call(list_push_fn2, &[mi_s2.into(), mi_val.into()], "pv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(mi_cur, mi_pv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_rb_nxt);

        // Copy stored key+value
        self.builder.position_at_end(mi_rb_copy);
        let mi_s3 = self
            .builder
            .build_load(self.list_type, mi_cur, "s3")
            .map_err(llvm_err)?
            .into_struct_value();
        let mi_gk = self
            .builder
            .build_call(list_get_fn2, &[mi_map.into(), mi_jv.into()], "gk2")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let mi_p1 = self
            .builder
            .build_call(list_push_fn2, &[mi_s3.into(), mi_gk.into()], "p1")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(mi_cur, mi_p1).map_err(llvm_err)?;
        let mi_j1 = self
            .builder
            .build_int_add(mi_jv, i64.const_int(1, false), "j1")
            .map_err(llvm_err)?;
        let mi_s4 = self
            .builder
            .build_load(self.list_type, mi_cur, "s4")
            .map_err(llvm_err)?
            .into_struct_value();
        let mi_gv = self
            .builder
            .build_call(list_get_fn2, &[mi_map.into(), mi_j1.into()], "gv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let mi_p2 = self
            .builder
            .build_call(list_push_fn2, &[mi_s4.into(), mi_gv.into()], "p2")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(mi_cur, mi_p2).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_rb_nxt);

        self.builder.position_at_end(mi_rb_nxt);
        let mi_nj = self
            .builder
            .build_int_add(mi_jv, i64.const_int(2, false), "nj")
            .map_err(llvm_err)?;
        self.builder.build_store(mi_j, mi_nj).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_rb_loop);

        // Done: append if not found
        self.builder.position_at_end(mi_rb_done);
        let mi_fm = self
            .builder
            .build_load(i64, mi_match_pos, "fm")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_nf = self
            .builder
            .build_int_compare(IntPredicate::EQ, mi_fm, sentinel, "nf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_nf, mi_rb_append, mi_rb_ret);

        self.builder.position_at_end(mi_rb_append);
        let mi_s5 = self
            .builder
            .build_load(self.list_type, mi_cur, "s5")
            .map_err(llvm_err)?
            .into_struct_value();
        let mi_ak = self
            .builder
            .build_call(list_push_fn2, &[mi_s5.into(), mi_key.into()], "ak")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(mi_cur, mi_ak).map_err(llvm_err)?;
        let mi_s6 = self
            .builder
            .build_load(self.list_type, mi_cur, "s6")
            .map_err(llvm_err)?
            .into_struct_value();
        let mi_av = self
            .builder
            .build_call(list_push_fn2, &[mi_s6.into(), mi_val.into()], "av")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(mi_cur, mi_av).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_rb_ret);

        self.builder.position_at_end(mi_rb_ret);
        let mi_result = self
            .builder
            .build_load(self.list_type, mi_cur, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mi_result));

        // ---- action_map_get({ptr,i64,i64}, {i64,ptr}) -> {i64,ptr} ----
        let mg_fn = self.module.add_function(
            "action_map_get",
            str_ty.fn_type(&[self.list_type.into(), str_ty.into()], false),
            None,
        );
        let mg_blocks: Vec<_> = (0..7)
            .map(|i| self.context.append_basic_block(mg_fn, &format!("b{}", i)))
            .collect();
        self.builder.position_at_end(mg_blocks[0]); // entry
        let mg_map = mg_fn.get_first_param().unwrap().into_struct_value();
        let mg_key = mg_fn.get_nth_param(1).unwrap().into_struct_value();
        let mg_len = self
            .builder
            .build_extract_value(mg_map, 1, "l")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_ktag = self
            .builder
            .build_extract_value(mg_key, 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_kptr = self
            .builder
            .build_extract_value(mg_key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mg_kp_i64 = self
            .builder
            .build_ptr_to_int(mg_kptr, i64, "kp_i64")
            .map_err(llvm_err)?;
        let mg_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(mg_i, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mg_blocks[1]); // search

        self.builder.position_at_end(mg_blocks[1]); // search
        let mg_iv = self
            .builder
            .build_load(i64, mg_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mg_iv, mg_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mg_cond, mg_blocks[2], mg_blocks[6]);

        self.builder.position_at_end(mg_blocks[2]); // body
        let mg_sk_cc = self
            .builder
            .build_call(list_get_fn2, &[mg_map.into(), mg_iv.into()], "gk")
            .map_err(llvm_err)?;
        let mg_sk = mg_sk_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let mg_sk_tag = self
            .builder
            .build_extract_value(mg_sk, 0, "skt")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, mg_sk_tag, mg_ktag, "teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mg_teq, mg_blocks[3], mg_blocks[5]);

        self.builder.position_at_end(mg_blocks[3]); // ckey
        let mg_kpz = self
            .builder
            .build_int_compare(IntPredicate::EQ, mg_kp_i64, zero, "kpz")
            .map_err(llvm_err)?;
        let mg_seq = self
            .builder
            .build_call(seq_fn_ref, &[mg_sk.into(), mg_key.into()], "seq")
            .map_err(llvm_err)?;
        let mg_fe = self
            .builder
            .build_select(
                mg_kpz,
                mg_teq,
                mg_seq.try_as_basic_value().unwrap_basic().into_int_value(),
                "fe",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            mg_fe.into_int_value(),
            mg_blocks[4],
            mg_blocks[5],
        );

        self.builder.position_at_end(mg_blocks[4]); // found
        let mg_j = self
            .builder
            .build_int_add(mg_iv, i64.const_int(1, false), "j")
            .map_err(llvm_err)?;
        let mg_val_cc = self
            .builder
            .build_call(list_get_fn2, &[mg_map.into(), mg_j.into()], "gv")
            .map_err(llvm_err)?;
        let mg_val = mg_val_cc.try_as_basic_value().unwrap_basic();
        let _ = self.builder.build_return(Some(&mg_val));

        self.builder.position_at_end(mg_blocks[5]); // next
        let mg_niv = self
            .builder
            .build_int_add(mg_iv, i64.const_int(2, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(mg_i, mg_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mg_blocks[1]);

        self.builder.position_at_end(mg_blocks[6]); // not_found
        let mg_ur = str_ty.get_undef();
        let mg_nf1 = self
            .builder
            .build_insert_value(mg_ur, zero, 0, "nf1")
            .map_err(llvm_err)?;
        let mg_nf2 = self
            .builder
            .build_insert_value(mg_nf1, ptr.const_zero(), 1, "nf2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mg_nf2));

        // ---- action_map_contains({ptr,i64,i64}, {i64,ptr}) -> i1 ----
        let mc_fn = self.module.add_function(
            "action_map_contains",
            b1.fn_type(&[self.list_type.into(), str_ty.into()], false),
            None,
        );
        let mc_blocks: Vec<_> = (0..7)
            .map(|i| self.context.append_basic_block(mc_fn, &format!("b{}", i)))
            .collect();
        self.builder.position_at_end(mc_blocks[0]); // entry
        let mc_map = mc_fn.get_first_param().unwrap().into_struct_value();
        let mc_key = mc_fn.get_nth_param(1).unwrap().into_struct_value();
        let mc_len = self
            .builder
            .build_extract_value(mc_map, 1, "l")
            .map_err(llvm_err)?
            .into_int_value();
        let mc_ktag = self
            .builder
            .build_extract_value(mc_key, 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let mc_kptr = self
            .builder
            .build_extract_value(mc_key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mc_kp_i64 = self
            .builder
            .build_ptr_to_int(mc_kptr, i64, "kp_i64")
            .map_err(llvm_err)?;
        let mc_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(mc_i, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mc_blocks[1]); // search

        self.builder.position_at_end(mc_blocks[1]); // search
        let mc_iv = self
            .builder
            .build_load(i64, mc_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mc_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mc_iv, mc_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mc_cond, mc_blocks[2], mc_blocks[6]);

        self.builder.position_at_end(mc_blocks[2]); // body
        let mc_sk_cc = self
            .builder
            .build_call(list_get_fn2, &[mc_map.into(), mc_iv.into()], "gk")
            .map_err(llvm_err)?;
        let mc_sk = mc_sk_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let mc_sk_tag = self
            .builder
            .build_extract_value(mc_sk, 0, "skt")
            .map_err(llvm_err)?
            .into_int_value();
        let mc_teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, mc_sk_tag, mc_ktag, "teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mc_teq, mc_blocks[3], mc_blocks[5]);

        self.builder.position_at_end(mc_blocks[3]); // ckey
        let mc_kpz = self
            .builder
            .build_int_compare(IntPredicate::EQ, mc_kp_i64, zero, "kpz")
            .map_err(llvm_err)?;
        let mc_seq = self
            .builder
            .build_call(seq_fn_ref, &[mc_sk.into(), mc_key.into()], "seq")
            .map_err(llvm_err)?;
        let mc_fe = self
            .builder
            .build_select(
                mc_kpz,
                mc_teq,
                mc_seq.try_as_basic_value().unwrap_basic().into_int_value(),
                "fe",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            mc_fe.into_int_value(),
            mc_blocks[4],
            mc_blocks[5],
        );

        self.builder.position_at_end(mc_blocks[4]); // found
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));

        self.builder.position_at_end(mc_blocks[5]); // next
        let mc_niv = self
            .builder
            .build_int_add(mc_iv, i64.const_int(2, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(mc_i, mc_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mc_blocks[1]);

        self.builder.position_at_end(mc_blocks[6]); // not_found
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        // ---- action_map_remove({ptr,i64,i64}, {i64,ptr}) -> {ptr,i64,i64} ----
        // Rebuild approach: scan source, skip matched entry, copy rest.
        let mr_fn = self.module.add_function(
            "action_map_remove",
            self.list_type
                .fn_type(&[self.list_type.into(), str_ty.into()], false),
            None,
        );
        let mr_entry = self.context.append_basic_block(mr_fn, "entry");
        let mr_search = self.context.append_basic_block(mr_fn, "search");
        let mr_body = self.context.append_basic_block(mr_fn, "body");
        let mr_ckey = self.context.append_basic_block(mr_fn, "ckey");
        let mr_found_bb = self.context.append_basic_block(mr_fn, "found");
        let mr_nxt = self.context.append_basic_block(mr_fn, "next");
        let mr_rebuild = self.context.append_basic_block(mr_fn, "rebuild");
        let mr_rb_loop = self.context.append_basic_block(mr_fn, "rb_loop");
        let mr_rb_body = self.context.append_basic_block(mr_fn, "rb_body");
        let mr_rb_skip = self.context.append_basic_block(mr_fn, "rb_skip");
        let mr_rb_copy = self.context.append_basic_block(mr_fn, "rb_copy");
        let mr_rb_nxt = self.context.append_basic_block(mr_fn, "rb_next");
        let mr_rb_done = self.context.append_basic_block(mr_fn, "rb_done");

        self.builder.position_at_end(mr_entry);
        let mr_map = mr_fn.get_first_param().unwrap().into_struct_value();
        let mr_key = mr_fn.get_nth_param(1).unwrap().into_struct_value();
        let mr_len = self
            .builder
            .build_extract_value(mr_map, 1, "l")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_ktag = self
            .builder
            .build_extract_value(mr_key, 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_kptr = self
            .builder
            .build_extract_value(mr_key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mr_kp_i64 = self
            .builder
            .build_ptr_to_int(mr_kptr, i64, "kp_i64")
            .map_err(llvm_err)?;
        let mr_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(mr_i, zero).map_err(llvm_err)?;
        let mr_match_pos = self.builder.build_alloca(i64, "mp").map_err(llvm_err)?;
        self.builder
            .build_store(mr_match_pos, sentinel)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_search);

        // Search: find key position
        self.builder.position_at_end(mr_search);
        let mr_iv = self
            .builder
            .build_load(i64, mr_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mr_iv, mr_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mr_cond, mr_body, mr_rebuild);

        self.builder.position_at_end(mr_body);
        let mr_gk_cc = self
            .builder
            .build_call(list_get_fn2, &[mr_map.into(), mr_iv.into()], "gk")
            .map_err(llvm_err)?;
        let mr_gk = mr_gk_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let mr_gk_tag = self
            .builder
            .build_extract_value(mr_gk, 0, "gkt")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, mr_gk_tag, mr_ktag, "teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mr_teq, mr_ckey, mr_nxt);

        self.builder.position_at_end(mr_ckey);
        let mr_kpz = self
            .builder
            .build_int_compare(IntPredicate::EQ, mr_kp_i64, zero, "kpz")
            .map_err(llvm_err)?;
        let mr_seq = self
            .builder
            .build_call(seq_fn_ref, &[mr_gk.into(), mr_key.into()], "seq")
            .map_err(llvm_err)?;
        let mr_fe = self
            .builder
            .build_select(
                mr_kpz,
                mr_teq,
                mr_seq.try_as_basic_value().unwrap_basic().into_int_value(),
                "fe",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mr_fe.into_int_value(), mr_found_bb, mr_nxt);

        self.builder.position_at_end(mr_found_bb);
        self.builder
            .build_store(mr_match_pos, mr_iv)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_rebuild);

        self.builder.position_at_end(mr_nxt);
        let mr_niv = self
            .builder
            .build_int_add(mr_iv, i64.const_int(2, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(mr_i, mr_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_search);

        // Rebuild: copy all entries except matched key+value pair
        self.builder.position_at_end(mr_rebuild);
        let mr_new_cc = self
            .builder
            .build_call(map_create_fn2, &[zero.into()], "new_map")
            .map_err(llvm_err)?;
        let mr_new = mr_new_cc.try_as_basic_value().unwrap_basic();
        let mr_cur = self
            .builder
            .build_alloca(self.list_type, "cur")
            .map_err(llvm_err)?;
        self.builder.build_store(mr_cur, mr_new).map_err(llvm_err)?;
        let mr_j = self.builder.build_alloca(i64, "j").map_err(llvm_err)?;
        self.builder.build_store(mr_j, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_rb_loop);

        self.builder.position_at_end(mr_rb_loop);
        let mr_jv = self
            .builder
            .build_load(i64, mr_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_jc = self
            .builder
            .build_int_compare(IntPredicate::SLT, mr_jv, mr_len, "jc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mr_jc, mr_rb_body, mr_rb_done);

        self.builder.position_at_end(mr_rb_body);
        let mr_mv = self
            .builder
            .build_load(i64, mr_match_pos, "mv")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_im = self
            .builder
            .build_int_compare(IntPredicate::EQ, mr_jv, mr_mv, "im")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mr_im, mr_rb_skip, mr_rb_copy);

        self.builder.position_at_end(mr_rb_skip);
        let _ = self.builder.build_unconditional_branch(mr_rb_nxt);

        self.builder.position_at_end(mr_rb_copy);
        let mr_s = self
            .builder
            .build_load(self.list_type, mr_cur, "s")
            .map_err(llvm_err)?
            .into_struct_value();
        // Push key at j
        let mr_g = self
            .builder
            .build_call(list_get_fn2, &[mr_map.into(), mr_jv.into()], "g")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let mr_p = self
            .builder
            .build_call(list_push_fn2, &[mr_s.into(), mr_g.into()], "p")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(mr_cur, mr_p).map_err(llvm_err)?;
        // Push value at j+1
        let mr_j1 = self
            .builder
            .build_int_add(mr_jv, i64.const_int(1, false), "j1")
            .map_err(llvm_err)?;
        let mr_s2 = self
            .builder
            .build_load(self.list_type, mr_cur, "s2")
            .map_err(llvm_err)?
            .into_struct_value();
        let mr_gv = self
            .builder
            .build_call(list_get_fn2, &[mr_map.into(), mr_j1.into()], "gv")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let mr_p2 = self
            .builder
            .build_call(list_push_fn2, &[mr_s2.into(), mr_gv.into()], "p2")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(mr_cur, mr_p2).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_rb_nxt);

        self.builder.position_at_end(mr_rb_nxt);
        let mr_nj = self
            .builder
            .build_int_add(mr_jv, i64.const_int(2, false), "nj")
            .map_err(llvm_err)?;
        self.builder.build_store(mr_j, mr_nj).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_rb_loop);

        self.builder.position_at_end(mr_rb_done);
        let mr_result = self
            .builder
            .build_load(self.list_type, mr_cur, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mr_result));

        Ok(())
    }
}
