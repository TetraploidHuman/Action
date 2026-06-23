use crate::{llvm_err, CodeGen};
use inkwell::values::FunctionValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn define_ht_rc_dec(
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
}
