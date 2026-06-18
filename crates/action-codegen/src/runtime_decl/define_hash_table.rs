// Submodule: runtime_decl/define_hash_table
//
// Open-addressing hash table for Map/Set with Robin-Hood probing.
// 40-byte entries: key_tag, key_ptr, val_tag, val_ptr, dist (probe distance from ideal slot).
// Struct { ptr data, i64 len, i64 cap } — reuses list_type; len = occupied count, cap = slot count.

use super::{llvm_err, CodeGen};
use inkwell::basic_block::BasicBlock;
use inkwell::types::BasicType;
use inkwell::values::{BasicValue, BasicValueEnum, FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    const HT_ENTRY_I64S: u64 = 5;
    const HT_ENTRY_BYTES: u64 = 40;
    const HT_SCALAR_MARKER: u64 = 1;
    const HT_TOMBSTONE: u64 = 2;
    const HT_MIN_CAP: u64 = 8;
    const HT_LOAD_NUM: u64 = 3;
    const HT_LOAD_DEN: u64 = 4;
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;
    const GOLDEN: u64 = 0x9e3779b97f4a7c15;

    pub(super) fn define_hash_table(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let malloc_rc = self.module.get_function("action_malloc_rc").unwrap();
        let memset = self.module.get_function("memset").unwrap();
        let memcpy = self.module.get_function("memcpy").unwrap();
        let seq_fn = self.module.get_function("action_string_eq").unwrap();
        let rc_dec = self.module.get_function("action_rc_dec").unwrap();
        let free_fn = self.module.get_function("free").unwrap();
        let zero = i64.const_int(0, false);
        let i32z = self.context.i32_type().const_int(0, false);

        self.define_ht_hash_str()?;
        self.define_ht_rehash(seq_fn, malloc_rc, memset)?;

        // action_ht_create(cap_hint) -> {ptr, i64, i64}
        let cr = self.module.add_function(
            "action_ht_create",
            self.list_type.fn_type(&[i64.into()], false),
            None,
        );
        let cr_e = self.context.append_basic_block(cr, "entry");
        self.builder.position_at_end(cr_e);
        let hint = cr.get_first_param().unwrap().into_int_value();
        let load_num = i64.const_int(Self::HT_LOAD_NUM, false);
        let load_den = i64.const_int(Self::HT_LOAD_DEN, false);
        let one = i64.const_int(1, false);
        // Scale element-count hint to slot capacity for 75% max load factor.
        let scaled = self
            .builder
            .build_int_add(
                self.builder
                    .build_int_unsigned_div(
                        self.builder
                            .build_int_mul(hint, load_den, "h4")
                            .map_err(llvm_err)?,
                        load_num,
                        "hs",
                    )
                    .map_err(llvm_err)?,
                one,
                "scaled",
            )
            .map_err(llvm_err)?;
        let cap = self.ht_round_cap_pow2(scaled)?;
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
        let _ = self
            .builder
            .build_call(memset, &[data.into(), i32z.into(), dsz.into()], "");
        let r = self.ht_pack(data, zero, cap)?;
        self.builder.build_return(Some(&r)).map_err(llvm_err)?;

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

        self.define_ht_insert(seq_fn, memcpy)?;
        self.define_ht_get_contains(seq_fn)?;
        self.define_ht_remove(seq_fn, memcpy)?;
        self.define_ht_rc_dec(rc_dec, free_fn)?;
        self.define_ht_from_list()?;

        Ok(())
    }

    /// FNV-1a hash over string bytes: action_ht_hash_str({i64 len, ptr}) -> i64
    fn define_ht_hash_str(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let i8 = self.context.i8_type();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let fnv_off = i64.const_int(Self::FNV_OFFSET, false);
        let fnv_prime = i64.const_int(Self::FNV_PRIME, false);

        let f = self.module.add_function(
            "action_ht_hash_str",
            i64.fn_type(&[self.string_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        let loop_bb = self.context.append_basic_block(f, "loop");
        let body = self.context.append_basic_block(f, "body");
        let done = self.context.append_basic_block(f, "done");
        let scalar_bb = self.context.append_basic_block(f, "scalar");
        let str_init = self.context.append_basic_block(f, "str_init");

        self.builder.position_at_end(entry);
        let s = f.get_first_param().unwrap().into_struct_value();
        let len = self.extract_int(s, 0, "len")?;
        let buf = self.extract_ptr(s, 1, "buf")?;
        let is_null = self.builder.build_is_null(buf, "nbuf").map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(is_null, scalar_bb, str_init)
            .map_err(llvm_err)?;

        self.builder.position_at_end(scalar_bb);
        let sh = self
            .builder
            .build_xor(len, i64.const_int(Self::GOLDEN, false), "sh")
            .map_err(llvm_err)?;
        let sh2 = self
            .builder
            .build_int_mul(sh, i64.const_int(Self::GOLDEN, false), "sh2")
            .map_err(llvm_err)?;
        self.builder.build_return(Some(&sh2)).map_err(llvm_err)?;

        self.builder.position_at_end(str_init);
        let h_a = self.builder.build_alloca(i64, "h").map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(h_a, fnv_off).map_err(llvm_err)?;
        self.builder.build_store(i_a, zero).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(llvm_err)?;

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
        let bp = unsafe {
            self.builder
                .build_gep(i8, buf, &[iv], "bp")
                .map_err(llvm_err)?
        };
        let byte = self
            .builder
            .build_load(i8, bp, "b")
            .map_err(llvm_err)?
            .into_int_value();
        let byte64 = self
            .builder
            .build_int_z_extend(byte, i64, "b64")
            .map_err(llvm_err)?;
        let hv = self.load_i64(h_a, "hv")?;
        let xored = self
            .builder
            .build_xor(hv, byte64, "xor")
            .map_err(llvm_err)?;
        let nh = self
            .builder
            .build_int_mul(xored, fnv_prime, "nh")
            .map_err(llvm_err)?;
        self.builder.build_store(h_a, nh).map_err(llvm_err)?;
        let niv = self
            .builder
            .build_int_add(iv, one, "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, niv).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(done);
        let ret = self.load_i64(h_a, "ret")?;
        self.builder.build_return(Some(&ret)).map_err(llvm_err)?;
        Ok(())
    }

    /// Reinsert active entries from old table into a new zeroed table (Robin-Hood).
    fn define_ht_rehash(
        &self,
        seq_fn: FunctionValue<'ctx>,
        malloc_rc: FunctionValue<'ctx>,
        memset: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let tomb = i64.const_int(Self::HT_TOMBSTONE, false);
        let i32z = self.context.i32_type().const_int(0, false);
        let hash_str = self.module.get_function("action_ht_hash_str").unwrap();

        let f = self.module.add_function(
            "action_ht_rehash",
            ptr.fn_type(&[ptr.into(), i64.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(f, "entry");
        let scan = self.context.append_basic_block(f, "scan");
        let scan_body = self.context.append_basic_block(f, "scan_body");
        let scan_skip = self.context.append_basic_block(f, "scan_skip");
        let scan_done = self.context.append_basic_block(f, "scan_done");
        let ins_init = self.context.append_basic_block(f, "ins_init");
        let ins_chk = self.context.append_basic_block(f, "ins_chk");
        let ins_store = self.context.append_basic_block(f, "ins_store");
        let ins_swap_chk = self.context.append_basic_block(f, "ins_swap_chk");
        let ins_swap_body = self.context.append_basic_block(f, "ins_swap_body");
        let ins_next = self.context.append_basic_block(f, "ins_next");

        self.builder.position_at_end(entry);
        let old_data = f.get_first_param().unwrap().into_pointer_value();
        let old_cap = f.get_nth_param(1).unwrap().into_int_value();
        let new_cap = f.get_nth_param(2).unwrap().into_int_value();
        let dsz = self
            .builder
            .build_int_mul(new_cap, i64.const_int(Self::HT_ENTRY_BYTES, false), "dsz")
            .map_err(llvm_err)?;
        let new_data = self
            .builder
            .build_call(malloc_rc, &[dsz.into()], "nd")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(memset, &[new_data.into(), i32z.into(), dsz.into()], "");
        let si_a = self.builder.build_alloca(i64, "si").map_err(llvm_err)?;
        let ikt_a = self.builder.build_alloca(i64, "ikt").map_err(llvm_err)?;
        let ikp_a = self.builder.build_alloca(i64, "ikp").map_err(llvm_err)?;
        let ivt_a = self.builder.build_alloca(i64, "ivt").map_err(llvm_err)?;
        let ivp_a = self.builder.build_alloca(i64, "ivp").map_err(llvm_err)?;
        let idist_a = self.builder.build_alloca(i64, "idist").map_err(llvm_err)?;
        let hash_a = self.builder.build_alloca(i64, "hash").map_err(llvm_err)?;
        self.builder.build_store(si_a, zero).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(scan)
            .map_err(llvm_err)?;

        self.builder.position_at_end(scan);
        let siv = self.load_i64(si_a, "siv")?;
        let scan_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, siv, old_cap, "sc")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(scan_cond, scan_body, scan_done)
            .map_err(llvm_err)?;

        self.builder.position_at_end(scan_body);
        let (ekt, ekp, evt, evp, _) = self.ht_load_slot(old_data, siv)?;
        let is_empty = self.ht_slot_is_empty(ekt, ekp, evt, evp)?;
        let is_tomb = self
            .builder
            .build_int_compare(IntPredicate::EQ, ekp, tomb, "tomb")
            .map_err(llvm_err)?;
        let skip = self
            .builder
            .build_or(is_empty, is_tomb, "skip")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(skip, scan_skip, ins_init)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_init);
        self.builder.build_store(ikt_a, ekt).map_err(llvm_err)?;
        self.builder.build_store(ikp_a, ekp).map_err(llvm_err)?;
        self.builder.build_store(ivt_a, evt).map_err(llvm_err)?;
        self.builder.build_store(ivp_a, evp).map_err(llvm_err)?;
        self.builder.build_store(idist_a, zero).map_err(llvm_err)?;
        let eh = self.ht_hash_parts(ekt, ekp, hash_str)?;
        self.builder.build_store(hash_a, eh).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(ins_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_chk);
        let cdist = self.load_i64(idist_a, "cdist")?;
        let chash = self.load_i64(hash_a, "chash")?;
        let idx = self.ht_probe_index(chash, cdist, new_cap)?;
        let (st, sp, svt, svp, sdist) = self.ht_load_slot(new_data, idx)?;
        let slot_empty = self.ht_slot_is_empty(st, sp, svt, svp)?;
        let sp_t = self
            .builder
            .build_int_compare(IntPredicate::EQ, sp, tomb, "spt")
            .map_err(llvm_err)?;
        let can_ins = self
            .builder
            .build_or(slot_empty, sp_t, "ci")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(can_ins, ins_store, ins_swap_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_store);
        let ikt = self.load_i64(ikt_a, "ikt")?;
        let ikp = self.load_i64(ikp_a, "ikp")?;
        let ivt = self.load_i64(ivt_a, "ivt")?;
        let ivp = self.load_i64(ivp_a, "ivp")?;
        self.ht_store_slot(new_data, idx, ikt, ikp, ivt, ivp, cdist)?;
        self.builder
            .build_unconditional_branch(scan_skip)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_swap_chk);
        let steal = self
            .builder
            .build_int_compare(IntPredicate::UGT, cdist, sdist, "steal")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(steal, ins_swap_body, ins_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_swap_body);
        let ikt = self.load_i64(ikt_a, "ikt_v")?;
        let ikp = self.load_i64(ikp_a, "ikp_v")?;
        let ivt = self.load_i64(ivt_a, "ivt_v")?;
        let ivp = self.load_i64(ivp_a, "ivp_v")?;
        self.ht_store_slot(new_data, idx, ikt, ikp, ivt, ivp, cdist)?;
        self.builder.build_store(ikt_a, st).map_err(llvm_err)?;
        self.builder.build_store(ikp_a, sp).map_err(llvm_err)?;
        self.builder.build_store(ivt_a, svt).map_err(llvm_err)?;
        self.builder.build_store(ivp_a, svp).map_err(llvm_err)?;
        self.builder.build_store(idist_a, sdist).map_err(llvm_err)?;
        let nh = self.ht_hash_parts(st, sp, hash_str)?;
        self.builder.build_store(hash_a, nh).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(ins_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_next);
        let ndist = self
            .builder
            .build_int_add(self.load_i64(idist_a, "od")?, one, "ndist")
            .map_err(llvm_err)?;
        self.builder.build_store(idist_a, ndist).map_err(llvm_err)?;
        let cap_eq = self
            .builder
            .build_int_compare(IntPredicate::UGE, ndist, new_cap, "ce")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cap_eq, scan_skip, ins_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(scan_skip);
        let nsi = self
            .builder
            .build_int_add(siv, one, "nsi")
            .map_err(llvm_err)?;
        self.builder.build_store(si_a, nsi).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(scan)
            .map_err(llvm_err)?;

        self.builder.position_at_end(scan_done);
        self.builder
            .build_return(Some(&new_data))
            .map_err(llvm_err)?;

        let _ = (seq_fn, old_data);
        Ok(())
    }

    fn define_ht_from_list(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let ht_create = self.module.get_function("action_ht_create").unwrap();
        let ht_insert = self.module.get_function("action_ht_insert").unwrap();
        let list_len_fn = self.module.get_function("action_list_len").unwrap();
        let list_get_fn = self.module.get_function("action_list_get").unwrap();
        let null_val: BasicValueEnum = {
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
        let len = len_cc.try_as_basic_value().unwrap_basic().into_int_value();
        let set_cc = self
            .builder
            .build_call(ht_create, &[len.into()], "set")
            .map_err(llvm_err)?;
        let set0 = set_cc.try_as_basic_value().unwrap_basic();
        let set_a = self
            .builder
            .build_alloca(self.list_type, "sa")
            .map_err(llvm_err)?;
        self.builder.build_store(set_a, set0).map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(i_a, zero).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(llvm_err)?;

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
        let niv = self
            .builder
            .build_int_add(iv, one, "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, niv).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(loop_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(done);
        let ret = self
            .builder
            .build_load(self.list_type, set_a, "ret")
            .map_err(llvm_err)?;
        self.builder.build_return(Some(&ret)).map_err(llvm_err)?;
        Ok(())
    }

    fn ht_round_cap_pow2(&self, hint: IntValue<'ctx>) -> Result<IntValue<'ctx>, String> {
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

    fn ht_probe_index(
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

    fn ht_hash_key(
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
    fn ht_hash_parts(
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
    fn ht_fat_from_parts(
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

    fn ht_key_eq_parts(
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

    fn ht_slot_is_empty(
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

    fn ht_key_eq(
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

    fn ht_kp_for_store(
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

    fn ht_grow_table(
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

    fn ht_load_slot(
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

    fn ht_store_slot(
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
    pub(super) fn ht_branch_if_slot_active(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
        active_bb: BasicBlock<'ctx>,
        skip_bb: BasicBlock<'ctx>,
    ) -> Result<(), String> {
        let (kt, kp, vt, vp, _) = self.ht_load_slot(data, slot)?;
        self.ht_branch_if_slot_active_fields(kt, kp, vt, vp, active_bb, skip_bb)
    }

    fn ht_branch_if_slot_active_fields(
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

    fn define_ht_insert(
        &self,
        seq_fn: FunctionValue<'ctx>,
        _memcpy_fn: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let tomb = i64.const_int(Self::HT_TOMBSTONE, false);
        let neg1 = i64.const_all_ones();
        let hash_str = self.module.get_function("action_ht_hash_str").unwrap();
        let load_num = i64.const_int(Self::HT_LOAD_NUM, false);
        let load_den = i64.const_int(Self::HT_LOAD_DEN, false);

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
        let grow_ck = self.context.append_basic_block(f, "grow_ck");
        let grow = self.context.append_basic_block(f, "grow");
        let rh_chk = self.context.append_basic_block(f, "rh_chk");
        let rh_empty = self.context.append_basic_block(f, "rh_empty");
        let rh_tomb = self.context.append_basic_block(f, "rh_tomb");
        let rh_key = self.context.append_basic_block(f, "rh_key");
        let rh_swap_chk = self.context.append_basic_block(f, "rh_swap_chk");
        let rh_swap = self.context.append_basic_block(f, "rh_swap");
        let rh_next = self.context.append_basic_block(f, "rh_next");
        let rh_fail = self.context.append_basic_block(f, "rh_fail");
        let update = self.context.append_basic_block(f, "update");
        let tomb_record = self.context.append_basic_block(f, "tomb_record");
        let tomb_cont = self.context.append_basic_block(f, "tomb_cont");

        self.builder.position_at_end(entry);
        let map = f.get_first_param().unwrap().into_struct_value();
        let key = f.get_nth_param(1).unwrap().into_struct_value();
        let val = f.get_nth_param(2).unwrap().into_struct_value();
        let data0 = self.extract_ptr(map, 0, "d")?;
        let len0 = self.extract_int(map, 1, "l")?;
        let cap0 = self.extract_int(map, 2, "c")?;
        let kt = self.extract_int(key, 0, "kt")?;
        let kp = self.extract_ptr(key, 1, "kp")?;
        let kpi = self
            .builder
            .build_ptr_to_int(kp, i64, "kpi")
            .map_err(llvm_err)?;
        let vt = self.extract_int(val, 0, "vt")?;
        let vp = self.extract_ptr(val, 1, "vp")?;
        let vpi = self
            .builder
            .build_ptr_to_int(vp, i64, "vpi")
            .map_err(llvm_err)?;
        let skp = self.ht_kp_for_store(kt, kpi, vt, vpi)?;

        let data = self.ht_cow(data0, cap0, entry, cow, merge)?;

        self.builder.position_at_end(merge);
        let hash = self.ht_hash_key(key, hash_str)?;
        let data_a = self.builder.build_alloca(ptr, "da").map_err(llvm_err)?;
        let cap_a = self.builder.build_alloca(i64, "ca").map_err(llvm_err)?;
        let ikt_a = self.builder.build_alloca(i64, "ikt").map_err(llvm_err)?;
        let ikp_a = self.builder.build_alloca(i64, "ikp").map_err(llvm_err)?;
        let ivt_a = self.builder.build_alloca(i64, "ivt").map_err(llvm_err)?;
        let ivp_a = self.builder.build_alloca(i64, "ivp").map_err(llvm_err)?;
        let idist_a = self.builder.build_alloca(i64, "idist").map_err(llvm_err)?;
        let hash_a = self.builder.build_alloca(i64, "hash").map_err(llvm_err)?;
        let ft_a = self.builder.build_alloca(i64, "ft").map_err(llvm_err)?;
        self.builder.build_store(data_a, data).map_err(llvm_err)?;
        self.builder.build_store(cap_a, cap0).map_err(llvm_err)?;
        self.builder.build_store(ikt_a, kt).map_err(llvm_err)?;
        self.builder.build_store(ikp_a, skp).map_err(llvm_err)?;
        self.builder.build_store(ivt_a, vt).map_err(llvm_err)?;
        self.builder.build_store(ivp_a, vpi).map_err(llvm_err)?;
        self.builder.build_store(idist_a, zero).map_err(llvm_err)?;
        self.builder.build_store(hash_a, hash).map_err(llvm_err)?;
        self.builder.build_store(ft_a, neg1).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(grow_ck)
            .map_err(llvm_err)?;

        self.builder.position_at_end(grow_ck);
        let cur_data = self
            .builder
            .build_load(ptr, data_a, "cd")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cur_cap = self.load_i64(cap_a, "cc")?;
        let nl_est = self
            .builder
            .build_int_add(len0, one, "nle")
            .map_err(llvm_err)?;
        let lhs = self
            .builder
            .build_int_mul(nl_est, load_den, "lhs")
            .map_err(llvm_err)?;
        let rhs = self
            .builder
            .build_int_mul(cur_cap, load_num, "rhs")
            .map_err(llvm_err)?;
        let need_grow = self
            .builder
            .build_int_compare(IntPredicate::UGT, lhs, rhs, "ng")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(need_grow, grow, rh_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(grow);
        let (gd, gc) = self.ht_grow_table(cur_data, cur_cap)?;
        self.builder.build_store(data_a, gd).map_err(llvm_err)?;
        self.builder.build_store(cap_a, gc).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(rh_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rh_chk);
        let data_w = self
            .builder
            .build_load(ptr, data_a, "dw")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cap_w = self.load_i64(cap_a, "cw")?;
        let cdist = self.load_i64(idist_a, "cdist")?;
        let chash = self.load_i64(hash_a, "chash")?;
        let idx = self.ht_probe_index(chash, cdist, cap_w)?;
        let (st, sp, svt, svp, sdist) = self.ht_load_slot(data_w, idx)?;
        let is_empty = self.ht_slot_is_empty(st, sp, svt, svp)?;
        self.builder
            .build_conditional_branch(is_empty, rh_empty, rh_tomb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rh_tomb);
        let is_tomb = self
            .builder
            .build_int_compare(IntPredicate::EQ, sp, tomb, "tomb")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(is_tomb, tomb_record, rh_key)
            .map_err(llvm_err)?;

        self.builder.position_at_end(tomb_record);
        let ftv = self.load_i64(ft_a, "ftv")?;
        let ft_unset = self
            .builder
            .build_int_compare(IntPredicate::EQ, ftv, neg1, "ftu")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(ft_unset, tomb_cont, rh_next)
            .map_err(llvm_err)?;
        self.builder.position_at_end(tomb_cont);
        self.builder.build_store(ft_a, idx).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(rh_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rh_key);
        let qkt = self.load_i64(ikt_a, "qkt")?;
        let qkp = self.load_i64(ikp_a, "qkp")?;
        let feq = self.ht_key_eq_parts(st, sp, qkt, qkp, seq_fn)?;
        self.builder
            .build_conditional_branch(feq, update, rh_swap_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rh_swap_chk);
        let steal = self
            .builder
            .build_int_compare(IntPredicate::UGT, cdist, sdist, "steal")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(steal, rh_swap, rh_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rh_swap);
        let ikt = self.load_i64(ikt_a, "ikt_v")?;
        let ikp = self.load_i64(ikp_a, "ikp_v")?;
        let ivt = self.load_i64(ivt_a, "ivt_v")?;
        let ivp = self.load_i64(ivp_a, "ivp_v")?;
        self.ht_store_slot(data_w, idx, ikt, ikp, ivt, ivp, cdist)?;
        self.builder.build_store(ikt_a, st).map_err(llvm_err)?;
        self.builder.build_store(ikp_a, sp).map_err(llvm_err)?;
        self.builder.build_store(ivt_a, svt).map_err(llvm_err)?;
        self.builder.build_store(ivp_a, svp).map_err(llvm_err)?;
        self.builder.build_store(idist_a, sdist).map_err(llvm_err)?;
        let nh = self.ht_hash_parts(st, sp, hash_str)?;
        self.builder.build_store(hash_a, nh).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(rh_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rh_next);
        let ndist = self
            .builder
            .build_int_add(self.load_i64(idist_a, "od")?, one, "ndist")
            .map_err(llvm_err)?;
        self.builder.build_store(idist_a, ndist).map_err(llvm_err)?;
        let cap_eq = self
            .builder
            .build_int_compare(IntPredicate::UGE, ndist, cap_w, "ce")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(cap_eq, rh_fail, rh_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(update);
        self.ht_store_slot(data_w, idx, st, sp, vt, vpi, sdist)?;
        let ru = self.ht_pack(data_w, len0, cap_w)?;
        self.builder.build_return(Some(&ru)).map_err(llvm_err)?;

        self.builder.position_at_end(rh_empty);
        let ftv2 = self.load_i64(ft_a, "ftv2")?;
        let use_ft = self
            .builder
            .build_int_compare(IntPredicate::NE, ftv2, neg1, "uft")
            .map_err(llvm_err)?;
        let iidx = self
            .builder
            .build_select(use_ft, ftv2, idx, "ins_idx")
            .map_err(llvm_err)?
            .into_int_value();
        let ikt2 = self.load_i64(ikt_a, "ikt2")?;
        let ikp2 = self.load_i64(ikp_a, "ikp2")?;
        let ivt2 = self.load_i64(ivt_a, "ivt2")?;
        let ivp2 = self.load_i64(ivp_a, "ivp2")?;
        let cdist2 = self.load_i64(idist_a, "cdist2")?;
        self.ht_store_slot(data_w, iidx, ikt2, ikp2, ivt2, ivp2, cdist2)?;
        let nl = self
            .builder
            .build_int_add(len0, one, "nl")
            .map_err(llvm_err)?;
        let ri = self.ht_pack(data_w, nl, cap_w)?;
        self.builder.build_return(Some(&ri)).map_err(llvm_err)?;

        self.builder.position_at_end(rh_fail);
        let rf = self.ht_pack(data_w, len0, cap_w)?;
        self.builder.build_return(Some(&rf)).map_err(llvm_err)?;

        Ok(())
    }

    fn define_ht_get_contains(&self, seq_fn: FunctionValue<'ctx>) -> Result<(), String> {
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

    fn define_ht_remove(
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

    fn define_ht_rc_dec(
        &self,
        rc_dec_fn: FunctionValue<'ctx>,
        free_fn: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let tomb = i64.const_int(Self::HT_TOMBSTONE, false);
        let marker = i64.const_int(Self::HT_SCALAR_MARKER, false);

        let f = self.module.add_function(
            "action_rc_dec_ht",
            self.void_ty()
                .fn_type(&[ptr.into(), i64.into(), i64.into()], false),
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
        let clskip = self.context.append_basic_block(f, "clskip");
        let free_bb = self.context.append_basic_block(f, "free");

        self.builder.position_at_end(entry);
        let data = f.get_first_param().unwrap().into_pointer_value();
        let cap = f.get_nth_param(1).unwrap().into_int_value();
        let _len = f.get_nth_param(2).unwrap().into_int_value();
        let is_null = self.builder.build_is_null(data, "n").map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(is_null, null_done, do_dec)
            .map_err(llvm_err)?;
        self.builder.position_at_end(null_done);
        self.builder.build_return(None).map_err(llvm_err)?;

        self.builder.position_at_end(do_dec);
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
        let nrc = self
            .builder
            .build_int_sub(rc, one, "nrc")
            .map_err(llvm_err)?;
        self.builder.build_store(rc_p, nrc).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(chk)
            .map_err(llvm_err)?;

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
        self.builder
            .build_unconditional_branch(clp)
            .map_err(llvm_err)?;

        self.builder.position_at_end(clp);
        let siv = self.load_i64(si, "siv")?;
        let sc = self
            .builder
            .build_int_compare(IntPredicate::SLT, siv, cap, "sc")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(sc, clb, free_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(clb);
        let (kt, kp, svt, vp, _) = self.ht_load_slot(data, siv)?;
        let kp0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kp, zero, "kp0")
            .map_err(llvm_err)?;
        let kt0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, kt, zero, "kt0")
            .map_err(llvm_err)?;
        let svt0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, svt, zero, "svt0")
            .map_err(llvm_err)?;
        let vp0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, vp, zero, "vp0")
            .map_err(llvm_err)?;
        let e12 = self.builder.build_and(kp0, kt0, "e12").map_err(llvm_err)?;
        let e34 = self.builder.build_and(svt0, vp0, "e34").map_err(llvm_err)?;
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
        let kp_bb = self.context.append_basic_block(f, "kpdec");
        let vp_bb = self.context.append_basic_block(f, "vpchk");
        self.builder
            .build_conditional_branch(inactive, clskip, kp_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(kp_bb);
        let kp_ne0 = self
            .builder
            .build_int_compare(IntPredicate::NE, kp, zero, "kpne0")
            .map_err(llvm_err)?;
        let kp_nem = self
            .builder
            .build_int_compare(IntPredicate::NE, kp, marker, "kpnem")
            .map_err(llvm_err)?;
        let kp_rc = self
            .builder
            .build_and(kp_ne0, kp_nem, "kprc")
            .map_err(llvm_err)?;
        let kp_dec = self.context.append_basic_block(f, "kpd");
        self.builder
            .build_conditional_branch(kp_rc, kp_dec, vp_bb)
            .map_err(llvm_err)?;
        self.builder.position_at_end(kp_dec);
        let _ = self.builder.build_call(
            rc_dec_fn,
            &[self
                .builder
                .build_int_to_ptr(kp, ptr, "kpp")
                .map_err(llvm_err)?
                .into()],
            "",
        );
        self.builder
            .build_unconditional_branch(vp_bb)
            .map_err(llvm_err)?;

        self.builder.position_at_end(vp_bb);
        let vp_ok = self
            .builder
            .build_int_compare(IntPredicate::NE, vp, zero, "vpo")
            .map_err(llvm_err)?;
        let vp_dec = self.context.append_basic_block(f, "vpd");
        self.builder
            .build_conditional_branch(vp_ok, vp_dec, clskip)
            .map_err(llvm_err)?;
        self.builder.position_at_end(vp_dec);
        let _ = self.builder.build_call(
            rc_dec_fn,
            &[self
                .builder
                .build_int_to_ptr(vp, ptr, "vpp")
                .map_err(llvm_err)?
                .into()],
            "",
        );
        self.builder
            .build_unconditional_branch(clskip)
            .map_err(llvm_err)?;

        self.builder.position_at_end(clskip);
        let nsi = self
            .builder
            .build_int_add(siv, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(si, nsi).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(clp)
            .map_err(llvm_err)?;

        self.builder.position_at_end(free_bb);
        let _ = self.builder.build_call(
            free_fn,
            &[self
                .builder
                .build_int_to_ptr(rc_a, ptr, "fp")
                .map_err(llvm_err)?
                .into()],
            "",
        );
        self.builder.build_return(None).map_err(llvm_err)?;

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

    /// Load key as fat struct from slot index; normalizes scalar marker kp 1 -> 0.
    pub(super) fn ht_key_fat_at(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let zero = i64.const_int(0, false);
        let marker = i64.const_int(Self::HT_SCALAR_MARKER, false);
        let (kt, kp, _, _, _) = self.ht_load_slot(data, slot)?;
        let is_mark = self
            .builder
            .build_int_compare(IntPredicate::EQ, kp, marker, "mk")
            .map_err(llvm_err)?;
        let norm_kp = self
            .builder
            .build_select(is_mark, zero, kp, "nkp")
            .map_err(llvm_err)?
            .into_int_value();
        let kp_p = self
            .builder
            .build_int_to_ptr(norm_kp, ptr, "kp")
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

    /// Load value as fat struct from slot index.
    pub(super) fn ht_val_fat_at(
        &self,
        data: PointerValue<'ctx>,
        slot: IntValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let (_, _, vt, vp, _) = self.ht_load_slot(data, slot)?;
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
