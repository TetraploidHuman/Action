// Submodule: pattern

use action_frontend::ast::*;
use inkwell::types::BasicTypeEnum;
use inkwell::values::{IntValue, PointerValue};
use inkwell::FloatPredicate;
use inkwell::IntPredicate;
use std::collections::HashMap;

use super::{llvm_err, CodeGen, GepCursor, InnerType, Scope, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_when(&mut self, w: &When) -> Result<TypedValue<'ctx>, String> {
        match &w.kind {
            WhenKind::OneLine {
                condition,
                then_expr,
                else_expr,
            } => {
                let c = self.compile_expr(condition)?;
                let c_bool = match c {
                    TypedValue::Bool(b) => b,
                    _ => return Err("when condition must be boolean".to_string()),
                };
                // Smart cast: when x != null { ... } or when x == null { ... } else { ... }
                let smart_var: Option<String> = match &condition.kind {
                    ExprKind::Binary(lhs, BinaryOp::Neq, rhs)
                    | ExprKind::Binary(lhs, BinaryOp::Eq, rhs) => match (&lhs.kind, &rhs.kind) {
                        (ExprKind::Ident(name), ExprKind::Null)
                        | (ExprKind::Null, ExprKind::Ident(name)) => Some(name.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(ref var) = smart_var {
                    let is_eq = matches!(&condition.kind, ExprKind::Binary(_, BinaryOp::Eq, _));
                    if is_eq {
                        // when x == null { null_body } else { non_null_body }
                        // Negate condition and swap branches so smart cast applies to non_null_body
                        let negated = self
                            .builder
                            .build_not(c_bool, "neg_cond")
                            .map_err(llvm_err)?;
                        self.not_null_set.insert(var.clone());
                        let result = self.compile_when_branch_lazy(negated, else_expr, then_expr);
                        self.not_null_set.remove(var);
                        result
                    } else {
                        // when x != null { non_null_body } [else { null_body }]
                        self.not_null_set.insert(var.clone());
                        let result = self.compile_when_branch_lazy(c_bool, then_expr, else_expr);
                        self.not_null_set.remove(var);
                        result
                    }
                } else {
                    self.compile_when_branch_lazy(c_bool, then_expr, else_expr)
                }
            }
            WhenKind::ValueMatch { value, arms } => self.compile_value_match(value, arms),
            WhenKind::ConditionChain { arms } => self.compile_condition_chain(arms),
        }
    }

    /// Compile a guard expression and return the boolean result.
    fn compile_guard(&mut self, guard: &Option<Box<Expr>>) -> Result<IntValue<'ctx>, String> {
        match guard {
            Some(expr) => {
                let val = self.compile_expr(expr)?;
                match val {
                    TypedValue::Bool(b) => Ok(b),
                    TypedValue::Int(i) => {
                        let zero = self.i64_ty().const_int(0, false);
                        Ok(self
                            .builder
                            .build_int_compare(IntPredicate::NE, i, zero, "guard_truthy")
                            .map_err(llvm_err)?)
                    }
                    _ => {
                        let b1 = self.bool_ty();
                        Ok(b1.const_int(1, false))
                    }
                }
            }
            None => Ok(self.bool_ty().const_int(1, false)),
        }
    }

    pub(super) fn compile_condition_chain(
        &mut self,
        arms: &[WhenArm],
    ) -> Result<TypedValue<'ctx>, String> {
        if arms.is_empty() {
            return Ok(TypedValue::Unit);
        }

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile when outside function")?;

        let merge_block = self.context.append_basic_block(current_fn, "chain_merge");

        // Infer result type from first arm (avoid compiling body just for type)
        let arm_types: Vec<Type> = arms.iter().map(|a| self.infer_expr_type(&a.body)).collect();
        let result_type = arm_types
            .iter()
            .find(
                |t| matches!(t, Type::Named(n) if self.enum_types.contains_key(n) || n == "String"),
            )
            .or_else(|| arm_types.first())
            .cloned()
            .unwrap_or_else(|| Type::Named("Int".into()));
        let result_ty = self.ast_type_to_basic_type(&result_type);

        // Allocate result at entry
        let entry = current_fn.get_first_basic_block().unwrap();
        let saved_pos = self.builder.get_insert_block();
        match entry.get_first_instruction() {
            Some(instr) => {
                let _ = self.builder.position_before(&instr);
            }
            None => self.builder.position_at_end(entry),
        }
        let result_alloca = self
            .builder
            .build_alloca(result_ty, "chain_result")
            .map_err(llvm_err)?;
        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }

        let mut next_check = self.context.append_basic_block(current_fn, "chain_check0");
        let _ = self.builder.build_unconditional_branch(next_check);
        let mut chain_enum_info: Option<(InnerType, bool)> = None;

        for (i, arm) in arms.iter().enumerate() {
            let is_last = i == arms.len() - 1;
            self.builder.position_at_end(next_check);

            let matches = self.compile_pattern_condition(&arm.pattern, None)?;
            // Check guard if present
            let matches = if arm.guard.is_some() {
                let mut saved_scope = Scope::new();
                std::mem::swap(&mut self.scope, &mut saved_scope);
                self.scope = Scope::with_parent(saved_scope);
                self.bind_pattern_vars(&arm.pattern, None, None)?;
                let guard_matches = self.compile_guard(&arm.guard)?;
                let combined = self
                    .builder
                    .build_and(matches, guard_matches, "guard_and")
                    .map_err(llvm_err)?;
                self.emit_scope_cleanup()?;
                let mut parent = Scope::new();
                std::mem::swap(&mut self.scope, &mut parent);
                if let Some(p) = parent.parent {
                    self.scope = *p;
                }
                combined
            } else {
                matches
            };
            let body_block = self
                .context
                .append_basic_block(current_fn, &format!("chain_body{}", i));

            if is_last {
                let _ = self
                    .builder
                    .build_conditional_branch(matches, body_block, merge_block);
            } else {
                next_check = self
                    .context
                    .append_basic_block(current_fn, &format!("chain_check{}", i + 1));
                let _ = self
                    .builder
                    .build_conditional_branch(matches, body_block, next_check);
            }

            self.builder.position_at_end(body_block);
            // Create child scope for pattern bindings
            let mut saved_scope = Scope::new();
            std::mem::swap(&mut self.scope, &mut saved_scope);
            self.scope = Scope::with_parent(saved_scope);
            self.bind_pattern_vars(&arm.pattern, None, None)?;
            let body_val = self.compile_expr(&arm.body)?;
            if let TypedValue::Enum(_, _, inner, rc) = &body_val {
                chain_enum_info = Some((*inner, *rc));
            }
            // RC inc the result before cleaning up the child scope, so
            // heap-typed pattern variables aren't freed prematurely.
            if self.is_scope_variable(&body_val) {
                self.rc_inc_typed_value(&body_val)?;
            }
            self.emit_scope_cleanup()?;
            self.store_value_to_alloca(&body_val, result_alloca)?;
            // Restore scope
            let mut parent = Scope::new();
            std::mem::swap(&mut self.scope, &mut parent);
            if let Some(p) = parent.parent {
                self.scope = *p;
            }
            let _ = self.builder.build_unconditional_branch(merge_block);
        }

        self.builder.position_at_end(merge_block);
        self.last_enum_inner = chain_enum_info;
        let loaded = self
            .builder
            .build_load(result_ty, result_alloca, "chain_ld")
            .map_err(llvm_err)?;
        self.bv_to_typed(loaded)
    }

    pub(super) fn compile_value_match(
        &mut self,
        value: &Expr,
        arms: &[WhenArm],
    ) -> Result<TypedValue<'ctx>, String> {
        if arms.is_empty() {
            return Ok(TypedValue::Unit);
        }

        // Check exhaustiveness for enum matching
        self.registry.check_when_exhaustive(arms)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile when outside function")?;

        // Compile the matched value once
        let matched_val = self.compile_expr(value)?;
        // Infer the AST type of the matched value for resolving generic enum params
        let matched_type = self.infer_expr_type(value);

        // Infer result type: prefer enum type if any arm returns one, otherwise use Int
        let arm_types: Vec<Type> = arms.iter().map(|a| self.infer_expr_type(&a.body)).collect();
        let result_type = arm_types
            .iter()
            .find(
                |t| matches!(t, Type::Named(n) if self.enum_types.contains_key(n) || n == "String"),
            )
            .or_else(|| arm_types.first())
            .cloned()
            .unwrap_or_else(|| Type::Named("Int".into()));
        let result_ty = self.ast_type_to_basic_type(&result_type);

        // Allocate result at entry
        let entry = current_fn.get_first_basic_block().unwrap();
        let saved_pos = self.builder.get_insert_block();
        match entry.get_first_instruction() {
            Some(instr) => {
                let _ = self.builder.position_before(&instr);
            }
            None => self.builder.position_at_end(entry),
        }
        let result_alloca = self
            .builder
            .build_alloca(result_ty, "match_result")
            .map_err(llvm_err)?;
        // Zero-initialize to prevent garbage reads when an arm stores fewer bytes
        // than the full result type (e.g., storing i64 into {i64, ptr} for Option)
        let zero = result_ty.const_zero();
        self.builder
            .build_store(result_alloca, zero)
            .map_err(llvm_err)?;
        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }

        let merge_block = self.context.append_basic_block(current_fn, "match_merge");
        let mut next_check = self.context.append_basic_block(current_fn, "match_check0");
        let _ = self.builder.build_unconditional_branch(next_check);
        // Track enum inner type from arm bodies to preserve through bv_to_typed
        let mut result_enum_info: Option<(InnerType, bool)> = None;

        for (i, arm) in arms.iter().enumerate() {
            let is_last = i == arms.len() - 1;
            self.builder.position_at_end(next_check);

            let matches = self.compile_pattern_match(&arm.pattern, &matched_val)?;
            // Check guard if present
            let matches = if arm.guard.is_some() {
                let mut saved_scope = Scope::new();
                std::mem::swap(&mut self.scope, &mut saved_scope);
                self.scope = Scope::with_parent(saved_scope);
                self.bind_pattern_vars(&arm.pattern, Some(&matched_val), Some(&matched_type))?;
                let guard_matches = self.compile_guard(&arm.guard)?;
                let combined = self
                    .builder
                    .build_and(matches, guard_matches, "guard_and")
                    .map_err(llvm_err)?;
                let mut parent = Scope::new();
                std::mem::swap(&mut self.scope, &mut parent);
                if let Some(p) = parent.parent {
                    self.scope = *p;
                }
                combined
            } else {
                matches
            };
            let body_block = self
                .context
                .append_basic_block(current_fn, &format!("match_body{}", i));

            if is_last {
                let _ = self
                    .builder
                    .build_conditional_branch(matches, body_block, merge_block);
            } else {
                next_check = self
                    .context
                    .append_basic_block(current_fn, &format!("match_check{}", i + 1));
                let _ = self
                    .builder
                    .build_conditional_branch(matches, body_block, next_check);
            }

            self.builder.position_at_end(body_block);
            // Smart cast: if matched value is an Ident of nullable type and this arm's
            // pattern is non-null, inject the ident into not_null_set so the ident
            // is treated as non-nullable inside this arm's body.
            let smart_var: Option<String> = match (&value.kind, &arm.pattern) {
                (ExprKind::Ident(_), Pattern::Null) => None,
                (ExprKind::Ident(name), _) => Some(name.clone()),
                _ => None,
            };
            if let Some(ref var) = smart_var {
                self.not_null_set.insert(var.clone());
            }
            // Create child scope and bind pattern variables to the matched value
            let mut saved_scope = Scope::new();
            std::mem::swap(&mut self.scope, &mut saved_scope);
            self.scope = Scope::with_parent(saved_scope);
            self.bind_pattern_vars(&arm.pattern, Some(&matched_val), Some(&matched_type))?;
            // When arm body is a zero-param lambda (parser wraps { ... } blocks
            // after -> as lambdas), compile the inner body directly so pattern
            // bindings are visible in the current scope.
            let body_val = if let ExprKind::Lambda { params, body, .. } = &arm.body.kind {
                if params.is_empty() {
                    self.compile_expr(body)?
                } else {
                    self.compile_expr(&arm.body)?
                }
            } else {
                self.compile_expr(&arm.body)?
            };
            if let Some(ref var) = smart_var {
                self.not_null_set.remove(var);
            }
            if let TypedValue::Enum(_, _, inner, rc) = &body_val {
                result_enum_info = Some((*inner, *rc));
            }
            // RC inc the result before cleaning up the child scope, so
            // heap-typed pattern variables aren't freed prematurely.
            if self.is_scope_variable(&body_val) {
                self.rc_inc_typed_value(&body_val)?;
            }
            self.emit_scope_cleanup()?;
            self.store_value_to_alloca(&body_val, result_alloca)?;
            let mut parent = Scope::new();
            std::mem::swap(&mut self.scope, &mut parent);
            if let Some(p) = parent.parent {
                self.scope = *p;
            }
            let _ = self.builder.build_unconditional_branch(merge_block);
        }

        self.builder.position_at_end(merge_block);
        self.last_enum_inner = result_enum_info;
        let loaded = self
            .builder
            .build_load(result_ty, result_alloca, "match_ld")
            .map_err(llvm_err)?;
        self.bv_to_typed(loaded)
    }

    /// Compile a pattern as a boolean condition (for ConditionChain).
    /// For ConditionChain, patterns act as conditions: Literal/Ident/Variable are truthy,
    /// Wildcard is always true.
    pub(super) fn compile_pattern_condition(
        &mut self,
        pattern: &Pattern,
        _matched_val: Option<&TypedValue<'ctx>>,
    ) -> Result<IntValue<'ctx>, String> {
        let b1 = self.bool_ty();
        match pattern {
            Pattern::Wildcard => Ok(b1.const_int(1, false)),
            Pattern::Literal(lit) => {
                // A literal is truthy — always true in condition context
                // Actually, for condition chain: `when { 0 -> "zero" }` — the literal IS the condition.
                // Treat any non-zero/non-null value as true
                match lit {
                    Literal::Bool(b) => Ok(b1.const_int(if *b { 1 } else { 0 }, false)),
                    Literal::Int(n) => Ok(b1.const_int(if *n != 0 { 1 } else { 0 }, false)),
                    Literal::Float(f) => Ok(b1.const_int(if *f != 0.0 { 1 } else { 0 }, false)),
                    Literal::Char(c) => Ok(b1.const_int(if *c != '\0' { 1 } else { 0 }, false)),
                    Literal::Unit => Ok(b1.const_int(0, false)),
                    _ => Ok(b1.const_int(1, false)),
                }
            }
            Pattern::Variable(_) => Ok(b1.const_int(1, false)), // Variable binding always matches
            Pattern::Range(_start, _end) => {
                // Range in condition context: treated as true (shouldn't normally appear here)
                Ok(b1.const_int(1, false))
            }
            Pattern::IsType(type_name) => Err(format!("'is {}' requires a matched value. Use 'when value {{ is {} -> ... }}' instead of a condition chain.", type_name, type_name)),
            Pattern::Or(patterns) => {
                let mut result = b1.const_int(0, false);
                for p in patterns {
                    let m = self.compile_pattern_condition(p, None)?;
                    result = self.builder.build_or(result, m, "or").map_err(llvm_err)?;
                }
                Ok(result)
            }
            Pattern::Constructor { .. } => Ok(b1.const_int(1, false)),
            Pattern::Tuple(_) => Ok(b1.const_int(1, false)),
            Pattern::Null => Ok(b1.const_int(0, false)),
            Pattern::Expr(expr) => {
                let val = self.compile_expr(expr)?;
                match val {
                    TypedValue::Bool(b) => Ok(b),
                    TypedValue::Int(i) => {
                        let zero = self.i64_ty().const_int(0, false);
                        Ok(self.builder.build_int_compare(IntPredicate::NE, i, zero, "cond_expr")
                            .map_err(llvm_err)?)
                    }
                    _ => Ok(b1.const_int(1, false)),
                }
            }
        }
    }

    /// Compile a pattern match against a value (for ValueMatch).
    /// Returns an i1: true if the pattern matches, false otherwise.
    pub(super) fn compile_pattern_match(
        &mut self,
        pattern: &Pattern,
        val: &TypedValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let b1 = self.bool_ty();
        match pattern {
            Pattern::Wildcard => Ok(b1.const_int(1, false)),
            Pattern::Literal(lit) => {
                match lit {
                    Literal::Int(n) => {
                        if let TypedValue::Int(iv) = val {
                            let const_val = self.i64_ty().const_int(*n as u64, true);
                            Ok(self
                                .builder
                                .build_int_compare(IntPredicate::EQ, *iv, const_val, "match_int")
                                .map_err(llvm_err)?)
                        } else {
                            Ok(b1.const_int(0, false))
                        }
                    }
                    Literal::Bool(b) => {
                        if let TypedValue::Bool(bv) = val {
                            let const_val = b1.const_int(if *b { 1 } else { 0 }, false);
                            Ok(self
                                .builder
                                .build_int_compare(IntPredicate::EQ, *bv, const_val, "match_bool")
                                .map_err(llvm_err)?)
                        } else {
                            Ok(b1.const_int(0, false))
                        }
                    }
                    Literal::String(s) => {
                        if let TypedValue::Str(ptr) = val {
                            let str_val = self.load_string(*ptr)?;
                            // Build expected string constant
                            let str_bytes = s.as_bytes();
                            let arr_ty = self.context.i8_type().array_type(str_bytes.len() as u32);
                            self.str_pat_counter += 1;
                            let gname = format!(".str_pat_{}", self.str_pat_counter);
                            let global = self.add_module_global(arr_ty, &gname)?;
                            let arr = self.context.const_string(str_bytes, false);
                            global.set_initializer(&arr);
                            let pat_data = global.as_pointer_value();
                            let undef = self.string_type.get_undef();
                            let pat_len = self.i64_ty().const_int(str_bytes.len() as u64, false);
                            let s1 = self
                                .builder
                                .build_insert_value(undef, pat_len, 0, "pat_len")
                                .map_err(llvm_err)?;
                            let pat_str_agg = self
                                .builder
                                .build_insert_value(s1, pat_data, 1, "pat_str")
                                .map_err(llvm_err)?;
                            let pat_str = pat_str_agg.into_struct_value();
                            let cc = self
                                .call_rt("action_string_eq", &[str_val.into(), pat_str.into()])?;
                            let eq_result = cc.try_as_basic_value().unwrap_basic().into_int_value();
                            Ok(eq_result)
                        } else {
                            Ok(b1.const_int(0, false))
                        }
                    }
                    Literal::Char(c) => {
                        if let TypedValue::Int(iv) = val {
                            let const_val = self.i64_ty().const_int(*c as u64, false);
                            Ok(self
                                .builder
                                .build_int_compare(IntPredicate::EQ, *iv, const_val, "match_char")
                                .map_err(llvm_err)?)
                        } else {
                            Ok(b1.const_int(0, false))
                        }
                    }
                    Literal::Float(f) => {
                        if let TypedValue::Float(fv) = val {
                            let const_val = self.f64_ty().const_float(*f);
                            Ok(self
                                .builder
                                .build_float_compare(
                                    FloatPredicate::OEQ,
                                    *fv,
                                    const_val,
                                    "match_float",
                                )
                                .map_err(llvm_err)?)
                        } else if let TypedValue::Int(iv) = val {
                            let fv = self
                                .builder
                                .build_signed_int_to_float(*iv, self.f64_ty(), "int2float")
                                .map_err(llvm_err)?;
                            let const_val = self.f64_ty().const_float(*f);
                            Ok(self
                                .builder
                                .build_float_compare(
                                    FloatPredicate::OEQ,
                                    fv,
                                    const_val,
                                    "match_float_from_int",
                                )
                                .map_err(llvm_err)?)
                        } else {
                            Ok(b1.const_int(0, false))
                        }
                    }
                    Literal::Unit => Ok(b1.const_int(0, false)),
                }
            }
            Pattern::Variable(_) => Ok(b1.const_int(1, false)),
            Pattern::Null => {
                // Match against null: check if val is a nullable type with null flag == 1
                match val {
                    TypedValue::Nullable(ptr, inner_bt) => {
                        let nullable_bt: BasicTypeEnum = {
                            let b1 = self.null_flag_ty();
                            let fields: &[BasicTypeEnum] = &[b1.into(), *inner_bt];
                            self.context.struct_type(fields, false).into()
                        };
                        let loaded = self
                            .builder
                            .build_load(nullable_bt, *ptr, "null_ld")
                            .map_err(llvm_err)?;
                        let null_struct = loaded.into_struct_value();
                        let null_flag = self
                            .builder
                            .build_extract_value(null_struct, 0, "null_flag")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let one = self.null_flag_ty().const_int(1, false);
                        Ok(self
                            .builder
                            .build_int_compare(IntPredicate::EQ, null_flag, one, "is_null")
                            .map_err(llvm_err)?)
                    }
                    _ => Ok(b1.const_int(0, false)),
                }
            }
            Pattern::Constructor { name, args: _, .. } => {
                // Check if val is an enum with matching variant tag
                if let TypedValue::Enum(ptr, enum_st, ..) = val {
                    let bt: BasicTypeEnum = (*enum_st).into();
                    let loaded = self
                        .builder
                        .build_load(bt, *ptr, "enum_ld")
                        .map_err(llvm_err)?;
                    let enum_struct = loaded.into_struct_value();
                    let tag = self
                        .builder
                        .build_extract_value(enum_struct, 0, "tag")
                        .map_err(llvm_err)?
                        .into_int_value();

                    if let Some((_, variant)) = self.registry.lookup_variant(name) {
                        let expected_tag = self.i64_ty().const_int(variant.tag as u64, false);
                        Ok(self
                            .builder
                            .build_int_compare(IntPredicate::EQ, tag, expected_tag, "tag_match")
                            .map_err(llvm_err)?)
                    } else {
                        Ok(b1.const_int(0, false))
                    }
                } else {
                    Ok(b1.const_int(0, false))
                }
            }
            Pattern::Range(start, end) => {
                if let TypedValue::Int(iv) = val {
                    let s = self.compile_expr(start)?;
                    let e = self.compile_expr(end)?;
                    let (sv, ev) = match (&s, &e) {
                        (TypedValue::Int(a), TypedValue::Int(b)) => (*a, *b),
                        _ => return Err("Range bounds must be integers".to_string()),
                    };
                    let ge = self
                        .builder
                        .build_int_compare(IntPredicate::SGE, *iv, sv, "range_lo")
                        .map_err(llvm_err)?;
                    let lt = self
                        .builder
                        .build_int_compare(IntPredicate::SLT, *iv, ev, "range_hi")
                        .map_err(llvm_err)?;
                    Ok(self
                        .builder
                        .build_and(ge, lt, "range_match")
                        .map_err(llvm_err)?)
                } else {
                    Ok(b1.const_int(0, false))
                }
            }
            Pattern::IsType(type_name) => {
                // Enum variant check: `is Some` on an Option enum value
                if let Some((_, variant)) = self.registry.lookup_variant(type_name) {
                    if let TypedValue::Enum(ptr, enum_st, ..) = val {
                        let bt: BasicTypeEnum = (*enum_st).into();
                        let loaded = self
                            .builder
                            .build_load(bt, *ptr, "is_enum_ld")
                            .map_err(llvm_err)?;
                        let enum_struct = loaded.into_struct_value();
                        let tag = self
                            .builder
                            .build_extract_value(enum_struct, 0, "is_tag")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let expected_tag = self.i64_ty().const_int(variant.tag as u64, false);
                        return Ok(self
                            .builder
                            .build_int_compare(IntPredicate::EQ, tag, expected_tag, "is_variant")
                            .map_err(llvm_err)?);
                    }
                    return Ok(b1.const_int(0, false));
                }
                // Compile-time type check against TypedValue variant
                let matches = match type_name.as_str() {
                    "Int" => matches!(val, TypedValue::Int(_)),
                    "Float" => matches!(val, TypedValue::Float(_)),
                    "Bool" => matches!(val, TypedValue::Bool(_)),
                    "String" => matches!(val, TypedValue::Str(_)),
                    "list" => matches!(val, TypedValue::List(_)),
                    _ => false,
                };
                Ok(b1.const_int(if matches { 1 } else { 0 }, false))
            }
            Pattern::Or(patterns) => {
                let mut result = b1.const_int(0, false);
                for p in patterns {
                    let m = self.compile_pattern_match(p, val)?;
                    result = self
                        .builder
                        .build_or(result, m, "or_match")
                        .map_err(llvm_err)?;
                }
                Ok(result)
            }
            Pattern::Tuple(patterns) => {
                if let TypedValue::Struct(ptr, struct_ty) = val {
                    let bt: BasicTypeEnum = (*struct_ty).into();
                    let loaded = self
                        .builder
                        .build_load(bt, *ptr, "tup_ld")
                        .map_err(llvm_err)?;
                    let struct_val = loaded.into_struct_value();
                    let mut result = b1.const_int(1, false);
                    for (i, sub) in patterns.iter().enumerate() {
                        let field_val = self
                            .builder
                            .build_extract_value(struct_val, i as u32, &format!("tm{}", i))
                            .map_err(llvm_err)?;
                        let tv = self.bv_to_typed(field_val)?;
                        let sub_match = self.compile_pattern_match(sub, &tv)?;
                        result = self
                            .builder
                            .build_and(result, sub_match, "tup_and")
                            .map_err(llvm_err)?;
                    }
                    Ok(result)
                } else {
                    Ok(b1.const_int(0, false))
                }
            }
            Pattern::Expr(expr) => {
                // In value-match context, evaluate expression as a condition.
                // If the value matches (truthy), the expression acts as a guard.
                let val = self.compile_expr(expr)?;
                match val {
                    TypedValue::Bool(b) => Ok(b),
                    TypedValue::Int(i) => {
                        let zero = self.i64_ty().const_int(0, false);
                        Ok(self
                            .builder
                            .build_int_compare(IntPredicate::NE, i, zero, "expr_match")
                            .map_err(llvm_err)?)
                    }
                    _ => Ok(b1.const_int(1, false)),
                }
            }
        }
    }

    /// Bind pattern variables into the current scope.
    /// For ValueMatch: bind the matched value to the variable name.
    /// For ConditionChain: the variable binding is just the condition value itself.
    pub(super) fn bind_pattern_vars(
        &mut self,
        pattern: &Pattern,
        matched_val: Option<&TypedValue<'ctx>>,
        matched_type: Option<&Type>,
    ) -> Result<(), String> {
        match pattern {
            Pattern::Variable(name) => {
                if let Some(val) = matched_val {
                    match val {
                        TypedValue::Nullable(nullable_ptr, inner_bt) => {
                            // Binding a pattern variable from a nullable value:
                            // extract the inner non-null value (field 1) and bind that.
                            // inner_bt is the full nullable struct type {i1, T}
                            let nullable_st = inner_bt.into_struct_type();
                            let loaded = self
                                .builder
                                .build_load(nullable_st, *nullable_ptr, "patnv_ld")
                                .map_err(llvm_err)?;
                            let inner_val = self
                                .builder
                                .build_extract_value(loaded.into_struct_value(), 1, "patnv_inner")
                                .map_err(llvm_err)?;
                            let typed_inner = self.bv_to_typed(inner_val)?;
                            let ty = typed_inner.get_type_for_alloca(self);
                            let alloca = self.builder.build_alloca(ty, name).map_err(llvm_err)?;
                            self.store_value_to_alloca(&typed_inner, alloca)?;
                            self.scope
                                .set(name.clone(), alloca, ty, typed_inner.val_kind());
                        }
                        _ => {
                            let ty = val.get_type_for_alloca(self);
                            let alloca = self.builder.build_alloca(ty, name).map_err(llvm_err)?;
                            self.store_value_to_alloca(val, alloca)?;
                            self.scope.set(name.clone(), alloca, ty, val.val_kind());
                        }
                    }
                }
            }
            Pattern::Constructor {
                name: variant_name,
                args,
                named_fields,
            } => {
                if let Some(TypedValue::Enum(ptr, enum_st, ..)) = matched_val {
                    let bt: BasicTypeEnum = (*enum_st).into();
                    let loaded = self
                        .builder
                        .build_load(bt, *ptr, "enum_ld")
                        .map_err(llvm_err)?;
                    let enum_struct = loaded.into_struct_value();
                    let data_ptr = self
                        .builder
                        .build_extract_value(enum_struct, 1, "data")
                        .map_err(llvm_err)?
                        .into_pointer_value();

                    // Try to resolve variant params if we have the matched type
                    let resolved_params = self.resolve_variant_params(
                        variant_name,
                        matched_type,
                        args.len() + named_fields.len(),
                    );

                    if args.len() == 1 && named_fields.is_empty() && resolved_params.is_some() {
                        // Single positional param: use type info to create proper TypedValue
                        let param_types = resolved_params
                            .as_ref()
                            .ok_or_else(|| "Missing resolved params".to_string())?;
                        if let Some(param_ty) = param_types.first() {
                            if let Type::Named(name) = param_ty {
                                if self.named_structs.contains_key(name.as_str()) && args.len() == 1
                                {
                                    let st = self.named_structs[name.as_str()];
                                    let bt: BasicTypeEnum = st.into();
                                    let alloca = self
                                        .builder
                                        .build_alloca(bt, "pat_struct")
                                        .map_err(llvm_err)?;
                                    // Load struct from heap (data_ptr points to the struct data)
                                    let loaded = self
                                        .builder
                                        .build_load(bt, data_ptr, "ps_ld")
                                        .map_err(llvm_err)?;
                                    self.builder.build_store(alloca, loaded).map_err(llvm_err)?;
                                    let tv = TypedValue::Struct(alloca, st);
                                    self.bind_pattern_vars(&args[0], Some(&tv), Some(param_ty))?;
                                    return Ok(());
                                }
                            }
                        }
                    }

                    // Fallback: load values from heap data using correct byte offsets and types
                    let total_params = args.len() + named_fields.len();
                    if total_params > 0 {
                        // Calculate byte offsets matching compile_enum_construct layout:
                        // 8 bytes for scalars (Int/Float/Bool), 16 for struct types (String)
                        let mut offsets: Vec<u64> = Vec::with_capacity(total_params);
                        let mut cur: u64 = 0;
                        for pi in 0..total_params {
                            offsets.push(cur);
                            let ft = resolved_params.as_ref().and_then(|p| p.get(pi));
                            cur += match ft {
                                Some(Type::Named(n)) if n == "String" || n == "Str" => 16,
                                _ => 8,
                            };
                        }
                        let i8_ty = self.context.i8_type();
                        let mut cur = GepCursor::new(data_ptr);
                        // Bind positional sub-patterns with correct byte offsets
                        for (i, sub) in args.iter().enumerate() {
                            let fp = cur.offset_gep(&self.builder, i8_ty, offsets[i], "efld")?;
                            let tv: TypedValue =
                                match resolved_params.as_ref().and_then(|p| p.get(i)) {
                                    Some(Type::Named(n)) if n == "String" || n == "Str" => {
                                        let loaded = self
                                            .builder
                                            .build_load(self.string_type, fp, "efld_str")
                                            .map_err(llvm_err)?;
                                        let a = self
                                            .builder
                                            .build_alloca(self.string_type, "efld_stmp")
                                            .map_err(llvm_err)?;
                                        self.builder.build_store(a, loaded).map_err(llvm_err)?;
                                        TypedValue::Str(a)
                                    }
                                    Some(Type::Named(n)) if n == "Float" || n == "Double" => {
                                        let loaded = self
                                            .builder
                                            .build_load(self.f64_ty(), fp, "efld_f64")
                                            .map_err(llvm_err)?
                                            .into_float_value();
                                        TypedValue::Float(loaded)
                                    }
                                    _ => {
                                        let loaded = self
                                            .builder
                                            .build_load(self.i64_ty(), fp, "efld_i64")
                                            .map_err(llvm_err)?
                                            .into_int_value();
                                        TypedValue::Int(loaded)
                                    }
                                };
                            let sub_ty = resolved_params.as_ref().and_then(|p| p.get(i));
                            self.bind_pattern_vars(sub, Some(&tv), sub_ty)?;
                        }
                        // Bind named fields similarly (cursor continues from last positional offset)
                        for (ni, (_, sub)) in named_fields.iter().enumerate() {
                            let idx = args.len() + ni;
                            let fp = cur.offset_gep(&self.builder, i8_ty, offsets[idx], "nefld")?;
                            let tv: TypedValue =
                                match resolved_params.as_ref().and_then(|p| p.get(idx)) {
                                    Some(Type::Named(n)) if n == "String" || n == "Str" => {
                                        let loaded = self
                                            .builder
                                            .build_load(self.string_type, fp, "nefld_str")
                                            .map_err(llvm_err)?;
                                        let a = self
                                            .builder
                                            .build_alloca(self.string_type, "nefld_stmp")
                                            .map_err(llvm_err)?;
                                        self.builder.build_store(a, loaded).map_err(llvm_err)?;
                                        TypedValue::Str(a)
                                    }
                                    Some(Type::Named(n)) if n == "Float" || n == "Double" => {
                                        let loaded = self
                                            .builder
                                            .build_load(self.f64_ty(), fp, "nefld_f64")
                                            .map_err(llvm_err)?
                                            .into_float_value();
                                        TypedValue::Float(loaded)
                                    }
                                    _ => {
                                        let loaded = self
                                            .builder
                                            .build_load(self.i64_ty(), fp, "nefld_i64")
                                            .map_err(llvm_err)?
                                            .into_int_value();
                                        TypedValue::Int(loaded)
                                    }
                                };
                            let sub_ty = resolved_params.as_ref().and_then(|p| p.get(idx));
                            self.bind_pattern_vars(sub, Some(&tv), sub_ty)?;
                        }
                    }
                } else {
                    // constructor not in registry (builtin stdlib enum, handled elsewhere)
                }
            }
            Pattern::Or(patterns) => {
                // For Or patterns, bind the first pattern's variables (simplified)
                if let Some(first) = patterns.first() {
                    self.bind_pattern_vars(first, matched_val, matched_type)?;
                }
            }
            Pattern::Tuple(patterns) => {
                if let Some(TypedValue::Struct(ptr, struct_ty)) = matched_val {
                    let bt: BasicTypeEnum = (*struct_ty).into();
                    let loaded = self
                        .builder
                        .build_load(bt, *ptr, "tuple_ld")
                        .map_err(llvm_err)?;
                    let struct_val = loaded.into_struct_value();
                    for (i, sub) in patterns.iter().enumerate() {
                        let field_val = self
                            .builder
                            .build_extract_value(struct_val, i as u32, &format!("t{}", i))
                            .map_err(llvm_err)?;
                        let tv = self.bv_to_typed(field_val)?;
                        self.bind_pattern_vars(sub, Some(&tv), None)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Resolve variant parameter types using the matched expression's AST type.
    /// For example, if matched_type is Option<Date> and variant is Some(T),
    /// resolve T = Date and return [Date].
    pub(super) fn resolve_variant_params(
        &self,
        variant_name: &str,
        matched_type: Option<&Type>,
        expected_count: usize,
    ) -> Option<Vec<Type>> {
        let mt = matched_type?;
        // Get the enum info for this variant
        let (enum_info, variant_info) = self.registry.lookup_variant(variant_name)?;
        // Check if the matched type matches the enum
        let enum_name = &enum_info.name;
        // Extract the concrete type params from matched_type
        let concrete_params: Option<&Vec<Type>> = match mt {
            Type::Named(n) if n == enum_name => Some(&vec![]), // for non-generic enums or enum with no params
            Type::Generic(base, params) => {
                if let Type::Named(n) = base.as_ref() {
                    if n == enum_name {
                        Some(params)
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        let concrete_params = concrete_params?;

        // Map type variable names to concrete types
        let mut type_map: HashMap<String, Type> = HashMap::new();
        for (i, tv) in enum_info.type_params.iter().enumerate() {
            if let Some(ct) = concrete_params.get(i) {
                type_map.insert(tv.clone(), ct.clone());
            }
        }

        // Now resolve each variant parameter
        let mut resolved = Vec::new();
        for param in &variant_info.params {
            let param_type = match param {
                EnumVariantParam::Positional(ty) => ty,
                EnumVariantParam::Named { ty, .. } => ty,
            };
            let concrete = self.resolve_type(param_type, &type_map);
            resolved.push(concrete);
        }

        if resolved.len() >= expected_count {
            Some(resolved)
        } else {
            None
        }
    }

    /// Resolve a type by substituting type variables with concrete types
    pub(super) fn resolve_type(&self, ty: &Type, type_map: &HashMap<String, Type>) -> Type {
        match ty {
            Type::Named(name) | Type::TypeVar(name) => {
                type_map.get(name).cloned().unwrap_or_else(|| ty.clone())
            }
            Type::Generic(base, params) => {
                let new_base = self.resolve_type(base, type_map);
                let new_params: Vec<Type> = params
                    .iter()
                    .map(|p| self.resolve_type(p, type_map))
                    .collect();
                Type::Generic(Box::new(new_base), new_params)
            }
            _ => ty.clone(),
        }
    }

    /// Store a branch result to the alloca, coercing between nullable and non-nullable.
    fn store_branch_result(
        &mut self,
        v: &TypedValue<'ctx>,
        alloca: PointerValue<'ctx>,
        result_type: &Type,
    ) -> Result<(), String> {
        let target_is_nullable = matches!(result_type, Type::Nullable(_));
        let value_is_nullable = matches!(v, TypedValue::Nullable(..));

        match (target_is_nullable, value_is_nullable) {
            (true, true) | (false, false) => {
                // Same nullability: store directly
                self.store_value_to_alloca(v, alloca)?;
            }
            (true, false) => {
                // Non-nullable value into nullable target: wrap in {i1=0, value}
                let inner_bt = self.ast_type_to_basic_type(&result_type);
                let struct_ty = inner_bt.into_struct_type();
                let undef = struct_ty.get_undef();
                let flag = self.null_flag_ty().const_int(0, false);
                let with_flag = self
                    .builder
                    .build_insert_value(undef, flag, 0, "br_flag")
                    .map_err(llvm_err)?;
                let bv = v
                    .to_bv()
                    .unwrap_or_else(|| self.i64_ty().const_int(0, false).into());
                let wrapped = self
                    .builder
                    .build_insert_value(with_flag, bv, 1, "br_val")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(alloca, wrapped)
                    .map_err(llvm_err)?;
            }
            (false, true) => {
                // Nullable value into non-nullable target: extract inner field
                if let TypedValue::Nullable(ptr, ty) = v {
                    let loaded = self
                        .builder
                        .build_load(*ty, *ptr, "br_ld")
                        .map_err(llvm_err)?;
                    let inner = self
                        .builder
                        .build_extract_value(loaded.into_struct_value(), 1, "br_inner")
                        .map_err(llvm_err)?;
                    self.builder.build_store(alloca, inner).map_err(llvm_err)?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn compile_when_branch_lazy(
        &mut self,
        c: IntValue<'ctx>,
        then_expr: &Expr,
        else_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile when outside function".to_string())?;

        let then_diverges = matches!(&then_expr.kind, ExprKind::Continue | ExprKind::Break);
        let else_diverges = matches!(&else_expr.kind, ExprKind::Continue | ExprKind::Break);

        // Infer result type from both branches (prefer nullable if either is nullable)
        let then_inferred = if !then_diverges {
            self.infer_expr_type(then_expr)
        } else {
            Type::Named("Int".into())
        };
        let else_inferred = if !else_diverges {
            self.infer_expr_type(else_expr)
        } else {
            then_inferred.clone()
        };
        // Choose the result type: prefer nullable if either branch is nullable.
        // If one branch is Nullable<Nothing> (null literal), promote to Nullable<T>
        // where T is the other branch's type, so null propagates correctly.
        let result_type = match (&then_inferred, &else_inferred) {
            // Both non-nullable: use then type
            (a, b) if !matches!(a, Type::Nullable(_)) && !matches!(b, Type::Nullable(_)) => {
                then_inferred.clone()
            }
            // Nullable<Nothing> + T → Nullable<T>
            (Type::Nullable(inner), other) | (other, Type::Nullable(inner)) if matches!(inner.as_ref(), Type::Named(n) if n == "Nothing") => {
                match other {
                    Type::Nullable(oi) => Type::Nullable(oi.clone()),
                    _ => Type::Nullable(Box::new(other.clone())),
                }
            }
            // Nullable<T> + Nullable<T> or Nullable<T> + T → Nullable<T>
            (Type::Nullable(inner), _) | (_, Type::Nullable(inner)) => {
                Type::Nullable(inner.clone())
            }
            _ => then_inferred.clone(),
        };
        let result_ty: BasicTypeEnum = self.ast_type_to_basic_type(&result_type);

        let then_block = self.context.append_basic_block(current_fn, "when_then");
        let else_block = self.context.append_basic_block(current_fn, "when_else");
        let merge_block = self.context.append_basic_block(current_fn, "when_merge");

        let _ = self
            .builder
            .build_conditional_branch(c, then_block, else_block);

        // Alloca at entry for the non-divergent branch(es) to store into
        let result_alloca = if !then_diverges || !else_diverges {
            let entry = current_fn.get_first_basic_block().unwrap();
            let saved_pos = self.builder.get_insert_block();
            match entry.get_first_instruction() {
                Some(instr) => {
                    let _ = self.builder.position_before(&instr);
                }
                None => self.builder.position_at_end(entry),
            }
            let alloca = self
                .builder
                .build_alloca(result_ty, "when_result")
                .map_err(llvm_err)?;
            if let Some(block) = saved_pos {
                self.builder.position_at_end(block);
            }
            Some(alloca)
        } else {
            None
        };

        // Track enum inner type from both branches for bv_to_typed
        let mut when_enum_info: Option<(InnerType, bool)> = None;

        // Then branch
        self.builder.position_at_end(then_block);
        if then_diverges {
            self.compile_expr(then_expr)?;
            // divergent: branch already built by compile_expr, nothing more
        } else {
            let tv = self.compile_expr(then_expr)?;
            if let TypedValue::Enum(_, _, inner, rc) = &tv {
                when_enum_info = Some((*inner, *rc));
            }
            self.store_branch_result(
                &tv,
                result_alloca.ok_or_else(|| "No result alloca".to_string())?,
                &result_type,
            )?;
            let _ = self.builder.build_unconditional_branch(merge_block);
        }

        // Else branch
        self.builder.position_at_end(else_block);
        if else_diverges {
            self.compile_expr(else_expr)?;
            // divergent: branch already built by compile_expr, nothing more
        } else {
            let ev = self.compile_expr(else_expr)?;
            if when_enum_info.is_none() {
                if let TypedValue::Enum(_, _, inner, rc) = &ev {
                    when_enum_info = Some((*inner, *rc));
                }
            }
            self.store_branch_result(
                &ev,
                result_alloca.ok_or_else(|| "No result alloca".to_string())?,
                &result_type,
            )?;
            let _ = self.builder.build_unconditional_branch(merge_block);
        }

        // Merge: load result if at least one branch reaches here
        self.builder.position_at_end(merge_block);
        if let Some(alloca) = result_alloca {
            self.last_enum_inner = when_enum_info;
            let loaded = self
                .builder
                .build_load(result_ty, alloca, "when_ld")
                .map_err(llvm_err)?;
            self.bv_to_typed(loaded)
        } else {
            // Both branches diverged — this merge block is unreachable
            Ok(TypedValue::Unit)
        }
    }

    pub(super) fn compile_hir_when(
        &mut self,
        w: &action_frontend::hir::HirWhen,
    ) -> Result<TypedValue<'ctx>, String> {
        use action_frontend::hir::{HirExprKind, HirWhenKind};
        match &w.kind {
            HirWhenKind::OneLine {
                condition,
                then_expr,
                else_expr,
            } => {
                let c = self.compile_hir_expr(condition)?;
                let c_bool = match c {
                    TypedValue::Bool(b) => b,
                    _ => return Err("when condition must be boolean".to_string()),
                };
                let smart_var: Option<String> = match &condition.kind {
                    HirExprKind::Binary(lhs, BinaryOp::Neq, rhs)
                    | HirExprKind::Binary(lhs, BinaryOp::Eq, rhs) => match (&lhs.kind, &rhs.kind) {
                        (HirExprKind::Ident(name), HirExprKind::Null)
                        | (HirExprKind::Null, HirExprKind::Ident(name)) => Some(name.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(ref var) = smart_var {
                    let is_eq = matches!(&condition.kind, HirExprKind::Binary(_, BinaryOp::Eq, _));
                    if is_eq {
                        let negated = self
                            .builder
                            .build_not(c_bool, "neg_cond")
                            .map_err(llvm_err)?;
                        self.not_null_set.insert(var.clone());
                        let result =
                            self.compile_when_branch_lazy_hir(negated, else_expr, then_expr);
                        self.not_null_set.remove(var);
                        result
                    } else {
                        self.not_null_set.insert(var.clone());
                        let result =
                            self.compile_when_branch_lazy_hir(c_bool, then_expr, else_expr);
                        self.not_null_set.remove(var);
                        result
                    }
                } else {
                    self.compile_when_branch_lazy_hir(c_bool, then_expr, else_expr)
                }
            }
            HirWhenKind::ValueMatch { value, arms } => self.compile_hir_value_match(value, arms),
            HirWhenKind::ConditionChain { arms } => self.compile_hir_condition_chain(arms),
        }
    }

    fn compile_hir_value_match(
        &mut self,
        value: &action_frontend::hir::HirExpr,
        arms: &[action_frontend::hir::HirWhenArm],
    ) -> Result<TypedValue<'ctx>, String> {
        if arms.is_empty() {
            return Ok(TypedValue::Unit);
        }
        self.registry
            .check_when_exhaustive(&arms.iter().map(|a| a.to_when_arm()).collect::<Vec<_>>())?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile when outside function")?;

        let matched_val = self.compile_hir_expr(value)?;
        let matched_type = value.ty.clone();
        let result_type = arms
            .first()
            .map(|a| a.body.ty.clone())
            .unwrap_or_else(|| Type::Named("Int".into()));
        let result_ty = self.ast_type_to_basic_type(&result_type);

        let entry = current_fn.get_first_basic_block().unwrap();
        let saved_pos = self.builder.get_insert_block();
        match entry.get_first_instruction() {
            Some(instr) => {
                let _ = self.builder.position_before(&instr);
            }
            None => self.builder.position_at_end(entry),
        }
        let result_alloca = self
            .builder
            .build_alloca(result_ty, "match_result")
            .map_err(llvm_err)?;
        let zero = result_ty.const_zero();
        self.builder
            .build_store(result_alloca, zero)
            .map_err(llvm_err)?;
        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }

        let merge_block = self.context.append_basic_block(current_fn, "match_merge");
        let mut next_check = self.context.append_basic_block(current_fn, "match_check0");
        let _ = self.builder.build_unconditional_branch(next_check);
        let mut result_enum_info: Option<(InnerType, bool)> = None;

        for (i, hir_arm) in arms.iter().enumerate() {
            let arm = hir_arm.to_when_arm();
            let is_last = i == arms.len() - 1;
            self.builder.position_at_end(next_check);

            let matches = self.compile_pattern_match(&arm.pattern, &matched_val)?;
            let matches = if hir_arm.guard.is_some() {
                let mut saved_scope = Scope::new();
                std::mem::swap(&mut self.scope, &mut saved_scope);
                self.scope = Scope::with_parent(saved_scope);
                self.bind_pattern_vars(&arm.pattern, Some(&matched_val), Some(&matched_type))?;
                let guard_matches = self.compile_guard_hir(hir_arm.guard.as_deref())?;
                let combined = self
                    .builder
                    .build_and(matches, guard_matches, "guard_and")
                    .map_err(llvm_err)?;
                self.emit_scope_cleanup()?;
                let mut parent = Scope::new();
                std::mem::swap(&mut self.scope, &mut parent);
                if let Some(p) = parent.parent {
                    self.scope = *p;
                }
                combined
            } else {
                matches
            };

            let body_block = self
                .context
                .append_basic_block(current_fn, &format!("match_body{}", i));
            if is_last {
                let _ = self
                    .builder
                    .build_conditional_branch(matches, body_block, merge_block);
            } else {
                next_check = self
                    .context
                    .append_basic_block(current_fn, &format!("match_check{}", i + 1));
                let _ = self
                    .builder
                    .build_conditional_branch(matches, body_block, next_check);
            }

            self.builder.position_at_end(body_block);
            let mut saved_scope = Scope::new();
            std::mem::swap(&mut self.scope, &mut saved_scope);
            self.scope = Scope::with_parent(saved_scope);
            self.bind_pattern_vars(&arm.pattern, Some(&matched_val), Some(&matched_type))?;
            let body_val = self.compile_hir_expr(&hir_arm.body)?;
            if result_enum_info.is_none() {
                if let TypedValue::Enum(_, _, inner, rc) = &body_val {
                    result_enum_info = Some((*inner, *rc));
                }
            }
            self.store_branch_result(&body_val, result_alloca, &result_type)?;
            self.emit_scope_cleanup()?;
            let mut parent = Scope::new();
            std::mem::swap(&mut self.scope, &mut parent);
            if let Some(p) = parent.parent {
                self.scope = *p;
            }
            let _ = self.builder.build_unconditional_branch(merge_block);
        }

        self.builder.position_at_end(merge_block);
        self.last_enum_inner = result_enum_info;
        let loaded = self
            .builder
            .build_load(result_ty, result_alloca, "match_ld")
            .map_err(llvm_err)?;
        self.bv_to_typed(loaded)
    }

    fn compile_hir_condition_chain(
        &mut self,
        arms: &[action_frontend::hir::HirWhenArm],
    ) -> Result<TypedValue<'ctx>, String> {
        let ast_arms: Vec<WhenArm> = arms.iter().map(|a| a.to_when_arm()).collect();
        self.compile_condition_chain_hir(&ast_arms, arms)
    }

    fn compile_condition_chain_hir(
        &mut self,
        arms: &[WhenArm],
        hir_arms: &[action_frontend::hir::HirWhenArm],
    ) -> Result<TypedValue<'ctx>, String> {
        if arms.is_empty() {
            return Ok(TypedValue::Unit);
        }

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile when outside function")?;

        let merge_block = self.context.append_basic_block(current_fn, "chain_merge");
        let result_type = hir_arms
            .first()
            .map(|a| a.body.ty.clone())
            .unwrap_or_else(|| Type::Named("Int".into()));
        let result_ty = self.ast_type_to_basic_type(&result_type);

        let entry = current_fn.get_first_basic_block().unwrap();
        let saved_pos = self.builder.get_insert_block();
        match entry.get_first_instruction() {
            Some(instr) => {
                let _ = self.builder.position_before(&instr);
            }
            None => self.builder.position_at_end(entry),
        }
        let result_alloca = self
            .builder
            .build_alloca(result_ty, "chain_result")
            .map_err(llvm_err)?;
        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }

        let mut next_check = self.context.append_basic_block(current_fn, "chain_check0");
        let _ = self.builder.build_unconditional_branch(next_check);
        let mut chain_enum_info: Option<(InnerType, bool)> = None;

        for (i, (arm, hir_arm)) in arms.iter().zip(hir_arms.iter()).enumerate() {
            let is_last = i == arms.len() - 1;
            self.builder.position_at_end(next_check);

            let matches = self.compile_pattern_condition(&arm.pattern, None)?;
            let matches = if arm.guard.is_some() {
                let mut saved_scope = Scope::new();
                std::mem::swap(&mut self.scope, &mut saved_scope);
                self.scope = Scope::with_parent(saved_scope);
                self.bind_pattern_vars(&arm.pattern, None, None)?;
                let guard_matches = self.compile_guard_hir(hir_arm.guard.as_deref())?;
                let combined = self
                    .builder
                    .build_and(matches, guard_matches, "guard_and")
                    .map_err(llvm_err)?;
                self.emit_scope_cleanup()?;
                let mut parent = Scope::new();
                std::mem::swap(&mut self.scope, &mut parent);
                if let Some(p) = parent.parent {
                    self.scope = *p;
                }
                combined
            } else {
                matches
            };
            let body_block = self
                .context
                .append_basic_block(current_fn, &format!("chain_body{}", i));

            if is_last {
                let _ = self
                    .builder
                    .build_conditional_branch(matches, body_block, merge_block);
            } else {
                next_check = self
                    .context
                    .append_basic_block(current_fn, &format!("chain_check{}", i + 1));
                let _ = self
                    .builder
                    .build_conditional_branch(matches, body_block, next_check);
            }

            self.builder.position_at_end(body_block);
            let mut saved_scope = Scope::new();
            std::mem::swap(&mut self.scope, &mut saved_scope);
            self.scope = Scope::with_parent(saved_scope);
            self.bind_pattern_vars(&arm.pattern, None, None)?;
            let body_val = self.compile_hir_expr(&hir_arm.body)?;
            if chain_enum_info.is_none() {
                if let TypedValue::Enum(_, _, inner, rc) = &body_val {
                    chain_enum_info = Some((*inner, *rc));
                }
            }
            self.store_branch_result(&body_val, result_alloca, &result_type)?;
            self.emit_scope_cleanup()?;
            let mut parent = Scope::new();
            std::mem::swap(&mut self.scope, &mut parent);
            if let Some(p) = parent.parent {
                self.scope = *p;
            }
            let _ = self.builder.build_unconditional_branch(merge_block);
        }

        self.builder.position_at_end(merge_block);
        self.last_enum_inner = chain_enum_info;
        let loaded = self
            .builder
            .build_load(result_ty, result_alloca, "chain_ld")
            .map_err(llvm_err)?;
        self.bv_to_typed(loaded)
    }

    fn compile_guard_hir(
        &mut self,
        guard: Option<&action_frontend::hir::HirExpr>,
    ) -> Result<IntValue<'ctx>, String> {
        match guard {
            Some(expr) => {
                let val = self.compile_hir_expr(expr)?;
                match val {
                    TypedValue::Bool(b) => Ok(b),
                    TypedValue::Int(i) => {
                        let zero = self.i64_ty().const_int(0, false);
                        Ok(self
                            .builder
                            .build_int_compare(IntPredicate::NE, i, zero, "guard_truthy")
                            .map_err(llvm_err)?)
                    }
                    _ => Ok(self.bool_ty().const_int(1, false)),
                }
            }
            None => Ok(self.bool_ty().const_int(1, false)),
        }
    }

    pub(super) fn compile_when_branch_lazy_hir(
        &mut self,
        c: IntValue<'ctx>,
        then_expr: &action_frontend::hir::HirExpr,
        else_expr: &action_frontend::hir::HirExpr,
    ) -> Result<TypedValue<'ctx>, String> {
        let then_diverges = matches!(
            then_expr.kind,
            action_frontend::hir::HirExprKind::Continue | action_frontend::hir::HirExprKind::Break
        );
        let else_diverges = matches!(
            else_expr.kind,
            action_frontend::hir::HirExprKind::Continue | action_frontend::hir::HirExprKind::Break
        );

        let then_inferred = if !then_diverges {
            then_expr.ty.clone()
        } else {
            Type::Named("Int".into())
        };
        let else_inferred = if !else_diverges {
            else_expr.ty.clone()
        } else {
            then_inferred.clone()
        };
        let result_type = match (&then_inferred, &else_inferred) {
            (a, b) if !matches!(a, Type::Nullable(_)) && !matches!(b, Type::Nullable(_)) => {
                then_inferred.clone()
            }
            (Type::Nullable(inner), other) | (other, Type::Nullable(inner)) if matches!(inner.as_ref(), Type::Named(n) if n == "Nothing") => {
                match other {
                    Type::Nullable(oi) => Type::Nullable(oi.clone()),
                    _ => Type::Nullable(Box::new(other.clone())),
                }
            }
            (Type::Nullable(inner), _) | (_, Type::Nullable(inner)) => {
                Type::Nullable(inner.clone())
            }
            _ => then_inferred.clone(),
        };
        let result_ty: BasicTypeEnum = self.ast_type_to_basic_type(&result_type);

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile when outside function".to_string())?;

        let then_block = self.context.append_basic_block(current_fn, "when_then");
        let else_block = self.context.append_basic_block(current_fn, "when_else");
        let merge_block = self.context.append_basic_block(current_fn, "when_merge");

        let _ = self
            .builder
            .build_conditional_branch(c, then_block, else_block);

        let result_alloca = if !then_diverges || !else_diverges {
            let entry = current_fn.get_first_basic_block().unwrap();
            let saved_pos = self.builder.get_insert_block();
            match entry.get_first_instruction() {
                Some(instr) => {
                    let _ = self.builder.position_before(&instr);
                }
                None => self.builder.position_at_end(entry),
            }
            let alloca = self
                .builder
                .build_alloca(result_ty, "when_result")
                .map_err(llvm_err)?;
            if let Some(block) = saved_pos {
                self.builder.position_at_end(block);
            }
            Some(alloca)
        } else {
            None
        };

        let mut when_enum_info: Option<(InnerType, bool)> = None;

        self.builder.position_at_end(then_block);
        if then_diverges {
            self.compile_hir_expr(then_expr)?;
        } else {
            let tv = self.compile_hir_expr(then_expr)?;
            if let TypedValue::Enum(_, _, inner, rc) = &tv {
                when_enum_info = Some((*inner, *rc));
            }
            self.store_branch_result(
                &tv,
                result_alloca.ok_or_else(|| "No result alloca".to_string())?,
                &result_type,
            )?;
            let _ = self.builder.build_unconditional_branch(merge_block);
        }

        self.builder.position_at_end(else_block);
        if else_diverges {
            self.compile_hir_expr(else_expr)?;
        } else {
            let ev = self.compile_hir_expr(else_expr)?;
            if when_enum_info.is_none() {
                if let TypedValue::Enum(_, _, inner, rc) = &ev {
                    when_enum_info = Some((*inner, *rc));
                }
            }
            self.store_branch_result(
                &ev,
                result_alloca.ok_or_else(|| "No result alloca".to_string())?,
                &result_type,
            )?;
            let _ = self.builder.build_unconditional_branch(merge_block);
        }

        self.builder.position_at_end(merge_block);
        if let Some(alloca) = result_alloca {
            self.last_enum_inner = when_enum_info;
            let loaded = self
                .builder
                .build_load(result_ty, alloca, "when_ld")
                .map_err(llvm_err)?;
            self.bv_to_typed(loaded)
        } else {
            Ok(TypedValue::Unit)
        }
    }
}
