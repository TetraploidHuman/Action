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

use super::call_arg::CallArg;
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

    pub(super) fn compile_nullable_index_values(
        &mut self,
        nullable_ptr: PointerValue<'ctx>,
        inner_bt: BasicTypeEnum<'ctx>,
        idx: CallArg<'_>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot index outside function")?;

        let nullable_st = inner_bt.into_struct_type();
        let null_bt: BasicTypeEnum = nullable_st.into();

        let loaded = self
            .builder
            .build_load(null_bt, nullable_ptr, "nidx_ld")
            .map_err(llvm_err)?;
        let nullable_struct = loaded.into_struct_value();
        let null_flag = self
            .builder
            .build_extract_value(nullable_struct, 0, "nidx_flag")
            .map_err(llvm_err)?
            .into_int_value();

        let b1 = self.null_flag_ty();
        let is_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                null_flag,
                b1.const_int(1, false),
                "nidx_is_null",
            )
            .map_err(llvm_err)?;

        let null_block = self.context.append_basic_block(current_fn, "nidx_null");
        let val_block = self.context.append_basic_block(current_fn, "nidx_val");
        let merge_block = self.context.append_basic_block(current_fn, "nidx_merge");

        self.builder
            .build_conditional_branch(is_null, null_block, val_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(null_block);
        let null_loaded = self
            .builder
            .build_load(null_bt, nullable_ptr, "nidx_null_ld")
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(val_block);
        let inner = self
            .builder
            .build_extract_value(nullable_struct, 1, "nidx_inner")
            .map_err(llvm_err)?;
        let inner_typed = self.bv_to_typed(inner)?;

        let idx_val = self.compile_call_arg(idx)?;
        let val_result: TypedValue = match &inner_typed {
            TypedValue::Map(map_ptr) => self.compile_map_index_key(*map_ptr, idx_val.clone())?,
            TypedValue::Set(set_ptr) => self.compile_set_index_key(*set_ptr, idx_val.clone())?,
            TypedValue::List(_) | TypedValue::LazyList(_) => {
                self.compile_index_values(inner_typed, idx_val)?
            }
            _ => return Err("Indexing not supported on this type".to_string()),
        };

        let val_bv = val_result
            .to_bv()
            .unwrap_or_else(|| self.i64_ty().const_int(0, false).into());
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(merge_block);
        let phi_type = val_bv.get_type();
        let phi = self
            .builder
            .build_phi(phi_type, "nidx_merge")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&null_loaded, null_block), (&val_bv, val_block)]);

        self.bv_to_typed(phi.as_basic_value())
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
            TypedValue::List(list_ptr) | TypedValue::LazyList(list_ptr) => {
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
                    .builder
                    .build_extract_value(str_val, 1, "data")
                    .map_err(llvm_err)?
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

        let i64 = self.i64_ty();

        // Create nullable {i1, i64} — extract actual value from fat struct
        let nullable_ty = self.get_nullable_type(i64.into(), "Nullable<Int>");
        let null_bt: BasicTypeEnum = nullable_ty.into();
        let null_alloca = self
            .builder
            .build_alloca(nullable_ty, "map_idx_null")
            .map_err(llvm_err)?;

        let map_loaded = self.load_list(map_ptr)?;
        let contains_fn = self
            .module
            .get_function("action_map_contains")
            .ok_or("action_map_contains not found")?;
        let cc = self
            .builder
            .build_call(
                contains_fn,
                &[map_loaded.into(), key_fat.into()],
                "contains",
            )
            .map_err(llvm_err)?;
        let contains = cc
            .try_as_basic_value()
            .basic()
            .ok_or("contains failed")?
            .into_int_value();

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile map index outside function")?;
        let some_bb = self.context.append_basic_block(current_fn, "map_idx_some");
        let none_bb = self.context.append_basic_block(current_fn, "map_idx_none");
        let merge_bb = self.context.append_basic_block(current_fn, "map_idx_merge");

        let _ = self
            .builder
            .build_conditional_branch(contains, some_bb, none_bb);

        // Some path: get fat struct from map, extract field 0 (the actual value), build {flag=0, val}
        self.builder.position_at_end(some_bb);
        let map_loaded2 = self.load_list(map_ptr)?;
        let get_fn = self
            .module
            .get_function("action_map_get")
            .ok_or("action_map_get not found")?;
        let key_fat2 = self.to_fat_struct(&key_val)?;
        let gc = self
            .builder
            .build_call(get_fn, &[map_loaded2.into(), key_fat2.into()], "get")
            .map_err(llvm_err)?;
        let val_fat = gc
            .try_as_basic_value()
            .basic()
            .ok_or("map_get failed")?
            .into_struct_value();
        // Extract the actual value (field 0) from the fat struct {val, ptr}
        let actual_val = self
            .builder
            .build_extract_value(val_fat, 0, "map_val")
            .map_err(llvm_err)?
            .into_int_value();
        // Build nullable {flag=0, actual_val}
        let undef = nullable_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(
                undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "some_flag",
            )
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, actual_val, 1, "some_val")
            .map_err(llvm_err)?;
        self.builder
            .build_store(null_alloca, r2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);

        // None path: build nullable {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let undef2 = nullable_ty.get_undef();
        let rn1 = self
            .builder
            .build_insert_value(
                undef2,
                self.null_flag_ty().const_int(1, false),
                0,
                "none_flag",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(null_alloca, rn1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);

        self.builder.position_at_end(merge_bb);
        Ok(TypedValue::Nullable(null_alloca, null_bt))
    }

    /// Set indexing: set[elem] -> T? (nullable)

    pub(super) fn compile_set_index_key(
        &mut self,
        set_ptr: PointerValue<'ctx>,
        elem_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let elem_fat = self.to_fat_struct(&elem_val)?;

        let i64 = self.i64_ty();

        // Create nullable {i1, i64} — extract actual value from fat struct
        let nullable_ty = self.get_nullable_type(i64.into(), "Nullable<Int>");
        let null_bt: BasicTypeEnum = nullable_ty.into();
        let null_alloca = self
            .builder
            .build_alloca(nullable_ty, "set_idx_null")
            .map_err(llvm_err)?;

        let set_loaded = self.load_list(set_ptr)?;
        let contains_fn = self
            .module
            .get_function("action_map_contains")
            .ok_or("action_map_contains not found")?;
        let cc = self
            .builder
            .build_call(
                contains_fn,
                &[set_loaded.into(), elem_fat.into()],
                "contains",
            )
            .map_err(llvm_err)?;
        let contains = cc
            .try_as_basic_value()
            .basic()
            .ok_or("contains failed")?
            .into_int_value();

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile set index outside function")?;
        let some_bb = self.context.append_basic_block(current_fn, "set_idx_some");
        let none_bb = self.context.append_basic_block(current_fn, "set_idx_none");
        let merge_bb = self.context.append_basic_block(current_fn, "set_idx_merge");

        let _ = self
            .builder
            .build_conditional_branch(contains, some_bb, none_bb);

        // Some path: extract field 0 from fat struct, wrap as nullable {flag=0, val}
        self.builder.position_at_end(some_bb);
        let elem_fat2 = self.to_fat_struct(&elem_val)?;
        // Extract actual value (field 0) from fat struct {val, ptr}
        let actual_val = self
            .builder
            .build_extract_value(elem_fat2.into_struct_value(), 0, "set_val")
            .map_err(llvm_err)?
            .into_int_value();
        let undef = nullable_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(
                undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "some_flag",
            )
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, actual_val, 1, "some_val")
            .map_err(llvm_err)?;
        self.builder
            .build_store(null_alloca, r2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);

        // None path: nullable {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let undef2 = nullable_ty.get_undef();
        let rn1 = self
            .builder
            .build_insert_value(
                undef2,
                self.null_flag_ty().const_int(1, false),
                0,
                "none_flag",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(null_alloca, rn1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);

        self.builder.position_at_end(merge_bb);
        Ok(TypedValue::Nullable(null_alloca, null_bt))
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
                self.assign_mutable_ident(name, v)
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
            HirExprKind::Tuple(names) => {
                for (i, (_, name_expr)) in names.iter().enumerate() {
                    let name = match &name_expr.kind {
                        HirExprKind::Ident(n) => n,
                        _ => return Err("Destructuring target must be an identifier".to_string()),
                    };
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
                Ok(v.clone())
            }
            HirExprKind::Index(obj, idx) => self.assign_index_hir(obj, idx, v),
            _ => Err(format!(
                "Complex assignment not yet supported: {:?}",
                target.kind
            )),
        }
    }

    fn assign_index_hir(
        &mut self,
        obj: &action_frontend::hir::HirExpr,
        idx: &action_frontend::hir::HirExpr,
        elem: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirExprKind;
        let idx_val = self.compile_hir_expr(idx)?;
        let idx_int = match idx_val {
            TypedValue::Int(v) => v,
            _ => return Err("Index must be an integer".to_string()),
        };
        let obj_val = self.compile_hir_expr(obj)?;
        match &obj_val {
            TypedValue::Struct(ptr, st) => {
                let index = match &idx.kind {
                    HirExprKind::Literal(Literal::Int(n)) => *n as u32,
                    _ => return Err("Tuple/struct index must be an integer literal".to_string()),
                };
                let field_ptr = self
                    .builder
                    .build_struct_gep(*st, *ptr, index, "tuple_set_gep")
                    .map_err(llvm_err)?;
                let field_types = st.get_field_types();
                if (index as usize) < field_types.len() {
                    let fk = self.struct_field_val_kind(st, index);
                    self.rc_dec_field_val(field_ptr, field_types[index as usize], fk)?;
                }
                if let Some(bv) = elem.to_bv() {
                    self.builder.build_store(field_ptr, bv).map_err(llvm_err)?;
                }
                self.rc_free_intermediate(&obj_val)?;
                Ok(elem.clone())
            }
            TypedValue::List(lp) => {
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
                    let fk = self.struct_field_val_kind(&st, idx);
                    self.rc_dec_field_val(field_ptr, field_types[idx as usize], fk)?;
                }
                if let Some(bv) = v.to_bv() {
                    self.builder.build_store(field_ptr, bv).map_err(llvm_err)?;
                }
                Ok(v.clone())
            }
            TypedValue::Nullable(nullable_ptr, inner_bt) => {
                let loaded = self
                    .builder
                    .build_load(inner_bt, nullable_ptr, "asn_nf_ld")
                    .map_err(llvm_err)?;
                let nf_struct = loaded.into_struct_value();
                let inner = self
                    .builder
                    .build_extract_value(nf_struct, 1, "asn_inner")
                    .map_err(llvm_err)?;
                let inner_typed = self.bv_to_typed(inner)?;
                match inner_typed {
                    TypedValue::Struct(ptr, st) => {
                        let idx = self.struct_field_index(&st, field)?;
                        let field_ptr = self
                            .builder
                            .build_struct_gep(st, ptr, idx, "field_gep2")
                            .map_err(llvm_err)?;
                        let field_types = st.get_field_types();
                        if (idx as usize) < field_types.len() {
                            let fk = self.struct_field_val_kind(&st, idx);
                            self.rc_dec_field_val(field_ptr, field_types[idx as usize], fk)?;
                        }
                        if let Some(bv) = v.to_bv() {
                            self.builder.build_store(field_ptr, bv).map_err(llvm_err)?;
                        }
                        let inner_st_bt: BasicTypeEnum = st.into();
                        let updated_inner = self
                            .builder
                            .build_load(inner_st_bt, ptr, "asn_upd")
                            .map_err(llvm_err)?;
                        let updated_nf = self
                            .builder
                            .build_insert_value(nf_struct, updated_inner, 1, "asn_nf_upd")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(nullable_ptr, updated_nf)
                            .map_err(llvm_err)?;
                        Ok(v.clone())
                    }
                    _ => Err(format!(
                        "Cannot assign to field '{}' of non-struct inner",
                        field
                    )),
                }
            }
            _ => Err(format!("Cannot assign to field '{}' of non-struct", field)),
        }
    }

    fn assign_mutable_ident(
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
                    let rdl_fn = self.module.get_function("action_rc_dec_list_node").unwrap();
                    let _ = self
                        .builder
                        .build_call(rdl_fn, &[data_ptr.into(), height.into()], "");
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
                    let rht_fn = self.module.get_function("action_rc_dec_ht").unwrap();
                    let _ = self.builder.build_call(
                        rht_fn,
                        &[data_ptr.into(), cap.into(), len.into()],
                        "",
                    );
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
        // Wrap non-nullable value into nullable when target is nullable
        let v = if var_kind == ValKind::Nullable && !matches!(&v, TypedValue::Nullable(..)) {
            let inner_bt = v.get_value_type(self);
            let nty = self.get_nullable_type(inner_bt, "assign_wrap");
            self.wrap_in_nullable(&v, nty)?
        } else {
            v
        };
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
            TypedValue::Nullable(ptr, ty) => {
                let loaded = self
                    .builder
                    .build_load(*ty, *ptr, "assign_nullable_ld")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(var_ptr, loaded)
                    .map_err(llvm_err)?;
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
