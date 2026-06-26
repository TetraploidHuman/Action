//! Expression codegen (R4-3).

use action_frontend::ast::*;
use inkwell::types::FunctionType;
use inkwell::values::{BasicValueEnum, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, InnerType, TypedValue, ValKind};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_literal(&mut self, lit: &Literal) -> Result<TypedValue<'ctx>, String> {
        match lit {
            Literal::Int(n) => Ok(TypedValue::Int(self.i64_ty().const_int(*n as u64, true))),
            Literal::Float(n) => Ok(TypedValue::Float(self.f64_ty().const_float(*n))),
            Literal::Bool(b) => Ok(TypedValue::Bool(
                self.bool_ty().const_int(if *b { 1 } else { 0 }, false),
            )),
            Literal::String(s) => self.compile_string_literal(s),
            Literal::Char(c) => Ok(TypedValue::Int(self.i64_ty().const_int(*c as u64, false))),
            Literal::Unit => Ok(TypedValue::Unit),
        }
    }

    pub(crate) fn compile_string_literal(&mut self, s: &str) -> Result<TypedValue<'ctx>, String> {
        let g = self
            .builder
            .build_global_string_ptr(s, ".str")
            .map_err(llvm_err)?;
        let len = self.i64_ty().const_int(s.len() as u64, false);
        let cc = self.call_rt(
            "action_string_create",
            &[g.as_pointer_value().into(), len.into()],
        )?;
        match cc.try_as_basic_value().basic() {
            Some(bv) => {
                // String is returned as {i64, i8*} struct by value.
                // Store it on the stack and use the pointer.
                let alloca = self
                    .builder
                    .build_alloca(self.string_type, "str_val")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, bv).map_err(llvm_err)?;
                Ok(TypedValue::Str(alloca))
            }
            None => Ok(TypedValue::Str(g.as_pointer_value())),
        }
    }

    pub(crate) fn compile_ident(&mut self, name: &str) -> Result<TypedValue<'ctx>, String> {
        // Check compile-time constants first
        if let Some(&(global_ptr, ty, kind)) = self.type_layout.consts.get(name) {
            let loaded = self
                .builder
                .build_load(ty, global_ptr, name)
                .map_err(llvm_err)?;
            match kind {
                ValKind::Str => {
                    // Const strings point to global data without an RC header.
                    // Copy to heap so RC operations (inc/dec) work on valid memory.
                    let str_struct = loaded.into_struct_value();
                    let data_ptr = self
                        .builder
                        .build_extract_value(str_struct, 1, "cdata")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    let len_val = self
                        .builder
                        .build_extract_value(str_struct, 0, "clen")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let len_plus1 = self
                        .builder
                        .build_int_add(len_val, self.i64_ty().const_int(1, false), "clen1")
                        .map_err(llvm_err)?;
                    let heap = self.malloc_rc(len_plus1)?;
                    // memcpy from global data to heap
                    self.builder
                        .build_memcpy(heap, 1, data_ptr, 1, len_plus1)
                        .map_err(llvm_err)?;
                    let str_ty = self.string_type;
                    let alloca = self
                        .builder
                        .build_alloca(str_ty, "const_str")
                        .map_err(llvm_err)?;
                    let len_field = self
                        .builder
                        .build_struct_gep(str_ty, alloca, 0, "cs_len")
                        .map_err(llvm_err)?;
                    self.builder
                        .build_store(len_field, len_val)
                        .map_err(llvm_err)?;
                    let ptr_field = self
                        .builder
                        .build_struct_gep(str_ty, alloca, 1, "cs_ptr")
                        .map_err(llvm_err)?;
                    self.builder
                        .build_store(ptr_field, heap)
                        .map_err(llvm_err)?;
                    return Ok(TypedValue::Str(alloca));
                }
                ValKind::List => {
                    let alloca = self
                        .builder
                        .build_alloca(ty, "const_list")
                        .map_err(llvm_err)?;
                    self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
                    return Ok(TypedValue::List(alloca));
                }
                ValKind::Map => {
                    let alloca = self
                        .builder
                        .build_alloca(ty, "const_map")
                        .map_err(llvm_err)?;
                    self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
                    return Ok(TypedValue::Map(alloca));
                }
                ValKind::Set => {
                    let alloca = self
                        .builder
                        .build_alloca(ty, "const_set")
                        .map_err(llvm_err)?;
                    self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
                    return Ok(TypedValue::Set(alloca));
                }
                ValKind::CString => {
                    if let BasicValueEnum::PointerValue(p) = loaded {
                        return Ok(TypedValue::CString(p));
                    }
                    return self.bv_to_typed(loaded);
                }
                ValKind::Ptr => {
                    if let BasicValueEnum::PointerValue(p) = loaded {
                        return Ok(TypedValue::Ptr(p));
                    }
                    return self.bv_to_typed(loaded);
                }
                ValKind::FileHandle => {
                    if let BasicValueEnum::PointerValue(p) = loaded {
                        return Ok(TypedValue::FileHandle(p));
                    }
                    return self.bv_to_typed(loaded);
                }
                _ => return self.bv_to_typed(loaded),
            }
        }
        // Check for lazy val first — extract data before borrowing self mutably
        let lazy_info: Option<(
            PointerValue<'ctx>,
            inkwell::types::BasicTypeEnum<'ctx>,
            ValKind,
            PointerValue<'ctx>,
            action_frontend::hir::HirExpr,
            Option<FunctionType<'ctx>>,
        )> = if let Some(var) = self.scope.get(name) {
            if let (Some(flag_ptr), Some(init_expr)) = (var.lazy_flag, var.lazy_init_expr.clone()) {
                Some((var.ptr, var.ty, var.kind, flag_ptr, init_expr, var.fn_type))
            } else {
                None
            }
        } else {
            None
        };

        if let Some((lazy_ptr, lazy_ty, lazy_kind, flag_ptr, init_expr, lazy_fn_type)) = lazy_info {
            let current_fn = self
                .builder
                .get_insert_block()
                .expect("Lazy init: no insert block")
                .get_parent()
                .expect("Lazy init: function has no parent");
            let init_block = self
                .context
                .append_basic_block(current_fn, &format!("lazy_init_{}", name));
            let merge_block = self
                .context
                .append_basic_block(current_fn, &format!("lazy_merge_{}", name));

            let flag_val = self
                .builder
                .build_load(self.bool_ty(), flag_ptr, "lazy_flag")
                .map_err(llvm_err)?;
            let is_init = self
                .builder
                .build_int_compare(
                    IntPredicate::NE,
                    flag_val.into_int_value(),
                    self.bool_ty().const_int(0, false),
                    "is_init",
                )
                .map_err(llvm_err)?;
            self.builder
                .build_conditional_branch(is_init, merge_block, init_block)
                .map_err(llvm_err)?;

            // Init block: evaluate initializer and store
            self.builder.position_at_end(init_block);
            let init_val = self.compile_hir_expr(&init_expr)?;
            self.store_typed_value(&init_val, lazy_ptr, lazy_ty)?;
            self.builder
                .build_store(flag_ptr, self.bool_ty().const_int(1, false))
                .map_err(llvm_err)?;
            self.builder
                .build_unconditional_branch(merge_block)
                .map_err(llvm_err)?;

            // Merge block: return the original lazy alloca so that
            // is_scope_variable() can recognize heap-typed values.
            self.builder.position_at_end(merge_block);

            return match lazy_kind {
                ValKind::Str => Ok(TypedValue::Str(lazy_ptr)),
                ValKind::Fn => {
                    let val = self
                        .builder
                        .build_load(lazy_ty, lazy_ptr, name)
                        .map_err(llvm_err)?;
                    if let BasicValueEnum::PointerValue(p) = val {
                        if let Some(ft) = lazy_fn_type {
                            return Ok(TypedValue::Fn(p, ft));
                        }
                    }
                    self.bv_to_typed(val)
                }
                ValKind::List => Ok(TypedValue::List(lazy_ptr)),
                ValKind::Map => Ok(TypedValue::Map(lazy_ptr)),
                ValKind::Set => Ok(TypedValue::Set(lazy_ptr)),
                ValKind::Task => {
                    let task_ptr = self
                        .builder
                        .build_load(self.ptr_ty(), lazy_ptr, "task_ld")
                        .map_err(llvm_err)?;
                    Ok(TypedValue::Task(task_ptr.into_pointer_value()))
                }
                ValKind::Stream => {
                    let stream_ptr = self
                        .builder
                        .build_load(self.ptr_ty(), lazy_ptr, "stream_ld")
                        .map_err(llvm_err)?;
                    Ok(TypedValue::Stream(stream_ptr.into_pointer_value()))
                }
                ValKind::LazyList => Ok(TypedValue::LazyList(lazy_ptr)),
                ValKind::CString => {
                    let val = self
                        .builder
                        .build_load(lazy_ty, lazy_ptr, name)
                        .map_err(llvm_err)?;
                    if let BasicValueEnum::PointerValue(p) = val {
                        Ok(TypedValue::CString(p))
                    } else {
                        self.bv_to_typed(val)
                    }
                }
                ValKind::Ptr => {
                    let val = self
                        .builder
                        .build_load(lazy_ty, lazy_ptr, name)
                        .map_err(llvm_err)?;
                    if let BasicValueEnum::PointerValue(p) = val {
                        Ok(TypedValue::Ptr(p))
                    } else {
                        self.bv_to_typed(val)
                    }
                }
                ValKind::FileHandle => {
                    let val = self
                        .builder
                        .build_load(lazy_ty, lazy_ptr, name)
                        .map_err(llvm_err)?;
                    if let BasicValueEnum::PointerValue(p) = val {
                        Ok(TypedValue::FileHandle(p))
                    } else {
                        self.bv_to_typed(val)
                    }
                }
                ValKind::Struct => {
                    let st = lazy_ty.into_struct_type();
                    Ok(TypedValue::Struct(lazy_ptr, st))
                }
                ValKind::Enum => {
                    let et = lazy_ty.into_struct_type();
                    let inner_type = self
                        .scope
                        .get(name)
                        .and_then(|v| v.enum_inner_type)
                        .unwrap_or(InnerType::Int);
                    let rc_managed = self
                        .scope
                        .get(name)
                        .map_or(false, |v| v.enum_data_rc_managed);
                    Ok(TypedValue::Enum(lazy_ptr, et, inner_type, rc_managed))
                }
                _ => {
                    let val = self
                        .builder
                        .build_load(lazy_ty, lazy_ptr, name)
                        .map_err(llvm_err)?;
                    self.bv_to_typed(val)
                }
            };
        }

        if let Some(var) = self.scope.get(name) {
            // For heap-typed variables, return the original scope alloca directly
            // so that is_scope_variable() can recognize them and compile_block's
            // rc_inc/emit_scope_cleanup can properly protect them.
            let kind = var.kind;

            match kind {
                ValKind::Str => {
                    return Ok(TypedValue::Str(var.ptr));
                }
                ValKind::Fn => {
                    let val = self
                        .builder
                        .build_load(var.ty, var.ptr, name)
                        .map_err(llvm_err)?;
                    if let BasicValueEnum::PointerValue(p) = val {
                        if var.is_closure {
                            if let (Some(ct), Some(cfp), Some(aft)) =
                                (var.closure_ty, var.closure_fn_ptr, var.actual_fn_type)
                            {
                                return Ok(TypedValue::Closure {
                                    fn_ptr: cfp,
                                    actual_fn_type: aft,
                                    closure_ptr: p,
                                    closure_ty: ct,
                                    alloca: Some(var.ptr),
                                    capture_ptr_rc_mask: var.closure_capture_ptr_rc_mask,
                                });
                            }
                            return Err(format!(
                                "Closure variable '{}' missing closure metadata",
                                name
                            ));
                        }
                        if let Some(ft) = var.fn_type {
                            return Ok(TypedValue::Fn(p, ft));
                        }
                        return Err(format!(
                            "Function variable '{}' has no type information (internal error: fn_type not preserved)",
                            name
                        ));
                    }
                    return Err(format!(
                        "Expected function pointer for '{}', got: {:?}",
                        name, val
                    ));
                }
                ValKind::List => {
                    return Ok(TypedValue::List(var.ptr));
                }
                ValKind::Map => {
                    return Ok(TypedValue::Map(var.ptr));
                }
                ValKind::Set => {
                    return Ok(TypedValue::Set(var.ptr));
                }
                ValKind::Task => {
                    let task_ptr = self
                        .builder
                        .build_load(self.ptr_ty(), var.ptr, "task_ld2")
                        .map_err(llvm_err)?;
                    return Ok(TypedValue::Task(task_ptr.into_pointer_value()));
                }
                ValKind::Stream => {
                    let stream_ptr = self
                        .builder
                        .build_load(self.ptr_ty(), var.ptr, "stream_ld2")
                        .map_err(llvm_err)?;
                    return Ok(TypedValue::Stream(stream_ptr.into_pointer_value()));
                }
                ValKind::LazyList => {
                    return Ok(TypedValue::LazyList(var.ptr));
                }
                ValKind::CString => {
                    let val = self
                        .builder
                        .build_load(var.ty, var.ptr, name)
                        .map_err(llvm_err)?;
                    if let BasicValueEnum::PointerValue(p) = val {
                        return Ok(TypedValue::CString(p));
                    }
                    return self.bv_to_typed(val);
                }
                ValKind::Ptr => {
                    let val = self
                        .builder
                        .build_load(var.ty, var.ptr, name)
                        .map_err(llvm_err)?;
                    if let BasicValueEnum::PointerValue(p) = val {
                        return Ok(TypedValue::Ptr(p));
                    }
                    return self.bv_to_typed(val);
                }
                ValKind::FileHandle => {
                    let val = self
                        .builder
                        .build_load(var.ty, var.ptr, name)
                        .map_err(llvm_err)?;
                    if let BasicValueEnum::PointerValue(p) = val {
                        return Ok(TypedValue::FileHandle(p));
                    }
                    return self.bv_to_typed(val);
                }
                ValKind::Struct => {
                    let st = var.ty.into_struct_type();
                    return Ok(TypedValue::Struct(var.ptr, st));
                }
                ValKind::Enum => {
                    let et = var.ty.into_struct_type();
                    let inner_type = var.enum_inner_type.unwrap_or(InnerType::Int);
                    let rc_managed = var.enum_data_rc_managed;
                    return Ok(TypedValue::Enum(var.ptr, et, inner_type, rc_managed));
                }
                _ => {
                    let val = self
                        .builder
                        .build_load(var.ty, var.ptr, name)
                        .map_err(llvm_err)?;
                    if val.is_struct_value() {
                        let alloca = self
                            .builder
                            .build_alloca(var.ty, "tmp_struct")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, val).map_err(llvm_err)?;
                        return Ok(TypedValue::Str(alloca));
                    }
                    self.bv_to_typed(val)
                }
            }
        } else if let Some(fn_val) = self.module.get_function(name) {
            let fn_ptr = fn_val.as_global_value().as_pointer_value();
            let fn_type = fn_val.get_type();
            return Ok(TypedValue::Fn(fn_ptr, fn_type));
        } else if let Some((enum_info, variant)) = self
            .registry
            .lookup_variant(name)
            .map(|(ei, vi)| (ei.clone(), vi.clone()))
        {
            if variant.params.is_empty() {
                // Unit variant: construct the enum value directly
                self.compile_enum_construct(&enum_info, &variant, &[])
            } else {
                Err(format!(
                    "Enum variant '{}' requires arguments (use the variant as a function call)",
                    name
                ))
            }
        } else if name == "pi" {
            let pi_val = self.f64_ty().const_float(std::f64::consts::PI);
            return Ok(TypedValue::Float(pi_val));
        } else if name == "e" {
            let e_val = self.f64_ty().const_float(std::f64::consts::E);
            return Ok(TypedValue::Float(e_val));
        } else {
            Err(format!("Undefined variable: {}", name))
        }
    }
}
