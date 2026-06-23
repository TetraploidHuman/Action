use crate::{llvm_err, CodeGen};
use inkwell::values::FunctionValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn define_ht_hash_str(&self) -> Result<(), String> {
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
    pub(crate) fn define_ht_rehash(
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

    /// Copy active Robin-Hood slots from `src` into `dest` (must be empty or partial).
    /// `dest_len_p` points to occupied count; incremented only on new keys (overwrites keep len).
    pub(crate) fn define_ht_bulk_copy_active_slots(
        &self,
        seq_fn: FunctionValue<'ctx>,
    ) -> Result<(), String> {
        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let tomb = i64.const_int(Self::HT_TOMBSTONE, false);
        let hash_str = self.module.get_function("action_ht_hash_str").unwrap();

        let f = self.module.add_function(
            "action_ht_bulk_copy_active_slots",
            self.void_ty().fn_type(
                &[ptr.into(), i64.into(), ptr.into(), ptr.into(), i64.into()],
                false,
            ),
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
        let ins_key_chk = self.context.append_basic_block(f, "ins_key_chk");
        let ins_update = self.context.append_basic_block(f, "ins_update");
        let ins_swap_chk = self.context.append_basic_block(f, "ins_swap_chk");
        let ins_swap_body = self.context.append_basic_block(f, "ins_swap_body");
        let ins_next = self.context.append_basic_block(f, "ins_next");

        self.builder.position_at_end(entry);
        let dest_data = f.get_first_param().unwrap().into_pointer_value();
        let dest_cap = f.get_nth_param(1).unwrap().into_int_value();
        let dest_len_p = f.get_nth_param(2).unwrap().into_pointer_value();
        let src_data = f.get_nth_param(3).unwrap().into_pointer_value();
        let src_cap = f.get_nth_param(4).unwrap().into_int_value();
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
            .build_int_compare(IntPredicate::SLT, siv, src_cap, "sc")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(scan_cond, scan_body, scan_done)
            .map_err(llvm_err)?;

        self.builder.position_at_end(scan_body);
        let (ekt, ekp, evt, evp, _) = self.ht_load_slot(src_data, siv)?;
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
        let idx = self.ht_probe_index(chash, cdist, dest_cap)?;
        let (st, sp, svt, svp, sdist) = self.ht_load_slot(dest_data, idx)?;
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
            .build_conditional_branch(can_ins, ins_store, ins_key_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_store);
        let ikt = self.load_i64(ikt_a, "ikt")?;
        let ikp = self.load_i64(ikp_a, "ikp")?;
        let ivt = self.load_i64(ivt_a, "ivt")?;
        let ivp = self.load_i64(ivp_a, "ivp")?;
        self.ht_store_slot(dest_data, idx, ikt, ikp, ivt, ivp, cdist)?;
        let cur_len = self
            .builder
            .build_load(i64, dest_len_p, "cl")
            .map_err(llvm_err)?
            .into_int_value();
        let new_len = self
            .builder
            .build_int_add(cur_len, one, "nl")
            .map_err(llvm_err)?;
        self.builder
            .build_store(dest_len_p, new_len)
            .map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(scan_skip)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_key_chk);
        let qkt = self.load_i64(ikt_a, "qkt")?;
        let qkp = self.load_i64(ikp_a, "qkp")?;
        let feq = self.ht_key_eq_parts(st, sp, qkt, qkp, seq_fn)?;
        self.builder
            .build_conditional_branch(feq, ins_update, ins_swap_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_swap_chk);
        let steal = self
            .builder
            .build_int_compare(IntPredicate::UGT, cdist, sdist, "steal")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(steal, ins_swap_body, ins_next)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_update);
        let ivt_u = self.load_i64(ivt_a, "ivtu")?;
        let ivp_u = self.load_i64(ivp_a, "ivpu")?;
        self.ht_store_slot(dest_data, idx, st, sp, ivt_u, ivp_u, sdist)?;
        self.builder
            .build_unconditional_branch(scan_skip)
            .map_err(llvm_err)?;

        self.builder.position_at_end(ins_swap_body);
        let ikt_v = self.load_i64(ikt_a, "ikt_v")?;
        let ikp_v = self.load_i64(ikp_a, "ikp_v")?;
        let ivt_v = self.load_i64(ivt_a, "ivt_v")?;
        let ivp_v = self.load_i64(ivp_a, "ivp_v")?;
        self.ht_store_slot(dest_data, idx, ikt_v, ikp_v, ivt_v, ivp_v, cdist)?;
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
            .build_int_compare(IntPredicate::UGE, ndist, dest_cap, "ce")
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
        self.builder.build_return(None).map_err(llvm_err)?;

        let _ = (seq_fn, dest_data, src_data);
        Ok(())
    }
}
