// Submodule: builtins_stdlib_datetime/format_parse

use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, GepCursor, InnerType, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn datetime_dispatch_format_parse(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
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
                        Ok(Some(TypedValue::Str(alloca)))
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
                        Ok(Some(TypedValue::Enum(
                            result_alloca,
                            enum_ty,
                            InnerType::Int,
                            false,
                        )))
                    }
                    _ => Err("parseDate: expects (String, String)".to_string()),
                }
            }
            _ => Ok(None),
        }
    }
}
