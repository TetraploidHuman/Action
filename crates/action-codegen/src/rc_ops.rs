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

    pub(super) fn rc_inc_string_val(
        &self,
        str_val: inkwell::values::StructValue<'ctx>,
    ) -> Result<(), String> {
        self.call_rt("action_string_rc_inc", &[str_val.into()])?;
        Ok(())
    }

    pub(super) fn rc_dec_string_val(
        &self,
        str_val: inkwell::values::StructValue<'ctx>,
    ) -> Result<(), String> {
        self.call_rt("action_string_rc_dec", &[str_val.into()])?;
        Ok(())
    }

    /// Decrement refcount on a heap-allocated value (frees if refcount reaches 0).
    pub(super) fn rc_dec(&self, ptr: PointerValue<'ctx>) -> Result<(), String> {
        self.call_rt("action_rc_dec", &[ptr.into()])?;
        Ok(())
    }

    /// Release one list-variable reference to `data_ptr` (root node).
    /// When root RC>1 only drops the ref; when RC==1 recursively frees the tree.
    pub(super) fn emit_rc_release_list_root(
        &self,
        data_ptr: PointerValue<'ctx>,
        height: inkwell::values::IntValue<'ctx>,
    ) -> Result<(), String> {
        use inkwell::IntPredicate;
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let fn_val = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("not in fn")?;
        let rc_p = self
            .builder
            .build_int_to_ptr(
                self.builder
                    .build_int_sub(
                        self.builder
                            .build_ptr_to_int(data_ptr, i64, "lr_pi")
                            .map_err(llvm_err)?,
                        i64.const_int(8, false),
                        "lr_rc_a",
                    )
                    .map_err(llvm_err)?,
                ptr,
                "lr_rc_p",
            )
            .map_err(llvm_err)?;
        let rc = self
            .builder
            .build_load(i64, rc_p, "lr_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let shared = self
            .builder
            .build_int_compare(IntPredicate::SGT, rc, i64.const_int(1, false), "lr_sh")
            .map_err(llvm_err)?;
        let dec_only_bb = self.context.append_basic_block(fn_val, "lr_dec");
        let dec_tree_bb = self.context.append_basic_block(fn_val, "lr_tree");
        let done_bb = self.context.append_basic_block(fn_val, "lr_done");
        let _ = self
            .builder
            .build_conditional_branch(shared, dec_only_bb, dec_tree_bb);
        self.builder.position_at_end(dec_only_bb);
        self.rc_dec(data_ptr)?;
        let _ = self.builder.build_unconditional_branch(done_bb);
        self.builder.position_at_end(dec_tree_bb);
        let rdl_fn = self
            .module
            .get_function("action_rc_dec_list_node")
            .ok_or("action_rc_dec_list_node not found")?;
        let _ = self
            .builder
            .build_call(rdl_fn, &[data_ptr.into(), height.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(done_bb);
        self.builder.position_at_end(done_bb);
        Ok(())
    }

    /// Emit RC decrement for all heap-typed variables in the current scope.
    pub(super) fn emit_scope_cleanup(&self) -> Result<(), String> {
        let mut vars: Vec<_> = self.scope.local_variables().iter().collect();
        vars.sort_by(|(a, _), (b, _)| a.cmp(b));
        let mut peer_cap_ptrs: Vec<PointerValue<'ctx>> = Vec::new();
        for (_, var) in &vars {
            if var.kind == ValKind::Fn && var.is_closure {
                let cap_ptr = self
                    .builder
                    .build_load(self.ptr_ty(), var.ptr, "peer_cap")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                peer_cap_ptrs.push(cap_ptr);
            }
        }
        let mut list_vars: Vec<_> = vars
            .iter()
            .filter(|(_, v)| v.kind == ValKind::List)
            .copied()
            .collect();
        list_vars.sort_by(|(a, _), (b, _)| b.cmp(a)); // derived bindings (e.g. ins) before originals (lst)
        for (_name, var) in vars
            .iter()
            .copied()
            .filter(|(_, v)| v.kind != ValKind::List)
        {
            self.emit_scope_cleanup_var(var, &peer_cap_ptrs)?;
        }
        for (_, var) in list_vars {
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
            self.emit_rc_release_list_root(data_ptr, height)?;
        }
        Ok(())
    }

    fn emit_scope_cleanup_var(
        &self,
        var: &super::ScopeVar<'ctx>,
        peer_cap_ptrs: &[PointerValue<'ctx>],
    ) -> Result<(), String> {
        match var.kind {
            ValKind::Str => {
                let str_val = self.load_string(var.ptr)?;
                self.rc_dec_string_val(str_val)?;
            }
            ValKind::Map | ValKind::Set => {
                let list_val = self.load_list(var.ptr)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(list_val, 0, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let len = self
                    .builder
                    .build_extract_value(list_val, 1, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cap = self
                    .builder
                    .build_extract_value(list_val, 2, "cap")
                    .map_err(llvm_err)?
                    .into_int_value();
                let rht_fn = self.module.get_function("action_rc_dec_ht").unwrap();
                let _ = self
                    .builder
                    .build_call(rht_fn, &[data_ptr.into(), cap.into(), len.into()], "")
                    .map_err(llvm_err)?;
            }
            ValKind::LazyList => {}
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
                let cap_ptr = self
                    .builder
                    .build_load(self.ptr_ty(), var.ptr, "closure_cleanup")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                if let Some(closure_ty) = var.closure_ty {
                    self.rc_dec_closure_captures(
                        cap_ptr,
                        closure_ty,
                        var.closure_capture_ptr_rc_mask,
                        peer_cap_ptrs,
                    )?;
                } else {
                    self.rc_dec(cap_ptr)?;
                }
            }
            ValKind::Struct => {
                if let BasicTypeEnum::StructType(st) = var.ty {
                    let loaded = self
                        .builder
                        .build_load(st, var.ptr, "struct_cleanup")
                        .map_err(llvm_err)?
                        .into_struct_value();
                    self.rc_struct_fields(loaded, st, false)?;
                }
            }
            _ => {}
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
                self.rc_dec_string_val(str_val)?;
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
                let len = self
                    .builder
                    .build_extract_value(list_val, 1, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cap = self
                    .builder
                    .build_extract_value(list_val, 2, "cap")
                    .map_err(llvm_err)?
                    .into_int_value();
                let rht_fn = self.module.get_function("action_rc_dec_ht").unwrap();
                let _ = self
                    .builder
                    .build_call(rht_fn, &[data_ptr.into(), cap.into(), len.into()], "")
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
        field_kind: ValKind,
    ) -> Result<(), String> {
        match field_type {
            BasicTypeEnum::StructType(ft_st) if ft_st == self.string_type => {
                let old = self
                    .builder
                    .build_load(ft_st, field_ptr, "fd_old")
                    .map_err(llvm_err)?
                    .into_struct_value();
                self.rc_dec_string_val(old)?;
            }
            BasicTypeEnum::StructType(ft_st) if ft_st == self.list_type => {
                let old = self
                    .builder
                    .build_load(ft_st, field_ptr, "fd_old")
                    .map_err(llvm_err)?
                    .into_struct_value();
                self.rc_dec_heap_collection(old, field_kind)?;
            }
            _ => {} // scalar or user struct (Bug #1 handles recursive field RC)
        }
        Ok(())
    }

    /// Release a List/Map/Set struct value using the correct runtime dec path.
    pub(super) fn rc_dec_heap_collection(
        &self,
        loaded: inkwell::values::StructValue<'ctx>,
        kind: ValKind,
    ) -> Result<(), String> {
        let data_ptr = self
            .builder
            .build_extract_value(loaded, 0, "hdc_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        match kind {
            ValKind::Map | ValKind::Set => {
                let len = self
                    .builder
                    .build_extract_value(loaded, 1, "hdc_len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cap = self
                    .builder
                    .build_extract_value(loaded, 2, "hdc_cap")
                    .map_err(llvm_err)?
                    .into_int_value();
                let rht_fn = self
                    .module
                    .get_function("action_rc_dec_ht")
                    .ok_or("action_rc_dec_ht not found")?;
                self.builder
                    .build_call(rht_fn, &[data_ptr.into(), cap.into(), len.into()], "")
                    .map_err(llvm_err)?;
            }
            _ => {
                let height = self
                    .builder
                    .build_extract_value(loaded, 2, "hdc_h")
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
                    let sv = field.into_struct_value();
                    if inc {
                        self.rc_inc_string_val(sv)?;
                    } else {
                        self.rc_dec_string_val(sv)?;
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
                        let field_kind = self.struct_field_val_kind(&struct_ty, i as u32);
                        self.rc_dec_heap_collection(sv, field_kind)?;
                    }
                }
                BasicTypeEnum::StructType(ft_st)
                    if *ft_st != self.string_type && *ft_st != self.list_type =>
                {
                    // Recursively handle nested user struct or enum types
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

    /// Increment RC for a heap-typed value being bound to a variable.
    pub(super) fn rc_inc_typed_value(&self, val: &TypedValue<'ctx>) -> Result<(), String> {
        match val {
            TypedValue::Str(ptr) => {
                let str_val = self.load_string(*ptr)?;
                self.rc_inc_string_val(str_val)?;
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

            _ => {}
        }
        Ok(())
    }

    /// RC decrement all captured heap values inside a closure's captures struct,
    /// then rc_dec the captures struct itself.
    ///
    /// `capture_ptr_rc_mask`: bit i set when capture field i is an RC-managed closure pointer
    /// (plain fn pointers are stored in pointer fields too but must not be rc_dec'd).
    /// `peer_cap_ptrs`: closure cap pointers of sibling bindings in the same scope; skip dec
    /// on those to avoid double-free when two closures capture each other.
    pub(super) fn rc_dec_closure_captures(
        &self,
        closure_ptr: PointerValue<'ctx>,
        closure_ty: StructType<'ctx>,
        capture_ptr_rc_mask: u64,
        peer_cap_ptrs: &[PointerValue<'ctx>],
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
                    self.rc_dec_string_val(field.into_struct_value())?;
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
                    if capture_ptr_rc_mask & (1u64 << i) == 0 {
                        continue;
                    }
                    let inner_ptr = field.into_pointer_value();
                    if peer_cap_ptrs.is_empty() {
                        self.rc_dec(inner_ptr)?;
                    } else {
                        let is_peer =
                            self.emit_closure_capture_is_peer(inner_ptr, peer_cap_ptrs)?;
                        let cur_bb = self
                            .builder
                            .get_insert_block()
                            .ok_or("rc_dec_closure_captures: no insert block")?;
                        let fn_val = cur_bb
                            .get_parent()
                            .ok_or("rc_dec_closure_captures: no parent fn")?;
                        let dec_bb = self.context.append_basic_block(fn_val, "cap_dec");
                        let after_bb = self.context.append_basic_block(fn_val, "cap_after");
                        self.builder
                            .build_conditional_branch(is_peer, after_bb, dec_bb)
                            .map_err(llvm_err)?;
                        self.builder.position_at_end(dec_bb);
                        self.rc_dec(inner_ptr)?;
                        self.builder
                            .build_unconditional_branch(after_bb)
                            .map_err(llvm_err)?;
                        self.builder.position_at_end(after_bb);
                    }
                }
                _ => {}
            }
        }
        self.rc_dec(closure_ptr)
    }

    /// Runtime branch: true when `inner_ptr` equals any pointer in `peer_cap_ptrs`.
    fn emit_closure_capture_is_peer(
        &self,
        inner_ptr: PointerValue<'ctx>,
        peer_cap_ptrs: &[PointerValue<'ctx>],
    ) -> Result<inkwell::values::IntValue<'ctx>, String> {
        if peer_cap_ptrs.is_empty() {
            return Ok(self.context.bool_type().const_int(0, false));
        }
        use inkwell::IntPredicate;
        let mut is_peer = self.context.bool_type().const_int(0, false);
        for peer in peer_cap_ptrs {
            let diff = self
                .builder
                .build_ptr_diff(self.context.i8_type(), inner_ptr, *peer, "cap_peer_diff")
                .map_err(llvm_err)?;
            let zero = self.i64_ty().const_int(0, false);
            let match_peer = self
                .builder
                .build_int_compare(IntPredicate::EQ, diff, zero, "cap_peer_eq")
                .map_err(llvm_err)?;
            is_peer = self
                .builder
                .build_or(is_peer, match_peer, "cap_peer_or")
                .map_err(llvm_err)?;
        }
        Ok(is_peer)
    }

    /// Mirrors rc_inc_typed_value, used to balance compile_block's RC inc when
    /// the block result is discarded (e.g., used as a statement).
    pub(super) fn rc_dec_typed_value(&self, val: &TypedValue<'ctx>) -> Result<(), String> {
        match val {
            TypedValue::Str(ptr) => {
                let str_val = self.load_string(*ptr)?;
                self.rc_dec_string_val(str_val)?;
            }
            TypedValue::List(ptr) => {
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
            TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                let list_val = self.load_list(*ptr)?;
                let data_ptr = self
                    .builder
                    .build_extract_value(list_val, 0, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let len = self
                    .builder
                    .build_extract_value(list_val, 1, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cap = self
                    .builder
                    .build_extract_value(list_val, 2, "cap")
                    .map_err(llvm_err)?
                    .into_int_value();
                let rht_fn = self
                    .module
                    .get_function("action_rc_dec_ht")
                    .ok_or("action_rc_dec_ht not found")?;
                let _ = self
                    .builder
                    .build_call(rht_fn, &[data_ptr.into(), cap.into(), len.into()], "")
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
                capture_ptr_rc_mask,
                ..
            } => {
                self.rc_dec_closure_captures(
                    *closure_ptr,
                    *closure_ty,
                    *capture_ptr_rc_mask,
                    &[],
                )?;
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

            _ => {}
        }
        Ok(())
    }

    /// True when another binding in the scope chain holds the same collection heap root.
    pub(super) fn collection_root_shared_in_scope(
        &self,
        exclude_ptr: PointerValue<'ctx>,
        old_data_ptr: PointerValue<'ctx>,
    ) -> Result<inkwell::values::IntValue<'ctx>, String> {
        use inkwell::IntPredicate;
        let i64 = self.i64_ty();
        let mut acc = self.context.bool_type().const_int(0, false);
        let mut scope = &self.scope;
        let old_i = self
            .builder
            .build_ptr_to_int(old_data_ptr, i64, "sh_old_i")
            .map_err(llvm_err)?;
        loop {
            for var in scope.local_variables().values() {
                if var.ptr == exclude_ptr {
                    continue;
                }
                if !matches!(var.kind, ValKind::List | ValKind::Map | ValKind::Set) {
                    continue;
                }
                let lv = self.load_list(var.ptr)?;
                let dp = self
                    .builder
                    .build_extract_value(lv, 0, "sh_dp")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let dp_i = self
                    .builder
                    .build_ptr_to_int(dp, i64, "sh_dp_i")
                    .map_err(llvm_err)?;
                let eq = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, dp_i, old_i, "sh_eq")
                    .map_err(llvm_err)?;
                acc = self.builder.build_or(acc, eq, "sh_or").map_err(llvm_err)?;
            }
            match &scope.parent {
                Some(p) => scope = p.as_ref(),
                None => break,
            }
        }
        Ok(acc)
    }

    /// Release old list on assign when the incoming value may share subtree nodes
    /// with the previous root (e.g. `lst = lst.insert(...)` after split_child).
    pub(super) fn emit_rc_release_list_on_assign(
        &self,
        exclude_ptr: PointerValue<'ctx>,
        old_data_ptr: PointerValue<'ctx>,
        old_height: IntValue<'ctx>,
        new_data_ptr: PointerValue<'ctx>,
        new_height: IntValue<'ctx>,
    ) -> Result<(), String> {
        const MAX_LIVE: usize = 8;
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let mut live: Vec<(PointerValue<'ctx>, IntValue<'ctx>)> = vec![(new_data_ptr, new_height)];
        let mut scope = &self.scope;
        loop {
            for var in scope.local_variables().values() {
                if var.ptr == exclude_ptr || var.kind != ValKind::List {
                    continue;
                }
                if live.len() >= MAX_LIVE {
                    break;
                }
                let lv = self.load_list(var.ptr)?;
                let dp = self
                    .builder
                    .build_extract_value(lv, 0, "la_dp")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let h = self
                    .builder
                    .build_extract_value(lv, 2, "la_h")
                    .map_err(llvm_err)?
                    .into_int_value();
                live.push((dp, h));
            }
            if live.len() >= MAX_LIVE {
                break;
            }
            match &scope.parent {
                Some(p) => scope = p.as_ref(),
                None => break,
            }
        }
        if live.is_empty() {
            return self.emit_rc_release_list_root(old_data_ptr, old_height);
        }
        let nodes_arr = self
            .builder
            .build_array_alloca(ptr, i64.const_int(MAX_LIVE as u64, false), "la_nodes")
            .map_err(llvm_err)?;
        let hs_arr = self
            .builder
            .build_array_alloca(i64, i64.const_int(MAX_LIVE as u64, false), "la_hs")
            .map_err(llvm_err)?;
        for i in 0..MAX_LIVE {
            let dp = if i < live.len() {
                live[i].0
            } else {
                ptr.const_null()
            };
            let h = if i < live.len() {
                live[i].1
            } else {
                i64.const_int(0, false)
            };
            let np = unsafe {
                self.builder
                    .build_gep(ptr, nodes_arr, &[i64.const_int(i as u64, false)], "la_n")
                    .map_err(llvm_err)?
            };
            let hp = unsafe {
                self.builder
                    .build_gep(i64, hs_arr, &[i64.const_int(i as u64, false)], "la_h")
                    .map_err(llvm_err)?
            };
            self.builder.build_store(np, dp).map_err(llvm_err)?;
            self.builder.build_store(hp, h).map_err(llvm_err)?;
        }
        let n = i64.const_int(live.len() as u64, false);
        let rla_fn = self
            .module
            .get_function("action_rc_release_list_on_assign")
            .ok_or("action_rc_release_list_on_assign not found")?;
        let _ = self.builder.build_call(
            rla_fn,
            &[
                old_data_ptr.into(),
                old_height.into(),
                nodes_arr.into(),
                hs_arr.into(),
                n.into(),
            ],
            "",
        );
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
    /// Check if `val` is a local variable in any scope (walks parent chain).
    fn is_var_in_full_scope_chain(&self, val: &TypedValue<'ctx>) -> bool {
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
            TypedValue::Fn(p, _) => Some(*p),
            TypedValue::Closure { alloca, .. } => *alloca,
            _ => None,
        };
        match alloca {
            Some(ptr) => {
                let mut s = &self.scope;
                loop {
                    if s.local_variables().values().any(|v| v.ptr == ptr) {
                        return true;
                    }
                    match &s.parent {
                        Some(p) => s = p.as_ref(),
                        None => break,
                    }
                }
                false
            }
            None => false,
        }
    }

    pub(super) fn rc_free_method_receiver(&self, val: &TypedValue<'ctx>) -> Result<(), String> {
        if self.is_var_in_full_scope_chain(val) {
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
