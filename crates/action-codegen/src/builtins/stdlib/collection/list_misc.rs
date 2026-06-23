// Submodule: builtins_stdlib_collection/list_misc

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn collection_dispatch_list_misc(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
            "setToList" => {
                if args.len() != 1 {
                    return Err("setToList expects 1 argument (set)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Set(p) => Ok(Some(TypedValue::List(p))),
                    _ => Err("setToList: argument must be a set".to_string()),
                }
            }
            "randChoice" => {
                if args.len() != 1 {
                    return Err("randChoice expects 1 argument (list)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let empty = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                len,
                                self.i64_ty().const_int(0, false),
                                "empty",
                            )
                            .map_err(llvm_err)?;
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .unwrap()
                            .get_parent()
                            .unwrap();
                        let has_elem = self.context.append_basic_block(current_fn, "has_elem");
                        let no_elem = self.context.append_basic_block(current_fn, "no_elem");
                        let merge = self.context.append_basic_block(current_fn, "merge");
                        let _ = self
                            .builder
                            .build_conditional_branch(empty, no_elem, has_elem);
                        // No element: return None (tag=0)
                        self.builder.position_at_end(no_elem);
                        let none_fat = self.string_type.get_undef();
                        let none1 = self
                            .builder
                            .build_insert_value(
                                none_fat,
                                self.i64_ty().const_int(0, false),
                                0,
                                "none_tag",
                            )
                            .map_err(llvm_err)?;
                        let none2 = self
                            .builder
                            .build_insert_value(none1, self.ptr_ty().const_zero(), 1, "none_data")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge);
                        let none_block = self.builder.get_insert_block().unwrap();
                        // Has element: pick random index
                        self.builder.position_at_end(has_elem);
                        let idx = self
                            .builder
                            .build_int_sub(len, self.i64_ty().const_int(1, false), "max_idx")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt(
                            "action_rand_int",
                            &[self.i64_ty().const_int(0, false).into(), idx.into()],
                        )?;
                        let ri = cc.try_as_basic_value().unwrap_basic().into_int_value();
                        let data = self
                            .builder
                            .build_extract_value(lv, 0, "data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let ep = unsafe {
                            self.builder
                                .build_gep(self.string_type, data, &[ri], "ep")
                                .map_err(llvm_err)
                        }?;
                        let elem = self
                            .builder
                            .build_load(self.string_type, ep, "elem")
                            .map_err(llvm_err)?
                            .into_struct_value();
                        // Wrap in Some: tag=1, data=ptr to elem copy
                        let malloc = self.module.get_function("malloc").unwrap();
                        let some_ptr = self
                            .builder
                            .build_call(
                                malloc,
                                &[self.i64_ty().const_int(16, false).into()],
                                "some",
                            )
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_pointer_value();
                        self.builder.build_store(some_ptr, elem).map_err(llvm_err)?;
                        let some_fat = self.string_type.get_undef();
                        let some1 = self
                            .builder
                            .build_insert_value(
                                some_fat,
                                self.i64_ty().const_int(1, false),
                                0,
                                "some_tag",
                            )
                            .map_err(llvm_err)?;
                        let some2 = self
                            .builder
                            .build_insert_value(some1, some_ptr, 1, "some_data")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge);
                        let some_block = self.builder.get_insert_block().unwrap();
                        // Merge
                        self.builder.position_at_end(merge);
                        let phi = self
                            .builder
                            .build_phi(self.string_type, "choice")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[
                            (&none2.as_basic_value_enum(), none_block),
                            (&some2.as_basic_value_enum(), some_block),
                        ]);
                        // Return as fat struct (Tag=EnumKind(3), data=ptr to fat value)
                        let opt_alloca = self
                            .builder
                            .build_alloca(self.string_type, "opt")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(opt_alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(opt_alloca))) // Reuse List type for the result
                    }
                    _ => Err("randChoice: argument must be a list".to_string()),
                }
            }
            "withIndex" => {
                if args.len() != 1 {
                    return Err("withIndex expects 1 argument (list)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_with_index", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("withIndex failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "wi")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("withIndex: argument must be a list".to_string()),
                }
            }
            _ => Ok(None),
        }
    }
}
