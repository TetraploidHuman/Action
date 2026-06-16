// Submodule: runtime_decl/define_hash_table
//
// Flat dense table for Map/Set: 32-byte entries (key_tag, key_ptr, val_tag, val_ptr).
// Struct { ptr data, i64 len, i64 cap } — reuses list_type.

use super::{llvm_err, CodeGen};
use inkwell::types::BasicType;
use inkwell::IntPredicate;
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};

impl<'ctx> CodeGen<'ctx> {
    const HT_ENTRY_I64S: u64 = 4;
    const HT_ENTRY_BYTES: u64 = 32;

    pub(super) fn define_hash_table(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let zero = i64.const_int(0, false);
        let malloc_rc = self.module.get_function("action_malloc_rc").unwrap();
        let realloc = self.module.get_function("realloc").unwrap();
        let memcpy = self.module.get_function("memcpy").unwrap();
        let seq_fn = self.module.get_function("action_string_eq").unwrap();
        let rc_dec = self.module.get_function("action_rc_dec").unwrap();
        let free_fn = self.module.get_function("free").unwrap();

        // action_ht_create(cap) -> {ptr, i64, i64}
        let cr = self.module.add_function(
            "action_ht_create",
            self.list_type.fn_type(&[i64.into()], false),
            None,
        );
        let cr_e = self.context.append_basic_block(cr, "entry");
        self.builder.position_at_end(cr_e);
        let cap = cr.get_first_param().unwrap().into_int_value();
        let dsz = self
            .builder
            .build_int_mul(cap, i64.const_int(Self::HT_ENTRY_BYTES, false), "dsz")
            .map_err(llvm_err)?;
        let data = self
            .builder
            .build_call(malloc_rc, &[dsz.into()], "d")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let r = self.ht_pack(data, zero, cap)?;
        self.builder.build_return(Some(&r));

        // action_ht_len
        let ln = self.module.add_function(
            "action_ht_len",
            i64.fn_type(&[self.list_type.into()], false),
            None,
        );
        let ln_e = self.context.append_basic_block(ln, "entry");
        self.builder.position_at_end(ln_e);
        let lnv = ln.get_first_param().unwrap().into_struct_value();
        self.builder
            .build_return(Some(
                &self
                    .builder
                    .build_extract_value(lnv, 1, "len")
                    .map_err(llvm_err)?,
            ))
            .map_err(llvm_err)?;

        self.define_ht_insert(seq_fn, memcpy, realloc)?;
        self.define_ht_get_contains(seq_fn)?;
        self.define_ht_remove(seq_fn, memcpy)?;
        self.define_ht_rc_dec(rc_dec, free_fn)?;
        self.define_ht_from_list()?;

        Ok(())
    }

    /// Build a Set (flat ht) from a List by inserting each element as key.
    fn define_ht_from_list(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let ht_create = self.module.get_function("action_ht_create").unwrap();
        let ht_insert = self.module.get_function("action_ht_insert").unwrap();
        let list_len_fn = self.module.get_function("action_list_len").unwrap();
        let list_get_fn = self.module.get_function("action_list_get").unwrap();
        let null_val: inkwell::values::BasicValueEnum = {
            let undef = str_ty.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, zero, 0, "sn0")
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(r1, self.ptr_ty().const_zero(), 1, "sn1")
                .map_err(llvm_err)?
                .as_basic_value_enum()
        };

