// Submodule: misc — miscellaneous compile helpers
//
// Index, range, block, assign, and string/list load helpers.
// These are free-standing compile_* functions and utility loaders
// that don't belong to a single domain.
//

use action_frontend::ast::Literal;
use inkwell::types::{BasicType, BasicTypeEnum};
use inkwell::values::{GlobalValue, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue, ValKind};

impl<'ctx> CodeGen<'ctx> {
    /// Park the IR builder in `__cg_anchor` so module-level mutations do not touch
    /// user/runtime function IR. Prefer [`Self::add_module_global`] over raw `module.add_global`.
    pub(super) fn detach_builder(&self) -> Result<(), String> {
        self.position_codegen_anchor()
    }

    /// Add a module global, temporarily detaching the IR builder from any in-progress block.
    /// Restores an in-progress (non-terminated) insertion point afterward.
    pub(super) fn add_module_global<T>(
        &self,
        ty: T,
        name: &str,
    ) -> Result<GlobalValue<'ctx>, String>
    where
        T: BasicType<'ctx>,
    {
        let saved_pos = self
            .builder
            .get_insert_block()
            .filter(|bb| bb.get_terminator().is_none());
        self.detach_builder()?;
        let global = self.module.add_global(ty, None, name);
        if let Some(bb) = saved_pos {
            self.builder.position_at_end(bb);
        }
        Ok(global)
    }

    /// Park the IR builder in a dedicated void function so module-level work
    /// (e.g. `add_global` for consts) does not corrupt runtime or user IR.
    pub(super) fn position_codegen_anchor(&self) -> Result<(), String> {
        let void = self.void_ty();
        let anchor_fn = if let Some(f) = self.module.get_function("__cg_anchor") {
            f
        } else {
            let f = self
                .module
                .add_function("__cg_anchor", void.fn_type(&[], false), None);
            let entry = self.context.append_basic_block(f, "entry");
            let park = self.context.append_basic_block(f, "park");
            self.builder.position_at_end(entry);
            self.builder
                .build_unconditional_branch(park)
                .map_err(llvm_err)?;
            f
        };
        let park = anchor_fn
            .get_last_basic_block()
            .ok_or("__cg_anchor missing park block")?;
        self.builder.position_at_end(park);
        Ok(())
    }

    pub(super) fn finalize_codegen_anchor(&self) -> Result<(), String> {
        if let Some(anchor_fn) = self.module.get_function("__cg_anchor") {
            if let Some(park) = anchor_fn.get_last_basic_block() {
                if park.get_terminator().is_none() {
                    self.builder.position_at_end(park);
                    self.builder.build_unreachable().map_err(llvm_err)?;
                }
            }
        }
        self.builder.clear_insertion_position();
        Ok(())
    }

