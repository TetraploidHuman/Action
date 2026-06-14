// Submodule: misc — miscellaneous compile helpers
//
// Index, range, block, assign, and string/list load helpers.
// These are free-standing compile_* functions and utility loaders
// that don't belong to a single domain.
//

use crate::ast::*;
use inkwell::types::BasicTypeEnum;
use inkwell::values::PointerValue;
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, Scope, TypedValue, ValKind};
use inkwell::values::IntValue;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_index(
        &mut self,
        obj: &Expr,
        idx: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let o = self.compile_expr(obj)?;

        // Nullable receiver: short-circuit on null, extract inner and index that
        if let TypedValue::Nullable(nullable_ptr, inner_bt) = o {
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

            // Null path: return null of the same nullable type
            self.builder.position_at_end(null_block);
            let null_loaded = self
                .builder
                .build_load(null_bt, nullable_ptr, "nidx_null_ld")
                .map_err(llvm_err)?;
            self.builder
                .build_unconditional_branch(merge_block)
                .map_err(llvm_err)?;

            // Value path: extract inner and index into it
            self.builder.position_at_end(val_block);
            let inner = self
                .builder
                .build_extract_value(nullable_struct, 1, "nidx_inner")
                .map_err(llvm_err)?;
            let inner_typed = self.bv_to_typed(inner)?;

            // Directly handle indexing on the inner TypedValue
            let idx_val = self.compile_expr(idx)?;
            let val_result: TypedValue = match &inner_typed {
                TypedValue::Map(map_ptr) => self.compile_map_index(*map_ptr, idx)?,
                TypedValue::Set(set_ptr) => self.compile_set_index(*set_ptr, idx)?,
                TypedValue::List(list_ptr) | TypedValue::LazyList(list_ptr) => {
                    let index_val = match idx_val {
                        TypedValue::Int(v) => v,
                        _ => return Err("Index must be an integer".to_string()),
                    };
                    let list_val = self.load_list(*list_ptr)?;
                    let cc =
                        self.call_rt("action_list_get", &[list_val.into(), index_val.into()])?;
                    match cc.try_as_basic_value().basic() {
                        Some(bv) => {
                            let fat = bv.into_struct_value();
                            let alloca = self
                                .builder
                                .build_alloca(self.string_type, "list_elem")
                                .map_err(llvm_err)?;
                            self.builder.build_store(alloca, fat).map_err(llvm_err)?;
                            TypedValue::Str(alloca)
                        }
                        None => return Err("list_get failed".to_string()),
                    }
                }
                _ => return Err("Indexing not supported on this type".to_string()),
            };

            let val_bv = val_result
                .to_bv()
                .unwrap_or_else(|| self.i64_ty().const_int(0, false).into());
            self.builder
                .build_unconditional_branch(merge_block)
                .map_err(llvm_err)?;

            // Merge: phi the null and value paths
            self.builder.position_at_end(merge_block);
            let phi_type = val_bv.get_type();
            let phi = self
                .builder
                .build_phi(phi_type, "nidx_merge")
                .map_err(llvm_err)?;
            phi.add_incoming(&[(&null_loaded, null_block), (&val_bv, val_block)]);

            return self.bv_to_typed(phi.as_basic_value());
        }

        // Tuple/struct indexing: requires compile-time constant integer index
        if let TypedValue::Struct(ptr, struct_ty) = &o {
            let index = match idx {
                Expr::Literal(Literal::Int(n)) => *n as u32,
                _ => return Err("Tuple/struct index must be an integer literal".to_string()),
            };
            let bt: BasicTypeEnum = (*struct_ty).into();
            let loaded = self
                .builder
                .build_load(bt, *ptr, "tuple_ld")
                .map_err(llvm_err)?;
            let struct_val = loaded.into_struct_value();
            let field_val = self
                .builder
                .build_extract_value(struct_val, index, "tuple_idx")
                .map_err(llvm_err)?;
            return self.bv_to_typed(field_val);
        }

        // Map indexing: map[key] -> Option<V>
        if let TypedValue::Map(map_ptr) = &o {
            return self.compile_map_index(*map_ptr, idx);
        }

        // Set indexing: set[elem] -> Option<T>
        if let TypedValue::Set(set_ptr) = &o {
            return self.compile_set_index(*set_ptr, idx);
        }

        let i = self.compile_expr(idx)?;
        let index_val = match i {
            TypedValue::Int(v) => v,
            _ => return Err("Index must be an integer".to_string()),
        };

        match o {
            TypedValue::List(list_ptr) | TypedValue::LazyList(list_ptr) => {
                let list_val = self.load_list(list_ptr)?;
                let cc = self.call_rt("action_list_get", &[list_val.into(), index_val.into()])?;
                match cc.try_as_basic_value().basic() {
                    Some(bv) => {
                        // list_get returns {i64, ptr} fat struct — the universal value repr.
                        // Store in string alloca; callers extract fields as needed.
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
                // Bounds check: clamp index to [0, len-1], return 0 for OOB
                let zero = self.i64_ty().const_int(0, false);
                let len_minus1 = self
                    .builder
                    .build_int_sub(len_val, self.i64_ty().const_int(1, false), "len1")
                    .map_err(llvm_err)?;
                let in_bounds = self
                    .builder
                    .build_and(
                        self.builder
                            .build_int_compare(inkwell::IntPredicate::SGE, index_val, zero, "ge0")
                            .map_err(llvm_err)?,
                        self.builder
                            .build_int_compare(
                                inkwell::IntPredicate::SLE,
                                index_val,
                                len_minus1,
                                "le_len",
                            )
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
                // Return 0 for out-of-bounds, actual char value for in-bounds
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

    pub(super) fn compile_map_index(
        &mut self,
        map_ptr: PointerValue<'ctx>,
        idx: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let key_val = self.compile_expr(idx)?;
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
    pub(super) fn compile_set_index(
        &mut self,
        set_ptr: PointerValue<'ctx>,
        idx: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let elem_val = self.compile_expr(idx)?;
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

    pub(super) fn compile_range(
        &mut self,
        start: &Expr,
        end: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        // Create a range struct {start: i64, end: i64, inclusive: i64}
        let start_v = self.compile_expr(start)?;
        let end_v = self.compile_expr(end)?;
        let start_int = match start_v {
            TypedValue::Int(v) => v,
            _ => return Err("Range start must be integer".into()),
        };
        let end_int = match end_v {
            TypedValue::Int(v) => v,
            _ => return Err("Range end must be integer".into()),
        };
        let range_ty = self.range_type;
        let alloca = self
            .builder
            .build_alloca(range_ty, "range")
            .map_err(llvm_err)?;
        let sptr = self
            .builder
            .build_struct_gep(range_ty, alloca, 0, "r_start")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sptr, start_int)
            .map_err(llvm_err)?;
        let eptr = self
            .builder
            .build_struct_gep(range_ty, alloca, 1, "r_end")
            .map_err(llvm_err)?;
        self.builder.build_store(eptr, end_int).map_err(llvm_err)?;
        let iptr = self
            .builder
            .build_struct_gep(range_ty, alloca, 2, "r_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(iptr, self.i64_ty().const_int(1, false))
            .map_err(llvm_err)?;
        Ok(TypedValue::Struct(alloca, range_ty))
    }

    pub(super) fn compile_block(&mut self, stmts: &[Stmt]) -> Result<TypedValue<'ctx>, String> {
        let mut saved = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved);
        self.scope = Scope::with_parent(saved);

        // Reset the flag — it will be set by inner compile_block calls for
        // Stmt::Expr that are themselves blocks, and by the final handling below.
        self.block_did_rc_inc = false;

        let mut last = TypedValue::Unit;
        for (_i, s) in stmts.iter().enumerate() {
            match s {
                Stmt::Expr { expr: e, .. } => {
                    // Discard the previous expression result before overwriting it.
                    // Non-last statement values are not used; heap-typed intermediates
                    // (RC=0) need rc_inc+rc_dec to trigger free, and scope-variable
                    // returns from inner blocks need rc_dec to drop the protection ref.
                    self.rc_discard_value(&last)?;
                    last = self.compile_expr(e)?;
                }
                _ => self.compile_stmt(s)?,
            }
        }

        // If a Return/Break/Continue was already emitted, the current block already
        // has a terminator and cleanup was done by that handler — skip to avoid
        // double rc_dec on scope variables.
        let current_block = self
            .builder
            .get_insert_block()
            .ok_or("compile_block: builder has no insert block")?;
        if current_block.get_terminator().is_none() {
            // RC inc the return value before cleaning up the scope — but only when
            // the last expression is a local variable that cleanup would decrement.
            // Literals and non-variable expressions don't need protection.
            if self.is_scope_variable(&last) {
                self.rc_inc_typed_value(&last)?;
                self.block_did_rc_inc = true;
            } else {
                self.block_did_rc_inc = false;
            }
            // RC cleanup: decrement refcounts on heap-typed variables in this scope
            self.emit_scope_cleanup()?;
        } else {
            self.block_did_rc_inc = false;
        }

        let mut parent = Scope::new();
        std::mem::swap(&mut self.scope, &mut parent);
        if let Some(p) = parent.parent {
            self.scope = *p;
        }
        Ok(last)
    }

    pub(super) fn compile_assign(
        &mut self,
        target: &Expr,
        value: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let v = self.compile_expr(value)?;
        // RC inc the new value before storing
        self.rc_inc_typed_value(&v)?;
        match target {
            Expr::Ident(name) => {
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
                // Dec RC of old value before overwriting
                if var_is_closure {
                    let cap_ptr = self
                        .builder
                        .build_load(self.ptr_ty(), var_ptr, "fn_dec_ptr")
                        .map_err(llvm_err)?
                        .into_pointer_value();
                    self.rc_dec(cap_ptr)?;
                } else {
                    self.rc_dec_at(var_ptr, var_kind, var_ty, var_rc_managed)?;
                }
                // Wrap non-nullable value into nullable when target is nullable
                let v = if var_kind == ValKind::Nullable && !matches!(&v, TypedValue::Nullable(..))
                {
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
            Expr::FieldAccess(obj, field) => {
                let obj_val = self.compile_expr(obj)?;
                match obj_val {
                    TypedValue::Struct(ptr, st) => {
                        let idx = self.struct_field_index(&st, field)?;
                        let field_ptr = self
                            .builder
                            .build_struct_gep(st, ptr, idx, "field_gep")
                            .map_err(llvm_err)?;
                        // RC-dec old value before overwriting (Bug #6)
                        let field_types = st.get_field_types();
                        if (idx as usize) < field_types.len() {
                            self.rc_dec_field_val(field_ptr, field_types[idx as usize])?;
                        }
                        if let Some(bv) = v.to_bv() {
                            self.builder.build_store(field_ptr, bv).map_err(llvm_err)?;
                        }
                        Ok(v)
                    }
                    TypedValue::Nullable(nullable_ptr, inner_bt) => {
                        // Extract the inner struct from the nullable wrapper
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
                                // RC-dec old value before overwriting (Bug #6)
                                let field_types = st.get_field_types();
                                if (idx as usize) < field_types.len() {
                                    self.rc_dec_field_val(field_ptr, field_types[idx as usize])?;
                                }
                                if let Some(bv) = v.to_bv() {
                                    self.builder.build_store(field_ptr, bv).map_err(llvm_err)?;
                                }
                                // Write back the modified inner struct into the nullable
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
                                Ok(v)
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
            Expr::Tuple(names) => {
                for (i, (_, name_expr)) in names.iter().enumerate() {
                    let name = match name_expr {
                        Expr::Ident(n) => n,
                        _ => return Err("Destructuring target must be an identifier".to_string()),
                    };
                    // Collect var info before mutable self call
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
                    let field_val = self.extract_field_from_struct(&v, i, None)?;
                    if let Some(bv) = field_val.to_bv() {
                        self.builder.build_store(var_ptr, bv).map_err(llvm_err)?;
                    }
                }
                Ok(v)
            }
            _ => Err("Complex assignment not yet supported".to_string()),
        }
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
