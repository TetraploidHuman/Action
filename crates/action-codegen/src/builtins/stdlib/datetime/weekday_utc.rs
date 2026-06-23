// Submodule: builtins_stdlib_datetime/weekday_utc

use inkwell::values::{IntValue, PointerValue};
use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn datetime_dispatch_weekday_utc(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
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
                        Ok(Some(TypedValue::Int(wd)))
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
                Ok(Some(TypedValue::Struct(alloca, sty)))
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
                Ok(Some(TypedValue::Int(abs_diff)))
            }
            _ => Ok(None),
        }
    }
}
