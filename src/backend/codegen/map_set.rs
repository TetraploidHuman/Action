// Submodule: map_set

use crate::ast::*;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};

use super::{llvm_err, CodeGen, GepCursor, InnerType, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// Extract map/set length from a loaded {ptr, len, cap} struct (field 1).
    pub(super) fn map_len_val(
        &self,
        map: inkwell::values::StructValue<'ctx>,
    ) -> Result<inkwell::values::IntValue<'ctx>, String> {
        Ok(self
            .builder
            .build_extract_value(map, 1, "map_len")
            .map_err(llvm_err)?
            .into_int_value())
    }

    pub(super) fn compile_map_lit(
        &mut self,
        entries: &[(Expr, Expr)],
    ) -> Result<TypedValue<'ctx>, String> {
        let cap = self.i64_ty().const_int((entries.len() + 4) as u64, false);
        let cc = self.call_rt("action_map_create", &[cap.into()])?;
        let map_bv = cc.try_as_basic_value().basic().ok_or("map_create failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "map_lit")
            .map_err(llvm_err)?;
        self.builder.build_store(alloca, map_bv).map_err(llvm_err)?;

        for (key_expr, val_expr) in entries {
            let key_val = self.compile_expr(key_expr)?;
            let val_val = self.compile_expr(val_expr)?;
            let key_fat = self.to_fat_struct(&key_val)?;
            let val_fat = self.to_fat_struct(&val_val)?;
            let map_loaded = self.load_list(alloca)?;
            let cc = match self.call_rt(
                "action_map_insert",
                &[map_loaded.into(), key_fat.into(), val_fat.into()],
            ) {
                Ok(v) => v,
                Err(e) => {
                    let _ = self.rc_free_intermediate(&val_val);
                    let _ = self.rc_free_intermediate(&key_val);
                    return Err(e);
                }
            };
            let new_map = cc.try_as_basic_value().basic().ok_or("map_insert failed")?;
            self.builder
                .build_store(alloca, new_map)
                .map_err(llvm_err)?;
        }

        Ok(TypedValue::Map(alloca))
    }

    pub(super) fn compile_set_lit(
        &mut self,
        elements: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        // Set uses the same layout as map but with 2-i64 entries instead of 4-i64
        // For simplicity, use map layout but store elements as keys with null values
        let cap = self.i64_ty().const_int((elements.len() + 4) as u64, false);
        let cc = self.call_rt("action_map_create", &[cap.into()])?;
        let set_bv = cc.try_as_basic_value().basic().ok_or("map_create failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "set_lit")
            .map_err(llvm_err)?;
        self.builder.build_store(alloca, set_bv).map_err(llvm_err)?;

        let null_val: BasicValueEnum = {
            let undef = self.string_type.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, self.i64_ty().const_int(0, false), 0, "sn0")
                .map_err(llvm_err)?;
            let r2 = self
                .builder
                .build_insert_value(r1, self.ptr_ty().const_zero(), 1, "sn1")
                .map_err(llvm_err)?;
            r2.as_basic_value_enum()
        };

        for elem_expr in elements {
            let elem_val = self.compile_expr(elem_expr)?;
            let elem_fat = self.to_fat_struct(&elem_val)?;
            let set_loaded = self.load_list(alloca)?;
            let cc = match self.call_rt(
                "action_map_insert",
                &[set_loaded.into(), elem_fat.into(), null_val.into()],
            ) {
                Ok(v) => v,
                Err(e) => {
                    let _ = self.rc_free_intermediate(&elem_val);
                    return Err(e);
                }
            };
            let new_set = cc.try_as_basic_value().basic().ok_or("map_insert failed")?;
            self.builder
                .build_store(alloca, new_set)
                .map_err(llvm_err)?;
        }

        Ok(TypedValue::Set(alloca))
    }

    pub(super) fn builtin_set_of(&mut self, args: &[Expr]) -> Result<TypedValue<'ctx>, String> {
        // Set.of(...) is equivalent to a set literal with the given elements
        let cap = self.i64_ty().const_int((args.len() + 4) as u64, false);
        let cc = self.call_rt("action_map_create", &[cap.into()])?;
        let set_bv = cc.try_as_basic_value().basic().ok_or("map_create failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "set_of")
            .map_err(llvm_err)?;
        self.builder.build_store(alloca, set_bv).map_err(llvm_err)?;

        let null_val: BasicValueEnum = {
            let undef = self.string_type.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, self.i64_ty().const_int(0, false), 0, "sn0")
                .map_err(llvm_err)?;
            let r2 = self
                .builder
                .build_insert_value(r1, self.ptr_ty().const_zero(), 1, "sn1")
                .map_err(llvm_err)?;
            r2.as_basic_value_enum()
        };

        for elem_expr in args {
            let elem_val = self.compile_expr(elem_expr)?;
            self.rc_inc_typed_value(&elem_val)?;
            let elem_fat = self.to_fat_struct(&elem_val)?;
            let set_loaded = self.load_list(alloca)?;
            let cc = match self.call_rt(
                "action_map_insert",
                &[set_loaded.into(), elem_fat.into(), null_val.into()],
            ) {
                Ok(v) => v,
                Err(e) => {
                    let _ = self.rc_dec_typed_value(&elem_val);
                    let _ = self.rc_free_intermediate(&elem_val);
                    return Err(e);
                }
            };
            let new_set = cc.try_as_basic_value().basic().ok_or("map_insert failed")?;
            self.builder
                .build_store(alloca, new_set)
                .map_err(llvm_err)?;
        }

        Ok(TypedValue::Set(alloca))
    }

    /// map.insert(key, val) — receiver alloca is pre-compiled to avoid double compilation.
    pub(super) fn builtin_map_insert(
        &mut self,
        map_ptr: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("map.insert expects 2 arguments (key, value)".to_string());
        }
        let key_val = self.compile_expr(&args[0])?;
        let val_val = self.compile_expr(&args[1])?;
        let key_fat = self.to_fat_struct(&key_val)?;
        let val_fat = self.to_fat_struct(&val_val)?;
        let map_loaded = self.load_list(map_ptr)?;
        let cc = match self.call_rt(
            "action_map_insert",
            &[map_loaded.into(), key_fat.into(), val_fat.into()],
        ) {
            Ok(v) => v,
            Err(e) => {
                let _ = self.rc_free_intermediate(&val_val);
                let _ = self.rc_free_intermediate(&key_val);
                return Err(e);
            }
        };
        let new_map = cc.try_as_basic_value().basic().ok_or("map_insert failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "map_inserted")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, new_map)
            .map_err(llvm_err)?;
        Ok(TypedValue::Map(alloca))
    }

    /// map.remove(key) — receiver alloca is pre-compiled to avoid double compilation.
    pub(super) fn builtin_map_remove(
        &mut self,
        map_ptr: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err("map.remove expects 1 argument (key)".to_string());
        }
        let key_val = self.compile_expr(&args[0])?;
        let key_fat = self.to_fat_struct(&key_val)?;
        let map_loaded = self.load_list(map_ptr)?;
        let remove_fn = self
            .module
            .get_function("action_map_remove")
            .ok_or("action_map_remove not found")?;
        let rc = match self
            .builder
            .build_call(remove_fn, &[map_loaded.into(), key_fat.into()], "remove")
            .map_err(|e| {
                let _ = self.rc_free_intermediate(&key_val);
                llvm_err(e)
            }) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        let new_map = rc.try_as_basic_value().basic().ok_or("remove failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "map_removed")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, new_map)
            .map_err(llvm_err)?;
        Ok(TypedValue::Map(alloca))
    }

    /// map.contains(key) — receiver alloca is pre-compiled to avoid double compilation.
    pub(super) fn builtin_map_contains(
        &mut self,
        map_ptr: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err("map.contains expects 1 argument (key)".to_string());
        }
        let key_val = self.compile_expr(&args[0])?;
        let key_fat = self.to_fat_struct(&key_val)?;
        let map_loaded = self.load_list(map_ptr)?;
        let contains_fn = self
            .module
            .get_function("action_map_contains")
            .ok_or("action_map_contains not found")?;
        let cc = match self
            .builder
            .build_call(
                contains_fn,
                &[map_loaded.into(), key_fat.into()],
                "contains",
            )
            .map_err(|e| {
                let _ = self.rc_free_intermediate(&key_val);
                llvm_err(e)
            }) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        let contains = cc
            .try_as_basic_value()
            .basic()
            .ok_or("contains failed")?
            .into_int_value();
        Ok(TypedValue::Bool(contains))
    }

    /// set.insert(elem) — receiver alloca is pre-compiled to avoid double compilation.
    pub(super) fn builtin_set_insert(
        &mut self,
        set_ptr: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err("set.insert expects 1 argument (element)".to_string());
        }
        let elem_val = self.compile_expr(&args[0])?;
        let elem_fat = self.to_fat_struct(&elem_val)?;

        let null_val: BasicValueEnum = {
            let undef = self.string_type.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, self.i64_ty().const_int(0, false), 0, "sn0")
                .map_err(llvm_err)?;
            let r2 = self
                .builder
                .build_insert_value(r1, self.ptr_ty().const_zero(), 1, "sn1")
                .map_err(llvm_err)?;
            r2.as_basic_value_enum()
        };

        let set_loaded = self.load_list(set_ptr)?;
        let cc = match self.call_rt(
            "action_map_insert",
            &[set_loaded.into(), elem_fat.into(), null_val.into()],
        ) {
            Ok(v) => v,
            Err(e) => {
                let _ = self.rc_free_intermediate(&elem_val);
                return Err(e);
            }
        };
        let new_set = cc.try_as_basic_value().basic().ok_or("map_insert failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "set_inserted")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, new_set)
            .map_err(llvm_err)?;
        Ok(TypedValue::Set(alloca))
    }

    /// set.remove(elem) — receiver alloca is pre-compiled to avoid double compilation.
    pub(super) fn builtin_set_remove(
        &mut self,
        set_ptr: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err("set.remove expects 1 argument (element)".to_string());
        }
        let elem_val = self.compile_expr(&args[0])?;
        let elem_fat = self.to_fat_struct(&elem_val)?;
        let set_loaded = self.load_list(set_ptr)?;
        let remove_fn = self
            .module
            .get_function("action_map_remove")
            .ok_or("action_map_remove not found")?;
        let rc = match self
            .builder
            .build_call(remove_fn, &[set_loaded.into(), elem_fat.into()], "remove")
            .map_err(|e| {
                let _ = self.rc_free_intermediate(&elem_val);
                llvm_err(e)
            }) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        let new_set = rc.try_as_basic_value().basic().ok_or("remove failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "set_removed")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, new_set)
            .map_err(llvm_err)?;
        Ok(TypedValue::Set(alloca))
    }

    /// set.contains(elem) — receiver alloca is pre-compiled to avoid double compilation.
    pub(super) fn builtin_set_contains(
        &mut self,
        set_ptr: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err("set.contains expects 1 argument (element)".to_string());
        }
        let elem_val = self.compile_expr(&args[0])?;
        let elem_fat = self.to_fat_struct(&elem_val)?;
        let set_loaded = self.load_list(set_ptr)?;
        let contains_fn = self
            .module
            .get_function("action_map_contains")
            .ok_or("action_map_contains not found")?;
        let cc = match self
            .builder
            .build_call(
                contains_fn,
                &[set_loaded.into(), elem_fat.into()],
                "contains",
            )
            .map_err(|e| {
                let _ = self.rc_free_intermediate(&elem_val);
                llvm_err(e)
            }) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };
        let contains = cc
            .try_as_basic_value()
            .basic()
            .ok_or("contains failed")?
            .into_int_value();
        Ok(TypedValue::Bool(contains))
    }

    pub(super) fn compile_enum_construct(
        &mut self,
        enum_info: &crate::typecheck::EnumInfo,
        variant: &crate::typecheck::EnumVariantInfo,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let ptr_ty = self.ptr_ty();

        // Get or create the enum LLVM type {i64, i8*}
        let enum_ty = *self
            .enum_types
            .get(&enum_info.name)
            .ok_or_else(|| format!("Enum '{}' not in type map", enum_info.name))?;

        // Allocate space for the enum struct on the stack
        let enum_bt: BasicTypeEnum = enum_ty.into();
        let alloca = self
            .builder
            .build_alloca(enum_bt, "enum_val")
            .map_err(llvm_err)?;

        // Set the discriminant (tag)
        let tag_val = i64.const_int(variant.tag as u64, false);
        let undef = enum_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, tag_val, 0, "tag")
            .map_err(llvm_err)?;

        // For data-carrying variants, allocate heap memory and store fields
        let (data_ptr, inner_type) = if variant.params.is_empty() {
            (ptr_ty.const_zero(), InnerType::Int) // null pointer for unit variants
        } else {
            // Compile args first to determine sizes
            let compiled: Vec<TypedValue> = args
                .iter()
                .map(|a| self.compile_expr(a))
                .collect::<Result<Vec<_>, _>>()?;
            // Calculate total bytes: each field uses its alloca type size
            let mut total_bytes: u64 = 0;
            let mut offsets: Vec<u64> = Vec::new();
            for v in &compiled {
                offsets.push(total_bytes);
                let field_ty = v.get_value_type(self);
                total_bytes += self.type_store_size(field_ty);
            }
            let buf = self.malloc_rc(i64.const_int(total_bytes as u64, false))?;

            // Store each field at its offset using chained GEP cursor
            let mut cur = GepCursor::new(buf);
            let i8_ty = self.context.i8_type();
            for (i, v) in compiled.iter().enumerate() {
                let field_ptr = cur.offset_gep(&self.builder, i8_ty, offsets[i], "field_ptr")?;
                // store_value_to_alloca handles load+store for complex types
                self.store_value_to_alloca(v, field_ptr)?;
            }
            // Determine inner type from the first data argument
            let inner = compiled.first().map_or(InnerType::Int, |v| match v {
                TypedValue::Float(_) => InnerType::Float,
                TypedValue::Str(_) => InnerType::Str,
                _ => InnerType::Int,
            });
            (buf, inner)
        };

        let r2 = self
            .builder
            .build_insert_value(r1, data_ptr, 1, "data")
            .map_err(llvm_err)?;
        self.builder.build_store(alloca, r2).map_err(llvm_err)?;

        // The heap buffer starts with RC=0 (from malloc_rc). rc_inc to 1 so the
        // enum owns its data. Scope cleanup will rc_dec it when the enum goes out
        // of scope. Without this, passing the enum to a function would leak or
        // double-free: function entry rc_inc→1, function exit rc_dec→0→free, but
        // the caller still holds a reference to the freed data.
        if !variant.params.is_empty() {
            self.rc_inc(data_ptr)?;
        }

        Ok(TypedValue::Enum(alloca, enum_ty, inner_type, true))
    }

    /// list.insert(index, elem) — receiver alloca is pre-compiled to avoid double compilation.
    pub(super) fn builtin_list_insert(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 2 {
            return Err("list.insert expects 2 arguments (index, element)".to_string());
        }
        let idx_val = self.compile_expr(&args[0])?;
        let elem_val = self.compile_expr(&args[1])?;
        match (&idx_val, &elem_val) {
            (TypedValue::Int(iv), _) => {
                let elem_fat = self.to_fat_struct(&elem_val)?;
                let lv = self.load_list(list_ptr)?;
                let result = match self.call_rt(
                    "action_list_insert",
                    &[lv.into(), (*iv).into(), elem_fat.into()],
                ) {
                    Ok(v) => v,
                    Err(e) => {
                        let _ = self.rc_free_intermediate(&elem_val);
                        return Err(e);
                    }
                };
                let rv = result.try_as_basic_value().basic().ok_or("insert failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "insert_result")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, rv).map_err(llvm_err)?;
                Ok(TypedValue::List(alloca))
            }
            _ => Err("list.insert expects an integer index".to_string()),
        }
    }

    /// list.remove(index) — receiver alloca is pre-compiled to avoid double compilation.
    pub(super) fn builtin_list_remove(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err("list.remove expects 1 argument (index)".to_string());
        }
        let idx_val = self.compile_expr(&args[0])?;
        match &idx_val {
            TypedValue::Int(iv) => {
                let lv = self.load_list(list_ptr)?;
                let result = self.call_rt("action_list_remove", &[lv.into(), (*iv).into()])?;
                let rv = result.try_as_basic_value().basic().ok_or("remove failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "remove_result")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, rv).map_err(llvm_err)?;
                Ok(TypedValue::List(alloca))
            }
            _ => Err("list.remove expects an integer index".to_string()),
        }
    }

    /// list.append(elem) — receiver alloca is pre-compiled to avoid double compilation.
    pub(super) fn builtin_list_append(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err("list.append expects 1 argument (element)".to_string());
        }
        let elem_val = self.compile_expr(&args[0])?;
        let elem_fat = self.to_fat_struct(&elem_val)?;
        let lv = self.load_list(list_ptr)?;
        let cc = match self.call_rt("action_list_push", &[lv.into(), elem_fat.into()]) {
            Ok(v) => v,
            Err(e) => {
                let _ = self.rc_free_intermediate(&elem_val);
                return Err(e);
            }
        };
        let new_list = cc.try_as_basic_value().basic().ok_or("list_push failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "appended")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, new_list)
            .map_err(llvm_err)?;
        Ok(TypedValue::List(alloca))
    }
}
