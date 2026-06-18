// Submodule: builtins_print

use action_frontend::ast::*;
use inkwell::values::{BasicValue, BasicValueEnum};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, InnerType, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn builtin_print(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.is_empty() {
            if name == "println" {
                let _ = self.call_rt("action_println", &[]);
            }
            return Ok(TypedValue::Unit);
        }
        let v = self.compile_expr(&args[0])?;
        match &v {
            TypedValue::Int(_) => {
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_int", &[bv.into()]);
                }
            }
            TypedValue::Float(_) => {
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_float", &[bv.into()]);
                }
            }
            TypedValue::Bool(_) => {
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_bool", &[bv.into()]);
                }
            }
            TypedValue::Str(ptr) => {
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
            TypedValue::Struct(_, _) => {
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
            TypedValue::Nullable(ptr, inner_bt) => {
                // Print nullable: check null flag, print "null" or inner value
                let loaded = self
                    .builder
                    .build_load(*inner_bt, *ptr, "print_null_ld")
                    .map_err(llvm_err)?;
                let nullable_struct = loaded.into_struct_value();
                let null_flag = self
                    .builder
                    .build_extract_value(nullable_struct, 0, "print_null_flag")
                    .map_err(llvm_err)?
                    .into_int_value();

                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("Cannot print outside function")?;
                let is_null = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        null_flag,
                        self.null_flag_ty().const_int(1, false),
                        "print_is_null",
                    )
                    .map_err(llvm_err)?;

                let null_block = self.context.append_basic_block(current_fn, "print_null");
                let val_block = self.context.append_basic_block(current_fn, "print_val");
                let merge_block = self.context.append_basic_block(current_fn, "print_merge");

                self.builder
                    .build_conditional_branch(is_null, null_block, val_block)
                    .map_err(llvm_err)?;

                // Print "null" using printf
                self.builder.position_at_end(null_block);
                if let Some(printf_fn) = self.module.get_function("printf") {
                    let null_str = self
                        .builder
                        .build_global_string_ptr("null", "null_str")
                        .map_err(llvm_err)?
                        .as_pointer_value();
                    let _ =
                        self.builder
                            .build_call(printf_fn, &[null_str.into()], "print_null_call");
                }
                self.builder
                    .build_unconditional_branch(merge_block)
                    .map_err(llvm_err)?;

                // Print inner value
                self.builder.position_at_end(val_block);
                let inner = self
                    .builder
                    .build_extract_value(nullable_struct, 1, "print_inner")
                    .map_err(llvm_err)?;
                let inner_typed = self.bv_to_typed(inner)?;
                match &inner_typed {
                    TypedValue::Int(v) => {
                        let _ = self.call_rt("action_print_int", &[v.as_basic_value_enum().into()]);
                    }
                    TypedValue::Float(v) => {
                        let _ =
                            self.call_rt("action_print_float", &[v.as_basic_value_enum().into()]);
                    }
                    TypedValue::Bool(v) => {
                        let _ =
                            self.call_rt("action_print_bool", &[v.as_basic_value_enum().into()]);
                    }
                    TypedValue::Str(v) => {
                        let _ = self.call_rt_with_str("action_print_string", *v);
                    }
                    _ => {
                        let bv = inner_typed
                            .to_bv()
                            .unwrap_or_else(|| self.i64_ty().const_int(0, false).into());
                        let _ = self.call_rt("action_print_int", &[bv.into()]);
                    }
                }
                self.builder
                    .build_unconditional_branch(merge_block)
                    .map_err(llvm_err)?;

                self.builder.position_at_end(merge_block);
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
