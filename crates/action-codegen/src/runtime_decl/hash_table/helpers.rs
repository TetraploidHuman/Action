use crate::{llvm_err, CodeGen};
use inkwell::basic_block::BasicBlock;
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn ht_round_cap_pow2(&self, hint: IntValue<'ctx>) -> Result<IntValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let min_cap = i64.const_int(Self::HT_MIN_CAP, false);
        let h = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::ULT, hint, min_cap, "ltmin")
                    .map_err(llvm_err)?,
                min_cap,
                hint,
                "h",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let cap_a = self.builder.build_alloca(i64, "cap").map_err(llvm_err)?;
        self.builder.build_store(cap_a, min_cap).map_err(llvm_err)?;
        let loop_bb = self.context.append_basic_block(
            self.builder
                .get_insert_block()
                .unwrap()
                .get_parent()
                .unwrap(),
            "rcap_loop",
        );
        let body = self.context.append_basic_block(
            self.builder
                .get_insert_block()
                .unwrap()
                .get_parent()
                .unwrap(),
            "rcap_body",
        );
        let done = self.context.append_basic_block(
            self.builder
                .get_insert_block()
                .unwrap()
                .get_parent()
                .unwrap(),
            "rcap_done",
        );
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(loop_bb);
        let cv = self.load_i64(cap_a, "cv")?;
        let ok = self
            .builder
            .build_int_compare(IntPredicate::UGE, cv, h, "ok")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(ok, done, body)
            .map_err(llvm_err)?;
        self.builder.position_at_end(body);
        let nv = self
            .builder
            .build_int_mul(cv, i64.const_int(2, false), "nv")
            .map_err(llvm_err)?;
        self.builder.build_store(cap_a, nv).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(done);
        self.load_i64(cap_a, "rcap")
    }

    pub(crate) fn ht_pack(
        &self,
        data: PointerValue<'ctx>,
        len: IntValue<'ctx>,
        cap: IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let u = self.list_type.get_undef();
        let r0 = self
            .builder
            .build_insert_value(u, data, 0, "r0")
            .map_err(llvm_err)?;
        let r1 = self
            .builder
            .build_insert_value(r0, len, 1, "r1")
            .map_err(llvm_err)?;
        self.builder
            .build_insert_value(r1, cap, 2, "r2")
            .map_err(llvm_err)
            .map(|v| v.as_basic_value_enum())
    }

    pub(crate) fn ht_cow(
        &self,
        data: PointerValue<'ctx>,
        cap: IntValue<'ctx>,
        entry: BasicBlock<'ctx>,
        cow_bb: BasicBlock<'ctx>,
        merge: BasicBlock<'ctx>,
    ) -> Result<PointerValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let malloc_rc = self.module.get_function("action_malloc_rc").unwrap();
        let memcpy = self.module.get_function("memcpy").unwrap();

        self.builder.position_at_end(entry);
        let di = self
            .builder
            .build_ptr_to_int(data, i64, "di")
            .map_err(llvm_err)?;
        let rc_a = self
            .builder
            .build_int_sub(di, i64.const_int(8, false), "rca")
            .map_err(llvm_err)?;
        let rc_p = self
            .builder
            .build_int_to_ptr(rc_a, ptr, "rcp")
            .map_err(llvm_err)?;
        let rc = self
            .builder
            .build_load(i64, rc_p, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let need = self
            .builder
            .build_int_compare(IntPredicate::SGT, rc, i64.const_int(1, false), "nc")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(need, cow_bb, merge)
            .map_err(llvm_err)?;

        self.builder.position_at_end(cow_bb);
        let sz = self
            .builder
            .build_int_mul(cap, i64.const_int(Self::HT_ENTRY_BYTES, false), "sz")
            .map_err(llvm_err)?;
        let nd = self
            .builder
            .build_call(malloc_rc, &[sz.into()], "nd")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(memcpy, &[nd.into(), data.into(), sz.into()], "");
        let orc = self
            .builder
            .build_load(i64, rc_p, "orc")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(
                rc_p,
                self.builder
                    .build_int_sub(orc, i64.const_int(1, false), "nrc")
                    .map_err(llvm_err)?,
            )
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(merge)
            .map_err(llvm_err)?;

        self.builder.position_at_end(merge);
        let phi = self.builder.build_phi(ptr, "dphi").map_err(llvm_err)?;
        phi.add_incoming(&[(&data, entry), (&nd, cow_bb)]);
        Ok(phi.as_basic_value().into_pointer_value())
    }

    pub(crate) fn ht_probe_index(
        &self,
        hash: IntValue<'ctx>,
        probe: IntValue<'ctx>,
        cap: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let mask = self
            .builder
            .build_int_sub(cap, i64.const_int(1, false), "mask")
            .map_err(llvm_err)?;
        let sum = self
            .builder
            .build_int_add(hash, probe, "sum")
            .map_err(llvm_err)?;
        Ok(self.builder.build_and(sum, mask, "idx").map_err(llvm_err)?)
    }

    pub(crate) fn ht_hash_key(
        &self,
        key: inkwell::values::StructValue<'ctx>,
        hash_str_fn: FunctionValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let kt = self.extract_int(key, 0, "kt")?;
        let kp = self
            .builder
            .build_extract_value(key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let kpi = self
            .builder
            .build_ptr_to_int(kp, i64, "kpi")
            .map_err(llvm_err)?;
        let kp0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kpi, i64.const_int(0, false), "kp0")
            .map_err(llvm_err)?;
        let kp1 = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                kpi,
                i64.const_int(Self::HT_SCALAR_MARKER, false),
                "kp1",
            )
            .map_err(llvm_err)?;
        let is_scalar = self.builder.build_or(kp0, kp1, "isc").map_err(llvm_err)?;
        let sh = self
            .builder
            .build_call(hash_str_fn, &[key.into()], "sh")
            .map_err(llvm_err)?;
        let str_hash = sh.try_as_basic_value().unwrap_basic().into_int_value();
        let xg = self
            .builder
            .build_xor(kt, i64.const_int(Self::GOLDEN, false), "xg")
            .map_err(llvm_err)?;
        let int_hash = self
            .builder
            .build_int_mul(xg, i64.const_int(Self::GOLDEN, false), "ih")
            .map_err(llvm_err)?;
        Ok(self
            .builder
            .build_select(is_scalar, int_hash, str_hash, "hash")
            .map_err(llvm_err)?
            .into_int_value())
    }

    /// Hash from raw key tag + key ptr-int (for Robin-Hood swap re-hash).
    pub(crate) fn ht_hash_parts(
        &self,
        kt: IntValue<'ctx>,
        kpi: IntValue<'ctx>,
        hash_str_fn: FunctionValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let zero = i64.const_int(0, false);
        let marker = i64.const_int(Self::HT_SCALAR_MARKER, false);
        let kp0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kpi, zero, "kp0")
            .map_err(llvm_err)?;
        let kp1 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kpi, marker, "kp1")
            .map_err(llvm_err)?;
        let is_scalar = self.builder.build_or(kp0, kp1, "isc").map_err(llvm_err)?;
        let kp_p = self
            .builder
            .build_int_to_ptr(kpi, ptr, "kpp")
            .map_err(llvm_err)?;
        let u = str_ty.get_undef();
        let sk1 = self
            .builder
            .build_insert_value(u, kt, 0, "sk1")
            .map_err(llvm_err)?;
        let sk2 = self
            .builder
            .build_insert_value(sk1, kp_p, 1, "sk2")
            .map_err(llvm_err)?
            .into_struct_value();
        let sh = self
            .builder
            .build_call(hash_str_fn, &[sk2.into()], "sh")
            .map_err(llvm_err)?;
        let str_hash = sh.try_as_basic_value().unwrap_basic().into_int_value();
        let xg = self
            .builder
            .build_xor(kt, i64.const_int(Self::GOLDEN, false), "xg")
            .map_err(llvm_err)?;
        let int_hash = self
            .builder
            .build_int_mul(xg, i64.const_int(Self::GOLDEN, false), "ih")
            .map_err(llvm_err)?;
        Ok(self
            .builder
            .build_select(is_scalar, int_hash, str_hash, "hash")
            .map_err(llvm_err)?
            .into_int_value())
    }

    /// Build fat struct from raw key tag + key ptr-int for ht_key_eq.
    pub(crate) fn ht_fat_from_parts(
        &self,
        kt: IntValue<'ctx>,
        kpi: IntValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let kp_p = self
            .builder
            .build_int_to_ptr(kpi, ptr, "kpp")
            .map_err(llvm_err)?;
        let u = str_ty.get_undef();
        let k1 = self
            .builder
            .build_insert_value(u, kt, 0, "k1")
            .map_err(llvm_err)?;
        Ok(self
            .builder
            .build_insert_value(k1, kp_p, 1, "k2")
            .map_err(llvm_err)?
            .into_struct_value())
    }

    pub(crate) fn ht_key_eq_parts(
        &self,
        a_kt: IntValue<'ctx>,
        a_kpi: IntValue<'ctx>,
        b_kt: IntValue<'ctx>,
        b_kpi: IntValue<'ctx>,
        seq_fn: FunctionValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let b = self.ht_fat_from_parts(b_kt, b_kpi)?;
        self.ht_key_eq(a_kt, a_kpi, b, seq_fn)
    }

    pub(crate) fn ht_slot_is_empty(
        &self,
        kt: IntValue<'ctx>,
        kp: IntValue<'ctx>,
        vt: IntValue<'ctx>,
        vp: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let kp0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kp, zero, "kp0")
            .map_err(llvm_err)?;
        let kt0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kt, zero, "kt0")
            .map_err(llvm_err)?;
        let vt0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, vt, zero, "vt0")
            .map_err(llvm_err)?;
        let vp0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, vp, zero, "vp0")
            .map_err(llvm_err)?;
        let e12 = self.builder.build_and(kp0, kt0, "e12").map_err(llvm_err)?;
        let e34 = self.builder.build_and(vt0, vp0, "e34").map_err(llvm_err)?;
        self.builder.build_and(e12, e34, "empty").map_err(llvm_err)
    }

    pub(crate) fn ht_key_eq(
        &self,
        et: IntValue<'ctx>,
        ep: IntValue<'ctx>,
        query: inkwell::values::StructValue<'ctx>,
        seq_fn: FunctionValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let marker = i64.const_int(Self::HT_SCALAR_MARKER, false);
        let qkt = self.extract_int(query, 0, "qkt")?;
        let qkp = self
            .builder
            .build_extract_value(query, 1, "qkp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let qkp_i = self
            .builder
            .build_ptr_to_int(qkp, i64, "qkpi")
            .map_err(llvm_err)?;
        let ep_sc0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, ep, zero, "ep0")
            .map_err(llvm_err)?;
        let ep_sc1 = self
            .builder
            .build_int_compare(IntPredicate::EQ, ep, marker, "ep1")
            .map_err(llvm_err)?;
        let entry_scalar = self
            .builder
            .build_or(ep_sc0, ep_sc1, "esc")
            .map_err(llvm_err)?;
        let qp_sc0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, qkp_i, zero, "qp0")
            .map_err(llvm_err)?;
        let qp_sc1 = self
            .builder
            .build_int_compare(IntPredicate::EQ, qkp_i, marker, "qp1")
            .map_err(llvm_err)?;
        let query_scalar = self
            .builder
            .build_or(qp_sc0, qp_sc1, "qsc")
            .map_err(llvm_err)?;
        let both_scalar = self
            .builder
            .build_and(entry_scalar, query_scalar, "bsc")
            .map_err(llvm_err)?;
        let teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, et, qkt, "teq")
            .map_err(llvm_err)?;
        let ep_p = self
            .builder
            .build_int_to_ptr(ep, ptr, "epp")
            .map_err(llvm_err)?;
        let u = self.string_type.get_undef();
        let s1 = self
            .builder
            .build_insert_value(u, et, 0, "s1")
            .map_err(llvm_err)?;
        let s2 = self
            .builder
            .build_insert_value(s1, ep_p, 1, "s2")
            .map_err(llvm_err)?
            .into_struct_value();
        let seq = self
            .builder
            .build_call(seq_fn, &[s2.into(), query.into()], "seq")
            .map_err(llvm_err)?;
        let sb = seq.try_as_basic_value().unwrap_basic().into_int_value();
        Ok(self
            .builder
            .build_select(both_scalar, teq, sb, "feq")
            .map_err(llvm_err)?
            .into_int_value())
    }

    pub(crate) fn ht_kp_for_store(
        &self,
        kt: IntValue<'ctx>,
        kpi: IntValue<'ctx>,
        vt: IntValue<'ctx>,
        vpi: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let marker = i64.const_int(Self::HT_SCALAR_MARKER, false);
        let k0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kt, zero, "k0")
            .map_err(llvm_err)?;
        let p0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kpi, zero, "p0")
            .map_err(llvm_err)?;
        let v0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, vt, zero, "v0")
            .map_err(llvm_err)?;
        let vp0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, vpi, zero, "vp0")
            .map_err(llvm_err)?;
        let a12 = self.builder.build_and(k0, p0, "a12").map_err(llvm_err)?;
        let a34 = self.builder.build_and(v0, vp0, "a34").map_err(llvm_err)?;
        let allz = self.builder.build_and(a12, a34, "allz").map_err(llvm_err)?;
        Ok(self
            .builder
            .build_select(allz, marker, kpi, "skp")
            .map_err(llvm_err)?
            .into_int_value())
    }

    pub(crate) fn ht_grow_table(
        &self,
        data: PointerValue<'ctx>,
        cap: IntValue<'ctx>,
    ) -> Result<(PointerValue<'ctx>, IntValue<'ctx>), String> {
        let i64 = self.i64_ty();
        let rehash = self.module.get_function("action_ht_rehash").unwrap();
        let new_cap = self
            .builder
            .build_int_mul(cap, i64.const_int(2, false), "ncap")
            .map_err(llvm_err)?;
        let nd = self
            .builder
            .build_call(rehash, &[data.into(), cap.into(), new_cap.into()], "nd")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        Ok((nd, new_cap))
    }

    pub(crate) fn ht_load_slot(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
    ) -> Result<
        (
            IntValue<'ctx>,
            IntValue<'ctx>,
            IntValue<'ctx>,
            IntValue<'ctx>,
            IntValue<'ctx>,
        ),
        String,
    > {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let off = self
            .builder
            .build_int_mul(slot, i64.const_int(Self::HT_ENTRY_I64S, false), "off")
            .map_err(llvm_err)?;
        let di64 = self
            .builder
            .build_pointer_cast(data, ptr, "di64")
            .map_err(llvm_err)?;
        let mut vals = Vec::new();
        for d in 0..5 {
            let o = if d == 0 {
                off
            } else {
                self.builder
                    .build_int_add(off, i64.const_int(d, false), "o")
                    .map_err(llvm_err)?
            };
            let p = unsafe {
                self.builder
                    .build_gep(i64, di64, &[o], "p")
                    .map_err(llvm_err)?
            };
            vals.push(
                self.builder
                    .build_load(i64, p, "v")
                    .map_err(llvm_err)?
                    .into_int_value(),
            );
        }
        Ok((vals[0], vals[1], vals[2], vals[3], vals[4]))
    }

    pub(crate) fn ht_store_slot(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
        kt: IntValue<'ctx>,
        kp: IntValue<'ctx>,
        vt: IntValue<'ctx>,
        vp: IntValue<'ctx>,
        dist: IntValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let off = self
            .builder
            .build_int_mul(slot, i64.const_int(Self::HT_ENTRY_I64S, false), "off")
            .map_err(llvm_err)?;
        let di64 = self
            .builder
            .build_pointer_cast(data, ptr, "di64")
            .map_err(llvm_err)?;
        for (d, v) in [(0, kt), (1, kp), (2, vt), (3, vp), (4, dist)] {
            let o = if d == 0 {
                off
            } else {
                self.builder
                    .build_int_add(off, i64.const_int(d as u64, false), "o")
                    .map_err(llvm_err)?
            };
            let p = unsafe {
                self.builder
                    .build_gep(i64, di64, &[o], "sp")
                    .map_err(llvm_err)?
            };
            self.builder.build_store(p, v).map_err(llvm_err)?;
        }
        Ok(())
    }

    /// Branch to `active_bb` if slot at `data[slot]` is active; otherwise `skip_bb`.
    pub(crate) fn ht_branch_if_slot_active(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
        active_bb: BasicBlock<'ctx>,
        skip_bb: BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let (kt, kp, vt, vp, _) = self.ht_load_slot(data, slot)?;
        self.ht_branch_if_slot_active_fields(kt, kp, vt, vp, active_bb, skip_bb)
    }

    pub(crate) fn ht_branch_if_slot_active_fields(
        &self,
        kt: IntValue<'ctx>,
        kp: IntValue<'ctx>,
        vt: IntValue<'ctx>,
        vp: IntValue<'ctx>,
        active_bb: BasicBlock<'ctx>,
        skip_bb: BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let tomb = i64.const_int(Self::HT_TOMBSTONE, false);
        let kp0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kp, zero, "kp0")
            .map_err(llvm_err)?;
        let kt0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kt, zero, "kt0")
            .map_err(llvm_err)?;
        let vt0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, vt, zero, "vt0")
            .map_err(llvm_err)?;
        let vp0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, vp, zero, "vp0")
            .map_err(llvm_err)?;
        let e12 = self.builder.build_and(kp0, kt0, "e12").map_err(llvm_err)?;
        let e34 = self.builder.build_and(vt0, vp0, "e34").map_err(llvm_err)?;
        let is_empty = self
            .builder
            .build_and(e12, e34, "empty")
            .map_err(llvm_err)?;
        let is_tomb = self
            .builder
            .build_int_compare(IntPredicate::EQ, kp, tomb, "tomb")
            .map_err(llvm_err)?;
        let inactive = self
            .builder
            .build_or(is_empty, is_tomb, "inact")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(inactive, skip_bb, active_bb)
            .map_err(llvm_err)?;
        Ok(())
    }
}
