//! Expression codegen (R4-3).

use action_frontend::ast::*;
use inkwell::builder::BuilderError;
use inkwell::values::IntValue;
use inkwell::{FloatPredicate, IntPredicate};

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn compile_and_hir(
        &mut self,
        lhs: &action_frontend::hir::HirExpr,
        rhs: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let left = self.compile_hir_expr(lhs)?;
        let left_bool = match left {
            TypedValue::Bool(b) => b,
            _ => return Err("&& requires boolean operands".to_string()),
        };

        let entry_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| "No insert block")?;
        let current_fn = entry_block
            .get_parent()
            .expect("CodeGen must have a parent function in the current block");
        let rhs_block = self.context.append_basic_block(current_fn, "and_rhs");
        let merge_block = self.context.append_basic_block(current_fn, "and_merge");
        let b1 = self.bool_ty();
        let false_val = b1.const_int(0, false);

        self.builder
            .build_conditional_branch(left_bool, rhs_block, merge_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rhs_block);
        let right = self.compile_hir_expr(rhs)?;
        let right_bool = match right {
            TypedValue::Bool(b) => b,
            _ => return Err("&& requires boolean operands".to_string()),
        };
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(merge_block);
        let phi = self.builder.build_phi(b1, "and_res").map_err(llvm_err)?;
        phi.add_incoming(&[
            (&false_val as &dyn inkwell::values::BasicValue, entry_block),
            (&right_bool, rhs_block),
        ]);

        Ok(TypedValue::Bool(phi.as_basic_value().into_int_value()))
    }

    /// Short-circuit OR on HIR expressions.
    pub(crate) fn compile_or_hir(
        &mut self,
        lhs: &action_frontend::hir::HirExpr,
        rhs: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let left = self.compile_hir_expr(lhs)?;
        let left_bool = match left {
            TypedValue::Bool(b) => b,
            _ => return Err("|| requires boolean operands".to_string()),
        };

        let entry_block = self
            .builder
            .get_insert_block()
            .ok_or_else(|| "No insert block")?;
        let current_fn = entry_block
            .get_parent()
            .expect("CodeGen must have a parent function in the current block");
        let rhs_block = self.context.append_basic_block(current_fn, "or_rhs");
        let merge_block = self.context.append_basic_block(current_fn, "or_merge");
        let b1 = self.bool_ty();
        let true_val = b1.const_int(1, false);

        self.builder
            .build_conditional_branch(left_bool, merge_block, rhs_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rhs_block);
        let right = self.compile_hir_expr(rhs)?;
        let right_bool = match right {
            TypedValue::Bool(b) => b,
            _ => return Err("|| requires boolean operands".to_string()),
        };
        self.builder
            .build_unconditional_branch(merge_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(merge_block);
        let phi = self.builder.build_phi(b1, "or_res").map_err(llvm_err)?;
        phi.add_incoming(&[
            (&true_val as &dyn inkwell::values::BasicValue, entry_block),
            (&right_bool, rhs_block),
        ]);

        Ok(TypedValue::Bool(phi.as_basic_value().into_int_value()))
    }

    pub(crate) fn bin_add(
        &mut self,
        l: &TypedValue<'ctx>,
        r: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match (l, r) {
            (TypedValue::Int(a), TypedValue::Int(b)) => Ok(TypedValue::Int(
                self.builder
                    .build_int_add(*a, *b, "add")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Float(a), TypedValue::Float(b)) => Ok(TypedValue::Float(
                self.builder
                    .build_float_add(*a, *b, "add")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Str(a), TypedValue::Str(b)) => {
                let cc = self.call_rt_with_2str("action_string_concat", *a, *b)?;
                // Free intermediate operands (not scope variables) after concat
                // takes ownership of the data.
                self.rc_free_intermediate(l)?;
                self.rc_free_intermediate(r)?;
                match cc.try_as_basic_value().basic() {
                    Some(bv) => {
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "concat")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, bv).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    None => Err("String concat failed".to_string()),
                }
            }
            // Int + Float / Float + Int → promote to Float
            (TypedValue::Float(_), _) | (_, TypedValue::Float(_)) => {
                let fa = self.promote_to_float(l)?;
                let fb = self.promote_to_float(r)?;
                Ok(TypedValue::Float(
                    self.builder
                        .build_float_add(fa, fb, "add")
                        .map_err(llvm_err)?,
                ))
            }
            _ => Err("Cannot add these types".to_string()),
        }
    }

    /// Promote Int to Float for mixed-type arithmetic
    pub(crate) fn promote_to_float(
        &self,
        v: &TypedValue<'ctx>,
    ) -> Result<inkwell::values::FloatValue<'ctx>, String> {
        match v {
            TypedValue::Int(i) => Ok(self
                .builder
                .build_signed_int_to_float(*i, self.f64_ty(), "promote")
                .map_err(llvm_err)?),
            TypedValue::Float(f) => Ok(*f),
            _ => Err("Cannot promote to Float".to_string()),
        }
    }

    pub(crate) fn bin_arith(
        &mut self,
        l: &TypedValue<'ctx>,
        r: &TypedValue<'ctx>,
        _n: &str,
        int_op: fn(
            &inkwell::builder::Builder<'ctx>,
            IntValue<'ctx>,
            IntValue<'ctx>,
        ) -> Result<IntValue<'ctx>, BuilderError>,
        float_op: fn(
            &inkwell::builder::Builder<'ctx>,
            inkwell::values::FloatValue<'ctx>,
            inkwell::values::FloatValue<'ctx>,
        ) -> Result<inkwell::values::FloatValue<'ctx>, BuilderError>,
    ) -> Result<TypedValue<'ctx>, String> {
        match (l, r) {
            (TypedValue::Int(a), TypedValue::Int(b)) => Ok(TypedValue::Int(
                int_op(&self.builder, *a, *b).map_err(llvm_err)?,
            )),
            (TypedValue::Float(a), TypedValue::Float(b)) => Ok(TypedValue::Float(
                float_op(&self.builder, *a, *b).map_err(llvm_err)?,
            )),
            // Int + Float → promote Int to Float
            (TypedValue::Float(_), _) | (_, TypedValue::Float(_)) => {
                let fa = self.promote_to_float(l)?;
                let fb = self.promote_to_float(r)?;
                Ok(TypedValue::Float(
                    float_op(&self.builder, fa, fb).map_err(llvm_err)?,
                ))
            }
            _ => Err("Cannot perform arithmetic on these types".to_string()),
        }
    }

    pub(crate) fn bin_div(
        &mut self,
        l: &TypedValue<'ctx>,
        r: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match (l, r) {
            (TypedValue::Int(a), TypedValue::Int(b)) => Ok(TypedValue::Int(
                self.builder
                    .build_int_signed_div(*a, *b, "div")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Float(a), TypedValue::Float(b)) => Ok(TypedValue::Float(
                self.builder
                    .build_float_div(*a, *b, "div")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Float(_), _) | (_, TypedValue::Float(_)) => {
                let fa = self.promote_to_float(l)?;
                let fb = self.promote_to_float(r)?;
                Ok(TypedValue::Float(
                    self.builder
                        .build_float_div(fa, fb, "div")
                        .map_err(llvm_err)?,
                ))
            }
            _ => Err("Cannot perform division on these types".to_string()),
        }
    }

    pub(crate) fn bin_mod(
        &mut self,
        l: &TypedValue<'ctx>,
        r: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match (l, r) {
            (TypedValue::Int(a), TypedValue::Int(b)) => Ok(TypedValue::Int(
                self.builder
                    .build_int_signed_rem(*a, *b, "mod")
                    .map_err(llvm_err)?,
            )),
            _ => Err("Modulo requires integer operands".to_string()),
        }
    }

    pub(crate) fn bin_pow(
        &mut self,
        l: &TypedValue<'ctx>,
        r: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match (l, r) {
            (TypedValue::Int(a), TypedValue::Int(b)) => {
                let pow_fn = self.module.get_function("action_int_pow").unwrap();
                let result = self
                    .builder
                    .build_call(pow_fn, &[(*a).into(), (*b).into()], "pow")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();
                Ok(TypedValue::Int(result))
            }
            (TypedValue::Float(a), TypedValue::Float(b)) => {
                let pow_fn = self.module.get_function("pow").unwrap();
                let result = self
                    .builder
                    .build_call(pow_fn, &[(*a).into(), (*b).into()], "pow")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_float_value();
                Ok(TypedValue::Float(result))
            }
            // Mixed Int/Float → promote to Float
            (TypedValue::Float(_), _) | (_, TypedValue::Float(_)) => {
                let fa = self.promote_to_float(l)?;
                let fb = self.promote_to_float(r)?;
                let pow_fn = self.module.get_function("pow").unwrap();
                let result = self
                    .builder
                    .build_call(pow_fn, &[fa.into(), fb.into()], "pow")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_float_value();
                Ok(TypedValue::Float(result))
            }
            _ => Err("** requires numeric operands".to_string()),
        }
    }

    pub(crate) fn bin_bitwise(
        &mut self,
        l: &TypedValue<'ctx>,
        r: &TypedValue<'ctx>,
        _n: &str,
        op: fn(
            &inkwell::builder::Builder<'ctx>,
            IntValue<'ctx>,
            IntValue<'ctx>,
        ) -> Result<IntValue<'ctx>, BuilderError>,
    ) -> Result<TypedValue<'ctx>, String> {
        match (l, r) {
            (TypedValue::Int(a), TypedValue::Int(b)) => Ok(TypedValue::Int(
                op(&self.builder, *a, *b).map_err(llvm_err)?,
            )),
            _ => Err("Bitwise operations require integer operands".to_string()),
        }
    }

    pub(crate) fn compare_eq(
        &mut self,
        l: &TypedValue<'ctx>,
        r: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match (l, r) {
            (TypedValue::Int(a), TypedValue::Int(b)) => Ok(TypedValue::Bool(
                self.builder
                    .build_int_compare(IntPredicate::EQ, *a, *b, "eq")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Bool(a), TypedValue::Bool(b)) => Ok(TypedValue::Bool(
                self.builder
                    .build_int_compare(IntPredicate::EQ, *a, *b, "eq")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Float(a), TypedValue::Float(b)) => Ok(TypedValue::Bool(
                self.builder
                    .build_float_compare(FloatPredicate::OEQ, *a, *b, "eq")
                    .map_err(llvm_err)?,
            )),
            // Int + Float comparison → promote Int to Float
            (TypedValue::Float(_), _) | (_, TypedValue::Float(_)) => {
                let fa = self.promote_to_float(l)?;
                let fb = self.promote_to_float(r)?;
                Ok(TypedValue::Bool(
                    self.builder
                        .build_float_compare(FloatPredicate::OEQ, fa, fb, "eq")
                        .map_err(llvm_err)?,
                ))
            }
            (TypedValue::Str(a), TypedValue::Str(b)) => {
                let sa = self.load_string(*a)?;
                let sb = self.load_string(*b)?;
                let cc = self.call_rt("action_string_eq", &[sa.into(), sb.into()])?;
                Ok(TypedValue::Bool(
                    cc.try_as_basic_value()
                        .basic()
                        .ok_or("streq failed")?
                        .into_int_value(),
                ))
            }
            (TypedValue::Nullable(l_ptr, l_ty), TypedValue::Nullable(r_ptr, _)) => {
                return self.compare_nullable_eq(*l_ptr, *r_ptr, *l_ty);
            }
            _ => Err("Cannot compare these types".to_string()),
        }
    }

    pub(crate) fn compare_neq(
        &mut self,
        l: &TypedValue<'ctx>,
        r: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match (l, r) {
            (TypedValue::Int(a), TypedValue::Int(b)) => Ok(TypedValue::Bool(
                self.builder
                    .build_int_compare(IntPredicate::NE, *a, *b, "neq")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Bool(a), TypedValue::Bool(b)) => Ok(TypedValue::Bool(
                self.builder
                    .build_int_compare(IntPredicate::NE, *a, *b, "neq")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Float(a), TypedValue::Float(b)) => Ok(TypedValue::Bool(
                self.builder
                    .build_float_compare(FloatPredicate::ONE, *a, *b, "neq")
                    .map_err(llvm_err)?,
            )),
            // Int + Float comparison → promote Int to Float
            (TypedValue::Float(_), _) | (_, TypedValue::Float(_)) => {
                let fa = self.promote_to_float(l)?;
                let fb = self.promote_to_float(r)?;
                Ok(TypedValue::Bool(
                    self.builder
                        .build_float_compare(FloatPredicate::ONE, fa, fb, "neq")
                        .map_err(llvm_err)?,
                ))
            }
            (TypedValue::Str(a), TypedValue::Str(b)) => {
                let sa = self.load_string(*a)?;
                let sb = self.load_string(*b)?;
                let cc = self.call_rt("action_string_eq", &[sa.into(), sb.into()])?;
                let eq = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("strneq failed")?
                    .into_int_value();
                let one = self.bool_ty().const_int(1, false);
                Ok(TypedValue::Bool(
                    self.builder
                        .build_xor(eq, one, "strneq")
                        .map_err(llvm_err)?,
                ))
            }
            (TypedValue::Nullable(l_ptr, l_ty), TypedValue::Nullable(r_ptr, _)) => {
                return self.compare_nullable_neq(*l_ptr, *r_ptr, *l_ty);
            }
            _ => Err("Cannot compare these types".to_string()),
        }
    }

    pub(crate) fn compare(
        &mut self,
        ip: IntPredicate,
        fp: FloatPredicate,
        l: &TypedValue<'ctx>,
        r: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        match (l, r) {
            (TypedValue::Int(a), TypedValue::Int(b)) => Ok(TypedValue::Bool(
                self.builder
                    .build_int_compare(ip, *a, *b, "cmp")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Bool(a), TypedValue::Bool(b)) => Ok(TypedValue::Bool(
                self.builder
                    .build_int_compare(ip, *a, *b, "cmp")
                    .map_err(llvm_err)?,
            )),
            (TypedValue::Float(a), TypedValue::Float(b)) => Ok(TypedValue::Bool(
                self.builder
                    .build_float_compare(fp, *a, *b, "cmp")
                    .map_err(llvm_err)?,
            )),
            // Int + Float comparison → promote Int to Float
            (TypedValue::Float(_), _) | (_, TypedValue::Float(_)) => {
                let fa = self.promote_to_float(l)?;
                let fb = self.promote_to_float(r)?;
                Ok(TypedValue::Bool(
                    self.builder
                        .build_float_compare(fp, fa, fb, "cmp")
                        .map_err(llvm_err)?,
                ))
            }
            (TypedValue::Str(a), TypedValue::Str(b)) => {
                let sa = self.load_string(*a)?;
                let sb = self.load_string(*b)?;
                let cc = self.call_rt("action_string_compare", &[sa.into(), sb.into()])?;
                let cmp = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("strcmp failed")?
                    .into_int_value();
                Ok(TypedValue::Bool(
                    self.builder
                        .build_int_compare(ip, cmp, self.i64_ty().const_int(0, false), "strcmp")
                        .map_err(llvm_err)?,
                ))
            }
            _ => Err("Cannot compare these types".to_string()),
        }
    }

    /// `in` operator: value in range, value in list, value in set, key in map

    /// `is` operator: expr is Type — runtime type check

    /// `is` operator on HIR expressions.
    pub(crate) fn bin_is_hir(
        &mut self,
        lhs: &action_frontend::hir::HirExpr,
        rhs: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let type_name = match &rhs.kind {
            action_frontend::hir::HirExprKind::Ident(name) => name.clone(),
            _ => return Err("'is' operator requires a type name on the right".into()),
        };

        if let Some((enum_info, variant_info)) = self.registry.lookup_variant(&type_name) {
            let variant_idx = enum_info
                .variants
                .iter()
                .position(|v| v.name == variant_info.name)
                .unwrap_or(0) as u64;

            let val = self.compile_hir_expr(lhs)?;
            match val {
                TypedValue::Enum(ptr, enum_ty, ..) => {
                    let loaded = self
                        .builder
                        .build_load(enum_ty, ptr, "is_ld")
                        .map_err(llvm_err)?;
                    let tag = self
                        .builder
                        .build_extract_value(loaded.into_struct_value(), 0, "tag")
                        .map_err(llvm_err)?;
                    let cmp = self
                        .builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            tag.into_int_value(),
                            self.i64_ty().const_int(variant_idx, false),
                            "is_match",
                        )
                        .map_err(llvm_err)?;
                    Ok(TypedValue::Bool(cmp))
                }
                _ => Ok(TypedValue::Bool(self.bool_ty().const_int(0, false))),
            }
        } else {
            let _ = self.compile_hir_expr(lhs)?;
            Ok(TypedValue::Bool(self.bool_ty().const_int(1, false)))
        }
    }

    /// `in` operator on HIR expressions.
    pub(crate) fn bin_in_hir(
        &mut self,
        lhs: &action_frontend::hir::HirExpr,
        rhs: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::HirExprKind;
        let value = self.compile_hir_expr(lhs)?;
        match &rhs.kind {
            HirExprKind::Range(start, end) => {
                let start_v = self.compile_hir_expr(start)?;
                let end_v = self.compile_hir_expr(end)?;
                let (start_int, end_int, val_int) = match (start_v, end_v, value) {
                    (TypedValue::Int(s), TypedValue::Int(e), TypedValue::Int(v)) => (s, e, v),
                    _ => return Err("Range bounds and value must be integers".into()),
                };
                let ge_start = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, val_int, start_int, "in_ge")
                    .map_err(llvm_err)?;
                let lt_end = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, val_int, end_int, "in_lt")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(
                    self.builder
                        .build_and(ge_start, lt_end, "in_range")
                        .map_err(llvm_err)?,
                ))
            }
            HirExprKind::Binary(start, BinaryOp::RangeExclusive, end) => {
                let start_v = self.compile_hir_expr(start)?;
                let end_v = self.compile_hir_expr(end)?;
                let (start_int, end_int, val_int) = match (start_v, end_v, value) {
                    (TypedValue::Int(s), TypedValue::Int(e), TypedValue::Int(v)) => (s, e, v),
                    _ => return Err("Range bounds and value must be integers".into()),
                };
                let ge_start = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, val_int, start_int, "in_ge")
                    .map_err(llvm_err)?;
                let lt_end = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, val_int, end_int, "in_lt")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(
                    self.builder
                        .build_and(ge_start, lt_end, "in_range_excl")
                        .map_err(llvm_err)?,
                ))
            }
            _ => {
                let collection = self.compile_hir_expr(rhs)?;
                match collection {
                    TypedValue::List(ptr) | TypedValue::Set(ptr) | TypedValue::LazyList(ptr) => {
                        let elem_fat = self.to_fat_struct(&value)?;
                        let list_val = self.load_list(ptr)?;
                        let cc = self
                            .call_rt("action_list_contains", &[list_val.into(), elem_fat.into()])?;
                        Ok(TypedValue::Bool(
                            cc.try_as_basic_value()
                                .basic()
                                .ok_or("list_contains failed")?
                                .into_int_value(),
                        ))
                    }
                    TypedValue::Stream(ptr) => {
                        let elem_fat = self.to_fat_struct(&value)?;
                        let list_field = self
                            .builder
                            .build_struct_gep(self.stream_type, ptr, 1, "in_strm_lf")
                            .map_err(llvm_err)?;
                        let list_val = self
                            .builder
                            .build_load(self.list_type, list_field, "in_strm_lv")
                            .map_err(llvm_err)?;
                        let cc = self
                            .call_rt("action_list_contains", &[list_val.into(), elem_fat.into()])?;
                        Ok(TypedValue::Bool(
                            cc.try_as_basic_value()
                                .basic()
                                .ok_or("list_contains failed")?
                                .into_int_value(),
                        ))
                    }
                    TypedValue::Map(ptr) => {
                        let key_fat = self.to_fat_struct(&value)?;
                        let map_val = self.load_list(ptr)?;
                        let cc =
                            self.call_rt("action_map_contains", &[map_val.into(), key_fat.into()])?;
                        Ok(TypedValue::Bool(
                            cc.try_as_basic_value()
                                .basic()
                                .ok_or("map_contains failed")?
                                .into_int_value(),
                        ))
                    }
                    _ => Err("'in' operator requires a range or collection on the right".into()),
                }
            }
        }
    }
}
