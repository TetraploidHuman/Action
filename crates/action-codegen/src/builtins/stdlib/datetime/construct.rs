// Submodule: builtins_stdlib_datetime/construct

use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, GepCursor, InnerType, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn datetime_dispatch_construct(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
            "date" => {
                if args.len() != 3 {
                    return Err("date expects 3 arguments (year, month, day)".to_string());
                }
                let yv = self.compile_call_arg(args[0])?;
                let mv = self.compile_call_arg(args[1])?;
                let dv = self.compile_call_arg(args[2])?;
                let y = yv.to_bv().ok_or("year must be Int")?.into_int_value();
                let m = mv.to_bv().ok_or("month must be Int")?.into_int_value();
                let d = dv.to_bv().ok_or("day must be Int")?.into_int_value();
                let i64_ty = self.i64_ty();
                let zero = i64_ty.const_int(0, false);
                let one = i64_ty.const_int(1, false);
                // year >= 1
                let y_ok = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, y, one, "y_ok")
                    .map_err(llvm_err)?;
                // 1 <= month <= 12
                let m_ge1 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, m, one, "m_ge")
                    .map_err(llvm_err)?;
                let m_le12 = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, m, i64_ty.const_int(12, false), "m_le")
                    .map_err(llvm_err)?;
                let m_ok = self
                    .builder
                    .build_and(m_ge1, m_le12, "m_ok")
                    .map_err(llvm_err)?;
                // Leap year: (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
                let y_mod4 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(4, false), "ym4")
                    .map_err(llvm_err)?;
                let y_mod100 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(100, false), "ym100")
                    .map_err(llvm_err)?;
                let y_mod400 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(400, false), "ym400")
                    .map_err(llvm_err)?;
                let div4_ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, y_mod4, zero, "d4")
                    .map_err(llvm_err)?;
                let div100_ok = self
                    .builder
                    .build_int_compare(IntPredicate::NE, y_mod100, zero, "d100")
                    .map_err(llvm_err)?;
                let div400_ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, y_mod400, zero, "d400")
                    .map_err(llvm_err)?;
                let leap_part1 = self
                    .builder
                    .build_and(div4_ok, div100_ok, "lp1")
                    .map_err(llvm_err)?;
                let is_leap = self
                    .builder
                    .build_or(leap_part1, div400_ok, "is_leap")
                    .map_err(llvm_err)?;
                // feb_days = is_leap ? 29 : 28
                let feb_days = self
                    .builder
                    .build_select(
                        is_leap,
                        i64_ty.const_int(29, false),
                        i64_ty.const_int(28, false),
                        "feb",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                // max_days based on month:
                // month 2 -> feb_days
                // month 4,6,9,11 -> 30
                // month 1,3,5,7,8,10,12 -> 31
                let is_feb = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(2, false), "is_feb")
                    .map_err(llvm_err)?;
                let is_30d = {
                    let m4 = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(4, false), "m4")
                        .map_err(llvm_err)?;
                    let m6 = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(6, false), "m6")
                        .map_err(llvm_err)?;
                    let m9 = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(9, false), "m9")
                        .map_err(llvm_err)?;
                    let m11 = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(11, false), "m11")
                        .map_err(llvm_err)?;
                    let t1 = self.builder.build_or(m4, m6, "t1").map_err(llvm_err)?;
                    let t2 = self.builder.build_or(m9, m11, "t2").map_err(llvm_err)?;
                    self.builder.build_or(t1, t2, "is_30d").map_err(llvm_err)?
                };
                let max_days_30or31 = self
                    .builder
                    .build_select(
                        is_30d,
                        i64_ty.const_int(30, false),
                        i64_ty.const_int(31, false),
                        "md_30or31",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                let max_days = self
                    .builder
                    .build_select(is_feb, feb_days, max_days_30or31, "max_days")
                    .map_err(llvm_err)?
                    .into_int_value();
                let d_ge1 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, d, one, "d_ge")
                    .map_err(llvm_err)?;
                let d_le_max = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, d, max_days, "d_le")
                    .map_err(llvm_err)?;
                let d_ok = self
                    .builder
                    .build_and(d_ge1, d_le_max, "d_ok")
                    .map_err(llvm_err)?;
                let ym_ok = self
                    .builder
                    .build_and(y_ok, m_ok, "ym_ok")
                    .map_err(llvm_err)?;
                let is_valid = self
                    .builder
                    .build_and(ym_ok, d_ok, "is_valid")
                    .map_err(llvm_err)?;
                // Build Option<Date>
                let enum_ty = self
                    .context
                    .struct_type(&[i64_ty.into(), self.ptr_ty().into()], false);
                let date_sty = self
                    .type_layout
                    .named_structs
                    .get("Date")
                    .copied()
                    .unwrap_or_else(|| self.context.struct_type(&[i64_ty.into(); 3], false));
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no fn")?;
                let some_bb = self.context.append_basic_block(current_fn, "d_some");
                let none_bb = self.context.append_basic_block(current_fn, "d_none");
                let merge_bb = self.context.append_basic_block(current_fn, "d_merge");
                let _ = self
                    .builder
                    .build_conditional_branch(is_valid, some_bb, none_bb);
                self.builder.position_at_end(some_bb);
                let date_size = i64_ty.const_int(24, false);
                let malloc_fn = self.module.get_function("malloc").unwrap();
                let heap = self
                    .builder
                    .build_call(malloc_fn, &[date_size.into()], "d_heap")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
                let d_cur = GepCursor::new(heap);
                let yp = d_cur.struct_gep(&self.builder, date_sty, 0, "d_yp")?;
                self.builder.build_store(yp, y).map_err(llvm_err)?;
                let mp = d_cur.struct_gep(&self.builder, date_sty, 1, "d_mp")?;
                self.builder.build_store(mp, m).map_err(llvm_err)?;
                let dp = d_cur.struct_gep(&self.builder, date_sty, 2, "d_dp")?;
                self.builder.build_store(dp, d).map_err(llvm_err)?;
                let undef = enum_ty.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, i64_ty.const_int(0, false), 0, "r1")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, heap, 1, "r2")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(none_bb);
                let undef2 = enum_ty.get_undef();
                let r3 = self
                    .builder
                    .build_insert_value(undef2, i64_ty.const_int(1, false), 0, "r3")
                    .map_err(llvm_err)?;
                let r4 = self
                    .builder
                    .build_insert_value(r3, self.ptr_ty().const_null(), 1, "r4")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(merge_bb);
                let phi = self.builder.build_phi(enum_ty, "d_phi").map_err(llvm_err)?;
                phi.add_incoming(&[(&r2, some_bb), (&r4, none_bb)]);
                let result_alloca = self
                    .builder
                    .build_alloca(enum_ty, "d_result")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(result_alloca, phi.as_basic_value())
                    .map_err(llvm_err)?;
                Ok(Some(TypedValue::Enum(
                    result_alloca,
                    enum_ty,
                    InnerType::Int,
                    false,
                )))
            }
            "datetime" => {
                if args.len() != 6 {
                    return Err(
                        "datetime expects 6 arguments (year, month, day, hour, minute, second)"
                            .to_string(),
                    );
                }
                let yv = self.compile_call_arg(args[0])?;
                let mov = self.compile_call_arg(args[1])?;
                let dv = self.compile_call_arg(args[2])?;
                let hv = self.compile_call_arg(args[3])?;
                let minv = self.compile_call_arg(args[4])?;
                let sv = self.compile_call_arg(args[5])?;
                let y = yv.to_bv().ok_or("year must be Int")?.into_int_value();
                let mo = mov.to_bv().ok_or("month must be Int")?.into_int_value();
                let d = dv.to_bv().ok_or("day must be Int")?.into_int_value();
                let h = hv.to_bv().ok_or("hour must be Int")?.into_int_value();
                let min = minv.to_bv().ok_or("minute must be Int")?.into_int_value();
                let s = sv.to_bv().ok_or("second must be Int")?.into_int_value();
                let i64_ty = self.i64_ty();
                let zero = i64_ty.const_int(0, false);
                let one = i64_ty.const_int(1, false);
                // Validate year, month, day (same as date)
                let y_ok = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, y, one, "y_ok")
                    .map_err(llvm_err)?;
                let m_ge1 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, mo, one, "m_ge")
                    .map_err(llvm_err)?;
                let m_le12 = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, mo, i64_ty.const_int(12, false), "m_le")
                    .map_err(llvm_err)?;
                let m_ok = self
                    .builder
                    .build_and(m_ge1, m_le12, "m_ok")
                    .map_err(llvm_err)?;
                let y_mod4 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(4, false), "ym4")
                    .map_err(llvm_err)?;
                let y_mod100 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(100, false), "ym100")
                    .map_err(llvm_err)?;
                let y_mod400 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(400, false), "ym400")
                    .map_err(llvm_err)?;
                let div4_ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, y_mod4, zero, "d4")
                    .map_err(llvm_err)?;
                let div100_ok = self
                    .builder
                    .build_int_compare(IntPredicate::NE, y_mod100, zero, "d100")
                    .map_err(llvm_err)?;
                let div400_ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, y_mod400, zero, "d400")
                    .map_err(llvm_err)?;
                let leap_part1 = self
                    .builder
                    .build_and(div4_ok, div100_ok, "lp1")
                    .map_err(llvm_err)?;
                let is_leap = self
                    .builder
                    .build_or(leap_part1, div400_ok, "is_leap")
                    .map_err(llvm_err)?;
                let feb_days = self
                    .builder
                    .build_select(
                        is_leap,
                        i64_ty.const_int(29, false),
                        i64_ty.const_int(28, false),
                        "feb",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_feb = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(2, false), "is_feb")
                    .map_err(llvm_err)?;
                let m4 = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(4, false), "m4")
                    .map_err(llvm_err)?;
                let m6 = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(6, false), "m6")
                    .map_err(llvm_err)?;
                let m9 = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(9, false), "m9")
                    .map_err(llvm_err)?;
                let m11 = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(11, false), "m11")
                    .map_err(llvm_err)?;
                let t1 = self.builder.build_or(m4, m6, "t1").map_err(llvm_err)?;
                let t2 = self.builder.build_or(m9, m11, "t2").map_err(llvm_err)?;
                let is_30d = self.builder.build_or(t1, t2, "is_30d").map_err(llvm_err)?;
                let max_days_30or31 = self
                    .builder
                    .build_select(
                        is_30d,
                        i64_ty.const_int(30, false),
                        i64_ty.const_int(31, false),
                        "md_30or31",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                let max_days = self
                    .builder
                    .build_select(is_feb, feb_days, max_days_30or31, "max_days")
                    .map_err(llvm_err)?
                    .into_int_value();
                let d_ge1 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, d, one, "d_ge")
                    .map_err(llvm_err)?;
                let d_le_max = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, d, max_days, "d_le")
                    .map_err(llvm_err)?;
                let d_ok = self
                    .builder
                    .build_and(d_ge1, d_le_max, "d_ok")
                    .map_err(llvm_err)?;
                // hour 0-23, minute 0-59, second 0-59
                let h_ge0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, h, zero, "h_ge")
                    .map_err(llvm_err)?;
                let h_le23 = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, h, i64_ty.const_int(23, false), "h_le")
                    .map_err(llvm_err)?;
                let h_ok = self
                    .builder
                    .build_and(h_ge0, h_le23, "h_ok")
                    .map_err(llvm_err)?;
                let min_ge0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, min, zero, "min_ge")
                    .map_err(llvm_err)?;
                let min_le59 = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLE,
                        min,
                        i64_ty.const_int(59, false),
                        "min_le",
                    )
                    .map_err(llvm_err)?;
                let min_ok = self
                    .builder
                    .build_and(min_ge0, min_le59, "min_ok")
                    .map_err(llvm_err)?;
                let s_ge0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, s, zero, "s_ge")
                    .map_err(llvm_err)?;
                let s_le59 = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, s, i64_ty.const_int(59, false), "s_le")
                    .map_err(llvm_err)?;
                let s_ok = self
                    .builder
                    .build_and(s_ge0, s_le59, "s_ok")
                    .map_err(llvm_err)?;
                let ym_ok = self
                    .builder
                    .build_and(y_ok, m_ok, "ym_ok")
                    .map_err(llvm_err)?;
                let ymd_ok = self
                    .builder
                    .build_and(ym_ok, d_ok, "ymd_ok")
                    .map_err(llvm_err)?;
                let hms_ok = self
                    .builder
                    .build_and(
                        self.builder
                            .build_and(h_ok, min_ok, "hm_ok")
                            .map_err(llvm_err)?,
                        s_ok,
                        "hms_ok",
                    )
                    .map_err(llvm_err)?;
                let is_valid = self
                    .builder
                    .build_and(ymd_ok, hms_ok, "is_valid")
                    .map_err(llvm_err)?;
                // Build Option<DateTime>
                let enum_ty = self
                    .context
                    .struct_type(&[i64_ty.into(), self.ptr_ty().into()], false);
                let dt_sty = self
                    .type_layout
                    .named_structs
                    .get("DateTime")
                    .copied()
                    .unwrap_or_else(|| self.context.struct_type(&[i64_ty.into(); 6], false));
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no fn")?;
                let some_bb = self.context.append_basic_block(current_fn, "dt_some");
                let none_bb = self.context.append_basic_block(current_fn, "dt_none");
                let merge_bb = self.context.append_basic_block(current_fn, "dt_merge");
                let _ = self
                    .builder
                    .build_conditional_branch(is_valid, some_bb, none_bb);
                self.builder.position_at_end(some_bb);
                let dt_size = i64_ty.const_int(48, false); // 6 * 8 bytes
                let malloc_fn = self.module.get_function("malloc").unwrap();
                let heap = self
                    .builder
                    .build_call(malloc_fn, &[dt_size.into()], "dt_heap")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
                for (i, val) in [y, mo, d, h, min, s].iter().enumerate() {
                    let fp = self
                        .builder
                        .build_struct_gep(dt_sty, heap, i as u32, "dt_f")
                        .map_err(llvm_err)?;
                    self.builder.build_store(fp, *val).map_err(llvm_err)?;
                }
                let undef = enum_ty.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, i64_ty.const_int(0, false), 0, "r1")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, heap, 1, "r2")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(none_bb);
                let undef2 = enum_ty.get_undef();
                let r3 = self
                    .builder
                    .build_insert_value(undef2, i64_ty.const_int(1, false), 0, "r3")
                    .map_err(llvm_err)?;
                let r4 = self
                    .builder
                    .build_insert_value(r3, self.ptr_ty().const_null(), 1, "r4")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(enum_ty, "dt_phi")
                    .map_err(llvm_err)?;
                phi.add_incoming(&[(&r2, some_bb), (&r4, none_bb)]);
                let result_alloca = self
                    .builder
                    .build_alloca(enum_ty, "dt_result")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(result_alloca, phi.as_basic_value())
                    .map_err(llvm_err)?;
                Ok(Some(TypedValue::Enum(
                    result_alloca,
                    enum_ty,
                    InnerType::Int,
                    false,
                )))
            }
            _ => Ok(None),
        }
    }
}
