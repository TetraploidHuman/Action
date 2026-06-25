// Submodule: builtins_stdlib_string — string builtin functions
//
// Extracted from builtins_stdlib.rs.
//
// Submodule: builtins_stdlib

use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_stdlib_string(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "toUpper" => {
                if args.len() != 1 {
                    return Err("toUpper expects 1 argument".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_to_upper", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("toUpper failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "upper")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("toUpper: argument must be a string".to_string()),
                }
            }
            "toLower" => {
                if args.len() != 1 {
                    return Err("toLower expects 1 argument".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_to_lower", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("toLower failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "lower")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("toLower: argument must be a string".to_string()),
                }
            }
            "trim" => {
                if args.len() != 1 {
                    return Err("trim expects 1 argument".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_trim", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("trim failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "trimmed")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("trim: argument must be a string".to_string()),
                }
            }
            "startsWith" => {
                if args.len() != 2 {
                    return Err("startsWith expects 2 arguments".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                let prefix = self.compile_call_arg(args[1])?;
                match (&s, &prefix) {
                    (TypedValue::Str(sp), TypedValue::Str(pp)) => {
                        let sv = self.load_string(*sp)?;
                        let pv = self.load_string(*pp)?;
                        let cc =
                            self.call_rt("action_string_starts_with", &[sv.into(), pv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("startsWith failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("startsWith: arguments must be strings".to_string()),
                }
            }
            "endsWith" => {
                if args.len() != 2 {
                    return Err("endsWith expects 2 arguments".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                let suffix = self.compile_call_arg(args[1])?;
                match (&s, &suffix) {
                    (TypedValue::Str(sp), TypedValue::Str(sup)) => {
                        let sv = self.load_string(*sp)?;
                        let suv = self.load_string(*sup)?;
                        let cc =
                            self.call_rt("action_string_ends_with", &[sv.into(), suv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("endsWith failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("endsWith: arguments must be strings".to_string()),
                }
            }
            "substring" => {
                if args.len() != 3 {
                    return Err("substring expects 3 arguments (str, start, len)".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                let start = self.compile_call_arg(args[1])?;
                let len = self.compile_call_arg(args[2])?;
                match s {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let start_bv = start.to_bv().ok_or("start must be a basic value")?;
                        let len_bv = len.to_bv().ok_or("len must be a basic value")?;
                        let cc = self.call_rt(
                            "action_string_substring",
                            &[sv.into(), start_bv.into(), len_bv.into()],
                        )?;
                        let result = cc.try_as_basic_value().basic().ok_or("substring failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "substr")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("substring: first argument must be a string".to_string()),
                }
            }
            "parseInt" => {
                if args.len() != 1 {
                    return Err("parseInt expects 1 argument".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                match s {
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
                    _ => Err("parseInt: argument must be a string".to_string()),
                }
            }
            "split" => {
                if args.len() != 2 {
                    return Err("split expects 2 arguments (string, delimiter)".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                let delim = self.compile_call_arg(args[1])?;
                match (&s, &delim) {
                    (TypedValue::Str(sp), TypedValue::Str(dp)) => {
                        let sv = self.load_string(*sp)?;
                        let dv = self.load_string(*dp)?;
                        let cc = self.call_rt("action_string_split", &[sv.into(), dv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("split failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "split_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("split: arguments must be strings".to_string()),
                }
            }
            "join" => {
                if args.len() != 2 {
                    return Err("join expects 2 arguments (list, delimiter)".to_string());
                }
                let list_val = self.compile_call_arg(args[0])?;
                let delim = self.compile_call_arg(args[1])?;
                match (&list_val, &delim) {
                    (TypedValue::List(lp), TypedValue::Str(dp)) => {
                        let lv = self.load_list(*lp)?;
                        let dv = self.load_string(*dp)?;
                        let cc = self.call_rt("action_string_join", &[lv.into(), dv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("join failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "join_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("join: first argument must be a list, second a string".to_string()),
                }
            }
            "replace" => {
                if args.len() != 3 {
                    return Err("replace expects 3 arguments (string, from, to)".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                let from = self.compile_call_arg(args[1])?;
                let to = self.compile_call_arg(args[2])?;
                match (&s, &from, &to) {
                    (TypedValue::Str(sp), TypedValue::Str(fp), TypedValue::Str(tp)) => {
                        let sv = self.load_string(*sp)?;
                        let fv = self.load_string(*fp)?;
                        let tv = self.load_string(*tp)?;
                        let cc = self
                            .call_rt("action_string_replace", &[sv.into(), fv.into(), tv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("replace failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "replace_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("replace: arguments must be strings".to_string()),
                }
            }
            "trimStart" => {
                if args.len() != 1 {
                    return Err("trimStart expects 1 argument".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_trim_start", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("trimStart failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "trimmed_start")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("trimStart: argument must be a string".to_string()),
                }
            }
            "trimEnd" => {
                if args.len() != 1 {
                    return Err("trimEnd expects 1 argument".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_trim_end", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("trimEnd failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "trimmed_end")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("trimEnd: argument must be a string".to_string()),
                }
            }
            "stringContains" => {
                if args.len() != 2 {
                    return Err("stringContains expects 2 arguments (str, substr)".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                let sub = self.compile_call_arg(args[1])?;
                match (&s, &sub) {
                    (TypedValue::Str(sp), TypedValue::Str(subp)) => {
                        let sv = self.load_string(*sp)?;
                        let subv = self.load_string(*subp)?;
                        let cc =
                            self.call_rt("action_string_contains", &[sv.into(), subv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("stringContains failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("stringContains: arguments must be strings".to_string()),
                }
            }
            "stringRepeat" => {
                if args.len() != 2 {
                    return Err("stringRepeat expects 2 arguments (str, count)".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                let count = self.compile_call_arg(args[1])?;
                match (s, count) {
                    (TypedValue::Str(sp), TypedValue::Int(cv)) => {
                        let sv = self.load_string(sp)?;
                        let cc = self.call_rt("action_string_repeat", &[sv.into(), cv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("stringRepeat failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "str_repeat")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err(
                        "stringRepeat: first argument must be a string, second an Int".to_string(),
                    ),
                }
            }
            "splitLines" => {
                if args.len() != 1 {
                    return Err("splitLines expects 1 argument (string)".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                match s {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let cc = self.call_rt("action_string_split_lines", &[sv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("splitLines failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "lines")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("splitLines: argument must be a string".to_string()),
                }
            }
            "indexOf" => {
                if args.len() != 2 {
                    return Err("indexOf expects 2 arguments".to_string());
                }
                let v1 = self.compile_call_arg(args[0])?;
                let v2 = self.compile_call_arg(args[1])?;
                match (&v1, &v2) {
                    // indexOf(element, list) -> Option<Int>
                    (elem, TypedValue::List(lp)) => {
                        let lv = self.load_list(*lp)?;
                        let fat = self.to_fat_struct(elem)?;
                        let cc = self.call_rt("action_list_index_of", &[lv.into(), fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("indexOf failed")?
                            .into_int_value();
                        let found = self
                            .builder
                            .build_int_compare(
                                IntPredicate::SGE,
                                result,
                                self.i64_ty().const_int(0, false),
                                "found",
                            )
                            .map_err(llvm_err)?;
                        self.build_nullable_int(result, found)
                    }
                    // indexOf(substring, string) -> Option<Int>
                    (TypedValue::Str(sp1), TypedValue::Str(sp2)) => {
                        let sv1 = self.load_string(*sp1)?;
                        let sv2 = self.load_string(*sp2)?;
                        // runtime expects (haystack, needle), so swap: sv2 is haystack, sv1 is needle
                        let cc =
                            self.call_rt("action_string_index_of", &[sv2.into(), sv1.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("indexOf failed")?
                            .into_int_value();
                        let neg_one = self.i64_ty().const_int((-1i64) as u64, true);
                        let found = self
                            .builder
                            .build_int_compare(IntPredicate::NE, result, neg_one, "found")
                            .map_err(llvm_err)?;
                        self.build_nullable_int(result, found)
                    }
                    _ => Err(
                        "indexOf: first arg must be (element, list) or (substring, string)"
                            .to_string(),
                    ),
                }
            }
            "chars" => {
                if args.len() != 1 {
                    return Err("chars expects 1 argument (string)".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                match s {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let cc = self.call_rt("action_string_chars", &[sv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("chars failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "chars")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("chars: argument must be a string".to_string()),
                }
            }
            "toChar" => {
                if args.len() != 1 {
                    return Err("toChar expects 1 argument (int)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Int(iv) => {
                        // Validate: code point must be in valid Unicode range
                        let max_cp = self.i64_ty().const_int(0x10FFFF, false);
                        let in_range = self
                            .builder
                            .build_int_compare(IntPredicate::ULE, iv, max_cp, "valid_cp")
                            .map_err(llvm_err)?;
                        let valid = self.build_nullable_int(iv, in_range);
                        valid
                    }
                    _ => Err("toChar: argument must be an Int".to_string()),
                }
            }
            "charCode" => {
                if args.len() != 1 {
                    return Err("charCode expects 1 argument (char)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Int(iv) => Ok(TypedValue::Int(iv)),
                    _ => Err("charCode: argument must be a Char".to_string()),
                }
            }
            "charAt" => {
                if args.len() != 2 {
                    return Err("charAt expects 2 arguments (string, index)".to_string());
                }
                let s = self.compile_call_arg(args[0])?;
                let idx = self.compile_call_arg(args[1])?;
                let s_ptr = match s {
                    TypedValue::Str(p) => p,
                    _ => return Err("charAt: first argument must be a string".to_string()),
                };
                let idx_val = match idx {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("charAt: second argument must be an int".to_string()),
                };
                let ss = self.load_string(s_ptr)?;
                let slen = self
                    .builder
                    .build_extract_value(ss, 0, "slen")
                    .map_err(llvm_err)?
                    .into_int_value();
                let sdata = self
                    .builder
                    .build_extract_value(ss, 1, "sdata")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                // Clamp negative index
                let zero = self.i64_ty().const_int(0, false);
                let neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx_val, zero, "neg")
                    .map_err(llvm_err)?;
                let adj_idx = self
                    .builder
                    .build_int_add(slen, idx_val, "adj")
                    .map_err(llvm_err)?;
                let real_idx = self
                    .builder
                    .build_select(neg, adj_idx, idx_val, "real_idx")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Read leading byte and determine UTF-8 byte count
                let gep = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), sdata, &[real_idx], "gep")
                        .map_err(llvm_err)?
                };
                let ch = self
                    .builder
                    .build_load(self.context.i8_type(), gep, "ch")
                    .map_err(llvm_err)?
                    .into_int_value();
                let nbytes = self
                    .call_rt("action_utf8_byte_len", &[ch.into()])?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();
                // Allocate nbytes+1 (for null terminator) with RC header so param
                // cleanup and scope rc_dec_string_val can release the buffer.
                let alloc_sz = self
                    .builder
                    .build_int_add(nbytes, self.i64_ty().const_int(1, false), "alloc_sz")
                    .map_err(llvm_err)?;
                let buf = self.malloc_rc(alloc_sz)?;
                // memcpy nbytes from sdata+real_idx to buf
                let memcpy_fn = self.module.get_function("memcpy").unwrap();
                let src = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), sdata, &[real_idx], "src")
                        .map_err(llvm_err)
                }?;
                let _ = self
                    .builder
                    .build_call(memcpy_fn, &[buf.into(), src.into(), nbytes.into()], "")
                    .map_err(llvm_err)?;
                // Null terminate
                let null_pos = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buf, &[nbytes], "null_pos")
                        .map_err(llvm_err)
                }?;
                self.builder
                    .build_store(null_pos, self.context.i8_type().const_int(0, false))
                    .map_err(llvm_err)?;
                // Build string struct
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, nbytes, 0, "r1")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, buf, 1, "r2")
                    .map_err(llvm_err)?;
                let sa = self
                    .builder
                    .build_alloca(self.string_type, "char_s")
                    .map_err(llvm_err)?;
                self.builder.build_store(sa, r2).map_err(llvm_err)?;
                Ok(TypedValue::Str(sa))
            }
            "isAlpha" => {
                if args.len() != 1 {
                    return Err("isAlpha expects 1 argument (char)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                let ch = match v {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("isAlpha: argument must be a char code (int)".to_string()),
                };
                let a_lower = self.i64_ty().const_int('a' as u64, false);
                let z_lower = self.i64_ty().const_int('z' as u64, false);
                let a_upper = self.i64_ty().const_int('A' as u64, false);
                let z_upper = self.i64_ty().const_int('Z' as u64, false);
                let is_lower = self
                    .builder
                    .build_and(
                        self.builder
                            .build_int_compare(IntPredicate::SGE, ch, a_lower, "ge_a")
                            .map_err(llvm_err)?,
                        self.builder
                            .build_int_compare(IntPredicate::SLE, ch, z_lower, "le_z")
                            .map_err(llvm_err)?,
                        "is_lower",
                    )
                    .map_err(llvm_err)?;
                let is_upper = self
                    .builder
                    .build_and(
                        self.builder
                            .build_int_compare(IntPredicate::SGE, ch, a_upper, "ge_A")
                            .map_err(llvm_err)?,
                        self.builder
                            .build_int_compare(IntPredicate::SLE, ch, z_upper, "le_Z")
                            .map_err(llvm_err)?,
                        "is_upper",
                    )
                    .map_err(llvm_err)?;
                let result = self
                    .builder
                    .build_or(is_lower, is_upper, "isAlpha")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(result))
            }
            "codeToChar" => {
                if args.len() != 1 {
                    return Err("codeToChar expects 1 argument (int)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
                let code = match v {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("codeToChar: argument must be an int".to_string()),
                };
                let i64 = self.i64_ty();
                let i8 = self.context.i8_type();
                // Allocate 5 bytes (max 4 byte UTF-8 + null terminator) with RC header.
                let alloc_sz = i64.const_int(5, false);
                let buf = self.malloc_rc(alloc_sz)?;
                // Call runtime UTF-8 encoder: nbytes = action_utf8_encode(code, buf)
                let nbytes = self
                    .call_rt("action_utf8_encode", &[code.into(), buf.into()])?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();
                // Null terminate at position nbytes
                let null_g = unsafe {
                    self.builder
                        .build_gep(i8, buf, &[nbytes], "null_g")
                        .map_err(llvm_err)
                }?;
                self.builder
                    .build_store(null_g, i8.const_int(0, false))
                    .map_err(llvm_err)?;
                // Build string struct: { len: i64, data: i8* }
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, nbytes, 0, "slen")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, buf, 1, "sdata")
                    .map_err(llvm_err)?;
                let sa = self
                    .builder
                    .build_alloca(self.string_type, "code_s")
                    .map_err(llvm_err)?;
                self.builder.build_store(sa, r2).map_err(llvm_err)?;
                Ok(TypedValue::Str(sa))
            }
            _ => Err(format!("Unknown string builtin: {}", name)),
        }
    }
}
