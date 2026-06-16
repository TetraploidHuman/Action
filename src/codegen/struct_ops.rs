// Submodule: struct_ops — struct/tuple/field/string-interpolation operations
//
// Extracted from misc.rs.
//

use crate::ast::*;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, InnerType, TypedValue, ValKind};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn struct_field_index(
        &self,
        st: &StructType<'ctx>,
        field: &str,
    ) -> Result<u32, String> {
        // Find the named struct whose LLVM type matches st
        for (name, named_st) in &self.named_structs {
            if *named_st == *st {
                if let Some(si) = self.registry.structs.values().find(|si| si.name == *name) {
                    return si
                        .fields
                        .iter()
                        .position(|(n, _)| n == field)
                        .map(|i| i as u32)
                        .ok_or_else(|| {
                            format!("Field '{}' not found in struct '{}'", field, name)
                        });
                }
            }
        }
        Err(format!("Field '{}' not found in struct", field))
    }

    /// ValKind for a struct field from the type registry (List vs Map vs Set).
    pub(super) fn struct_field_val_kind(
        &self,
        st: &StructType<'ctx>,
        field_idx: u32,
    ) -> ValKind {
        for (name, named_st) in &self.named_structs {
            if *named_st == *st {
                if let Some(si) = self.registry.structs.get(name) {
                    if let Some((_, ty)) = si.fields.get(field_idx as usize) {
                        return self.param_val_kind(Some(ty));
                    }
                }
                break;
            }
        }
        ValKind::List
    }

    /// Extract a field value from a TypedValue::Struct at the given index.
    /// `inner_type` is the InnerType of the data inside enum fields, if known.
    /// When None (struct name not tracked at codegen level), defaults to Int.
    pub(super) fn extract_field_from_struct(
        &mut self,
        struct_val: &TypedValue<'ctx>,
        idx: usize,
        inner_type: Option<InnerType>,
    ) -> Result<TypedValue<'ctx>, String> {
        match struct_val {
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                let loaded = self
                    .builder
                    .build_load(bt, *ptr, "field_load")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let field = self
                    .builder
                    .build_extract_value(loaded, idx as u32, &format!("f{}", idx))
                    .map_err(llvm_err)?;
                let field_ty = field.get_type();
                let alloca = self
                    .builder
                    .build_alloca(field_ty, "field_tmp")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, field).map_err(llvm_err)?;
                let kind = self.bv_kind(&field);
                match kind {
                    ValKind::Str => Ok(TypedValue::Str(alloca)),
                    ValKind::List => Ok(TypedValue::List(alloca)),
                    ValKind::Map => Ok(TypedValue::Map(alloca)),
                    ValKind::Set => Ok(TypedValue::Set(alloca)),
                    ValKind::Struct => Ok(TypedValue::Struct(alloca, *st)),
                    ValKind::Enum => Ok(TypedValue::Enum(
                        alloca,
                        *st,
                        inner_type.unwrap_or(InnerType::Int),
                        false,
                    )),
                    ValKind::Bool => Ok(TypedValue::Bool(field.into_int_value())),
                    ValKind::Int => Ok(TypedValue::Int(field.into_int_value())),
                    ValKind::Float => Ok(TypedValue::Float(field.into_float_value())),
                    _ => Ok(TypedValue::Unit),
                }
            }
            _ => Err("Cannot extract field from non-struct value".to_string()),
        }
    }

    pub(super) fn compile_string_interp(
        &mut self,
        parts: &[StringPart],
    ) -> Result<TypedValue<'ctx>, String> {
        let mut result: Option<PointerValue<'ctx>> = None;
        for p in parts {
            let str_ptr = match p {
                StringPart::Literal(s) => {
                    let tv = self.compile_string_literal(s)?;
                    match tv {
                        TypedValue::Str(ptr) => Some(ptr),
                        _ => None,
                    }
                }
                StringPart::Expr(expr) => {
                    let val = self.compile_expr(expr)?;
                    self.value_to_string_ptr(&val)?
                }
            };

            if let Some(ptr) = str_ptr {
                result = match result {
                    None => Some(ptr),
                    Some(acc) => {
                        let cc = self.call_rt_with_2str("action_string_concat", acc, ptr)?;
                        // Free the accumulator's data if it's an intermediate (not a scope
                        // variable). Intermediates start at RC=0 so rc_inc+rc_dec triggers
                        // the free via RC 0→1→0.
                        if !self.is_scope_variable(&TypedValue::Str(acc)) {
                            let old_str = self.load_string(acc)?;
                            let old_data = self
                                .builder
                                .build_extract_value(old_str, 1, "old_data")
                                .map_err(llvm_err)?
                                .into_pointer_value();
                            self.rc_inc(old_data)?;
                            self.rc_dec(old_data)?;
                        }
                        // Free the part being concatenated if it's an intermediate.
                        if !self.is_scope_variable(&TypedValue::Str(ptr)) {
                            let part_str = self.load_string(ptr)?;
                            let part_data = self
                                .builder
                                .build_extract_value(part_str, 1, "part_data")
                                .map_err(llvm_err)?
                                .into_pointer_value();
                            self.rc_inc(part_data)?;
                            self.rc_dec(part_data)?;
                        }
                        match cc.try_as_basic_value().basic() {
                            Some(bv) => {
                                let alloca = self
                                    .builder
                                    .build_alloca(self.string_type, "interp")
                                    .map_err(llvm_err)?;
                                self.builder.build_store(alloca, bv).map_err(llvm_err)?;
                                Some(alloca)
                            }
                            None => Some(acc),
                        }
                    }
                };
            }
        }
        match result {
            Some(ptr) => Ok(TypedValue::Str(ptr)),
            None => {
                let g = self
                    .builder
                    .build_global_string_ptr("", "empty")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Str(g.as_pointer_value()))
            }
        }
    }

    /// Convert a typed value to a string pointer (for string interpolation)
    pub(super) fn value_to_string_ptr(
        &mut self,
        val: &TypedValue<'ctx>,
    ) -> Result<Option<PointerValue<'ctx>>, String> {
        match val {
            TypedValue::Int(iv) => {
                let cc = self.call_rt("action_int_to_string", &[(*iv).into()])?;
                match cc.try_as_basic_value().basic() {
                    Some(bv) => {
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "int_str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, bv).map_err(llvm_err)?;
                        Ok(Some(alloca))
                    }
                    None => Ok(None),
                }
            }
            TypedValue::Float(fv) => {
                let cc = self.call_rt("action_float_to_string", &[(*fv).into()])?;
                match cc.try_as_basic_value().basic() {
                    Some(bv) => {
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "float_str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, bv).map_err(llvm_err)?;
                        Ok(Some(alloca))
                    }
                    None => Ok(None),
                }
            }
            TypedValue::Str(ptr) => Ok(Some(*ptr)),
            TypedValue::Bool(bv) => {
                // Convert bool to string "true" or "false"
                let true_str = self.compile_string_literal("true")?;
                let false_str = self.compile_string_literal("false")?;
                if let (TypedValue::Str(tp), TypedValue::Str(fp)) = (&true_str, &false_str) {
                    let current_fn = self
                        .builder
                        .get_insert_block()
                        .unwrap()
                        .get_parent()
                        .unwrap();
                    let true_block = self.context.append_basic_block(current_fn, "bool_true");
                    let false_block = self.context.append_basic_block(current_fn, "bool_false");
                    let merge_block = self.context.append_basic_block(current_fn, "bool_merge");

                    self.builder
                        .build_conditional_branch(*bv, true_block, false_block)
                        .map_err(llvm_err)?;

                    self.builder.position_at_end(true_block);
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .map_err(llvm_err)?;

                    self.builder.position_at_end(false_block);
                    self.builder
                        .build_unconditional_branch(merge_block)
                        .map_err(llvm_err)?;

                    self.builder.position_at_end(merge_block);
                    let phi = self
                        .builder
                        .build_phi(self.ptr_ty(), "bool_str")
                        .map_err(llvm_err)?;
                    let tp_bv: BasicValueEnum = (*tp).into();
                    let fp_bv: BasicValueEnum = (*fp).into();
                    phi.add_incoming(&[(&tp_bv, true_block), (&fp_bv, false_block)]);
                    Ok(Some(phi.as_basic_value().into_pointer_value()))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None), // Floats and other types not yet supported in interpolation
        }
    }

    pub(super) fn compile_field_access(
        &mut self,
        obj: &Expr,
        field: &str,
    ) -> Result<TypedValue<'ctx>, String> {
        // Handle enum variant access: EnumName.Variant
        if let Expr::Ident(enum_name) = obj {
            if self.enum_types.contains_key(enum_name) {
                // Look up the variant in this specific enum
                let variant_info = self
                    .registry
                    .lookup_variant(field)
                    .map(|(ei, vi)| (ei.clone(), vi.clone()));
                if let Some((enum_info, variant)) = variant_info {
                    if enum_info.name == *enum_name {
                        if variant.params.is_empty() {
                            return self.compile_enum_construct(&enum_info, &variant, &[]);
                        }
                        return Err(format!(
                            "Enum variant '{}.{}' requires arguments",
                            enum_name, field
                        ));
                    }
                }
                return Err(format!(
                    "Variant '{}' not found in enum '{}'",
                    field, enum_name
                ));
            }
            // Check if it's a module-qualified function call handled elsewhere (e.g., math.add)
        }
        let o = self.compile_expr(obj)?;

        // If receiver is nullable, auto short-circuit on null
        if let TypedValue::Nullable(nullable_ptr, inner_bt) = o {
            let current_fn = self
                .builder
                .get_insert_block()
                .and_then(|b| b.get_parent())
                .ok_or("Cannot access field outside function")?;

            let b1 = self.null_flag_ty();
            let nullable_st = inner_bt.into_struct_type();
            let null_bt: BasicTypeEnum = nullable_st.into();

            let loaded = self
                .builder
                .build_load(null_bt, nullable_ptr, "nfa_ld")
                .map_err(llvm_err)?;
            let nullable_struct = loaded.into_struct_value();
            let null_flag = self
                .builder
                .build_extract_value(nullable_struct, 0, "nfa_flag")
                .map_err(llvm_err)?
                .into_int_value();

            let is_null = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    null_flag,
                    b1.const_int(1, false),
                    "nfa_is_null",
                )
                .map_err(llvm_err)?;

            let null_block = self.context.append_basic_block(current_fn, "nfa_null");
            let val_block = self.context.append_basic_block(current_fn, "nfa_val");
            let merge_block = self.context.append_basic_block(current_fn, "nfa_merge");

            self.builder
                .build_conditional_branch(is_null, null_block, val_block)
                .map_err(llvm_err)?;

            // Value path: extract inner, access field, wrap result in nullable.
            // Processed first so the wrapped result type informs the null path.
            self.builder.position_at_end(val_block);
            let inner = self
                .builder
                .build_extract_value(nullable_struct, 1, "nfa_inner")
                .map_err(llvm_err)?;
            let inner_typed = self.bv_to_typed(inner)?;

            let field_result =
                self.compile_field_access_on_typed_value(&inner_typed, field, inner_bt)?;
            let field_bt = field_result.get_value_type(self);
            let field_wrapped = self.wrap_in_typed_nullable(&field_result, field_bt)?;
            let (wrapped_ptr, wrapped_bt) = match field_wrapped {
                TypedValue::Nullable(p, t) => (p, t),
                _ => return Err("wrap_in_typed_nullable did not return Nullable".to_string()),
            };
            let val_loaded = self
                .builder
                .build_load(wrapped_bt, wrapped_ptr, "nfa_val_ld")
                .map_err(llvm_err)?;
            self.builder
                .build_unconditional_branch(merge_block)
                .map_err(llvm_err)?;

            // Null path: produce null of the same wrapped type as the value path
            self.builder.position_at_end(null_block);
            let wrapped_struct_ty = wrapped_bt.into_struct_type();
            let undef = wrapped_struct_ty.get_undef();
            let null_struct = self
                .builder
                .build_insert_value(undef, b1.const_int(1, false), 0, "nfa_null_flag")
                .map_err(llvm_err)?;
            self.builder
                .build_unconditional_branch(merge_block)
                .map_err(llvm_err)?;

            // Merge: phi the null and value paths (both have the same struct type)
            self.builder.position_at_end(merge_block);
            let phi = self
                .builder
                .build_phi(wrapped_bt, "nfa_merge")
                .map_err(llvm_err)?;
            phi.add_incoming(&[(&null_struct, null_block), (&val_loaded, val_block)]);

            let result_alloca = self
                .builder
                .build_alloca(wrapped_struct_ty, "nfa_result")
                .map_err(llvm_err)?;
            self.builder
                .build_store(result_alloca, phi.as_basic_value())
                .map_err(llvm_err)?;
            return Ok(TypedValue::Nullable(result_alloca, wrapped_bt));
        }

        if let TypedValue::Str(ptr) = &o {
            if field == "length" {
                let gep = self
                    .builder
                    .build_struct_gep(self.string_type, *ptr, 0, "lenp")
                    .map_err(llvm_err)?;
                let len = self
                    .builder
                    .build_load(self.i64_ty(), gep, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                return Ok(TypedValue::Int(len));
            }
        }
        if let TypedValue::Struct(ptr, struct_ty) = &o {
            let bt: BasicTypeEnum = (*struct_ty).into();
            let loaded = self
                .builder
                .build_load(bt, *ptr, "struct_ld")
                .map_err(llvm_err)?;
            let struct_val = loaded.into_struct_value();

            // Check if field is a numeric index for tuple access: .0, .1, etc.
            if let Ok(idx) = field.parse::<usize>() {
                let field_val = self
                    .builder
                    .build_extract_value(struct_val, idx as u32, field)
                    .map_err(llvm_err)?;
                return self.bv_to_typed(field_val);
            }

            let field_names = self.lookup_struct_field_names(*struct_ty);
            let idx = field_names
                .iter()
                .position(|n| n == field)
                .ok_or_else(|| format!("Field '{}' not found on struct", field))?;
            let field_val = self
                .builder
                .build_extract_value(struct_val, idx as u32, field)
                .map_err(llvm_err)?;
            return self.bv_to_typed(field_val);
        }
        Err(format!("Field '{}' not supported on this type", field))
    }

    pub(super) fn lookup_struct_field_names(&self, struct_ty: StructType<'ctx>) -> Vec<String> {
        for (name, st) in &self.named_structs {
            if *st == struct_ty {
                if let Some(info) = self.registry.get_struct(name) {
                    return info.fields.iter().map(|(n, _)| n.clone()).collect();
                }
            }
        }
        for (names, st) in &self.anon_structs {
            if *st == struct_ty {
                return names.clone();
            }
        }
        vec![]
    }

    pub(super) fn compile_struct_lit(
        &mut self,
        fields: &[(String, Expr)],
    ) -> Result<TypedValue<'ctx>, String> {
        let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();

        // Compile all field expressions first so we can determine their types
        let mut field_vals: Vec<TypedValue> = Vec::new();
        for (_, expr) in fields.iter() {
            field_vals.push(self.compile_expr(expr)?);
        }

        // Determine struct type from registry (named) or from actual field types (anonymous)
        let struct_ty = if let Some(info) = self.registry.find_struct_by_fields(&field_names) {
            *self
                .named_structs
                .get(&info.name)
                .ok_or_else(|| format!("Struct '{}' not in LLVM type map", info.name))?
        } else if let Some(ct) = self.anon_structs.get(&field_names) {
            *ct
        } else {
            let field_tys: Vec<BasicTypeEnum> =
                field_vals.iter().map(|v| v.get_value_type(self)).collect();
            let anon_ty = self.context.struct_type(&field_tys, false);
            self.anon_structs.insert(field_names, anon_ty);
            anon_ty
        };

        let bt: BasicTypeEnum = struct_ty.into();
        let alloca = self
            .builder
            .build_alloca(bt, "struct_lit")
            .map_err(llvm_err)?;

        let field_types = struct_ty.get_field_types();
        let undef = struct_ty.get_undef();
        let mut result = undef;

        for (i, val) in field_vals.iter().enumerate() {
            let expected_ft = field_types.get(i).copied();
            let bv = match val {
                TypedValue::Struct(ptr, ty) => {
                    let sbt: BasicTypeEnum = (*ty).into();
                    self.builder
                        .build_load(sbt, *ptr, "field_struct")
                        .map_err(llvm_err)?
                        .as_basic_value_enum()
                }
                TypedValue::Nullable(ptr, ty) => self
                    .builder
                    .build_load(*ty, *ptr, "field_nullable")
                    .map_err(llvm_err)?,
                _ => {
                    // If the struct field expects a nullable type but we have a scalar,
                    // wrap the scalar in a nullable struct {i8=0, scalar}
                    let needs_wrap = expected_ft
                        .map(|ft| {
                            if let BasicTypeEnum::StructType(st) = ft {
                                let fts = st.get_field_types();
                                fts.len() == 2
                                    && matches!(fts[0], BasicTypeEnum::IntType(t) if t.get_bit_width() == 8)
                            } else {
                                false
                            }
                        })
                        .unwrap_or(false);
                    if needs_wrap {
                        let field_st = if let BasicTypeEnum::StructType(st) = expected_ft.unwrap() {
                            st
                        } else {
                            return Err("Expected struct type for nullable field".into());
                        };
                        let undef_f = field_st.get_undef();
                        let flag = self.null_flag_ty().const_int(0, false);
                        let with_flag = self
                            .builder
                            .build_insert_value(undef_f, flag, 0, "slf_flag")
                            .map_err(llvm_err)?;
                        let scalar = val
                            .to_bv()
                            .unwrap_or_else(|| self.i64_ty().const_int(0, false).into());
                        self.builder
                            .build_insert_value(with_flag, scalar, 1, "slf_val")
                            .map_err(llvm_err)?
                            .as_basic_value_enum()
                    } else {
                        val.to_bv().unwrap_or_else(|| {
                            self.i64_ty().const_int(0, false).as_basic_value_enum()
                        })
                    }
                }
            };
            result = self
                .builder
                .build_insert_value(result, bv, i as u32, "field")
                .map_err(llvm_err)?
                .into_struct_value();
        }

        self.builder.build_store(alloca, result).map_err(llvm_err)?;
        Ok(TypedValue::Struct(alloca, struct_ty))
    }

    pub(super) fn compile_tuple(
        &mut self,
        exprs: &[(Option<String>, Expr)],
    ) -> Result<TypedValue<'ctx>, String> {
        if exprs.is_empty() {
            return Ok(TypedValue::Unit);
        }
        // First compile all field values
        let mut values: Vec<TypedValue<'ctx>> = Vec::new();
        let mut field_names: Vec<String> = Vec::new();
        for (name_opt, expr) in exprs {
            let val = self.compile_expr(expr)?;
            values.push(val);
            if let Some(name) = name_opt {
                field_names.push(name.clone());
            } else {
                field_names.push(format!("_{}", field_names.len()));
            }
        }

        // Convert each value to BasicValueEnum and collect the *actual* LLVM types
        let mut field_tys: Vec<BasicTypeEnum> = Vec::new();
        let mut field_bvs: Vec<BasicValueEnum> = Vec::new();
        for val in &values {
            let bv: BasicValueEnum = match val {
                TypedValue::Str(ptr) => {
                    let loaded = self.load_string(*ptr)?;
                    loaded.as_basic_value_enum()
                }
                TypedValue::List(ptr) => {
                    let loaded = self.load_list(*ptr)?;
                    loaded.as_basic_value_enum()
                }
                TypedValue::Struct(ptr, st) => {
                    let bt2: BasicTypeEnum = (*st).into();
                    self.builder
                        .build_load(bt2, *ptr, "tuple_field")
                        .map_err(llvm_err)?
                }
                TypedValue::Enum(ptr, et, ..) => {
                    let bt2: BasicTypeEnum = (*et).into();
                    self.builder
                        .build_load(bt2, *ptr, "tuple_field")
                        .map_err(llvm_err)?
                }
                TypedValue::Nullable(ptr, ty) => self
                    .builder
                    .build_load(*ty, *ptr, "tuple_field_nullable")
                    .map_err(llvm_err)?,
                _ => val
                    .to_bv()
                    .unwrap_or_else(|| self.i64_ty().const_int(0, false).as_basic_value_enum()),
            };
            field_tys.push(bv.get_type());
            field_bvs.push(bv);
        }

        let struct_ty = self.context.struct_type(&field_tys, false);
        // Register in anon_structs so field access by name works
        self.anon_structs.entry(field_names).or_insert(struct_ty);
        let bt: BasicTypeEnum = struct_ty.into();
        let alloca = self.builder.build_alloca(bt, "tuple").map_err(llvm_err)?;

        let undef = struct_ty.get_undef();
        let mut result = undef;
        for (i, bv) in field_bvs.iter().enumerate() {
            result = self
                .builder
                .build_insert_value(result, *bv, i as u32, "tuple_elem")
                .map_err(llvm_err)?
                .into_struct_value();
        }
        self.builder.build_store(alloca, result).map_err(llvm_err)?;
        Ok(TypedValue::Struct(alloca, struct_ty))
    }
}
