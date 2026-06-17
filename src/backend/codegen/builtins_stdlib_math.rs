// Submodule: builtins_stdlib_math — math builtin functions
//
// Extracted from builtins_stdlib.rs.
//
// Submodule: builtins_stdlib

use crate::ast::*;
use inkwell::FloatPredicate;
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn builtin_stdlib_math(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "abs" => {
                if args.len() != 1 {
                    return Err("abs expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => {
                        let zero = self.i64_ty().const_int(0, false);
                        let neg = self.builder.build_int_neg(iv, "neg").map_err(llvm_err)?;
                        let is_neg = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, iv, zero, "is_neg")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_neg, neg, iv, "abs_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    TypedValue::Float(fv) => {
                        let zero = self.f64_ty().const_float(0.0);
                        let neg = self.builder.build_float_neg(fv, "neg").map_err(llvm_err)?;
                        let is_neg = self
                            .builder
                            .build_float_compare(FloatPredicate::OLT, fv, zero, "is_neg")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_neg, neg, fv, "fabs_result")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("abs: argument must be Int or Float".to_string()),
                }
            }
            "min" => {
                if args.len() != 2 {
                    return Err("min expects 2 arguments".to_string());
                }
                let a = self.compile_expr(&args[0])?;
                let b = self.compile_expr(&args[1])?;
                match (&a, &b) {
                    (TypedValue::Int(av), TypedValue::Int(bv)) => {
                        let is_lt = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, *av, *bv, "is_lt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_lt, *av, *bv, "min_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    (TypedValue::Float(av), TypedValue::Float(bv)) => {
                        let is_lt = self
                            .builder
                            .build_float_compare(FloatPredicate::OLT, *av, *bv, "is_lt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_lt, *av, *bv, "fmin_result")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("min: arguments must be both Int or both Float".to_string()),
                }
            }
            "max" => {
                if args.len() != 2 {
                    return Err("max expects 2 arguments".to_string());
                }
                let a = self.compile_expr(&args[0])?;
                let b = self.compile_expr(&args[1])?;
                match (&a, &b) {
                    (TypedValue::Int(av), TypedValue::Int(bv)) => {
                        let is_gt = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, *av, *bv, "is_gt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_gt, *av, *bv, "max_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    (TypedValue::Float(av), TypedValue::Float(bv)) => {
                        let is_gt = self
                            .builder
                            .build_float_compare(FloatPredicate::OGT, *av, *bv, "is_gt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_gt, *av, *bv, "fmax_result")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("max: arguments must be both Int or both Float".to_string()),
                }
            }
            "sqrt" => {
                if args.len() != 1 {
                    return Err("sqrt expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let sqrt_fn = self.module.get_function("sqrt").unwrap();
                let r = self
                    .builder
                    .build_call(sqrt_fn, &[fv.into()], "sqrt")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("sqrt failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "cbrt" => {
                if args.len() != 1 {
                    return Err("cbrt expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let cbrt_fn = self.module.get_function("cbrt").unwrap();
                let r = self
                    .builder
                    .build_call(cbrt_fn, &[fv.into()], "cbrt")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("cbrt failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "sin" => {
                if args.len() != 1 {
                    return Err("sin expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("sin").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "sin")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("sin failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "cos" => {
                if args.len() != 1 {
                    return Err("cos expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("cos").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "cos")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("cos failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "tan" => {
                if args.len() != 1 {
                    return Err("tan expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("tan").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "tan")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("tan failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "asin" => {
                if args.len() != 1 {
                    return Err("asin expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("asin").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "asin")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("asin failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "acos" => {
                if args.len() != 1 {
                    return Err("acos expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("acos").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "acos")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("acos failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "atan" => {
                if args.len() != 1 {
                    return Err("atan expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("atan").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "atan")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("atan failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "atan2" => {
                if args.len() != 2 {
                    return Err("atan2 expects 2 arguments".to_string());
                }
                let y = self.compile_expr(&args[0])?;
                let x = self.compile_expr(&args[1])?;
                let yv = self.typed_to_float(&y)?;
                let xv = self.typed_to_float(&x)?;
                let f = self.module.get_function("atan2").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[yv.into(), xv.into()], "atan2")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("atan2 failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "log" => {
                if args.len() != 1 {
                    return Err("log expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("log").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "log")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("log failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "log2" => {
                if args.len() != 1 {
                    return Err("log2 expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("log2").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "log2")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("log2 failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "log10" => {
                if args.len() != 1 {
                    return Err("log10 expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("log10").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "log10")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("log10 failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "exp" => {
                if args.len() != 1 {
                    return Err("exp expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("exp").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "exp")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("exp failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "floor" => {
                if args.len() != 1 {
                    return Err("floor expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("floor").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "floor")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("floor failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "ceil" => {
                if args.len() != 1 {
                    return Err("ceil expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("ceil").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "ceil")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("ceil failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "round" => {
                if args.len() != 1 {
                    return Err("round expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("round").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "round")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("round failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "pi" => {
                if !args.is_empty() {
                    return Err("pi expects no arguments".to_string());
                }
                let pi_val = self.f64_ty().const_float(std::f64::consts::PI);
                Ok(TypedValue::Float(pi_val))
            }
            "e" => {
                if !args.is_empty() {
                    return Err("e expects no arguments".to_string());
                }
                let e_val = self.f64_ty().const_float(std::f64::consts::E);
                Ok(TypedValue::Float(e_val))
            }
            "clamp" => {
                if args.len() != 3 {
                    return Err("clamp expects 3 arguments (value, min, max)".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                let min = self.compile_expr(&args[1])?;
                let max = self.compile_expr(&args[2])?;
                match (&val, &min, &max) {
                    (TypedValue::Int(vv), TypedValue::Int(mn), TypedValue::Int(mx)) => {
                        let lt_min = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, *vv, *mn, "lt_min")
                            .map_err(llvm_err)?;
                        let r1 = self
                            .builder
                            .build_select(lt_min, *mn, *vv, "clamp1")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let gt_max = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, r1, *mx, "gt_max")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_select(gt_max, *mx, r1, "clamp2")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(r2))
                    }
                    (TypedValue::Float(vv), TypedValue::Float(mn), TypedValue::Float(mx)) => {
                        let lt_min = self
                            .builder
                            .build_float_compare(FloatPredicate::OLT, *vv, *mn, "lt_min")
                            .map_err(llvm_err)?;
                        let r1 = self
                            .builder
                            .build_select(lt_min, *mn, *vv, "clamp1")
                            .map_err(llvm_err)?
                            .into_float_value();
                        let gt_max = self
                            .builder
                            .build_float_compare(FloatPredicate::OGT, r1, *mx, "gt_max")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_select(gt_max, *mx, r1, "clamp2")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(r2))
                    }
                    _ => Err("clamp: arguments must be all Int or all Float".to_string()),
                }
            }
            "isNaN" => {
                if args.len() != 1 {
                    return Err("isNaN expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let is_nan = self
                    .builder
                    .build_float_compare(FloatPredicate::UNO, fv, fv, "isNaN")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_nan))
            }
            "isInfinite" => {
                if args.len() != 1 {
                    return Err("isInfinite expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let inf = self.f64_ty().const_float(f64::INFINITY);
                let is_pos_inf = self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, fv, inf, "is_pos_inf")
                    .map_err(llvm_err)?;
                let neg_inf = self.f64_ty().const_float(f64::NEG_INFINITY);
                let is_neg_inf = self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, fv, neg_inf, "is_neg_inf")
                    .map_err(llvm_err)?;
                let is_inf = self
                    .builder
                    .build_or(is_pos_inf, is_neg_inf, "is_inf")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_inf))
            }
            "pow" => {
                if args.len() != 2 {
                    return Err("pow expects 2 arguments".to_string());
                }
                let base = self.compile_expr(&args[0])?;
                let exp = self.compile_expr(&args[1])?;
                match (&base, &exp) {
                    (TypedValue::Float(bv), TypedValue::Float(ev)) => {
                        let cc = self.call_rt("action_pow", &[(*bv).into(), (*ev).into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    (TypedValue::Int(bv), TypedValue::Int(ev)) => {
                        let bf = self
                            .builder
                            .build_signed_int_to_float(*bv, self.f64_ty(), "bf")
                            .map_err(llvm_err)?;
                        let ef = self
                            .builder
                            .build_signed_int_to_float(*ev, self.f64_ty(), "ef")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_pow", &[bf.into(), ef.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    // Mixed Int/Float → promote Int to Float
                    (TypedValue::Int(bv), TypedValue::Float(ev)) => {
                        let bf = self
                            .builder
                            .build_signed_int_to_float(*bv, self.f64_ty(), "bf")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_pow", &[bf.into(), (*ev).into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    (TypedValue::Float(bv), TypedValue::Int(ev)) => {
                        let ef = self
                            .builder
                            .build_signed_int_to_float(*ev, self.f64_ty(), "ef")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_pow", &[(*bv).into(), ef.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("pow: arguments must be numeric".to_string()),
                }
            }
            _ => Err(format!("Unknown math builtin: {}", name)),
        }
    }
}
