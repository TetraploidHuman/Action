use crate::{llvm_err, CodeGen};
use inkwell::values::FunctionValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn define_ht_insert(
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
        let grow = self.context.append_basic_block(f, "grow");
        let rh_empty_ins = self.context.append_basic_block(f, "rh_empty_ins");
        let rh_chk = self.context.append_basic_block(f, "rh_chk");
        let rh_empty = self.context.append_basic_block(f, "rh_empty");
        let rh_tomb = self.context.append_basic_block(f, "rh_tomb");
        let rh_key = self.context.append_basic_block(f, "rh_key");
        let rh_swap_chk = self.context.append_basic_block(f, "rh_swap_chk");
        let rh_swap = self.context.append_basic_block(f, "rh_swap");
        let rh_next = self.context.append_basic_block(f, "rh_next");
        let rh_fail = self.context.append_basic_block(f, "rh_fail");
        let rh_fail_ret = self.context.append_basic_block(f, "rh_fail_ret");
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
        let cur_cap_g = self.load_i64(cap_a, "cc_g")?;
        let nl_est = self
            .builder
            .build_int_add(len0, one, "nle")
            .map_err(llvm_err)?;
        let lhs_g = self
            .builder
            .build_int_mul(nl_est, load_den, "lhs_g")
            .map_err(llvm_err)?;
        let rhs_g = self
            .builder
            .build_int_mul(cur_cap_g, load_num, "rhs_g")
            .map_err(llvm_err)?;
        let need_grow = self
            .builder
            .build_int_compare(IntPredicate::UGT, lhs_g, rhs_g, "ng")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(need_grow, grow, rh_empty_ins)
            .map_err(llvm_err)?;

        self.builder.position_at_end(grow);
        let cur_data = self
            .builder
            .build_load(ptr, data_a, "cd_gr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cur_cap = self.load_i64(cap_a, "cc_gr")?;
        let (gd, gc) = self.ht_grow_table(cur_data, cur_cap)?;
        self.builder.build_store(data_a, gd).map_err(llvm_err)?;
        self.builder.build_store(cap_a, gc).map_err(llvm_err)?;
        self.builder.build_store(idist_a, zero).map_err(llvm_err)?;
        self.builder.build_store(ft_a, neg1).map_err(llvm_err)?;
        self.builder
            .build_unconditional_branch(rh_chk)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rh_empty_ins);
        let data_w = self
            .builder
            .build_load(ptr, data_a, "dw_ei")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cap_w = self.load_i64(cap_a, "cw_ei")?;
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
        let cur_cap_f = self.load_i64(cap_a, "cc_f")?;
        let nl_est_f = self
            .builder
            .build_int_add(len0, one, "nle_f")
            .map_err(llvm_err)?;
        let lhs_f = self
            .builder
            .build_int_mul(nl_est_f, load_den, "lhs_f")
            .map_err(llvm_err)?;
        let rhs_f = self
            .builder
            .build_int_mul(cur_cap_f, load_num, "rhs_f")
            .map_err(llvm_err)?;
        let need_grow_f = self
            .builder
            .build_int_compare(IntPredicate::UGT, lhs_f, rhs_f, "ng_f")
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(need_grow_f, grow, rh_fail_ret)
            .map_err(llvm_err)?;

        self.builder.position_at_end(rh_fail_ret);
        let data_f = self
            .builder
            .build_load(ptr, data_a, "dw_f")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cap_f = self.load_i64(cap_a, "cap_f")?;
        let rf = self.ht_pack(data_f, len0, cap_f)?;
        self.builder.build_return(Some(&rf)).map_err(llvm_err)?;

        Ok(())
    }
}
