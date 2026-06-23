// Submodule: runtime_decl/str_adv/replace (R6)

use crate::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_replace(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let str_ty = self.string_type;
        let ptr = self.ptr_ty();
        let _b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let memcmp_fn = self.module.get_function("memcmp").unwrap();
        let memcpy_fn = self.module.get_function("memcpy").unwrap();
        let str_data_fn = self.module.get_function("action_string_data").unwrap();

        // ---- action_string_replace({i64, ptr}, {i64, ptr}, {i64, ptr}) -> {i64, ptr} ----
        let rp_fn = self.module.add_function(
            "action_string_replace",
            str_ty.fn_type(&[str_ty.into(), str_ty.into(), str_ty.into()], false),
            None,
        );
        let rp_entry = self.context.append_basic_block(rp_fn, "entry");
        self.builder.position_at_end(rp_entry);
        let rp_s = rp_fn.get_first_param().unwrap().into_struct_value();
        let rp_from = rp_fn.get_nth_param(1).unwrap().into_struct_value();
        let rp_to = rp_fn.get_nth_param(2).unwrap().into_struct_value();
        let rp_slen = self
            .builder
            .build_extract_value(rp_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_sdata_cc = self
            .builder
            .build_call(str_data_fn, &[rp_s.into()], "sd")
            .map_err(llvm_err)?;
        let rp_sdata = rp_sdata_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let rp_flen = self
            .builder
            .build_extract_value(rp_from, 0, "flen")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_fdata_cc = self
            .builder
            .build_call(str_data_fn, &[rp_from.into()], "fd")
            .map_err(llvm_err)?;
        let rp_fdata = rp_fdata_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let rp_tlen = self
            .builder
            .build_extract_value(rp_to, 0, "tlen")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_tdata_cc = self
            .builder
            .build_call(str_data_fn, &[rp_to.into()], "td")
            .map_err(llvm_err)?;
        let rp_tdata = rp_tdata_cc
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();

        // If from is empty, return copy of original
        let rp_fzero = self
            .builder
            .build_int_compare(IntPredicate::EQ, rp_flen, i64.const_int(0, false), "fzero")
            .map_err(llvm_err)?;
        let rp_have_from = self.context.append_basic_block(rp_fn, "have_from");
        let rp_copy_ret = self.context.append_basic_block(rp_fn, "copy_ret");
        let _ = self
            .builder
            .build_conditional_branch(rp_fzero, rp_copy_ret, rp_have_from);

        // Copy return: just duplicate the original string
        self.builder.position_at_end(rp_copy_ret);
        let rp_calc = self
            .builder
            .build_int_add(rp_slen, i64.const_int(1, false), "calc")
            .map_err(llvm_err)?;
        let rp_cbuf = self
            .builder
            .build_call(malloc_rc_fn, &[rp_calc.into()], "cbuf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for this newly allocated buffer (malloc_rc starts at 0)
        let rp_cbuf_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(rp_cbuf, i64, "rp_cbuf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "rp_cbuf_rc_addr",
            )
            .map_err(llvm_err)?;
        let rp_cbuf_rc_p = self
            .builder
            .build_int_to_ptr(rp_cbuf_rc_addr, ptr, "rp_cbuf_rc_p")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_cbuf_rc_p, i64.const_int(1, false))
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[rp_cbuf.into(), rp_sdata.into(), rp_slen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let rp_cnull = unsafe {
            self.builder
                .build_gep(i8, rp_cbuf, &[rp_slen], "cnull")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(rp_cnull, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let rp_cund = str_ty.get_undef();
        let rp_cr1 = self
            .builder
            .build_insert_value(rp_cund, rp_slen, 0, "cr1")
            .map_err(llvm_err)?;
        let rp_cr2 = self
            .builder
            .build_insert_value(rp_cr1, rp_cbuf, 1, "cr2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&rp_cr2));

        // have_from: count occurrences and compute result size
        self.builder.position_at_end(rp_have_from);
        let rp_ri = self.builder.build_alloca(i64, "ri").map_err(llvm_err)?;
        let rp_rlast = self.builder.build_alloca(i64, "rlast").map_err(llvm_err)?;
        let rp_count = self.builder.build_alloca(i64, "rcount").map_err(llvm_err)?;
        self.builder
            .build_store(rp_ri, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_rlast, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_count, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let rp_hdr = self.context.append_basic_block(rp_fn, "hdr");
        let rp_body = self.context.append_basic_block(rp_fn, "body");
        let rp_ck = self.context.append_basic_block(rp_fn, "ck");
        let rp_nxt = self.context.append_basic_block(rp_fn, "nxt");
        let rp_build = self.context.append_basic_block(rp_fn, "build");
        let _ = self.builder.build_unconditional_branch(rp_hdr);

        // Scan loop: find matches, count them
        self.builder.position_at_end(rp_hdr);
        let rp_riv = self
            .builder
            .build_load(i64, rp_ri, "riv")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_end = self
            .builder
            .build_int_add(rp_riv, rp_flen, "end")
            .map_err(llvm_err)?;
        let rp_ok = self
            .builder
            .build_int_compare(IntPredicate::ULE, rp_end, rp_slen, "ok")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rp_ok, rp_body, rp_build);

        self.builder.position_at_end(rp_body);
        let rp_rsrc = unsafe {
            self.builder
                .build_gep(i8, rp_sdata, &[rp_riv], "rsrc")
                .map_err(llvm_err)
        }?;
        let rp_rmc = self
            .builder
            .build_call(
                memcmp_fn,
                &[rp_rsrc.into(), rp_fdata.into(), rp_flen.into()],
                "rmc",
            )
            .map_err(llvm_err)?;
        let rp_rmcr = rp_rmc.try_as_basic_value().unwrap_basic().into_int_value();
        let rp_rm = self
            .builder
            .build_int_compare(IntPredicate::EQ, rp_rmcr, i32.const_int(0, false), "rm")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(rp_rm, rp_ck, rp_nxt);

        self.builder.position_at_end(rp_ck);
        let rp_rc = self
            .builder
            .build_load(i64, rp_count, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_nc = self
            .builder
            .build_int_add(rp_rc, i64.const_int(1, false), "nc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_count, rp_nc)
            .map_err(llvm_err)?;
        let rp_nri = self
            .builder
            .build_int_add(rp_riv, rp_flen, "nri")
            .map_err(llvm_err)?;
        self.builder.build_store(rp_ri, rp_nri).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_hdr);

        self.builder.position_at_end(rp_nxt);
        let rp_nri2 = self
            .builder
            .build_int_add(rp_riv, i64.const_int(1, false), "nri2")
            .map_err(llvm_err)?;
        self.builder.build_store(rp_ri, rp_nri2).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_hdr);

        // build: allocate and copy with replacements
        self.builder.position_at_end(rp_build);
        let rp_fc = self
            .builder
            .build_load(i64, rp_count, "fc")
            .map_err(llvm_err)?
            .into_int_value();
        // new_len = slen + count * (tlen - flen)
        let rp_diff = self
            .builder
            .build_int_sub(rp_tlen, rp_flen, "diff")
            .map_err(llvm_err)?;
        let rp_extra = self
            .builder
            .build_int_mul(rp_fc, rp_diff, "extra")
            .map_err(llvm_err)?;
        let rp_nlen = self
            .builder
            .build_int_add(rp_slen, rp_extra, "nlen")
            .map_err(llvm_err)?;
        let rp_nalc = self
            .builder
            .build_int_add(rp_nlen, i64.const_int(1, false), "nalc")
            .map_err(llvm_err)?;
        let rp_nbuf = self
            .builder
            .build_call(malloc_rc_fn, &[rp_nalc.into()], "nbuf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Set RC=1 for this newly allocated buffer (malloc_rc starts at 0)
        let rp_nbuf_rc_addr = self
            .builder
            .build_int_sub(
                self.builder
                    .build_ptr_to_int(rp_nbuf, i64, "rp_nbuf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(8, false),
                "rp_nbuf_rc_addr",
            )
            .map_err(llvm_err)?;
        let rp_nbuf_rc_p = self
            .builder
            .build_int_to_ptr(rp_nbuf_rc_addr, ptr, "rp_nbuf_rc_p")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_nbuf_rc_p, i64.const_int(1, false))
            .map_err(llvm_err)?;

        // Reset scan
        self.builder
            .build_store(rp_ri, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_rlast, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let rp_wpos = self.builder.build_alloca(i64, "wpos").map_err(llvm_err)?;
        self.builder
            .build_store(rp_wpos, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let rp_bhdr = self.context.append_basic_block(rp_fn, "bhdr");
        let rp_bbody = self.context.append_basic_block(rp_fn, "bbody");
        let rp_bck = self.context.append_basic_block(rp_fn, "bck");
        let rp_bnxt = self.context.append_basic_block(rp_fn, "bnxt");
        let rp_bfinal = self.context.append_basic_block(rp_fn, "bfinal");
        let rp_bdone = self.context.append_basic_block(rp_fn, "bdone");
        let _ = self.builder.build_unconditional_branch(rp_bhdr);

        self.builder.position_at_end(rp_bhdr);
        let rp_briv = self
            .builder
            .build_load(i64, rp_ri, "briv")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_bend = self
            .builder
            .build_int_add(rp_briv, rp_flen, "bend")
            .map_err(llvm_err)?;
        let rp_bok = self
            .builder
            .build_int_compare(IntPredicate::ULE, rp_bend, rp_slen, "bok")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rp_bok, rp_bbody, rp_bfinal);

        self.builder.position_at_end(rp_bbody);
        let rp_brsrc = unsafe {
            self.builder
                .build_gep(i8, rp_sdata, &[rp_briv], "brsrc")
                .map_err(llvm_err)
        }?;
        let rp_bmc = self
            .builder
            .build_call(
                memcmp_fn,
                &[rp_brsrc.into(), rp_fdata.into(), rp_flen.into()],
                "bmc",
            )
            .map_err(llvm_err)?;
        let rp_bmcr = rp_bmc.try_as_basic_value().unwrap_basic().into_int_value();
        let rp_bm = self
            .builder
            .build_int_compare(IntPredicate::EQ, rp_bmcr, i32.const_int(0, false), "bm")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rp_bm, rp_bck, rp_bnxt);

        // Match found: copy any non-matched part before it, then copy replacement
        self.builder.position_at_end(rp_bck);
        let rp_blast = self
            .builder
            .build_load(i64, rp_rlast, "blast")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_bgap = self
            .builder
            .build_int_sub(rp_briv, rp_blast, "bgap")
            .map_err(llvm_err)?;
        let rp_bwp = self
            .builder
            .build_load(i64, rp_wpos, "bwp")
            .map_err(llvm_err)?
            .into_int_value();
        // Copy gap (non-matched chars before this match)
        let rp_bgsrc = unsafe {
            self.builder
                .build_gep(i8, rp_sdata, &[rp_blast], "bgsrc")
                .map_err(llvm_err)
        }?;
        let rp_bgdst = unsafe {
            self.builder
                .build_gep(i8, rp_nbuf, &[rp_bwp], "bgdst")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[rp_bgdst.into(), rp_bgsrc.into(), rp_bgap.into()],
                "",
            )
            .map_err(llvm_err)?;
        let rp_bnwp1 = self
            .builder
            .build_int_add(rp_bwp, rp_bgap, "bnwp1")
            .map_err(llvm_err)?;
        // Copy replacement
        let rp_brdst = unsafe {
            self.builder
                .build_gep(i8, rp_nbuf, &[rp_bnwp1], "brdst")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[rp_brdst.into(), rp_tdata.into(), rp_tlen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let rp_bnwp2 = self
            .builder
            .build_int_add(rp_bnwp1, rp_tlen, "bnwp2")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_wpos, rp_bnwp2)
            .map_err(llvm_err)?;
        let rp_bnri = self
            .builder
            .build_int_add(rp_briv, rp_flen, "bnri")
            .map_err(llvm_err)?;
        self.builder.build_store(rp_ri, rp_bnri).map_err(llvm_err)?;
        self.builder
            .build_store(rp_rlast, rp_bnri)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_bhdr);

        self.builder.position_at_end(rp_bnxt);
        let rp_bnri2 = self
            .builder
            .build_int_add(rp_briv, i64.const_int(1, false), "bnri2")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_ri, rp_bnri2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_bhdr);

        // Copy remaining after last match
        self.builder.position_at_end(rp_bfinal);
        let rp_blast2 = self
            .builder
            .build_load(i64, rp_rlast, "blast2")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_brem = self
            .builder
            .build_int_sub(rp_slen, rp_blast2, "brem")
            .map_err(llvm_err)?;
        let rp_bwp2 = self
            .builder
            .build_load(i64, rp_wpos, "bwp2")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_brsrc2 = unsafe {
            self.builder
                .build_gep(i8, rp_sdata, &[rp_blast2], "brsrc2")
                .map_err(llvm_err)
        }?;
        let rp_brdst2 = unsafe {
            self.builder
                .build_gep(i8, rp_nbuf, &[rp_bwp2], "brdst2")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[rp_brdst2.into(), rp_brsrc2.into(), rp_brem.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _rp_bnwp3 = self
            .builder
            .build_int_add(rp_bwp2, rp_brem, "bnwp3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_bdone);

        self.builder.position_at_end(rp_bdone);
        let rp_fwpos = self
            .builder
            .build_load(i64, rp_wpos, "fwpos")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_bnull = unsafe {
            self.builder
                .build_gep(i8, rp_nbuf, &[rp_fwpos], "bnull")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(rp_bnull, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let rp_rund = str_ty.get_undef();
        let rp_rr1 = self
            .builder
            .build_insert_value(rp_rund, rp_fwpos, 0, "rr1")
            .map_err(llvm_err)?;
        let rp_rr2 = self
            .builder
            .build_insert_value(rp_rr1, rp_nbuf, 1, "rr2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&rp_rr2));

        Ok(())
    }
}
