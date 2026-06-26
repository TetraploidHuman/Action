// Submodule: builtins_stdlib_collection/list_gen

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn collection_dispatch_list_gen(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
            "range" => {
                if args.len() != 2 {
                    return Err("range expects 2 arguments (start, end)".to_string());
                }
                let start = self.compile_call_arg(args[0])?;
                let end = self.compile_call_arg(args[1])?;
                match (&start, &end) {
                    (TypedValue::Int(sv), TypedValue::Int(ev)) => {
                        let cc =
                            self.call_rt("action_list_range", &[(*sv).into(), (*ev).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("range failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "range")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("range: arguments must be Int".to_string()),
                }
            }
            "repeat" => {
                if args.len() != 2 {
                    return Err("repeat expects 2 arguments (value, count)".to_string());
                }
                let val = self.compile_call_arg(args[0])?;
                let count = self.compile_call_arg(args[1])?;
                match count {
                    TypedValue::Int(cv) => {
                        let cap = self.i64_ty().const_int(4, false);
                        let new_list = self.call_rt("action_list_create", &[cap.into()])?;
                        let new_list_bv = new_list
                            .try_as_basic_value()
                            .basic()
                            .ok_or("create failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "repeat")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, new_list_bv)
                            .map_err(llvm_err)?;
                        let fat = self.to_fat_struct(&val)?;
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let entry_block = current_fn.get_last_basic_block().unwrap();
                        let loop_bb = self.context.append_basic_block(current_fn, "repeat_loop");
                        let done_bb = self.context.append_basic_block(current_fn, "repeat_done");
                        let _ = self.builder.build_unconditional_branch(loop_bb);
                        self.builder.position_at_end(loop_bb);
                        let i = self
                            .builder
                            .build_phi(self.i64_ty(), "rep_i")
                            .map_err(llvm_err)?;
                        let lv = self.load_list(alloca)?;
                        let pushed = self.call_rt("action_list_push", &[lv.into(), fat.into()])?;
                        let pb = pushed.try_as_basic_value().basic().ok_or("push failed")?;
                        self.builder.build_store(alloca, pb).map_err(llvm_err)?;
                        let ni = self
                            .builder
                            .build_int_add(
                                i.as_basic_value().into_int_value(),
                                self.i64_ty().const_int(1, false),
                                "rep_ni",
                            )
                            .map_err(llvm_err)?;
                        let done_cond = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, ni, cv, "rep_done")
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
                    _ => Err("repeat: second argument must be Int".to_string()),
                }
            }
            "tail" => {
                if args.len() != 1 {
                    return Err("tail expects 1 argument (list)".to_string());
                }
                Ok(Some(self.compile_tail_fallible_call(args[0])?))
            }
            "zip" => {
                if args.len() != 2 {
                    return Err("zip expects 2 arguments (list1, list2)".to_string());
                }
                let v1 = self.compile_call_arg(args[0])?;
                let v2 = self.compile_call_arg(args[1])?;
                match (&v1, &v2) {
                    (TypedValue::List(lp1), TypedValue::List(lp2)) => {
                        let lv1 = self.load_list(*lp1)?;
                        let lv2 = self.load_list(*lp2)?;
                        let cc = self.call_rt("action_list_zip", &[lv1.into(), lv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("zip failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "zip")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("zip: arguments must be lists".to_string()),
                }
            }
            "insert" => {
                if args.len() != 3 {
                    return Err("insert expects 3 arguments (list, index, elem)".to_string());
                }
                let list_val = self.compile_call_arg(args[0])?;
                let idx_val = self.compile_call_arg(args[1])?;
                let elem_val = self.compile_call_arg(args[2])?;
                match (&list_val, &idx_val) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let result = self.call_rt(
                            "action_list_insert",
                            &[lv.into(), (*iv).into(), fat.into()],
                        )?;
                        let rv = result.try_as_basic_value().basic().ok_or("insert failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "insert_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, rv).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("insert expects (List, Int, Any)".to_string()),
                }
            }
            "init" => {
                if args.len() != 1 {
                    return Err("init expects 1 argument (list)".to_string());
                }
                Ok(Some(self.compile_init_fallible_call(args[0])?))
            }
            _ => Ok(None),
        }
    }
}
