// Submodule: builtins_print

use inkwell::values::{BasicValue, BasicValueEnum};

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, InnerType, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_print(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.is_empty() {
            if name == "println" {
                let _ = self.call_rt("action_println", &[]);
            }
            return Ok(TypedValue::Unit);
        }
        let v = self.compile_call_arg(args[0])?;
        match &v {
            TypedValue::Int(_) | TypedValue::FallibleInt { .. } => {
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_int", &[bv.into()]);
                }
            }
            TypedValue::Float(_) | TypedValue::FallibleFloat { .. } => {
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_float", &[bv.into()]);
                }
            }
            TypedValue::Bool(_) => {
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_bool", &[bv.into()]);
                }
            }
            TypedValue::Str(ptr) | TypedValue::FallibleStr { val: ptr, .. } => {
                let _ = self.call_rt_with_str("action_print_string", *ptr);
            }
            TypedValue::Fn(_, _) | TypedValue::Closure { .. } => {
                /* print fn/closure pointer as int */
                if let Some(bv) = v.to_bv() {
                    if let BasicValueEnum::PointerValue(p) = bv {
                        let int_val = self
                            .builder
                            .build_ptr_to_int(p, self.i64_ty(), "fn_ptr_as_int")
                            .map_err(llvm_err)?;
                        let _ = self.call_rt("action_print_int", &[int_val.into()]);
                    }
                }
            }
            TypedValue::List(ptr) | TypedValue::Set(ptr) | TypedValue::Map(ptr) => {
                let list = self.load_list(*ptr)?;
                let _ = self.call_rt("action_list_print", &[list.into()]);
            }
            TypedValue::Task(ptr) => {
                let task_val = self
                    .builder
                    .build_load(self.task_type, *ptr, "print_task")
                    .map_err(llvm_err)?;
                let _ = self.call_rt("action_print_task", &[task_val.into()]);
            }
            TypedValue::Stream(ptr) => {
                // Stream is {mutex, cond, closed, list}; load list from field 3
                let list_field = self
                    .builder
                    .build_struct_gep(self.stream_type, *ptr, 3, "print_sl_field")
                    .map_err(llvm_err)?;
                let list_val = self
                    .builder
                    .build_load(self.list_type, list_field, "print_sl")
                    .map_err(llvm_err)?;
                let _ = self.call_rt("action_list_print", &[list_val.into()]);
            }
            TypedValue::LazyList(ptr) => {
                let list_val = self
                    .builder
                    .build_load(self.list_type, *ptr, "print_ll")
                    .map_err(llvm_err)?;
                let _ = self.call_rt("action_list_print", &[list_val.into()]);
            }
            TypedValue::CString(_p) | TypedValue::Ptr(_p) | TypedValue::FileHandle(_p) => {
                // Print pointer value as hex
                if let Some(bv) = v.to_bv() {
                    if let BasicValueEnum::PointerValue(p) = bv {
                        let int_val = self
                            .builder
                            .build_ptr_to_int(p, self.i64_ty(), "ptr_as_int")
                            .map_err(llvm_err)?;
                        let _ = self.call_rt("action_print_int", &[int_val.into()]);
                    }
                }
            }
            TypedValue::Struct(_, _) | TypedValue::FallibleStruct { .. } => {
                let _ = self.call_rt("action_print_struct", &[]);
            }
            TypedValue::Enum(ptr, _, inner_type, _) => {
                let enum_st = self
                    .context
                    .struct_type(&[self.i64_ty().into(), self.ptr_ty().into()], false);
                let loaded = self
                    .builder
                    .build_load(enum_st, *ptr, "print_enum_ld")
                    .map_err(llvm_err)?;
                if *inner_type == InnerType::Float {
                    let _ = self.call_rt("action_print_enum_float", &[loaded.into()]);
                } else {
                    let _ = self.call_rt("action_print_enum", &[loaded.into()]);
                }
            }
            TypedValue::FalliblePtr { val, .. } => {
                let _ = self.call_rt("action_print_int", &[val.as_basic_value_enum().into()]);
            }
            TypedValue::Unit => {}
        }
        if name == "println" {
            let _ = self.call_rt("action_println", &[]);
        }
        self.rc_free_intermediate(&v)?;
        Ok(TypedValue::Unit)
    }
}
