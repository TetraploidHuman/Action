// Submodule: builtins_map

use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// mapFilter(map, predicate) or mapFilter(predicate, map) or mapFilter(map) { k, v -> ... }
    /// Predicate takes (key_tag, val_tag) -> Bool (fat {i64,ptr} with tag=1 true, 0 false)
    pub(crate) fn builtin_map_filter(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, map_ptr) = if let Some(lam) = trailing {
            if args.len() != 1 {
                return Err("mapFilter with trailing lambda expects 1 argument (map)".to_string());
            }
            let mv = self.compile_call_arg(args[0])?;
            let fv = self.compile_call_arg(lam)?;
            (fv, mv)
        } else if args.len() == 2 {
            // Could be mapFilter(map, fn) or mapFilter(fn, map) - check types
            let a0 = self.compile_call_arg(args[0])?;
            let a1 = self.compile_call_arg(args[1])?;
            if matches!(a0, TypedValue::Map(_)) {
                (a1, a0)
            } else {
                (a0, a1)
            }
        } else {
            return Err("mapFilter expects 2 arguments (map, predicate)".to_string());
        };

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("mapFilter: predicate must be a function".to_string()),
        };
        let map_ptr = match map_ptr {
            TypedValue::Map(p) => p,
            _ => return Err("mapFilter: first argument must be a map".to_string()),
        };

        let map_struct = self.load_list(map_ptr)?;
        let input_len = self.list_len_val(map_struct)?;
        let map_cap = self
            .builder
            .build_extract_value(map_struct, 2, "mf_cap")
            .map_err(llvm_err)?
            .into_int_value();
        let data_ptr = self.list_data_ptr(map_struct)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile mapFilter outside function")?;

        let i64 = self.i64_ty();

        // Create new empty map (use input_len as capacity)
        let cc = self.call_rt("action_map_create", &[input_len.into()])?;
        let new_map_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "mf_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_map_bv)
            .map_err(llvm_err)?;

        let i_alloca = self.builder.build_alloca(i64, "mf_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_header = self.context.append_basic_block(current_fn, "mf_hdr");
        let loop_chk = self.context.append_basic_block(current_fn, "mf_chk");
        let loop_body = self.context.append_basic_block(current_fn, "mf_bdy");
        let loop_insert = self.context.append_basic_block(current_fn, "mf_ins");
        let loop_next = self.context.append_basic_block(current_fn, "mf_nxt");
        let loop_exit = self.context.append_basic_block(current_fn, "mf_ext");

        let _ = self.builder.build_unconditional_branch(loop_header);

        // Header: scan slots 0..cap-1 (Robin-Hood layout)
        self.builder.position_at_end(loop_header);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, map_cap, "mf_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_chk, loop_exit);

        self.builder.position_at_end(loop_chk);
        self.ht_branch_if_slot_active(data_ptr, i_val, loop_body, loop_next)?;

        // Body: load key/value from slot, call predicate
        self.builder.position_at_end(loop_body);
        let key_fat = self.ht_key_fat_at(data_ptr, i_val)?;
        let val_fat = self.ht_val_fat_at(data_ptr, i_val)?;
        let kt = self
            .builder
            .build_extract_value(key_fat.into_struct_value(), 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let vt = self
            .builder
            .build_extract_value(val_fat.into_struct_value(), 0, "vt")
            .map_err(llvm_err)?
            .into_int_value();

        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[kt.into(), vt.into()], "mf_call")
            .map_err(llvm_err)?;
        let pred_bv = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("mf call failed")?;
        let pred_tag = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let keep = self
            .builder
            .build_int_compare(IntPredicate::NE, pred_tag, i64.const_int(0, false), "keep")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(keep, loop_insert, loop_next);

        // Insert: add entry to result map, then go to next
        self.builder.position_at_end(loop_insert);
        let cur_map = self
            .builder
            .build_load(self.list_type, result_alloca, "cur_map")
            .map_err(llvm_err)?
            .into_struct_value();
        let ins_cc = self.call_rt(
            "action_map_insert",
            &[cur_map.into(), key_fat.into(), val_fat.into()],
        )?;
        let new_map = ins_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_insert failed")?;
        self.builder
            .build_store(result_alloca, new_map)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_next);

        // Next: increment i, go back to header
        self.builder.position_at_end(loop_next);
        let ni = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_header);

        // Exit
        self.builder.position_at_end(loop_exit);
        Ok(TypedValue::Map(result_alloca))
    }

    /// mapMapValues(map, transform) or mapMapValues(transform, map) or mapMapValues(map) { v -> ... }
    /// Transform takes val_tag -> new_val (fat {i64, ptr})
    pub(crate) fn builtin_map_map_values(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, map_ptr) = if let Some(lam) = trailing {
            if args.len() != 1 {
                return Err(
                    "mapMapValues with trailing lambda expects 1 argument (map)".to_string()
                );
            }
            let mv = self.compile_call_arg(args[0])?;
            let fv = self.compile_call_arg(lam)?;
            (fv, mv)
        } else if args.len() == 2 {
            let a0 = self.compile_call_arg(args[0])?;
            let a1 = self.compile_call_arg(args[1])?;
            if matches!(a0, TypedValue::Map(_)) {
                (a1, a0)
            } else {
                (a0, a1)
            }
        } else {
            return Err("mapMapValues expects 2 arguments (map, transform)".to_string());
        };

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("mapMapValues: transform must be a function".to_string()),
        };
        let map_ptr = match map_ptr {
            TypedValue::Map(p) => p,
            _ => return Err("mapMapValues: first argument must be a map".to_string()),
        };

        let map_struct = self.load_list(map_ptr)?;
        let input_len = self.list_len_val(map_struct)?;
        let map_cap = self
            .builder
            .build_extract_value(map_struct, 2, "mmv_cap")
            .map_err(llvm_err)?
            .into_int_value();
        let data_ptr = self.list_data_ptr(map_struct)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile mapMapValues outside function")?;

        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);

        // CoW-copy table in place (keys/dist unchanged); update values without re-insert reprobing.
        let entry_bb = self
            .builder
            .get_insert_block()
            .ok_or("mapMapValues: no insert block")?;
        let cow_bb = self.context.append_basic_block(current_fn, "mmv_cow");
        let cow_merge = self.context.append_basic_block(current_fn, "mmv_mrg");
        let cow_data = self.ht_cow(data_ptr, map_cap, entry_bb, cow_bb, cow_merge)?;
        self.builder.position_at_end(cow_merge);
        let result_map = self.ht_pack(cow_data, input_len, map_cap)?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "mmv_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, result_map)
            .map_err(llvm_err)?;

        let i_alloca = self.builder.build_alloca(i64, "mmv_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_header = self.context.append_basic_block(current_fn, "mmv_hdr");
        let loop_chk = self.context.append_basic_block(current_fn, "mmv_chk");
        let loop_body = self.context.append_basic_block(current_fn, "mmv_bdy");
        let loop_vp_dec = self.context.append_basic_block(current_fn, "mmv_vpd");
        let loop_store = self.context.append_basic_block(current_fn, "mmv_st");
        let loop_next = self.context.append_basic_block(current_fn, "mmv_nxt");
        let loop_exit = self.context.append_basic_block(current_fn, "mmv_ext");

        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_header);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, map_cap, "mmv_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_chk, loop_exit);

        self.builder.position_at_end(loop_chk);
        self.ht_branch_if_slot_active(cow_data, i_val, loop_body, loop_next)?;

        self.builder.position_at_end(loop_body);
        let (kt, kp, _vt_old, vp_old, dist) = self.ht_load_slot(cow_data, i_val)?;
        let val_fat = self.ht_val_fat_at(cow_data, i_val)?;
        let vt = self
            .builder
            .build_extract_value(val_fat.into_struct_value(), 0, "vt")
            .map_err(llvm_err)?
            .into_int_value();

        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[vt.into()], "mmv_call")
            .map_err(llvm_err)?;
        let new_val_bv = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("mmv call failed")?;
        let new_val = new_val_bv.into_struct_value();
        let new_vt = self
            .builder
            .build_extract_value(new_val, 0, "nvt")
            .map_err(llvm_err)?
            .into_int_value();
        let new_vp_ptr = self
            .builder
            .build_extract_value(new_val, 1, "nvp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let new_vp = self
            .builder
            .build_ptr_to_int(new_vp_ptr, i64, "nvp_i")
            .map_err(llvm_err)?;

        let vp_ne = self
            .builder
            .build_int_compare(IntPredicate::NE, vp_old, zero, "vp_ne")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(vp_ne, loop_vp_dec, loop_store);

        self.builder.position_at_end(loop_vp_dec);
        let rc_dec_fn = self
            .module
            .get_function("action_rc_dec")
            .ok_or("action_rc_dec not found")?;
        let _ = self.builder.build_call(
            rc_dec_fn,
            &[self
                .builder
                .build_int_to_ptr(vp_old, ptr, "vp_p")
                .map_err(llvm_err)?
                .into()],
            "",
        );
        let _ = self.builder.build_unconditional_branch(loop_store);

        self.builder.position_at_end(loop_store);
        self.ht_store_slot(cow_data, i_val, kt, kp, new_vt, new_vp, dist)?;
        let _ = self.builder.build_unconditional_branch(loop_next);

        self.builder.position_at_end(loop_next);
        let ni = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_exit);
        Ok(TypedValue::Map(result_alloca))
    }

    /// mapFold(map, init, folder) or mapFold(init, folder, map) or mapFold(init, map) { acc, k, v -> ... }
    /// Folder takes (acc_tag, key_tag, val_tag) -> new_acc (fat {i64, ptr})
    pub(crate) fn builtin_map_fold(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, init_val, map_ptr) = if let Some(lam) = trailing {
            if args.len() != 2 {
                return Err(
                    "mapFold with trailing lambda expects 2 arguments (map, init)".to_string(),
                );
            }
            let a0 = self.compile_call_arg(args[0])?;
            let a1 = self.compile_call_arg(args[1])?;
            let fv = self.compile_call_arg(lam)?;
            if matches!(a0, TypedValue::Map(_)) {
                (fv, a1, a0)
            } else {
                (fv, a0, a1)
            }
        } else if args.len() == 3 {
            // Could be mapFold(fn, init, map) or mapFold(init, fn, map) or mapFold(init, map, fn)
            // Try to determine by checking which arg is a map
            let a0 = self.compile_call_arg(args[0])?;
            let a1 = self.compile_call_arg(args[1])?;
            let a2 = self.compile_call_arg(args[2])?;
            if matches!(a2, TypedValue::Map(_)) {
                // Last is map, first two are fn+init or init+fn
                if matches!(a1, TypedValue::Fn(_, _)) {
                    (a1, a0, a2) // fn, init, map
                } else {
                    (a0, a1, a2) // fn(assume a0), init(a1), map(a2)
                }
            } else if matches!(a1, TypedValue::Map(_)) {
                (a0, a2, a1) // fn(a0), init(a2), map(a1)
            } else if matches!(a0, TypedValue::Map(_)) {
                (a1, a2, a0) // fn(a1), init(a2), map(a0)
            } else {
                return Err("mapFold: one argument must be a map".to_string());
            }
        } else {
            return Err("mapFold expects 3 arguments (map, init, folder)".to_string());
        };

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("mapFold: folder must be a function".to_string()),
        };
        let map_ptr = match map_ptr {
            TypedValue::Map(p) => p,
            _ => return Err("mapFold: map argument must be a map".to_string()),
        };
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Err("mapFold: init must be an integer".to_string()),
        };

        let map_struct = self.load_list(map_ptr)?;
        let _input_len = self.list_len_val(map_struct)?;
        let map_cap = self
            .builder
            .build_extract_value(map_struct, 2, "mfld_cap")
            .map_err(llvm_err)?
            .into_int_value();
        let data_ptr = self.list_data_ptr(map_struct)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile mapFold outside function")?;

        let i64 = self.i64_ty();

        let acc_alloca = self
            .builder
            .build_alloca(i64, "mfld_acc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(acc_alloca, init_i64)
            .map_err(llvm_err)?;

        let i_alloca = self.builder.build_alloca(i64, "mfld_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_header = self.context.append_basic_block(current_fn, "mfld_hdr");
        let loop_chk = self.context.append_basic_block(current_fn, "mfld_chk");
        let loop_body = self.context.append_basic_block(current_fn, "mfld_bdy");
        let loop_next = self.context.append_basic_block(current_fn, "mfld_nxt");
        let loop_exit = self.context.append_basic_block(current_fn, "mfld_ext");

        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_header);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, map_cap, "mfld_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_chk, loop_exit);

        self.builder.position_at_end(loop_chk);
        self.ht_branch_if_slot_active(data_ptr, i_val, loop_body, loop_next)?;

        self.builder.position_at_end(loop_body);
        let key_fat = self.ht_key_fat_at(data_ptr, i_val)?;
        let val_fat = self.ht_val_fat_at(data_ptr, i_val)?;
        let kt = self
            .builder
            .build_extract_value(key_fat.into_struct_value(), 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let vt = self
            .builder
            .build_extract_value(val_fat.into_struct_value(), 0, "vt")
            .map_err(llvm_err)?
            .into_int_value();

        // Call folder(acc_tag, key_tag, val_tag) -> fat {i64, ptr} (new acc)
        let acc = self
            .builder
            .build_load(i64, acc_alloca, "acc")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into(), i64.into()], false);
        let call_result = self
            .builder
            .build_indirect_call(
                fn_type,
                fn_ptr,
                &[acc.into(), kt.into(), vt.into()],
                "mfld_call",
            )
            .map_err(llvm_err)?;
        let new_acc_bv = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("mfld call failed")?;
        let new_acc = if new_acc_bv.is_struct_value() {
            self.builder
                .build_extract_value(new_acc_bv.into_struct_value(), 0, "mfld_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            new_acc_bv.into_int_value()
        };
        self.builder
            .build_store(acc_alloca, new_acc)
            .map_err(llvm_err)?;

        let ni = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_exit);
        let final_acc = self
            .builder
            .build_load(i64, acc_alloca, "final_acc")
            .map_err(llvm_err)?;
        Ok(TypedValue::Int(final_acc.into_int_value()))
    }
}
