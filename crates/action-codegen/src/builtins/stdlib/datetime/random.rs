// Submodule: builtins_stdlib_datetime/random

use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn datetime_dispatch_random(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        match name {
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
                Ok(Some(TypedValue::Struct(alloca, rand_sty)))
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
                Ok(Some(TypedValue::Struct(tup_alloca, tuple_sty)))
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
                Ok(Some(TypedValue::Int(result)))
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
                Ok(Some(TypedValue::Float(result)))
            }
            _ => Ok(None),
        }
    }
}
