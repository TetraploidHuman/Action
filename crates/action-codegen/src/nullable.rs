// Submodule: nullable — nullable value operations
//
// Extracted from misc.rs.
//

use action_frontend::ast::*;
use inkwell::types::BasicType;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, PointerValue};
use inkwell::IntPredicate;

use super::call_arg::CallArg;
use super::{llvm_err, CodeGen, TypedValue, ValKind};

impl<'ctx> CodeGen<'ctx> {
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
    pub(super) fn compile_nullable_method_call_call_args(
        &mut self,
        nullable_ptr: PointerValue<'ctx>,
        inner_bt: BasicTypeEnum<'ctx>,
        receiver: CallArg<'_>,
        method: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        self.compile_nullable_method_call_call_args_inner(
            nullable_ptr,
            inner_bt,
            receiver,
            method,
            args,
            trailing,
        )
    }

    fn compile_nullable_method_call_call_args_inner(
        &mut self,
        nullable_ptr: PointerValue<'ctx>,
        inner_bt: BasicTypeEnum<'ctx>,
        receiver: CallArg<'_>,
        method: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
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
        if let Some(recv_name) = Self::call_arg_ident_name(receiver) {
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
        let counter = self.nullable_state.synthetic_counter;
        self.nullable_state.synthetic_counter += 1;
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
        let syn_ident = super::call_arg::synthetic_hir_ident(synthetic_name.clone());
        let syn_recv = CallArg::hir(&syn_ident);
        let method_result = match self.compile_ufcs_method(syn_recv, method, args, trailing) {
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
        let nty = self.get_nullable_type(
            result_bt,
            &format!("__nmc_res_{}", self.nullable_state.synthetic_counter),
        );
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

    pub(super) fn compile_or_block_hir(
        &mut self,
        nullable: &action_frontend::hir::HirExpr,
        fallback: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let cond_val = self.compile_hir_expr(nullable)?;
        self.compile_or_block_from_cond(cond_val, |cg| cg.compile_hir_expr(fallback))
    }

    fn compile_or_block_from_cond<F>(
        &mut self,
        cond_val: TypedValue<'ctx>,
        compile_fallback: F,
    ) -> Result<TypedValue<'ctx>, String>
    where
        F: FnOnce(&mut Self) -> Result<TypedValue<'ctx>, String>,
    {
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
        let default_val = compile_fallback(self)?;
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
    pub(super) fn compile_field_access_on_typed_value(
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
}
