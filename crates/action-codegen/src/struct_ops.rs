// Submodule: struct_ops — struct/tuple/field/string-interpolation operations
//
// Extracted from misc.rs.
//

use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};

use super::{llvm_err, CodeGen, InnerType, TypedValue, ValKind};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn struct_field_index(
        &self,
        st: &StructType<'ctx>,
        field: &str,
    ) -> Result<u32, String> {
        // Find the named struct whose LLVM type matches st
        for (name, named_st) in &self.named_structs {
            if named_st == st {
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
    pub(super) fn struct_field_val_kind(&self, st: &StructType<'ctx>, field_idx: u32) -> ValKind {
        for (name, named_st) in &self.named_structs {
            if named_st == st {
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

    pub(super) fn compile_string_interp_hir(
        &mut self,
        parts: &[action_frontend::hir::HirStringPart],
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirStringPart;
        let mut result: Option<PointerValue<'ctx>> = None;
        for p in parts {
            let str_ptr = match p {
                HirStringPart::Literal(s) => {
                    let tv = self.compile_string_literal(s)?;
                    match tv {
                        TypedValue::Str(ptr) => Some(ptr),
                        _ => None,
                    }
                }
                HirStringPart::Expr(expr) => {
                    let val = self.compile_hir_expr(expr)?;
                    match self.value_to_string_ptr(&val)? {
                        Some(ptr) => Some(ptr),
                        None => {
                            return Err(
                                "Unsupported type in string interpolation".to_string()
                            )
                        }
                    }
                }
            };

            if let Some(ptr) = str_ptr {
                result = match result {
                    None => Some(ptr),
                    Some(acc) => {
                        let cc = self.call_rt_with_2str("action_string_concat", acc, ptr)?;
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
            TypedValue::Str(ptr) => Ok(Some(*ptr)),
            TypedValue::Nullable(np, inner_ty) => {
                let null_bt = self.get_nullable_type(*inner_ty, "interp_null");
                let loaded = self
                    .builder
                    .build_load(null_bt, *np, "interp_null_ld")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let flag = self
                    .builder
                    .build_extract_value(loaded, 0, "interp_null_flag")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_null = self
                    .builder
                    .build_int_compare(
                        inkwell::IntPredicate::EQ,
                        flag,
                        self.null_flag_ty().const_int(1, false),
                        "interp_is_null",
                    )
                    .map_err(llvm_err)?;
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                let null_bb = self.context.append_basic_block(current_fn, "interp_null");
                let val_bb = self.context.append_basic_block(current_fn, "interp_val");
                let merge_bb = self.context.append_basic_block(current_fn, "interp_merge");
                self.builder
                    .build_conditional_branch(is_null, null_bb, val_bb)
                    .map_err(llvm_err)?;
                self.builder.position_at_end(null_bb);
                let null_str = self.compile_string_literal("null")?;
                let null_ptr = match null_str {
                    TypedValue::Str(p) => p,
                    _ => return Ok(None),
                };
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(llvm_err)?;
                self.builder.position_at_end(val_bb);
                let inner = self
                    .builder
                    .build_extract_value(loaded, 1, "interp_inner")
                    .map_err(llvm_err)?;
                let inner_tv = self.bv_to_typed(inner)?;
                let inner_ptr = self.value_to_string_ptr(&inner_tv)?;
                self.builder
                    .build_unconditional_branch(merge_bb)
                    .map_err(llvm_err)?;
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(self.ptr_ty(), "interp_null_str")
                    .map_err(llvm_err)?;
                if let Some(ip) = inner_ptr {
                    let np_bv: BasicValueEnum = null_ptr.into();
                    let ip_bv: BasicValueEnum = ip.into();
                    phi.add_incoming(&[(&np_bv, null_bb), (&ip_bv, val_bb)]);
                    Ok(Some(phi.as_basic_value().into_pointer_value()))
                } else {
                    Ok(Some(null_ptr))
                }
            }
            _ => Ok(None),
        }
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

    pub(super) fn compile_struct_lit_values(
        &mut self,
        field_names: &[String],
        field_vals: Vec<TypedValue<'ctx>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let struct_ty = if let Some(info) = self.registry.find_struct_by_fields(&field_names) {
            *self
                .named_structs
                .get(&info.name)
                .ok_or_else(|| format!("Struct '{}' not in LLVM type map", info.name))?
        } else if let Some(ct) = self.anon_structs.get(field_names) {
            *ct
        } else {
            let field_tys: Vec<BasicTypeEnum> =
                field_vals.iter().map(|v| v.get_value_type(self)).collect();
            let anon_ty = self.context.struct_type(&field_tys, false);
            self.anon_structs.insert(field_names.to_vec(), anon_ty);
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

    pub(super) fn compile_tuple_call_args(
        &mut self,
        items: &[(Option<String>, super::call_arg::CallArg<'_>)],
    ) -> Result<TypedValue<'ctx>, String> {
        if items.is_empty() {
            return Ok(TypedValue::Unit);
        }
        let mut compiled = Vec::with_capacity(items.len());
        for (name_opt, arg) in items {
            compiled.push((name_opt.clone(), self.compile_call_arg(*arg)?));
        }
        self.compile_tuple_values(&compiled)
    }

    pub(super) fn compile_tuple_values(
        &mut self,
        items: &[(Option<String>, TypedValue<'ctx>)],
    ) -> Result<TypedValue<'ctx>, String> {
        if items.is_empty() {
            return Ok(TypedValue::Unit);
        }
        let mut field_names: Vec<String> = Vec::new();
        for (name_opt, _) in items {
            if let Some(name) = name_opt {
                field_names.push(name.clone());
            } else {
                field_names.push(format!("_{}", field_names.len()));
            }
        }

        let mut field_tys: Vec<BasicTypeEnum> = Vec::new();
        let mut field_bvs: Vec<BasicValueEnum> = Vec::new();
        for (_, val) in items {
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