        let f = self.module.add_function(
            "action_ht_from_list",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        let loop_bb = self.context.append_basic_block(f, "loop");
        let body = self.context.append_basic_block(f, "body");
        let done = self.context.append_basic_block(f, "done");

        self.builder.position_at_end(entry);
        let lst = f.get_first_param().unwrap().into_struct_value();
        let len_cc = self
            .builder
            .build_call(list_len_fn, &[lst.into()], "ll")
            .map_err(llvm_err)?;
        let len = len_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let cap = self
            .builder
            .build_int_add(len, i64.const_int(4, false), "cap")
            .map_err(llvm_err)?;
        let set_cc = self
            .builder
            .build_call(ht_create, &[cap.into()], "set")
            .map_err(llvm_err)?;
        let set0 = set_cc.try_as_basic_value().unwrap_basic();
        let set_a = self
            .builder
            .build_alloca(self.list_type, "sa")
            .map_err(llvm_err)?;
        self.builder.build_store(set_a, set0).map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(i_a, zero).map_err(llvm_err)?;
        self.builder.build_unconditional_branch(loop_bb).map_err(llvm_err)?;

        self.builder.position_at_end(loop_bb);
        let iv = self.load_i64(i_a, "iv")?;
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, len, "cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body, done)
            .map_err(llvm_err)?;

        self.builder.position_at_end(body);
        let set_v = self
            .builder
            .build_load(self.list_type, set_a, "sv")
            .map_err(llvm_err)?
            .into_struct_value();
        let elem = self
            .builder
            .build_call(list_get_fn, &[lst.into(), iv.into()], "el")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let ins = self
            .builder
            .build_call(
                ht_insert,
                &[set_v.into(), elem.into(), null_val.into()],
                "ins",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder.build_store(set_a, ins).map_err(llvm_err)?;
        let niv = self.builder.build_int_add(iv, one, "niv").map_err(llvm_err)?;
        self.builder.build_store(i_a, niv).map_err(llvm_err)?;
        self.builder.build_unconditional_branch(loop_bb).map_err(llvm_err)?;

        self.builder.position_at_end(done);
        let ret = self
            .builder
            .build_load(self.list_type, set_a, "ret")
            .map_err(llvm_err)?;
        self.builder.build_return(Some(&ret)).map_err(llvm_err)?;
        Ok(())
    }

    fn ht_pack(
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

    fn ht_cow(
        &self,
        data: PointerValue<'ctx>,
        cap: IntValue<'ctx>,
        entry: inkwell::basic_block::BasicBlock<'ctx>,
        cow_bb: inkwell::basic_block::BasicBlock<'ctx>,
        merge: inkwell::basic_block::BasicBlock<'ctx>,
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
        self.builder.build_unconditional_branch(merge).map_err(llvm_err)?;

        self.builder.position_at_end(merge);
        let phi = self.builder.build_phi(ptr, "dphi").map_err(llvm_err)?;
        phi.add_incoming(&[(&data, entry), (&nd, cow_bb)]);
        Ok(phi.as_basic_value().into_pointer_value())
    }

    fn ht_key_eq(
        &self,
        et: IntValue<'ctx>,
        ep: IntValue<'ctx>,
        query: inkwell::values::StructValue<'ctx>,
        seq_fn: inkwell::values::FunctionValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let qkt = self
            .builder
            .build_extract_value(query, 0, "qkt")
            .map_err(llvm_err)?
            .into_int_value();
        let qkp = self
            .builder
            .build_extract_value(query, 1, "qkp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let qkp_i = self
            .builder
            .build_ptr_to_int(qkp, i64, "qkpi")
            .map_err(llvm_err)?;
        let teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, et, qkt, "teq")
            .map_err(llvm_err)?;
        let kpz = self
            .builder
            .build_int_compare(IntPredicate::EQ, qkp_i, zero, "kpz")
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
            .build_select(kpz, teq, sb, "feq")
            .map_err(llvm_err)?
            .into_int_value())
    }

    fn ht_load_slot(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
    ) -> Result<(IntValue<'ctx>, IntValue<'ctx>, IntValue<'ctx>, IntValue<'ctx>), String> {
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
        for d in 0..4 {
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
        Ok((vals[0], vals[1], vals[2], vals[3]))
    }

    fn ht_store_slot(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
        kt: IntValue<'ctx>,
        kp: IntValue<'ctx>,
        vt: IntValue<'ctx>,
        vp: IntValue<'ctx>,
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
        for (d, v) in [(0, kt), (1, kp), (2, vt), (3, vp)] {
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

    fn define_ht_insert(
        &self,
        seq_fn: inkwell::values::FunctionValue<'ctx>,
        memcpy_fn: inkwell::values::FunctionValue<'ctx>,
        realloc_fn: inkwell::values::FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let four = i64.const_int(4, false);

        let f = self.module.add_function(
            "action_ht_insert",
            self.list_type.fn_type(
                &[self.list_type.into(), str_ty.into(), str_ty.into()],
                false,
            ),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        let cow = self.context.append_basic_block(f, "cow");
        let merge = self.context.append_basic_block(f, "merge");
        let search = self.context.append_basic_block(f, "search");
        let body = self.context.append_basic_block(f, "body");
        let ckey = self.context.append_basic_block(f, "ckey");
        let update = self.context.append_basic_block(f, "update");
        let next = self.context.append_basic_block(f, "next");
        let append_ck = self.context.append_basic_block(f, "append_ck");
        let append_grow = self.context.append_basic_block(f, "append_grow");
        let append_store = self.context.append_basic_block(f, "append_store");

        self.builder.position_at_end(entry);
        let map = f.get_first_param().unwrap().into_struct_value();
        let key = f.get_nth_param(1).unwrap().into_struct_value();
        let val = f.get_nth_param(2).unwrap().into_struct_value();
        let data0 = self.extract_ptr(map, 0, "d")?;
        let len0 = self.extract_int(map, 1, "l")?;
        let cap0 = self.extract_int(map, 2, "c")?;
        let kt = self.extract_int(key, 0, "kt")?;
        let kp = self
            .builder
            .build_extract_value(key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let kpi = self.builder.build_ptr_to_int(kp, i64, "kpi").map_err(llvm_err)?;
        let vt = self.extract_int(val, 0, "vt")?;
        let vp = self
            .builder
            .build_extract_value(val, 1, "vp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let vpi = self.builder.build_ptr_to_int(vp, i64, "vpi").map_err(llvm_err)?;

        let data = self.ht_cow(data0, cap0, entry, cow, merge)?;

        self.builder.position_at_end(merge);
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(i_a, zero).map_err(llvm_err)?;
        self.builder.build_unconditional_branch(search).map_err(llvm_err)?;

        self.builder.position_at_end(search);
        let iv = self.load_i64(i_a, "iv")?;
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, len0, "cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, body, append_ck)
            .map_err(llvm_err)?;

        self.builder.position_at_end(body);
        let (et, ep, _, _) = self.ht_load_slot(data, iv)?;
        let teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, et, kt, "teq")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(teq, ckey, next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ckey);
        let feq = self.ht_key_eq(et, ep, key, seq_fn)?;
        self.builder
            .build_conditional_branch(feq, update, next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(update);
        self.ht_store_slot(data, iv, et, ep, vt, vpi)?;
        let r = self.ht_pack(data, len0, cap0)?;
        self.builder.build_return(Some(&r));

        self.builder.position_at_end(next);
        self.builder
            .build_store(
                i_a,
                self.builder.build_int_add(iv, one, "niv").map_err(llvm_err)?,
            )
            .map_err(llvm_err)?;
        self.builder.build_unconditional_branch(search).map_err(llvm_err)?;

        self.builder.position_at_end(append_ck);
        let need_grow = self
            .builder
            .build_int_compare(IntPredicate::SGE, len0, cap0, "ng")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(need_grow, append_grow, append_store)
            .map_err(llvm_err)?;

        self.builder.position_at_end(append_grow);
        let cap_small = self
            .builder
            .build_int_compare(IntPredicate::SLT, cap0, four, "cs")
            .map_err(llvm_err)?;
        let cap2x = self.builder.build_int_mul(cap0, i64.const_int(2, false), "c2").map_err(llvm_err)?;
        let new_cap = self
            .builder
            .build_select(cap_small, four, cap2x, "ncap")
            .map_err(llvm_err)?
            .into_int_value();
        let data_size = self
            .builder
            .build_int_mul(new_cap, i64.const_int(Self::HT_ENTRY_BYTES, false), "dsz")
            .map_err(llvm_err)?;
        let malloc_rc = self.module.get_function("action_malloc_rc").unwrap();
        let new_data2 = self
            .builder
            .build_call(malloc_rc, &[data_size.into()], "nd")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let old_bytes = self
            .builder
            .build_int_mul(cap0, i64.const_int(Self::HT_ENTRY_BYTES, false), "ob")
            .map_err(llvm_err)?;
        let _ = self.builder.build_call(
            memcpy_fn,
            &[new_data2.into(), data.into(), old_bytes.into()],
            "",
        );
        self.builder.build_unconditional_branch(append_store).map_err(llvm_err)?;

        self.builder.position_at_end(append_store);
        let phi_data = self.builder.build_phi(ptr, "pd").map_err(llvm_err)?;
        phi_data.add_incoming(&[(&data, append_ck), (&new_data2, append_grow)]);
        let phi_cap = self.builder.build_phi(i64, "pc").map_err(llvm_err)?;
        phi_cap.add_incoming(&[(&cap0, append_ck), (&new_cap, append_grow)]);
        let ad = phi_data.as_basic_value().into_pointer_value();
        let ac = phi_cap.as_basic_value().into_int_value();
        self.ht_store_slot(ad, len0, kt, kpi, vt, vpi)?;
        let nl = self.builder.build_int_add(len0, one, "nl").map_err(llvm_err)?;
        let rr = self.ht_pack(ad, nl, ac)?;
        self.builder.build_return(Some(&rr));

        let _ = memcpy_fn;
        Ok(())
    }

    fn define_ht_get_contains(
        &self,
        seq_fn: inkwell::values::FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let b1_ty = self.bool_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let ptr = self.ptr_ty();

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
            let b1_bb = self.context.append_basic_block(f, "b1");
            let b2 = self.context.append_basic_block(f, "b2");
            let b3 = self.context.append_basic_block(f, "b3");
            let b4 = self.context.append_basic_block(f, "b4");
            let b5 = self.context.append_basic_block(f, "b5");
            let b6 = self.context.append_basic_block(f, "b6");

            self.builder.position_at_end(b0);
            let map = f.get_first_param().unwrap().into_struct_value();
            let key = f.get_nth_param(1).unwrap().into_struct_value();
            let data = self.extract_ptr(map, 0, "d")?;
            let len = self.extract_int(map, 1, "l")?;
            let kt = self.extract_int(key, 0, "kt")?;
            let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
            self.builder.build_store(i_a, zero).map_err(llvm_err)?;
            self.builder.build_unconditional_branch(b1_bb).map_err(llvm_err)?;

            self.builder.position_at_end(b1_bb);
            let iv = self.load_i64(i_a, "iv")?;
            let cond = self
                .builder
                .build_int_compare(IntPredicate::SLT, iv, len, "cond")
                .map_err(llvm_err)?;
            self.builder
                .build_conditional_branch(cond, b2, b6)
                .map_err(llvm_err)?;

            self.builder.position_at_end(b2);
            let (et, ep, svt, svp) = self.ht_load_slot(data, iv)?;
            let teq = self
                .builder
                .build_int_compare(IntPredicate::EQ, et, kt, "teq")
                .map_err(llvm_err)?;
            self.builder
                .build_conditional_branch(teq, b3, b5)
                .map_err(llvm_err)?;

            self.builder.position_at_end(b3);
            let feq = self.ht_key_eq(et, ep, key, seq_fn)?;
            self.builder
                .build_conditional_branch(feq, b4, b5)
                .map_err(llvm_err)?;

            self.builder.position_at_end(b4);
            if is_get {
                let vp = self.builder.build_int_to_ptr(svp, ptr, "vp").map_err(llvm_err)?;
                let u = str_ty.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(u, svt, 0, "r1")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, vp, 1, "r2")
                    .map_err(llvm_err)?;
                self.builder.build_return(Some(&r2));
            } else {
                self.builder
                    .build_return(Some(&b1_ty.const_int(1, false)))
                    .map_err(llvm_err)?;
            }

            self.builder.position_at_end(b5);
            self.builder
                .build_store(
                    i_a,
                    self.builder.build_int_add(iv, one, "niv").map_err(llvm_err)?,
                )
                .map_err(llvm_err)?;
            self.builder.build_unconditional_branch(b1_bb).map_err(llvm_err)?;

            self.builder.position_at_end(b6);
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
                self.builder.build_return(Some(&z));
            } else {
                self.builder
                    .build_return(Some(&b1_ty.const_int(0, false)))
                    .map_err(llvm_err)?;
            }
        }
        Ok(())
    }

    fn define_ht_remove(
        &self,
        seq_fn: inkwell::values::FunctionValue<'ctx>,
        memcpy_fn: inkwell::values::FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let i8 = self.context.i8_type();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);

        let f = self.module.add_function(
            "action_ht_remove",
            self.list_type.fn_type(&[self.list_type.into(), str_ty.into()], false),
            None,
        );
        let b0 = self.context.append_basic_block(f, "b0");
        let cow = self.context.append_basic_block(f, "cow");
        let merge = self.context.append_basic_block(f, "merge");
        let b1 = self.context.append_basic_block(f, "b1");
        let b2 = self.context.append_basic_block(f, "b2");
        let b3 = self.context.append_basic_block(f, "b3");
        let b4 = self.context.append_basic_block(f, "b4");
        let b5 = self.context.append_basic_block(f, "b5");
        let b6 = self.context.append_basic_block(f, "b6");
        let b7 = self.context.append_basic_block(f, "b7");

        self.builder.position_at_end(b0);
        let map = f.get_first_param().unwrap().into_struct_value();
        let key = f.get_nth_param(1).unwrap().into_struct_value();
        let data0 = self.extract_ptr(map, 0, "d")?;
        let len0 = self.extract_int(map, 1, "l")?;
        let cap0 = self.extract_int(map, 2, "c")?;
        let kt = self.extract_int(key, 0, "kt")?;
        let data = self.ht_cow(data0, cap0, b0, cow, merge)?;

        self.builder.position_at_end(merge);
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(i_a, zero).map_err(llvm_err)?;
        self.builder.build_unconditional_branch(b1).map_err(llvm_err)?;

        self.builder.position_at_end(b1);
        let iv = self.load_i64(i_a, "iv")?;
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, len0, "cond")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cond, b2, b7)
            .map_err(llvm_err)?;

        self.builder.position_at_end(b2);
        let (et, ep, _, _) = self.ht_load_slot(data, iv)?;
        let teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, et, kt, "teq")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(teq, b3, b6)
            .map_err(llvm_err)?;

        self.builder.position_at_end(b3);
        let feq = self.ht_key_eq(et, ep, key, seq_fn)?;
        self.builder
            .build_conditional_branch(feq, b4, b6)
            .map_err(llvm_err)?;

        self.builder.position_at_end(b4);
        let len_dec = self.builder.build_int_sub(len0, one, "ld").map_err(llvm_err)?;
        let iv_p1 = self.builder.build_int_add(iv, one, "ip1").map_err(llvm_err)?;
        let remaining = self
            .builder
            .build_int_sub(len0, iv_p1, "rem")
            .map_err(llvm_err)?;
        let has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, remaining, zero, "hr")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(has_rem, b5, b7)
            .map_err(llvm_err)?;

