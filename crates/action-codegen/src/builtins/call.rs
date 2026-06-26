// Submodule: builtins_call

use action_frontend::ast::Type;
use action_frontend::builtin::UfcsReceiverKind;
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::IntPredicate;

use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// AST call compilation (test-only; production uses [`compile_call_hir`]).

    /// Perform an indirect function call through a TypedValue::Fn, TypedValue::Closure, or TypedValue::Int.

    /// Read-only List UFCS methods using the already-compiled receiver value.
    /// Returns `None` when `method` is not handled here.
    pub(crate) fn compile_list_readonly_ufcs(
        &mut self,
        lp: PointerValue<'ctx>,
        recv_val: &TypedValue<'ctx>,
        method: &str,
        args: &[crate::call_arg::CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let Some(def) = action_frontend::builtin::lookup_ufcs(UfcsReceiverKind::List, method)
        else {
            return Ok(None);
        };
        if !def.readonly {
            return Ok(None);
        }
        let lv = self.load_list(lp)?;
        let zero = self.i64_ty().const_int(0, false);
        match method {
            "len" => {
                let len = self.list_len_val(lv)?;
                Ok(Some(TypedValue::Int(len)))
            }
            "isEmpty" => {
                let len = self.list_len_val(lv)?;
                let is_empty = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                    .map_err(llvm_err)?;
                Ok(Some(TypedValue::Bool(is_empty)))
            }
            "head" => {
                if !args.is_empty() {
                    return Err("list.head expects 0 arguments".to_string());
                }
                self.rc_free_intermediate(recv_val)?;
                self.compile_head_fallible_on_list_ptr(lp).map(Some)
            }
            "tail" => {
                if !args.is_empty() {
                    return Err("list.tail expects 0 arguments".to_string());
                }
                let len = self.list_len_val(lv)?;
                let is_empty = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                    .map_err(llvm_err)?;
                let cc = self.call_rt("action_list_tail", &[lv.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("tail failed")?
                    .into_struct_value();
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "ufcs_tail")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, result).map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                self.build_fallible_list_from_not_empty(alloca, is_empty).map(Some)
            }
            "get" => {
                if args.len() != 1 {
                    return Err("list.get expects 1 argument".to_string());
                }
                let idx_val = self.compile_call_arg(args[0])?;
                let iv = match idx_val {
                    TypedValue::Int(v) => v,
                    _ => return Err("list.get: index must be Int".to_string()),
                };
                self.rc_free_intermediate(recv_val)?;
                self.compile_list_get_fallible_on_ptr(lp, iv).map(Some)
            }
            "contains" => {
                if args.len() != 1 {
                    return Err("list.contains expects 1 argument".to_string());
                }
                let elem_val = self.compile_call_arg(args[0])?;
                let fat = self.to_fat_struct(&elem_val)?;
                let cc = self.call_rt("action_list_contains", &[lv.into(), fat.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("contains failed")?
                    .into_int_value();
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::Bool(result)))
            }
            "indexOf" => {
                if args.len() != 1 {
                    return Err("list.indexOf expects 1 argument".to_string());
                }
                let elem_val = self.compile_call_arg(args[0])?;
                let fat = self.to_fat_struct(&elem_val)?;
                let cc = self.call_rt("action_list_index_of", &[lv.into(), fat.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("indexOf failed")?
                    .into_int_value();
                let found = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, result, zero, "found")
                    .map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                self.build_fallible_int_from_ok(result, found).map(Some)
            }
            "last" => {
                if !args.is_empty() {
                    return Err("list.last expects 0 arguments".to_string());
                }
                self.rc_free_intermediate(recv_val)?;
                self.compile_last_fallible_on_list_ptr(lp).map(Some)
            }
            "reverse" => {
                if !args.is_empty() {
                    return Err("list.reverse expects 0 arguments".to_string());
                }
                let cc = self.call_rt("action_list_reverse", &[lv.into()])?;
                let result = cc.try_as_basic_value().basic().ok_or("reverse failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "ufcs_rev")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, result).map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::List(alloca)))
            }
            "sum" => {
                if !args.is_empty() {
                    return Err("list.sum expects 0 arguments".to_string());
                }
                let result = self.list_sum_from_loaded(lv)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::Int(result)))
            }
            "withIndex" => {
                if !args.is_empty() {
                    return Err("list.withIndex expects 0 arguments".to_string());
                }
                let cc = self.call_rt("action_list_with_index", &[lv.into()])?;
                let result = cc.try_as_basic_value().basic().ok_or("withIndex failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "ufcs_wi")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, result).map_err(llvm_err)?;
                self.rc_free_intermediate(recv_val)?;
                Ok(Some(TypedValue::List(alloca)))
            }
            _ => Ok(None),
        }
    }

    /// Callback List UFCS (any/all/find/findIndex) using compiled receiver — no rc_free + recompile.
    pub(crate) fn compile_list_callback_ufcs(
        &mut self,
        lp: PointerValue<'ctx>,
        method: &str,
        args: &[crate::call_arg::CallArg<'_>],
        trailing: Option<crate::call_arg::CallArg<'_>>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let fn_val = if let Some(lam) = trailing {
            self.compile_call_arg(lam)?
        } else if args.len() == 1 {
            self.compile_call_arg(args[0])?
        } else {
            return Ok(None);
        };
        match method {
            "any" => {
                let fn_ptr = self.callback_fn_ptr(&fn_val, "any")?;
                let lv = self.load_list(lp)?;
                let cc = self.call_rt("action_list_any_walk", &[lv.into(), fn_ptr.into()])?;
                let res = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("any_walk failed")?
                    .into_int_value();
                Ok(Some(TypedValue::Bool(res)))
            }
            "all" => {
                let fn_ptr = self.callback_fn_ptr(&fn_val, "all")?;
                let lv = self.load_list(lp)?;
                let cc = self.call_rt("action_list_all_walk", &[lv.into(), fn_ptr.into()])?;
                let res = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("all_walk failed")?
                    .into_int_value();
                Ok(Some(TypedValue::Bool(res)))
            }
            "find" => Ok(Some(self.find_on_list_ptr(
                lp,
                fn_val,
                &Type::Named("Int".into()),
            )?)),
            "findIndex" => Ok(Some(self.find_index_on_list_ptr(lp, fn_val)?)),
            _ => Ok(None),
        }
    }

    /// Convert a TypedValue to a BasicValueEnum suitable for passing as a
    /// function call argument, without re-compiling the expression.
    pub(crate) fn typed_value_to_bv(&self, av: &TypedValue<'ctx>) -> BasicValueEnum<'ctx> {
        av.to_bv().unwrap_or_else(|| match av {
            TypedValue::Str(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.string_type, *ptr, "arg_str")
                    .unwrap();
                ld.into()
            }
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.list_type, *ptr, "arg_list")
                    .unwrap();
                ld.into()
            }
            TypedValue::LazyList(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "arg_ll")
                    .unwrap();
                ld.into()
            }
            TypedValue::Task(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.task_type, *ptr, "arg_task")
                    .unwrap();
                ld.into()
            }
            TypedValue::Stream(ptr) => {
                let lf = self
                    .builder
                    .build_struct_gep(self.stream_type, *ptr, 3, "arg_slf")
                    .unwrap();
                let ld = self
                    .builder
                    .build_load(self.list_type, lf, "arg_sl")
                    .unwrap();
                ld.into()
            }
            TypedValue::Struct(ptr, st) => {
                let ld = self.builder.build_load(*st, *ptr, "arg_struct").unwrap();
                ld.into()
            }
            TypedValue::Enum(ptr, et, ..) => {
                let ld = self.builder.build_load(*et, *ptr, "arg_enum").unwrap();
                ld.into()
            }
            TypedValue::CString(p) | TypedValue::Ptr(p) | TypedValue::FileHandle(p) => (*p).into(),
            _ => self.i64_ty().const_int(0, false).into(),
        })
    }
}
