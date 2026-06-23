// Submodule: builtins_stdlib_collection/list_transform

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn collection_dispatch_list_transform(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
            "unique" => {
                if args.len() != 1 {
                    return Err("unique expects 1 argument (list)".to_string());
                }
                let v = self.compile_call_arg(args[0])?;
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
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("unique: argument must be a list".to_string()),
                }
            }
            "slice" => {
                if args.len() != 3 {
                    return Err("slice expects 3 arguments (collection, start, end)".to_string());
                }
                let coll_v = self.compile_call_arg(args[0])?;
                let start_v = self.compile_call_arg(args[1])?;
                let end_v = self.compile_call_arg(args[2])?;
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
                        Ok(Some(TypedValue::List(alloca)))
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
                        Ok(Some(TypedValue::Str(alloca)))
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
                let v = self.compile_call_arg(args[0])?;
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
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("flatten: argument must be a list".to_string()),
                }
            }
            "splitAt" => {
                if args.len() != 2 {
                    return Err("splitAt expects 2 arguments (list, index)".to_string());
                }
                let list_v = self.compile_call_arg(args[0])?;
                let idx_v = self.compile_call_arg(args[1])?;
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
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("splitAt: first argument must be a list, second Int".to_string()),
                }
            }
            "chunks" => {
                if args.len() != 2 {
                    return Err("chunks expects 2 arguments (list, size)".to_string());
                }
                let list_v = self.compile_call_arg(args[0])?;
                let size_v = self.compile_call_arg(args[1])?;
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
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("chunks: first argument must be a list, second Int".to_string()),
                }
            }
            "windows" => {
                if args.len() != 2 {
                    return Err("windows expects 2 arguments (list, size)".to_string());
                }
                let list_v = self.compile_call_arg(args[0])?;
                let size_v = self.compile_call_arg(args[1])?;
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
                        Ok(Some(TypedValue::List(alloca)))
                    }
                    _ => Err("windows: first argument must be a list, second Int".to_string()),
                }
            }
            _ => Ok(None),
        }
    }
}
