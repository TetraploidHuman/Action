// Submodule: builtins_stdlib_collection/list_basic

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};
use inkwell::values::{IntValue, StructValue};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn list_sum_from_loaded(
        &mut self,
        list: StructValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let len = self.list_len_val(list)?;
        let data = self.list_data_ptr(list)?;
        let current = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let sum_a = self
            .builder
            .build_alloca(self.i64_ty(), "sum")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sum_a, self.i64_ty().const_int(0, false))
            .map_err(llvm_err)?;
        let i_a = self
            .builder
            .build_alloca(self.i64_ty(), "i")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_a, self.i64_ty().const_int(0, false))
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current, "sum_hdr");
        let bdy = self.context.append_basic_block(current, "sum_bdy");
        let ext = self.context.append_basic_block(current, "sum_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(self.i64_ty(), i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let ep = unsafe {
            self.builder
                .build_gep(self.string_type, data, &[iv], "ep")
                .map_err(llvm_err)
        }?;
        let ev = self
            .builder
            .build_load(self.string_type, ep, "ev")
            .map_err(llvm_err)?;
        let etag = self
            .builder
            .build_extract_value(ev.into_struct_value(), 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let cur = self
            .builder
            .build_load(self.i64_ty(), sum_a, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let new_sum = self
            .builder
            .build_int_add(cur, etag, "new_sum")
            .map_err(llvm_err)?;
        self.builder.build_store(sum_a, new_sum).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, self.i64_ty().const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        self.builder
            .build_load(self.i64_ty(), sum_a, "result")
            .map_err(llvm_err)
            .map(|v| v.into_int_value())
    }
}

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn collection_dispatch_list_basic(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
            "head" => {
                if args.len() != 1 {
                    return Err("head expects 1 argument".to_string());
                }
                self.compile_head_fallible(args[0]).map(Some)
            }
            "last" => {
                if args.len() != 1 {
                    return Err("last expects 1 argument".to_string());
                }
                self.compile_last_fallible(args[0]).map(Some)
            }
            "get" => {
                if args.len() != 2 {
                    return Err("get expects 2 arguments (list, index)".to_string());
                }
                let idx_val = self.compile_call_arg(args[1])?;
                if let TypedValue::Int(iv) = idx_val {
                    return self.compile_list_index_fallible(args[0], iv).map(Some);
                }
                Err("get: index must be Int".to_string())
            }
            "remove" => {
                if args.len() != 2 {
                    return Err("remove expects 2 arguments (list, index)".to_string());
                }
                let list_val = self.compile_call_arg(args[0])?;
                let idx_val = self.compile_call_arg(args[1])?;
                match (&list_val, &idx_val) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let result =
                            self.call_rt("action_list_remove", &[lv.into(), (*iv).into()])?;
                        let rv = result.try_as_basic_value().basic().ok_or("remove failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "remove_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, rv).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("remove expects (List, Int)".to_string()),
                }
            }
            "reverse" => {
                if args.len() != 1 {
                    return Err("reverse expects 1 argument".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_reverse", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("reverse failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "rev")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("reverse: argument must be a list".to_string()),
                }
            }
            "contains" => {
                if args.len() != 2 {
                    return Err("contains expects 2 arguments (list, element)".to_string());
                }
                let list_val = self.compile_call_arg(args[0])?;
                let elem_val = self.compile_call_arg(args[1])?;
                match (&list_val, &elem_val) {
                    (TypedValue::List(lp), _) => {
                        let lv = self.load_list(*lp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let cc = self.call_rt("action_list_contains", &[lv.into(), fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("contains failed")?
                            .into_int_value();
                        Ok(Some(TypedValue::Bool(result)))
                    }
                    (TypedValue::Set(sp), _) => {
                        let lv = self.load_list(*sp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let cc = self.call_rt("action_map_contains", &[lv.into(), fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("contains failed")?
                            .into_int_value();
                        Ok(Some(TypedValue::Bool(result)))
                    }
                    _ => Err("contains: first argument must be a list or set".to_string()),
                }
            }
            "containsKey" => {
                if args.len() != 2 {
                    return Err("containsKey expects 2 arguments (map, key)".to_string());
                }
                let map_val = self.compile_call_arg(args[0])?;
                let key_val = self.compile_call_arg(args[1])?;
                match &map_val {
                    TypedValue::Map(mp) => {
                        let lv = self.load_list(*mp)?;
                        let key_fat = self.to_fat_struct(&key_val)?;
                        let cc =
                            self.call_rt("action_map_contains", &[lv.into(), key_fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("map_contains failed")?
                            .into_int_value();
                        Ok(Some(TypedValue::Bool(result)))
                    }
                    _ => Err("containsKey: first argument must be a map".to_string()),
                }
            }
            "prepend" => {
                if args.len() != 2 {
                    return Err("prepend expects 2 arguments (element, list)".to_string());
                }
                let elem_val = self.compile_call_arg(args[0])?;
                let list_val = self.compile_call_arg(args[1])?;
                match list_val {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len_bv = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let new_cap = self
                            .builder
                            .build_int_add(len_bv, self.i64_ty().const_int(4, false), "new_cap")
                            .map_err(llvm_err)?;
                        let new_list = self.call_rt("action_list_create", &[new_cap.into()])?;
                        let new_list_bv = new_list
                            .try_as_basic_value()
                            .basic()
                            .ok_or("create failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "prepend")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, new_list_bv)
                            .map_err(llvm_err)?;
                        // Push element first
                        let fat = self.to_fat_struct(&elem_val)?;
                        let lv2 = self.load_list(alloca)?;
                        let pushed1 =
                            self.call_rt("action_list_push", &[lv2.into(), fat.into()])?;
                        let pb1 = pushed1.try_as_basic_value().basic().ok_or("push1 failed")?;
                        self.builder.build_store(alloca, pb1).map_err(llvm_err)?;
                        // Then push all original elements
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let entry_block = current_fn.get_last_basic_block().unwrap();
                        let loop_bb = self.context.append_basic_block(current_fn, "prepend_loop");
                        let done_bb = self.context.append_basic_block(current_fn, "prepend_done");
                        let _ = self.builder.build_unconditional_branch(loop_bb);
                        self.builder.position_at_end(loop_bb);
                        let i = self
                            .builder
                            .build_phi(self.i64_ty(), "pp_i")
                            .map_err(llvm_err)?;
                        let lv_orig = self.load_list(lp)?;
                        let lv_cur = self.load_list(alloca)?;
                        let elem = self.call_rt(
                            "action_list_get",
                            &[lv_orig.into(), i.as_basic_value().into_int_value().into()],
                        )?;
                        let elem_bv = elem.try_as_basic_value().basic().ok_or("get failed")?;
                        let pushed =
                            self.call_rt("action_list_push", &[lv_cur.into(), elem_bv.into()])?;
                        let pb = pushed.try_as_basic_value().basic().ok_or("push2 failed")?;
                        self.builder.build_store(alloca, pb).map_err(llvm_err)?;
                        let ni = self
                            .builder
                            .build_int_add(
                                i.as_basic_value().into_int_value(),
                                self.i64_ty().const_int(1, false),
                                "pp_ni",
                            )
                            .map_err(llvm_err)?;
                        let done_cond = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, ni, len_bv, "pp_done")
                            .map_err(llvm_err)?;
                        let loop_block = self.builder.get_insert_block().unwrap();
                        i.add_incoming(&[
                            (&self.i64_ty().const_int(0, false), entry_block),
                            (&ni, loop_block),
                        ]);
                        let _ = self
                            .builder
                            .build_conditional_branch(done_cond, done_bb, loop_bb);
                        self.builder.position_at_end(done_bb);
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("prepend: second argument must be a list".to_string()),
                }
            }
            "take" => {
                if args.len() != 2 {
                    return Err("take expects 2 arguments (list, n)".to_string());
                }
                let list_val = self.compile_call_arg(args[0])?;
                let n_val = self.compile_call_arg(args[1])?;
                match (&list_val, &n_val) {
                    (TypedValue::List(lp), TypedValue::Int(nv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_take", &[lv.into(), (*nv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("take failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "take")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("take: first argument must be a list, second an Int".to_string()),
                }
            }
            "drop" => {
                if args.len() != 2 {
                    return Err("drop expects 2 arguments (list, n)".to_string());
                }
                let list_val = self.compile_call_arg(args[0])?;
                let n_val = self.compile_call_arg(args[1])?;
                match (&list_val, &n_val) {
                    (TypedValue::List(lp), TypedValue::Int(nv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_drop", &[lv.into(), (*nv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("drop failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "drop")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("drop: first argument must be a list, second an Int".to_string()),
                }
            }
            _ => Ok(None),
        }
    }
}
