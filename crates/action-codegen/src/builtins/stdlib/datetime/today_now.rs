// Submodule: builtins_stdlib_datetime/today_now

use crate::{llvm_err, CodeGen, GepCursor, TypedValue};

impl<'ctx> CodeGen<'ctx> {
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
