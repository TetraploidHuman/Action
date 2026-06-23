// Submodule: builtins_stdlib_datetime/accessors

use inkwell::values::{IntValue, PointerValue};
use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn datetime_dispatch_accessors(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
            "today" => {
                if !args.is_empty() {
                    return Err("today expects no arguments".to_string());
                }
                // Call C time() and localtime_r() to get real current date
                Ok(Some(self.emit_today_now(false)?))
            }
            "now" => {
                if !args.is_empty() {
                    return Err("now expects no arguments".to_string());
                }
                Ok(Some(self.emit_today_now(true)?))
            }
            // DateTime/Date field accessors
            "year" | "month" | "day" | "hour" | "minute" | "second" => {
                if args.len() != 1 {
                    return Err(format!("{} expects 1 argument", name));
                }
                let v = self.compile_call_arg(args[0])?;
                match v {
                    TypedValue::Struct(p, st) => {
                        let field_idx = match name {
                            "year" => 0,
                            "month" => 1,
                            "day" => 2,
                            "hour" => 3,
                            "minute" => 4,
                            "second" => 5,
                            _ => return Err("bad field".to_string()),
                        };
                        let fptr = self
                            .builder
                            .build_struct_gep(st, p, field_idx, "fptr")
                            .map_err(llvm_err)?;
                        let val = self
                            .builder
                            .build_load(self.i64_ty(), fptr, "val")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(Some(TypedValue::Int(val)))
                    }
                    _ => Err(format!(
                        "{}: argument must be a Date or DateTime struct",
                        name
                    )),
                }
            }
            "addDays" => {
                if args.len() != 2 {
                    return Err("addDays expects 2 arguments (date, days)".to_string());
                }
                let d = self.compile_call_arg(args[0])?;
                let days = self.compile_call_arg(args[1])?;
                let days_bv = days.to_bv().ok_or("days must be Int")?;
                match d {
                    TypedValue::Struct(p, st) => {
                        // Create a new Date struct with added days
                        let alloca = self
                            .builder
                            .build_alloca(st, "new_date")
                            .map_err(llvm_err)?;
                        for i in 0..3u32 {
                            let fptr = self
                                .builder
                                .build_struct_gep(st, p, i, "fptr")
                                .map_err(llvm_err)?;
                            let fval = self
                                .builder
                                .build_load(self.i64_ty(), fptr, "fval")
                                .map_err(llvm_err)?
                                .into_int_value();
                            let new_val = if i == 2 {
                                self.builder
                                    .build_int_add(fval, days_bv.into_int_value(), "new_day")
                                    .map_err(llvm_err)?
                                    .into()
                            } else {
                                fval
                            };
                            let dfptr = self
                                .builder
                                .build_struct_gep(st, alloca, i, "dfptr")
                                .map_err(llvm_err)?;
                            self.builder.build_store(dfptr, new_val).map_err(llvm_err)?;
                        }
                        Ok(Some(TypedValue::Struct(alloca, st)))
                    }
                    _ => Err("addDays: first argument must be a Date struct".to_string()),
                }
            }
            "addHours" => {
                if args.len() != 2 {
                    return Err("addHours expects 2 arguments (datetime, hours)".to_string());
                }
                let d = self.compile_call_arg(args[0])?;
                let hours = self.compile_call_arg(args[1])?;
                let hours_bv = hours.to_bv().ok_or("hours must be Int")?;
                match d {
                    TypedValue::Struct(p, st) => {
                        let alloca = self.builder.build_alloca(st, "new_dt").map_err(llvm_err)?;
                        for i in 0..6u32 {
                            let fptr = self
                                .builder
                                .build_struct_gep(st, p, i, "fptr")
                                .map_err(llvm_err)?;
                            let fval = self
                                .builder
                                .build_load(self.i64_ty(), fptr, "fval")
                                .map_err(llvm_err)?
                                .into_int_value();
                            let new_val = if i == 3 {
                                self.builder
                                    .build_int_add(fval, hours_bv.into_int_value(), "new_hour")
                                    .map_err(llvm_err)?
                                    .into()
                            } else {
                                fval
                            };
                            let dfptr = self
                                .builder
                                .build_struct_gep(st, alloca, i, "dfptr")
                                .map_err(llvm_err)?;
                            self.builder.build_store(dfptr, new_val).map_err(llvm_err)?;
                        }
                        Ok(Some(TypedValue::Struct(alloca, st)))
                    }
                    _ => Err("addHours: first argument must be a DateTime struct".to_string()),
                }
            }
            "diffDays" => {
                if args.len() != 2 {
                    return Err("diffDays expects 2 arguments (date1, date2)".to_string());
                }
                let d1 = self.compile_call_arg(args[0])?;
                let d2 = self.compile_call_arg(args[1])?;
                let (p1, st1) = match d1 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffDays: arguments must be Date structs".to_string()),
                };
                let (p2, st2) = match d2 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffDays: arguments must be Date structs".to_string()),
                };
                let i64_ty = self.i64_ty();
                // Julian Day Number: JDN = D + (153*m+2)/5 + 365*y + y/4 - y/100 + y/400 - 32045
                // where a = (14-M)/12, y = Y+4800-a, m = M+12*a-3
                let jdn = |yp: PointerValue<'ctx>,
                           sty: inkwell::types::StructType<'ctx>|
                 -> Result<IntValue<'ctx>, String> {
                    let y_ptr = self
                        .builder
                        .build_struct_gep(sty, yp, 0, "j_y")
                        .map_err(llvm_err)?;
                    let y_val = self
                        .builder
                        .build_load(i64_ty, y_ptr, "j_yv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let m_ptr = self
                        .builder
                        .build_struct_gep(sty, yp, 1, "j_m")
                        .map_err(llvm_err)?;
                    let m_val = self
                        .builder
                        .build_load(i64_ty, m_ptr, "j_mv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let d_ptr = self
                        .builder
                        .build_struct_gep(sty, yp, 2, "j_d")
                        .map_err(llvm_err)?;
                    let d_val = self
                        .builder
                        .build_load(i64_ty, d_ptr, "j_dv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let c12 = i64_ty.const_int(12, false);
                    let c14 = i64_ty.const_int(14, false);
                    let c4800 = i64_ty.const_int(4800, false);
                    let c3 = i64_ty.const_int(3, false);
                    let c4 = i64_ty.const_int(4, false);
                    let c100 = i64_ty.const_int(100, false);
                    let c400 = i64_ty.const_int(400, false);
                    let c153 = i64_ty.const_int(153, false);
                    let c2 = i64_ty.const_int(2, false);
                    let c5 = i64_ty.const_int(5, false);
                    let c365 = i64_ty.const_int(365, false);
                    let c32045 = i64_ty.const_int(32045, false);
                    // a = (14 - M) / 12
                    let a = self
                        .builder
                        .build_int_signed_div(
                            self.builder
                                .build_int_sub(c14, m_val, "t_a1")
                                .map_err(llvm_err)?,
                            c12,
                            "a",
                        )
                        .map_err(llvm_err)?;
                    // y = Y + 4800 - a
                    let y = self
                        .builder
                        .build_int_sub(
                            self.builder
                                .build_int_add(y_val, c4800, "t_y1")
                                .map_err(llvm_err)?,
                            a,
                            "y",
                        )
                        .map_err(llvm_err)?;
                    // m = M + 12*a - 3
                    let m = self
                        .builder
                        .build_int_sub(
                            self.builder
                                .build_int_add(
                                    m_val,
                                    self.builder
                                        .build_int_mul(c12, a, "t_m1")
                                        .map_err(llvm_err)?,
                                    "t_m2",
                                )
                                .map_err(llvm_err)?,
                            c3,
                            "m",
                        )
                        .map_err(llvm_err)?;
                    // term1 = (153*m + 2) / 5
                    let term1 = self
                        .builder
                        .build_int_signed_div(
                            self.builder
                                .build_int_add(
                                    self.builder
                                        .build_int_mul(c153, m, "t_t1a")
                                        .map_err(llvm_err)?,
                                    c2,
                                    "t_t1b",
                                )
                                .map_err(llvm_err)?,
                            c5,
                            "term1",
                        )
                        .map_err(llvm_err)?;
                    // term2 = 365*y
                    let term2 = self
                        .builder
                        .build_int_mul(c365, y, "term2")
                        .map_err(llvm_err)?;
                    // term3 = y/4
                    let term3 = self
                        .builder
                        .build_int_signed_div(y, c4, "term3")
                        .map_err(llvm_err)?;
                    // term4 = y/100
                    let term4 = self
                        .builder
                        .build_int_signed_div(y, c100, "term4")
                        .map_err(llvm_err)?;
                    // term5 = y/400
                    let term5 = self
                        .builder
                        .build_int_signed_div(y, c400, "term5")
                        .map_err(llvm_err)?;
                    // JDN = D + term1 + term2 + term3 - term4 + term5 - 32045
                    let s1 = self
                        .builder
                        .build_int_add(d_val, term1, "s1")
                        .map_err(llvm_err)?;
                    let s2 = self
                        .builder
                        .build_int_add(s1, term2, "s2")
                        .map_err(llvm_err)?;
                    let s3 = self
                        .builder
                        .build_int_add(s2, term3, "s3")
                        .map_err(llvm_err)?;
                    let s4 = self
                        .builder
                        .build_int_sub(s3, term4, "s4")
                        .map_err(llvm_err)?;
                    let s5 = self
                        .builder
                        .build_int_add(s4, term5, "s5")
                        .map_err(llvm_err)?;
                    let jdn_val = self
                        .builder
                        .build_int_sub(s5, c32045, "jdn")
                        .map_err(llvm_err)?;
                    Ok(jdn_val)
                };
                let j1 = jdn(p1, st1)?;
                let j2 = jdn(p2, st2)?;
                let diff = self
                    .builder
                    .build_int_sub(j1, j2, "diff")
                    .map_err(llvm_err)?;
                let zero = i64_ty.const_int(0, false);
                let nd = self.builder.build_int_neg(diff, "nd").map_err(llvm_err)?;
                let is_neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, diff, zero, "is_neg")
                    .map_err(llvm_err)?;
                let abs_diff = self
                    .builder
                    .build_select(is_neg, nd, diff, "abs_diff")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(Some(TypedValue::Int(abs_diff)))
            }
            _ => Ok(None),
        }
    }
}
