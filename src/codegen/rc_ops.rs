// Submodule: rc_ops — reference counting operations
//
// Extracted from misc.rs.
//

use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::PointerValue;

use super::{llvm_err, CodeGen, TypedValue, ValKind};
use inkwell::values::{BasicMetadataValueEnum, IntValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn call_rt(
        &self,
        name: &str,
        args: &[BasicMetadataValueEnum<'ctx>],
    ) -> Result<inkwell::values::CallSiteValue<'ctx>, String> {
        let func = self
            .module
            .get_function(name)
            .ok_or_else(|| format!("Runtime fn '{}' not found", name))?;
        self.builder.build_call(func, args, "").map_err(llvm_err)
    }
    /// Allocate memory with a refcount header. Returns data pointer (ptr+8).
    pub(super) fn malloc_rc(&self, size: IntValue<'ctx>) -> Result<PointerValue<'ctx>, String> {
        let func = self
            .module
            .get_function("action_malloc_rc")
            .ok_or("action_malloc_rc not found")?;
        let result = self
            .builder
            .build_call(func, &[size.into()], "malloc_rc")
            .map_err(llvm_err)?;
        Ok(result
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value())
    }

    /// Increment refcount on a heap-allocated value.
    pub(super) fn rc_inc(&self, ptr: PointerValue<'ctx>) -> Result<(), String> {
        self.call_rt("action_rc_inc", &[ptr.into()])?;
        Ok(())
    }

    /// Decrement refcount on a heap-allocated value (frees if refcount reaches 0).
    pub(super) fn rc_dec(&self, ptr: PointerValue<'ctx>) -> Result<(), String> {
        self.call_rt("action_rc_dec", &[ptr.into()])?;
        Ok(())
    }

    /// Emit RC decrement for all heap-typed variables in the current scope.
    pub(super) fn emit_scope_cleanup(&self) -> Result<(), String> {
        for (_name, var) in self.scope.local_variables() {
            match var.kind {
                ValKind::Str => {
                    let str_val = self.load_string(var.ptr)?;
                    let data_ptr = self
                        .builder
                        .build_extract_value(str_val, 1, "data")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    self.rc_dec(data_ptr)?;
                }
                ValKind::List => {
                    let list_val = self.load_list(var.ptr)?;
                    let data_ptr = self
                        .builder
                        .build_extract_value(list_val, 0, "data")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    let height = self
                        .builder
                        .build_extract_value(list_val, 2, "height")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let rdl_fn = self.module.get_function("action_rc_dec_list_node").unwrap();
                    let _ = self
                        .builder
                        .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
                        .map_err(llvm_err)?;
                }
                ValKind::Map | ValKind::Set => {
                    let list_val = self.load_list(var.ptr)?;
                    let data_ptr = self
                        .builder
                        .build_extract_value(list_val, 0, "data")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    let height = self
                        .builder
                        .build_extract_value(list_val, 2, "height")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let rdl_fn = self.module.get_function("action_rc_dec_list_node").unwrap();
                    let _ = self
                        .builder
                        .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
                        .map_err(llvm_err)?;
                }
                ValKind::LazyList => {
                    // LazyList is stack-only, no heap data to clean up
                }
                ValKind::Stream => {
                    let stream_heap_ptr = self
                        .builder
                        .build_load(var.ty, var.ptr, "stream_cleanup_ptr")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    let stream_typed = self
                        .builder
                        .build_pointer_cast(stream_heap_ptr, self.ptr_ty(), "stream_typed")
                        .map_err(llvm_err)?;
                    let list_gep = self
                        .builder
                        .build_struct_gep(self.stream_type, stream_typed, 3, "slist_gep")
                        .map_err(llvm_err)?;
                    let list_val = self
                        .builder
                        .build_load(self.list_type, list_gep, "slist")
                        .map_err(llvm_err)?;
                    let data_ptr = self
                        .builder
                        .build_extract_value(list_val.into_struct_value(), 0, "sdata")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    self.rc_dec(data_ptr)?;
                }
                ValKind::Task => {
                    let task_heap_ptr = self
                        .builder
                        .build_load(self.ptr_ty(), var.ptr, "task_cleanup_ptr")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    let task_typed = self
                        .builder
                        .build_pointer_cast(task_heap_ptr, self.ptr_ty(), "task_typed")
                        .map_err(llvm_err)?;
                    let list_gep = self
                        .builder
                        .build_struct_gep(self.task_type, task_typed, 4, "tlist_gep")
                        .map_err(llvm_err)?;
                    let list_val = self
                        .builder
                        .build_load(self.list_type, list_gep, "tlist")
                        .map_err(llvm_err)?;
                    let data_ptr = self
                        .builder
                        .build_extract_value(list_val.into_struct_value(), 0, "tdata")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    self.rc_dec(data_ptr)?;
                }
                ValKind::Enum if var.enum_data_rc_managed => {
                    let loaded = self
                        .builder
                        .build_load(var.ty, var.ptr, "enum_cleanup")
                        .map_err(llvm_err)?;
                    let data_ptr = self
                        .builder
                        .build_extract_value(loaded.into_struct_value(), 1, "edata")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    self.rc_dec(data_ptr)?;
                }
                ValKind::Fn if var.is_closure => {
                    // Closure: the alloca stores a pointer to the captures struct.
                    // First rc_dec captured heap values inside, then rc_dec the struct.
                    let cap_ptr = self
                        .builder
                        .build_load(self.ptr_ty(), var.ptr, "closure_cleanup")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    if let Some(closure_ty) = var.closure_ty {
                        self.rc_dec_closure_captures(cap_ptr, closure_ty)?;
                    } else {
                        self.rc_dec(cap_ptr)?;
                    }
                }
                ValKind::Struct => {
                    // Struct has heap-typed fields stored inline; rc_dec each
                    if let BasicTypeEnum::StructType(st) = var.ty {
                        let loaded = self
                            .builder
                            .build_load(st, var.ptr, "struct_cleanup")
                            .map_err(llvm_err)?
                            .into_struct_value();
                        self.rc_struct_fields(loaded, st, false)?;
                    }
                }
                ValKind::Nullable => {
                    self.rc_nullable_inner(var.ptr, var.ty, false)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Decrement RC for a variable's old value before reassignment.
    pub(super) fn rc_dec_at(
        &self,
        ptr: PointerValue<'ctx>,
        kind: ValKind,
        ty: inkwell::types::BasicTypeEnum<'ctx>,
        rc_managed: bool,
    ) -> Result<(), String> {
        match kind {
            ValKind::Str => {
                let str_val = self.load_string(ptr)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(str_val, 1, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.rc_dec(data_ptr)?;
            }
            ValKind::List => {
                let list_val = self.load_list(ptr)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(list_val, 0, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let height = self
                    .builder
                    .build_extract_value(list_val, 2, "height")
                    .map_err(llvm_err)?
                    .into_int_value();
                let rdl_fn = self.module.get_function("action_rc_dec_list_node").unwrap();
                let _ = self
                    .builder
                    .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
                    .map_err(llvm_err)?;
            }
            ValKind::Map | ValKind::Set => {
                let list_val = self.load_list(ptr)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(list_val, 0, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let height = self
                    .builder
                    .build_extract_value(list_val, 2, "height")
                    .map_err(llvm_err)?
                    .into_int_value();
                let rdl_fn = self.module.get_function("action_rc_dec_list_node").unwrap();
                let _ = self
                    .builder
                    .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
                    .map_err(llvm_err)?;
            }
            ValKind::Enum if rc_managed => {
                let loaded = self
                    .builder
                    .build_load(ty, ptr, "enum_dec")
                    .map_err(llvm_err)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(loaded.into_struct_value(), 1, "edata")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.rc_dec(data_ptr)?;
            }
            ValKind::Struct => {
                if let BasicTypeEnum::StructType(st) = ty {
                    let loaded = self
                        .builder
                        .build_load(st, ptr, "struct_old_dec")
                        .map_err(llvm_err)?
                        .into_struct_value();
                    self.rc_struct_fields(loaded, st, false)?;
                }
            }
            ValKind::Nullable => {
                self.rc_nullable_inner(ptr, ty, false)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// RC-dec the old value at a struct field pointer before overwriting.
    /// The field's LLVM type determines how to extract and release heap pointers.
    pub(super) fn rc_dec_field_val(
        &self,
        field_ptr: PointerValue<'ctx>,
        field_type: inkwell::types::BasicTypeEnum<'ctx>,
    ) -> Result<(), String> {
        match field_type {
            BasicTypeEnum::StructType(ft_st) if ft_st == self.string_type => {
                let old = self
                    .builder
                    .build_load(ft_st, field_ptr, "fd_old")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let data_ptr = self
                    .builder
                    .build_extract_value(old, 1, "fd_data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.rc_dec(data_ptr)?;
            }
            BasicTypeEnum::StructType(ft_st) if ft_st == self.list_type => {
                let old = self
                    .builder
                    .build_load(ft_st, field_ptr, "fd_old")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let data_ptr = self
                    .builder
                    .build_extract_value(old, 0, "fd_data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let height = self
                    .builder
                    .build_extract_value(old, 2, "fd_height")
                    .map_err(llvm_err)?
                    .into_int_value();
                let rdl_fn = self
                    .module
                    .get_function("action_rc_dec_list_node")
                    .ok_or("action_rc_dec_list_node not found")?;
                self.builder
                    .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
                    .map_err(llvm_err)?;
            }
            _ => {} // scalar or user struct (Bug #1 handles recursive field RC)
        }
        Ok(())
    }

    /// Recursively rc_inc or rc_dec heap-typed fields of a struct (or sub-struct).
    pub(super) fn rc_struct_fields(
        &self,
        struct_val: inkwell::values::StructValue<'ctx>,
        struct_ty: StructType<'ctx>,
        inc: bool,
    ) -> Result<(), String> {
        for (i, field_type) in struct_ty.get_field_types().iter().enumerate() {
            let field = self
                .builder
                .build_extract_value(struct_val, i as u32, "rc_sf")
                .map_err(llvm_err)?;
            match field_type {
                BasicTypeEnum::StructType(ft_st) if *ft_st == self.string_type => {
                    let data_ptr = self
                        .builder
                        .build_extract_value(field.into_struct_value(), 1, "rc_sd")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    if inc {
                        self.rc_inc(data_ptr)?;
                    } else {
                        self.rc_dec(data_ptr)?;
                    }
                }
                BasicTypeEnum::StructType(ft_st) if *ft_st == self.list_type => {
                    let sv = field.into_struct_value();
                    let data_ptr = self
                        .builder
                        .build_extract_value(sv, 0, "rc_ld")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    if inc {
                        self.rc_inc(data_ptr)?;
                    } else {
                        let height = self
                            .builder
                            .build_extract_value(sv, 2, "rc_lh")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let rdl_fn = self.module.get_function("action_rc_dec_list_node").unwrap();
                        self.builder
                            .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
                            .map_err(llvm_err)?;
                    }
                }
                BasicTypeEnum::StructType(ft_st)
                    if *ft_st != self.string_type && *ft_st != self.list_type =>
                {
                    // Recursively handle nested user struct or nullable/enum types
                    self.rc_struct_fields(field.into_struct_value(), *ft_st, inc)?;
                }
                BasicTypeEnum::PointerType(_) => {
                    let inner_ptr = field.into_pointer_value();
                    if inc {
                        self.rc_inc(inner_ptr)?;
                    } else {
                        self.rc_dec(inner_ptr)?;
                    }
                }
                _ => {} // scalar
            }
        }
        Ok(())
    }

    /// RC-inc or RC-dec the inner value of a nullable, skipping the null case.
    /// Null nullables have zero-filled inners, and rc_inc/rc_dec are null-safe,
    /// so we skip the conditional branch on the null flag for simplicity.
    pub(super) fn rc_nullable_inner(
        &self,
        ptr: PointerValue<'ctx>,
        nullable_ty: inkwell::types::BasicTypeEnum<'ctx>,
        inc: bool,
    ) -> Result<(), String> {
        let st = nullable_ty.into_struct_type();
        let field_types = st.get_field_types();
        if field_types.len() < 2 {
            return Ok(());
        }
        let inner_ft = field_types[1];

        // Load the nullable struct and check the null flag before touching inner.
        let loaded = self
            .builder
            .build_load(st, ptr, "nul_ld")
            .map_err(llvm_err)?
            .into_struct_value();
        let null_flag = self
            .builder
            .build_extract_value(loaded, 0, "nul_flag")
            .map_err(llvm_err)?
            .into_int_value();

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("rc_nullable_inner: not in a function")?;
        let is_not_null = self
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                null_flag,
                null_flag.get_type().const_int(0, false),
                "nul_is_not_null",
            )
            .map_err(llvm_err)?;

        let process_bb = self.context.append_basic_block(current_fn, "nul_process");
        let merge_bb = self.context.append_basic_block(current_fn, "nul_merge");
        self.builder
            .build_conditional_branch(is_not_null, process_bb, merge_bb)
            .map_err(llvm_err)?;

        // Process inner value only when not null
        self.builder.position_at_end(process_bb);
        match inner_ft {
            BasicTypeEnum::StructType(inner_st) => {
                let inner = self
                    .builder
                    .build_extract_value(loaded, 1, "nul_inner")
                    .map_err(llvm_err)?
                    .into_struct_value();
                if inner_st == self.string_type {
                    let data_ptr = self
                        .builder
                        .build_extract_value(inner, 1, "nsd")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    if inc {
                        self.rc_inc(data_ptr)?;
                    } else {
                        self.rc_dec(data_ptr)?;
                    }
                } else if inner_st == self.list_type {
                    let data_ptr = self
                        .builder
                        .build_extract_value(inner, 0, "nld")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    if inc {
                        self.rc_inc(data_ptr)?;
                    } else {
                        let height = self
                            .builder
                            .build_extract_value(inner, 2, "nlh")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let rdl_fn = self.module.get_function("action_rc_dec_list_node").unwrap();
                        self.builder
                            .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
                            .map_err(llvm_err)?;
                    }
                } else {
                    self.rc_struct_fields(inner, inner_st, inc)?;
                }
            }
            BasicTypeEnum::PointerType(_) => {
                let inner_ptr = self
                    .builder
                    .build_extract_value(loaded, 1, "nul_inner_ptr")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                if inc {
                    self.rc_inc(inner_ptr)?;
                } else {
                    self.rc_dec(inner_ptr)?;
                }
            }
            _ => {} // scalar inners don't have heap data
        }
        self.builder
            .build_unconditional_branch(merge_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(merge_bb);
        Ok(())
    }

    /// Increment RC for a heap-typed value being bound to a variable.
    pub(super) fn rc_inc_typed_value(&self, val: &TypedValue<'ctx>) -> Result<(), String> {
        match val {
            TypedValue::Str(ptr) => {
                let str_val = self.load_string(*ptr)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(str_val, 1, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.rc_inc(data_ptr)?;
            }
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                let list_val = self.load_list(*ptr)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(list_val, 0, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.rc_inc(data_ptr)?;
            }
            TypedValue::LazyList(_) => {
                // LazyList is stack-only, no heap data to RC
            }
            TypedValue::Enum(alloca, enum_ty, _, true) => {
                let bt: BasicTypeEnum = (*enum_ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *alloca, "enum_rcinc")
                    .map_err(llvm_err)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(loaded.into_struct_value(), 1, "edata")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.rc_inc(data_ptr)?;
            }
            TypedValue::Closure { closure_ptr, .. } => {
                self.rc_inc(*closure_ptr)?;
            }
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "rc_struct_inc")
                    .map_err(llvm_err)?
                    .into_struct_value();
                self.rc_struct_fields(loaded, *st, true)?;
            }
            TypedValue::Nullable(ptr, ty) => {
                self.rc_nullable_inner(*ptr, *ty, true)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Decrement RC for a heap-typed value returned from a block expression.
    /// RC decrement all captured heap values inside a closure's captures struct,
    /// then rc_dec the captures struct itself.
    pub(super) fn rc_dec_closure_captures(
        &self,
        closure_ptr: PointerValue<'ctx>,
        closure_ty: StructType<'ctx>,
    ) -> Result<(), String> {
        let typed_ptr = self
            .builder
            .build_pointer_cast(
                closure_ptr,
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "cc_typed",
            )
            .map_err(llvm_err)?;
        let struct_val = self
            .builder
            .build_load(closure_ty, typed_ptr, "cc_val")
            .map_err(llvm_err)?
            .into_struct_value();
        for (i, field_type) in closure_ty.get_field_types().iter().enumerate() {
            let field = self
                .builder
                .build_extract_value(struct_val, i as u32, "cc_f")
                .map_err(llvm_err)?;
            match field_type {
                BasicTypeEnum::StructType(st) if *st == self.string_type => {
                    let data_ptr = self
                        .builder
                        .build_extract_value(field.into_struct_value(), 1, "cc_sd")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    self.rc_dec(data_ptr)?;
                }
                BasicTypeEnum::StructType(st) if *st == self.list_type => {
                    let sv = field.into_struct_value();
                    let data_ptr = self
                        .builder
                        .build_extract_value(sv, 0, "cc_ld")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    let height = self
                        .builder
                        .build_extract_value(sv, 2, "cc_lh")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let rdl_fn = self.module.get_function("action_rc_dec_list_node").unwrap();
                    let _ = self
                        .builder
                        .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
                        .map_err(llvm_err)?;
                }
                BasicTypeEnum::PointerType(_) => {
                    // Inner closure's captures struct pointer
                    let inner_ptr = field.into_pointer_value();
                    self.rc_dec(inner_ptr)?;
                }
                _ => {}
            }
        }
        self.rc_dec(closure_ptr)
    }

    /// Mirrors rc_inc_typed_value, used to balance compile_block's RC inc when
    /// the block result is discarded (e.g., used as a statement).
    pub(super) fn rc_dec_typed_value(&self, val: &TypedValue<'ctx>) -> Result<(), String> {
        match val {
            TypedValue::Str(ptr) => {
                let str_val = self.load_string(*ptr)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(str_val, 1, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.rc_dec(data_ptr)?;
            }
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                let list_val = self.load_list(*ptr)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(list_val, 0, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let height = self
                    .builder
                    .build_extract_value(list_val, 2, "height")
                    .map_err(llvm_err)?
                    .into_int_value();
                let rdl_fn = self
                    .module
                    .get_function("action_rc_dec_list_node")
                    .ok_or("action_rc_dec_list_node not found")?;
                let _ = self
                    .builder
                    .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
                    .map_err(llvm_err)?;
            }
            TypedValue::LazyList(_) => {}
            TypedValue::Enum(alloca, enum_ty, _, true) => {
                let bt: BasicTypeEnum = (*enum_ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *alloca, "enum_rcdec")
                    .map_err(llvm_err)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(loaded.into_struct_value(), 1, "edata")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.rc_dec(data_ptr)?;
            }
            TypedValue::Closure {
                closure_ptr,
                closure_ty,
                ..
            } => {
                self.rc_dec_closure_captures(*closure_ptr, *closure_ty)?;
            }
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "rc_struct_dec")
                    .map_err(llvm_err)?
                    .into_struct_value();
                self.rc_struct_fields(loaded, *st, false)?;
            }
            TypedValue::Nullable(ptr, ty) => {
                self.rc_nullable_inner(*ptr, *ty, false)?;
            }
            _ => {}
        }
        Ok(())
    }

    /// Free an intermediate heap-typed value that is not a scope variable.
    /// Uses rc_inc+rc_dec to safely release. For tree values (List/Map/Set) with RC=1,
    /// this keeps the node alive (1→2→1) — the final scope cleanup handles actual freeing.
    /// Non-tree heap values (String, etc.) start at RC=0, so rc_inc+rc_dec triggers 0→1→0→free.
    pub(super) fn rc_free_intermediate(&self, val: &TypedValue<'ctx>) -> Result<(), String> {
        if !self.is_scope_variable(val) {
            self.rc_inc_typed_value(val)?;
            self.rc_dec_typed_value(val)?;
        }
        Ok(())
    }

    /// Free a method receiver intermediate. Tree types (List/Map/Set/Enum) start at
    /// RC≥1 and use direct rc_dec (1→0→free). Other types (String with RC=0) use
    /// rc_inc+rc_dec (0→1→0→free). Only for method dispatch where the receiver is
    /// recompiled independently — NOT for function call argument cleanup.
    pub(super) fn rc_free_method_receiver(&self, val: &TypedValue<'ctx>) -> Result<(), String> {
        if self.is_scope_variable(val) {
            return Ok(());
        }
        match val {
            TypedValue::List(_)
            | TypedValue::Map(_)
            | TypedValue::Set(_)
            | TypedValue::Enum(..) => {
                self.rc_dec_typed_value(val)?;
            }
            _ => {
                self.rc_inc_typed_value(val)?;
                self.rc_dec_typed_value(val)?;
            }
        }
        Ok(())
    }

    /// Discard a value that is no longer needed (e.g., for-loop body return value).
    /// Handles both scope variables (compile_block rc_inc'd for protection) and
    /// intermediates (RC=0).
    pub(super) fn rc_discard_value(&self, val: &TypedValue<'ctx>) -> Result<(), String> {
        if self.block_did_rc_inc {
            // compile_block already added one extra RC to protect from scope cleanup;
            // undo that since the caller doesn't take ownership.
            self.rc_dec_typed_value(val)?;
        } else {
            // Intermediate with RC=0; rc_inc+rc_dec triggers the free path.
            self.rc_free_intermediate(val)?;
        }
        Ok(())
    }

    /// Check whether a TypedValue corresponds to a local variable in the current scope
    /// by comparing alloca pointers.
    pub(super) fn is_scope_variable(&self, val: &TypedValue<'ctx>) -> bool {
        let alloca: Option<PointerValue<'ctx>> = match val {
            TypedValue::Str(p)
            | TypedValue::List(p)
            | TypedValue::Map(p)
            | TypedValue::Set(p)
            | TypedValue::Task(p)
            | TypedValue::Stream(p)
            | TypedValue::LazyList(p)
            | TypedValue::CString(p)
            | TypedValue::FileHandle(p)
            | TypedValue::Ptr(p) => Some(*p),
            TypedValue::Struct(p, _) => Some(*p),
            TypedValue::Enum(p, _, _, _) => Some(*p),
            TypedValue::Nullable(p, _) => Some(*p),
            TypedValue::Fn(p, _) => Some(*p),
            TypedValue::Closure { alloca, .. } => *alloca,
            _ => None,
        };
        match alloca {
            Some(ptr) => self.scope.local_variables().values().any(|v| v.ptr == ptr),
            None => false,
        }
    }
}
