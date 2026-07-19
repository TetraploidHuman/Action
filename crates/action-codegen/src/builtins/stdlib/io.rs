// Submodule: builtins_stdlib_io — file and stream IO builtin functions
//
// Extracted from builtins_stdlib.rs.
//
// Submodule: builtins_stdlib

use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_stdlib_io(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "readLine" => {
                if !args.is_empty() {
                    return Err("readLine expects no arguments".to_string());
                }
                if self.module.get_function("action_read_line").is_none() {
                    self.emit_read_line_runtime()?;
                }
                self.compile_read_line_fallible()
            }
            // M42: process-local bootstrap session buffers (see host-rt/runtime_bs_buf.rs).
            "bsBufClear" => {
                if args.len() != 1 {
                    return Err("bsBufClear expects 1 argument (slot)".to_string());
                }
                let slot = self.compile_call_arg(args[0])?;
                match slot {
                    TypedValue::Int(sv) => {
                        let cc = self.call_rt("action_bs_buf_clear", &[sv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("bsBufClear failed")?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    _ => Err("bsBufClear: slot must be Int".to_string()),
                }
            }
            "bsBufAppend" => {
                if args.len() != 2 {
                    return Err("bsBufAppend expects 2 arguments (slot, content)".to_string());
                }
                let slot = self.compile_call_arg(args[0])?;
                let content = self.compile_call_arg(args[1])?;
                match (&slot, &content) {
                    (TypedValue::Int(sv), TypedValue::Str(cp)) => {
                        let cv = self.load_string(*cp)?;
                        let cc = self.call_rt("action_bs_buf_append", &[(*sv).into(), cv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("bsBufAppend failed")?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    _ => Err("bsBufAppend: (Int, String) required".to_string()),
                }
            }
            "bsBufSet" => {
                if args.len() != 2 {
                    return Err("bsBufSet expects 2 arguments (slot, content)".to_string());
                }
                let slot = self.compile_call_arg(args[0])?;
                let content = self.compile_call_arg(args[1])?;
                match (&slot, &content) {
                    (TypedValue::Int(sv), TypedValue::Str(cp)) => {
                        let cv = self.load_string(*cp)?;
                        let cc = self.call_rt("action_bs_buf_set", &[(*sv).into(), cv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("bsBufSet failed")?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    _ => Err("bsBufSet: (Int, String) required".to_string()),
                }
            }
            "bsBufGet" => {
                if args.len() != 1 {
                    return Err("bsBufGet expects 1 argument (slot)".to_string());
                }
                let slot = self.compile_call_arg(args[0])?;
                match slot {
                    TypedValue::Int(sv) => {
                        let cc = self.call_rt("action_bs_buf_get", &[sv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("bsBufGet failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "bs_buf")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("bsBufGet: slot must be Int".to_string()),
                }
            }
            // M45: process-local Int session slots (span / line-col).
            "bsIntSet" => {
                if args.len() != 2 {
                    return Err("bsIntSet expects 2 arguments (slot, value)".to_string());
                }
                let slot = self.compile_call_arg(args[0])?;
                let value = self.compile_call_arg(args[1])?;
                match (&slot, &value) {
                    (TypedValue::Int(sv), TypedValue::Int(vv)) => {
                        let cc = self.call_rt("action_bs_int_set", &[(*sv).into(), (*vv).into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("bsIntSet failed")?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    _ => Err("bsIntSet: (Int, Int) required".to_string()),
                }
            }
            "bsIntGet" => {
                if args.len() != 1 {
                    return Err("bsIntGet expects 1 argument (slot)".to_string());
                }
                let slot = self.compile_call_arg(args[0])?;
                match slot {
                    TypedValue::Int(sv) => {
                        let cc = self.call_rt("action_bs_int_get", &[sv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("bsIntGet failed")?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    _ => Err("bsIntGet: slot must be Int".to_string()),
                }
            }
            "readFile" => {
                if args.len() != 1 {
                    return Err("readFile expects 1 argument (path)".to_string());
                }
                let path = self.compile_call_arg(args[0])?;
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
                let path = self.compile_call_arg(args[0])?;
                let content = self.compile_call_arg(args[1])?;
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
                let path = self.compile_call_arg(args[0])?;
                let content = self.compile_call_arg(args[1])?;
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
                let path = self.compile_call_arg(args[0])?;
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
                let path = self.compile_call_arg(args[0])?;
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
                let path = self.compile_call_arg(args[0])?;
                let mode = self.compile_call_arg(args[1])?;
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
                let file = self.compile_call_arg(args[0])?;
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
                let file = self.compile_call_arg(args[0])?;
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
                self.compile_file_read_line_fallible(args[0])
            }
            "fileReadBytes" => {
                if args.len() != 2 {
                    return Err("fileReadBytes expects 2 arguments (file, size)".to_string());
                }
                let file = self.compile_call_arg(args[0])?;
                let size = self.compile_call_arg(args[1])?;
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
                let file = self.compile_call_arg(args[0])?;
                let data = self.compile_call_arg(args[1])?;
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
                let file = self.compile_call_arg(args[0])?;
                let data = self.compile_call_arg(args[1])?;
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
                let file = self.compile_call_arg(args[0])?;
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
                let file = self.compile_call_arg(args[0])?;
                let offset = self.compile_call_arg(args[1])?;
                let whence = self.compile_call_arg(args[2])?;
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
                let file = self.compile_call_arg(args[0])?;
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
            "readDir" => {
                if args.len() != 1 {
                    return Err("readDir expects 1 argument (path)".to_string());
                }
                if self.module.get_function("action_read_dir").is_none() {
                    self.emit_read_dir_runtime()?;
                }
                let v = self.compile_call_arg(args[0])?;
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
            _ => Err(format!("Unknown IO builtin: {}", name)),
        }
    }
}
