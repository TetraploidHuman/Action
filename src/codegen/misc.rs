// Submodule: misc

use crate::ast::*;
use inkwell::types::BasicType;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, InnerType, Scope, TypedValue, ValKind};
use inkwell::types::BasicMetadataTypeEnum;
use inkwell::types::FunctionType;
use inkwell::values::{BasicMetadataValueEnum, IntValue};

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
            .build_insert_value(
                undef,
                self.null_flag_ty().const_int(1, false),
                0,
                "null_flag",
            )
            .map_err(llvm_err)?;
        let null_val = self
            .builder
            .build_insert_value(with_flag, self.i64_ty().const_int(0, false), 1, "null_val")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, null_val)
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, generic_nullable.into()))
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
                            Type::Named(n) if n == "map" => true,
                            Type::Generic(b, _) => {
                                matches!(b.as_ref(), Type::Named(n) if n == "map")
                            }
                            _ => false,
                        };
                        let is_set = match inner_ast.as_ref() {
                            Type::Set(..) => true,
                            Type::Named(n) if n == "set" => true,
                            Type::Generic(b, _) => {
                                matches!(b.as_ref(), Type::Named(n) if n == "set")
                            }
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
        let actual_inner_bt = inner_typed.get_value_type(self);

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
        // compile_call may fail when the inner type is generic (e.g. null literal
        // with no type annotation) because the method can't be resolved on i64.
        // The null path is always taken at runtime, so the method result is unused.
        let method_result = match self.compile_call(&syn_func, args, trailing) {
            Ok(v) => v,
            Err(_) => TypedValue::Int(self.i64_ty().const_int(0, false)),
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

        let result_bt = method_result.get_value_type(self);
        let nty =
            self.get_nullable_type(result_bt, &format!("__nmc_res_{}", self.synthetic_counter));
        let wrapped = self.wrap_in_nullable(&method_result, nty)?;
        let (wrapped_ptr, wrapped_bt) = match &wrapped {
            TypedValue::Nullable(p, t) => (*p, *t),
            _ => return Err("wrap_in_typed_nullable did not return Nullable".to_string()),
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

    /// Compile or-block: nullable or { fallback }
    /// If nullable is null (flag=1), return fallback; otherwise return inner value
    pub(super) fn compile_or_block(
        &mut self,
        nullable: &Expr,
        fallback: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let cond_val = self.compile_expr(nullable)?;
        let (cond_ptr, cond_ty) = match &cond_val {
            TypedValue::Nullable(p, t) => (*p, *t),
            _ => return Ok(cond_val), // not nullable, just return as-is
        };

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile or-block outside function")?;

        let loaded = self
            .builder
            .build_load(cond_ty, cond_ptr, "orblk_ld")
            .map_err(llvm_err)?;
        let nullable_struct = loaded.into_struct_value();
        let null_flag = self
            .builder
            .build_extract_value(nullable_struct, 0, "orblk_flag")
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

        let null_block = self.context.append_basic_block(current_fn, "orblk_null");
        let val_block = self.context.append_basic_block(current_fn, "orblk_val");
        let merge_block = self.context.append_basic_block(current_fn, "orblk_merge");

        self.builder
            .build_conditional_branch(is_null, null_block, val_block)
            .map_err(llvm_err)?;

        // Null path: evaluate and return fallback
        self.builder.position_at_end(null_block);
        let default_val = self.compile_expr(fallback)?;
        let default_is_nullable = matches!(&default_val, TypedValue::Nullable(..));
        // When the default is nullable, the result is also nullable so the PHI
        // type is the full nullable struct. Otherwise use the inner type of the
        // condition's nullable struct (not get_type_for_alloca, which returns ptr).
        let phi_ty = if default_is_nullable {
            default_val.get_value_type(self)
        } else if let BasicTypeEnum::StructType(st) = cond_ty {
            st.get_field_types()
                .get(1)
                .copied()
                .unwrap_or(default_val.get_type_for_alloca(self))
        } else {
            default_val.get_type_for_alloca(self)
        };
        let default_bv = match &default_val {
            TypedValue::Str(ptr) => self
                .builder
                .build_load(self.string_type, *ptr, "orblk_def_ld")
                .map_err(llvm_err)?
                .as_basic_value_enum(),
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                self.builder
                    .build_load(bt, *ptr, "orblk_def_ld")
                    .map_err(llvm_err)?
            }
            TypedValue::Enum(ptr, et, ..) => {
                let bt: BasicTypeEnum = (*et).into();
                self.builder
                    .build_load(bt, *ptr, "orblk_def_ld")
                    .map_err(llvm_err)?
            }
            TypedValue::Nullable(ptr, ty) => self
                .builder
                .build_load(*ty, *ptr, "orblk_def_ld")
                .map_err(llvm_err)?,
            TypedValue::Bool(v) => {
                // Bool values may be i64 (from C runtime) or i1; coerce to match phi_ty.
                if matches!(phi_ty, BasicTypeEnum::IntType(t) if t.get_bit_width() == 1) {
                    self.builder
                        .build_int_truncate(*v, self.bool_ty(), "orblk_def_bool")
                        .map_err(llvm_err)?
                        .as_basic_value_enum()
                } else if matches!(phi_ty, BasicTypeEnum::IntType(t) if t.get_bit_width() == 64) {
                    self.builder
                        .build_int_z_extend(*v, self.i64_ty(), "orblk_def_bool64")
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
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(llvm_err)?;

        // Non-null path: extract inner value
        self.builder.position_at_end(val_block);
        let inner_val = self
            .builder
            .build_extract_value(nullable_struct, 1, "orblk_inner")
            .map_err(llvm_err)?;
        // When inner value is a fat return struct ({i64, ptr} aka string_type)
        // but the default is a scalar, extract the i64 tag from the fat struct.
        // This handles builtins like head/last that return Nullable<FatReturn>.
        let inner_bv = if default_is_nullable {
            let struct_ty = phi_ty.into_struct_type();
            let undef = struct_ty.get_undef();
            let with_flag = self
                .builder
                .build_insert_value(
                    undef,
                    self.null_flag_ty().const_int(0, false),
                    0,
                    "orblk_flag",
                )
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(with_flag, inner_val, 1, "orblk_wrapped")
                .map_err(llvm_err)?
                .as_basic_value_enum()
        } else {
            // Only extract the i64 tag from a fat-return struct when the default
            // is a scalar (Int/Float). When the default is also a struct (e.g. String),
            // keep the inner value as-is so the PHI types match.
            let default_is_scalar = matches!(
                phi_ty,
                BasicTypeEnum::IntType(_) | BasicTypeEnum::FloatType(_)
            );
            let val = match inner_val {
                BasicValueEnum::StructValue(sv)
                    if sv.get_type() == self.string_type && default_is_scalar =>
                {
                    self.builder
                        .build_extract_value(sv, 0, "orblk_fat_i64")
                        .map_err(llvm_err)?
                }
                _ => inner_val,
            };
            // When the nullable inner type doesn't match the default type (e.g.
            // inner is i64 from a failed dispatch fallback but default is i1),
            // convert to match the PHI type.
            match (val, phi_ty) {
                (BasicValueEnum::IntValue(iv), BasicTypeEnum::IntType(it))
                    if iv.get_type() != it =>
                {
                    self.builder
                        .build_int_truncate(iv, it, "orblk_conv")
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
            .build_phi(phi_ty, "orblk_res")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&default_bv, null_block), (&inner_bv, val_block)]);

        let mut result = self.bv_to_typed(phi.as_basic_value())?;
        // bv_to_typed treats i64 as Int, but when the default is Bool the
        // result should be Bool (e.g. `nullableBool or { false }`).
        if matches!(&default_val, TypedValue::Bool(_)) {
            if let TypedValue::Int(v) = &result {
                let b = self
                    .builder
                    .build_int_truncate(*v, self.bool_ty(), "orblk_int2bool")
                    .map_err(llvm_err)?;
                result = TypedValue::Bool(b);
            }
        }
        Ok(result)
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
            .build_insert_value(
                undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "wrap_flag",
            )
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
        Ok(TypedValue::Nullable(alloca, nullable_struct_ty.into()))
    }

    /// Load the null flag (field 0) from a nullable struct — 1 means null, 0 means valid.
    pub(super) fn load_null_flag(
        &mut self,
        ptr: PointerValue<'ctx>,
        ty: BasicTypeEnum<'ctx>,
    ) -> Result<inkwell::values::IntValue<'ctx>, String> {
        let loaded = self
            .builder
            .build_load(ty, ptr, "ld_flag")
            .map_err(llvm_err)?;
        Ok(self
            .builder
            .build_extract_value(loaded.into_struct_value(), 0, "null_flag")
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
        let l_is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, l_flag, one, "l_is_null")
            .map_err(llvm_err)?;
        let r_is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, r_flag, one, "r_is_null")
            .map_err(llvm_err)?;
        let both_null = self
            .builder
            .build_and(l_is_null, r_is_null, "both_null")
            .map_err(llvm_err)?;
        // both_valid: l_flag==0 && r_flag==0
        let l_valid = self
            .builder
            .build_int_compare(IntPredicate::EQ, l_flag, zero, "l_valid")
            .map_err(llvm_err)?;
        let r_valid = self
            .builder
            .build_int_compare(IntPredicate::EQ, r_flag, zero, "r_valid")
            .map_err(llvm_err)?;
        let both_valid = self
            .builder
            .build_and(l_valid, r_valid, "both_valid")
            .map_err(llvm_err)?;
        // inner_eq: only meaningful when both valid — compare field 1 as same-typed values
        let struct_ty = ty.into_struct_type();
        let inner_field_ty = struct_ty
            .get_field_type_at_index(1)
            .ok_or("nullable struct missing field 1")?;
        let inner_eq = match inner_field_ty {
            BasicTypeEnum::IntType(_) => {
                let l_inner = self
                    .builder
                    .build_extract_value(
                        self.builder
                            .build_load(ty, l_ptr, "eq_ld_l")
                            .map_err(llvm_err)?
                            .into_struct_value(),
                        1,
                        "l_inner",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                let r_inner = self
                    .builder
                    .build_extract_value(
                        self.builder
                            .build_load(ty, r_ptr, "eq_ld_r")
                            .map_err(llvm_err)?
                            .into_struct_value(),
                        1,
                        "r_inner",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                self.builder
                    .build_int_compare(IntPredicate::EQ, l_inner, r_inner, "inner_eq")
                    .map_err(llvm_err)?
            }
            BasicTypeEnum::FloatType(_) => {
                let l_inner = self
                    .builder
                    .build_extract_value(
                        self.builder
                            .build_load(ty, l_ptr, "eq_ld_l")
                            .map_err(llvm_err)?
                            .into_struct_value(),
                        1,
                        "l_inner",
                    )
                    .map_err(llvm_err)?
                    .into_float_value();
                let r_inner = self
                    .builder
                    .build_extract_value(
                        self.builder
                            .build_load(ty, r_ptr, "eq_ld_r")
                            .map_err(llvm_err)?
                            .into_struct_value(),
                        1,
                        "r_inner",
                    )
                    .map_err(llvm_err)?
                    .into_float_value();
                self.builder
                    .build_float_compare(inkwell::FloatPredicate::OEQ, l_inner, r_inner, "inner_eq")
                    .map_err(llvm_err)?
            }
            _ => {
                // For struct types (String, List, user-defined structs, etc.)
                // compare the inner values byte-by-byte via memcmp.
                let l_struct_val = self
                    .builder
                    .build_load(ty, l_ptr, "eq_ld_l")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let r_struct_val = self
                    .builder
                    .build_load(ty, r_ptr, "eq_ld_r")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let l_inner = self
                    .builder
                    .build_extract_value(l_struct_val, 1, "l_inner")
                    .map_err(llvm_err)?;
                let r_inner = self
                    .builder
                    .build_extract_value(r_struct_val, 1, "r_inner")
                    .map_err(llvm_err)?;
                let inner_size = inner_field_ty
                    .size_of()
                    .ok_or("cannot get inner field size")?;
                // Stack-allocate space for the inner values so we can memcmp them
                let l_tmp = self
                    .builder
                    .build_alloca(inner_field_ty, "l_tmp")
                    .map_err(llvm_err)?;
                let r_tmp = self
                    .builder
                    .build_alloca(inner_field_ty, "r_tmp")
                    .map_err(llvm_err)?;
                self.builder.build_store(l_tmp, l_inner).map_err(llvm_err)?;
                self.builder.build_store(r_tmp, r_inner).map_err(llvm_err)?;
                let l_byte = self
                    .builder
                    .build_pointer_cast(l_tmp, self.ptr_ty(), "l_byte")
                    .map_err(llvm_err)?;
                let r_byte = self
                    .builder
                    .build_pointer_cast(r_tmp, self.ptr_ty(), "r_byte")
                    .map_err(llvm_err)?;
                // size_of returns an IntValue; zero-extend to i64 for memcmp
                let inner_size_val = self
                    .builder
                    .build_int_z_extend(inner_size, self.i64_ty(), "inner_sz")
                    .map_err(llvm_err)?;
                let memcmp_fn = self
                    .module
                    .get_function("memcmp")
                    .ok_or("memcmp not found")?;
                let cmp_result = self
                    .builder
                    .build_call(
                        memcmp_fn,
                        &[l_byte.into(), r_byte.into(), inner_size_val.into()],
                        "inner_cmp",
                    )
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();
                self.builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        cmp_result,
                        self.i32_ty().const_int(0, false),
                        "inner_eq",
                    )
                    .map_err(llvm_err)?
            }
        };
        let valid_eq = self
            .builder
            .build_and(both_valid, inner_eq, "valid_eq")
            .map_err(llvm_err)?;
        Ok(TypedValue::Bool(
            self.builder
                .build_or(both_null, valid_eq, "nullable_eq")
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
        let l_is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, l_flag, one, "l_is_null")
            .map_err(llvm_err)?;
        let r_is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, r_flag, one, "r_is_null")
            .map_err(llvm_err)?;
        // xor: exactly one is null → not equal
        let one_null = self
            .builder
            .build_xor(l_is_null, r_is_null, "one_null")
            .map_err(llvm_err)?;
        // both_valid
        let l_valid = self
            .builder
            .build_int_compare(IntPredicate::EQ, l_flag, zero, "l_valid_ne")
            .map_err(llvm_err)?;
        let r_valid = self
            .builder
            .build_int_compare(IntPredicate::EQ, r_flag, zero, "r_valid_ne")
            .map_err(llvm_err)?;
        let both_valid = self
            .builder
            .build_and(l_valid, r_valid, "both_valid_ne")
            .map_err(llvm_err)?;
        let struct_ty = ty.into_struct_type();
        let inner_field_ty = struct_ty
            .get_field_type_at_index(1)
            .ok_or("nullable struct missing field 1")?;
        let inner_ne = match inner_field_ty {
            BasicTypeEnum::IntType(_) => {
                let l_inner = self
                    .builder
                    .build_extract_value(
                        self.builder
                            .build_load(ty, l_ptr, "ne_ld_l")
                            .map_err(llvm_err)?
                            .into_struct_value(),
                        1,
                        "l_inner",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                let r_inner = self
                    .builder
                    .build_extract_value(
                        self.builder
                            .build_load(ty, r_ptr, "ne_ld_r")
                            .map_err(llvm_err)?
                            .into_struct_value(),
                        1,
                        "r_inner",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                self.builder
                    .build_int_compare(IntPredicate::NE, l_inner, r_inner, "inner_ne")
                    .map_err(llvm_err)?
            }
            BasicTypeEnum::FloatType(_) => {
                let l_inner = self
                    .builder
                    .build_extract_value(
                        self.builder
                            .build_load(ty, l_ptr, "ne_ld_l")
                            .map_err(llvm_err)?
                            .into_struct_value(),
                        1,
                        "l_inner",
                    )
                    .map_err(llvm_err)?
                    .into_float_value();
                let r_inner = self
                    .builder
                    .build_extract_value(
                        self.builder
                            .build_load(ty, r_ptr, "ne_ld_r")
                            .map_err(llvm_err)?
                            .into_struct_value(),
                        1,
                        "r_inner",
                    )
                    .map_err(llvm_err)?
                    .into_float_value();
                self.builder
                    .build_float_compare(inkwell::FloatPredicate::ONE, l_inner, r_inner, "inner_ne")
                    .map_err(llvm_err)?
            }
            _ => {
                // For struct types, compare inner values via memcmp (same as eq case)
                let l_struct_val = self
                    .builder
                    .build_load(ty, l_ptr, "ne_ld_l")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let r_struct_val = self
                    .builder
                    .build_load(ty, r_ptr, "ne_ld_r")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let l_inner = self
                    .builder
                    .build_extract_value(l_struct_val, 1, "l_inner_ne")
                    .map_err(llvm_err)?;
                let r_inner = self
                    .builder
                    .build_extract_value(r_struct_val, 1, "r_inner_ne")
                    .map_err(llvm_err)?;
                let inner_size = inner_field_ty
                    .size_of()
                    .ok_or("cannot get inner field size")?;
                let l_tmp = self
                    .builder
                    .build_alloca(inner_field_ty, "l_tmp_ne")
                    .map_err(llvm_err)?;
                let r_tmp = self
                    .builder
                    .build_alloca(inner_field_ty, "r_tmp_ne")
                    .map_err(llvm_err)?;
                self.builder.build_store(l_tmp, l_inner).map_err(llvm_err)?;
                self.builder.build_store(r_tmp, r_inner).map_err(llvm_err)?;
                let l_byte = self
                    .builder
                    .build_pointer_cast(l_tmp, self.ptr_ty(), "l_byte_ne")
                    .map_err(llvm_err)?;
                let r_byte = self
                    .builder
                    .build_pointer_cast(r_tmp, self.ptr_ty(), "r_byte_ne")
                    .map_err(llvm_err)?;
                let inner_size_val = self
                    .builder
                    .build_int_z_extend(inner_size, self.i64_ty(), "inner_sz_ne")
                    .map_err(llvm_err)?;
                let memcmp_fn = self
                    .module
                    .get_function("memcmp")
                    .ok_or("memcmp not found")?;
                let cmp_result = self
                    .builder
                    .build_call(
                        memcmp_fn,
                        &[l_byte.into(), r_byte.into(), inner_size_val.into()],
                        "inner_cmp_ne",
                    )
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();
                self.builder
                    .build_int_compare(
                        IntPredicate::NE,
                        cmp_result,
                        self.i32_ty().const_int(0, false),
                        "inner_ne",
                    )
                    .map_err(llvm_err)?
            }
        };
        let valid_ne = self
            .builder
            .build_and(both_valid, inner_ne, "valid_ne")
            .map_err(llvm_err)?;
        Ok(TypedValue::Bool(
            self.builder
                .build_or(one_null, valid_ne, "nullable_ne")
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
        let value_bv = match value {
            TypedValue::Str(ptr) => self
                .builder
                .build_load(self.string_type, *ptr, "wtv_s")
                .map_err(llvm_err)?
                .as_basic_value_enum(),
            TypedValue::Struct(ptr, st) => {
                let bt: BasicTypeEnum = (*st).into();
                self.builder
                    .build_load(bt, *ptr, "wtv_st")
                    .map_err(llvm_err)?
            }
            TypedValue::Enum(ptr, et, ..) => {
                let bt: BasicTypeEnum = (*et).into();
                self.builder
                    .build_load(bt, *ptr, "wtv_en")
                    .map_err(llvm_err)?
            }
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                self.load_list(*ptr)?.as_basic_value_enum()
            }
            TypedValue::Bool(v) => self
                .builder
                .build_int_truncate(*v, self.bool_ty(), "wtv_bool")
                .map_err(llvm_err)?
                .as_basic_value_enum(),
            _ => value
                .to_bv()
                .unwrap_or_else(|| self.i64_ty().const_int(0, false).into()),
        };
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

// ---------------------------------------------------------------------------
// Helpers extracted from mod.rs: type helpers, RC helpers, type inference, name mangling
// ---------------------------------------------------------------------------
impl<'ctx> CodeGen<'ctx> {
    pub fn set_opt_level(&mut self, level: u8) {
        self.opt_level = level.min(3);
    }

    /// Get or create a nullable LLVM struct type {i1, T} for the given inner type.
    pub(super) fn get_nullable_type(
        &mut self,
        inner_type: BasicTypeEnum<'ctx>,
        name_hint: &str,
    ) -> StructType<'ctx> {
        if let Some(ct) = self.nullable_types.get(name_hint) {
            return *ct;
        }
        let nullable_ty = self
            .context
            .struct_type(&[self.null_flag_ty().into(), inner_type], false);
        self.nullable_types
            .insert(name_hint.to_string(), nullable_ty);
        nullable_ty
    }

    /// Convert Int or Float TypedValue to FloatValue (Int gets converted via sitofp).
    pub(super) fn typed_to_float(
        &self,
        val: &TypedValue<'ctx>,
    ) -> Result<inkwell::values::FloatValue<'ctx>, String> {
        match val {
            TypedValue::Float(fv) => Ok(*fv),
            TypedValue::Int(iv) => self
                .builder
                .build_signed_int_to_float(*iv, self.f64_ty(), "i2f")
                .map_err(|e| format!("LLVM error: {}", e)),
            _ => Err("Expected Int or Float".to_string()),
        }
    }

    pub(super) fn i64_ty(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i64_type()
    }
    pub(super) fn i32_ty(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i32_type()
    }
    pub(super) fn f64_ty(&self) -> inkwell::types::FloatType<'ctx> {
        self.context.f64_type()
    }
    pub(super) fn bool_ty(&self) -> inkwell::types::IntType<'ctx> {
        self.context.bool_type()
    }
    /// i8 type for nullable struct flags — avoids LLVM i1-in-struct selection issues
    pub(super) fn null_flag_ty(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i8_type()
    }
    pub(super) fn void_ty(&self) -> inkwell::types::VoidType<'ctx> {
        self.context.void_type()
    }
    pub(super) fn ptr_ty(&self) -> inkwell::types::PointerType<'ctx> {
        self.context.ptr_type(inkwell::AddressSpace::default())
    }

    /// Compute the store size (bytes) of an LLVM type for x86-64 ABI.
    pub(super) fn type_store_size(&self, ty: BasicTypeEnum<'ctx>) -> u64 {
        match ty {
            BasicTypeEnum::IntType(it) => {
                let bw = it.get_bit_width() as u64;
                (bw + 7) / 8
            }
            BasicTypeEnum::FloatType(_) => 8,
            BasicTypeEnum::PointerType(_) => 8,
            BasicTypeEnum::StructType(st) => self.struct_store_size(st),
            BasicTypeEnum::ArrayType(at) => {
                let elem_size = self.type_store_size(at.get_element_type());
                let len = at.len() as u64;
                elem_size * len
            }
            _ => 8,
        }
    }

    fn struct_store_size(&self, st: StructType<'ctx>) -> u64 {
        let fields = st.get_field_types();
        if fields.is_empty() {
            return 0;
        }
        let mut max_align: u64 = 1;
        let mut offset: u64 = 0;
        for field in &fields {
            let f_size = self.type_store_size(*field);
            let f_align = self.type_alignment(*field);
            max_align = max_align.max(f_align);
            offset = (offset + f_align - 1) / f_align * f_align;
            offset += f_size;
        }
        (offset + max_align - 1) / max_align * max_align
    }

    fn type_alignment(&self, ty: BasicTypeEnum<'ctx>) -> u64 {
        match ty {
            BasicTypeEnum::IntType(it) => {
                let bw = it.get_bit_width() as u64;
                ((bw + 7) / 8).min(8)
            }
            BasicTypeEnum::FloatType(_) => 8,
            BasicTypeEnum::PointerType(_) => 8,
            BasicTypeEnum::StructType(st) => {
                let fields = st.get_field_types();
                if fields.is_empty() {
                    return 1;
                }
                fields
                    .iter()
                    .map(|f| self.type_alignment(*f))
                    .max()
                    .unwrap_or(8)
            }
            BasicTypeEnum::ArrayType(at) => self.type_alignment(at.get_element_type()),
            _ => 8,
        }
    }

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

    /// Extract list data pointer from a loaded list struct
    #[allow(dead_code)]
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

    /// Guess the return type from the function body expression when no annotation is provided.
    pub(super) fn infer_return_type(&self, body: &Expr) -> Option<Type> {
        match body {
            Expr::Block(stmts) => stmts.last().and_then(|s| match s {
                Stmt::Expr { expr: e, .. } => Some(self.infer_expr_type(e)),
                _ => None,
            }),
            _ => Some(self.infer_expr_type(body)),
        }
    }

    pub(super) fn infer_expr_type(&self, expr: &Expr) -> Type {
        match expr {
            Expr::Literal(Literal::String(_)) | Expr::StringInterpolate(_) => {
                Type::Named("String".into())
            }
            Expr::Literal(Literal::Int(_)) => Type::Named("Int".into()),
            Expr::Literal(Literal::Float(_)) => Type::Named("Float".into()),
            Expr::Literal(Literal::Bool(_)) => Type::Named("Bool".into()),
            Expr::Literal(Literal::Char(_)) => Type::Named("Char".into()),
            Expr::Binary(left, op, _) => {
                if *op == BinaryOp::Add {
                    // If either side is a string, result is string
                    if matches!(self.infer_expr_type(left), Type::Named(ref n) if n == "String") {
                        return Type::Named("String".into());
                    }
                }
                Type::Named("Int".into())
            }
            Expr::Call { func, .. } => {
                if let Expr::Ident(name) = func.as_ref() {
                    match name.as_str() {
                        "print" | "println" | "action_json_free" => Type::Unit,
                        "toString" | "toUpper" | "toLower" => Type::Named("String".into()),
                        "substring" | "unwrapOr" | "readLine" | "jsonEscape" | "httpRequest"
                        | "str" | "chatOnce" | "storeMessages" | "extractContent"
                        | "handleChat" => Type::Named("String".into()),
                        "parseDate" | "date" => {
                            Type::Nullable(Box::new(Type::Named("Date".into())))
                        }
                        "datetime" => Type::Nullable(Box::new(Type::Named("DateTime".into()))),
                        "format" => Type::Named("String".into()),
                        "now" => Type::Named("DateTime".into()),
                        "today" => Type::Named("Date".into()),
                        "find" => Type::Nullable(Box::new(Type::Named("Int".into()))),
                        "flip" | "constant" | "identity" => Type::Named("Int".into()),
                        "Random_new" => Type::Named("Random".into()),
                        "nextInt" => Type::Generic(
                            Box::new(Type::Named("Tuple".into())),
                            vec![Type::Named("Random".into()), Type::Named("Int".into())],
                        ),
                        "count" => Type::Named("Int".into()),
                        "partition" => Type::Generic(
                            Box::new(Type::Named("Tuple".into())),
                            vec![Type::Named("list".into()), Type::Named("list".into())],
                        ),
                        "__list" => Type::Named("list".into()),
                        "__set" => Type::Named("set".into()),
                        _ => {
                            if self.registry.lookup_variant(name).is_some() {
                                let enum_name = self
                                    .registry
                                    .variant_to_enum
                                    .get(name)
                                    .cloned()
                                    .unwrap_or_default();
                                Type::Named(enum_name)
                            } else {
                                Type::Named("Int".into())
                            }
                        }
                    }
                } else {
                    Type::Named("Int".into())
                }
            }
            Expr::When(w) => self.infer_when_type(&w.kind),
            Expr::Continue | Expr::Break => Type::Unit,
            Expr::For(_) => Type::Unit,
            Expr::Block(stmts) => stmts
                .last()
                .map(|s| match s {
                    Stmt::Expr { expr: e, .. } => self.infer_expr_type(e),
                    _ => Type::Unit,
                })
                .unwrap_or(Type::Unit),
            Expr::Ident(name) => {
                // Check scope first for AST type info
                if let Some(sv) = self.scope.get(name) {
                    if let Some(ref ast_type) = sv.ast_type {
                        return ast_type.clone();
                    }
                    // Fallback: use val_kind to infer basic type
                    match sv.kind {
                        ValKind::Enum => {
                            // Try to find which enum type
                            for enum_name in self.enum_types.keys() {
                                if sv.ty == (*self.enum_types.get(enum_name).unwrap()).into() {
                                    return Type::Named(enum_name.clone());
                                }
                            }
                            Type::Named("Int".into())
                        }
                        ValKind::Str => Type::Named("String".into()),
                        ValKind::Struct => Type::Named("Int".into()), // ambiguous, default
                        ValKind::List => Type::Named("list".into()),
                        ValKind::Map => Type::Named("map".into()),
                        ValKind::Set => Type::Named("set".into()),
                        ValKind::Fn => Type::Named("Int".into()),
                        _ => Type::Named("Int".into()),
                    }
                } else if self.registry.lookup_variant(name).is_some() {
                    let enum_name = self
                        .registry
                        .variant_to_enum
                        .get(name)
                        .cloned()
                        .unwrap_or_default();
                    Type::Named(enum_name)
                } else {
                    Type::Named("Int".into())
                }
            }
            Expr::MapLiteral(_) => Type::Map(
                Box::new(Type::Named("String".into())),
                Box::new(Type::Named("Int".into())),
            ),
            Expr::SetLiteral(_) => Type::Set(Box::new(Type::Named("Int".into()))),
            Expr::Null => Type::Nullable(Box::new(Type::Named("Nothing".into()))),
            Expr::OrBlock { nullable, fallback } => {
                let cond_ty = self.infer_expr_type(nullable);
                match cond_ty {
                    Type::Nullable(inner) => *inner,
                    _ => self.infer_expr_type(fallback),
                }
            }
            _ => Type::Named("Int".into()),
        }
    }

    fn infer_when_type(&self, kind: &WhenKind) -> Type {
        match kind {
            WhenKind::OneLine {
                then_expr,
                else_expr,
                ..
            } => {
                let t = self.infer_expr_type(then_expr);
                if matches!(t, Type::Unit) {
                    self.infer_expr_type(else_expr)
                } else {
                    t
                }
            }
            WhenKind::ValueMatch { arms, .. } | WhenKind::ConditionChain { arms } => arms
                .first()
                .map(|a| self.infer_expr_type(&a.body))
                .unwrap_or(Type::Unit),
        }
    }

    pub(super) fn build_fn_type(
        &mut self,
        ret_ast: Option<&Type>,
        name: &str,
        param_tys: &[BasicMetadataTypeEnum<'ctx>],
    ) -> FunctionType<'ctx> {
        match ret_ast {
            Some(Type::Unit) => self.void_ty().fn_type(param_tys, false),
            Some(Type::Named(n)) => match n.as_str() {
                "Float" | "Double" => self.f64_ty().fn_type(param_tys, false),
                "Bool" => self.bool_ty().fn_type(param_tys, false),
                "String" | "Str" => self.string_type.fn_type(param_tys, false),
                "Unit" => self.void_ty().fn_type(param_tys, false),
                "Int" => self.i64_ty().fn_type(param_tys, false),
                "list" | "set" | "map" => self.list_type.fn_type(param_tys, false),
                "LazyList" => self.lazylist_type.fn_type(param_tys, false),
                name => {
                    if let Some(st) = self.named_structs.get(name) {
                        (*st).fn_type(param_tys, false)
                    } else if let Some(et) = self.enum_types.get(name) {
                        (*et).fn_type(param_tys, false)
                    } else {
                        self.i64_ty().fn_type(param_tys, false)
                    }
                }
            },
            Some(Type::Function(_, _)) => self.ptr_ty().fn_type(param_tys, false),
            None => {
                if name == "main" {
                    self.void_ty().fn_type(param_tys, false)
                } else {
                    // Use named fat-return type to distinguish from enum types
                    self.fat_return_type.fn_type(param_tys, false)
                }
            }
            Some(Type::Struct(fields)) => {
                let field_tys: Vec<BasicTypeEnum> = fields
                    .iter()
                    .map(|(_, ty)| self.ast_type_to_basic_type(ty))
                    .collect();
                let st = self.context.struct_type(&field_tys, false);
                st.fn_type(param_tys, false)
            }
            Some(Type::Map(_, _)) | Some(Type::Set(_)) => {
                // Map and Set use the {ptr, i64, i64} list layout
                let fat_ty = self.list_type;
                fat_ty.fn_type(param_tys, false)
            }
            Some(Type::Ptr(_)) | Some(Type::CString) | Some(Type::FileHandle) => {
                self.ptr_ty().fn_type(param_tys, false)
            }
            Some(Type::Nullable(inner)) => {
                let bt = self.ast_type_to_basic_type(inner);
                let nullable_st = self.get_nullable_type(bt, &format!("Nullable<{}>", inner));
                nullable_st.fn_type(param_tys, false)
            }
            Some(Type::Generic(base, _)) => match base.as_ref() {
                Type::Named(n) if n == "Ptr" => self.ptr_ty().fn_type(param_tys, false),
                _ => self.string_type.fn_type(param_tys, false),
            },
            _ => self.string_type.fn_type(param_tys, false),
        }
    }

    /// Mangle a function name by appending param types: add(Int,Float) → add_Int_Float
    pub(super) fn mangle_name(name: &str, param_types: &[Type]) -> String {
        if param_types.is_empty() {
            return name.to_string();
        }
        let parts: Vec<String> = param_types.iter().map(|t| format!("{}", t)).collect();
        format!("{}_{}", name, parts.join("_"))
    }

    /// Map a TypedValue to a type name string for overload resolution.
    pub(super) fn typed_value_type_name(&self, v: &TypedValue<'ctx>) -> String {
        match v {
            TypedValue::Int(_) => "Int".to_string(),
            TypedValue::Float(_) => "Float".to_string(),
            TypedValue::Bool(_) => "Bool".to_string(),
            TypedValue::Str(_) => "String".to_string(),
            TypedValue::Fn(_, _) | TypedValue::Closure { .. } => "Fn".to_string(),
            TypedValue::List(_) => "list".to_string(),
            TypedValue::Struct(_, st) => {
                // Try to find the named struct type
                for (name, ty) in &self.named_structs {
                    if *ty == *st {
                        return name.clone();
                    }
                }
                "Struct".to_string()
            }
            TypedValue::Enum(..) => {
                // Enum types are anonymous {i64, ptr} — for overload resolution
                // we use the registry to find the enum name
                "Enum".to_string()
            }
            TypedValue::Map(_) => "map".to_string(),
            TypedValue::Set(_) => "set".to_string(),
            TypedValue::Task(_) => "Task".to_string(),
            TypedValue::Stream(_) => "Stream".to_string(),
            TypedValue::LazyList(_) => "LazyList".to_string(),
            TypedValue::CString(_) => "CString".to_string(),
            TypedValue::Ptr(_) => "Ptr".to_string(),
            TypedValue::FileHandle(_) => "FileHandle".to_string(),
            TypedValue::Nullable(_, _) => "Nullable".to_string(),
            TypedValue::Unit => "Unit".to_string(),
        }
    }
    // ---- TypedValue helpers ----
}
impl<'ctx> TypedValue<'ctx> {
    pub(super) fn get_type_for_alloca(
        &self,
        cg: &CodeGen<'ctx>,
    ) -> inkwell::types::BasicTypeEnum<'ctx> {
        match self {
            TypedValue::CString(_) | TypedValue::Ptr(_) | TypedValue::FileHandle(_) => {
                cg.ptr_ty().into()
            }
            TypedValue::Struct(_, ty) => (*ty).into(),
            TypedValue::Enum(_, ty, ..) => (*ty).into(),
            TypedValue::Nullable(_, ty) => *ty,
            TypedValue::Unit => cg.i64_ty().into(),
            TypedValue::Int(_) => cg.i64_ty().into(),
            TypedValue::Float(_) => cg.f64_ty().into(),
            TypedValue::Bool(_) => cg.bool_ty().into(),
            TypedValue::Str(_)
            | TypedValue::Fn(_, _)
            | TypedValue::Closure { .. }
            | TypedValue::List(_)
            | TypedValue::Map(_)
            | TypedValue::Set(_)
            | TypedValue::Task(_)
            | TypedValue::Stream(_)
            | TypedValue::LazyList(_) => cg.ptr_ty().into(),
        }
    }

    /// The actual LLVM type of the value (not the alloca pointer type).
    /// For strings this returns the {i64, ptr} struct, not ptr.
    pub(super) fn get_value_type(&self, cg: &CodeGen<'ctx>) -> inkwell::types::BasicTypeEnum<'ctx> {
        match self {
            TypedValue::Str(_) => cg.string_type.into(),
            TypedValue::List(_) | TypedValue::Map(_) | TypedValue::Set(_) => cg.list_type.into(),
            TypedValue::Stream(_) => cg.stream_type.into(),
            TypedValue::Task(_) => cg.task_type.into(),
            TypedValue::LazyList(_) => cg.lazylist_type.into(),
            TypedValue::Struct(_, ty) => (*ty).into(),
            TypedValue::Enum(_, ty, ..) => (*ty).into(),
            TypedValue::Nullable(_, ty) => *ty,
            TypedValue::Bool(_) => cg.i64_ty().into(),
            TypedValue::Int(_) => cg.i64_ty().into(),
            TypedValue::Float(_) => cg.f64_ty().into(),
            TypedValue::CString(_) | TypedValue::Ptr(_) | TypedValue::FileHandle(_) => {
                cg.ptr_ty().into()
            }
            TypedValue::Unit | TypedValue::Fn(_, _) | TypedValue::Closure { .. } => {
                cg.ptr_ty().into()
            }
        }
    }

    pub(super) fn val_kind(&self) -> ValKind {
        match self {
            TypedValue::Int(_) => ValKind::Int,
            TypedValue::Float(_) => ValKind::Float,
            TypedValue::Bool(_) => ValKind::Bool,
            TypedValue::Str(_) => ValKind::Str,
            TypedValue::Fn(_, _) | TypedValue::Closure { .. } => ValKind::Fn,
            TypedValue::List(_) => ValKind::List,
            TypedValue::Map(_) => ValKind::Map,
            TypedValue::Set(_) => ValKind::Set,
            TypedValue::Task(_) => ValKind::Task,
            TypedValue::Stream(_) => ValKind::Stream,
            TypedValue::LazyList(_) => ValKind::LazyList,
            TypedValue::CString(_) => ValKind::CString,
            TypedValue::Ptr(_) => ValKind::Ptr,
            TypedValue::FileHandle(_) => ValKind::FileHandle,
            TypedValue::Struct(_, _) => ValKind::Struct,
            TypedValue::Enum(..) => ValKind::Enum,
            TypedValue::Nullable(_, _) => ValKind::Nullable,
            TypedValue::Unit => ValKind::Unit,
        }
    }
}
