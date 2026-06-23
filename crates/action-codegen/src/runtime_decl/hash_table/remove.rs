use crate::{llvm_err, CodeGen};
use inkwell::values::FunctionValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn define_ht_remove(
        &self,
        seq_fn: FunctionValue<'ctx>,
        _memcpy_fn: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let tomb = i64.const_int(Self::HT_TOMBSTONE, false);
        let hash_str = self.module.get_function("action_ht_hash_str").unwrap();
        let rc_dec_fn = self.module.get_function("action_rc_dec").unwrap();
        let ptr = self.ptr_ty();

        let f = self.module.add_function(
            "action_ht_remove",
            self.list_type
                .fn_type(&[self.list_type.into(), str_ty.into()], false),
            None,
        );
        let b0 = self.context.append_basic_block(f, "b0");
        let cow = self.context.append_basic_block(f, "cow");
        let merge = self.context.append_basic_block(f, "merge");
        let probe = self.context.append_basic_block(f, "probe");
        let probe_chk = self.context.append_basic_block(f, "probe_chk");
        let probe_key = self.context.append_basic_block(f, "probe_key");
        let probe_next = self.context.append_basic_block(f, "probe_next");
        let removed = self.context.append_basic_block(f, "removed");
        let not_found = self.context.append_basic_block(f, "not_found");

        self.builder.position_at_end(b0);
        let map = f.get_first_param().unwrap().into_struct_value();
        let key = f.get_nth_param(1).unwrap().into_struct_value();
        let data0 = self.extract_ptr(map, 0, "d")?;
        let len0 = self.extract_int(map, 1, "l")?;
        let cap0 = self.extract_int(map, 2, "c")?;
        let data = self.ht_cow(data0, cap0, b0, cow, merge)?;

        self.builder.position_at_end(merge);
        let hash = self.ht_hash_key(key, hash_str)?;
        let pr_a = self.builder.build_alloca(i64, "pr").map_err(llvm_err)?;
        self.builder.build_store(pr_a, zero).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(probe)
            .map_err(llvm_err)?;

        self.builder.position_at_end(probe);
        let prv = self.load_i64(pr_a, "prv")?;
        let idx = self.ht_probe_index(hash, prv, cap0)?;
        let (st, sp, svt, svp, sdist) = self.ht_load_slot(data, idx)?;
        self.builder
            .build_unconditional_branch(probe_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(probe_chk);
        let is_empty = self.ht_slot_is_empty(st, sp, svt, svp)?;
        let probe_tomb = self.context.append_basic_block(f, "probe_tomb");
        let probe_rh = self.context.append_basic_block(f, "probe_rh");
        self.builder
            .build_conditional_branch(is_empty, not_found, probe_tomb)
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
            .build_conditional_branch(too_far, not_found, probe_key)
            .map_err(llvm_err)?;

        self.builder.position_at_end(probe_key);
        let feq = self.ht_key_eq(st, sp, key, seq_fn)?;
        self.builder
            .build_conditional_branch(feq, removed, probe_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(removed);
        let marker = i64.const_int(Self::HT_SCALAR_MARKER, false);
        let kp_ne0 = self
            .builder
            .build_int_compare(IntPredicate::NE, sp, zero, "kpne0")
            .map_err(llvm_err)?;
        let kp_nem = self
            .builder
            .build_int_compare(IntPredicate::NE, sp, marker, "kpnem")
            .map_err(llvm_err)?;
        let kp_rc = self
            .builder
            .build_and(kp_ne0, kp_nem, "kprc")
            .map_err(llvm_err)?;
        let kp_dec_bb = self.context.append_basic_block(f, "kpdec");
        let vp_chk = self.context.append_basic_block(f, "vpchk");
        self.builder
            .build_conditional_branch(kp_rc, kp_dec_bb, vp_chk)
            .map_err(llvm_err)?;
        self.builder.position_at_end(kp_dec_bb);
        let _ = self.builder.build_call(
            rc_dec_fn,
            &[self
                .builder
                .build_int_to_ptr(sp, ptr, "kpp")
                .map_err(llvm_err)?
                .into()],
            "",
        );
        self.builder
            .build_unconditional_branch(vp_chk)
            .map_err(llvm_err)?;
        self.builder.position_at_end(vp_chk);
        let vp_ok = self
            .builder
            .build_int_compare(IntPredicate::NE, svp, zero, "vpo")
            .map_err(llvm_err)?;
        let vp_dec = self.context.append_basic_block(f, "vpdec");
        let tomb_bb = self.context.append_basic_block(f, "tomb");
        self.builder
            .build_conditional_branch(vp_ok, vp_dec, tomb_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(vp_dec);
        let _ = self.builder.build_call(
            rc_dec_fn,
            &[self
                .builder
                .build_int_to_ptr(svp, ptr, "vpp")
                .map_err(llvm_err)?
                .into()],
            "",
        );
        self.builder
            .build_unconditional_branch(tomb_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(tomb_bb);
        self.ht_store_slot(data, idx, zero, tomb, zero, zero, zero)?;
        let nlen = self
            .builder
            .build_int_sub(len0, one, "nlen")
            .map_err(llvm_err)?;
        let rr = self.ht_pack(data, nlen, cap0)?;
        self.builder.build_return(Some(&rr)).map_err(llvm_err)?;

        self.builder.position_at_end(probe_next);
        let cap_eq = self
            .builder
            .build_int_compare(IntPredicate::UGE, prv, cap0, "ce")
            .map_err(llvm_err)?;
        let npr = self
            .builder
            .build_int_add(prv, one, "npr")
            .map_err(llvm_err)?;
        self.builder.build_store(pr_a, npr).map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cap_eq, not_found, probe)
            .map_err(llvm_err)?;

        self.builder.position_at_end(not_found);
        let rn = self.ht_pack(data, len0, cap0)?;
        self.builder.build_return(Some(&rn)).map_err(llvm_err)?;

        Ok(())
    }
}