        self.builder.position_at_end(b5);
        let src_off = self
            .builder
            .build_int_mul(iv_p1, i64.const_int(Self::HT_ENTRY_BYTES, false), "so")
            .map_err(llvm_err)?;
        let dst_off = self
            .builder
            .build_int_mul(iv, i64.const_int(Self::HT_ENTRY_BYTES, false), "do")
            .map_err(llvm_err)?;
        let src = unsafe {
            self.builder
                .build_gep(i8, data, &[src_off], "src")
                .map_err(llvm_err)?
        };
        let dst = unsafe {
            self.builder
                .build_gep(i8, data, &[dst_off], "dst")
                .map_err(llvm_err)?
        };
        let rem_bytes = self
            .builder
            .build_int_mul(remaining, i64.const_int(Self::HT_ENTRY_BYTES, false), "rb")
            .map_err(llvm_err)?;
        let _ = self.builder.build_call(
            memcpy_fn,
            &[dst.into(), src.into(), rem_bytes.into()],
            "",
        );
        self.builder.build_unconditional_branch(b7).map_err(llvm_err)?;

        self.builder.position_at_end(b6);
        self.builder
            .build_store(
                i_a,
                self.builder.build_int_add(iv, one, "niv").map_err(llvm_err)?,
            )
            .map_err(llvm_err)?;
        self.builder.build_unconditional_branch(b1).map_err(llvm_err)?;

