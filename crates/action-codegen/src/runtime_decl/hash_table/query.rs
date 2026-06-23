use crate::{llvm_err, CodeGen};
use inkwell::types::BasicType;
use inkwell::values::FunctionValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn define_ht_get_contains(&self, seq_fn: FunctionValue<'ctx>) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let b1_ty = self.bool_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let ptr = self.ptr_ty();
        let hash_str = self.module.get_function("action_ht_hash_str").unwrap();

        let tomb = i64.const_int(Self::HT_TOMBSTONE, false);

        use inkwell::types::BasicTypeEnum;

        for (name, ret_ty, is_get) in [
            ("action_ht_get", BasicTypeEnum::StructType(str_ty), true),
            ("action_ht_contains", BasicTypeEnum::IntType(b1_ty), false),
        ] {
            let f = self.module.add_function(
                name,
                ret_ty.fn_type(&[self.list_type.into(), str_ty.into()], false),
                None,
            );
            let b0 = self.context.append_basic_block(f, "b0");
            let probe = self.context.append_basic_block(f, "probe");
            let probe_chk = self.context.append_basic_block(f, "probe_chk");
            let probe_tomb = self.context.append_basic_block(f, "probe_tomb");
            let probe_rh = self.context.append_basic_block(f, "probe_rh");
            let found = self.context.append_basic_block(f, "found");
            let probe_key = self.context.append_basic_block(f, "probe_key");
            let probe_next = self.context.append_basic_block(f, "probe_next");
            let miss = self.context.append_basic_block(f, "miss");

            self.builder.position_at_end(b0);
            let map = f.get_first_param().unwrap().into_struct_value();
            let key = f.get_nth_param(1).unwrap().into_struct_value();
            let data = self.extract_ptr(map, 0, "d")?;
            let cap = self.extract_int(map, 2, "c")?;
            let hash = self.ht_hash_key(key, hash_str)?;
            let pr_a = self.builder.build_alloca(i64, "pr").map_err(llvm_err)?;
            self.builder.build_store(pr_a, zero).map_err(llvm_err)?;
            self.builder
                .build_unconditional_branch(probe)
                .map_err(llvm_err)?;

            self.builder.position_at_end(probe);
            let prv = self.load_i64(pr_a, "prv")?;
            let idx = self.ht_probe_index(hash, prv, cap)?;
            let (st, sp, svt, svp, sdist) = self.ht_load_slot(data, idx)?;
            self.builder
                .build_unconditional_branch(probe_chk)
                .map_err(llvm_err)?;

            self.builder.position_at_end(probe_chk);
            let is_empty = self.ht_slot_is_empty(st, sp, svt, svp)?;
            self.builder
                .build_conditional_branch(is_empty, miss, probe_tomb)
                .map_err(llvm_err)?;

            self.builder.position_at_end(probe_tomb);
            let is_tomb = self
                .builder
                .build_int_compare(IntPredicate::EQ, sp, tomb, "tomb")
                .map_err(llvm_err)?;
            self.builder
                .build_conditional_branch(is_tomb, probe_next, probe_rh)
                .map_err(llvm_err)?;

            self.builder.position_at_end(probe_rh);
            let too_far = self
                .builder
                .build_int_compare(IntPredicate::UGT, prv, sdist, "tf")
                .map_err(llvm_err)?;
            self.builder
                .build_conditional_branch(too_far, miss, probe_key)
                .map_err(llvm_err)?;

            self.builder.position_at_end(probe_key);
            let feq = self.ht_key_eq(st, sp, key, seq_fn)?;
            self.builder
                .build_conditional_branch(feq, found, probe_next)
                .map_err(llvm_err)?;

            self.builder.position_at_end(found);
            if is_get {
                let vp = self
                    .builder
                    .build_int_to_ptr(svp, ptr, "vp")
                    .map_err(llvm_err)?;
                let u = str_ty.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(u, svt, 0, "r1")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, vp, 1, "r2")
                    .map_err(llvm_err)?;
                self.builder.build_return(Some(&r2)).map_err(llvm_err)?;
            } else {
                self.builder
                    .build_return(Some(&b1_ty.const_int(1, false)))
                    .map_err(llvm_err)?;
            }

            self.builder.position_at_end(probe_next);
            let cap_eq = self
                .builder
                .build_int_compare(IntPredicate::UGE, prv, cap, "ce")
                .map_err(llvm_err)?;
            let npr = self
                .builder
                .build_int_add(prv, one, "npr")
                .map_err(llvm_err)?;
            self.builder.build_store(pr_a, npr).map_err(llvm_err)?;
            self.builder
                .build_conditional_branch(cap_eq, miss, probe)
                .map_err(llvm_err)?;

            self.builder.position_at_end(miss);
            if is_get {
                let u = str_ty.get_undef();
                let z = self
                    .builder
                    .build_insert_value(
                        self.builder
                            .build_insert_value(u, zero, 0, "z0")
                            .map_err(llvm_err)?,
                        ptr.const_zero(),
                        1,
                        "z1",
                    )
                    .map_err(llvm_err)?;
                self.builder.build_return(Some(&z)).map_err(llvm_err)?;
            } else {
                self.builder
                    .build_return(Some(&b1_ty.const_int(0, false)))
                    .map_err(llvm_err)?;
            }
        }
        Ok(())
    }
}
