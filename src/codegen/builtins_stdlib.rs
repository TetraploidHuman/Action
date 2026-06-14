// Submodule: builtins_stdlib

use crate::ast::*;
use inkwell::values::{BasicValue, IntValue, PointerValue};
use inkwell::{FloatPredicate, IntPredicate};

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
                        let raw_len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let two = self.i64_ty().const_int(2, false);
                        let len = self
                            .builder
                            .build_int_signed_div(raw_len, two, "entries")
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Int(len))
                    }
                    TypedValue::Set(ptr) => {
                        let m = self.load_list(ptr)?;
                        let raw_len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let two = self.i64_ty().const_int(2, false);
                        let len = self
                            .builder
                            .build_int_signed_div(raw_len, two, "entries")
                            .map_err(llvm_err)?;
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
                        let raw_len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let two = self.i64_ty().const_int(2, false);
                        self.builder
                            .build_int_signed_div(raw_len, two, "entries")
                            .map_err(llvm_err)?
                    }
                    TypedValue::Set(ptr) => {
                        let m = self.load_list(ptr)?;
                        let raw_len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let two = self.i64_ty().const_int(2, false);
                        self.builder
                            .build_int_signed_div(raw_len, two, "entries")
                            .map_err(llvm_err)?
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
            "toUpper" => {
                if args.len() != 1 {
                    return Err("toUpper expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
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
                let v = self.compile_expr(&args[0])?;
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
                let v = self.compile_expr(&args[0])?;
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
            "readLine" => {
                if !args.is_empty() {
                    return Err("readLine expects no arguments".to_string());
                }
                if self.module.get_function("action_read_line").is_none() {
                    self.emit_read_line_runtime()?;
                }
                let cc = self.call_rt("action_read_line", &[])?;
                let result_struct = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("readLine failed")?
                    .into_struct_value();
                // Extract string {i64, ptr} and success flag i1
                let str_len = self
                    .builder
                    .build_extract_value(result_struct, 0, "slen")
                    .map_err(llvm_err)?
                    .into_int_value();
                let str_ptr = self
                    .builder
                    .build_extract_value(result_struct, 1, "sptr")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let ok = self
                    .builder
                    .build_extract_value(result_struct, 2, "ok")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Build the string fat struct and store in alloca
                let line_undef = self.string_type.get_undef();
                let line1 = self
                    .builder
                    .build_insert_value(line_undef, str_len, 0, "l_len")
                    .map_err(llvm_err)?;
                let line_val = self
                    .builder
                    .build_insert_value(line1, str_ptr, 1, "l_ptr")
                    .map_err(llvm_err)?;
                let fat_alloca = self
                    .builder
                    .build_alloca(self.string_type, "line")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(fat_alloca, line_val)
                    .map_err(llvm_err)?;
                let flag_alloca = self
                    .builder
                    .build_alloca(self.bool_ty(), "line_ok")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(flag_alloca, ok)
                    .map_err(llvm_err)?;
                self.build_nullable_str(fat_alloca, flag_alloca)
            }
            "startsWith" => {
                if args.len() != 2 {
                    return Err("startsWith expects 2 arguments".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let prefix = self.compile_expr(&args[1])?;
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
                let s = self.compile_expr(&args[0])?;
                let suffix = self.compile_expr(&args[1])?;
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
                let s = self.compile_expr(&args[0])?;
                let start = self.compile_expr(&args[1])?;
                let len = self.compile_expr(&args[2])?;
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
                let s = self.compile_expr(&args[0])?;
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
            "readFile" => {
                if args.len() != 1 {
                    return Err("readFile expects 1 argument (path)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                match path {
                    TypedValue::Str(pp) => {
                        let pv = self.load_string(pp)?;
                        let cc = self.call_rt("action_read_file", &[pv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("readFile failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "content")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("readFile: argument must be a string".to_string()),
                }
            }
            "writeFile" => {
                if args.len() != 2 {
                    return Err("writeFile expects 2 arguments (path, content)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                let content = self.compile_expr(&args[1])?;
                match (&path, &content) {
                    (TypedValue::Str(pp), TypedValue::Str(cp)) => {
                        let pv = self.load_string(*pp)?;
                        let cv = self.load_string(*cp)?;
                        let cc = self.call_rt("action_write_file", &[pv.into(), cv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("writeFile failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("writeFile: arguments must be strings".to_string()),
                }
            }
            "appendFile" => {
                if args.len() != 2 {
                    return Err("appendFile expects 2 arguments (path, content)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                let content = self.compile_expr(&args[1])?;
                match (&path, &content) {
                    (TypedValue::Str(pp), TypedValue::Str(cp)) => {
                        let pv = self.load_string(*pp)?;
                        let cv = self.load_string(*cp)?;
                        let cc = self.call_rt("action_file_append", &[pv.into(), cv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("appendFile failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("appendFile: arguments must be strings".to_string()),
                }
            }
            "exists" => {
                if args.len() != 1 {
                    return Err("exists expects 1 argument (path)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                match path {
                    TypedValue::Str(pp) => {
                        let pv = self.load_string(pp)?;
                        let cc = self.call_rt("action_file_exists", &[pv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("exists failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("exists: argument must be a string".to_string()),
                }
            }
            "deleteFile" => {
                if args.len() != 1 {
                    return Err("deleteFile expects 1 argument (path)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                match path {
                    TypedValue::Str(pp) => {
                        let pv = self.load_string(pp)?;
                        let cc = self.call_rt("action_file_delete", &[pv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("deleteFile failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("deleteFile: argument must be a string".to_string()),
                }
            }
            // ---- Streaming File I/O ----
            "openFile" => {
                if args.len() != 2 {
                    return Err("openFile expects 2 arguments (path, mode)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                let mode = self.compile_expr(&args[1])?;
                match (&path, &mode) {
                    (TypedValue::Str(pp), TypedValue::Str(mp)) => {
                        let path_s = self.load_string(*pp)?;
                        let mode_s = self.load_string(*mp)?;
                        let cc =
                            self.call_rt("action_file_open", &[path_s.into(), mode_s.into()])?;
                        let file_ptr = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("openFile failed")?
                            .into_pointer_value();
                        Ok(TypedValue::FileHandle(file_ptr))
                    }
                    _ => Err("openFile: arguments must be strings (path, mode)".to_string()),
                }
            }
            "closeFile" => {
                if args.len() != 1 {
                    return Err("closeFile expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_close", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("closeFile failed")?
                            .into_int_value();
                        let ok = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                result,
                                self.i32_ty().const_int(0, false),
                                "ok",
                            )
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Bool(ok))
                    }
                    _ => Err("closeFile: argument must be a FileHandle".to_string()),
                }
            }
            "isEof" => {
                if args.len() != 1 {
                    return Err("isEof expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_eof", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("isEof failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("isEof: argument must be a FileHandle".to_string()),
                }
            }
            "fileReadLine" => {
                if args.len() != 1 {
                    return Err("fileReadLine expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_read_line", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileReadLine failed")?
                            .into_struct_value();
                        // Build string from len+ptr
                        let len = self
                            .builder
                            .build_extract_value(result, 0, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let data = self
                            .builder
                            .build_extract_value(result, 1, "data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let str_struct =
                            self.call_rt("action_string_create", &[data.into(), len.into()])?;
                        let str_val = str_struct
                            .try_as_basic_value()
                            .basic()
                            .ok_or("string_create failed")?;
                        let str_alloca = self
                            .builder
                            .build_alloca(self.string_type, "str_tmp")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(str_alloca, str_val)
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Str(str_alloca))
                    }
                    _ => Err("fileReadLine: argument must be a FileHandle".to_string()),
                }
            }
            "fileReadBytes" => {
                if args.len() != 2 {
                    return Err("fileReadBytes expects 2 arguments (file, size)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                let size = self.compile_expr(&args[1])?;
                match (&file, &size) {
                    (TypedValue::FileHandle(p), TypedValue::Int(s)) => {
                        let cc =
                            self.call_rt("action_file_read_bytes", &[(*p).into(), (*s).into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileReadBytes failed")?
                            .into_struct_value();
                        let len = self
                            .builder
                            .build_extract_value(result, 0, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let data = self
                            .builder
                            .build_extract_value(result, 1, "data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let str_struct =
                            self.call_rt("action_string_create", &[data.into(), len.into()])?;
                        let str_val = str_struct
                            .try_as_basic_value()
                            .basic()
                            .ok_or("string_create failed")?;
                        let str_alloca = self
                            .builder
                            .build_alloca(self.string_type, "rb_tmp")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(str_alloca, str_val)
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Str(str_alloca))
                    }
                    _ => Err("fileReadBytes: arguments must be (FileHandle, Int)".to_string()),
                }
            }
            "fileWrite" => {
                if args.len() != 2 {
                    return Err("fileWrite expects 2 arguments (file, data)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                let data = self.compile_expr(&args[1])?;
                match (&file, &data) {
                    (TypedValue::FileHandle(fp), TypedValue::Str(dp)) => {
                        let data_s = self.load_string(*dp)?;
                        let data_len = self
                            .builder
                            .build_extract_value(data_s, 0, "dlen")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let data_ptr = self
                            .builder
                            .build_extract_value(data_s, 1, "dptr")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let cc = self.call_rt(
                            "action_file_write_bytes",
                            &[(*fp).into(), data_ptr.into(), data_len.into()],
                        )?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileWrite failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("fileWrite: arguments must be (FileHandle, String)".to_string()),
                }
            }
            "fileWriteLine" => {
                if args.len() != 2 {
                    return Err("fileWriteLine expects 2 arguments (file, data)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                let data = self.compile_expr(&args[1])?;
                match (&file, &data) {
                    (TypedValue::FileHandle(fp), TypedValue::Str(dp)) => {
                        let data_s = self.load_string(*dp)?;
                        let data_len = self
                            .builder
                            .build_extract_value(data_s, 0, "dlen")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let data_ptr = self
                            .builder
                            .build_extract_value(data_s, 1, "dptr")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        // Write data first
                        let cc1 = self.call_rt(
                            "action_file_write_bytes",
                            &[(*fp).into(), data_ptr.into(), data_len.into()],
                        )?;
                        // Write newline: create a buffer with "\n\0"
                        let malloc_fn = self.module.get_function("malloc").unwrap();
                        let nl_len = self.i64_ty().const_int(1, false);
                        let nl_buf = self
                            .builder
                            .build_call(
                                malloc_fn,
                                &[self.i64_ty().const_int(2, false).into()],
                                "nl_buf",
                            )
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_pointer_value();
                        self.builder
                            .build_store(nl_buf, self.context.i8_type().const_int(10, false))
                            .map_err(llvm_err)?;
                        let _ = self.call_rt(
                            "action_file_write_bytes",
                            &[(*fp).into(), nl_buf.into(), nl_len.into()],
                        )?;
                        let result = cc1
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileWriteLine failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("fileWriteLine: arguments must be (FileHandle, String)".to_string()),
                }
            }
            "fileFlush" => {
                if args.len() != 1 {
                    return Err("fileFlush expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_flush", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileFlush failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("fileFlush: argument must be a FileHandle".to_string()),
                }
            }
            "fileSeek" => {
                if args.len() != 3 {
                    return Err("fileSeek expects 3 arguments (file, offset, whence)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                let offset = self.compile_expr(&args[1])?;
                let whence = self.compile_expr(&args[2])?;
                match (&file, &offset, &whence) {
                    (TypedValue::FileHandle(p), TypedValue::Int(o), TypedValue::Int(w)) => {
                        let w32 = self
                            .builder
                            .build_int_truncate(*w, self.i32_ty(), "w32")
                            .map_err(llvm_err)?;
                        let cc = self
                            .call_rt("action_file_seek", &[(*p).into(), (*o).into(), w32.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileSeek failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("fileSeek: arguments must be (FileHandle, Int, Int)".to_string()),
                }
            }
            "fileTell" => {
                if args.len() != 1 {
                    return Err("fileTell expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_tell", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileTell failed")?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    _ => Err("fileTell: argument must be a FileHandle".to_string()),
                }
            }
            "randInt" => {
                if args.len() != 2 {
                    return Err("randInt expects 2 arguments (min, max)".to_string());
                }
                let min = self.compile_expr(&args[0])?;
                let max = self.compile_expr(&args[1])?;
                let min_bv = min.to_bv().ok_or("min must be a basic value")?;
                let max_bv = max.to_bv().ok_or("max must be a basic value")?;
                let cc = self.call_rt("action_rand_int", &[min_bv.into(), max_bv.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("randInt failed")?
                    .into_int_value();
                Ok(TypedValue::Int(result))
            }
            "randFloat" => {
                if !args.is_empty() {
                    return Err("randFloat expects no arguments".to_string());
                }
                let cc = self.call_rt("action_rand_float", &[])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("randFloat failed")?
                    .into_float_value();
                Ok(TypedValue::Float(result))
            }
            "split" => {
                if args.len() != 2 {
                    return Err("split expects 2 arguments (string, delimiter)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let delim = self.compile_expr(&args[1])?;
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
                let list_val = self.compile_expr(&args[0])?;
                let delim = self.compile_expr(&args[1])?;
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
                let s = self.compile_expr(&args[0])?;
                let from = self.compile_expr(&args[1])?;
                let to = self.compile_expr(&args[2])?;
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
            "abs" => {
                if args.len() != 1 {
                    return Err("abs expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => {
                        let zero = self.i64_ty().const_int(0, false);
                        let neg = self.builder.build_int_neg(iv, "neg").map_err(llvm_err)?;
                        let is_neg = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, iv, zero, "is_neg")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_neg, neg, iv, "abs_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    TypedValue::Float(fv) => {
                        let zero = self.f64_ty().const_float(0.0);
                        let neg = self.builder.build_float_neg(fv, "neg").map_err(llvm_err)?;
                        let is_neg = self
                            .builder
                            .build_float_compare(FloatPredicate::OLT, fv, zero, "is_neg")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_neg, neg, fv, "fabs_result")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("abs: argument must be Int or Float".to_string()),
                }
            }
            "min" => {
                if args.len() != 2 {
                    return Err("min expects 2 arguments".to_string());
                }
                let a = self.compile_expr(&args[0])?;
                let b = self.compile_expr(&args[1])?;
                match (&a, &b) {
                    (TypedValue::Int(av), TypedValue::Int(bv)) => {
                        let is_lt = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, *av, *bv, "is_lt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_lt, *av, *bv, "min_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    (TypedValue::Float(av), TypedValue::Float(bv)) => {
                        let is_lt = self
                            .builder
                            .build_float_compare(FloatPredicate::OLT, *av, *bv, "is_lt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_lt, *av, *bv, "fmin_result")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("min: arguments must be both Int or both Float".to_string()),
                }
            }
            "max" => {
                if args.len() != 2 {
                    return Err("max expects 2 arguments".to_string());
                }
                let a = self.compile_expr(&args[0])?;
                let b = self.compile_expr(&args[1])?;
                match (&a, &b) {
                    (TypedValue::Int(av), TypedValue::Int(bv)) => {
                        let is_gt = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, *av, *bv, "is_gt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_gt, *av, *bv, "max_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    (TypedValue::Float(av), TypedValue::Float(bv)) => {
                        let is_gt = self
                            .builder
                            .build_float_compare(FloatPredicate::OGT, *av, *bv, "is_gt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_gt, *av, *bv, "fmax_result")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("max: arguments must be both Int or both Float".to_string()),
                }
            }
            "sqrt" => {
                if args.len() != 1 {
                    return Err("sqrt expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let sqrt_fn = self.module.get_function("sqrt").unwrap();
                let r = self
                    .builder
                    .build_call(sqrt_fn, &[fv.into()], "sqrt")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("sqrt failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "cbrt" => {
                if args.len() != 1 {
                    return Err("cbrt expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let cbrt_fn = self.module.get_function("cbrt").unwrap();
                let r = self
                    .builder
                    .build_call(cbrt_fn, &[fv.into()], "cbrt")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("cbrt failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "sin" => {
                if args.len() != 1 {
                    return Err("sin expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("sin").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "sin")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("sin failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "cos" => {
                if args.len() != 1 {
                    return Err("cos expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("cos").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "cos")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("cos failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "tan" => {
                if args.len() != 1 {
                    return Err("tan expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("tan").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "tan")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("tan failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "asin" => {
                if args.len() != 1 {
                    return Err("asin expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("asin").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "asin")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("asin failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "acos" => {
                if args.len() != 1 {
                    return Err("acos expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("acos").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "acos")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("acos failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "atan" => {
                if args.len() != 1 {
                    return Err("atan expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("atan").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "atan")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("atan failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "atan2" => {
                if args.len() != 2 {
                    return Err("atan2 expects 2 arguments".to_string());
                }
                let y = self.compile_expr(&args[0])?;
                let x = self.compile_expr(&args[1])?;
                let yv = self.typed_to_float(&y)?;
                let xv = self.typed_to_float(&x)?;
                let f = self.module.get_function("atan2").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[yv.into(), xv.into()], "atan2")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("atan2 failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "log" => {
                if args.len() != 1 {
                    return Err("log expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("log").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "log")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("log failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "log2" => {
                if args.len() != 1 {
                    return Err("log2 expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("log2").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "log2")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("log2 failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "log10" => {
                if args.len() != 1 {
                    return Err("log10 expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("log10").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "log10")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("log10 failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "exp" => {
                if args.len() != 1 {
                    return Err("exp expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("exp").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "exp")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("exp failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "floor" => {
                if args.len() != 1 {
                    return Err("floor expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("floor").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "floor")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("floor failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "ceil" => {
                if args.len() != 1 {
                    return Err("ceil expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("ceil").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "ceil")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("ceil failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "round" => {
                if args.len() != 1 {
                    return Err("round expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("round").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "round")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("round failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "pi" => {
                if !args.is_empty() {
                    return Err("pi expects no arguments".to_string());
                }
                let pi_val = self.f64_ty().const_float(std::f64::consts::PI);
                Ok(TypedValue::Float(pi_val))
            }
            "e" => {
                if !args.is_empty() {
                    return Err("e expects no arguments".to_string());
                }
                let e_val = self.f64_ty().const_float(std::f64::consts::E);
                Ok(TypedValue::Float(e_val))
            }
            "clamp" => {
                if args.len() != 3 {
                    return Err("clamp expects 3 arguments (value, min, max)".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                let min = self.compile_expr(&args[1])?;
                let max = self.compile_expr(&args[2])?;
                match (&val, &min, &max) {
                    (TypedValue::Int(vv), TypedValue::Int(mn), TypedValue::Int(mx)) => {
                        let lt_min = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, *vv, *mn, "lt_min")
                            .map_err(llvm_err)?;
                        let r1 = self
                            .builder
                            .build_select(lt_min, *mn, *vv, "clamp1")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let gt_max = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, r1, *mx, "gt_max")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_select(gt_max, *mx, r1, "clamp2")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(r2))
                    }
                    (TypedValue::Float(vv), TypedValue::Float(mn), TypedValue::Float(mx)) => {
                        let lt_min = self
                            .builder
                            .build_float_compare(FloatPredicate::OLT, *vv, *mn, "lt_min")
                            .map_err(llvm_err)?;
                        let r1 = self
                            .builder
                            .build_select(lt_min, *mn, *vv, "clamp1")
                            .map_err(llvm_err)?
                            .into_float_value();
                        let gt_max = self
                            .builder
                            .build_float_compare(FloatPredicate::OGT, r1, *mx, "gt_max")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_select(gt_max, *mx, r1, "clamp2")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(r2))
                    }
                    _ => Err("clamp: arguments must be all Int or all Float".to_string()),
                }
            }
            "isNaN" => {
                if args.len() != 1 {
                    return Err("isNaN expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let is_nan = self
                    .builder
                    .build_float_compare(FloatPredicate::UNO, fv, fv, "isNaN")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_nan))
            }
            "isInfinite" => {
                if args.len() != 1 {
                    return Err("isInfinite expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let inf = self.f64_ty().const_float(f64::INFINITY);
                let is_pos_inf = self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, fv, inf, "is_pos_inf")
                    .map_err(llvm_err)?;
                let neg_inf = self.f64_ty().const_float(f64::NEG_INFINITY);
                let is_neg_inf = self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, fv, neg_inf, "is_neg_inf")
                    .map_err(llvm_err)?;
                let is_inf = self
                    .builder
                    .build_or(is_pos_inf, is_neg_inf, "is_inf")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_inf))
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
            "head" => {
                if args.len() != 1 {
                    return Err("head expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) | TypedValue::LazyList(lp) => {
                        let list_val = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(list_val, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let zero = self.i64_ty().const_int(0, false);
                        let empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        // The nullable wraps the i64 tag of the fat element struct
                        let nullable_ty =
                            self.get_nullable_type(self.i64_ty().into(), "Nullable<Int>");
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "head_some");
                        let none_bb = self.context.append_basic_block(current_fn, "head_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "head_merge");
                        let _ = self
                            .builder
                            .build_conditional_branch(empty, none_bb, some_bb);
                        // Some: {flag=0, elem_tag} — extract i64 tag from fat elem
                        self.builder.position_at_end(some_bb);
                        let elem =
                            self.call_rt("action_list_get", &[list_val.into(), zero.into()])?;
                        let elem_bv = elem
                            .try_as_basic_value()
                            .basic()
                            .ok_or("get failed")?
                            .into_struct_value();
                        let elem_tag = self
                            .builder
                            .build_extract_value(elem_bv, 0, "elem_tag")
                            .map_err(llvm_err)?;
                        let some_struct = {
                            let undef = nullable_ty.get_undef();
                            let r1 = self
                                .builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(0, false),
                                    0,
                                    "s_flag",
                                )
                                .map_err(llvm_err)?;
                            self.builder
                                .build_insert_value(r1, elem_tag, 1, "s_val")
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None: {flag=1, undef}
                        self.builder.position_at_end(none_bb);
                        let none_struct = {
                            let undef = nullable_ty.get_undef();
                            self.builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(1, false),
                                    0,
                                    "n_flag",
                                )
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(nullable_ty, "head_result")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                        let alloca = self
                            .builder
                            .build_alloca(nullable_ty, "head")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
                    }
                    _ => Err("head: argument must be a list".to_string()),
                }
            }
            "last" => {
                if args.len() != 1 {
                    return Err("last expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) | TypedValue::LazyList(lp) => {
                        let list_val = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(list_val, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let zero = self.i64_ty().const_int(0, false);
                        let empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        let last_idx = self
                            .builder
                            .build_int_sub(len, self.i64_ty().const_int(1, false), "last_idx")
                            .map_err(llvm_err)?;
                        let nullable_ty =
                            self.get_nullable_type(self.i64_ty().into(), "Nullable<Int>");
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "last_some");
                        let none_bb = self.context.append_basic_block(current_fn, "last_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "last_merge");
                        let _ = self
                            .builder
                            .build_conditional_branch(empty, none_bb, some_bb);
                        // Some: {flag=0, elem_tag} — extract i64 tag from fat elem
                        self.builder.position_at_end(some_bb);
                        let elem =
                            self.call_rt("action_list_get", &[list_val.into(), last_idx.into()])?;
                        let elem_bv = elem
                            .try_as_basic_value()
                            .basic()
                            .ok_or("get failed")?
                            .into_struct_value();
                        let elem_tag = self
                            .builder
                            .build_extract_value(elem_bv, 0, "elem_tag")
                            .map_err(llvm_err)?;
                        let some_struct = {
                            let undef = nullable_ty.get_undef();
                            let r1 = self
                                .builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(0, false),
                                    0,
                                    "s_flag",
                                )
                                .map_err(llvm_err)?;
                            self.builder
                                .build_insert_value(r1, elem_tag, 1, "s_val")
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None: {flag=1, undef}
                        self.builder.position_at_end(none_bb);
                        let none_struct = {
                            let undef = nullable_ty.get_undef();
                            self.builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(1, false),
                                    0,
                                    "n_flag",
                                )
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(nullable_ty, "last_result")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                        let alloca = self
                            .builder
                            .build_alloca(nullable_ty, "last")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
                    }
                    _ => Err("last: argument must be a list".to_string()),
                }
            }
            "get" => {
                if args.len() != 2 {
                    return Err("get expects 2 arguments (list, index)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let idx_val = self.compile_expr(&args[1])?;
                match (&list_val, &idx_val) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let zero = self.i64_ty().const_int(0, false);
                        let neg = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, *iv, zero, "neg")
                            .map_err(llvm_err)?;
                        let ge_len = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, *iv, len, "ge_len")
                            .map_err(llvm_err)?;
                        let oob = self
                            .builder
                            .build_or(neg, ge_len, "oob")
                            .map_err(llvm_err)?;
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "get_some");
                        let none_bb = self.context.append_basic_block(current_fn, "get_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "get_merge");
                        let _ = self.builder.build_conditional_branch(oob, none_bb, some_bb);
                        // Some: {flag=0, elem} — value inlined, no heap alloc
                        self.builder.position_at_end(some_bb);
                        let elem = self.call_rt("action_list_get", &[lv.into(), (*iv).into()])?;
                        let elem_bv = elem.try_as_basic_value().basic().ok_or("get failed")?;
                        let nullable_ty =
                            self.get_nullable_type(self.string_type.into(), "Nullable<Str>");
                        let some_struct = {
                            let undef = nullable_ty.get_undef();
                            let r1 = self
                                .builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(0, false),
                                    0,
                                    "s_flag",
                                )
                                .map_err(llvm_err)?;
                            self.builder
                                .build_insert_value(r1, elem_bv, 1, "s_val")
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None: {flag=1, undef}
                        self.builder.position_at_end(none_bb);
                        let none_struct = {
                            let undef = nullable_ty.get_undef();
                            self.builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(1, false),
                                    0,
                                    "n_flag",
                                )
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(nullable_ty, "get_result")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                        let alloca = self
                            .builder
                            .build_alloca(nullable_ty, "get")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
                    }
                    _ => Err("get: first argument must be a list, second an Int".to_string()),
                }
            }
            "remove" => {
                if args.len() != 2 {
                    return Err("remove expects 2 arguments (list, index)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let idx_val = self.compile_expr(&args[1])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("remove expects (List, Int)".to_string()),
                }
            }
            "reverse" => {
                if args.len() != 1 {
                    return Err("reverse expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("reverse: argument must be a list".to_string()),
                }
            }
            "contains" => {
                if args.len() != 2 {
                    return Err("contains expects 2 arguments (list, element)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let elem_val = self.compile_expr(&args[1])?;
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
                        Ok(TypedValue::Bool(result))
                    }
                    (TypedValue::Set(sp), _) => {
                        let lv = self.load_list(*sp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let cc = self.call_rt("action_list_contains", &[lv.into(), fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("contains failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("contains: first argument must be a list or set".to_string()),
                }
            }
            "containsKey" => {
                if args.len() != 2 {
                    return Err("containsKey expects 2 arguments (map, key)".to_string());
                }
                let map_val = self.compile_expr(&args[0])?;
                let key_val = self.compile_expr(&args[1])?;
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
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("containsKey: first argument must be a map".to_string()),
                }
            }
            "prepend" => {
                if args.len() != 2 {
                    return Err("prepend expects 2 arguments (element, list)".to_string());
                }
                let elem_val = self.compile_expr(&args[0])?;
                let list_val = self.compile_expr(&args[1])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("prepend: second argument must be a list".to_string()),
                }
            }
            "take" => {
                if args.len() != 2 {
                    return Err("take expects 2 arguments (list, n)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let n_val = self.compile_expr(&args[1])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("take: first argument must be a list, second an Int".to_string()),
                }
            }
            "drop" => {
                if args.len() != 2 {
                    return Err("drop expects 2 arguments (list, n)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let n_val = self.compile_expr(&args[1])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("drop: first argument must be a list, second an Int".to_string()),
                }
            }
            "range" => {
                if args.len() != 2 {
                    return Err("range expects 2 arguments (start, end)".to_string());
                }
                let start = self.compile_expr(&args[0])?;
                let end = self.compile_expr(&args[1])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("range: arguments must be Int".to_string()),
                }
            }
            "repeat" => {
                if args.len() != 2 {
                    return Err("repeat expects 2 arguments (value, count)".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                let count = self.compile_expr(&args[1])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("repeat: second argument must be Int".to_string()),
                }
            }
            "trimStart" => {
                if args.len() != 1 {
                    return Err("trimStart expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
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
                let v = self.compile_expr(&args[0])?;
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
                let s = self.compile_expr(&args[0])?;
                let sub = self.compile_expr(&args[1])?;
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
                let s = self.compile_expr(&args[0])?;
                let count = self.compile_expr(&args[1])?;
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
            "tail" => {
                if args.len() != 1 {
                    return Err("tail expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let is_empty = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                len,
                                self.i64_ty().const_int(0, false),
                                "empty",
                            )
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_list_tail", &[lv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("tail failed")?
                            .into_struct_value();
                        self.build_nullable_list(result, is_empty)
                    }
                    _ => Err("tail: argument must be a list".to_string()),
                }
            }
            "zip" => {
                if args.len() != 2 {
                    return Err("zip expects 2 arguments (list1, list2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("zip: arguments must be lists".to_string()),
                }
            }
            "splitLines" => {
                if args.len() != 1 {
                    return Err("splitLines expects 1 argument (string)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
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
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
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
            "insert" => {
                if args.len() != 3 {
                    return Err("insert expects 3 arguments (list, index, elem)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let idx_val = self.compile_expr(&args[1])?;
                let elem_val = self.compile_expr(&args[2])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("insert expects (List, Int, Any)".to_string()),
                }
            }
            "init" => {
                if args.len() != 1 {
                    return Err("init expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let is_empty = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                len,
                                self.i64_ty().const_int(0, false),
                                "empty",
                            )
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_list_init", &[lv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("init failed")?
                            .into_struct_value();
                        self.build_nullable_list(result, is_empty)
                    }
                    _ => Err("init: argument must be a list".to_string()),
                }
            }
            "chars" => {
                if args.len() != 1 {
                    return Err("chars expects 1 argument (string)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
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
            "setToList" => {
                if args.len() != 1 {
                    return Err("setToList expects 1 argument (set)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Set(p) => Ok(TypedValue::List(p)),
                    _ => Err("setToList: argument must be a set".to_string()),
                }
            }
            "setFromList" => {
                if args.len() != 1 {
                    return Err("setFromList expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(p) => Ok(TypedValue::Set(p)),
                    _ => Err("setFromList: argument must be a list".to_string()),
                }
            }
            "fromList" => {
                if args.len() != 1 {
                    return Err("fromList expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(p) => Ok(TypedValue::Set(p)),
                    _ => Err("fromList: argument must be a list".to_string()),
                }
            }
            "today" => {
                if !args.is_empty() {
                    return Err("today expects no arguments".to_string());
                }
                // Call C time() and localtime_r() to get real current date
                self.emit_today_now(false)
            }
            "now" => {
                if !args.is_empty() {
                    return Err("now expects no arguments".to_string());
                }
                self.emit_today_now(true)
            }
            // DateTime/Date field accessors
            "year" | "month" | "day" | "hour" | "minute" | "second" => {
                if args.len() != 1 {
                    return Err(format!("{} expects 1 argument", name));
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Struct(p, st) => {
                        let field_idx = match name {
                            "year" => 0,
                            "month" => 1,
                            "day" => 2,
                            "hour" => 3,
                            "minute" => 4,
                            "second" => 5,
                            _ => return Err("bad field".to_string()),
                        };
                        let fptr = self
                            .builder
                            .build_struct_gep(st, p, field_idx, "fptr")
                            .map_err(llvm_err)?;
                        let val = self
                            .builder
                            .build_load(self.i64_ty(), fptr, "val")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(val))
                    }
                    _ => Err(format!(
                        "{}: argument must be a Date or DateTime struct",
                        name
                    )),
                }
            }
            "addDays" => {
                if args.len() != 2 {
                    return Err("addDays expects 2 arguments (date, days)".to_string());
                }
                let d = self.compile_expr(&args[0])?;
                let days = self.compile_expr(&args[1])?;
                let days_bv = days.to_bv().ok_or("days must be Int")?;
                match d {
                    TypedValue::Struct(p, st) => {
                        // Create a new Date struct with added days
                        let alloca = self
                            .builder
                            .build_alloca(st, "new_date")
                            .map_err(llvm_err)?;
                        for i in 0..3u32 {
                            let fptr = self
                                .builder
                                .build_struct_gep(st, p, i, "fptr")
                                .map_err(llvm_err)?;
                            let fval = self
                                .builder
                                .build_load(self.i64_ty(), fptr, "fval")
                                .map_err(llvm_err)?
                                .into_int_value();
                            let new_val = if i == 2 {
                                self.builder
                                    .build_int_add(fval, days_bv.into_int_value(), "new_day")
                                    .map_err(llvm_err)?
                                    .into()
                            } else {
                                fval
                            };
                            let dfptr = self
                                .builder
                                .build_struct_gep(st, alloca, i, "dfptr")
                                .map_err(llvm_err)?;
                            self.builder.build_store(dfptr, new_val).map_err(llvm_err)?;
                        }
                        Ok(TypedValue::Struct(alloca, st))
                    }
                    _ => Err("addDays: first argument must be a Date struct".to_string()),
                }
            }
            "addHours" => {
                if args.len() != 2 {
                    return Err("addHours expects 2 arguments (datetime, hours)".to_string());
                }
                let d = self.compile_expr(&args[0])?;
                let hours = self.compile_expr(&args[1])?;
                let hours_bv = hours.to_bv().ok_or("hours must be Int")?;
                match d {
                    TypedValue::Struct(p, st) => {
                        let alloca = self.builder.build_alloca(st, "new_dt").map_err(llvm_err)?;
                        for i in 0..6u32 {
                            let fptr = self
                                .builder
                                .build_struct_gep(st, p, i, "fptr")
                                .map_err(llvm_err)?;
                            let fval = self
                                .builder
                                .build_load(self.i64_ty(), fptr, "fval")
                                .map_err(llvm_err)?
                                .into_int_value();
                            let new_val = if i == 3 {
                                self.builder
                                    .build_int_add(fval, hours_bv.into_int_value(), "new_hour")
                                    .map_err(llvm_err)?
                                    .into()
                            } else {
                                fval
                            };
                            let dfptr = self
                                .builder
                                .build_struct_gep(st, alloca, i, "dfptr")
                                .map_err(llvm_err)?;
                            self.builder.build_store(dfptr, new_val).map_err(llvm_err)?;
                        }
                        Ok(TypedValue::Struct(alloca, st))
                    }
                    _ => Err("addHours: first argument must be a DateTime struct".to_string()),
                }
            }
            "randChoice" => {
                if args.len() != 1 {
                    return Err("randChoice expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
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
                        Ok(TypedValue::List(opt_alloca)) // Reuse List type for the result
                    }
                    _ => Err("randChoice: argument must be a list".to_string()),
                }
            }
            "toChar" => {
                if args.len() != 1 {
                    return Err("toChar expects 1 argument (int)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
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
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => Ok(TypedValue::Int(iv)),
                    _ => Err("charCode: argument must be a Char".to_string()),
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
            "withIndex" => {
                if args.len() != 1 {
                    return Err("withIndex expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
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
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("withIndex: argument must be a list".to_string()),
                }
            }
            "unique" => {
                if args.len() != 1 {
                    return Err("unique expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_unique", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("unique failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "unique")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("unique: argument must be a list".to_string()),
                }
            }
            "slice" => {
                if args.len() != 3 {
                    return Err("slice expects 3 arguments (collection, start, end)".to_string());
                }
                let coll_v = self.compile_expr(&args[0])?;
                let start_v = self.compile_expr(&args[1])?;
                let end_v = self.compile_expr(&args[2])?;
                match (&coll_v, &start_v, &end_v) {
                    // slice(List<T>, Int, Int) -> List<T>  with [start, end) semantics
                    (TypedValue::List(lp), TypedValue::Int(sv), TypedValue::Int(ev)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt(
                            "action_list_slice",
                            &[lv.into(), (*sv).into(), (*ev).into()],
                        )?;
                        let result = cc.try_as_basic_value().basic().ok_or("slice failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "slice")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    // slice(String, Int, Int) -> String  with [start, end) semantics
                    (TypedValue::Str(sp), TypedValue::Int(sv), TypedValue::Int(ev)) => {
                        let str_val = self.load_string(*sp)?;
                        let len = self
                            .builder
                            .build_int_sub(*ev, *sv, "slice_len")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt(
                            "action_string_substring",
                            &[str_val.into(), (*sv).into(), len.into()],
                        )?;
                        let result = cc.try_as_basic_value().basic().ok_or("slice failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "slice_str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err(
                        "slice: first argument must be a list or string, second and third Int"
                            .to_string(),
                    ),
                }
            }
            "flatten" => {
                if args.len() != 1 {
                    return Err("flatten expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_flatten", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("flatten failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "flatten")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("flatten: argument must be a list".to_string()),
                }
            }
            "splitAt" => {
                if args.len() != 2 {
                    return Err("splitAt expects 2 arguments (list, index)".to_string());
                }
                let list_v = self.compile_expr(&args[0])?;
                let idx_v = self.compile_expr(&args[1])?;
                match (&list_v, &idx_v) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc =
                            self.call_rt("action_list_split_at", &[lv.into(), (*iv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("splitAt failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "splitAt")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("splitAt: first argument must be a list, second Int".to_string()),
                }
            }
            "chunks" => {
                if args.len() != 2 {
                    return Err("chunks expects 2 arguments (list, size)".to_string());
                }
                let list_v = self.compile_expr(&args[0])?;
                let size_v = self.compile_expr(&args[1])?;
                match (&list_v, &size_v) {
                    (TypedValue::List(lp), TypedValue::Int(sv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_chunks", &[lv.into(), (*sv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("chunks failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "chunks")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("chunks: first argument must be a list, second Int".to_string()),
                }
            }
            "windows" => {
                if args.len() != 2 {
                    return Err("windows expects 2 arguments (list, size)".to_string());
                }
                let list_v = self.compile_expr(&args[0])?;
                let size_v = self.compile_expr(&args[1])?;
                match (&list_v, &size_v) {
                    (TypedValue::List(lp), TypedValue::Int(sv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_windows", &[lv.into(), (*sv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("windows failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "windows")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("windows: first argument must be a list, second Int".to_string()),
                }
            }
            "pow" => {
                if args.len() != 2 {
                    return Err("pow expects 2 arguments".to_string());
                }
                let base = self.compile_expr(&args[0])?;
                let exp = self.compile_expr(&args[1])?;
                match (&base, &exp) {
                    (TypedValue::Float(bv), TypedValue::Float(ev)) => {
                        let cc = self.call_rt("action_pow", &[(*bv).into(), (*ev).into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    (TypedValue::Int(bv), TypedValue::Int(ev)) => {
                        let bf = self
                            .builder
                            .build_signed_int_to_float(*bv, self.f64_ty(), "bf")
                            .map_err(llvm_err)?;
                        let ef = self
                            .builder
                            .build_signed_int_to_float(*ev, self.f64_ty(), "ef")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_pow", &[bf.into(), ef.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    // Mixed Int/Float → promote Int to Float
                    (TypedValue::Int(bv), TypedValue::Float(ev)) => {
                        let bf = self
                            .builder
                            .build_signed_int_to_float(*bv, self.f64_ty(), "bf")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_pow", &[bf.into(), (*ev).into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    (TypedValue::Float(bv), TypedValue::Int(ev)) => {
                        let ef = self
                            .builder
                            .build_signed_int_to_float(*ev, self.f64_ty(), "ef")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_pow", &[(*bv).into(), ef.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("pow: arguments must be numeric".to_string()),
                }
            }
            "mapKeys" => {
                if args.len() != 1 {
                    return Err("mapKeys expects 1 argument (map)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_keys", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapKeys failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "keys")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("mapKeys: argument must be a map".to_string()),
                }
            }
            "mapValues" => {
                if args.len() != 1 {
                    return Err("mapValues expects 1 argument (map)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_values", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapValues failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "values")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("mapValues: argument must be a map".to_string()),
                }
            }
            "mapEntries" => {
                if args.len() != 1 {
                    return Err("mapEntries expects 1 argument (map)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_entries", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapEntries failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "entries")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("mapEntries: argument must be a map".to_string()),
                }
            }
            "mapUnion" => {
                if args.len() != 2 {
                    return Err("map.union expects 2 arguments (map1, map2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Map(mp1), TypedValue::Map(mp2)) => {
                        let mv1 = self.load_list(*mp1)?;
                        let mv2 = self.load_list(*mp2)?;
                        let cc = self.call_rt("action_map_union", &[mv1.into(), mv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("map.union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "mapUnion")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Map(alloca))
                    }
                    _ => Err("map.union: arguments must be maps".to_string()),
                }
            }
            "setUnion" => {
                if args.len() != 2 {
                    return Err("set.union expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc = self.call_rt("action_set_union", &[sv1.into(), sv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("set.union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "union")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("set.union: arguments must be sets".to_string()),
                }
            }
            "setIntersection" => {
                if args.len() != 2 {
                    return Err("set.intersection expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc =
                            self.call_rt("action_set_intersection", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.intersection failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "intersection")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("set.intersection: arguments must be sets".to_string()),
                }
            }
            "setDifference" => {
                if args.len() != 2 {
                    return Err("set.difference expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc =
                            self.call_rt("action_set_difference", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.difference failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "difference")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("set.difference: arguments must be sets".to_string()),
                }
            }
            "setIsSubset" => {
                if args.len() != 2 {
                    return Err("set.isSubset expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc = self.call_rt("action_set_is_subset", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.isSubset failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("set.isSubset: arguments must be sets".to_string()),
                }
            }
            "randShuffle" => {
                if args.len() != 1 {
                    return Err("randShuffle expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_rand_shuffle", &[lv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("randShuffle failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "shuffled")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("randShuffle: argument must be a list".to_string()),
                }
            }
            "sorted" => {
                if args.len() != 1 {
                    return Err("sorted expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_sorted", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("sorted failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "sorted")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("sorted: argument must be a list".to_string()),
                }
            }
            "readDir" => {
                if args.len() != 1 {
                    return Err("readDir expects 1 argument (path)".to_string());
                }
                if self.module.get_function("action_read_dir").is_none() {
                    self.emit_read_dir_runtime()?;
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_read_dir", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("readDir failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "readDir")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("readDir: argument must be a string".to_string()),
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
            "diffDays" => {
                if args.len() != 2 {
                    return Err("diffDays expects 2 arguments (date1, date2)".to_string());
                }
                let d1 = self.compile_expr(&args[0])?;
                let d2 = self.compile_expr(&args[1])?;
                let (p1, st1) = match d1 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffDays: arguments must be Date structs".to_string()),
                };
                let (p2, st2) = match d2 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffDays: arguments must be Date structs".to_string()),
                };
                let i64_ty = self.i64_ty();
                // Julian Day Number: JDN = D + (153*m+2)/5 + 365*y + y/4 - y/100 + y/400 - 32045
                // where a = (14-M)/12, y = Y+4800-a, m = M+12*a-3
                let jdn = |yp: PointerValue<'ctx>,
                           sty: inkwell::types::StructType<'ctx>|
                 -> Result<IntValue<'ctx>, String> {
                    let y_ptr = self
                        .builder
                        .build_struct_gep(sty, yp, 0, "j_y")
                        .map_err(llvm_err)?;
                    let y_val = self
                        .builder
                        .build_load(i64_ty, y_ptr, "j_yv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let m_ptr = self
                        .builder
                        .build_struct_gep(sty, yp, 1, "j_m")
                        .map_err(llvm_err)?;
                    let m_val = self
                        .builder
                        .build_load(i64_ty, m_ptr, "j_mv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let d_ptr = self
                        .builder
                        .build_struct_gep(sty, yp, 2, "j_d")
                        .map_err(llvm_err)?;
                    let d_val = self
                        .builder
                        .build_load(i64_ty, d_ptr, "j_dv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let c12 = i64_ty.const_int(12, false);
                    let c14 = i64_ty.const_int(14, false);
                    let c4800 = i64_ty.const_int(4800, false);
                    let c3 = i64_ty.const_int(3, false);
                    let c4 = i64_ty.const_int(4, false);
                    let c100 = i64_ty.const_int(100, false);
                    let c400 = i64_ty.const_int(400, false);
                    let c153 = i64_ty.const_int(153, false);
                    let c2 = i64_ty.const_int(2, false);
                    let c5 = i64_ty.const_int(5, false);
                    let c365 = i64_ty.const_int(365, false);
                    let c32045 = i64_ty.const_int(32045, false);
                    // a = (14 - M) / 12
                    let a = self
                        .builder
                        .build_int_signed_div(
                            self.builder
                                .build_int_sub(c14, m_val, "t_a1")
                                .map_err(llvm_err)?,
                            c12,
                            "a",
                        )
                        .map_err(llvm_err)?;
                    // y = Y + 4800 - a
                    let y = self
                        .builder
                        .build_int_sub(
                            self.builder
                                .build_int_add(y_val, c4800, "t_y1")
                                .map_err(llvm_err)?,
                            a,
                            "y",
                        )
                        .map_err(llvm_err)?;
                    // m = M + 12*a - 3
                    let m = self
                        .builder
                        .build_int_sub(
                            self.builder
                                .build_int_add(
                                    m_val,
                                    self.builder
                                        .build_int_mul(c12, a, "t_m1")
                                        .map_err(llvm_err)?,
                                    "t_m2",
                                )
                                .map_err(llvm_err)?,
                            c3,
                            "m",
                        )
                        .map_err(llvm_err)?;
                    // term1 = (153*m + 2) / 5
                    let term1 = self
                        .builder
                        .build_int_signed_div(
                            self.builder
                                .build_int_add(
                                    self.builder
                                        .build_int_mul(c153, m, "t_t1a")
                                        .map_err(llvm_err)?,
                                    c2,
                                    "t_t1b",
                                )
                                .map_err(llvm_err)?,
                            c5,
                            "term1",
                        )
                        .map_err(llvm_err)?;
                    // term2 = 365*y
                    let term2 = self
                        .builder
                        .build_int_mul(c365, y, "term2")
                        .map_err(llvm_err)?;
                    // term3 = y/4
                    let term3 = self
                        .builder
                        .build_int_signed_div(y, c4, "term3")
                        .map_err(llvm_err)?;
                    // term4 = y/100
                    let term4 = self
                        .builder
                        .build_int_signed_div(y, c100, "term4")
                        .map_err(llvm_err)?;
                    // term5 = y/400
                    let term5 = self
                        .builder
                        .build_int_signed_div(y, c400, "term5")
                        .map_err(llvm_err)?;
                    // JDN = D + term1 + term2 + term3 - term4 + term5 - 32045
                    let s1 = self
                        .builder
                        .build_int_add(d_val, term1, "s1")
                        .map_err(llvm_err)?;
                    let s2 = self
                        .builder
                        .build_int_add(s1, term2, "s2")
                        .map_err(llvm_err)?;
                    let s3 = self
                        .builder
                        .build_int_add(s2, term3, "s3")
                        .map_err(llvm_err)?;
                    let s4 = self
                        .builder
                        .build_int_sub(s3, term4, "s4")
                        .map_err(llvm_err)?;
                    let s5 = self
                        .builder
                        .build_int_add(s4, term5, "s5")
                        .map_err(llvm_err)?;
                    let jdn_val = self
                        .builder
                        .build_int_sub(s5, c32045, "jdn")
                        .map_err(llvm_err)?;
                    Ok(jdn_val)
                };
                let j1 = jdn(p1, st1)?;
                let j2 = jdn(p2, st2)?;
                let diff = self
                    .builder
                    .build_int_sub(j1, j2, "diff")
                    .map_err(llvm_err)?;
                let zero = i64_ty.const_int(0, false);
                let nd = self.builder.build_int_neg(diff, "nd").map_err(llvm_err)?;
                let is_neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, diff, zero, "is_neg")
                    .map_err(llvm_err)?;
                let abs_diff = self
                    .builder
                    .build_select(is_neg, nd, diff, "abs_diff")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::Int(abs_diff))
            }
            "weekday" => {
                if args.len() != 1 {
                    return Err("weekday expects 1 argument (date)".to_string());
                }
                let d = self.compile_expr(&args[0])?;
                match d {
                    TypedValue::Struct(p, st) => {
                        // Use mktime to compute proper weekday
                        // Build struct tm: {i32 x 9}
                        let i32_ty = self.context.i32_type();
                        let tm_ty = self.context.struct_type(&[i32_ty.into(); 9], false);
                        let tm_a = self.builder.build_alloca(tm_ty, "tm").map_err(llvm_err)?;
                        let i64_ty = self.i64_ty();
                        // Extract year, month, day from Date struct
                        let yp = self
                            .builder
                            .build_struct_gep(st, p, 0, "w_yp")
                            .map_err(llvm_err)?;
                        let yv = self
                            .builder
                            .build_load(i64_ty, yp, "w_yv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let mp = self
                            .builder
                            .build_struct_gep(st, p, 1, "w_mp")
                            .map_err(llvm_err)?;
                        let mv = self
                            .builder
                            .build_load(i64_ty, mp, "w_mv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let dp = self
                            .builder
                            .build_struct_gep(st, p, 2, "w_dp")
                            .map_err(llvm_err)?;
                        let dv = self
                            .builder
                            .build_load(i64_ty, dp, "w_dv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        // tm_sec = 0
                        let f0 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 0, "f0")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f0, i32_ty.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_min = 0
                        let f1 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 1, "f1")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f1, i32_ty.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_hour = 12 (noon, avoid DST issues)
                        let f2 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 2, "f2")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f2, i32_ty.const_int(12, false))
                            .map_err(llvm_err)?;
                        // tm_mday = day
                        let f3 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 3, "f3")
                            .map_err(llvm_err)?;
                        let dv32 = self
                            .builder
                            .build_int_truncate(dv, i32_ty, "dv32")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f3, dv32).map_err(llvm_err)?;
                        // tm_mon = month - 1
                        let f4 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 4, "f4")
                            .map_err(llvm_err)?;
                        let mon_minus = self
                            .builder
                            .build_int_sub(mv, i64_ty.const_int(1, false), "mon_minus")
                            .map_err(llvm_err)?;
                        let mon32 = self
                            .builder
                            .build_int_truncate(mon_minus, i32_ty, "mon32")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f4, mon32).map_err(llvm_err)?;
                        // tm_year = year - 1900
                        let f5 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 5, "f5")
                            .map_err(llvm_err)?;
                        let y_minus = self
                            .builder
                            .build_int_sub(yv, i64_ty.const_int(1900, false), "y_minus")
                            .map_err(llvm_err)?;
                        let y32 = self
                            .builder
                            .build_int_truncate(y_minus, i32_ty, "y32")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f5, y32).map_err(llvm_err)?;
                        // Remaining fields init to 0
                        for i in 6..9u32 {
                            let f = self
                                .builder
                                .build_struct_gep(tm_ty, tm_a, i, "f")
                                .map_err(llvm_err)?;
                            self.builder
                                .build_store(f, i32_ty.const_int(0, false))
                                .map_err(llvm_err)?;
                        }
                        // Call mktime
                        let mktime_fn = self.module.get_function("mktime").unwrap_or_else(|| {
                            self.module.add_function(
                                "mktime",
                                self.i64_ty().fn_type(&[self.ptr_ty().into()], false),
                                None,
                            )
                        });
                        let _ = self
                            .builder
                            .build_call(mktime_fn, &[tm_a.into()], "")
                            .map_err(llvm_err)?;
                        // Read tm_wday (field 6)
                        let wf = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 6, "wf")
                            .map_err(llvm_err)?;
                        let wday32 = self
                            .builder
                            .build_load(i32_ty, wf, "wday")
                            .map_err(llvm_err)?
                            .into_int_value();
                        // Convert: C wday 0=Sunday -> Atomic 1=Monday..7=Sunday
                        // Atomic weekday: 1=Monday, 7=Sunday
                        // C: 0=Sun,1=Mon,2=Tue,3=Wed,4=Thu,5=Fri,6=Sat
                        // Map: C=0->7, C=1->1, C=2->2, C=3->3, C=4->4, C=5->5, C=6->6
                        let wd_c0 = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                wday32,
                                i32_ty.const_int(0, false),
                                "wd_c0",
                            )
                            .map_err(llvm_err)?;
                        let wd32 = self
                            .builder
                            .build_select(wd_c0, i32_ty.const_int(7, false), wday32, "wd")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let wd = self
                            .builder
                            .build_int_s_extend(wd32, i64_ty, "wd64")
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Int(wd))
                    }
                    _ => Err("weekday: argument must be a Date struct".to_string()),
                }
            }
            "sum" => {
                if args.len() != 1 {
                    return Err("sum expects 1 argument (list)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("sum: argument must be a list".to_string()),
                };
                let list = self.load_list(list_ptr)?;
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
                let result = self
                    .builder
                    .build_load(self.i64_ty(), sum_a, "result")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Int(result.into_int_value()))
            }
            "product" => {
                if args.len() != 1 {
                    return Err("product expects 1 argument (list)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("product: argument must be a list".to_string()),
                };
                let list = self.load_list(list_ptr)?;
                let len = self.list_len_val(list)?;
                let data = self.list_data_ptr(list)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                let prod_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "prod")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(prod_a, self.i64_ty().const_int(1, false))
                    .map_err(llvm_err)?;
                let i_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "i")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(i_a, self.i64_ty().const_int(0, false))
                    .map_err(llvm_err)?;
                let hdr = self.context.append_basic_block(current, "prod_hdr");
                let bdy = self.context.append_basic_block(current, "prod_bdy");
                let ext = self.context.append_basic_block(current, "prod_ext");
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
                    .build_load(self.i64_ty(), prod_a, "cur")
                    .map_err(llvm_err)?
                    .into_int_value();
                let new_prod = self
                    .builder
                    .build_int_mul(cur, etag, "new_prod")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(prod_a, new_prod)
                    .map_err(llvm_err)?;
                let ni = self
                    .builder
                    .build_int_add(iv, self.i64_ty().const_int(1, false), "ni")
                    .map_err(llvm_err)?;
                self.builder.build_store(i_a, ni).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(ext);
                let result = self
                    .builder
                    .build_load(self.i64_ty(), prod_a, "result")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Int(result.into_int_value()))
            }
            "digits" => {
                // digits(n) -> List<Int>: decimal digits of abs(n), MSD first. 0 -> [0].
                if args.len() != 1 {
                    return Err("digits expects 1 argument (int)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let n = match v {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("digits: argument must be an int".to_string()),
                };
                let ten = self.i64_ty().const_int(10, false);
                let zero = self.i64_ty().const_int(0, false);
                let one = self.i64_ty().const_int(1, false);
                // abs_n = n < 0 ? -n : n
                let neg = self.builder.build_int_neg(n, "neg").map_err(llvm_err)?;
                let is_neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, n, zero, "is_neg")
                    .map_err(llvm_err)?;
                let abs_n = self
                    .builder
                    .build_select(is_neg, neg, n, "abs_n")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_zero = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, n, zero, "is0")
                    .map_err(llvm_err)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                // Count digits via repeated division
                let dc_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "dc")
                    .map_err(llvm_err)?;
                self.builder.build_store(dc_a, zero).map_err(llvm_err)?;
                let tmp_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "tmp")
                    .map_err(llvm_err)?;
                self.builder.build_store(tmp_a, abs_n).map_err(llvm_err)?;
                let cnt_hdr = self.context.append_basic_block(current, "dc_hdr");
                let cnt_bdy = self.context.append_basic_block(current, "dc_bdy");
                let cnt_ext = self.context.append_basic_block(current, "dc_ext");
                let _ = self.builder.build_unconditional_branch(cnt_hdr);
                self.builder.position_at_end(cnt_hdr);
                let tv = self
                    .builder
                    .build_load(self.i64_ty(), tmp_a, "tv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let gt0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGT, tv, zero, "gt0")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(gt0, cnt_bdy, cnt_ext);
                self.builder.position_at_end(cnt_bdy);
                let dv = self
                    .builder
                    .build_load(self.i64_ty(), dc_a, "dv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let nd = self
                    .builder
                    .build_int_add(dv, one, "nd")
                    .map_err(llvm_err)?;
                self.builder.build_store(dc_a, nd).map_err(llvm_err)?;
                let nt = self
                    .builder
                    .build_int_signed_div(tv, ten, "nt")
                    .map_err(llvm_err)?;
                self.builder.build_store(tmp_a, nt).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(cnt_hdr);
                self.builder.position_at_end(cnt_ext);
                let ndigits = self
                    .builder
                    .build_load(self.i64_ty(), dc_a, "nd")
                    .map_err(llvm_err)?
                    .into_int_value();
                // 0 -> 1 digit
                let final_dc = self
                    .builder
                    .build_select(is_zero, one, ndigits, "fdc")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Create result list with capacity = final_dc
                let cc = self.call_rt("action_list_create", &[final_dc.into()])?;
                let res_bv = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("list_create failed")?;
                let res_a = self
                    .builder
                    .build_alloca(self.list_type, "digits_res")
                    .map_err(llvm_err)?;
                self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
                // Compute 10^(ndigits-1) iteratively
                let pow_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "pow10")
                    .map_err(llvm_err)?;
                self.builder.build_store(pow_a, one).map_err(llvm_err)?;
                let pi_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "pi")
                    .map_err(llvm_err)?;
                self.builder.build_store(pi_a, one).map_err(llvm_err)?;
                let pow_hdr = self.context.append_basic_block(current, "pow_hdr");
                let pow_bdy = self.context.append_basic_block(current, "pow_bdy");
                let pow_ext = self.context.append_basic_block(current, "pow_ext");
                let _ = self.builder.build_unconditional_branch(pow_hdr);
                self.builder.position_at_end(pow_hdr);
                let piv = self
                    .builder
                    .build_load(self.i64_ty(), pi_a, "piv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let plt = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, piv, final_dc, "plt")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(plt, pow_bdy, pow_ext);
                self.builder.position_at_end(pow_bdy);
                let pv = self
                    .builder
                    .build_load(self.i64_ty(), pow_a, "pv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let npv = self
                    .builder
                    .build_int_mul(pv, ten, "npv")
                    .map_err(llvm_err)?;
                self.builder.build_store(pow_a, npv).map_err(llvm_err)?;
                let npi = self
                    .builder
                    .build_int_add(piv, one, "npi")
                    .map_err(llvm_err)?;
                self.builder.build_store(pi_a, npi).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(pow_hdr);
                self.builder.position_at_end(pow_ext);
                let pow10 = self
                    .builder
                    .build_load(self.i64_ty(), pow_a, "pow10")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Extract digits MSD-first: for i in 0..ndigits { d = (abs_n / pow10) % 10; push; pow10 /= 10 }
                self.builder.build_store(tmp_a, abs_n).map_err(llvm_err)?;
                let di_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "di")
                    .map_err(llvm_err)?;
                self.builder.build_store(di_a, zero).map_err(llvm_err)?;
                let p10_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "p10")
                    .map_err(llvm_err)?;
                self.builder.build_store(p10_a, pow10).map_err(llvm_err)?;
                let fill_hdr = self.context.append_basic_block(current, "fill_hdr");
                let fill_bdy = self.context.append_basic_block(current, "fill_bdy");
                let fill_ext = self.context.append_basic_block(current, "fill_ext");
                let _ = self.builder.build_unconditional_branch(fill_hdr);
                self.builder.position_at_end(fill_hdr);
                let div = self
                    .builder
                    .build_load(self.i64_ty(), di_a, "div")
                    .map_err(llvm_err)?
                    .into_int_value();
                let flt = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, div, final_dc, "flt")
                    .map_err(llvm_err)?;
                let _ = self
                    .builder
                    .build_conditional_branch(flt, fill_bdy, fill_ext);
                self.builder.position_at_end(fill_bdy);
                let cur_pow = self
                    .builder
                    .build_load(self.i64_ty(), p10_a, "cur_pow")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cur_n = self
                    .builder
                    .build_load(self.i64_ty(), tmp_a, "cur_n")
                    .map_err(llvm_err)?
                    .into_int_value();
                let q = self
                    .builder
                    .build_int_signed_div(cur_n, cur_pow, "q")
                    .map_err(llvm_err)?;
                let digit = self
                    .builder
                    .build_int_signed_rem(q, ten, "digit")
                    .map_err(llvm_err)?;
                // Build fat struct {digit, null} and push
                let undef = self.string_type.get_undef();
                let d1 = self
                    .builder
                    .build_insert_value(undef, digit, 0, "d1")
                    .map_err(llvm_err)?;
                let d2 = self
                    .builder
                    .build_insert_value(d1, self.ptr_ty().const_zero(), 1, "d2")
                    .map_err(llvm_err)?;
                let rl = self
                    .builder
                    .build_load(self.list_type, res_a, "rl")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let rp = self.call_rt(
                    "action_list_push",
                    &[rl.into(), d2.as_basic_value_enum().into()],
                )?;
                self.builder
                    .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
                    .map_err(llvm_err)?;
                // Advance: i++, pow10 /= 10
                let ndi = self
                    .builder
                    .build_int_add(div, one, "ndi")
                    .map_err(llvm_err)?;
                self.builder.build_store(di_a, ndi).map_err(llvm_err)?;
                let np10 = self
                    .builder
                    .build_int_signed_div(cur_pow, ten, "np10")
                    .map_err(llvm_err)?;
                self.builder.build_store(p10_a, np10).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(fill_hdr);
                self.builder.position_at_end(fill_ext);
                Ok(TypedValue::List(res_a))
            }
            "charAt" => {
                if args.len() != 2 {
                    return Err("charAt expects 2 arguments (string, index)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let idx = self.compile_expr(&args[1])?;
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
                // Allocate nbytes+1 (for null terminator)
                let alloc_sz = self
                    .builder
                    .build_int_add(nbytes, self.i64_ty().const_int(1, false), "alloc_sz")
                    .map_err(llvm_err)?;
                let malloc_fn = self.module.get_function("malloc").unwrap();
                let buf = self
                    .builder
                    .build_call(malloc_fn, &[alloc_sz.into()], "buf")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
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
                let v = self.compile_expr(&args[0])?;
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
                let v = self.compile_expr(&args[0])?;
                let code = match v {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("codeToChar: argument must be an int".to_string()),
                };
                let i64 = self.i64_ty();
                let i8 = self.context.i8_type();
                // Allocate 5 bytes (max 4 byte UTF-8 + null terminator)
                let malloc_fn = self.module.get_function("malloc").unwrap();
                let alloc_sz = i64.const_int(5, false);
                let buf = self
                    .builder
                    .build_call(malloc_fn, &[alloc_sz.into()], "buf")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
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
            "nowUtc" => {
                if !args.is_empty() {
                    return Err("nowUtc expects no arguments".to_string());
                }
                let sty = self.context.struct_type(&[self.i64_ty().into(); 6], false);
                let alloca = self.builder.build_alloca(sty, "nowUtc").map_err(llvm_err)?;
                let time_fn = self
                    .module
                    .get_function("time")
                    .ok_or("time function not found")?;
                let null_ptr = self.ptr_ty().const_null();
                let ts = self
                    .builder
                    .build_call(time_fn, &[null_ptr.into()], "ts")
                    .map_err(llvm_err)?;
                let ts_val = ts.try_as_basic_value().unwrap_basic().into_int_value();
                let gmtime_fn = self
                    .module
                    .get_function("gmtime_r")
                    .ok_or("gmtime_r function not found")?;
                let tm_ptr = self.builder.build_alloca(sty, "tm").map_err(llvm_err)?;
                let gmtime_call = self
                    .builder
                    .build_call(gmtime_fn, &[ts_val.into(), tm_ptr.into()], "")
                    .map_err(llvm_err)?;
                let _ = gmtime_call.try_as_basic_value().basic();
                // Copy tm struct to result (year+1900, month, day, hour, min, sec)
                for i in 0..6u32 {
                    let src_p = self
                        .builder
                        .build_struct_gep(sty, tm_ptr, i, "tm_f")
                        .map_err(llvm_err)?;
                    let val = self
                        .builder
                        .build_load(self.i64_ty(), src_p, "val")
                        .map_err(llvm_err)?;
                    let dst_p = self
                        .builder
                        .build_struct_gep(sty, alloca, i, "dst_f")
                        .map_err(llvm_err)?;
                    self.builder.build_store(dst_p, val).map_err(llvm_err)?;
                }
                // Fix year: tm_year is years since 1900
                let yp = self
                    .builder
                    .build_struct_gep(sty, alloca, 0, "yp")
                    .map_err(llvm_err)?;
                let yv = self
                    .builder
                    .build_load(self.i64_ty(), yp, "yv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let ya = self
                    .builder
                    .build_int_add(yv, self.i64_ty().const_int(1900, false), "ya")
                    .map_err(llvm_err)?;
                self.builder.build_store(yp, ya).map_err(llvm_err)?;
                Ok(TypedValue::Struct(alloca, sty))
            }
            "diffSeconds" => {
                if args.len() != 2 {
                    return Err("diffSeconds expects 2 arguments (dt1, dt2)".to_string());
                }
                let d1 = self.compile_expr(&args[0])?;
                let d2 = self.compile_expr(&args[1])?;
                let (p1, st1) = match d1 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffSeconds: arguments must be DateTime structs".to_string()),
                };
                let (p2, _st2) = match d2 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffSeconds: arguments must be DateTime structs".to_string()),
                };
                let i64_ty = self.i64_ty();
                // Approximate seconds from year/month/day/hour/min/sec
                let extract = |builder: &inkwell::builder::Builder<'ctx>,
                               p: PointerValue<'ctx>,
                               st: inkwell::types::StructType<'ctx>|
                 -> Result<IntValue<'ctx>, String> {
                    let yp = builder.build_struct_gep(st, p, 0, "yp").map_err(llvm_err)?;
                    let y = builder
                        .build_load(i64_ty, yp, "y")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let mp = builder.build_struct_gep(st, p, 1, "mp").map_err(llvm_err)?;
                    let m = builder
                        .build_load(i64_ty, mp, "m")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let dp = builder.build_struct_gep(st, p, 2, "dp").map_err(llvm_err)?;
                    let d = builder
                        .build_load(i64_ty, dp, "d")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let hp = builder.build_struct_gep(st, p, 3, "hp").map_err(llvm_err)?;
                    let h = builder
                        .build_load(i64_ty, hp, "h")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let minp = builder
                        .build_struct_gep(st, p, 4, "minp")
                        .map_err(llvm_err)?;
                    let minv = builder
                        .build_load(i64_ty, minp, "min")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let sp = builder.build_struct_gep(st, p, 5, "sp").map_err(llvm_err)?;
                    let s = builder
                        .build_load(i64_ty, sp, "s")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let d365 = builder
                        .build_int_mul(y, i64_ty.const_int(365, false), "d365")
                        .map_err(llvm_err)?;
                    let d30 = builder
                        .build_int_mul(m, i64_ty.const_int(30, false), "d30")
                        .map_err(llvm_err)?;
                    let days = builder
                        .build_int_add(
                            builder.build_int_add(d365, d30, "d1").map_err(llvm_err)?,
                            d,
                            "d2",
                        )
                        .map_err(llvm_err)?;
                    let secs_per_day = i64_ty.const_int(86400, false);
                    let ds = builder
                        .build_int_mul(days, secs_per_day, "ds")
                        .map_err(llvm_err)?;
                    let hs = builder
                        .build_int_mul(h, i64_ty.const_int(3600, false), "hs")
                        .map_err(llvm_err)?;
                    let ms = builder
                        .build_int_mul(minv, i64_ty.const_int(60, false), "ms")
                        .map_err(llvm_err)?;
                    let total = builder
                        .build_int_add(
                            builder
                                .build_int_add(
                                    builder.build_int_add(ds, hs, "t1").map_err(llvm_err)?,
                                    ms,
                                    "t2",
                                )
                                .map_err(llvm_err)?,
                            s,
                            "t3",
                        )
                        .map_err(llvm_err)?;
                    Ok(total)
                };
                let t1 = extract(&self.builder, p1, st1)?;
                let t2 = extract(&self.builder, p2, st1)?;
                let diff = self
                    .builder
                    .build_int_sub(t1, t2, "diff")
                    .map_err(llvm_err)?;
                // Absolute value
                let zero = self.i64_ty().const_int(0, false);
                let nd = self.builder.build_int_neg(diff, "nd").map_err(llvm_err)?;
                let is_neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, diff, zero, "is_neg")
                    .map_err(llvm_err)?;
                let abs_diff = self
                    .builder
                    .build_select(is_neg, nd, diff, "abs_diff")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::Int(abs_diff))
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
            "date" | "datetime" | "format" => self.builtin_stdlib_datetime(name, args),
            _ => Err(format!("Unknown builtin: {}", name)),
        }
    }

}
