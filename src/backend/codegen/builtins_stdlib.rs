// Submodule: builtins_stdlib

use crate::ast::*;
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// Builtin stdlib functions: len, is_empty, append, concat
    pub(super) fn builtin_stdlib(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "to" => {
                if args.len() != 2 {
                    return Err("to expects 2 arguments".to_string());
                }
                self.compile_tuple(&[(None, args[0].clone()), (None, args[1].clone())])
            }
            "len" => {
                if args.len() != 1 {
                    return Err("len expects 1 argument".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                match val {
                    TypedValue::List(ptr) => {
                        let list = self.load_list(ptr)?;
                        let len = self
                            .builder
                            .build_extract_value(list, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(len))
                    }
                    TypedValue::LazyList(ptr) => {
                        let ll_sv = self
                            .builder
                            .build_load(self.lazylist_type, ptr, "len_ll")
                            .map_err(llvm_err)?
                            .into_struct_value();
                        let take_count = self
                            .builder
                            .build_extract_value(ll_sv, 3, "len_tc")
                            .map_err(llvm_err)?
                            .into_int_value();
                        // If take_count > 0, that's the length. If 0 (no step fn), it's 1.
                        // If -1 (infinite), return -1.
                        let zero = self.i64_ty().const_int(0, false);
                        let one = self.i64_ty().const_int(1, false);
                        let is_zero = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, take_count, zero, "tc_zero")
                            .map_err(llvm_err)?;
                        let result_len = self
                            .builder
                            .build_select(is_zero, one, take_count, "ll_len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result_len))
                    }
                    TypedValue::Str(ptr) => {
                        let str_val = self.load_string(ptr)?;
                        let len = self
                            .builder
                            .build_extract_value(str_val, 0, "slen")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(len))
                    }
                    TypedValue::Map(ptr) => {
                        let m = self.load_list(ptr)?;
                        let len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(len))
                    }
                    TypedValue::Set(ptr) => {
                        let m = self.load_list(ptr)?;
                        let len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(len))
                    }
                    _ => Err(
                        "len: argument must be a list, string, map, set, or lazy list".to_string(),
                    ),
                }
            }
            "isEmpty" => {
                if args.len() != 1 {
                    return Err("isEmpty expects 1 argument".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                let len = match val {
                    TypedValue::List(ptr) => {
                        let list = self.load_list(ptr)?;
                        self.builder
                            .build_extract_value(list, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value()
                    }
                    TypedValue::LazyList(_) => {
                        // A LazyList always has at least the head element, so never empty
                        self.i64_ty().const_int(1, false)
                    }
                    TypedValue::Str(ptr) => {
                        let str_val = self.load_string(ptr)?;
                        self.builder
                            .build_extract_value(str_val, 0, "slen")
                            .map_err(llvm_err)?
                            .into_int_value()
                    }
                    TypedValue::Map(ptr) => {
                        let m = self.load_list(ptr)?;
                        self.builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value()
                    }
                    TypedValue::Set(ptr) => {
                        let m = self.load_list(ptr)?;
                        self.builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value()
                    }
                    _ => {
                        return Err(
                            "is_empty: argument must be a list, string, map, set, or lazy list"
                                .to_string(),
                        )
                    }
                };
                let zero = self.i64_ty().const_int(0, false);
                let is_empty = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "is_empty")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_empty))
            }
            "append" => {
                if args.len() != 2 {
                    return Err("append expects 2 arguments (list, element)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("append: first argument must be a list".to_string()),
                };
                let elem_val = self.compile_expr(&args[1])?;
                // action_list_push handles rc_inc of the element data_ptr internally
                let elem_fat = self.to_fat_struct(&elem_val)?;
                let list = self.load_list(list_ptr)?;
                let cc = self.call_rt("action_list_push", &[list.into(), elem_fat.into()])?;
                let new_list = cc.try_as_basic_value().basic().ok_or("list_push failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "appended")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(alloca, new_list)
                    .map_err(llvm_err)?;
                Ok(TypedValue::List(alloca))
            }
            "concat" => {
                if args.len() != 2 {
                    return Err("concat expects 2 arguments".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Str(p1), TypedValue::Str(p2)) => {
                        let s1 = self.load_string(*p1)?;
                        let s2 = self.load_string(*p2)?;
                        let cc = self.call_rt("action_string_concat", &[s1.into(), s2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("string_concat failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "concat_str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    (TypedValue::List(p1), TypedValue::List(p2)) => {
                        let l1 = self.load_list(*p1)?;
                        let l2 = self.load_list(*p2)?;
                        let result = self
                            .call_rt("action_list_concat", &[l1.into(), l2.into()])?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("list_concat failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "concat_list")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    (TypedValue::Map(p1), TypedValue::Map(p2)) => {
                        let m1 = self.load_list(*p1)?;
                        let m2 = self.load_list(*p2)?;
                        let result = self
                            .call_rt("action_map_union", &[m1.into(), m2.into()])?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("map_union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "concat_map")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Map(alloca))
                    }
                    (TypedValue::Set(p1), TypedValue::Set(p2)) => {
                        let s1 = self.load_list(*p1)?;
                        let s2 = self.load_list(*p2)?;
                        let result = self
                            .call_rt("action_set_union", &[s1.into(), s2.into()])?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set_union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "concat_set")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("concat: arguments must be both strings, both lists, both maps, or both sets".to_string()),
                }
            }
            "toUpper" | "toLower" | "trim" | "startsWith" | "endsWith" | "substring"
            | "parseInt" | "split" | "join" | "replace" | "trimStart" | "trimEnd"
            | "stringContains" | "stringRepeat" | "splitLines" | "indexOf" | "chars" | "charAt"
            | "isAlpha" | "codeToChar" | "toChar" | "charCode" => {
                self.builtin_stdlib_string(name, args)
            }
            "head" | "last" | "get" | "remove" | "reverse" | "contains" | "prepend" | "take"
            | "drop" | "range" | "repeat" | "tail" | "zip" | "insert" | "init" | "withIndex"
            | "unique" | "slice" | "flatten" | "splitAt" | "chunks" | "windows" | "sorted"
            | "containsKey" | "setToList" | "mapKeys" | "mapValues" | "mapEntries" | "mapUnion"
            | "setUnion" | "setIntersection" | "setDifference" | "setIsSubset" | "sum"
            | "product" | "digits" | "randChoice" | "randShuffle" => {
                self.builtin_stdlib_collection(name, args)
            }
            "readLine" | "readFile" | "writeFile" | "appendFile" | "exists" | "deleteFile"
            | "openFile" | "closeFile" | "isEof" | "fileReadLine" | "fileReadBytes"
            | "fileWrite" | "fileWriteLine" | "fileFlush" | "fileSeek" | "fileTell" | "readDir" => {
                self.builtin_stdlib_io(name, args)
            }
            "abs" | "min" | "max" | "sqrt" | "cbrt" | "sin" | "cos" | "tan" | "asin" | "acos"
            | "atan" | "atan2" | "log" | "log2" | "log10" | "exp" | "floor" | "ceil" | "round"
            | "pi" | "e" | "clamp" | "isNaN" | "isInfinite" | "pow" => {
                self.builtin_stdlib_math(name, args)
            }
            "panic" => {
                if args.len() != 1 {
                    return Err("panic expects 1 argument (message)".to_string());
                }
                let msg = self.compile_expr(&args[0])?;
                match msg {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let _ = self.call_rt("action_print_string", &[sv.into()])?;
                        let _ = self.call_rt("action_println", &[])?;
                        // Call exit(1)
                        let exit_fn = self.module.get_function("exit");
                        if exit_fn.is_none() {
                            let _ = self.module.add_function(
                                "exit",
                                self.void_ty().fn_type(&[self.i32_ty().into()], false),
                                None,
                            );
                        }
                        let exit_fn = self.module.get_function("exit").unwrap();
                        let one = self.i32_ty().const_int(1, false);
                        let _ = self
                            .builder
                            .build_call(exit_fn, &[one.into()], "")
                            .map_err(llvm_err)?;
                        self.builder.build_unreachable().map_err(llvm_err)?;
                        Ok(TypedValue::Unit)
                    }
                    _ => Err("panic: argument must be a string".to_string()),
                }
            }
            "assert" => {
                if args.len() != 2 {
                    return Err("assert expects 2 arguments (condition, message)".to_string());
                }
                let cond = self.compile_expr(&args[0])?;
                let cond_bool = match cond {
                    TypedValue::Bool(b) => b,
                    _ => return Err("assert: first argument must be a Bool".to_string()),
                };
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let assert_ok_bb = self.context.append_basic_block(current_fn, "assert_ok");
                let assert_fail_bb = self.context.append_basic_block(current_fn, "assert_fail");
                let assert_merge_bb = self.context.append_basic_block(current_fn, "assert_merge");
                let _ =
                    self.builder
                        .build_conditional_branch(cond_bool, assert_ok_bb, assert_fail_bb);
                // Fail: print message and exit
                self.builder.position_at_end(assert_fail_bb);
                let msg = self.compile_expr(&args[1])?;
                match msg {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let prefix = self.compile_string_literal("Assertion failed: ")?;
                        let prefix_sv = match prefix {
                            TypedValue::Str(pp) => self.load_string(pp)?,
                            _ => return Err("internal error".to_string()),
                        };
                        let cc =
                            self.call_rt("action_string_concat", &[prefix_sv.into(), sv.into()])?;
                        let full = cc.try_as_basic_value().basic().ok_or("concat failed")?;
                        let _ = self.call_rt("action_print_string", &[full.into()])?;
                        let _ = self.call_rt("action_println", &[])?;
                        let exit_fn = self.module.get_function("exit");
                        if exit_fn.is_none() {
                            let _ = self.module.add_function(
                                "exit",
                                self.void_ty().fn_type(&[self.i32_ty().into()], false),
                                None,
                            );
                        }
                        let exit_fn = self.module.get_function("exit").unwrap();
                        let _ = self
                            .builder
                            .build_call(exit_fn, &[self.i32_ty().const_int(1, false).into()], "")
                            .map_err(llvm_err)?;
                        self.builder.build_unreachable().map_err(llvm_err)?;
                    }
                    _ => return Err("assert: second argument must be a string".to_string()),
                }
                // Ok: continue
                self.builder.position_at_end(assert_ok_bb);
                let _ = self.builder.build_unconditional_branch(assert_merge_bb);
                self.builder.position_at_end(assert_merge_bb);
                Ok(TypedValue::Unit)
            }
            "toString" => {
                if args.len() != 1 {
                    return Err("toString expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => {
                        let cc = self.call_rt("action_int_to_string", &[iv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("intToString failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    TypedValue::Float(fv) => {
                        let cc = self.call_rt("action_float_to_string", &[fv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("floatToString failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "fstr")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    TypedValue::Bool(bv) => {
                        let true_lit = self.compile_string_literal("true")?;
                        let false_lit = self.compile_string_literal("false")?;
                        let true_sv = match true_lit {
                            TypedValue::Str(tp) => self.load_string(tp)?,
                            _ => return Err("internal".to_string()),
                        };
                        let false_sv = match false_lit {
                            TypedValue::Str(fp) => self.load_string(fp)?,
                            _ => return Err("internal".to_string()),
                        };
                        let result = self
                            .builder
                            .build_select(bv, true_sv, false_sv, "bool_str")
                            .map_err(llvm_err)?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "bstr")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, result.into_struct_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "idstr")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, sv).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => {
                        let placeholder = self.compile_string_literal("[Object]")?;
                        match placeholder {
                            TypedValue::Str(pp) => {
                                let pv = self.load_string(pp)?;
                                let alloca = self
                                    .builder
                                    .build_alloca(self.string_type, "objstr")
                                    .map_err(llvm_err)?;
                                self.builder.build_store(alloca, pv).map_err(llvm_err)?;
                                Ok(TypedValue::Str(alloca))
                            }
                            _ => Err("internal error".to_string()),
                        }
                    }
                }
            }
            "setFromList" | "fromList" => {
                if args.len() != 1 {
                    return Err(format!("{} expects 1 argument (list)", name));
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(p) => {
                        let lv = self.load_list(p)?;
                        let cc = self.call_rt("action_set_from_list", &[lv.into()])?;
                        let sv = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set_from_list failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "set_from_list")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, sv).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err(format!("{}: argument must be a list", name)),
                }
            }
            "toInt" => {
                if args.len() != 1 {
                    return Err("toInt expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => {
                        self.build_nullable_int(iv, self.bool_ty().const_int(1, false))
                    }
                    TypedValue::Float(fv) => {
                        let i = self
                            .builder
                            .build_float_to_signed_int(fv, self.i64_ty(), "ftoi")
                            .map_err(llvm_err)?;
                        self.build_nullable_int(i, self.bool_ty().const_int(1, false))
                    }
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let cc = self.call_rt("action_parse_int", &[sv.into()])?;
                        let result_struct = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("parseInt failed")?
                            .into_struct_value();
                        let val = self
                            .builder
                            .build_extract_value(result_struct, 0, "val")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let ok = self
                            .builder
                            .build_extract_value(result_struct, 1, "ok")
                            .map_err(llvm_err)?
                            .into_int_value();
                        self.build_nullable_int(val, ok)
                    }
                    _ => Err("toInt: cannot convert to Int".to_string()),
                }
            }
            "toFloat" => {
                if args.len() != 1 {
                    return Err("toFloat expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let always_true = self.bool_ty().const_int(1, false);
                match v {
                    TypedValue::Float(fv) => self.build_nullable_float(fv, always_true),
                    TypedValue::Int(iv) => {
                        let f = self
                            .builder
                            .build_signed_int_to_float(iv, self.f64_ty(), "itof")
                            .map_err(llvm_err)?;
                        self.build_nullable_float(f, always_true)
                    }
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let len = self
                            .builder
                            .build_extract_value(sv, 0, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let has_chars = self
                            .builder
                            .build_int_compare(
                                IntPredicate::UGT,
                                len,
                                self.i64_ty().const_int(0, false),
                                "has_chars",
                            )
                            .map_err(llvm_err)?;
                        // Check first char is digit, '-', '+', or '.'
                        let data_ptr = self
                            .builder
                            .build_extract_value(sv, 1, "dptr")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let first_char = self
                            .builder
                            .build_load(self.context.i8_type(), data_ptr, "first_char")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let is_digit = self
                            .builder
                            .build_int_compare(
                                IntPredicate::UGE,
                                first_char,
                                self.context.i8_type().const_int(b'0' as u64, false),
                                "isd",
                            )
                            .map_err(llvm_err)?;
                        let le9 = self
                            .builder
                            .build_int_compare(
                                IntPredicate::ULE,
                                first_char,
                                self.context.i8_type().const_int(b'9' as u64, false),
                                "le9",
                            )
                            .map_err(llvm_err)?;
                        let is_d = self
                            .builder
                            .build_and(is_digit, le9, "is_digit")
                            .map_err(llvm_err)?;
                        let is_minus = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                first_char,
                                self.context.i8_type().const_int(b'-' as u64, false),
                                "is_minus",
                            )
                            .map_err(llvm_err)?;
                        let is_plus = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                first_char,
                                self.context.i8_type().const_int(b'+' as u64, false),
                                "is_plus",
                            )
                            .map_err(llvm_err)?;
                        let is_dot = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                first_char,
                                self.context.i8_type().const_int(b'.' as u64, false),
                                "is_dot",
                            )
                            .map_err(llvm_err)?;
                        let is_sign = self
                            .builder
                            .build_or(is_minus, is_plus, "is_sign")
                            .map_err(llvm_err)?;
                        let is_num_start = self
                            .builder
                            .build_or(is_d, is_sign, "is_num1")
                            .map_err(llvm_err)?;
                        let is_valid = self
                            .builder
                            .build_or(is_num_start, is_dot, "is_valid")
                            .map_err(llvm_err)?;
                        let ok = self
                            .builder
                            .build_and(has_chars, is_valid, "ok")
                            .map_err(llvm_err)?;
                        let strtod_fn = self.module.get_function("strtod").unwrap();
                        let null_ptr = self.ptr_ty().const_zero();
                        let result = self
                            .builder
                            .build_call(strtod_fn, &[data_ptr.into(), null_ptr.into()], "fval")
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("strtod failed")?
                            .into_float_value();
                        self.build_nullable_float(result, ok)
                    }
                    _ => Err("toFloat: cannot convert to Float".to_string()),
                }
            }
            "identity" => {
                if args.len() != 1 {
                    return Err("identity expects 1 argument".to_string());
                }
                self.compile_expr(&args[0])
            }
            "compose" => {
                if args.len() != 3 {
                    return Err("compose expects 3 arguments (f, g, x)".to_string());
                }
                // compose(f, g, x) = f(g(x))
                let inner = Expr::Call {
                    func: Box::new(args[1].clone()),
                    args: vec![args[2].clone()],
                    trailing_lambda: None,
                };
                let outer = Expr::Call {
                    func: Box::new(args[0].clone()),
                    args: vec![inner],
                    trailing_lambda: None,
                };
                self.compile_expr(&outer)
            }
            "flip" => {
                if args.len() != 3 {
                    return Err("flip expects 3 arguments (f, a, b)".to_string());
                }
                // flip(f, a, b) = f(b, a)
                let call = Expr::Call {
                    func: Box::new(args[0].clone()),
                    args: vec![args[2].clone(), args[1].clone()],
                    trailing_lambda: None,
                };
                self.compile_expr(&call)
            }
            "constant" => {
                if args.len() != 2 {
                    return Err("constant expects 2 arguments (a, b)".to_string());
                }
                // constant(a, b) = a (returns first argument, ignores second)
                self.compile_expr(&args[0])
            }
            "uncurry" => {
                if args.len() != 3 {
                    return Err("uncurry expects 3 arguments (f, a, b)".to_string());
                }
                // uncurry(f, a, b) = f(a)(b)
                let inner = Expr::Call {
                    func: Box::new(args[0].clone()),
                    args: vec![args[1].clone()],
                    trailing_lambda: None,
                };
                let outer = Expr::Call {
                    func: Box::new(inner),
                    args: vec![args[2].clone()],
                    trailing_lambda: None,
                };
                self.compile_expr(&outer)
            }
            "curry" => {
                if args.len() != 2 {
                    return Err("curry expects 2 arguments (f, a)".to_string());
                }
                // curry(f, a) → creates a lambda |b| f(a, b)
                // We implement this by compiling the partial application as a lambda expression
                let lambda = Expr::Lambda {
                    params: vec!["b".to_string()],
                    body: Box::new(Expr::Call {
                        func: Box::new(args[0].clone()),
                        args: vec![args[1].clone(), Expr::Ident("b".to_string())],
                        trailing_lambda: None,
                    }),
                    implicit_it: false,
                };
                self.compile_expr(&lambda)
            }
            // ---- LazyList operations ----
            "toList" => {
                if args.len() != 1 {
                    return Err("toList expects 1 argument (lazy_list or set)".to_string());
                }
                self.builtin_to_list(&args[0])
            }
            "toLazyList" => {
                if args.len() != 1 {
                    return Err("toLazyList expects 1 argument (list)".to_string());
                }
                self.builtin_to_lazy_list(&args[0])
            }
            "lazyTake" => {
                if args.len() != 2 {
                    return Err("lazyTake expects 2 arguments (n, lazy_list)".to_string());
                }
                self.builtin_lazy_take(&args[0], &args[1])
            }
            "lazyDrop" => {
                if args.len() != 2 {
                    return Err("lazyDrop expects 2 arguments (n, lazy_list)".to_string());
                }
                self.builtin_lazy_drop(&args[0], &args[1])
            }
            "lazyMap" => {
                if args.len() != 2 {
                    return Err("lazyMap expects 2 arguments (fn, lazy_list)".to_string());
                }
                self.builtin_lazy_map(&args[0], &args[1])
            }
            "lazyFilter" => {
                if args.len() != 2 {
                    return Err("lazyFilter expects 2 arguments (fn, lazy_list)".to_string());
                }
                self.builtin_lazy_filter(&args[0], &args[1])
            }
            "lazyTakeWhile" => {
                if args.len() != 2 {
                    return Err("lazyTakeWhile expects 2 arguments (fn, lazy_list)".to_string());
                }
                self.builtin_lazy_take_while(&args[0], &args[1])
            }
            "lazyHead" => {
                if args.len() != 1 {
                    return Err("lazyHead expects 1 argument (lazy_list)".to_string());
                }
                self.builtin_lazy_head(&args[0])
            }
            "lazyZip" => {
                if args.len() != 2 {
                    return Err("lazy.zip expects 2 arguments (lazy1, lazy2)".to_string());
                }
                self.builtin_lazy_zip(&args[0], &args[1])
            }
            "toCString" => {
                if args.len() != 1 {
                    return Err("toCString expects 1 argument".to_string());
                }
                self.builtin_to_cstring(&args[0])
            }
            "fromCString" => {
                if args.len() != 1 {
                    return Err("fromCString expects 1 argument".to_string());
                }
                self.builtin_from_cstring(&args[0])
            }
            "isNull" => {
                if args.len() != 1 {
                    return Err("isNull expects 1 argument".to_string());
                }
                self.builtin_is_null(&args[0])
            }
            "deref" => {
                if args.len() != 1 {
                    return Err("deref expects 1 argument".to_string());
                }
                self.builtin_deref(&args[0])
            }
            "ping" => {
                let result = self.call_rt("action_test_ping", &[])?;
                let val = result
                    .try_as_basic_value()
                    .basic()
                    .ok_or("ping call failed")?
                    .into_int_value();
                Ok(TypedValue::Int(val))
            }
            "httpRequest" => {
                if args.len() != 4 {
                    return Err(
                        "httpRequest expects 4 arguments (method, url, headers, body)".to_string(),
                    );
                }
                self.builtin_http_request(&args[0], &args[1], &args[2], &args[3])
            }
            "today" | "now" | "year" | "month" | "day" | "hour" | "minute" | "second"
            | "addDays" | "addHours" | "diffDays" | "weekday" | "nowUtc" | "diffSeconds"
            | "date" | "datetime" | "format" | "parseDate" | "Random_new" | "nextInt"
            | "randInt" | "randFloat" => self.builtin_stdlib_datetime(name, args),
            _ => Err(format!("Unknown builtin: {}", name)),
        }
    }
}
