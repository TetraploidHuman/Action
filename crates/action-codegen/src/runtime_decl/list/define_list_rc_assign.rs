// List assign RC: release old tree nodes not reachable from live scope bindings.

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(in crate::runtime_decl) fn define_list_rc_assign(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let i1 = self.context.bool_type();

        let rc_dec_fn = self.module.get_function("action_rc_dec").unwrap();

        // ---- action_list_tree_contains_node(root, height, target) -> i1 ----
        let cnt_fn = self.module.add_function(
            "action_list_tree_contains_node",
            i1.fn_type(&[ptr.into(), i64.into(), ptr.into()], false),
            None,
        );
        let cnt_entry = self.context.append_basic_block(cnt_fn, "entry");
        let cnt_null = self.context.append_basic_block(cnt_fn, "null");
        let cnt_hit = self.context.append_basic_block(cnt_fn, "hit");
        let cnt_concat = self.context.append_basic_block(cnt_fn, "concat");
        let cnt_internal = self.context.append_basic_block(cnt_fn, "internal");
        let cnt_leaf = self.context.append_basic_block(cnt_fn, "leaf");
        let cnt_done = self.context.append_basic_block(cnt_fn, "done");

        self.builder.position_at_end(cnt_entry);
        let cnt_root = cnt_fn.get_first_param().unwrap().into_pointer_value();
        let cnt_h = cnt_fn.get_nth_param(1).unwrap().into_int_value();
        let cnt_target = cnt_fn.get_nth_param(2).unwrap().into_pointer_value();
        let cnt_is_null = self
            .builder
            .build_is_null(cnt_root, "n")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cnt_is_null, cnt_null, cnt_hit);
        self.builder.position_at_end(cnt_null);
        let _ = self.builder.build_return(Some(&i1.const_int(0, false)));
        self.builder.position_at_end(cnt_hit);
        let cnt_eq = self
            .builder
            .build_ptr_to_int(cnt_root, i64, "r")
            .map_err(llvm_err)?;
        let cnt_teq = self
            .builder
            .build_ptr_to_int(cnt_target, i64, "t")
            .map_err(llvm_err)?;
        let cnt_same = self
            .builder
            .build_int_compare(IntPredicate::EQ, cnt_eq, cnt_teq, "same")
            .map_err(llvm_err)?;
        let cnt_not_same = self.context.append_basic_block(cnt_fn, "not_same");
        let _ = self
            .builder
            .build_conditional_branch(cnt_same, cnt_done, cnt_not_same);
        self.builder.position_at_end(cnt_done);
        let _ = self.builder.build_return(Some(&i1.const_int(1, false)));
        self.builder.position_at_end(cnt_not_same);
        let cnt_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                cnt_h,
                i64.const_int(-1i64 as u64, true),
                "ic",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cnt_is_concat, cnt_concat, cnt_internal);

        // concat: check left/right subtree roots
        self.builder.position_at_end(cnt_concat);
        let cnt_lnp = unsafe {
            self.builder
                .build_gep(i8, cnt_root, &[i64.const_int(16, false)], "lnp")
                .map_err(llvm_err)
        }?;
        let cnt_lnode = self
            .builder
            .build_load(ptr, cnt_lnp, "ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cnt_lhp = unsafe {
            self.builder
                .build_gep(i8, cnt_root, &[i64.const_int(32, false)], "lhp")
                .map_err(llvm_err)
        }?;
        let cnt_lh = self
            .builder
            .build_load(i64, cnt_lhp, "lh")
            .map_err(llvm_err)?
            .into_int_value();
        let cnt_lhit = self
            .builder
            .build_call(
                cnt_fn,
                &[cnt_lnode.into(), cnt_lh.into(), cnt_target.into()],
                "lh",
            )
            .map_err(llvm_err)?;
        let cnt_lhit_b = cnt_lhit
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let cnt_after_l = self.context.append_basic_block(cnt_fn, "after_l");
        let _ = self
            .builder
            .build_conditional_branch(cnt_lhit_b, cnt_done, cnt_after_l);
        self.builder.position_at_end(cnt_after_l);
        let cnt_rnp = unsafe {
            self.builder
                .build_gep(i8, cnt_root, &[i64.const_int(40, false)], "rnp")
                .map_err(llvm_err)
        }?;
        let cnt_rnode = self
            .builder
            .build_load(ptr, cnt_rnp, "rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cnt_rhp = unsafe {
            self.builder
                .build_gep(i8, cnt_root, &[i64.const_int(56, false)], "rhp")
                .map_err(llvm_err)
        }?;
        let cnt_rh = self
            .builder
            .build_load(i64, cnt_rhp, "rh")
            .map_err(llvm_err)?
            .into_int_value();
        let cnt_rhit = self
            .builder
            .build_call(
                cnt_fn,
                &[cnt_rnode.into(), cnt_rh.into(), cnt_target.into()],
                "rh",
            )
            .map_err(llvm_err)?;
        let cnt_rhit_b = cnt_rhit
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let cnt_false = self.context.append_basic_block(cnt_fn, "false");
        let _ = self
            .builder
            .build_conditional_branch(cnt_rhit_b, cnt_done, cnt_false);
        self.builder.position_at_end(cnt_false);
        let _ = self.builder.build_return(Some(&i1.const_int(0, false)));

        // internal h>0: iterate child slots
        let cnt_int_loop = self.context.append_basic_block(cnt_fn, "int_loop");
        let cnt_int_hdr = self.context.append_basic_block(cnt_fn, "int_hdr");
        let cnt_int_body = self.context.append_basic_block(cnt_fn, "int_body");
        let cnt_int_next = self.context.append_basic_block(cnt_fn, "int_next");
        self.builder.position_at_end(cnt_internal);
        let cnt_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, cnt_h, i64.const_int(0, false), "il")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cnt_is_leaf, cnt_leaf, cnt_int_loop);

        self.builder.position_at_end(cnt_leaf);
        let _ = self.builder.build_return(Some(&i1.const_int(0, false)));

        self.builder.position_at_end(cnt_int_loop);
        let cnt_count_raw = self
            .builder
            .build_load(i32, cnt_root, "cr")
            .map_err(llvm_err)?
            .into_int_value();
        let cnt_count = self
            .builder
            .build_int_z_extend(cnt_count_raw, i64, "c")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(cnt_int_hdr);
        self.builder.position_at_end(cnt_int_hdr);
        let cnt_phi = self.builder.build_phi(i64, "i").map_err(llvm_err)?;
        cnt_phi.add_incoming(&[(&i64.const_int(0, false), cnt_int_loop)]);
        let cnt_i = cnt_phi.as_basic_value().into_int_value();
        let cnt_done_i = self
            .builder
            .build_int_compare(IntPredicate::SGE, cnt_i, cnt_count, "di")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cnt_done_i, cnt_false, cnt_int_body);
        self.builder.position_at_end(cnt_int_body);
        let cnt_off = self
            .builder
            .build_int_add(
                i64.const_int(16, false),
                self.builder
                    .build_int_mul(cnt_i, i64.const_int(16, false), "m")
                    .map_err(llvm_err)?,
                "off",
            )
            .map_err(llvm_err)?;
        let cnt_ep = unsafe {
            self.builder
                .build_gep(i8, cnt_root, &[cnt_off], "ep")
                .map_err(llvm_err)
        }?;
        let cnt_child = self
            .builder
            .build_load(ptr, cnt_ep, "ch")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cnt_ch_hit = self
            .builder
            .build_call(
                cnt_fn,
                &[
                    cnt_child.into(),
                    self.builder
                        .build_int_sub(cnt_h, i64.const_int(1, false), "nh")
                        .map_err(llvm_err)?
                        .into(),
                    cnt_target.into(),
                ],
                "chh",
            )
            .map_err(llvm_err)?;
        let cnt_ch_hit_b = cnt_ch_hit
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(cnt_ch_hit_b, cnt_done, cnt_int_next);
        self.builder.position_at_end(cnt_int_next);
        let cnt_ni = self
            .builder
            .build_int_add(cnt_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        cnt_phi.add_incoming(&[(&cnt_ni, cnt_int_next)]);
        let _ = self.builder.build_unconditional_branch(cnt_int_hdr);

        // ---- action_rc_release_list_on_assign(old, h, live_nodes*, live_hs*, n) ----
        let rla_fn = self.module.add_function(
            "action_rc_release_list_on_assign",
            self.context.void_type().fn_type(
                &[ptr.into(), i64.into(), ptr.into(), ptr.into(), i64.into()],
                false,
            ),
            None,
        );
        let rla_entry = self.context.append_basic_block(rla_fn, "entry");
        let rla_body = self.context.append_basic_block(rla_fn, "body");
        let rla_ret = self.context.append_basic_block(rla_fn, "ret");
        let rla_rec_fn = self.module.add_function(
            "action_rc_release_list_on_assign_rec",
            self.context.void_type().fn_type(
                &[ptr.into(), i64.into(), ptr.into(), ptr.into(), i64.into()],
                false,
            ),
            None,
        );

        self.builder.position_at_end(rla_entry);
        let rla_old = rla_fn.get_first_param().unwrap().into_pointer_value();
        let rla_h = rla_fn.get_nth_param(1).unwrap().into_int_value();
        let rla_lives = rla_fn.get_nth_param(2).unwrap().into_pointer_value();
        let rla_lhs = rla_fn.get_nth_param(3).unwrap().into_pointer_value();
        let rla_n = rla_fn.get_nth_param(4).unwrap().into_int_value();
        let rla_null = self
            .builder
            .build_is_null(rla_old, "on")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rla_null, rla_ret, rla_body);
        self.builder.position_at_end(rla_body);
        let _ = self.builder.build_call(
            rla_rec_fn,
            &[
                rla_old.into(),
                rla_h.into(),
                rla_lives.into(),
                rla_lhs.into(),
                rla_n.into(),
            ],
            "",
        );
        let _ = self.builder.build_unconditional_branch(rla_ret);
        self.builder.position_at_end(rla_ret);
        let _ = self.builder.build_return(None);

        // recursive release: skip subtrees reachable from live bindings
        let rec_entry = self.context.append_basic_block(rla_rec_fn, "entry");
        let rec_null = self.context.append_basic_block(rla_rec_fn, "null");
        let rec_skip = self.context.append_basic_block(rla_rec_fn, "skip");
        let rec_concat = self.context.append_basic_block(rla_rec_fn, "concat");
        let rec_leaf = self.context.append_basic_block(rla_rec_fn, "leaf");
        let rec_internal = self.context.append_basic_block(rla_rec_fn, "internal");
        let rec_dec = self.context.append_basic_block(rla_rec_fn, "dec");

        self.builder.position_at_end(rec_entry);
        let rec_node = rla_rec_fn.get_first_param().unwrap().into_pointer_value();
        let rec_h = rla_rec_fn.get_nth_param(1).unwrap().into_int_value();
        let rec_lives = rla_rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let rec_lhs = rla_rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let rec_n = rla_rec_fn.get_nth_param(4).unwrap().into_int_value();

        let rec_is_null = self
            .builder
            .build_is_null(rec_node, "nn")
            .map_err(llvm_err)?;
        let rec_after_null = self.context.append_basic_block(rla_rec_fn, "after_null");
        let _ = self
            .builder
            .build_conditional_branch(rec_is_null, rec_null, rec_after_null);

        self.builder.position_at_end(rec_null);
        let _ = self.builder.build_return(None);

        // live scan loop
        let rec_live_hdr = self.context.append_basic_block(rla_rec_fn, "live_hdr");
        let rec_live_bdy = self.context.append_basic_block(rla_rec_fn, "live_bdy");
        let rec_live_done = self.context.append_basic_block(rla_rec_fn, "live_done");
        self.builder.position_at_end(rec_after_null);
        let _ = self.builder.build_unconditional_branch(rec_live_hdr);
        self.builder.position_at_end(rec_live_hdr);
        let rec_li_phi = self.builder.build_phi(i64, "li").map_err(llvm_err)?;
        rec_li_phi.add_incoming(&[(&i64.const_int(0, false), rec_after_null)]);
        let rec_li = rec_li_phi.as_basic_value().into_int_value();
        let rec_li_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, rec_li, rec_n, "ld")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rec_li_done, rec_live_done, rec_live_bdy);
        self.builder.position_at_end(rec_live_bdy);
        let rec_lp = unsafe {
            self.builder
                .build_gep(ptr, rec_lives, &[rec_li], "lp")
                .map_err(llvm_err)
        }?;
        let rec_live_root = self
            .builder
            .build_load(ptr, rec_lp, "lr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rec_lhp = unsafe {
            self.builder
                .build_gep(i64, rec_lhs, &[rec_li], "lhp")
                .map_err(llvm_err)
        }?;
        let rec_live_h = self
            .builder
            .build_load(i64, rec_lhp, "lh")
            .map_err(llvm_err)?
            .into_int_value();
        let rec_lnull = self
            .builder
            .build_is_null(rec_live_root, "ln")
            .map_err(llvm_err)?;
        let rec_lnext = self.context.append_basic_block(rla_rec_fn, "lnext");
        let rec_lcheck = self.context.append_basic_block(rla_rec_fn, "lcheck");
        let _ = self
            .builder
            .build_conditional_branch(rec_lnull, rec_lnext, rec_lcheck);
        self.builder.position_at_end(rec_lcheck);
        let rec_hit = self
            .builder
            .build_call(
                cnt_fn,
                &[rec_live_root.into(), rec_live_h.into(), rec_node.into()],
                "hit",
            )
            .map_err(llvm_err)?;
        let rec_hit_b = rec_hit
            .try_as_basic_value()
            .basic()
            .unwrap()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(rec_hit_b, rec_skip, rec_lnext);
        self.builder.position_at_end(rec_lnext);
        let rec_nli = self
            .builder
            .build_int_add(rec_li, i64.const_int(1, false), "nli")
            .map_err(llvm_err)?;
        rec_li_phi.add_incoming(&[(&rec_nli, rec_lnext)]);
        let _ = self.builder.build_unconditional_branch(rec_live_hdr);
        self.builder.position_at_end(rec_skip);
        let _ = self.builder.build_return(None);

        self.builder.position_at_end(rec_live_done);
        let rec_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                rec_h,
                i64.const_int(-1i64 as u64, true),
                "ic",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rec_is_concat, rec_concat, rec_internal);

        // concat: release children then dec concat node
        self.builder.position_at_end(rec_concat);
        let rec_lnp = unsafe {
            self.builder
                .build_gep(i8, rec_node, &[i64.const_int(16, false)], "lnp")
                .map_err(llvm_err)
        }?;
        let rec_lnode = self
            .builder
            .build_load(ptr, rec_lnp, "ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rec_lhp2 = unsafe {
            self.builder
                .build_gep(i8, rec_node, &[i64.const_int(32, false)], "lhp")
                .map_err(llvm_err)
        }?;
        let rec_lh = self
            .builder
            .build_load(i64, rec_lhp2, "lh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_call(
            rla_rec_fn,
            &[
                rec_lnode.into(),
                rec_lh.into(),
                rec_lives.into(),
                rec_lhs.into(),
                rec_n.into(),
            ],
            "",
        );
        let rec_rnp = unsafe {
            self.builder
                .build_gep(i8, rec_node, &[i64.const_int(40, false)], "rnp")
                .map_err(llvm_err)
        }?;
        let rec_rnode = self
            .builder
            .build_load(ptr, rec_rnp, "rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rec_rhp2 = unsafe {
            self.builder
                .build_gep(i8, rec_node, &[i64.const_int(56, false)], "rhp")
                .map_err(llvm_err)
        }?;
        let rec_rh = self
            .builder
            .build_load(i64, rec_rhp2, "rh")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_call(
            rla_rec_fn,
            &[
                rec_rnode.into(),
                rec_rh.into(),
                rec_lives.into(),
                rec_lhs.into(),
                rec_n.into(),
            ],
            "",
        );
        let _ = self.builder.build_unconditional_branch(rec_dec);

        // leaf vs internal
        self.builder.position_at_end(rec_internal);
        let rec_count_raw = self
            .builder
            .build_load(i32, rec_node, "cr")
            .map_err(llvm_err)?
            .into_int_value();
        let rec_count = self
            .builder
            .build_int_z_extend(rec_count_raw, i64, "c")
            .map_err(llvm_err)?;
        let rec_is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, rec_h, i64.const_int(0, false), "il")
            .map_err(llvm_err)?;
        let rec_int_children = self.context.append_basic_block(rla_rec_fn, "int_ch");
        let _ = self
            .builder
            .build_conditional_branch(rec_is_leaf, rec_leaf, rec_int_children);

        // h>0: child loop
        let rec_ch_loop = self.context.append_basic_block(rla_rec_fn, "ch_loop");
        let rec_ch_body = self.context.append_basic_block(rla_rec_fn, "ch_body");
        let rec_ch_next = self.context.append_basic_block(rla_rec_fn, "ch_next");
        self.builder.position_at_end(rec_int_children);
        let _ = self.builder.build_unconditional_branch(rec_ch_loop);
        self.builder.position_at_end(rec_ch_loop);
        let rec_ci_phi = self.builder.build_phi(i64, "ci").map_err(llvm_err)?;
        rec_ci_phi.add_incoming(&[(&i64.const_int(0, false), rec_int_children)]);
        let rec_ci = rec_ci_phi.as_basic_value().into_int_value();
        let rec_ci_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, rec_ci, rec_count, "cd")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rec_ci_done, rec_dec, rec_ch_body);
        self.builder.position_at_end(rec_ch_body);
        let rec_coff = self
            .builder
            .build_int_add(
                i64.const_int(16, false),
                self.builder
                    .build_int_mul(rec_ci, i64.const_int(16, false), "cm")
                    .map_err(llvm_err)?,
                "coff",
            )
            .map_err(llvm_err)?;
        let rec_ce = unsafe {
            self.builder
                .build_gep(i8, rec_node, &[rec_coff], "cep")
                .map_err(llvm_err)
        }?;
        let rec_child = self
            .builder
            .build_load(ptr, rec_ce, "ch")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rec_ch_null = self
            .builder
            .build_is_null(rec_child, "cn")
            .map_err(llvm_err)?;
        let rec_ch_do = self.context.append_basic_block(rla_rec_fn, "ch_do");
        let _ = self
            .builder
            .build_conditional_branch(rec_ch_null, rec_ch_next, rec_ch_do);
        self.builder.position_at_end(rec_ch_do);
        let rec_nh = self
            .builder
            .build_int_sub(rec_h, i64.const_int(1, false), "nh")
            .map_err(llvm_err)?;
        let _ = self.builder.build_call(
            rla_rec_fn,
            &[
                rec_child.into(),
                rec_nh.into(),
                rec_lives.into(),
                rec_lhs.into(),
                rec_n.into(),
            ],
            "",
        );
        let _ = self.builder.build_unconditional_branch(rec_ch_next);
        self.builder.position_at_end(rec_ch_next);
        let rec_nci = self
            .builder
            .build_int_add(rec_ci, i64.const_int(1, false), "nci")
            .map_err(llvm_err)?;
        rec_ci_phi.add_incoming(&[(&rec_nci, rec_ch_next)]);
        let _ = self.builder.build_unconditional_branch(rec_ch_loop);

        // h==0 isolated leaf: drop this node's ref (int/by-value elems need no per-element dec)
        self.builder.position_at_end(rec_leaf);
        let _ = self
            .builder
            .build_call(rc_dec_fn, &[rec_node.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

        // h>0 internal / concat: node shell only (children already visited)
        self.builder.position_at_end(rec_dec);
        let _ = self
            .builder
            .build_call(rc_dec_fn, &[rec_node.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

        Ok(())
    }
}