        self.builder.position_at_end(b7);
        let ret_len = self.builder.build_phi(i64, "rl").map_err(llvm_err)?;
        ret_len.add_incoming(&[(&len0, b1), (&len_dec, b4), (&len_dec, b5)]);
        let r = self.ht_pack(data, ret_len.as_basic_value().into_int_value(), cap0)?;
        self.builder.build_return(Some(&r));

        Ok(())
    }

    fn define_ht_rc_dec(
        &self,
        rc_dec_fn: inkwell::values::FunctionValue<'ctx>,
        free_fn: inkwell::values::FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);

        let f = self.module.add_function(
            "action_rc_dec_ht",
            self.void_ty().fn_type(&[ptr.into(), i64.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        let null_done = self.context.append_basic_block(f, "null");
        let do_dec = self.context.append_basic_block(f, "do_dec");
        let chk = self.context.append_basic_block(f, "chk");
        let done = self.context.append_basic_block(f, "done");
        let clean = self.context.append_basic_block(f, "clean");
        let clp = self.context.append_basic_block(f, "clp");
        let clb = self.context.append_basic_block(f, "clb");
        let free_bb = self.context.append_basic_block(f, "free");

        self.builder.position_at_end(entry);
        let data = f.get_first_param().unwrap().into_pointer_value();
        let cap = f.get_nth_param(1).unwrap().into_int_value();
        let len = f.get_nth_param(2).unwrap().into_int_value();
        let is_null = self.builder.build_is_null(data, "n").map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(is_null, null_done, do_dec)
            .map_err(llvm_err)?;
        self.builder.position_at_end(null_done);
        self.builder.build_return(None).map_err(llvm_err)?;

        self.builder.position_at_end(do_dec);
        let di = self.builder.build_ptr_to_int(data, i64, "di").map_err(llvm_err)?;
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
        let nrc = self.builder.build_int_sub(rc, one, "nrc").map_err(llvm_err)?;
        self.builder.build_store(rc_p, nrc).map_err(llvm_err)?;
        self.builder.build_unconditional_branch(chk).map_err(llvm_err)?;

        self.builder.position_at_end(chk);
        let is_z = self
            .builder
            .build_int_compare(IntPredicate::EQ, nrc, zero, "z")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(is_z, clean, done)
            .map_err(llvm_err)?;
        self.builder.position_at_end(done);
        self.builder.build_return(None).map_err(llvm_err)?;

        self.builder.position_at_end(clean);
        let si = self.builder.build_alloca(i64, "si").map_err(llvm_err)?;
        self.builder.build_store(si, zero).map_err(llvm_err)?;
        self.builder.build_unconditional_branch(clp).map_err(llvm_err)?;

        self.builder.position_at_end(clp);
        let siv = self.load_i64(si, "siv")?;
        let sc = self
            .builder
            .build_int_compare(IntPredicate::SLT, siv, len, "sc")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(sc, clb, free_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(clb);
        let (_, kp, _, vp) = self.ht_load_slot(data, siv)?;
        let kp_ok = self
            .builder
            .build_int_compare(IntPredicate::NE, kp, zero, "kpo")
            .map_err(llvm_err)?;
        let kp_bb = self.context.append_basic_block(f, "kpdec");
        let vp_bb = self.context.append_basic_block(f, "vpchk");
        let skip = self.context.append_basic_block(f, "clskip");
        self.builder
            .build_conditional_branch(kp_ok, kp_bb, vp_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(kp_bb);
        let _ = self.builder.build_call(
            rc_dec_fn,
            &[self.builder.build_int_to_ptr(kp, ptr, "kpp").map_err(llvm_err)?.into()],
            "",
        );
        self.builder.build_unconditional_branch(vp_bb).map_err(llvm_err)?;
        self.builder.position_at_end(vp_bb);
        let vp_ok = self
            .builder
            .build_int_compare(IntPredicate::NE, vp, zero, "vpo")
            .map_err(llvm_err)?;
        let vp_dec = self.context.append_basic_block(f, "vpdec");
        self.builder
            .build_conditional_branch(vp_ok, vp_dec, skip)
            .map_err(llvm_err)?;
        self.builder.position_at_end(vp_dec);
        let _ = self.builder.build_call(
            rc_dec_fn,
            &[self.builder.build_int_to_ptr(vp, ptr, "vpp").map_err(llvm_err)?.into()],
            "",
        );
        self.builder.build_unconditional_branch(skip).map_err(llvm_err)?;
        self.builder.position_at_end(skip);
        self.builder
            .build_store(
                si,
                self.builder.build_int_add(siv, one, "ni").map_err(llvm_err)?,
            )
            .map_err(llvm_err)?;
        self.builder.build_unconditional_branch(clp).map_err(llvm_err)?;

        self.builder.position_at_end(free_bb);
        let _ = self.builder.build_call(
            free_fn,
            &[self.builder.build_int_to_ptr(rc_a, ptr, "fp").map_err(llvm_err)?.into()],
            "",
        );
        self.builder.build_return(None).map_err(llvm_err)?;

        let _ = cap;
        Ok(())
    }

    fn extract_ptr(
        &self,
        st: inkwell::values::StructValue<'ctx>,
        idx: u32,
        name: &str,
    ) -> Result<PointerValue<'ctx>, String> {
        Ok(self
            .builder
            .build_extract_value(st, idx, name)
            .map_err(llvm_err)?
            .into_pointer_value())
    }

    fn extract_int(
        &self,
        st: inkwell::values::StructValue<'ctx>,
        idx: u32,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        Ok(self
            .builder
            .build_extract_value(st, idx, name)
            .map_err(llvm_err)?
            .into_int_value())
    }

    fn load_i64(&self, a: PointerValue<'ctx>, name: &str) -> Result<IntValue<'ctx>, String> {
        Ok(self
            .builder
            .build_load(self.i64_ty(), a, name)
            .map_err(llvm_err)?
            .into_int_value())
    }

    /// Load key as fat struct from dense slot `i` (0..len-1).
    pub(super) fn ht_key_fat_at(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let (kt, kp, _, _) = self.ht_load_slot(data, slot)?;
        let kp_p = self
            .builder
            .build_int_to_ptr(kp, ptr, "kp")
            .map_err(llvm_err)?;
        let u = str_ty.get_undef();
        let k1 = self
            .builder
            .build_insert_value(u, kt, 0, "k1")
            .map_err(llvm_err)?;
        self.builder
            .build_insert_value(k1, kp_p, 1, "k2")
            .map_err(llvm_err)
            .map(|v| v.as_basic_value_enum())
    }

    /// Load value as fat struct from dense slot `i`.
    pub(super) fn ht_val_fat_at(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let (_, _, vt, vp) = self.ht_load_slot(data, slot)?;
        let vp_p = self
            .builder
            .build_int_to_ptr(vp, ptr, "vp")
            .map_err(llvm_err)?;
        let u = str_ty.get_undef();
        let v1 = self
            .builder
            .build_insert_value(u, vt, 0, "v1")
            .map_err(llvm_err)?;
        self.builder
            .build_insert_value(v1, vp_p, 1, "v2")
            .map_err(llvm_err)
            .map(|v| v.as_basic_value_enum())
    }
}
