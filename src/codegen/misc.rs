// Submodule: misc

use crate::ast::*;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, InnerType, Scope, TypedValue, ValKind};

impl<'ctx> CodeGen<'ctx> {
    /// Compile null literal: returns a nullable value with null flag set to 1.
    /// Without type context, returns a simple {i1=1, i64=0} as a generic null.
    /// The caller should wrap this in the correct nullable type when the target is known.
    pub(super) fn compile_null(&mut self) -> Result<TypedValue<'ctx>, String> {
        // Create a generic nullable {i8=1, i64=undef} — i8=1 means "null"
        let generic_nullable = self
            .context
            .struct_type(&[self.null_flag_ty().into(), self.i64_ty().into()], false);
        let alloca = self
            .builder
            .build_alloca(generic_nullable, "null_val")
            .map_err(llvm_err)?;
        let undef = generic_nullable.get_undef();
        let with_flag = self
            .builder
            .build_insert_value(undef, self.null_flag_ty().const_int(1, false), 0, "null_flag")
            .map_err(llvm_err)?;
        let null_val = self
            .builder
            .build_insert_value(with_flag, self.i64_ty().const_int(0, false), 1, "null_val")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, null_val)
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(
            alloca,
            generic_nullable.into(),
        ))
    }

    /// Compile a method call on a nullable receiver with auto short-circuit.
    /// If the receiver is null, returns null. Otherwise, extracts the inner
    /// non-null value and calls the method on it, wrapping the result in nullable.
    pub(super) fn compile_nullable_method_call(
        &mut self,
        nullable_ptr: PointerValue<'ctx>,
        inner_bt: BasicTypeEnum<'ctx>,
        _receiver: &Expr,
        method: &str,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot call method outside function")?;

        let nullable_st = inner_bt.into_struct_type();
        let null_bt: BasicTypeEnum = nullable_st.into();
        let loaded = self
            .builder
            .build_load(null_bt, nullable_ptr, "nmc_ld")
            .map_err(llvm_err)?;
        let null_sv = loaded.into_struct_value();
        let null_flag = self
            .builder
            .build_extract_value(null_sv, 0, "nmc_flag")
            .map_err(llvm_err)?
            .into_int_value();

        let b1 = self.null_flag_ty();
        let is_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                null_flag,
                b1.const_int(1, false),
                "nmc_is_null",
            )
            .map_err(llvm_err)?;

        let null_block = self.context.append_basic_block(current_fn, "nmc_null");
        let val_block = self.context.append_basic_block(current_fn, "nmc_val");
        let merge_block = self.context.append_basic_block(current_fn, "nmc_merge");

        self.builder
            .build_conditional_branch(is_null, null_block, val_block)
            .map_err(llvm_err)?;

        // Value path: extract inner, create synthetic scope variable,
        // call method dispatch, wrap result in nullable.
        // Processed first so the wrapped result type informs the null path.
        self.builder.position_at_end(val_block);
        let inner_val = self
            .builder
            .build_extract_value(null_sv, 1, "nmc_inner")
            .map_err(llvm_err)?;
        let mut inner_typed = self.bv_to_typed(inner_val)?;

        // bv_to_typed treats {ptr,i64,i64} as List by default, but the receiver
        // may be a typed nullable (e.g. Map?, Set?).  Look up the AST type
        // annotation to correct the ValKind before we push the synthetic scope var.
        if let Expr::Ident(recv_name) = _receiver {
            if let Some(sv) = self.scope.get(recv_name) {
                if let Some(Type::Nullable(inner_ast)) = &sv.ast_type {
                    let ptr = match &inner_typed {
                        TypedValue::List(p) => Some(*p),
                        _ => None,
                    };
                    if let Some(p) = ptr {
                        let is_map = match inner_ast.as_ref() {
                            Type::Map(..) => true,
                            Type::Named(n) if n == "Map" => true,
                            Type::Generic(b, _) => matches!(b.as_ref(), Type::Named(n) if n == "Map"),
                            _ => false,
                        };
                        let is_set = match inner_ast.as_ref() {
                            Type::Set(..) => true,
                            Type::Named(n) if n == "Set" => true,
                            Type::Generic(b, _) => matches!(b.as_ref(), Type::Named(n) if n == "Set"),
                            _ => false,
                        };
                        if is_map {
                            inner_typed = TypedValue::Map(p);
                        } else if is_set {
                            inner_typed = TypedValue::Set(p);
                        }
                    }
                }
            }
        }

        // Determine ValKind from the inner TypedValue
        let inner_kind = match &inner_typed {
            TypedValue::Int(_) => ValKind::Int,
            TypedValue::Float(_) => ValKind::Float,
            TypedValue::Bool(_) => ValKind::Bool,
            TypedValue::Str(_) => ValKind::Str,
            TypedValue::List(_) => ValKind::List,
            TypedValue::Map(_) => ValKind::Map,
            TypedValue::Set(_) => ValKind::Set,
            TypedValue::LazyList(_) => ValKind::LazyList,
            TypedValue::Struct(..) => ValKind::Struct,
            TypedValue::Enum(..) => ValKind::Enum,
            TypedValue::Stream(_) => ValKind::Stream,
            TypedValue::Task(_) => ValKind::Task,
            TypedValue::Fn(..) | TypedValue::Closure { .. } => ValKind::Fn,
            TypedValue::Ptr(_) => ValKind::Ptr,
            TypedValue::CString(_) => ValKind::CString,
            TypedValue::FileHandle(_) => ValKind::FileHandle,
            TypedValue::Nullable(..) => ValKind::Nullable,
            TypedValue::Unit => ValKind::Unit,
        };

        // inner_bt is the full nullable struct type {i8, T} — use the actual
        // inner value's type (T) for the scope variable alloca
        let actual_inner_bt = inner_typed.get_type_for_alloca(self);

        // Create alloca and store inner value for scope lookup
        let inner_alloca = self
            .builder
            .build_alloca(actual_inner_bt, "nmc_inner_a")
            .map_err(llvm_err)?;
        // Load the inner value properly — to_bv() returns None for complex
        // types (Str, List, Map, etc.), so we must load from the alloca.
        let inner_bv = match &inner_typed {
            TypedValue::Str(ptr) => self
                .builder
                .build_load(self.string_type, *ptr, "nmc_ld_inner")
                .map_err(llvm_err)?
                .into(),
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => self
                .builder
                .build_load(self.list_type, *ptr, "nmc_ld_inner")
                .map_err(llvm_err)?
                .into(),
            TypedValue::LazyList(ptr) => self
                .builder
                .build_load(self.lazylist_type, *ptr, "nmc_ld_inner")
                .map_err(llvm_err)?
                .into(),
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                self.builder
                    .build_load(bt, *ptr, "nmc_ld_inner")
                    .map_err(llvm_err)?
            }
            TypedValue::Enum(ptr, et, ..) => {
                let bt: BasicTypeEnum = (*et).into();
                self.builder
                    .build_load(bt, *ptr, "nmc_ld_inner")
                    .map_err(llvm_err)?
            }
            _ => inner_typed
                .to_bv()
                .unwrap_or_else(|| self.i64_ty().const_int(0, false).into()),
        };
        self.builder
            .build_store(inner_alloca, inner_bv)
            .map_err(llvm_err)?;

        // Push synthetic scope variable
        let counter = self.synthetic_counter;
        self.synthetic_counter += 1;
        let synthetic_name = format!("__nmc_{}", counter);
        self.scope.set(
            synthetic_name.clone(),
            inner_alloca,
            actual_inner_bt,
            inner_kind,
        );

        // Build synthetic FieldAccess and recurse into method dispatch.
        // If dispatch fails (e.g., inner type is generic i64 from a null literal
        // that lacks concrete type info), use Int(0) as a fallback result.
        // The null path will always be taken at runtime in that case anyway.
        let syn_func = Expr::FieldAccess(
            Box::new(Expr::Ident(synthetic_name.clone())),
            method.to_string(),
        );
        let method_result = match self.compile_call(&syn_func, args, trailing) {
            Ok(v) => v,
            Err(_e) => {
                TypedValue::Int(self.i64_ty().const_int(0, false))
            }
        };
        self.scope.remove_var(&synthetic_name);

        // If the method already returns Nullable (e.g. head/last),
        // don't double-wrap — use the method's result type directly.
        // The null path creates null of the same type.
        if let TypedValue::Nullable(method_ptr, method_bt) = &method_result {
            let val_loaded = self
                .builder
                .build_load(*method_bt, *method_ptr, "nmc_val_ld")
                .map_err(llvm_err)?;
            let val_end_block = self
                .builder
                .get_insert_block()
                .ok_or("no insert block after val path")?;
            self.builder
                .build_unconditional_branch(merge_block)
                .map_err(llvm_err)?;

            // Null path: create null of the same nullable type
            self.builder.position_at_end(null_block);
            let method_st = method_bt.into_struct_type();
            let undef = method_st.get_undef();
            let null_struct = self
                .builder
                .build_insert_value(undef, b1.const_int(1, false), 0, "nmc_null_f")
                .map_err(llvm_err)?;
            self.builder
                .build_unconditional_branch(merge_block)
                .map_err(llvm_err)?;

            // Merge
            self.builder.position_at_end(merge_block);
            let phi = self
                .builder
                .build_phi(*method_bt, "nmc_merge")
                .map_err(llvm_err)?;
            phi.add_incoming(&[(&null_struct, null_block), (&val_loaded, val_end_block)]);

            let result_alloca = self
                .builder
                .build_alloca(method_st, "nmc_result")
                .map_err(llvm_err)?;
            self.builder
                .build_store(result_alloca, phi.as_basic_value())
                .map_err(llvm_err)?;
            return Ok(TypedValue::Nullable(result_alloca, *method_bt));
        }

        let result_bt = method_result.get_type_for_alloca(self);
        let nty = self.get_nullable_type(
            result_bt,
            &format!("__nmc_res_{}", self.synthetic_counter),
        );
        let wrapped = self.wrap_in_nullable(&method_result, nty)?;
        let (wrapped_ptr, wrapped_bt) = match &wrapped {
            TypedValue::Nullable(p, t) => (*p, *t),
            _ => {
                return Err(
                    "wrap_in_typed_nullable did not return Nullable".to_string(),
                )
            }
        };
        let val_loaded = self
            .builder
            .build_load(wrapped_bt, wrapped_ptr, "nmc_val_ld")
            .map_err(llvm_err)?;
        let val_end_block = self
            .builder
            .get_insert_block()
            .ok_or("no insert block after val path")?;
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(llvm_err)?;

        // Null path: produce null of the same wrapped type as the value path
        self.builder.position_at_end(null_block);
        let wrapped_st = wrapped_bt.into_struct_type();
        let undef = wrapped_st.get_undef();
        let null_struct = self
            .builder
            .build_insert_value(undef, b1.const_int(1, false), 0, "nmc_null_f")
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(llvm_err)?;

        // Merge: phi the null and value paths
        self.builder.position_at_end(merge_block);
        let phi = self
            .builder
            .build_phi(wrapped_bt, "nmc_merge")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&null_struct, null_block), (&val_loaded, val_end_block)]);

        let result_alloca = self
            .builder
            .build_alloca(wrapped_st, "nmc_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(result_alloca, wrapped_bt))
    }

    /// Compile Elvis operator: condition ?: default
    /// If condition is null (flag=1), return default; otherwise return condition inner value
    pub(super) fn compile_elvis(
        &mut self,
        condition: &Expr,
        default: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let cond_val = self.compile_expr(condition)?;
        let (cond_ptr, cond_ty) = match &cond_val {
            TypedValue::Nullable(p, t) => (*p, *t),
            _ => return Ok(cond_val), // not nullable, just return condition as-is
        };

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile elvis outside function")?;

        let loaded = self
            .builder
            .build_load(cond_ty, cond_ptr, "elvis_ld")
            .map_err(llvm_err)?;
        let nullable_struct = loaded.into_struct_value();
        let null_flag = self
            .builder
            .build_extract_value(nullable_struct, 0, "elvis_flag")
            .map_err(llvm_err)?
            .into_int_value();

        let is_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                null_flag,
                self.null_flag_ty().const_int(1, false),
                "is_null",
            )
            .map_err(llvm_err)?;

        let null_block = self.context.append_basic_block(current_fn, "elvis_null");
        let val_block = self.context.append_basic_block(current_fn, "elvis_val");
        let merge_block = self.context.append_basic_block(current_fn, "elvis_merge");

        self.builder
            .build_conditional_branch(is_null, null_block, val_block)
            .map_err(llvm_err)?;

        // Null path: evaluate and return default
        self.builder.position_at_end(null_block);
        let default_val = self.compile_expr(default)?;
        let default_ty = default_val.get_type_for_alloca(self);
        let default_bv = match &default_val {
            TypedValue::Str(ptr) => self
                .builder
                .build_load(self.string_type, *ptr, "elvis_def_ld")
                .map_err(llvm_err)?
                .as_basic_value_enum(),
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                self.builder
                    .build_load(bt, *ptr, "elvis_def_ld")
                    .map_err(llvm_err)?
            }
            TypedValue::Enum(ptr, et, ..) => {
                let bt: BasicTypeEnum = (*et).into();
                self.builder
                    .build_load(bt, *ptr, "elvis_def_ld")
                    .map_err(llvm_err)?
            }
            TypedValue::Nullable(ptr, ty) => self
                .builder
                .build_load(*ty, *ptr, "elvis_def_ld")
                .map_err(llvm_err)?,
            TypedValue::Bool(v) => {
                // Bool values may be i64 but default_ty is i1; truncate to match.
                if matches!(default_ty, BasicTypeEnum::IntType(t) if t.get_bit_width() == 1) {
                    self.builder
                        .build_int_truncate(*v, self.bool_ty(), "elvis_def_bool")
                        .map_err(llvm_err)?
                        .as_basic_value_enum()
                } else {
                    (*v).as_basic_value_enum()
                }
            }
            _ => default_val
                .to_bv()
                .unwrap_or_else(|| self.i64_ty().const_int(0, false).into()),
        };
        let default_is_nullable = matches!(&default_val, TypedValue::Nullable(..));
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(llvm_err)?;

        // Non-null path: extract inner value
        self.builder.position_at_end(val_block);
        let inner_val = self
            .builder
            .build_extract_value(nullable_struct, 1, "elvis_inner")
            .map_err(llvm_err)?;
        // When inner value is a fat return struct ({i64, ptr} aka string_type)
        // but the default is a scalar, extract the i64 tag from the fat struct.
        // This handles builtins like head/last that return Nullable<FatReturn>.
        let inner_bv = if default_is_nullable {
            let struct_ty = default_ty.into_struct_type();
            let undef = struct_ty.get_undef();
            let with_flag = self
                .builder
                .build_insert_value(undef, self.null_flag_ty().const_int(0, false), 0, "ei_flag")
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(with_flag, inner_val, 1, "ei_wrapped")
                .map_err(llvm_err)?
                .as_basic_value_enum()
        } else {
            // Only extract the i64 tag from a fat-return struct when the default
            // is a scalar (Int/Float). When the default is also a struct (e.g. String),
            // keep the inner value as-is so the PHI types match.
            let default_is_scalar = matches!(
                default_ty,
                BasicTypeEnum::IntType(_) | BasicTypeEnum::FloatType(_)
            );
            let val = match inner_val {
                BasicValueEnum::StructValue(sv)
                    if sv.get_type() == self.string_type && default_is_scalar =>
                {
                    self.builder
                        .build_extract_value(sv, 0, "ei_fat_i64")
                        .map_err(llvm_err)?
                }
                _ => inner_val,
            };
            // When the nullable inner type doesn't match the default type (e.g.
            // inner is i64 from a failed dispatch fallback but default is i1),
            // convert to match the PHI type.
            match (val, default_ty) {
                (BasicValueEnum::IntValue(iv), BasicTypeEnum::IntType(it))
                    if iv.get_type() != it =>
                {
                    self.builder
                        .build_int_truncate(iv, it, "ei_conv")
                        .map_err(llvm_err)?
                        .as_basic_value_enum()
                }
                _ => val,
            }
        };
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(merge_block);
        let phi = self
            .builder
            .build_phi(default_ty, "elvis_res")
            .map_err(llvm_err)?;
        phi.add_incoming(&[
            (&default_bv, null_block),
            (&inner_bv, val_block),
        ]);

        self.bv_to_typed(phi.as_basic_value())
    }

    /// Wrap a non-null value in a nullable type: create {i1=0, value} struct
    pub(super) fn wrap_in_nullable(
        &mut self,
        value: &TypedValue<'ctx>,
        nullable_struct_ty: StructType<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let alloca = self
            .builder
            .build_alloca(nullable_struct_ty, "wrap_nullable")
            .map_err(llvm_err)?;
        let undef = nullable_struct_ty.get_undef();
        let with_flag = self
            .builder
            .build_insert_value(undef, self.null_flag_ty().const_int(0, false), 0, "wrap_flag")
            .map_err(llvm_err)?;
        let value_bv = match value {
            TypedValue::Str(ptr) => self
                .builder
                .build_load(self.string_type, *ptr, "wrap_ld")
                .map_err(llvm_err)?
                .as_basic_value_enum(),
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                self.builder
                    .build_load(bt, *ptr, "wrap_ld")
                    .map_err(llvm_err)?
            }
            TypedValue::Enum(ptr, et, ..) => {
                let bt: BasicTypeEnum = (*et).into();
                self.builder
                    .build_load(bt, *ptr, "wrap_ld")
                    .map_err(llvm_err)?
            }
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                self.load_list(*ptr)?.as_basic_value_enum()
            }
            TypedValue::Bool(v) => {
                // Bool values may be i64 (from C functions) but the nullable struct
                // field is i1. Truncate to match the field type.
                self.builder
                    .build_int_truncate(*v, self.bool_ty(), "wrap_bool")
                    .map_err(llvm_err)?
                    .as_basic_value_enum()
            }
            _ => value
                .to_bv()
                .unwrap_or_else(|| self.i64_ty().const_int(0, false).into()),
        };
        let wrapped = self
            .builder
            .build_insert_value(with_flag, value_bv, 1, "wrap_val")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, wrapped)
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(
            alloca,
            nullable_struct_ty.into(),
        ))
    }

    /// Load the null flag (field 0) from a nullable struct — 1 means null, 0 means valid.
    pub(super) fn load_null_flag(
        &mut self,
        ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<inkwell::values::IntValue<'ctx>, String> {
        let loaded = self.builder.build_load(ty, ptr, "ld_flag")
            .map_err(llvm_err)?;
        Ok(self.builder.build_extract_value(loaded.into_struct_value(), 0, "null_flag")
            .map_err(llvm_err)?
            .into_int_value())
    }

    /// Compare two nullable values for equality using branching to avoid extracting
    /// undefined inner values when either side is null.
    pub(super) fn compare_nullable_eq(
        &mut self,
        l_ptr: PointerValue<'ctx>,
        r_ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let l_flag = self.load_null_flag(l_ptr, ty)?;
        let r_flag = self.load_null_flag(r_ptr, ty)?;
        let one = self.null_flag_ty().const_int(1, false);
        let zero = self.null_flag_ty().const_int(0, false);
        let l_is_null = self.builder.build_int_compare(IntPredicate::EQ, l_flag, one, "l_is_null")
            .map_err(llvm_err)?;
        let r_is_null = self.builder.build_int_compare(IntPredicate::EQ, r_flag, one, "r_is_null")
            .map_err(llvm_err)?;
        let both_null = self.builder.build_and(l_is_null, r_is_null, "both_null")
            .map_err(llvm_err)?;
        // both_valid: l_flag==0 && r_flag==0
        let l_valid = self.builder.build_int_compare(IntPredicate::EQ, l_flag, zero, "l_valid")
            .map_err(llvm_err)?;
        let r_valid = self.builder.build_int_compare(IntPredicate::EQ, r_flag, zero, "r_valid")
            .map_err(llvm_err)?;
        let both_valid = self.builder.build_and(l_valid, r_valid, "both_valid")
            .map_err(llvm_err)?;
        // inner_eq: only meaningful when both valid — compare field 1 as same-typed values
        let struct_ty = ty.into_struct_type();
        let inner_field_ty = struct_ty.get_field_type_at_index(1)
            .ok_or("nullable struct missing field 1")?;
        let inner_eq = match inner_field_ty {
            BasicTypeEnum::IntType(_) => {
                let l_inner = self.builder.build_extract_value(
                    self.builder.build_load(ty, l_ptr, "eq_ld_l")
                        .map_err(llvm_err)?.into_struct_value(), 1, "l_inner"
                ).map_err(llvm_err)?.into_int_value();
                let r_inner = self.builder.build_extract_value(
                    self.builder.build_load(ty, r_ptr, "eq_ld_r")
                        .map_err(llvm_err)?.into_struct_value(), 1, "r_inner"
                ).map_err(llvm_err)?.into_int_value();
                self.builder.build_int_compare(IntPredicate::EQ, l_inner, r_inner, "inner_eq")
                    .map_err(llvm_err)?
            }
            BasicTypeEnum::FloatType(_) => {
                let l_inner = self.builder.build_extract_value(
                    self.builder.build_load(ty, l_ptr, "eq_ld_l")
                        .map_err(llvm_err)?.into_struct_value(), 1, "l_inner"
                ).map_err(llvm_err)?.into_float_value();
                let r_inner = self.builder.build_extract_value(
                    self.builder.build_load(ty, r_ptr, "eq_ld_r")
                        .map_err(llvm_err)?.into_struct_value(), 1, "r_inner"
                ).map_err(llvm_err)?.into_float_value();
                self.builder.build_float_compare(inkwell::FloatPredicate::OEQ, l_inner, r_inner, "inner_eq")
                    .map_err(llvm_err)?
            }
            _ => {
                // For struct types (String, etc.), fall back to ptr comparison or assume equal
                // In practice, this path is only hit when both values are valid and neither is null
                // For simplicity, compare the raw bytes via pointer equality (conservative)
                let l_ptr_int = self.builder.build_ptr_to_int(l_ptr, self.i64_ty(), "l_ptr_i")
                    .map_err(llvm_err)?;
                let r_ptr_int = self.builder.build_ptr_to_int(r_ptr, self.i64_ty(), "r_ptr_i")
                    .map_err(llvm_err)?;
                self.builder.build_int_compare(IntPredicate::EQ, l_ptr_int, r_ptr_int, "ptr_eq")
                    .map_err(llvm_err)?
            }
        };
        let valid_eq = self.builder.build_and(both_valid, inner_eq, "valid_eq")
            .map_err(llvm_err)?;
        Ok(TypedValue::Bool(
            self.builder.build_or(both_null, valid_eq, "nullable_eq")
                .map_err(llvm_err)?,
        ))
    }

    /// Compare two nullable values for inequality.
    pub(super) fn compare_nullable_neq(
        &mut self,
        l_ptr: PointerValue<'ctx>,
        r_ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let l_flag = self.load_null_flag(l_ptr, ty)?;
        let r_flag = self.load_null_flag(r_ptr, ty)?;
        let one = self.null_flag_ty().const_int(1, false);
        let zero = self.null_flag_ty().const_int(0, false);
        let l_is_null = self.builder.build_int_compare(IntPredicate::EQ, l_flag, one, "l_is_null")
            .map_err(llvm_err)?;
        let r_is_null = self.builder.build_int_compare(IntPredicate::EQ, r_flag, one, "r_is_null")
            .map_err(llvm_err)?;
        // xor: exactly one is null → not equal
        let one_null = self.builder.build_xor(l_is_null, r_is_null, "one_null")
            .map_err(llvm_err)?;
        // both_valid
        let l_valid = self.builder.build_int_compare(IntPredicate::EQ, l_flag, zero, "l_valid_ne")
            .map_err(llvm_err)?;
        let r_valid = self.builder.build_int_compare(IntPredicate::EQ, r_flag, zero, "r_valid_ne")
            .map_err(llvm_err)?;
        let both_valid = self.builder.build_and(l_valid, r_valid, "both_valid_ne")
            .map_err(llvm_err)?;
        let struct_ty = ty.into_struct_type();
        let inner_field_ty = struct_ty.get_field_type_at_index(1)
            .ok_or("nullable struct missing field 1")?;
        let inner_ne = match inner_field_ty {
            BasicTypeEnum::IntType(_) => {
                let l_inner = self.builder.build_extract_value(
                    self.builder.build_load(ty, l_ptr, "ne_ld_l")
                        .map_err(llvm_err)?.into_struct_value(), 1, "l_inner"
                ).map_err(llvm_err)?.into_int_value();
                let r_inner = self.builder.build_extract_value(
                    self.builder.build_load(ty, r_ptr, "ne_ld_r")
                        .map_err(llvm_err)?.into_struct_value(), 1, "r_inner"
                ).map_err(llvm_err)?.into_int_value();
                self.builder.build_int_compare(IntPredicate::NE, l_inner, r_inner, "inner_ne")
                    .map_err(llvm_err)?
            }
            BasicTypeEnum::FloatType(_) => {
                let l_inner = self.builder.build_extract_value(
                    self.builder.build_load(ty, l_ptr, "ne_ld_l")
                        .map_err(llvm_err)?.into_struct_value(), 1, "l_inner"
                ).map_err(llvm_err)?.into_float_value();
                let r_inner = self.builder.build_extract_value(
                    self.builder.build_load(ty, r_ptr, "ne_ld_r")
                        .map_err(llvm_err)?.into_struct_value(), 1, "r_inner"
                ).map_err(llvm_err)?.into_float_value();
                self.builder.build_float_compare(inkwell::FloatPredicate::ONE, l_inner, r_inner, "inner_ne")
                    .map_err(llvm_err)?
            }
            _ => {
                let l_ptr_int = self.builder.build_ptr_to_int(l_ptr, self.i64_ty(), "l_ptr_i_ne")
                    .map_err(llvm_err)?;
                let r_ptr_int = self.builder.build_ptr_to_int(r_ptr, self.i64_ty(), "r_ptr_i_ne")
                    .map_err(llvm_err)?;
                self.builder.build_int_compare(IntPredicate::NE, l_ptr_int, r_ptr_int, "ptr_ne")
                    .map_err(llvm_err)?
            }
        };
        let valid_ne = self.builder.build_and(both_valid, inner_ne, "valid_ne")
            .map_err(llvm_err)?;
        Ok(TypedValue::Bool(
            self.builder.build_or(one_null, valid_ne, "nullable_ne")
                .map_err(llvm_err)?,
        ))
    }

    /// Access a field on an already-compiled TypedValue (non-nullable).
    fn compile_field_access_on_typed_value(
        &mut self,
        val: &TypedValue<'ctx>,
        field: &str,
        _val_bt: BasicTypeEnum<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        if let TypedValue::Str(ptr) = val {
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
        if let TypedValue::Struct(ptr, struct_ty) = val {
            let bt: BasicTypeEnum = (*struct_ty).into();
            let loaded = self
                .builder
                .build_load(bt, *ptr, "fa_tv_ld")
                .map_err(llvm_err)?;
            let struct_val = loaded.into_struct_value();

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

    /// Wrap a TypedValue in a nullable struct of matching LLVM type.
    fn wrap_in_typed_nullable(
        &mut self,
        value: &TypedValue<'ctx>,
        inner_bt: BasicTypeEnum<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let b1 = self.null_flag_ty();
        let nullable_fields: &[BasicTypeEnum] = &[b1.into(), inner_bt];
        let nullable_ty = self.context.struct_type(nullable_fields, false);

        let alloca = self
            .builder
            .build_alloca(nullable_ty, "wrap_tv")
            .map_err(llvm_err)?;
        let undef = nullable_ty.get_undef();
        let with_flag = self
            .builder
            .build_insert_value(undef, b1.const_int(0, false), 0, "wrap_tv_flag")
            .map_err(llvm_err)?;
        let value_bv = value
            .to_bv()
            .unwrap_or_else(|| self.i64_ty().const_int(0, false).into());
        let wrapped = self
            .builder
            .build_insert_value(with_flag, value_bv, 1, "wrap_tv_val")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, wrapped)
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
    }

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
                    let cc = self.call_rt(
                        "action_list_get",
                        &[list_val.into(), index_val.into()],
                    )?;
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
                    .build_load(i8, char_ptr, "char")
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
        let key_val2 = self.compile_expr(idx)?;
        let key_fat2 = self.to_fat_struct(&key_val2)?;
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
            .build_insert_value(undef, self.null_flag_ty().const_int(0, false), 0, "some_flag")
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
            .build_insert_value(undef2, self.null_flag_ty().const_int(1, false), 0, "none_flag")
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
        let elem_val2 = self.compile_expr(idx)?;
        let elem_fat2 = self.to_fat_struct(&elem_val2)?;
        // Extract actual value (field 0) from fat struct {val, ptr}
        let actual_val = self
            .builder
            .build_extract_value(elem_fat2.into_struct_value(), 0, "set_val")
            .map_err(llvm_err)?
            .into_int_value();
        let undef = nullable_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, self.null_flag_ty().const_int(0, false), 0, "some_flag")
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
            .build_insert_value(undef2, self.null_flag_ty().const_int(1, false), 0, "none_flag")
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

        let mut last = TypedValue::Unit;
        for (_i, s) in stmts.iter().enumerate() {
            match s {
                Stmt::Expr { expr: e, .. } => last = self.compile_expr(e)?,
                _ => self.compile_stmt(s)?,
            }
        }

        // RC inc the return value before cleaning up the scope.
        // Without this, emit_scope_cleanup would rc_dec the variable being
        // returned (e.g. `var r = ...; r`), freeing its data before the
        // caller can take ownership.
        self.rc_inc_typed_value(&last)?;

        // RC cleanup: decrement refcounts on heap-typed variables in this scope
        self.emit_scope_cleanup()?;

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
        match target {
            Expr::Ident(name) => {
                let (var_ptr, var_kind, var_ty, var_rc_managed) = {
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
                    (var.ptr, var.kind, var.ty, var.enum_data_rc_managed)
                };
                // Wrap non-nullable value into nullable when target is nullable
                let v = if var_kind == ValKind::Nullable && !matches!(&v, TypedValue::Nullable(..)) {
                    let inner_bt = v.get_type_for_alloca(self);
                    let nty = self.get_nullable_type(inner_bt, "assign_wrap");
                    self.wrap_in_nullable(&v, nty)?
                } else {
                    v
                };
                // Decrement RC of old value before overwriting
                self.rc_dec_at(var_ptr, var_kind, var_ty, var_rc_managed)?;
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
                // Increment RC of new value
                self.rc_inc_typed_value(&v)?;
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
                                if let Some(bv) = v.to_bv() {
                                    self.builder
                                        .build_store(field_ptr, bv)
                                        .map_err(llvm_err)?;
                                }
                                // Write back the modified inner struct into the nullable
                                let inner_st_bt: BasicTypeEnum = st.into();
                                let updated_inner = self
                                    .builder
                                    .build_load(inner_st_bt, ptr, "asn_upd")
                                    .map_err(llvm_err)?;
                                let updated_nf = self
                                    .builder
                                    .build_insert_value(
                                        nf_struct,
                                        updated_inner,
                                        1,
                                        "asn_nf_upd",
                                    )
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

    /// Get the field index within a struct type by field name
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
                        // Decrement old accumulator's RC since the concat result owns a new ref
                        let old_str = self.load_string(acc)?;
                        let old_data = self
                            .builder
                            .build_extract_value(old_str, 1, "old_data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        self.rc_dec(old_data)?;
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
            let field_bt = field_result.get_type_for_alloca(self);
            let field_wrapped = self.wrap_in_typed_nullable(&field_result, field_bt)?;
            let (wrapped_ptr, wrapped_bt) = match &field_wrapped {
                TypedValue::Nullable(p, t) => (*p, *t),
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
            let field_tys: Vec<BasicTypeEnum> = field_vals
                .iter()
                .map(|v| v.get_type_for_alloca(self))
                .collect();
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
                        val.to_bv()
                            .unwrap_or_else(|| self.i64_ty().const_int(0, false).as_basic_value_enum())
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
        let mut field_tys: Vec<BasicTypeEnum> = Vec::new();
        let mut values: Vec<TypedValue<'ctx>> = Vec::new();
        let mut field_names: Vec<String> = Vec::new();
        for (name_opt, expr) in exprs {
            let val = self.compile_expr(expr)?;
            field_tys.push(val.get_type_for_alloca(self));
            values.push(val);
            if let Some(name) = name_opt {
                field_names.push(name.clone());
            } else {
                field_names.push(format!("_{}", field_names.len()));
            }
        }
        let struct_ty = self.context.struct_type(&field_tys, false);
        // Register in anon_structs so field access by name works
        self.anon_structs.entry(field_names).or_insert(struct_ty);
        let bt: BasicTypeEnum = struct_ty.into();
        let alloca = self.builder.build_alloca(bt, "tuple").map_err(llvm_err)?;

        let undef = struct_ty.get_undef();
        let mut result = undef;
        for (i, val) in values.iter().enumerate() {
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
            result = self
                .builder
                .build_insert_value(result, bv, i as u32, "tuple_elem")
                .map_err(llvm_err)?
                .into_struct_value();
        }
        self.builder.build_store(alloca, result).map_err(llvm_err)?;
        Ok(TypedValue::Struct(alloca, struct_ty))
    }

    /// Convert a compile result to a fat {i64, ptr} struct value for map/set runtime calls
    pub(super) fn to_fat_struct(
        &mut self,
        val: &TypedValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match val {
            TypedValue::Str(ptr) => Ok(self.load_string(*ptr)?.into()),
            TypedValue::Enum(ptr, ty, ..) => {
                let bt: BasicTypeEnum = (*ty).into();
                Ok(self
                    .builder
                    .build_load(bt, *ptr, "fat_enum")
                    .map_err(llvm_err)?)
            }
            TypedValue::Struct(ptr, ty) => {
                let bt: BasicTypeEnum = (*ty).into();
                Ok(self
                    .builder
                    .build_load(bt, *ptr, "fat_struct")
                    .map_err(llvm_err)?)
            }
            TypedValue::List(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(6, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            TypedValue::Map(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(7, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            TypedValue::Set(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(8, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            TypedValue::Task(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(9, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            TypedValue::Stream(ptr) => {
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, self.i64_ty().const_int(10, false), 0, "tag")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, *ptr, 1, "data")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
            _ => {
                // Scalar value: wrap in {scalar, null}
                let bv = val
                    .to_bv()
                    .unwrap_or_else(|| self.i64_ty().const_int(0, false).into());
                // Coerce Float/Bool to i64 (field 0 of string_type is i64)
                let coerced: BasicValueEnum = match bv {
                    BasicValueEnum::FloatValue(fv) => {
                        let tmp = self
                            .builder
                            .build_alloca(self.f64_ty(), "ftmp")
                            .map_err(llvm_err)?;
                        self.builder.build_store(tmp, fv).map_err(llvm_err)?;
                        let casted = self
                            .builder
                            .build_pointer_cast(tmp, self.ptr_ty(), "fcast")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_load(self.i64_ty(), casted, "fbits")
                            .map_err(llvm_err)?
                    }
                    BasicValueEnum::IntValue(iv) if iv.get_type().get_bit_width() == 1 => self
                        .builder
                        .build_int_z_extend(iv, self.i64_ty(), "b2i")
                        .map_err(llvm_err)?
                        .as_basic_value_enum(),
                    _ => bv,
                };
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, coerced, 0, "wrap0")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, self.ptr_ty().const_zero(), 1, "wrap1")
                    .map_err(llvm_err)?;
                Ok(r2.as_basic_value_enum())
            }
        }
    }
}
