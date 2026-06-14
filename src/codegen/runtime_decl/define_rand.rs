// Submodule: runtime_decl/define_rand
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_rand(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let f64 = self.f64_ty();
        let _void = self.void_ty();
        let _ptr = self.ptr_ty();
        let _str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let _i32 = self.context.i32_type();
        let _i8 = self.context.i8_type();
            let _list_create_fn = self.module.get_function("action_list_create").unwrap();
            let _list_push_fn = self.module.get_function("action_list_push").unwrap();
            let _list_get_fn = self.module.get_function("action_list_get").unwrap();
            // ---- action_rand_init() ----
            // Simple LCG state: uses a global i64 seed initialized to 1
            let rand_seed_g = self.module.add_global(i64, None, "action_rand_seed");
            rand_seed_g.set_initializer(&i64.const_int(123456789, false));

            // ---- action_rand_int(i64 min, i64 max) -> i64 ----
            let ri_fn = self.module.add_function(
                "action_rand_int",
                i64.fn_type(&[i64.into(), i64.into()], false),
                None,
            );
            let entry = self.context.append_basic_block(ri_fn, "entry");
            self.builder.position_at_end(entry);
            let ri_min = ri_fn.get_first_param().unwrap().into_int_value();
            let ri_max = ri_fn.get_nth_param(1).unwrap().into_int_value();
            // LCG: seed = seed * 1103515245 + 12345
            let ri_seed_ptr = rand_seed_g.as_pointer_value();
            let ri_old_seed = self
                .builder
                .build_load(i64, ri_seed_ptr, "old_seed")
                .map_err(llvm_err)?
                .into_int_value();
            let ri_mul = self
                .builder
                .build_int_mul(ri_old_seed, i64.const_int(1103515245, false), "mul")
                .map_err(llvm_err)?;
            let ri_new_seed = self
                .builder
                .build_int_add(ri_mul, i64.const_int(12345, false), "new_seed")
                .map_err(llvm_err)?;
            self.builder
                .build_store(ri_seed_ptr, ri_new_seed)
                .map_err(llvm_err)?;
            // range = max - min + 1
            let ri_range = self
                .builder
                .build_int_sub(ri_max, ri_min, "sub")
                .map_err(llvm_err)?;
            let ri_range1 = self
                .builder
                .build_int_add(ri_range, i64.const_int(1, false), "range1")
                .map_err(llvm_err)?;
            // result = min + (new_seed % range)
            let _ri_range_pos = self
                .builder
                .build_int_compare(IntPredicate::SGT, ri_range1, i64.const_int(0, false), "pos")
                .map_err(llvm_err)?;
            // Use unsigned remainder to avoid negative issues
            let ri_rem = self
                .builder
                .build_int_unsigned_rem(ri_new_seed, ri_range1, "rem")
                .map_err(llvm_err)?;
            let ri_zero = self
                .builder
                .build_int_compare(
                    IntPredicate::ULE,
                    ri_range1,
                    i64.const_int(0, false),
                    "zero_range",
                )
                .map_err(llvm_err)?;
            // If range <= 0, return min
            let ri_result = self
                .builder
                .build_select(
                    ri_zero,
                    ri_min,
                    self.builder
                        .build_int_add(ri_min, ri_rem, "add")
                        .map_err(llvm_err)?,
                    "result",
                )
                .map_err(llvm_err)?
                .into_int_value();
            let _ = self.builder.build_return(Some(&ri_result));

            // ---- action_rand_float() -> f64 ----
            let rf_fn =
                self.module
                    .add_function("action_rand_float", f64.fn_type(&[], false), None);
            let entry = self.context.append_basic_block(rf_fn, "entry");
            self.builder.position_at_end(entry);
            // Use the same LCG seed, return value in [0, 1)
            let rf_seed_ptr = rand_seed_g.as_pointer_value();
            let rf_old_seed = self
                .builder
                .build_load(i64, rf_seed_ptr, "old_seed")
                .map_err(llvm_err)?
                .into_int_value();
            let rf_mul = self
                .builder
                .build_int_mul(rf_old_seed, i64.const_int(1103515245, false), "mul")
                .map_err(llvm_err)?;
            let rf_new_seed = self
                .builder
                .build_int_add(rf_mul, i64.const_int(12345, false), "new_seed")
                .map_err(llvm_err)?;
            self.builder
                .build_store(rf_seed_ptr, rf_new_seed)
                .map_err(llvm_err)?;
            // Convert to float: (new_seed & 0x7fffffffffffffff) / 0x7fffffffffffffff
            let rf_mask = i64.const_int(0x7fffffffffffffff_u64, false);
            let rf_masked = self
                .builder
                .build_and(rf_new_seed, rf_mask, "masked")
                .map_err(llvm_err)?;
            let rf_f64 = self
                .builder
                .build_unsigned_int_to_float(rf_masked, f64, "f64")
                .map_err(llvm_err)?;
            let rf_divisor = f64.const_float(0x7fffffffffffffff_u64 as f64);
            let rf_result = self
                .builder
                .build_float_div(rf_f64, rf_divisor, "result")
                .map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&rf_result));

            Ok(())
    }
}
