// R7: list sort runtime (extracted from define_misc)

use super::{llvm_err, CodeGen};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_misc_sort(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let _f64 = self.f64_ty();
        let ptr = self.ptr_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let malloc_fn = self.module.get_function("malloc").unwrap();
        let free_fn = self.module.get_function("free").unwrap();
        let qsort_fn = self.module.get_function("qsort").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        let _list_set_fn = self.module.get_function("action_list_set").unwrap();
        let elem_sz = i64.const_int(16, false);
        let zero_i64 = i64.const_int(0, false);
        let one_i64 = i64.const_int(1, false);
        let saved_pos = self.builder.get_insert_block();

        // ---- action_list_sorted_cmp(ptr a, ptr b) -> i32 (qsort comparator, Int in field 0) ----
        let scmp_fn = self.module.add_function(
            "action_list_sorted_cmp",
            i32.fn_type(&[ptr.into(), ptr.into()], false),
            None,
        );
        let scmp_entry = self.context.append_basic_block(scmp_fn, "entry");
        let scmp_ret_gt = self.context.append_basic_block(scmp_fn, "ret_gt");
        let scmp_chk_lt = self.context.append_basic_block(scmp_fn, "chk_lt");
        let scmp_ret_lt = self.context.append_basic_block(scmp_fn, "ret_lt");
        let scmp_ret_eq = self.context.append_basic_block(scmp_fn, "ret_eq");
        self.builder.position_at_end(scmp_entry);
        let scmp_a = scmp_fn.get_first_param().unwrap().into_pointer_value();
        let scmp_b = scmp_fn.get_nth_param(1).unwrap().into_pointer_value();
        let scmp_ea = self
            .builder
            .build_load(self.string_type, scmp_a, "ea")
            .map_err(llvm_err)?
            .into_struct_value();
        let scmp_eb = self
            .builder
            .build_load(self.string_type, scmp_b, "eb")
            .map_err(llvm_err)?
            .into_struct_value();
        let scmp_va = self
            .builder
            .build_extract_value(scmp_ea, 0, "va")
            .map_err(llvm_err)?
            .into_int_value();
        let scmp_vb = self
            .builder
            .build_extract_value(scmp_eb, 0, "vb")
            .map_err(llvm_err)?
            .into_int_value();
        let scmp_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, scmp_va, scmp_vb, "gt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(scmp_gt, scmp_ret_gt, scmp_chk_lt);
        self.builder.position_at_end(scmp_ret_gt);
        let _ = self.builder.build_return(Some(&i32.const_int(1, false)));
        self.builder.position_at_end(scmp_chk_lt);
        let scmp_lt = self
            .builder
            .build_int_compare(IntPredicate::SLT, scmp_va, scmp_vb, "lt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(scmp_lt, scmp_ret_lt, scmp_ret_eq);
        self.builder.position_at_end(scmp_ret_lt);
        let _ = self
            .builder
            .build_return(Some(&i32.const_int(-1i64 as u64, true)));
        self.builder.position_at_end(scmp_ret_eq);
        let _ = self.builder.build_return(Some(&i32.const_int(0, false)));

        // ---- action_list_sorted_cmp_float: qsort comparator for Float elements ----
        let fcmp_fn = self.module.add_function(
            "action_list_sorted_cmp_float",
            i32.fn_type(&[ptr.into(), ptr.into()], false),
            None,
        );
        let fcmp_entry = self.context.append_basic_block(fcmp_fn, "entry");
        let fcmp_ret_gt = self.context.append_basic_block(fcmp_fn, "ret_gt");
        let fcmp_chk_lt = self.context.append_basic_block(fcmp_fn, "chk_lt");
        let fcmp_ret_lt = self.context.append_basic_block(fcmp_fn, "ret_lt");
        let fcmp_ret_eq = self.context.append_basic_block(fcmp_fn, "ret_eq");
        self.builder.position_at_end(fcmp_entry);
        let fcmp_a = fcmp_fn.get_first_param().unwrap().into_pointer_value();
        let fcmp_b = fcmp_fn.get_nth_param(1).unwrap().into_pointer_value();
        let fcmp_ea = self
            .builder
            .build_load(self.string_type, fcmp_a, "ea")
            .map_err(llvm_err)?
            .into_struct_value();
        let fcmp_eb = self
            .builder
            .build_load(self.string_type, fcmp_b, "eb")
            .map_err(llvm_err)?
            .into_struct_value();
        let fcmp_ba = self
            .builder
            .build_extract_value(fcmp_ea, 0, "ba")
            .map_err(llvm_err)?
            .into_int_value();
        let fcmp_bb = self
            .builder
            .build_extract_value(fcmp_eb, 0, "bb")
            .map_err(llvm_err)?
            .into_int_value();
        let fcmp_fa = self
            .builder
            .build_bit_cast(fcmp_ba, self.f64_ty(), "fa")
            .map_err(llvm_err)?
            .into_float_value();
        let fcmp_fb = self
            .builder
            .build_bit_cast(fcmp_bb, self.f64_ty(), "fb")
            .map_err(llvm_err)?
            .into_float_value();
        let fcmp_gt = self
            .builder
            .build_float_compare(inkwell::FloatPredicate::OGT, fcmp_fa, fcmp_fb, "gt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fcmp_gt, fcmp_ret_gt, fcmp_chk_lt);
        self.builder.position_at_end(fcmp_ret_gt);
        let _ = self.builder.build_return(Some(&i32.const_int(1, false)));
        self.builder.position_at_end(fcmp_chk_lt);
        let fcmp_lt = self
            .builder
            .build_float_compare(inkwell::FloatPredicate::OLT, fcmp_fa, fcmp_fb, "lt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fcmp_lt, fcmp_ret_lt, fcmp_ret_eq);
        self.builder.position_at_end(fcmp_ret_lt);
        let _ = self
            .builder
            .build_return(Some(&i32.const_int(-1i64 as u64, true)));
        self.builder.position_at_end(fcmp_ret_eq);
        let _ = self.builder.build_return(Some(&i32.const_int(0, false)));

        // ---- action_float_bits_gt(i64, i64) -> i1: compare float bit patterns ----
        let fgt_fn = self.module.add_function(
            "action_float_bits_gt",
            self.bool_ty().fn_type(&[i64.into(), i64.into()], false),
            None,
        );
        let fgt_entry = self.context.append_basic_block(fgt_fn, "entry");
        self.builder.position_at_end(fgt_entry);
        let fgt_a = fgt_fn.get_first_param().unwrap().into_int_value();
        let fgt_b = fgt_fn.get_nth_param(1).unwrap().into_int_value();
        let fgt_fa = self
            .builder
            .build_bit_cast(fgt_a, self.f64_ty(), "fa")
            .map_err(llvm_err)?
            .into_float_value();
        let fgt_fb = self
            .builder
            .build_bit_cast(fgt_b, self.f64_ty(), "fb")
            .map_err(llvm_err)?
            .into_float_value();
        let fgt_res = self
            .builder
            .build_float_compare(inkwell::FloatPredicate::OGT, fgt_fa, fgt_fb, "gt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fgt_res));

        // ---- action_list_sorted({ptr, i64, i64}) -> {ptr, i64, i64} (Int default) ----
        let srt_fn = self.module.add_function(
            "action_list_sorted",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let srt_entry = self.context.append_basic_block(srt_fn, "entry");
        self.builder.position_at_end(srt_entry);
        let srt_in = srt_fn.get_first_param().unwrap().into_struct_value();
        let srt_len = self
            .builder
            .build_extract_value(srt_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        // Copy input
        let srt_copy = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let srt_copyv = srt_copy.try_as_basic_value().unwrap_basic();
        let srt_ra = self
            .builder
            .build_alloca(self.list_type, "srt_ra")
            .map_err(llvm_err)?;
        self.builder
            .build_store(srt_ra, srt_copyv)
            .map_err(llvm_err)?;
        let srt_ci = self.builder.build_alloca(i64, "srt_ci").map_err(llvm_err)?;
        self.builder
            .build_store(srt_ci, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let srt_cloop = self.context.append_basic_block(srt_fn, "cloop");
        let srt_cbody = self.context.append_basic_block(srt_fn, "cbody");
        let srt_cdone = self.context.append_basic_block(srt_fn, "cdone");
        let _ = self.builder.build_unconditional_branch(srt_cloop);
        self.builder.position_at_end(srt_cloop);
        let srt_civ = self
            .builder
            .build_load(i64, srt_ci, "civ")
            .map_err(llvm_err)?
            .into_int_value();
        let srt_ccond = self
            .builder
            .build_int_compare(IntPredicate::SLT, srt_civ, srt_len, "ccond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(srt_ccond, srt_cbody, srt_cdone);
        self.builder.position_at_end(srt_cbody);
        let srt_get_fn = self.module.get_function("action_list_get").unwrap();
        let srt_cev = self
            .builder
            .build_call(srt_get_fn, &[srt_in.into(), srt_civ.into()], "cev")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let srt_ccl = self
            .builder
            .build_load(self.list_type, srt_ra, "ccl")
            .map_err(llvm_err)?
            .into_struct_value();
        let srt_cps = self.call_rt(
            "action_list_push",
            &[srt_ccl.into(), srt_cev.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(srt_ra, srt_cps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let srt_cinc = self
            .builder
            .build_int_add(srt_civ, i64.const_int(1, false), "cinc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(srt_ci, srt_cinc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(srt_cloop);
        // O(n log n) sort via qsort; height==0 fast path sorts B-tree leaf in place
        self.builder.position_at_end(srt_cdone);
        let srt_two = i64.const_int(2, false);
        let srt_le1 = self.context.append_basic_block(srt_fn, "le1");
        let srt_sort = self.context.append_basic_block(srt_fn, "sort");
        let srt_leaf_qsort = self.context.append_basic_block(srt_fn, "leaf_qsort");
        let srt_flat_qsort = self.context.append_basic_block(srt_fn, "flat_qsort");
        let srt_ret = self.context.append_basic_block(srt_fn, "ret");
        let srt_len_le1 = self
            .builder
            .build_int_compare(IntPredicate::SLT, srt_len, srt_two, "len_le1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(srt_len_le1, srt_le1, srt_sort);
        self.builder.position_at_end(srt_le1);
        let _ = self.builder.build_unconditional_branch(srt_ret);
        self.builder.position_at_end(srt_sort);
        let srt_copy_list = self
            .builder
            .build_load(self.list_type, srt_ra, "copy_list")
            .map_err(llvm_err)?
            .into_struct_value();
        let srt_height = self
            .builder
            .build_extract_value(srt_copy_list, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let srt_h0 = self
            .builder
            .build_int_compare(IntPredicate::EQ, srt_height, zero_i64, "h0")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(srt_h0, srt_leaf_qsort, srt_flat_qsort);
        // Single-leaf list: qsort element array in place (max 64 elems per leaf)
        self.builder.position_at_end(srt_leaf_qsort);
        let srt_leaf_node = self
            .builder
            .build_extract_value(srt_copy_list, 0, "leaf_node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let srt_leaf_i8 = self
            .builder
            .build_pointer_cast(srt_leaf_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let srt_elem_base = unsafe {
            self.builder
                .build_gep(i8, srt_leaf_i8, &[i64.const_int(8, false)], "elem_base")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_call(
            qsort_fn,
            &[
                srt_elem_base.into(),
                srt_len.into(),
                elem_sz.into(),
                scmp_fn.as_global_value().as_pointer_value().into(),
            ],
            "",
        );
        let _ = self.builder.build_unconditional_branch(srt_ret);
        // Multi-level tree: extract to flat buffer, qsort, write back via list_set
        self.builder.position_at_end(srt_flat_qsort);
        let srt_bytes = self
            .builder
            .build_int_mul(srt_len, elem_sz, "bytes")
            .map_err(llvm_err)?;
        let srt_arr = self
            .builder
            .build_call(malloc_fn, &[srt_bytes.into()], "arr")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let srt_fi = self.builder.build_alloca(i64, "srt_fi").map_err(llvm_err)?;
        self.builder
            .build_store(srt_fi, zero_i64)
            .map_err(llvm_err)?;
        let srt_floop = self.context.append_basic_block(srt_fn, "floop");
        let srt_fbody = self.context.append_basic_block(srt_fn, "fbody");
        let srt_fdone = self.context.append_basic_block(srt_fn, "fdone");
        let _ = self.builder.build_unconditional_branch(srt_floop);
        self.builder.position_at_end(srt_floop);
        let srt_fiv = self
            .builder
            .build_load(i64, srt_fi, "fiv")
            .map_err(llvm_err)?
            .into_int_value();
        let srt_fcond = self
            .builder
            .build_int_compare(IntPredicate::SLT, srt_fiv, srt_len, "fcond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(srt_fcond, srt_fbody, srt_fdone);
        self.builder.position_at_end(srt_fbody);
        let srt_fl = self
            .builder
            .build_load(self.list_type, srt_ra, "fl")
            .map_err(llvm_err)?
            .into_struct_value();
        let srt_fev = self
            .builder
            .build_call(_list_get_fn, &[srt_fl.into(), srt_fiv.into()], "fev")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let srt_slot = unsafe {
            self.builder
                .build_gep(self.string_type, srt_arr, &[srt_fiv], "slot")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(srt_slot, srt_fev)
            .map_err(llvm_err)?;
        let srt_finc = self
            .builder
            .build_int_add(srt_fiv, one_i64, "finc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(srt_fi, srt_finc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(srt_floop);
        self.builder.position_at_end(srt_fdone);
        let _ = self.builder.build_call(
            qsort_fn,
            &[
                srt_arr.into(),
                srt_len.into(),
                elem_sz.into(),
                scmp_fn.as_global_value().as_pointer_value().into(),
            ],
            "",
        );
        let srt_wi = self.builder.build_alloca(i64, "srt_wi").map_err(llvm_err)?;
        self.builder
            .build_store(srt_wi, zero_i64)
            .map_err(llvm_err)?;
        let srt_wloop = self.context.append_basic_block(srt_fn, "wloop");
        let srt_wbody = self.context.append_basic_block(srt_fn, "wdone");
        let srt_wcont = self.context.append_basic_block(srt_fn, "wbody");
        let _ = self.builder.build_unconditional_branch(srt_wloop);
        self.builder.position_at_end(srt_wloop);
        let srt_wiv = self
            .builder
            .build_load(i64, srt_wi, "wiv")
            .map_err(llvm_err)?
            .into_int_value();
        let srt_wcond = self
            .builder
            .build_int_compare(IntPredicate::SLT, srt_wiv, srt_len, "wcond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(srt_wcond, srt_wcont, srt_wbody);
        self.builder.position_at_end(srt_wcont);
        let srt_wl = self
            .builder
            .build_load(self.list_type, srt_ra, "wl")
            .map_err(llvm_err)?
            .into_struct_value();
        let srt_wslot = unsafe {
            self.builder
                .build_gep(self.string_type, srt_arr, &[srt_wiv], "wslot")
                .map_err(llvm_err)
        }?;
        let srt_wev = self
            .builder
            .build_load(self.string_type, srt_wslot, "wev")
            .map_err(llvm_err)?;
        let srt_wset = self
            .builder
            .build_call(
                _list_set_fn,
                &[srt_wl.into(), srt_wiv.into(), srt_wev.into()],
                "wset",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(srt_ra, srt_wset)
            .map_err(llvm_err)?;
        let srt_winc = self
            .builder
            .build_int_add(srt_wiv, one_i64, "winc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(srt_wi, srt_winc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(srt_wloop);
        self.builder.position_at_end(srt_wbody);
        let _ = self
            .builder
            .build_call(free_fn, &[srt_arr.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(srt_ret);
        self.builder.position_at_end(srt_ret);
        let srt_rt = self
            .builder
            .build_load(self.list_type, srt_ra, "srt_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&srt_rt));

        // ---- action_list_sorted_by({ptr,i64,i64} list, ptr fn) -> {ptr,i64,i64} ----
        // Custom comparator sort (insertion sort); fn(a_tag, b_tag) -> Bool, true if a > b
        let sb_fn = self.module.add_function(
            "action_list_sorted_by",
            self.list_type
                .fn_type(&[self.list_type.into(), ptr.into()], false),
            None,
        );
        let lambda_fn_ty = self.string_type.fn_type(&[i64.into(), i64.into()], false);
        let sb_entry = self.context.append_basic_block(sb_fn, "entry");
        self.builder.position_at_end(sb_entry);
        let sb_in = sb_fn.get_first_param().unwrap().into_struct_value();
        let sb_fn_ptr = sb_fn.get_nth_param(1).unwrap().into_pointer_value();
        let sb_len = self
            .builder
            .build_extract_value(sb_in, 1, "sb_len")
            .map_err(llvm_err)?
            .into_int_value();
        let sb_res_a = self
            .builder
            .build_alloca(self.list_type, "sb_res")
            .map_err(llvm_err)?;
        let sb_cc = self.call_rt("action_list_create", &[sb_len.into()])?;
        self.builder
            .build_store(sb_res_a, sb_cc.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        // Copy input elements to result
        let sb_ci = self.builder.build_alloca(i64, "sb_ci").map_err(llvm_err)?;
        self.builder
            .build_store(sb_ci, zero_i64)
            .map_err(llvm_err)?;
        let sb_chdr = self.context.append_basic_block(sb_fn, "copy_hdr");
        let sb_cbdy = self.context.append_basic_block(sb_fn, "copy_bdy");
        let sb_cext = self.context.append_basic_block(sb_fn, "copy_ext");
        let _ = self.builder.build_unconditional_branch(sb_chdr);
        self.builder.position_at_end(sb_chdr);
        let sb_civ = self
            .builder
            .build_load(i64, sb_ci, "civ")
            .map_err(llvm_err)?
            .into_int_value();
        let sb_ccond = self
            .builder
            .build_int_compare(IntPredicate::SLT, sb_civ, sb_len, "c_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sb_ccond, sb_cbdy, sb_cext);
        self.builder.position_at_end(sb_cbdy);
        let sb_elem = self
            .builder
            .build_call(_list_get_fn, &[sb_in.into(), sb_civ.into()], "elem")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        let sb_rl = self
            .builder
            .build_load(self.list_type, sb_res_a, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let sb_rp = self.call_rt("action_list_push", &[sb_rl.into(), sb_elem.into()])?;
        self.builder
            .build_store(sb_res_a, sb_rp.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let sb_cinc = self
            .builder
            .build_int_add(sb_civ, one_i64, "cinc")
            .map_err(llvm_err)?;
        self.builder.build_store(sb_ci, sb_cinc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sb_chdr);
        // Insertion sort
        self.builder.position_at_end(sb_cext);
        let sb_i = self.builder.build_alloca(i64, "sb_i").map_err(llvm_err)?;
        self.builder.build_store(sb_i, one_i64).map_err(llvm_err)?;
        let sb_ohdr = self.context.append_basic_block(sb_fn, "outer_hdr");
        let sb_obdy = self.context.append_basic_block(sb_fn, "outer_bdy");
        let sb_oext = self.context.append_basic_block(sb_fn, "outer_ext");
        let _ = self.builder.build_unconditional_branch(sb_ohdr);
        self.builder.position_at_end(sb_ohdr);
        let sb_iv_o = self
            .builder
            .build_load(i64, sb_i, "iv_o")
            .map_err(llvm_err)?
            .into_int_value();
        let sb_o_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, sb_iv_o, sb_len, "o_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sb_o_cond, sb_obdy, sb_oext);
        self.builder.position_at_end(sb_obdy);
        let sb_j = self.builder.build_alloca(i64, "sb_j").map_err(llvm_err)?;
        self.builder.build_store(sb_j, sb_iv_o).map_err(llvm_err)?;
        let sb_ihdr = self.context.append_basic_block(sb_fn, "inner_hdr");
        let sb_ibdy = self.context.append_basic_block(sb_fn, "inner_bdy");
        let sb_iext = self.context.append_basic_block(sb_fn, "inner_ext");
        let _ = self.builder.build_unconditional_branch(sb_ihdr);
        self.builder.position_at_end(sb_ihdr);
        let sb_jv = self
            .builder
            .build_load(i64, sb_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let sb_j_cond = self
            .builder
            .build_int_compare(IntPredicate::SGT, sb_jv, zero_i64, "j_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sb_j_cond, sb_ibdy, sb_iext);
        self.builder.position_at_end(sb_ibdy);
        let sb_jm1 = self
            .builder
            .build_int_sub(sb_jv, one_i64, "jm1")
            .map_err(llvm_err)?;
        let sb_rl_jm1 = self
            .builder
            .build_load(self.list_type, sb_res_a, "rl_jm1")
            .map_err(llvm_err)?
            .into_struct_value();
        let sb_ev_jm1 = self
            .builder
            .build_call(_list_get_fn, &[sb_rl_jm1.into(), sb_jm1.into()], "ev_jm1")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let sb_tag_jm1 = self
            .builder
            .build_extract_value(sb_ev_jm1, 0, "t_jm1")
            .map_err(llvm_err)?
            .into_int_value();
        let sb_rl_j = self
            .builder
            .build_load(self.list_type, sb_res_a, "rl_j")
            .map_err(llvm_err)?
            .into_struct_value();
        let sb_ev_j = self
            .builder
            .build_call(_list_get_fn, &[sb_rl_j.into(), sb_jv.into()], "ev_j")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();
        let sb_tag_j = self
            .builder
            .build_extract_value(sb_ev_j, 0, "t_j")
            .map_err(llvm_err)?
            .into_int_value();
        let sb_cmp_r = self
            .builder
            .build_indirect_call(
                lambda_fn_ty,
                sb_fn_ptr,
                &[sb_tag_jm1.into(), sb_tag_j.into()],
                "sb_cmp",
            )
            .map_err(llvm_err)?;
        let sb_cmp_bv = sb_cmp_r
            .try_as_basic_value()
            .basic()
            .ok_or("sorted_by cmp failed")?;
        let sb_cmp = if sb_cmp_bv.is_struct_value() {
            self.builder
                .build_extract_value(sb_cmp_bv.into_struct_value(), 0, "cmp")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            sb_cmp_bv.into_int_value()
        };
        let sb_should_swap = self
            .builder
            .build_int_compare(IntPredicate::NE, sb_cmp, zero_i64, "should_swap")
            .map_err(llvm_err)?;
        let sb_swap_bb = self.context.append_basic_block(sb_fn, "swap");
        let sb_noswap_bb = self.context.append_basic_block(sb_fn, "noswap");
        let _ = self
            .builder
            .build_conditional_branch(sb_should_swap, sb_swap_bb, sb_noswap_bb);
        self.builder.position_at_end(sb_swap_bb);
        let sb_rl_sw = self
            .builder
            .build_load(self.list_type, sb_res_a, "rl_sw")
            .map_err(llvm_err)?
            .into_struct_value();
        let sb_set1 = self
            .builder
            .build_call(
                _list_set_fn,
                &[sb_rl_sw.into(), sb_jm1.into(), sb_ev_j.into()],
                "set1",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(sb_res_a, sb_set1)
            .map_err(llvm_err)?;
        let sb_rl2_sw = self
            .builder
            .build_load(self.list_type, sb_res_a, "rl2_sw")
            .map_err(llvm_err)?
            .into_struct_value();
        let sb_set2 = self
            .builder
            .build_call(
                _list_set_fn,
                &[sb_rl2_sw.into(), sb_jv.into(), sb_ev_jm1.into()],
                "set2",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic();
        self.builder
            .build_store(sb_res_a, sb_set2)
            .map_err(llvm_err)?;
        self.builder.build_store(sb_j, sb_jm1).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sb_ihdr);
        self.builder.position_at_end(sb_noswap_bb);
        let _ = self.builder.build_unconditional_branch(sb_iext);
        self.builder.position_at_end(sb_iext);
        let sb_ni_o = self
            .builder
            .build_int_add(sb_iv_o, one_i64, "ni_o")
            .map_err(llvm_err)?;
        self.builder.build_store(sb_i, sb_ni_o).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sb_ohdr);
        self.builder.position_at_end(sb_oext);
        let sb_rt = self
            .builder
            .build_load(self.list_type, sb_res_a, "sb_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sb_rt));

        if let Some(pos) = saved_pos {
            self.builder.position_at_end(pos);
        }
        Ok(())
    }
}