    /// Compile index access on already-compiled values (list/lazy list/string).
    pub(super) fn compile_index_values(
        &mut self,
        obj_val: TypedValue<'ctx>,
        idx_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let index_val = match idx_val {
            TypedValue::Int(v) => v,
            _ => return Err("Index must be an integer".to_string()),
        };
        match obj_val {
            TypedValue::List(list_ptr) => {
                let fat = if let Some(cache) = self.loop_control.list_loop_get_cache {
                    self.list_get_cached_fat(list_ptr, index_val, cache)?
                        .into_struct_value()
                } else {
                    let list_val = self.load_list(list_ptr)?;
                    let cc =
                        self.call_rt("action_list_get", &[list_val.into(), index_val.into()])?;
                    cc.try_as_basic_value()
                        .basic()
                        .ok_or("list_get failed")?
                        .into_struct_value()
                };
                let alloca = self
                    .builder
                    .build_alloca(self.string_type, "list_elem")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, fat).map_err(llvm_err)?;
                Ok(TypedValue::Str(alloca))
            }
            TypedValue::LazyList(list_ptr) => {
                let list_val = self.load_list(list_ptr)?;
                let cc = self.call_rt("action_list_get", &[list_val.into(), index_val.into()])?;
                match cc.try_as_basic_value().basic() {
                    Some(bv) => {
                        let fat = bv.into_struct_value();
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "list_elem")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, fat).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    None => Err("list_get failed".to_string()),
                }
            }
            TypedValue::Str(str_ptr) => {
                let str_val = self.load_string(str_ptr)?;
                let len_val = self
                    .builder
                    .build_extract_value(str_val, 0, "slen")
                    .map_err(llvm_err)?
                    .into_int_value();
                let data = self
                    .call_rt("action_string_data", &[str_val.into()])?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("string index: action_string_data failed")?
                    .into_pointer_value();
                let zero = self.i64_ty().const_int(0, false);
                let len_minus1 = self
                    .builder
                    .build_int_sub(len_val, self.i64_ty().const_int(1, false), "len1")
                    .map_err(llvm_err)?;
                let in_bounds = self
                    .builder
                    .build_and(
                        self.builder
                            .build_int_compare(IntPredicate::SGE, index_val, zero, "ge0")
                            .map_err(llvm_err)?,
                        self.builder
                            .build_int_compare(IntPredicate::SLE, index_val, len_minus1, "le_len")
                            .map_err(llvm_err)?,
                        "in_bounds",
                    )
                    .map_err(llvm_err)?;
                let safe_idx = self
                    .builder
                    .build_select(in_bounds, index_val, zero, "safe_idx")
                    .map_err(llvm_err)?
                    .into_int_value();
                let i8 = self.context.i8_type();
                let char_ptr = unsafe {
                    self.builder
                        .build_gep(i8, data, &[safe_idx], "char_ptr")
                        .map_err(llvm_err)
                }?;
                let char_val = self
                    .builder
                    .build_load(i8, char_ptr, "Char")
                    .map_err(llvm_err)?
                    .into_int_value();
                let raw = self
                    .builder
                    .build_int_z_extend(char_val, self.i64_ty(), "char_ext")
                    .map_err(llvm_err)?;
                let result = self
                    .builder
                    .build_select(in_bounds, raw, zero, "idx_result")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::Int(result))
            }
            _ => Err("Index access not supported for this type".to_string()),
        }
    }

    pub(super) fn compile_map_index_key(
        &mut self,
        map_ptr: PointerValue<'ctx>,
        key_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let key_fat = self.to_fat_struct(&key_val)?;
        let map_loaded = self.load_list(map_ptr)?;
        let cc = self.call_rt("action_map_contains", &[map_loaded.into(), key_fat.into()])?;
        let contains = cc
            .try_as_basic_value()
            .basic()
            .ok_or("contains failed")?
            .into_int_value();
        let map_loaded2 = self.load_list(map_ptr)?;
        let key_fat2 = self.to_fat_struct(&key_val)?;
        let gc = self.call_rt("action_map_get", &[map_loaded2.into(), key_fat2.into()])?;
        let val_fat = gc
            .try_as_basic_value()
            .basic()
            .ok_or("map_get failed")?
            .into_struct_value();
        let actual_val = self
            .builder
            .build_extract_value(val_fat, 0, "map_val")
            .map_err(llvm_err)?
            .into_int_value();
        self.build_fallible_int_from_ok(actual_val, contains)
    }

    pub(super) fn compile_set_index_key(
        &mut self,
        set_ptr: PointerValue<'ctx>,
        elem_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let elem_fat = self.to_fat_struct(&elem_val)?;
        let set_loaded = self.load_list(set_ptr)?;
        let cc = self.call_rt("action_map_contains", &[set_loaded.into(), elem_fat.into()])?;
        let contains = cc
            .try_as_basic_value()
            .basic()
            .ok_or("contains failed")?
            .into_int_value();
        let elem_fat2 = self.to_fat_struct(&elem_val)?;
        let actual_val = self
            .builder
            .build_extract_value(elem_fat2.into_struct_value(), 0, "set_val")
            .map_err(llvm_err)?
            .into_int_value();
        self.build_fallible_int_from_ok(actual_val, contains)
    }

    /// UFCS mutators used as statements (`m.insert(k, v)` without assignment) must write
    /// the new collection back into the receiver lvalue.
    pub(super) fn try_compile_mutating_ufcs_stmt_writeback(
        &mut self,
        expr: &action_frontend::hir::HirExpr,
    ) -> Result<bool, String> {
        use action_frontend::hir::HirExprKind;

        let HirExprKind::Call {
            func,
            args: _,
            trailing_lambda: _,
        } = &expr.kind
        else {
            return Ok(false);
        };
        let HirExprKind::FieldAccess(receiver, method) = &func.kind else {
            return Ok(false);
        };
        if !Self::is_mutating_collection_ufcs_method(method) {
            return Ok(false);
        };
        if !Self::hir_lvalue_is_assignable(receiver) {
            return Ok(false);
        }
        let result = self.compile_hir_expr(expr)?;
        self.write_back_hir_lvalue(receiver, result)?;
        Ok(true)
    }

    fn is_mutating_collection_ufcs_method(method: &str) -> bool {
        matches!(method, "insert" | "remove" | "append" | "prepend")
    }

    fn hir_lvalue_is_assignable(expr: &action_frontend::hir::HirExpr) -> bool {
        use action_frontend::hir::HirExprKind;
        match &expr.kind {
            HirExprKind::Ident(_) => true,
            HirExprKind::FieldAccess(obj, _) => Self::hir_lvalue_is_assignable(obj),
            HirExprKind::Index(obj, _) => Self::hir_lvalue_is_assignable(obj),
            _ => false,
        }
    }

    pub(super) fn compile_assign_hir(
        &mut self,
        target: &action_frontend::hir::HirExpr,
        value: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirExprKind;
        match &target.kind {
            HirExprKind::Ident(name) => {
                let v = self.compile_hir_expr(value)?;
                let is_closure = matches!(v, TypedValue::Closure { .. });
                let is_fn = matches!(v, TypedValue::Fn(_, _));
                let result = self.assign_mutable_ident(name, v)?;
                if is_closure {
                    self.scope.set_direct_fn_name(name, None);
                } else if is_fn {
                    let dn = Self::resolve_stored_direct_fn_name(self, value);
                    self.scope.set_direct_fn_name(name, dn);
                }
                Ok(result)
            }
            _ => {
                let v = self.compile_hir_expr(value)?;
                self.rc_inc_typed_value(&v)?;
                self.compile_assign_field_hir(target, &v)
            }
        }
    }

    pub(super) fn compile_assign_field_hir(
        &mut self,
        target: &action_frontend::hir::HirExpr,
        v: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirExprKind;
        match &target.kind {
            HirExprKind::FieldAccess(obj, field) => {
                let obj_val = self.compile_hir_expr(obj)?;
                self.assign_field_on_value(obj_val, field, v)
            }
            HirExprKind::Tuple(names) => self.assign_tuple_hir(names, v),
            HirExprKind::Index(obj, idx) => self.assign_index_hir(obj, idx, v),
            _ => Err(format!(
                "Complex assignment not yet supported: {:?}",
                target.kind
            )),
        }
    }

    fn assign_tuple_hir(
        &mut self,
        names: &[(Option<String>, action_frontend::hir::HirExpr)],
        v: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirExprKind;
        for (i, (_, name_expr)) in names.iter().enumerate() {
            match &name_expr.kind {
                HirExprKind::Ident(name) => {
                    let var_ptr = {
                        let var = self
                            .scope
                            .get(name)
                            .ok_or_else(|| format!("Undefined variable: {}", name))?;
                        if !var.mutable {
                            return Err(format!("Cannot assign to immutable variable '{}'", name));
                        }
                        var.ptr
                    };
                    let field_val = self.extract_field_from_struct(v, i, None)?;
                    if let Some(bv) = field_val.to_bv() {
                        self.builder.build_store(var_ptr, bv).map_err(llvm_err)?;
                    }
                }
                HirExprKind::Tuple(nested) => {
                    let nested_val = self.extract_field_from_struct(v, i, None)?;
                    self.assign_tuple_hir(nested, &nested_val)?;
                }
                _ => {
                    return Err(
                        "Destructuring assignment target must be an identifier or nested tuple"
                            .to_string(),
                    );
                }
            }
        }
        Ok(v.clone())
    }

    fn assign_index_hir(
        &mut self,
        obj: &action_frontend::hir::HirExpr,
        idx: &action_frontend::hir::HirExpr,
        elem: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirExprKind;
        let idx_val = self.compile_hir_expr(idx)?;
        let obj_val = self.compile_hir_expr(obj)?;
        match &obj_val {
            TypedValue::Map(map_ptr) => {
                let key_fat = self.to_fat_struct(&idx_val)?;
                let val_fat = self.to_fat_struct(elem)?;
                let map_loaded = self.load_list(*map_ptr)?;
                let cc = self.call_rt(
                    "action_map_insert",
                    &[map_loaded.into(), key_fat.into(), val_fat.into()],
                )?;
                let new_map = cc.try_as_basic_value().basic().ok_or("map insert failed")?;
                let scratch = self
                    .builder
                    .build_alloca(self.list_type, "map_assign")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(scratch, new_map)
                    .map_err(llvm_err)?;
                self.rc_free_intermediate(&obj_val)?;
                self.write_back_hir_lvalue(obj, TypedValue::Map(scratch))
            }
            TypedValue::Struct(ptr, st) => {
                let idx_int = match idx_val {
                    TypedValue::Int(v) => v,
                    _ => return Err("Struct index must be an integer".to_string()),
                };
                let index = match &idx.kind {
                    HirExprKind::Literal(Literal::Int(n)) => *n as u32,
                    _ => return Err("Tuple/struct index must be an integer literal".to_string()),
                };
                let _ = idx_int;
                let field_ptr = self
                    .builder
                    .build_struct_gep(*st, *ptr, index, "tuple_set_gep")
                    .map_err(llvm_err)?;
                let field_types = st.get_field_types();
                if (index as usize) < field_types.len() {
                    let fk = self.struct_field_val_kind(st, index)?;
                    self.rc_dec_field_val(field_ptr, field_types[index as usize], fk)?;
                }
                if let Some(bv) = elem.to_bv() {
                    self.builder.build_store(field_ptr, bv).map_err(llvm_err)?;
                }
                self.rc_free_intermediate(&obj_val)?;
                Ok(elem.clone())
            }
            TypedValue::List(lp) => {
                let idx_int = match idx_val {
                    TypedValue::Int(v) => v,
                    _ => return Err("List index must be an integer".to_string()),
                };
                let new_list = self.list_set_at(*lp, idx_int, elem)?;
                self.rc_free_intermediate(&obj_val)?;
                self.write_back_hir_lvalue(obj, new_list)
            }
            _ => Err("Cannot assign to index of this type".to_string()),
        }
    }

    fn write_back_hir_lvalue(
        &mut self,
        target: &action_frontend::hir::HirExpr,
        new_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirExprKind;
        match &target.kind {
            HirExprKind::Ident(name) => self.assign_mutable_ident(name, new_val),
            HirExprKind::FieldAccess(obj, field) => {
                let obj_val = self.compile_hir_expr(obj)?;
                self.assign_field_on_value(obj_val, field, &new_val)
            }
            HirExprKind::Index(outer, outer_idx) => {
                let outer_idx_val = self.compile_hir_expr(outer_idx)?;
                let outer_idx_int = match outer_idx_val {
                    TypedValue::Int(v) => v,
                    _ => return Err("Index must be an integer".to_string()),
                };
                let outer_container = self.compile_hir_expr(outer)?;
                let updated_outer = match &outer_container {
                    TypedValue::List(lp) => self.list_set_at(*lp, outer_idx_int, &new_val)?,
                    _ => {
                        return Err("Nested index assignment requires a list container".to_string())
                    }
                };
                self.rc_free_intermediate(&outer_container)?;
                self.write_back_hir_lvalue(outer, updated_outer)
            }
            _ => Err("Invalid index assignment target".to_string()),
        }
    }

    fn list_set_at(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        idx: IntValue<'ctx>,
        elem: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let loaded = self.load_list(list_ptr)?;
        let fat = self.to_fat_struct(elem)?;
        let set_fn = self
            .module
            .get_function("action_list_set")
            .ok_or("action_list_set not found")?;
        let cc = self
            .builder
            .build_call(set_fn, &[loaded.into(), idx.into(), fat.into()], "list_set")
            .map_err(llvm_err)?;
        let new_list = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_set returned void")?
            .into_struct_value();
        let alloca = self
            .builder
            .build_alloca(self.list_type, "list_set_out")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, new_list)
            .map_err(llvm_err)?;
        Ok(TypedValue::List(alloca))
    }

    fn assign_field_on_value(
        &mut self,
        obj_val: TypedValue<'ctx>,
        field: &str,
        v: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match obj_val {
            TypedValue::Struct(ptr, st) => {
                let idx = self.struct_field_index(&st, field)?;
                let field_ptr = self
                    .builder
                    .build_struct_gep(st, ptr, idx, "field_gep")
                    .map_err(llvm_err)?;
                let field_types = st.get_field_types();
                if (idx as usize) < field_types.len() {
                    let fk = self.struct_field_val_kind(&st, idx)?;
                    self.rc_dec_field_val(field_ptr, field_types[idx as usize], fk)?;
                }
                if let Some(bv) = v.to_bv() {
                    self.builder.build_store(field_ptr, bv).map_err(llvm_err)?;
                }
                Ok(v.clone())
            }
            _ => Err(format!("Cannot assign to field '{}' of non-struct", field)),
        }
    }

    pub(super) fn assign_mutable_ident(
        &mut self,
        name: &str,
        v: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (var_ptr, var_kind, var_ty, var_rc_managed, var_is_closure) = {
            let var = self
                .scope
                .get(name)
                .ok_or_else(|| format!("Undefined variable: {}", name))?;
            if !var.mutable {
                return Err(format!(
                    "Cannot assign to immutable variable '{}' (use 'var' instead of 'val')",
                    name
                ));
            }
            (
                var.ptr,
                var.kind,
                var.ty,
                var.enum_data_rc_managed,
                var.is_closure,
            )
        };
        // Snapshot old heap value before RHS — self-assignments like `m = m.insert(...)`
        // read the variable during RHS; dec must happen after RHS using the snapshot.
        let old_list = if matches!(var_kind, ValKind::List | ValKind::Map | ValKind::Set) {
            Some(self.load_list(var_ptr)?)
        } else {
            None
        };
        let old_str = if var_kind == ValKind::Str {
            Some(self.load_string(var_ptr)?)
        } else {
            None
        };
        // Skip rc_dec/rc_inc when in-place update reuses the same heap pointer.
        let skip_rc_transfer = match (&old_list, &old_str, &v) {
            (Some(old), _, TypedValue::List(np) | TypedValue::Map(np) | TypedValue::Set(np)) => {
                let new_loaded = self.load_list(*np)?;
                let old_data = self
                    .builder
                    .build_extract_value(*old, 0, "cmp_od")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let new_data = self
                    .builder
                    .build_extract_value(new_loaded, 0, "cmp_nd")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        self.builder
                            .build_ptr_to_int(old_data, self.i64_ty(), "odi")
                            .map_err(llvm_err)?,
                        self.builder
                            .build_ptr_to_int(new_data, self.i64_ty(), "ndi")
                            .map_err(llvm_err)?,
                        "same_ptr",
                    )
                    .map_err(llvm_err)?
            }
            (_, Some(old), TypedValue::Str(sp)) => {
                let new_loaded = self.load_string(*sp)?;
                let old_data = self
                    .builder
                    .build_extract_value(*old, 1, "cmp_os")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let new_data = self
                    .builder
                    .build_extract_value(new_loaded, 1, "cmp_ns")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                self.builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        self.builder
                            .build_ptr_to_int(old_data, self.i64_ty(), "osi")
                            .map_err(llvm_err)?,
                        self.builder
                            .build_ptr_to_int(new_data, self.i64_ty(), "nsi")
                            .map_err(llvm_err)?,
                        "same_sptr",
                    )
                    .map_err(llvm_err)?
            }
            _ => self.context.bool_type().const_int(0, false).into(),
        };
        let fn_val = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("not in fn")?;
        let do_rc_bb = self.context.append_basic_block(fn_val, "asg_rc");
        let skip_rc_bb = self.context.append_basic_block(fn_val, "asg_skip_rc");
        let after_rc_bb = self.context.append_basic_block(fn_val, "asg_after_rc");
        self.builder
            .build_conditional_branch(skip_rc_transfer, skip_rc_bb, do_rc_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(do_rc_bb);
        if var_is_closure {
            let cap_ptr = self
                .builder
                .build_load(self.ptr_ty(), var_ptr, "fn_dec_ptr")
                .map_err(llvm_err)?
                .into_pointer_value();
            self.rc_dec(cap_ptr)?;
        } else if let Some(old) = old_list {
            match var_kind {
                ValKind::List => {
                    let data_ptr = self
                        .builder
                        .build_extract_value(old, 0, "old_data")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    let height = self
                        .builder
                        .build_extract_value(old, 2, "old_h")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let is_shared = self.collection_root_shared_in_scope(var_ptr, data_ptr)?;
                    let list_sh_bb = self.context.append_basic_block(fn_val, "asg_list_sh");
                    let list_ar_bb = self.context.append_basic_block(fn_val, "asg_list_ar");
                    let list_dec_done = self.context.append_basic_block(fn_val, "asg_list_done");
                    let _ = self
                        .builder
                        .build_conditional_branch(is_shared, list_sh_bb, list_ar_bb);
                    self.builder.position_at_end(list_sh_bb);
                    self.rc_dec(data_ptr)?;
                    let _ = self.builder.build_unconditional_branch(list_dec_done);
                    self.builder.position_at_end(list_ar_bb);
                    let (new_data_ptr, new_height) = match &v {
                        TypedValue::List(np) => {
                            let new_loaded = self.load_list(*np)?;
                            let dp = self
                                .builder
                                .build_extract_value(new_loaded, 0, "new_data")
                                .map_err(llvm_err)?
                                .into_pointer_value();
                            let h = self
                                .builder
                                .build_extract_value(new_loaded, 2, "new_h")
                                .map_err(llvm_err)?
                                .into_int_value();
                            (dp, h)
                        }
                        _ => unreachable!("List assign with non-List RHS"),
                    };
                    self.emit_rc_release_list_on_assign(
                        var_ptr,
                        data_ptr,
                        height,
                        new_data_ptr,
                        new_height,
                    )?;
                    let _ = self.builder.build_unconditional_branch(list_dec_done);
                    self.builder.position_at_end(list_dec_done);
                }
                ValKind::Map | ValKind::Set => {
                    let data_ptr = self
                        .builder
                        .build_extract_value(old, 0, "old_data")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    let len = self
                        .builder
                        .build_extract_value(old, 1, "old_len")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let cap = self
                        .builder
                        .build_extract_value(old, 2, "old_cap")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let is_shared = self.collection_root_shared_in_scope(var_ptr, data_ptr)?;
                    let ht_sh_bb = self.context.append_basic_block(fn_val, "asg_ht_sh");
                    let ht_ex_bb = self.context.append_basic_block(fn_val, "asg_ht_ex");
                    let ht_dec_done = self.context.append_basic_block(fn_val, "asg_ht_done");
                    let _ = self
                        .builder
                        .build_conditional_branch(is_shared, ht_sh_bb, ht_ex_bb);
                    self.builder.position_at_end(ht_sh_bb);
                    self.rc_dec(data_ptr)?;
                    let _ = self.builder.build_unconditional_branch(ht_dec_done);
                    self.builder.position_at_end(ht_ex_bb);
                    let rht_fn = self.module.get_function("action_rc_dec_ht").unwrap();
                    let _ = self.builder.build_call(
                        rht_fn,
                        &[data_ptr.into(), cap.into(), len.into()],
                        "",
                    );
                    let _ = self.builder.build_unconditional_branch(ht_dec_done);
                    self.builder.position_at_end(ht_dec_done);
                }
                _ => {}
            }
        } else if let Some(old) = old_str {
            let data_ptr = self
                .builder
                .build_extract_value(old, 1, "old_sdata")
                .map_err(llvm_err)?
                .into_pointer_value();
            self.rc_dec(data_ptr)?;
        } else {
            self.rc_dec_at(var_ptr, var_kind, var_ty, var_rc_managed)?;
        }
        self.rc_inc_typed_value(&v)?;
        self.builder
            .build_unconditional_branch(after_rc_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(skip_rc_bb);
        self.builder
            .build_unconditional_branch(after_rc_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(after_rc_bb);
        match &v {
            TypedValue::Str(ptr) => {
                let str_struct = self.load_string(*ptr)?;
                self.builder
                    .build_store(var_ptr, str_struct)
                    .map_err(llvm_err)?;
            }
            TypedValue::List(ptr)
            | TypedValue::Map(ptr)
            | TypedValue::Set(ptr)
            | TypedValue::Task(ptr)
            | TypedValue::Stream(ptr) => {
                let list_struct = self.load_list(*ptr)?;
                self.builder
                    .build_store(var_ptr, list_struct)
                    .map_err(llvm_err)?;
            }
            TypedValue::Struct(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "assign_ld")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(var_ptr, loaded)
                    .map_err(llvm_err)?;
            }
            TypedValue::Enum(ptr, ty, inner_type, rc_managed) => {
                let bt: BasicTypeEnum = (*ty).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "assign_ld")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(var_ptr, loaded)
                    .map_err(llvm_err)?;
                // Update RC managed flag for the new enum value
                self.scope.set_enum_inner_type(name, *inner_type);
                self.scope.set_enum_data_rc_managed(name, *rc_managed);
            }
            TypedValue::LazyList(ptr)
            | TypedValue::CString(ptr)
            | TypedValue::Ptr(ptr)
            | TypedValue::FileHandle(ptr) => {
                self.builder.build_store(var_ptr, *ptr).map_err(llvm_err)?;
            }
            TypedValue::Fn(fn_ptr, fn_type) => {
                self.builder
                    .build_store(var_ptr, *fn_ptr)
                    .map_err(llvm_err)?;
                self.scope.set_fn_type(name, Some(*fn_type));
            }
            TypedValue::Closure {
                fn_ptr,
                actual_fn_type,
                closure_ptr,
                closure_ty,
                capture_ptr_rc_mask,
                ..
            } => {
                self.builder
                    .build_store(var_ptr, *closure_ptr)
                    .map_err(llvm_err)?;
                self.scope.set_closure_info(
                    name,
                    *closure_ty,
                    *fn_ptr,
                    *actual_fn_type,
                    *capture_ptr_rc_mask,
                );
            }
            _ => {
                if let Some(bv) = v.to_bv() {
                    self.builder.build_store(var_ptr, bv).map_err(llvm_err)?;
                }
            }
        }
        Ok(v)
    }

    /// Load a string struct value from its alloca pointer
    pub(super) fn load_string(
        &self,
        ptr: PointerValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let loaded = self
            .builder
            .build_load(self.string_type, ptr, "str_load")
            .map_err(llvm_err)?;
        Ok(loaded.into_struct_value())
    }

    /// Call a runtime function with a string argument (loads from alloca first)
    pub(super) fn call_rt_with_str(
        &self,
        name: &str,
        str_ptr: PointerValue<'ctx>,
    ) -> Result<inkwell::values::CallSiteValue<'ctx>, String> {
        let str_val = self.load_string(str_ptr)?;
        self.call_rt(name, &[str_val.into()])
    }

    /// Call a runtime function with two string arguments
    pub(super) fn call_rt_with_2str(
        &self,
        name: &str,
        s1: PointerValue<'ctx>,
        s2: PointerValue<'ctx>,
    ) -> Result<inkwell::values::CallSiteValue<'ctx>, String> {
        let v1 = self.load_string(s1)?;
        let v2 = self.load_string(s2)?;
        self.call_rt(name, &[v1.into(), v2.into()])
    }

    /// Load a list struct value from its alloca pointer
    pub(super) fn load_list(
        &self,
        ptr: PointerValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let loaded = self
            .builder
            .build_load(self.list_type, ptr, "list_load")
            .map_err(llvm_err)?;
        Ok(loaded.into_struct_value())
    }
    /// Extract list length from a loaded list struct
    pub(super) fn list_len_val(
        &self,
        list: inkwell::values::StructValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        Ok(self
            .builder
            .build_extract_value(list, 1, "list_len")
            .map_err(llvm_err)?
            .into_int_value())
    }

    /// Extract the data pointer from a loaded list struct (field 0)
    pub(super) fn list_data_ptr(
        &self,
        list: inkwell::values::StructValue<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        Ok(self
            .builder
            .build_extract_value(list, 0, "list_data")
            .map_err(llvm_err)?
            .into_pointer_value())
    }
}
