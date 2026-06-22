// Submodule: runtime_decl/define_misc
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::values::BasicValue;
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_misc(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let _str_ty = self.string_type;
        let _b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let _zero = self.i64_ty().const_int(0, false);
        let _malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let _fmt_lb_ptr = self.make_global_str(".fmt_lb", b"[\0")?;
        let _fmt_rb_ptr = self.make_global_str(".fmt_rb", b"]\0")?;
        let _fmt_sep_ptr = self.make_global_str(".fmt_sep", b", \0")?;
        let saved_pos = self.builder.get_insert_block();
        let _malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let _fmt_lb_ptr = self.make_global_str(".fmt_lb", b"[\0")?;
        let _fmt_rb_ptr = self.make_global_str(".fmt_rb", b"]\0")?;
        let _fmt_sep_ptr = self.make_global_str(".fmt_sep", b", \0")?;

        let _list_create_fn = self.module.get_function("action_list_create").unwrap();
        let _list_push_fn = self.module.get_function("action_list_push").unwrap();
        let _list_get_fn = self.module.get_function("action_list_get").unwrap();
        let _list_set_fn = self.module.get_function("action_list_set").unwrap();
        let malloc_fn = self.module.get_function("malloc").unwrap();
        let free_fn = self.module.get_function("free").unwrap();
        let qsort_fn = self.module.get_function("qsort").unwrap();
        let elem_sz = i64.const_int(16, false);
        let zero_i64 = i64.const_int(0, false);
        let one_i64 = i64.const_int(1, false);

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

        // ---- action_map_union({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Merges two maps. Entries from second map overwrite first.
        let mu_fn = self.module.add_function(
            "action_map_union",
            self.list_type
                .fn_type(&[self.list_type.into(), self.list_type.into()], false),
            None,
        );
        let mu_entry = self.context.append_basic_block(mu_fn, "entry");
        self.builder.position_at_end(mu_entry);
        let mu_a = mu_fn.get_first_param().unwrap().into_struct_value();
        let mu_b = mu_fn.get_nth_param(1).unwrap().into_struct_value();
        let mu_adata = self
            .builder
            .build_extract_value(mu_a, 0, "adata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mu_bdata = self
            .builder
            .build_extract_value(mu_b, 0, "bdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mu_alen = self
            .builder
            .build_extract_value(mu_a, 1, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_acap = self
            .builder
            .build_extract_value(mu_a, 2, "acap")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_blen = self
            .builder
            .build_extract_value(mu_b, 1, "blen")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_bcap = self
            .builder
            .build_extract_value(mu_b, 2, "bcap")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_cap = self
            .builder
            .build_int_add(mu_alen, mu_blen, "cap_hint")
            .map_err(llvm_err)?;
        let mu_create = self.module.get_function("action_map_create").unwrap();
        let bulk_fn = self
            .module
            .get_function("action_ht_bulk_copy_active_slots")
            .unwrap();
        let mu_res = self
            .builder
            .build_call(mu_create, &[mu_cap.into()], "res")
            .map_err(llvm_err)?;
        let mu_resv = mu_res.try_as_basic_value().unwrap_basic();
        let mu_ra = self
            .builder
            .build_alloca(self.list_type, "mu_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(mu_ra, mu_resv).map_err(llvm_err)?;
        let mu_res_loaded = self
            .builder
            .build_load(self.list_type, mu_ra, "mu_loaded")
            .map_err(llvm_err)?
            .into_struct_value();
        let mu_dest_data = self
            .builder
            .build_extract_value(mu_res_loaded, 0, "dest_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mu_dest_cap = self
            .builder
            .build_extract_value(mu_res_loaded, 2, "dest_cap")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_len_p = self
            .builder
            .build_struct_gep(self.list_type, mu_ra, 1, "len_p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                bulk_fn,
                &[
                    mu_dest_data.into(),
                    mu_dest_cap.into(),
                    mu_len_p.into(),
                    mu_adata.into(),
                    mu_acap.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let mi_fn = self.module.get_function("action_map_insert").unwrap();
        let mu_j = self.builder.build_alloca(i64, "mu_j").map_err(llvm_err)?;
        self.builder
            .build_store(mu_j, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let mu_loop = self.context.append_basic_block(mu_fn, "loop_b");
        let mu_chk = self.context.append_basic_block(mu_fn, "chk_b");
        let mu_body = self.context.append_basic_block(mu_fn, "body_b");
        let mu_skip = self.context.append_basic_block(mu_fn, "skip_b");
        let mu_done = self.context.append_basic_block(mu_fn, "done_b");
        let _ = self.builder.build_unconditional_branch(mu_loop);
        self.builder.position_at_end(mu_loop);
        let mu_jv = self
            .builder
            .build_load(i64, mu_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_c2 = self
            .builder
            .build_int_compare(IntPredicate::SLT, mu_jv, mu_bcap, "c2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mu_c2, mu_chk, mu_done);
        self.builder.position_at_end(mu_chk);
        self.ht_branch_if_slot_active(mu_bdata, mu_jv, mu_body, mu_skip)?;
        self.builder.position_at_end(mu_body);
        let mu_key = self.ht_key_fat_at(mu_bdata, mu_jv)?;
        let mu_val = self.ht_val_fat_at(mu_bdata, mu_jv)?;
        let mu_cl = self
            .builder
            .build_load(self.list_type, mu_ra, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let mu_ins = self
            .builder
            .build_call(mi_fn, &[mu_cl.into(), mu_key.into(), mu_val.into()], "ins")
            .map_err(llvm_err)?;
        self.builder
            .build_store(mu_ra, mu_ins.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mu_skip);
        self.builder.position_at_end(mu_skip);
        let mu_inc = self
            .builder
            .build_int_add(mu_jv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(mu_j, mu_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mu_loop);
        self.builder.position_at_end(mu_done);
        let mu_rt = self
            .builder
            .build_load(self.list_type, mu_ra, "mu_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mu_rt));

        // ---- action_pow(f64, f64) -> f64 ----
        let pow_fn = self.module.add_function(
            "action_pow",
            f64.fn_type(&[f64.into(), f64.into()], false),
            None,
        );
        let pow_entry = self.context.append_basic_block(pow_fn, "entry");
        self.builder.position_at_end(pow_entry);
        let pow_base = pow_fn.get_first_param().unwrap().into_float_value();
        let pow_exp = pow_fn.get_nth_param(1).unwrap().into_float_value();
        let pow_c_fn = self.module.get_function("pow").unwrap();
        let pow_r = self
            .builder
            .build_call(pow_c_fn, &[pow_base.into(), pow_exp.into()], "r")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_float_value();
        let _ = self.builder.build_return(Some(&pow_r));

        // ---- RC (Reference Counting) runtime ----
        // action_rc_inc(i8* ptr): increment refcount at ptr-8. Null-safe.
        let rc_inc_fn = self.module.get_function("action_rc_inc").unwrap();
        let rc_inc_entry = self.context.append_basic_block(rc_inc_fn, "entry");
        let rc_inc_do = self.context.append_basic_block(rc_inc_fn, "do_inc");
        let rc_inc_done = self.context.append_basic_block(rc_inc_fn, "done");
        self.builder.position_at_end(rc_inc_entry);
        let rc_inc_ptr = rc_inc_fn.get_first_param().unwrap().into_pointer_value();
        let rc_is_null = self
            .builder
            .build_is_null(rc_inc_ptr, "is_null")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rc_is_null, rc_inc_done, rc_inc_do);
        self.builder.position_at_end(rc_inc_do);
        let rc_inc_i64 = self
            .builder
            .build_ptr_to_int(rc_inc_ptr, i64, "rc_i64")
            .map_err(llvm_err)?;
        let rc_inc_minus8 = self
            .builder
            .build_int_sub(rc_inc_i64, i64.const_int(8, false), "minus8")
            .map_err(llvm_err)?;
        let rc_inc_i64p = self
            .builder
            .build_int_to_ptr(rc_inc_minus8, ptr, "rc_i64p")
            .map_err(llvm_err)?;
        let rc_inc_val = self
            .builder
            .build_load(self.i64_ty(), rc_inc_i64p, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let rc_inc_new = self
            .builder
            .build_int_add(rc_inc_val, i64.const_int(1, false), "new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(rc_inc_i64p, rc_inc_new)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rc_inc_done);
        self.builder.position_at_end(rc_inc_done);
        let _ = self.builder.build_return(None);

        // action_rc_dec(i8* ptr): decrement refcount at ptr-8, free if zero. Null-safe.
        let rc_dec_fn = self.module.get_function("action_rc_dec").unwrap();
        let rc_dec_entry = self.context.append_basic_block(rc_dec_fn, "entry");
        let rc_dec_null_bb = self.context.append_basic_block(rc_dec_fn, "null_check");
        let rc_dec_free_bb = self.context.append_basic_block(rc_dec_fn, "do_free");
        let rc_dec_done_bb = self.context.append_basic_block(rc_dec_fn, "done");
        self.builder.position_at_end(rc_dec_entry);
        let rc_dec_ptr = rc_dec_fn.get_first_param().unwrap().into_pointer_value();
        let rc_is_null2 = self
            .builder
            .build_is_null(rc_dec_ptr, "is_null")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rc_is_null2, rc_dec_done_bb, rc_dec_null_bb);
        self.builder.position_at_end(rc_dec_null_bb);
        let rc_dec_i64 = self
            .builder
            .build_ptr_to_int(rc_dec_ptr, i64, "rc_i64")
            .map_err(llvm_err)?;
        let rc_dec_minus8 = self
            .builder
            .build_int_sub(rc_dec_i64, i64.const_int(8, false), "minus8")
            .map_err(llvm_err)?;
        let rc_dec_i64p = self
            .builder
            .build_int_to_ptr(rc_dec_minus8, ptr, "rc_i64p")
            .map_err(llvm_err)?;
        let rc_dec_val = self
            .builder
            .build_load(self.i64_ty(), rc_dec_i64p, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let rc_dec_new = self
            .builder
            .build_int_sub(rc_dec_val, i64.const_int(1, false), "new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(rc_dec_i64p, rc_dec_new)
            .map_err(llvm_err)?;
        let rc_is_zero = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                rc_dec_new,
                i64.const_int(0, false),
                "is_zero",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rc_is_zero, rc_dec_free_bb, rc_dec_done_bb);
        self.builder.position_at_end(rc_dec_free_bb);
        let free_func = self.module.get_function("free").unwrap();
        let rc_dec_free_ptr = self
            .builder
            .build_int_to_ptr(rc_dec_minus8, ptr, "free_ptr")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(free_func, &[rc_dec_free_ptr.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rc_dec_done_bb);
        self.builder.position_at_end(rc_dec_done_bb);
        let _ = self.builder.build_return(None);

        // action_malloc_rc(i64 size) -> i8*: allocate size+8, zero rc, return ptr+8
        let malloc_rc_fn_body = self.module.get_function("action_malloc_rc").unwrap();
        let malloc_rc_entry = self.context.append_basic_block(malloc_rc_fn_body, "entry");
        self.builder.position_at_end(malloc_rc_entry);
        let malloc_rc_size = malloc_rc_fn_body
            .get_first_param()
            .unwrap()
            .into_int_value();
        let malloc_rc_total = self
            .builder
            .build_int_add(malloc_rc_size, i64.const_int(8, false), "total")
            .map_err(llvm_err)?;
        let malloc_rc_func = self.module.get_function("malloc").unwrap();
        let malloc_rc_raw = self
            .builder
            .build_call(malloc_rc_func, &[malloc_rc_total.into()], "raw")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let malloc_rc_i64p = self
            .builder
            .build_pointer_cast(malloc_rc_raw, ptr, "rc_i64p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(malloc_rc_i64p, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let malloc_rc_data = unsafe {
            self.builder
                .build_gep(i8, malloc_rc_raw, &[i64.const_int(8, false)], "data")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_return(Some(&malloc_rc_data));

        // ---- action_rc_dec_list_node(ptr node_ptr, i64 height): recursive RC decrement for tree ----
        // height==0: leaf (elements), height>0: internal (children)
        let rdl_fn = self.module.add_function(
            "action_rc_dec_list_node",
            void.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let rdl_entry = self.context.append_basic_block(rdl_fn, "entry");
        let rdl_null_done = self.context.append_basic_block(rdl_fn, "null_done");
        let rdl_dec = self.context.append_basic_block(rdl_fn, "do_dec");
        let rdl_check_zero = self.context.append_basic_block(rdl_fn, "check_zero");
        let rdl_done = self.context.append_basic_block(rdl_fn, "done");
        let rdl_leaf_cleanup = self.context.append_basic_block(rdl_fn, "leaf_cleanup");
        let rdl_int_cleanup = self.context.append_basic_block(rdl_fn, "int_cleanup");
        let rdl_free_node = self.context.append_basic_block(rdl_fn, "free_node");
        let rdl_iter_body = self.context.append_basic_block(rdl_fn, "iter_body");
        let rdl_iter_next = self.context.append_basic_block(rdl_fn, "iter_next");

        // entry: null check
        self.builder.position_at_end(rdl_entry);
        let rdl_node = rdl_fn.get_first_param().unwrap().into_pointer_value();
        let rdl_height = rdl_fn.get_nth_param(1).unwrap().into_int_value();
        let rdl_is_null = self
            .builder
            .build_is_null(rdl_node, "is_null")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rdl_is_null, rdl_null_done, rdl_dec);
        self.builder.position_at_end(rdl_null_done);
        let _ = self.builder.build_return(None);

        // do_dec: load rc at node_ptr - 8, decrement, store
        self.builder.position_at_end(rdl_dec);
        let rdl_ptr_i64 = self
            .builder
            .build_ptr_to_int(rdl_node, i64, "pi64")
            .map_err(llvm_err)?;
        let rdl_rc_addr = self
            .builder
            .build_int_sub(rdl_ptr_i64, i64.const_int(8, false), "rc_addr")
            .map_err(llvm_err)?;
        let rdl_rc_p = self
            .builder
            .build_int_to_ptr(rdl_rc_addr, ptr, "rc_p")
            .map_err(llvm_err)?;
        let rdl_rc = self
            .builder
            .build_load(i64, rdl_rc_p, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let rdl_new_rc = self
            .builder
            .build_int_sub(rdl_rc, i64.const_int(1, false), "new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(rdl_rc_p, rdl_new_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rdl_check_zero);

        // check_zero: if new_rc != 0, return early
        self.builder.position_at_end(rdl_check_zero);
        let rdl_is_zero = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                rdl_new_rc,
                i64.const_int(0, false),
                "is_zero",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rdl_is_zero, rdl_leaf_cleanup, rdl_done);
        self.builder.position_at_end(rdl_done);
        let _ = self.builder.build_return(None);

        // leaf_cleanup: branch based on height (-1=concat, 0=leaf, >0=internal)
        self.builder.position_at_end(rdl_leaf_cleanup);
        let rdl_is_concat = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                rdl_height,
                i64.const_int(-1i64 as u64, true),
                "is_concat",
            )
            .map_err(llvm_err)?;
        let rdl_cleanup_normal = self.context.append_basic_block(rdl_fn, "cleanup_normal");
        let rdl_concat_cleanup = self.context.append_basic_block(rdl_fn, "concat_cleanup");
        let _ = self.builder.build_conditional_branch(
            rdl_is_concat,
            rdl_concat_cleanup,
            rdl_cleanup_normal,
        );

        // concat_cleanup: decrement RC of left and right subtrees, then free
        self.builder.position_at_end(rdl_concat_cleanup);
        // Load left list: {ptr node, i64 len, i64 height} at offset 16
        let rdll_node_ptr = unsafe {
            self.builder
                .build_gep(i8, rdl_node, &[i64.const_int(16, false)], "rdll_np")
                .map_err(llvm_err)
        }?;
        let rdll_node = self
            .builder
            .build_load(ptr, rdll_node_ptr, "rdll_n")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rdll_h_ptr = unsafe {
            self.builder
                .build_gep(i8, rdl_node, &[i64.const_int(32, false)], "rdll_hp")
                .map_err(llvm_err)
        }?;
        let rdll_h = self
            .builder
            .build_load(i64, rdll_h_ptr, "rdll_h")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(rdl_fn, &[rdll_node.into(), rdll_h.into()], "")
            .map_err(llvm_err)?;
        // Load right list: at offset 40
        let rdlr_node_ptr = unsafe {
            self.builder
                .build_gep(i8, rdl_node, &[i64.const_int(40, false)], "rdlr_np")
                .map_err(llvm_err)
        }?;
        let rdlr_node = self
            .builder
            .build_load(ptr, rdlr_node_ptr, "rdlr_n")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rdlr_h_ptr = unsafe {
            self.builder
                .build_gep(i8, rdl_node, &[i64.const_int(56, false)], "rdlr_hp")
                .map_err(llvm_err)
        }?;
        let rdlr_h = self
            .builder
            .build_load(i64, rdlr_h_ptr, "rdlr_h")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(rdl_fn, &[rdlr_node.into(), rdlr_h.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rdl_free_node);

        // cleanup_normal: original logic for leaf (h=0) and internal (h>0)
        self.builder.position_at_end(rdl_cleanup_normal);
        let rdl_is_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                rdl_height,
                i64.const_int(0, false),
                "is_leaf",
            )
            .map_err(llvm_err)?;
        let _ =
            self.builder
                .build_conditional_branch(rdl_is_leaf, rdl_int_cleanup, rdl_int_cleanup);
        // Both leaf and internal iterate entries at byte offset 16+i*16.
        // The pointer at that offset is: for leaf -> data ptr (call action_rc_dec),
        // for internal -> child ptr (call action_rc_dec_list_node with height-1).
        // We'll use the same iteration for both and branch on rdl_is_leaf for the action.

        // Start iteration: count at byte 0, i=0
        // This block is for both leaf and internal cleanup
        self.builder.position_at_end(rdl_int_cleanup);
        let rdl_count_raw = self
            .builder
            .build_load(i32, rdl_node, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let rdl_count = self
            .builder
            .build_int_z_extend(rdl_count_raw, i64, "count")
            .map_err(llvm_err)?;
        let rdl_count_zero = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                rdl_count,
                i64.const_int(0, false),
                "count_zero",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rdl_count_zero, rdl_free_node, rdl_iter_body);

        // iter_body: load entry pointer at byte 16 + i*16
        self.builder.position_at_end(rdl_iter_body);
        let rdl_phi_i = self.builder.build_phi(i64, "phi_i").map_err(llvm_err)?;
        rdl_phi_i.add_incoming(&[(&i64.const_int(0, false), rdl_int_cleanup)]);
        let rdl_i = rdl_phi_i.as_basic_value().into_int_value();
        let rdl_done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, rdl_i, rdl_count, "done_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rdl_done_cond, rdl_free_node, rdl_iter_next);

        // iter_next: process entry i
        self.builder.position_at_end(rdl_iter_next);
        // Compute byte offset: 16 + i*16
        let rdl_i16 = self
            .builder
            .build_int_mul(rdl_i, i64.const_int(16, false), "i16")
            .map_err(llvm_err)?;
        let rdl_off = self
            .builder
            .build_int_add(i64.const_int(16, false), rdl_i16, "off")
            .map_err(llvm_err)?;
        let rdl_ep = unsafe {
            self.builder
                .build_gep(i8, rdl_node, &[rdl_off], "ep")
                .map_err(llvm_err)
        }?;
        let rdl_ptr = self
            .builder
            .build_load(ptr, rdl_ep, "ptr_val")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rdl_ptr_nonnull = self
            .builder
            .build_is_not_null(rdl_ptr, "ptr_nonnull")
            .map_err(llvm_err)?;
        let rdl_call_skip = self.context.append_basic_block(rdl_fn, "call_skip");
        let rdl_call_do = self.context.append_basic_block(rdl_fn, "call_do");
        let _ = self
            .builder
            .build_conditional_branch(rdl_ptr_nonnull, rdl_call_do, rdl_call_skip);

        // call_do: branch on leaf vs internal to handle the pointer correctly
        self.builder.position_at_end(rdl_call_do);
        // rdl_is_leaf from rdl_leaf_cleanup dominates this block, so use it directly
        let rdl_call_leaf = self.context.append_basic_block(rdl_fn, "call_leaf");
        let rdl_call_int = self.context.append_basic_block(rdl_fn, "call_int");
        let _ = self
            .builder
            .build_conditional_branch(rdl_is_leaf, rdl_call_leaf, rdl_call_int);

        // call_leaf: rc_dec the string element (slice-aware)
        self.builder.position_at_end(rdl_call_leaf);
        let str_rc_dec_fn = self.module.get_function("action_string_rc_dec").unwrap();
        let rdl_elem_base = unsafe {
            self.builder
                .build_gep(i8, rdl_node, &[i64.const_int(8, false)], "elem_base")
                .map_err(llvm_err)
        }?;
        let rdl_str_gep = unsafe {
            self.builder
                .build_gep(self.string_type, rdl_elem_base, &[rdl_i], "str_gep")
                .map_err(llvm_err)
        }?;
        let rdl_str_val = self
            .builder
            .build_load(self.string_type, rdl_str_gep, "str_val")
            .map_err(llvm_err)?
            .into_struct_value();
        let _ = self
            .builder
            .build_call(str_rc_dec_fn, &[rdl_str_val.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rdl_call_skip);

        // call_int: recurse on child node with height-1
        self.builder.position_at_end(rdl_call_int);
        let rdl_child_h = self
            .builder
            .build_int_sub(rdl_height, i64.const_int(1, false), "child_h")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(rdl_fn, &[rdl_ptr.into(), rdl_child_h.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rdl_call_skip);

        // call_skip: increment i and loop back
        self.builder.position_at_end(rdl_call_skip);
        let rdl_next_i = self
            .builder
            .build_int_add(rdl_i, i64.const_int(1, false), "next_i")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rdl_iter_body);
        rdl_phi_i.add_incoming(&[(&rdl_next_i, rdl_call_skip)]);

        // iter_done and free_node: free the node
        // free_node: call free(node_ptr - 8)
        self.builder.position_at_end(rdl_free_node);
        let rdl_free_p = self
            .builder
            .build_int_to_ptr(rdl_rc_addr, ptr, "free_p")
            .map_err(llvm_err)?;
        let free_func = self.module.get_function("free").unwrap();
        let _ = self
            .builder
            .build_call(free_func, &[rdl_free_p.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

        // action_utf8_encode body: encode a Unicode code point into UTF-8 bytes

        // action_utf8_encode body: encode a Unicode code point into UTF-8 bytes
        // Takes (i64 code_point, i8* buf) -> returns i64 byte_count (1-4)
        let utf8_encode_fn_body = self.module.get_function("action_utf8_encode").unwrap();
        let utf8_entry = self
            .context
            .append_basic_block(utf8_encode_fn_body, "entry");
        let utf8_1b = self
            .context
            .append_basic_block(utf8_encode_fn_body, "one_byte");
        let utf8_2b = self
            .context
            .append_basic_block(utf8_encode_fn_body, "two_byte");
        let utf8_3b = self
            .context
            .append_basic_block(utf8_encode_fn_body, "three_byte");
        let utf8_4b = self
            .context
            .append_basic_block(utf8_encode_fn_body, "four_byte");
        self.builder.position_at_end(utf8_entry);
        let ucode = utf8_encode_fn_body
            .get_first_param()
            .unwrap()
            .into_int_value();
        let ubuf = utf8_encode_fn_body
            .get_nth_param(1)
            .unwrap()
            .into_pointer_value();
        let u0x7f = i64.const_int(0x7F, false);
        let u0x7ff = i64.const_int(0x7FF, false);
        let u0xffff = i64.const_int(0xFFFF, false);
        let is_1 = self
            .builder
            .build_int_compare(IntPredicate::ULE, ucode, u0x7f, "is1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_1, utf8_1b, utf8_2b);
        // 1-byte: buf[0] = code (0x00-0x7F)
        self.builder.position_at_end(utf8_1b);
        let u1 = self
            .builder
            .build_int_truncate(ucode, i8, "u1")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(ubuf, u1).map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&i64.const_int(1, false)));
        // 2-byte check: code <= 0x7FF?
        self.builder.position_at_end(utf8_2b);
        let is_2 = self
            .builder
            .build_int_compare(IntPredicate::ULE, ucode, u0x7ff, "is2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_2, utf8_3b, utf8_4b);
        // Write 2-byte: buf[0] = 0xC0 | (code >> 6); buf[1] = 0x80 | (code & 0x3F)
        self.builder.position_at_end(utf8_3b);
        let u6 = i64.const_int(6, false);
        let ucp6 = self
            .builder
            .build_right_shift(ucode, u6, false, "cp6")
            .map_err(llvm_err)?;
        let ulead2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucp6, i8, "l2t")
                    .map_err(llvm_err)?,
                i8.const_int(0xC0, false),
                "lead2",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(ubuf, ulead2).map_err(llvm_err)?;
        let umask = i64.const_int(0x3F, false);
        let ucont2 = self
            .builder
            .build_and(ucode, umask, "cont2")
            .map_err(llvm_err)?;
        let ub2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont2, i8, "c2t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b2",
            )
            .map_err(llvm_err)?;
        let ugp1 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(1, false)], "gp1")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp1, ub2).map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&i64.const_int(2, false)));
        // 3-byte check: code <= 0xFFFF?
        self.builder.position_at_end(utf8_4b);
        let is_3 = self
            .builder
            .build_int_compare(IntPredicate::ULE, ucode, u0xffff, "is3")
            .map_err(llvm_err)?;
        let utf8_3b_write = self
            .context
            .append_basic_block(utf8_encode_fn_body, "three_byte_write");
        let utf8_4b_write = self
            .context
            .append_basic_block(utf8_encode_fn_body, "four_byte_write");
        let _ = self
            .builder
            .build_conditional_branch(is_3, utf8_3b_write, utf8_4b_write);
        // Write 3-byte: buf[0] = 0xE0 | (code >> 12); buf[1] = 0x80 | ((code >> 6) & 0x3F); buf[2] = 0x80 | (code & 0x3F)
        self.builder.position_at_end(utf8_3b_write);
        let u12 = i64.const_int(12, false);
        let ucp12 = self
            .builder
            .build_right_shift(ucode, u12, false, "cp12")
            .map_err(llvm_err)?;
        let ulead3 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucp12, i8, "l3t")
                    .map_err(llvm_err)?,
                i8.const_int(0xE0, false),
                "lead3",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(ubuf, ulead3).map_err(llvm_err)?;
        let ucp6b = self
            .builder
            .build_right_shift(ucode, u6, false, "cp6b")
            .map_err(llvm_err)?;
        let ucont3_1 = self
            .builder
            .build_and(ucp6b, umask, "c3_1")
            .map_err(llvm_err)?;
        let ub3_1 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont3_1, i8, "c3_1t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b3_1",
            )
            .map_err(llvm_err)?;
        let ugp3_1 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(1, false)], "gp3_1")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp3_1, ub3_1).map_err(llvm_err)?;
        let ucont3_2 = self
            .builder
            .build_and(ucode, umask, "c3_2")
            .map_err(llvm_err)?;
        let ub3_2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont3_2, i8, "c3_2t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b3_2",
            )
            .map_err(llvm_err)?;
        let ugp3_2 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(2, false)], "gp3_2")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp3_2, ub3_2).map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&i64.const_int(3, false)));
        // Write 4-byte: buf[0] = 0xF0 | (code >> 18); buf[1] = 0x80 | ((code >> 12) & 0x3F);
        //                buf[2] = 0x80 | ((code >> 6) & 0x3F); buf[3] = 0x80 | (code & 0x3F)
        self.builder.position_at_end(utf8_4b_write);
        let u18 = i64.const_int(18, false);
        let ucp18 = self
            .builder
            .build_right_shift(ucode, u18, false, "cp18")
            .map_err(llvm_err)?;
        let ulead4 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucp18, i8, "l4t")
                    .map_err(llvm_err)?,
                i8.const_int(0xF0, false),
                "lead4",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(ubuf, ulead4).map_err(llvm_err)?;
        let u4_12 = i64.const_int(12, false);
        let u4_6 = i64.const_int(6, false);
        let ucp12b4 = self
            .builder
            .build_right_shift(ucode, u4_12, false, "cp12b4")
            .map_err(llvm_err)?;
        let ucont4_1 = self
            .builder
            .build_and(ucp12b4, umask, "c4_1")
            .map_err(llvm_err)?;
        let ub4_1 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont4_1, i8, "c4_1t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b4_1",
            )
            .map_err(llvm_err)?;
        let ugp4_1 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(1, false)], "gp4_1")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp4_1, ub4_1).map_err(llvm_err)?;
        let ucp6b4 = self
            .builder
            .build_right_shift(ucode, u4_6, false, "cp6b4")
            .map_err(llvm_err)?;
        let ucont4_2 = self
            .builder
            .build_and(ucp6b4, umask, "c4_2")
            .map_err(llvm_err)?;
        let ub4_2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont4_2, i8, "c4_2t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b4_2",
            )
            .map_err(llvm_err)?;
        let ugp4_2 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(2, false)], "gp4_2")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp4_2, ub4_2).map_err(llvm_err)?;
        let ucont4_3 = self
            .builder
            .build_and(ucode, umask, "c4_3")
            .map_err(llvm_err)?;
        let ub4_3 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont4_3, i8, "c4_3t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b4_3",
            )
            .map_err(llvm_err)?;
        let ugp4_3 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(3, false)], "gp4_3")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp4_3, ub4_3).map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&i64.const_int(4, false)));

        // action_utf8_byte_len body: determine UTF-8 byte count from leading byte
        let utf8_bl_fn = self.module.get_function("action_utf8_byte_len").unwrap();
        let bl_entry = self.context.append_basic_block(utf8_bl_fn, "entry");
        self.builder.position_at_end(bl_entry);
        let bl_byte = utf8_bl_fn.get_first_param().unwrap().into_int_value();
        let bl_byte_zext = self
            .builder
            .build_int_z_extend(bl_byte, i64, "zext")
            .map_err(llvm_err)?;
        // Check if continuation byte (10xxxxxx) → treat as 1
        let bl_80 = i64.const_int(0x80, false);
        let is_ascii = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_and(bl_byte_zext, bl_80, "and80")
                    .map_err(llvm_err)?,
                i64.const_int(0, false),
                "is_ascii",
            )
            .map_err(llvm_err)?;
        // Check 2-byte: (byte & 0xE0) == 0xC0
        let bl_e0 = i64.const_int(0xE0, false);
        let bl_c0 = i64.const_int(0xC0, false);
        let is_2b = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_and(bl_byte_zext, bl_e0, "andE0")
                    .map_err(llvm_err)?,
                bl_c0,
                "is_2b",
            )
            .map_err(llvm_err)?;
        // Check 3-byte: (byte & 0xF0) == 0xE0
        let bl_f0 = i64.const_int(0xF0, false);
        let bl_e0c = i64.const_int(0xE0, false);
        let is_3b = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_and(bl_byte_zext, bl_f0, "andF0")
                    .map_err(llvm_err)?,
                bl_e0c,
                "is_3b",
            )
            .map_err(llvm_err)?;
        // Check 4-byte: (byte & 0xF8) == 0xF0
        let bl_f8 = i64.const_int(0xF8, false);
        let bl_f0c = i64.const_int(0xF0, false);
        let _is_4b = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_and(bl_byte_zext, bl_f8, "andF8")
                    .map_err(llvm_err)?,
                bl_f0c,
                "is_4b",
            )
            .map_err(llvm_err)?;
        // Select: 3/4, 2/selected, 1/selected
        let one = i64.const_int(1, false);
        let two = i64.const_int(2, false);
        let three = i64.const_int(3, false);
        let four = i64.const_int(4, false);
        let bl_s3 = self
            .builder
            .build_select(is_3b, three, four, "s3")
            .map_err(llvm_err)?
            .into_int_value();
        let bl_s2 = self
            .builder
            .build_select(is_2b, two, bl_s3, "s2")
            .map_err(llvm_err)?
            .into_int_value();
        let bl_result = self
            .builder
            .build_select(is_ascii, one, bl_s2, "s1")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_return(Some(&bl_result));

        // Restore builder position
        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }

        Ok(())
    }
}
