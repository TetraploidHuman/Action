// Sequential list access cache for for-loop iteration (O(1) within leaf).

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(in crate::runtime_decl) fn define_list_iter(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let list_get_fn = self.module.get_function("action_list_get").unwrap();

        // Cache layout at cache_ptr (i8*): [valid:i64][last_idx:i64][leaf:ptr][pos:i64]
        let cache_valid_off = zero;
        let cache_last_off = i64.const_int(8, false);
        let cache_leaf_off = i64.const_int(16, false);
        let cache_pos_off = i64.const_int(24, false);

        // ---- action_list_find_leaf(node, height, idx) -> {ptr leaf, i64 pos} ----
        let fl_fn = self.module.add_function(
            "action_list_find_leaf",
            self.context
                .struct_type(&[ptr.into(), i64.into()], false)
                .fn_type(&[ptr.into(), i64.into(), i64.into()], false),
            None,
        );
        let fl_entry = self.context.append_basic_block(fl_fn, "entry");
        let fl_h0 = self.context.append_basic_block(fl_fn, "h0");
        let fl_hgt0 = self.context.append_basic_block(fl_fn, "hgt0");
        let fl_loop = self.context.append_basic_block(fl_fn, "loop");
        let fl_found = self.context.append_basic_block(fl_fn, "found");
        let fl_scan_loop = self.context.append_basic_block(fl_fn, "scan_loop");
        let fl_scan_body = self.context.append_basic_block(fl_fn, "scan_body");
        let fl_scan_found = self.context.append_basic_block(fl_fn, "scan_found");
        let fl_scan_next = self.context.append_basic_block(fl_fn, "scan_next");

        self.builder.position_at_end(fl_entry);
        let fl_node = fl_fn.get_first_param().unwrap().into_pointer_value();
        let fl_height = fl_fn.get_nth_param(1).unwrap().into_int_value();
        let fl_idx = fl_fn.get_nth_param(2).unwrap().into_int_value();
        let is_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, fl_height, zero, "is_h0")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(is_h0, fl_h0, fl_hgt0);

        self.builder.position_at_end(fl_h0);
        let fl_pair_ty = self.context.struct_type(&[ptr.into(), i64.into()], false);
        let fl_undef = fl_pair_ty.get_undef();
        let fl_r1 = self
            .builder
            .build_insert_value(fl_undef, fl_node, 0, "r1")
            .map_err(llvm_err)?;
        let fl_r2 = self
            .builder
            .build_insert_value(fl_r1, fl_idx, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fl_r2));

        self.builder.position_at_end(fl_hgt0);
        let _ = self.builder.build_unconditional_branch(fl_loop);

        self.builder.position_at_end(fl_loop);
        let phi_node = self.builder.build_phi(ptr, "phi_n").map_err(llvm_err)?;
        let phi_h = self.builder.build_phi(i64, "phi_h").map_err(llvm_err)?;
        let phi_idx = self.builder.build_phi(i64, "phi_i").map_err(llvm_err)?;
        phi_node.add_incoming(&[(&fl_node, fl_hgt0)]);
        phi_h.add_incoming(&[(&fl_height, fl_hgt0)]);
        phi_idx.add_incoming(&[(&fl_idx, fl_hgt0)]);
        let cur_node = phi_node.as_basic_value().into_pointer_value();
        let cur_h = phi_h.as_basic_value().into_int_value();
        let cur_idx = phi_idx.as_basic_value().into_int_value();
        let at_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, cur_h, zero, "at_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(at_leaf, fl_found, fl_scan_loop);

        self.builder.position_at_end(fl_found);
        let fl_f_undef = fl_pair_ty.get_undef();
        let fl_f1 = self
            .builder
            .build_insert_value(fl_f_undef, cur_node, 0, "fr1")
            .map_err(llvm_err)?;
        let fl_f2 = self
            .builder
            .build_insert_value(fl_f1, cur_idx, 1, "fr2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fl_f2));

        self.builder.position_at_end(fl_scan_loop);
        let scan_i_phi = self.builder.build_phi(i64, "scan_i").map_err(llvm_err)?;
        let scan_acc_phi = self.builder.build_phi(i64, "scan_acc").map_err(llvm_err)?;
        scan_i_phi.add_incoming(&[(&zero, fl_loop)]);
        scan_acc_phi.add_incoming(&[(&zero, fl_loop)]);
        let scan_i = scan_i_phi.as_basic_value().into_int_value();
        let scan_acc = scan_acc_phi.as_basic_value().into_int_value();
        let int_i8 = self
            .builder
            .build_pointer_cast(cur_node, ptr, "int_i8")
            .map_err(llvm_err)?;
        let int_count_raw = self
            .builder
            .build_load(self.context.i32_type(), int_i8, "ic_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let int_count = self
            .builder
            .build_int_z_extend(int_count_raw, i64, "ic")
            .map_err(llvm_err)?;
        let done = self
            .builder
            .build_int_compare(IntPredicate::SGE, scan_i, int_count, "done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done, fl_scan_found, fl_scan_body);

        self.builder.position_at_end(fl_scan_body);
        let cb = unsafe {
            self.builder
                .build_gep(i8, int_i8, &[i64.const_int(16, false)], "cb")
                .map_err(llvm_err)
        }?;
        let cep = unsafe {
            self.builder
                .build_gep(self.child_entry_type, cb, &[scan_i], "cep")
                .map_err(llvm_err)
        }?;
        let ct = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, cep, "ce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                1,
                "ct",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let new_acc = self
            .builder
            .build_int_add(scan_acc, ct, "na")
            .map_err(llvm_err)?;
        let hit = self
            .builder
            .build_int_compare(IntPredicate::SLT, cur_idx, new_acc, "hit")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(hit, fl_scan_found, fl_scan_next);

        self.builder.position_at_end(fl_scan_next);
        let ni = self
            .builder
            .build_int_add(scan_i, one, "ni")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fl_scan_loop);
        scan_i_phi.add_incoming(&[(&ni, fl_scan_next)]);
        scan_acc_phi.add_incoming(&[(&new_acc, fl_scan_next)]);

        self.builder.position_at_end(fl_scan_found);
        let fi_phi = self.builder.build_phi(i64, "fi").map_err(llvm_err)?;
        let fa_phi = self.builder.build_phi(i64, "fa").map_err(llvm_err)?;
        fi_phi.add_incoming(&[(&scan_i, fl_scan_body), (&scan_i, fl_scan_loop)]);
        fa_phi.add_incoming(&[(&scan_acc, fl_scan_body), (&scan_acc, fl_scan_loop)]);
        let fi = fi_phi.as_basic_value().into_int_value();
        let fa = fa_phi.as_basic_value().into_int_value();
        let fcb = unsafe {
            self.builder
                .build_gep(i8, int_i8, &[i64.const_int(16, false)], "fcb")
                .map_err(llvm_err)
        }?;
        let fcep = unsafe {
            self.builder
                .build_gep(self.child_entry_type, fcb, &[fi], "fcep")
                .map_err(llvm_err)
        }?;
        let child = self
            .builder
            .build_extract_value(
                self.builder
                    .build_load(self.child_entry_type, fcep, "fce")
                    .map_err(llvm_err)?
                    .into_struct_value(),
                0,
                "child",
            )
            .map_err(llvm_err)?
            .into_pointer_value();
        let local = self
            .builder
            .build_int_sub(cur_idx, fa, "local")
            .map_err(llvm_err)?;
        let new_h = self
            .builder
            .build_int_sub(cur_h, one, "nh")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fl_loop);
        phi_node.add_incoming(&[(&child, fl_scan_found)]);
        phi_h.add_incoming(&[(&new_h, fl_scan_found)]);
        phi_idx.add_incoming(&[(&local, fl_scan_found)]);

        // ---- action_list_get_cached(list, idx, cache_ptr) -> {i64, ptr} ----
        let lgc_fn = self.module.add_function(
            "action_list_get_cached",
            self.string_type
                .fn_type(&[self.list_type.into(), i64.into(), ptr.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(lgc_fn, "entry");
        let fast = self.context.append_basic_block(lgc_fn, "fast");
        let fast_load = self.context.append_basic_block(lgc_fn, "fast_load");
        let slow = self.context.append_basic_block(lgc_fn, "slow");
        let slow_update = self.context.append_basic_block(lgc_fn, "slow_update");
        let ret_bb = self.context.append_basic_block(lgc_fn, "ret");

        self.builder.position_at_end(entry);
        let lgc_list = lgc_fn.get_first_param().unwrap().into_struct_value();
        let lgc_idx = lgc_fn.get_nth_param(1).unwrap().into_int_value();
        let lgc_cache = lgc_fn.get_nth_param(2).unwrap().into_pointer_value();
        let valid_p = unsafe {
            self.builder
                .build_gep(i8, lgc_cache, &[cache_valid_off], "valid_p")
                .map_err(llvm_err)
        }?;
        let last_p = unsafe {
            self.builder
                .build_gep(i8, lgc_cache, &[cache_last_off], "last_p")
                .map_err(llvm_err)
        }?;
        let leaf_p = unsafe {
            self.builder
                .build_gep(i8, lgc_cache, &[cache_leaf_off], "leaf_p")
                .map_err(llvm_err)
        }?;
        let pos_p = unsafe {
            self.builder
                .build_gep(i8, lgc_cache, &[cache_pos_off], "pos_p")
                .map_err(llvm_err)
        }?;
        let valid = self
            .builder
            .build_load(i64, valid_p, "valid")
            .map_err(llvm_err)?
            .into_int_value();
        let last_idx = self
            .builder
            .build_load(i64, last_p, "last_idx")
            .map_err(llvm_err)?
            .into_int_value();
        let is_valid = self
            .builder
            .build_int_compare(IntPredicate::NE, valid, zero, "is_valid")
            .map_err(llvm_err)?;
        let expected = self
            .builder
            .build_int_add(last_idx, one, "expected")
            .map_err(llvm_err)?;
        let is_seq = self
            .builder
            .build_int_compare(IntPredicate::EQ, lgc_idx, expected, "is_seq")
            .map_err(llvm_err)?;
        let can_fast = self
            .builder
            .build_and(is_valid, is_seq, "can_fast")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(can_fast, fast, slow);

        self.builder.position_at_end(fast);
        let leaf = self
            .builder
            .build_load(ptr, leaf_p, "leaf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pos = self
            .builder
            .build_load(i64, pos_p, "pos")
            .map_err(llvm_err)?
            .into_int_value();
        let leaf_i8 = self
            .builder
            .build_pointer_cast(leaf, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let count_raw = self
            .builder
            .build_load(self.context.i32_type(), leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let count = self
            .builder
            .build_int_z_extend(count_raw, i64, "count")
            .map_err(llvm_err)?;
        let next_pos = self
            .builder
            .build_int_add(pos, one, "next_pos")
            .map_err(llvm_err)?;
        let in_leaf = self
            .builder
            .build_int_compare(IntPredicate::SLT, next_pos, count, "in_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(in_leaf, fast_load, slow);

        self.builder.position_at_end(fast_load);
        let eb = unsafe {
            self.builder
                .build_gep(i8, leaf_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)
        }?;
        let ep = unsafe {
            self.builder
                .build_gep(self.string_type, eb, &[next_pos], "ep")
                .map_err(llvm_err)
        }?;
        let elem = self
            .builder
            .build_load(self.string_type, ep, "elem")
            .map_err(llvm_err)?;
        self.builder
            .build_store(pos_p, next_pos)
            .map_err(llvm_err)?;
        self.builder
            .build_store(last_p, lgc_idx)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ret_bb);

        self.builder.position_at_end(slow);
        let elem_slow = self
            .builder
            .build_call(list_get_fn, &[lgc_list.into(), lgc_idx.into()], "get")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?;
        let _ = self.builder.build_unconditional_branch(slow_update);

        self.builder.position_at_end(slow_update);
        let lgc_node = self
            .builder
            .build_extract_value(lgc_list, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lgc_height = self
            .builder
            .build_extract_value(lgc_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let fl = self
            .builder
            .build_call(
                fl_fn,
                &[lgc_node.into(), lgc_height.into(), lgc_idx.into()],
                "fl",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("find_leaf failed")?
            .into_struct_value();
        let fl_leaf = self
            .builder
            .build_extract_value(fl, 0, "fl_leaf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_pos = self
            .builder
            .build_extract_value(fl, 1, "fl_pos")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder.build_store(valid_p, one).map_err(llvm_err)?;
        self.builder
            .build_store(last_p, lgc_idx)
            .map_err(llvm_err)?;
        self.builder
            .build_store(leaf_p, fl_leaf)
            .map_err(llvm_err)?;
        self.builder.build_store(pos_p, fl_pos).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ret_bb);

        self.builder.position_at_end(ret_bb);
        let ret_phi = self
            .builder
            .build_phi(self.string_type, "ret_phi")
            .map_err(llvm_err)?;
        ret_phi.add_incoming(&[(&elem, fast_load), (&elem_slow, slow_update)]);
        let _ = self.builder.build_return(Some(&ret_phi.as_basic_value()));

        Ok(())
    }
}
