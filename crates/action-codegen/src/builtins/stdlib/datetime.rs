// Submodule: builtins_stdlib_datetime — datetime/date/format builtin functions
//
// Extracted from builtins_stdlib.rs.
//
// Submodule: builtins_stdlib

use inkwell::values::{IntValue, PointerValue};
use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, GepCursor, InnerType, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_stdlib_datetime(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "format" => {
                if args.len() != 2 {
                    return Err("format expects 2 arguments (datetime, format_str)".to_string());
                }
                let dt = self.compile_call_arg(args[0])?;
                let fmt = self.compile_call_arg(args[1])?;
                match (&dt, &fmt) {
                    (TypedValue::Struct(dt_ptr, dt_st), TypedValue::Str(fmt_ptr)) => {
                        let fmt_val = self.load_string(*fmt_ptr)?;
                        let fmt_data = self
                            .builder
                            .build_extract_value(fmt_val, 1, "fmt_data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        // Extract DateTime fields: {year, month, day, hour, minute, second}
                        let cur = GepCursor::new(*dt_ptr);
                        let fptr0 = cur.struct_gep(&self.builder, *dt_st, 0, "dt_f")?;
                        let year = self
                            .builder
                            .build_load(self.i64_ty(), fptr0, "dt_v")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let fptr1 = cur.struct_gep(&self.builder, *dt_st, 1, "dt_f")?;
                        let month = self
                            .builder
                            .build_load(self.i64_ty(), fptr1, "dt_v")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let fptr2 = cur.struct_gep(&self.builder, *dt_st, 2, "dt_f")?;
                        let day = self
                            .builder
                            .build_load(self.i64_ty(), fptr2, "dt_v")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let fptr3 = cur.struct_gep(&self.builder, *dt_st, 3, "dt_f")?;
                        let hour = self
                            .builder
                            .build_load(self.i64_ty(), fptr3, "dt_v")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let fptr4 = cur.struct_gep(&self.builder, *dt_st, 4, "dt_f")?;
                        let minute = self
                            .builder
                            .build_load(self.i64_ty(), fptr4, "dt_v")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let fptr5 = cur.struct_gep(&self.builder, *dt_st, 5, "dt_f")?;
                        let second = self
                            .builder
                            .build_load(self.i64_ty(), fptr5, "dt_v")
                            .map_err(llvm_err)?
                            .into_int_value();
                        // Build struct tm: {i32 x 9}
                        let i32 = self.context.i32_type();
                        let tm_ty = self.context.struct_type(&[i32.into(); 9], false);
                        let tm_a = self.builder.build_alloca(tm_ty, "tm").map_err(llvm_err)?;
                        let tm_cur = GepCursor::new(tm_a);
                        // tm_sec = second
                        let tm_sec = self
                            .builder
                            .build_int_truncate(second, i32, "tm_sec")
                            .map_err(llvm_err)?;
                        let f0 = tm_cur.struct_gep(&self.builder, tm_ty, 0, "f0")?;
                        self.builder.build_store(f0, tm_sec).map_err(llvm_err)?;
                        // tm_min = minute
                        let tm_min = self
                            .builder
                            .build_int_truncate(minute, i32, "tm_min")
                            .map_err(llvm_err)?;
                        let f1 = tm_cur.struct_gep(&self.builder, tm_ty, 1, "f1")?;
                        self.builder.build_store(f1, tm_min).map_err(llvm_err)?;
                        // tm_hour = hour
                        let tm_hour = self
                            .builder
                            .build_int_truncate(hour, i32, "tm_hour")
                            .map_err(llvm_err)?;
                        let f2 = tm_cur.struct_gep(&self.builder, tm_ty, 2, "f2")?;
                        self.builder.build_store(f2, tm_hour).map_err(llvm_err)?;
                        // tm_mday = day
                        let tm_mday = self
                            .builder
                            .build_int_truncate(day, i32, "tm_mday")
                            .map_err(llvm_err)?;
                        let f3 = tm_cur.struct_gep(&self.builder, tm_ty, 3, "f3")?;
                        self.builder.build_store(f3, tm_mday).map_err(llvm_err)?;
                        // tm_mon = month - 1
                        let mon_minus = self
                            .builder
                            .build_int_sub(month, self.i64_ty().const_int(1, false), "mon_minus")
                            .map_err(llvm_err)?;
                        let tm_mon = self
                            .builder
                            .build_int_truncate(mon_minus, i32, "tm_mon")
                            .map_err(llvm_err)?;
                        let f4 = tm_cur.struct_gep(&self.builder, tm_ty, 4, "f4")?;
                        self.builder.build_store(f4, tm_mon).map_err(llvm_err)?;
                        // tm_year = year - 1900
                        let year_minus = self
                            .builder
                            .build_int_sub(year, self.i64_ty().const_int(1900, false), "year_minus")
                            .map_err(llvm_err)?;
                        let tm_year = self
                            .builder
                            .build_int_truncate(year_minus, i32, "tm_year")
                            .map_err(llvm_err)?;
                        let f5 = tm_cur.struct_gep(&self.builder, tm_ty, 5, "f5")?;
                        self.builder.build_store(f5, tm_year).map_err(llvm_err)?;
                        // tm_wday = 0
                        let f6 = tm_cur.struct_gep(&self.builder, tm_ty, 6, "f6")?;
                        self.builder
                            .build_store(f6, i32.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_yday = 0
                        let f7 = tm_cur.struct_gep(&self.builder, tm_ty, 7, "f7")?;
                        self.builder
                            .build_store(f7, i32.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_isdst = -1
                        let f8 = tm_cur.struct_gep(&self.builder, tm_ty, 8, "f8")?;
                        self.builder
                            .build_store(f8, i32.const_int(0xFFFFFFFFu64 as u64, false))
                            .map_err(llvm_err)?;
                        // Allocate buffer and call strftime
                        let buf_size = self.i64_ty().const_int(256, false);
                        let malloc_fn = self.module.get_function("malloc").unwrap();
                        let buf = self
                            .builder
                            .build_call(malloc_fn, &[buf_size.into()], "fmt_buf")
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_pointer_value();
                        let strftime_fn = self
                            .module
                            .get_function("strftime")
                            .ok_or("strftime not found")?;
                        let _ = self
                            .builder
                            .build_call(
                                strftime_fn,
                                &[buf.into(), buf_size.into(), fmt_data.into(), tm_a.into()],
                                "",
                            )
                            .map_err(llvm_err)?;
                        // Build Atomic string: {i64, i8*} with strlen
                        let strlen_fn = self
                            .module
                            .get_function("strlen")
                            .ok_or("strlen not found")?;
                        let len = self
                            .builder
                            .build_call(strlen_fn, &[buf.into()], "fmt_len")
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_int_value();
                        let fat = self.string_type.get_undef();
                        let r1 = self
                            .builder
                            .build_insert_value(fat, len, 0, "r1")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_insert_value(r1, buf, 1, "r2")
                            .map_err(llvm_err)?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "fmt_str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, r2).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("format: expects (DateTime, String)".to_string()),
                }
            }
            "parseDate" => {
                if args.len() != 2 {
                    return Err("parseDate expects 2 arguments (format_str, date_str)".to_string());
                }
                let fmt_v = self.compile_call_arg(args[0])?;
                let date_v = self.compile_call_arg(args[1])?;
                match (&fmt_v, &date_v) {
                    (TypedValue::Str(_fmt_ptr), TypedValue::Str(date_ptr)) => {
                        let date_val = self.load_string(*date_ptr)?;
                        let date_data = self
                            .builder
                            .build_extract_value(date_val, 1, "pd_date")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        // Use sscanf to parse the date string with format "%d-%d-%d"
                        let i32_ty = self.context.i32_type();
                        let sscanf_ty = self
                            .i32_ty()
                            .fn_type(&[self.ptr_ty().into(), self.ptr_ty().into()], true);
                        let sscanf_fn = self
                            .module
                            .get_function("sscanf")
                            .unwrap_or_else(|| self.module.add_function("sscanf", sscanf_ty, None));
                        // Stack-allocate year, month, day as i32
                        let y_ptr = self
                            .builder
                            .build_alloca(i32_ty, "pd_y")
                            .map_err(llvm_err)?;
                        let m_ptr = self
                            .builder
                            .build_alloca(i32_ty, "pd_m")
                            .map_err(llvm_err)?;
                        let d_ptr = self
                            .builder
                            .build_alloca(i32_ty, "pd_d")
                            .map_err(llvm_err)?;
                        let fmt_str = self
                            .builder
                            .build_global_string_ptr("%d-%d-%d", "pd_fmt")
                            .map_err(llvm_err)?;
                        let ret = self
                            .builder
                            .build_call(
                                sscanf_fn,
                                &[
                                    date_data.into(),
                                    fmt_str.as_pointer_value().into(),
                                    y_ptr.into(),
                                    m_ptr.into(),
                                    d_ptr.into(),
                                ],
                                "pd_ret",
                            )
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_int_value();
                        let ok = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                ret,
                                i32_ty.const_int(3, false),
                                "pd_ok",
                            )
                            .map_err(llvm_err)?;
                        // Build Option<Date>
                        let enum_ty = self
                            .context
                            .struct_type(&[self.i64_ty().into(), self.ptr_ty().into()], false);
                        let some_sty = self
                            .type_layout
                            .named_structs
                            .get("Date")
                            .copied()
                            .unwrap_or_else(|| {
                                self.context.struct_type(&[self.i64_ty().into(); 3], false)
                            });
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "pd_some");
                        let none_bb = self.context.append_basic_block(current_fn, "pd_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "pd_merge");
                        let _ = self.builder.build_conditional_branch(ok, some_bb, none_bb);
                        // Some branch
                        self.builder.position_at_end(some_bb);
                        let y_val = self
                            .builder
                            .build_load(i32_ty, y_ptr, "pd_yv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let m_val = self
                            .builder
                            .build_load(i32_ty, m_ptr, "pd_mv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let d_val = self
                            .builder
                            .build_load(i32_ty, d_ptr, "pd_dv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let year_i64 = self
                            .builder
                            .build_int_s_extend(y_val, self.i64_ty(), "py")
                            .map_err(llvm_err)?;
                        let month_i64 = self
                            .builder
                            .build_int_s_extend(m_val, self.i64_ty(), "pm")
                            .map_err(llvm_err)?;
                        let day_i64 = self
                            .builder
                            .build_int_s_extend(d_val, self.i64_ty(), "pd")
                            .map_err(llvm_err)?;
                        let date_size = self.i64_ty().const_int(24, false);
                        let malloc_fn = self.module.get_function("malloc").unwrap();
                        let heap = self
                            .builder
                            .build_call(malloc_fn, &[date_size.into()], "pd_heap")
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_pointer_value();
                        let dp = self
                            .builder
                            .build_pointer_cast(heap, self.ptr_ty(), "dp")
                            .map_err(llvm_err)?;
                        let pd_cur = GepCursor::new(dp);
                        let yp = pd_cur.struct_gep(&self.builder, some_sty, 0, "yp")?;
                        self.builder.build_store(yp, year_i64).map_err(llvm_err)?;
                        let mp = pd_cur.struct_gep(&self.builder, some_sty, 1, "mp")?;
                        self.builder.build_store(mp, month_i64).map_err(llvm_err)?;
                        let dap = pd_cur.struct_gep(&self.builder, some_sty, 2, "dap")?;
                        self.builder.build_store(dap, day_i64).map_err(llvm_err)?;
                        let undef = enum_ty.get_undef();
                        let r1 = self
                            .builder
                            .build_insert_value(undef, self.i64_ty().const_int(0, false), 0, "r1")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_insert_value(r1, heap, 1, "r2")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None branch
                        self.builder.position_at_end(none_bb);
                        let undef2 = enum_ty.get_undef();
                        let r3 = self
                            .builder
                            .build_insert_value(undef2, self.i64_ty().const_int(1, false), 0, "r3")
                            .map_err(llvm_err)?;
                        let r4 = self
                            .builder
                            .build_insert_value(r3, self.ptr_ty().const_null(), 1, "r4")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge with phi
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(enum_ty, "pd_phi")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&r2, some_bb), (&r4, none_bb)]);
                        let result_alloca = self
                            .builder
                            .build_alloca(enum_ty, "pd_result")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(result_alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Enum(
                            result_alloca,
                            enum_ty,
                            InnerType::Int,
                            false,
                        ))
                    }
                    _ => Err("parseDate: expects (String, String)".to_string()),
                }
            }
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
                Ok(TypedValue::Enum(
                    result_alloca,
                    enum_ty,
                    InnerType::Int,
                    false,
                ))
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
                Ok(TypedValue::Enum(
                    result_alloca,
                    enum_ty,
                    InnerType::Int,
                    false,
                ))
            }
            "Random_new" => {
                if args.len() != 1 {
                    return Err("Random_new expects 1 argument (seed)".to_string());
                }
                let seed_v = self.compile_call_arg(args[0])?;
                let seed = seed_v.to_bv().ok_or("seed must be Int")?.into_int_value();
                // Random struct is just {i64} wrapping the seed
                let rand_sty = self.context.struct_type(&[self.i64_ty().into()], false);
                let alloca = self
                    .builder
                    .build_alloca(rand_sty, "rand")
                    .map_err(llvm_err)?;
                let f0 = self
                    .builder
                    .build_struct_gep(rand_sty, alloca, 0, "f0")
                    .map_err(llvm_err)?;
                self.builder.build_store(f0, seed).map_err(llvm_err)?;
                Ok(TypedValue::Struct(alloca, rand_sty))
            }
            "nextInt" => {
                if args.len() != 3 {
                    return Err("nextInt expects 3 arguments (random, min, max)".to_string());
                }
                let rng_v = self.compile_call_arg(args[0])?;
                let min_v = self.compile_call_arg(args[1])?;
                let max_v = self.compile_call_arg(args[2])?;
                let (rng_ptr, rng_st) = match rng_v {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("nextInt: first argument must be a Random struct".to_string()),
                };
                let min = min_v.to_bv().ok_or("min must be Int")?.into_int_value();
                let max = max_v.to_bv().ok_or("max must be Int")?.into_int_value();
                let i64_ty = self.i64_ty();
                // Load current seed
                let f0 = self
                    .builder
                    .build_struct_gep(rng_st, rng_ptr, 0, "f0")
                    .map_err(llvm_err)?;
                let seed = self
                    .builder
                    .build_load(i64_ty, f0, "seed")
                    .map_err(llvm_err)?
                    .into_int_value();
                // xorshift64 PRNG
                // x ^= x << 13; x ^= x >> 7; x ^= x << 17
                let c13 = i64_ty.const_int(13, false);
                let c7 = i64_ty.const_int(7, false);
                let c17 = i64_ty.const_int(17, false);
                let x1 = self
                    .builder
                    .build_xor(
                        seed,
                        self.builder
                            .build_left_shift(seed, c13, "s1")
                            .map_err(llvm_err)?,
                        "x1",
                    )
                    .map_err(llvm_err)?;
                let x2 = self
                    .builder
                    .build_xor(
                        x1,
                        self.builder
                            .build_right_shift(x1, c7, false, "s2")
                            .map_err(llvm_err)?,
                        "x2",
                    )
                    .map_err(llvm_err)?;
                let x3 = self
                    .builder
                    .build_xor(
                        x2,
                        self.builder
                            .build_left_shift(x2, c17, "s3")
                            .map_err(llvm_err)?,
                        "x3",
                    )
                    .map_err(llvm_err)?;
                // Ensure non-zero (degenerates to 0 otherwise)
                let zero = i64_ty.const_int(0, false);
                let is_zero = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, x3, zero, "is_zero")
                    .map_err(llvm_err)?;
                let new_seed = self
                    .builder
                    .build_select(is_zero, i64_ty.const_int(1, false), x3, "new_seed")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Compute value in [min, max] range
                let range = self
                    .builder
                    .build_int_sub(max, min, "range")
                    .map_err(llvm_err)?;
                let range_plus_1 = self
                    .builder
                    .build_int_add(range, i64_ty.const_int(1, false), "rp1")
                    .map_err(llvm_err)?;
                // Use unsigned remainder for proper range mapping
                let value = self
                    .builder
                    .build_int_unsigned_rem(new_seed, range_plus_1, "val_mod")
                    .map_err(llvm_err)?;
                let result = self
                    .builder
                    .build_int_add(value, min, "result")
                    .map_err(llvm_err)?;
                // Build result tuple (Random, Int)
                let rand_sty = rng_st;
                let tuple_sty = self
                    .context
                    .struct_type(&[rand_sty.into(), i64_ty.into()], false);
                let tup_alloca = self
                    .builder
                    .build_alloca(tuple_sty, "tup")
                    .map_err(llvm_err)?;
                // Store new Random
                let rng_field = self
                    .builder
                    .build_struct_gep(tuple_sty, tup_alloca, 0, "rf")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(rng_field, new_seed)
                    .map_err(llvm_err)?;
                // Store int result
                let int_field = self
                    .builder
                    .build_struct_gep(tuple_sty, tup_alloca, 1, "inf")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(int_field, result)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Struct(tup_alloca, tuple_sty))
            }
            "randInt" => {
                if args.len() != 2 {
                    return Err("randInt expects 2 arguments (min, max)".to_string());
                }
                let min = self.compile_call_arg(args[0])?;
                let max = self.compile_call_arg(args[1])?;
                let min_bv = min.to_bv().ok_or("min must be a basic value")?;
                let max_bv = max.to_bv().ok_or("max must be a basic value")?;
                let cc = self.call_rt("action_rand_int", &[min_bv.into(), max_bv.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("randInt failed")?
                    .into_int_value();
                Ok(TypedValue::Int(result))
            }
            "randFloat" => {
                if !args.is_empty() {
                    return Err("randFloat expects no arguments".to_string());
                }
                let cc = self.call_rt("action_rand_float", &[])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("randFloat failed")?
                    .into_float_value();
                Ok(TypedValue::Float(result))
            }
            "today" => {
                if !args.is_empty() {
                    return Err("today expects no arguments".to_string());
                }
                // Call C time() and localtime_r() to get real current date
                self.emit_today_now(false)
            }
            "now" => {
                if !args.is_empty() {
                    return Err("now expects no arguments".to_string());
                }
                self.emit_today_now(true)
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
                        Ok(TypedValue::Int(val))
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
                        Ok(TypedValue::Struct(alloca, st))
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
                        Ok(TypedValue::Struct(alloca, st))
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
                Ok(TypedValue::Int(abs_diff))
            }
            "weekday" => {
                if args.len() != 1 {
                    return Err("weekday expects 1 argument (date)".to_string());
                }
                let d = self.compile_call_arg(args[0])?;
                match d {
                    TypedValue::Struct(p, st) => {
                        // Use mktime to compute proper weekday
                        // Build struct tm: {i32 x 9}
                        let i32_ty = self.context.i32_type();
                        let tm_ty = self.context.struct_type(&[i32_ty.into(); 9], false);
                        let tm_a = self.builder.build_alloca(tm_ty, "tm").map_err(llvm_err)?;
                        let i64_ty = self.i64_ty();
                        // Extract year, month, day from Date struct
                        let yp = self
                            .builder
                            .build_struct_gep(st, p, 0, "w_yp")
                            .map_err(llvm_err)?;
                        let yv = self
                            .builder
                            .build_load(i64_ty, yp, "w_yv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let mp = self
                            .builder
                            .build_struct_gep(st, p, 1, "w_mp")
                            .map_err(llvm_err)?;
                        let mv = self
                            .builder
                            .build_load(i64_ty, mp, "w_mv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let dp = self
                            .builder
                            .build_struct_gep(st, p, 2, "w_dp")
                            .map_err(llvm_err)?;
                        let dv = self
                            .builder
                            .build_load(i64_ty, dp, "w_dv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        // tm_sec = 0
                        let f0 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 0, "f0")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f0, i32_ty.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_min = 0
                        let f1 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 1, "f1")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f1, i32_ty.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_hour = 12 (noon, avoid DST issues)
                        let f2 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 2, "f2")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f2, i32_ty.const_int(12, false))
                            .map_err(llvm_err)?;
                        // tm_mday = day
                        let f3 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 3, "f3")
                            .map_err(llvm_err)?;
                        let dv32 = self
                            .builder
                            .build_int_truncate(dv, i32_ty, "dv32")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f3, dv32).map_err(llvm_err)?;
                        // tm_mon = month - 1
                        let f4 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 4, "f4")
                            .map_err(llvm_err)?;
                        let mon_minus = self
                            .builder
                            .build_int_sub(mv, i64_ty.const_int(1, false), "mon_minus")
                            .map_err(llvm_err)?;
                        let mon32 = self
                            .builder
                            .build_int_truncate(mon_minus, i32_ty, "mon32")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f4, mon32).map_err(llvm_err)?;
                        // tm_year = year - 1900
                        let f5 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 5, "f5")
                            .map_err(llvm_err)?;
                        let y_minus = self
                            .builder
                            .build_int_sub(yv, i64_ty.const_int(1900, false), "y_minus")
                            .map_err(llvm_err)?;
                        let y32 = self
                            .builder
                            .build_int_truncate(y_minus, i32_ty, "y32")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f5, y32).map_err(llvm_err)?;
                        // Remaining fields init to 0
                        for i in 6..9u32 {
                            let f = self
                                .builder
                                .build_struct_gep(tm_ty, tm_a, i, "f")
                                .map_err(llvm_err)?;
                            self.builder
                                .build_store(f, i32_ty.const_int(0, false))
                                .map_err(llvm_err)?;
                        }
                        // Call mktime
                        let mktime_fn = self.module.get_function("mktime").unwrap_or_else(|| {
                            self.module.add_function(
                                "mktime",
                                self.i64_ty().fn_type(&[self.ptr_ty().into()], false),
                                None,
                            )
                        });
                        let _ = self
                            .builder
                            .build_call(mktime_fn, &[tm_a.into()], "")
                            .map_err(llvm_err)?;
                        // Read tm_wday (field 6)
                        let wf = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 6, "wf")
                            .map_err(llvm_err)?;
                        let wday32 = self
                            .builder
                            .build_load(i32_ty, wf, "wday")
                            .map_err(llvm_err)?
                            .into_int_value();
                        // Convert: C wday 0=Sunday -> Atomic 1=Monday..7=Sunday
                        // Atomic weekday: 1=Monday, 7=Sunday
                        // C: 0=Sun,1=Mon,2=Tue,3=Wed,4=Thu,5=Fri,6=Sat
                        // Map: C=0->7, C=1->1, C=2->2, C=3->3, C=4->4, C=5->5, C=6->6
                        let wd_c0 = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                wday32,
                                i32_ty.const_int(0, false),
                                "wd_c0",
                            )
                            .map_err(llvm_err)?;
                        let wd32 = self
                            .builder
                            .build_select(wd_c0, i32_ty.const_int(7, false), wday32, "wd")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let wd = self
                            .builder
                            .build_int_s_extend(wd32, i64_ty, "wd64")
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Int(wd))
                    }
                    _ => Err("weekday: argument must be a Date struct".to_string()),
                }
            }
            "nowUtc" => {
                if !args.is_empty() {
                    return Err("nowUtc expects no arguments".to_string());
                }
                let sty = self.context.struct_type(&[self.i64_ty().into(); 6], false);
                let alloca = self.builder.build_alloca(sty, "nowUtc").map_err(llvm_err)?;
                let time_fn = self
                    .module
                    .get_function("time")
                    .ok_or("time function not found")?;
                let null_ptr = self.ptr_ty().const_null();
                let ts = self
                    .builder
                    .build_call(time_fn, &[null_ptr.into()], "ts")
                    .map_err(llvm_err)?;
                let ts_val = ts.try_as_basic_value().unwrap_basic().into_int_value();
                let gmtime_fn = self
                    .module
                    .get_function("gmtime_r")
                    .ok_or("gmtime_r function not found")?;
                let tm_ptr = self.builder.build_alloca(sty, "tm").map_err(llvm_err)?;
                let gmtime_call = self
                    .builder
                    .build_call(gmtime_fn, &[ts_val.into(), tm_ptr.into()], "")
                    .map_err(llvm_err)?;
                let _ = gmtime_call.try_as_basic_value().basic();
                // Copy tm struct to result (year+1900, month, day, hour, min, sec)
                for i in 0..6u32 {
                    let src_p = self
                        .builder
                        .build_struct_gep(sty, tm_ptr, i, "tm_f")
                        .map_err(llvm_err)?;
                    let val = self
                        .builder
                        .build_load(self.i64_ty(), src_p, "val")
                        .map_err(llvm_err)?;
                    let dst_p = self
                        .builder
                        .build_struct_gep(sty, alloca, i, "dst_f")
                        .map_err(llvm_err)?;
                    self.builder.build_store(dst_p, val).map_err(llvm_err)?;
                }
                // Fix year: tm_year is years since 1900
                let yp = self
                    .builder
                    .build_struct_gep(sty, alloca, 0, "yp")
                    .map_err(llvm_err)?;
                let yv = self
                    .builder
                    .build_load(self.i64_ty(), yp, "yv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let ya = self
                    .builder
                    .build_int_add(yv, self.i64_ty().const_int(1900, false), "ya")
                    .map_err(llvm_err)?;
                self.builder.build_store(yp, ya).map_err(llvm_err)?;
                Ok(TypedValue::Struct(alloca, sty))
            }
            "diffSeconds" => {
                if args.len() != 2 {
                    return Err("diffSeconds expects 2 arguments (dt1, dt2)".to_string());
                }
                let d1 = self.compile_call_arg(args[0])?;
                let d2 = self.compile_call_arg(args[1])?;
                let (p1, st1) = match d1 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffSeconds: arguments must be DateTime structs".to_string()),
                };
                let (p2, _st2) = match d2 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffSeconds: arguments must be DateTime structs".to_string()),
                };
                let i64_ty = self.i64_ty();
                // Approximate seconds from year/month/day/hour/min/sec
                let extract = |builder: &inkwell::builder::Builder<'ctx>,
                               p: PointerValue<'ctx>,
                               st: inkwell::types::StructType<'ctx>|
                 -> Result<IntValue<'ctx>, String> {
                    let yp = builder.build_struct_gep(st, p, 0, "yp").map_err(llvm_err)?;
                    let y = builder
                        .build_load(i64_ty, yp, "y")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let mp = builder.build_struct_gep(st, p, 1, "mp").map_err(llvm_err)?;
                    let m = builder
                        .build_load(i64_ty, mp, "m")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let dp = builder.build_struct_gep(st, p, 2, "dp").map_err(llvm_err)?;
                    let d = builder
                        .build_load(i64_ty, dp, "d")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let hp = builder.build_struct_gep(st, p, 3, "hp").map_err(llvm_err)?;
                    let h = builder
                        .build_load(i64_ty, hp, "h")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let minp = builder
                        .build_struct_gep(st, p, 4, "minp")
                        .map_err(llvm_err)?;
                    let minv = builder
                        .build_load(i64_ty, minp, "min")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let sp = builder.build_struct_gep(st, p, 5, "sp").map_err(llvm_err)?;
                    let s = builder
                        .build_load(i64_ty, sp, "s")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let d365 = builder
                        .build_int_mul(y, i64_ty.const_int(365, false), "d365")
                        .map_err(llvm_err)?;
                    let d30 = builder
                        .build_int_mul(m, i64_ty.const_int(30, false), "d30")
                        .map_err(llvm_err)?;
                    let days = builder
                        .build_int_add(
                            builder.build_int_add(d365, d30, "d1").map_err(llvm_err)?,
                            d,
                            "d2",
                        )
                        .map_err(llvm_err)?;
                    let secs_per_day = i64_ty.const_int(86400, false);
                    let ds = builder
                        .build_int_mul(days, secs_per_day, "ds")
                        .map_err(llvm_err)?;
                    let hs = builder
                        .build_int_mul(h, i64_ty.const_int(3600, false), "hs")
                        .map_err(llvm_err)?;
                    let ms = builder
                        .build_int_mul(minv, i64_ty.const_int(60, false), "ms")
                        .map_err(llvm_err)?;
                    let total = builder
                        .build_int_add(
                            builder
                                .build_int_add(
                                    builder.build_int_add(ds, hs, "t1").map_err(llvm_err)?,
                                    ms,
                                    "t2",
                                )
                                .map_err(llvm_err)?,
                            s,
                            "t3",
                        )
                        .map_err(llvm_err)?;
                    Ok(total)
                };
                let t1 = extract(&self.builder, p1, st1)?;
                let t2 = extract(&self.builder, p2, st1)?;
                let diff = self
                    .builder
                    .build_int_sub(t1, t2, "diff")
                    .map_err(llvm_err)?;
                // Absolute value
                let zero = self.i64_ty().const_int(0, false);
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
                Ok(TypedValue::Int(abs_diff))
            }
            _ => Err(format!("Unknown datetime builtin: {}", name)),
        }
    }

    pub(crate) fn emit_today_now(
        &mut self,
        include_time: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let i32 = self.i32_ty();
        let ptr = self.ptr_ty();

        // Declare time(3) if not already declared: time_t time(time_t *tloc)
        let time_fn = self.module.get_function("time").unwrap_or_else(|| {
            self.module
                .add_function("time", i64.fn_type(&[ptr.into()], false), None)
        });

        // Declare localtime_r(3) if not already declared: struct tm *localtime_r(const time_t *timep, struct tm *result)
        let loc_fn = self.module.get_function("localtime_r").unwrap_or_else(|| {
            self.module.add_function(
                "localtime_r",
                ptr.fn_type(&[ptr.into(), ptr.into()], false),
                None,
            )
        });

        // struct tm = {i32, i32, i32, i32, i32, i32, i32, i32, i32}
        let tm_ty = self.context.struct_type(
            &[
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
            ],
            false,
        );

        // Call time(NULL) — pass null for tloc
        let null_ptr = ptr.const_zero();
        let now_ts = self
            .builder
            .build_call(time_fn, &[null_ptr.into()], "now_ts")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("time() call failed")?;

        // Allocate struct tm on stack, zero-init
        let tm_a = self
            .builder
            .build_alloca(tm_ty, "tm_buf")
            .map_err(llvm_err)?;
        let zero_i32 = i32.const_int(0, false);
        for i in 0..9u32 {
            let fp = self
                .builder
                .build_struct_gep(tm_ty, tm_a, i, "tm_f")
                .map_err(llvm_err)?;
            self.builder.build_store(fp, zero_i32).map_err(llvm_err)?;
        }

        // Allocate time_t for passing to localtime_r
        let ts_a = self.builder.build_alloca(i64, "ts_buf").map_err(llvm_err)?;
        self.builder.build_store(ts_a, now_ts).map_err(llvm_err)?;

        // Call localtime_r(&ts, &tm)
        let _ = self
            .builder
            .build_call(loc_fn, &[ts_a.into(), tm_a.into()], "")
            .map_err(llvm_err)?;

        // Load fields from struct tm
        let tm_cur = GepCursor::new(tm_a);

        // tm_year: years since 1900 → actual year = tm_year + 1900
        let tm_year_p = tm_cur.struct_gep(&self.builder, tm_ty, 5, "tm_year_p")?;
        let tm_year = self
            .builder
            .build_load(i32, tm_year_p, "tm_year")
            .map_err(llvm_err)?
            .into_int_value();
        let year = self
            .builder
            .build_int_add(
                self.builder
                    .build_int_s_extend(tm_year, i64, "year_ext")
                    .map_err(llvm_err)?,
                i64.const_int(1900, false),
                "year",
            )
            .map_err(llvm_err)?;

        // tm_mon: 0-11 → month = tm_mon + 1
        let tm_mon_p = tm_cur.struct_gep(&self.builder, tm_ty, 4, "tm_mon_p")?;
        let tm_mon = self
            .builder
            .build_load(i32, tm_mon_p, "tm_mon")
            .map_err(llvm_err)?
            .into_int_value();
        let month = self
            .builder
            .build_int_add(
                self.builder
                    .build_int_s_extend(tm_mon, i64, "mon_ext")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
                "month",
            )
            .map_err(llvm_err)?;

        // tm_mday: 1-31
        let tm_day_p = tm_cur.struct_gep(&self.builder, tm_ty, 3, "tm_day_p")?;
        let tm_day = self
            .builder
            .build_load(i32, tm_day_p, "tm_day")
            .map_err(llvm_err)?
            .into_int_value();
        let day = self
            .builder
            .build_int_s_extend(tm_day, i64, "day_ext")
            .map_err(llvm_err)?;

        if include_time {
            let dt_struct = self.type_layout.named_structs.get("DateTime").or_else(|| {
                self.type_layout
                    .anon_structs
                    .values()
                    .find(|s| s.get_field_types().len() == 6)
            });
            match dt_struct {
                Some(sty) => {
                    let sty = *sty;
                    let alloca = self.builder.build_alloca(sty, "now").map_err(llvm_err)?;
                    let now_cur = GepCursor::new(alloca);
                    // Store year, month, day
                    for (i, val) in [(0u32, year), (1, month), (2, day)].iter() {
                        let fp = now_cur.struct_gep(&self.builder, sty, *i, "f")?;
                        self.builder.build_store(fp, *val).map_err(llvm_err)?;
                    }
                    // tm_hour: 0-23
                    let tm_h_p = tm_cur.struct_gep(&self.builder, tm_ty, 2, "tm_h_p")?;
                    let tm_h = self
                        .builder
                        .build_load(i32, tm_h_p, "tm_h")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let hour = self
                        .builder
                        .build_int_s_extend(tm_h, i64, "h_ext")
                        .map_err(llvm_err)?;
                    // tm_min: 0-59
                    let tm_m_p = tm_cur.struct_gep(&self.builder, tm_ty, 1, "tm_min_p")?;
                    let tm_m = self
                        .builder
                        .build_load(i32, tm_m_p, "tm_m")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let min = self
                        .builder
                        .build_int_s_extend(tm_m, i64, "m_ext")
                        .map_err(llvm_err)?;
                    // tm_sec: 0-60
                    let tm_s_p = tm_cur.struct_gep(&self.builder, tm_ty, 0, "tm_s_p")?;
                    let tm_s = self
                        .builder
                        .build_load(i32, tm_s_p, "tm_s")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let sec = self
                        .builder
                        .build_int_s_extend(tm_s, i64, "s_ext")
                        .map_err(llvm_err)?;
                    for (i, val) in [(3u32, hour), (4, min), (5, sec)].iter() {
                        let fp = now_cur.struct_gep(&self.builder, sty, *i, "f")?;
                        self.builder.build_store(fp, *val).map_err(llvm_err)?;
                    }
                    Ok(TypedValue::Struct(alloca, sty))
                }
                None => Err("now: DateTime type not defined".to_string()),
            }
        } else {
            let date_struct = self.type_layout.named_structs.get("Date").or_else(|| {
                self.type_layout
                    .anon_structs
                    .values()
                    .find(|s| s.get_field_types().len() == 3)
            });
            match date_struct {
                Some(sty) => {
                    let sty = *sty;
                    let alloca = self.builder.build_alloca(sty, "today").map_err(llvm_err)?;
                    let today_cur = GepCursor::new(alloca);
                    for (i, val) in [(0u32, year), (1, month), (2, day)].iter() {
                        let fp = today_cur.struct_gep(&self.builder, sty, *i, "f")?;
                        self.builder.build_store(fp, *val).map_err(llvm_err)?;
                    }
                    Ok(TypedValue::Struct(alloca, sty))
                }
                None => Err("today: Date type not defined".to_string()),
            }
        }
    }
}
