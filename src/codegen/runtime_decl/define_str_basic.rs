// Submodule: runtime_decl/define_str_basic
//
// Generated from runtime_decl closure.

use super::{llvm_err, CodeGen};
use inkwell::IntPredicate;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn define_str_basic(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let f64 = self.f64_ty();
        let _void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
let _zero = self.i64_ty().const_int(0, false);
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();

        let sprintf_fn = self.module.get_function("sprintf").unwrap();
        let strlen_fn = self.module.get_function("strlen").unwrap();
        let memcmp_fn = self.module.get_function("memcmp").unwrap();

            // ---- action_string_create(ptr, i64) -> {i64, ptr} ----
            let str_create_fn = self.module.add_function(
                "action_string_create",
                str_ty.fn_type(&[ptr.into(), i64.into()], false),
                None,
            );
            let entry = self.context.append_basic_block(str_create_fn, "entry");
            self.builder.position_at_end(entry);
            let data = str_create_fn
                .get_first_param()
                .unwrap()
                .into_pointer_value();
            let len = str_create_fn.get_nth_param(1).unwrap().into_int_value();
            // Allocate len+1 bytes with RC header
            let one = i64.const_int(1, false);
            let alloc_size = self
                .builder
                .build_int_add(len, one, "alloc_size")
                .map_err(llvm_err)?;
            let buf = self
                .builder
                .build_call(malloc_rc_fn, &[alloc_size.into()], "buf")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let _ = self
                .builder
                .build_memcpy(buf, 1, data, 1, len)
                .map_err(llvm_err)?;
            // Null-terminate at buf[len]
            let null_pos = unsafe {
                self.builder
                    .build_gep(i8, buf, &[len], "null_pos")
                    .map_err(llvm_err)
            }?;
            let zero_byte = i8.const_int(0, false);
            let _ = self
                .builder
                .build_store(null_pos, zero_byte)
                .map_err(llvm_err)?;
            let undef = str_ty.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, len, 0, "r1")
                .map_err(llvm_err)?;
            let r2 = self
                .builder
                .build_insert_value(r1, buf, 1, "r2")
                .map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&r2));

            // ---- action_string_concat({i64, ptr}, {i64, ptr}) -> {i64, ptr} ----
            let str_concat_fn = self.module.add_function(
                "action_string_concat",
                str_ty.fn_type(&[str_ty.into(), str_ty.into()], false),
                None,
            );
            let entry = self.context.append_basic_block(str_concat_fn, "entry");
            self.builder.position_at_end(entry);
            let s1 = str_concat_fn.get_first_param().unwrap().into_struct_value();
            let s2 = str_concat_fn.get_nth_param(1).unwrap().into_struct_value();
            let len1 = self
                .builder
                .build_extract_value(s1, 0, "len1")
                .map_err(llvm_err)?
                .into_int_value();
            let data1 = self
                .builder
                .build_extract_value(s1, 1, "data1")
                .map_err(llvm_err)?
                .into_pointer_value();
            let len2 = self
                .builder
                .build_extract_value(s2, 0, "len2")
                .map_err(llvm_err)?
                .into_int_value();
            let data2 = self
                .builder
                .build_extract_value(s2, 1, "data2")
                .map_err(llvm_err)?
                .into_pointer_value();
            let total = self
                .builder
                .build_int_add(len1, len2, "total")
                .map_err(llvm_err)?;
            let alloc_size = self
                .builder
                .build_int_add(total, i64.const_int(1, false), "alloc_size")
                .map_err(llvm_err)?;
            let buf = self
                .builder
                .build_call(malloc_rc_fn, &[alloc_size.into()], "buf")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let _ = self
                .builder
                .build_memcpy(buf, 1, data1, 1, len1)
                .map_err(llvm_err)?;
            let offset = unsafe {
                self.builder
                    .build_gep(i8, buf, &[len1], "offset")
                    .map_err(llvm_err)
            }?;
            let _ = self
                .builder
                .build_memcpy(offset, 1, data2, 1, len2)
                .map_err(llvm_err)?;
            // Null terminate
            let null_pos = unsafe {
                self.builder
                    .build_gep(i8, buf, &[total], "null_pos")
                    .map_err(llvm_err)
            }?;
            self.builder
                .build_store(null_pos, i8.const_int(0, false))
                .map_err(llvm_err)?;
            let undef = str_ty.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, total, 0, "r1")
                .map_err(llvm_err)?;
            let r2 = self
                .builder
                .build_insert_value(r1, buf, 1, "r2")
                .map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&r2));

            // === Helper macro for list-rebuild functions ===
            // Generates: create new empty list, then for i in [start..end) step step,
            // get element from source via action_list_get, push to new list via action_list_push.
            // $src: source list StructValue
            // $len: number of elements in source
            // $start: initial loop counter value
            // $cond: loop-continuation check (references `iv` for current counter)
            // $next: next counter value (references `iv` for current counter)
            

            // ---- action_string_eq({i64, ptr}, {i64, ptr}) -> i1 ----
            let str_eq_fn = self.module.add_function(
                "action_string_eq",
                b1.fn_type(&[str_ty.into(), str_ty.into()], false),
                None,
            );
            let entry_bb = self.context.append_basic_block(str_eq_fn, "entry");
            let compare_bb = self.context.append_basic_block(str_eq_fn, "compare");
            let check_ptr_bb = self.context.append_basic_block(str_eq_fn, "check_ptr");
            let do_memcmp_bb = self.context.append_basic_block(str_eq_fn, "do_memcmp");
            let true_bb = self.context.append_basic_block(str_eq_fn, "true");
            let false_bb = self.context.append_basic_block(str_eq_fn, "false");
            let end_bb = self.context.append_basic_block(str_eq_fn, "end");
            let s1 = str_eq_fn.get_first_param().unwrap().into_struct_value();
            let s2 = str_eq_fn.get_nth_param(1).unwrap().into_struct_value();

            self.builder.position_at_end(entry_bb);
            let len1 = self
                .builder
                .build_extract_value(s1, 0, "len1")
                .map_err(llvm_err)?
                .into_int_value();
            let len2 = self
                .builder
                .build_extract_value(s2, 0, "len2")
                .map_err(llvm_err)?
                .into_int_value();
            let len_eq = self
                .builder
                .build_int_compare(IntPredicate::EQ, len1, len2, "len_eq")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(len_eq, compare_bb, false_bb);

            self.builder.position_at_end(compare_bb);
            let zero_len = self.i64_ty().const_int(0, false);
            let is_empty = self
                .builder
                .build_int_compare(IntPredicate::EQ, len1, zero_len, "is_empty")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(is_empty, true_bb, check_ptr_bb);

            // Check for null pointers: if either is null, it's a scalar comparison — tags already match, so equal
            self.builder.position_at_end(check_ptr_bb);
            let data1 = self
                .builder
                .build_extract_value(s1, 1, "data1")
                .map_err(llvm_err)?
                .into_pointer_value();
            let data2 = self
                .builder
                .build_extract_value(s2, 1, "data2")
                .map_err(llvm_err)?
                .into_pointer_value();
            let null_ptr = self.ptr_ty().const_zero();
            let d1_null = self
                .builder
                .build_int_compare(IntPredicate::EQ, data1, null_ptr, "d1_null")
                .map_err(llvm_err)?;
            let d2_null = self
                .builder
                .build_int_compare(IntPredicate::EQ, data2, null_ptr, "d2_null")
                .map_err(llvm_err)?;
            let any_null = self
                .builder
                .build_or(d1_null, d2_null, "any_null")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(any_null, true_bb, do_memcmp_bb);

            self.builder.position_at_end(do_memcmp_bb);
            let memcmp_call = self
                .builder
                .build_call(memcmp_fn, &[data1.into(), data2.into(), len1.into()], "cmp")
                .map_err(llvm_err)?;
            let cmp_result = memcmp_call
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let zero_i32 = i32.const_int(0, false);
            let content_eq = self
                .builder
                .build_int_compare(IntPredicate::EQ, cmp_result, zero_i32, "content_eq")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(end_bb);

            self.builder.position_at_end(true_bb);
            let _ = self.builder.build_unconditional_branch(end_bb);

            self.builder.position_at_end(false_bb);
            let _ = self.builder.build_unconditional_branch(end_bb);

            self.builder.position_at_end(end_bb);
            let phi = self.builder.build_phi(b1, "eq_result").map_err(llvm_err)?;
            phi.add_incoming(&[
                (&b1.const_int(1, false), true_bb),
                (&b1.const_int(0, false), false_bb),
                (&content_eq, do_memcmp_bb),
            ]);
            let _ = self.builder.build_return(Some(&phi.as_basic_value()));

            // ---- action_string_len({i64, ptr}) -> i64 ----
            let str_len_fn = self.module.add_function(
                "action_string_len",
                i64.fn_type(&[str_ty.into()], false),
                None,
            );
            let entry = self.context.append_basic_block(str_len_fn, "entry");
            self.builder.position_at_end(entry);
            let sl_s = str_len_fn.get_first_param().unwrap().into_struct_value();
            let sl_len = self
                .builder
                .build_extract_value(sl_s, 0, "len")
                .map_err(llvm_err)?
                .into_int_value();
            let _ = self.builder.build_return(Some(&sl_len));

            // ---- action_int_to_string(i64) -> {i64, ptr} ----
            let int_to_str_fn = self.module.add_function(
                "action_int_to_string",
                str_ty.fn_type(&[i64.into()], false),
                None,
            );
            let entry = self.context.append_basic_block(int_to_str_fn, "entry");
            self.builder.position_at_end(entry);
            let n = int_to_str_fn.get_first_param().unwrap().into_int_value();
            // Allocate 32-byte buffer with RC header
            let buf32 = self.i64_ty().const_int(32, false);
            let buf = self
                .builder
                .build_call(malloc_rc_fn, &[buf32.into()], "buf")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            // sprintf(buf, "%ld", n)
            let fmt_int = self.make_global_str(".fmt_int_str", b"%ld\0");
            let _ = self
                .builder
                .build_call(sprintf_fn, &[buf.into(), fmt_int.into(), n.into()], "")
                .map_err(llvm_err)?;
            // len = strlen(buf)
            let len = self
                .builder
                .build_call(strlen_fn, &[buf.into()], "len")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            // Return {len, buf}
            let undef = str_ty.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, len, 0, "r1")
                .map_err(llvm_err)?;
            let r2 = self
                .builder
                .build_insert_value(r1, buf, 1, "r2")
                .map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&r2));

            // ---- action_float_to_string(f64) -> {i64, ptr} ----
            let float_to_str_fn = self.module.add_function(
                "action_float_to_string",
                str_ty.fn_type(&[f64.into()], false),
                None,
            );
            let entry = self.context.append_basic_block(float_to_str_fn, "entry");
            self.builder.position_at_end(entry);
            let n = float_to_str_fn
                .get_first_param()
                .unwrap()
                .into_float_value();
            let buf32 = self.i64_ty().const_int(32, false);
            let buf = self
                .builder
                .build_call(malloc_rc_fn, &[buf32.into()], "buf")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let fmt_float = self.make_global_str(".fmt_float_str", b"%g\0");
            let _ = self
                .builder
                .build_call(sprintf_fn, &[buf.into(), fmt_float.into(), n.into()], "")
                .map_err(llvm_err)?;
            let len = self
                .builder
                .build_call(strlen_fn, &[buf.into()], "len")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let undef = str_ty.get_undef();
            let r1 = self
                .builder
                .build_insert_value(undef, len, 0, "r1")
                .map_err(llvm_err)?;
            let r2 = self
                .builder
                .build_insert_value(r1, buf, 1, "r2")
                .map_err(llvm_err)?;
            let _ = self.builder.build_return(Some(&r2));

            // ---- action_int_pow(i64, i64) -> i64 (exponentiation by squaring) ----
            let int_pow_fn = self.module.add_function(
                "action_int_pow",
                i64.fn_type(&[i64.into(), i64.into()], false),
                None,
            );
            let entry = self.context.append_basic_block(int_pow_fn, "entry");
            let loop_bb = self.context.append_basic_block(int_pow_fn, "loop");
            let odd_bb = self.context.append_basic_block(int_pow_fn, "odd");
            let after_mul_bb = self.context.append_basic_block(int_pow_fn, "after_mul");
            let done_bb = self.context.append_basic_block(int_pow_fn, "done");

            let base = int_pow_fn.get_first_param().unwrap().into_int_value();
            let exp = int_pow_fn.get_nth_param(1).unwrap().into_int_value();

            self.builder.position_at_end(entry);
            let result_alloca = self.builder.build_alloca(i64, "result").map_err(llvm_err)?;
            let b_alloca = self.builder.build_alloca(i64, "b").map_err(llvm_err)?;
            let e_alloca = self.builder.build_alloca(i64, "e").map_err(llvm_err)?;
            let one = i64.const_int(1, false);
            let zero = i64.const_int(0, false);
            self.builder
                .build_store(result_alloca, one)
                .map_err(llvm_err)?;
            self.builder.build_store(b_alloca, base).map_err(llvm_err)?;
            self.builder.build_store(e_alloca, exp).map_err(llvm_err)?;
            let exp_neg = self
                .builder
                .build_int_compare(IntPredicate::SLT, exp, zero, "neg")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(exp_neg, done_bb, loop_bb);

            // loop: while e > 0
            self.builder.position_at_end(loop_bb);
            let e_cur = self
                .builder
                .build_load(i64, e_alloca, "e_cur")
                .map_err(llvm_err)?
                .into_int_value();
            let e_gt_zero = self
                .builder
                .build_int_compare(IntPredicate::SGT, e_cur, zero, "gt")
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(e_gt_zero, odd_bb, done_bb);

            // odd: if e & 1 then result *= b
            self.builder.position_at_end(odd_bb);
            let e_val = self
                .builder
                .build_load(i64, e_alloca, "e_val")
                .map_err(llvm_err)?
                .into_int_value();
            let is_odd = self
                .builder
                .build_and(e_val, one, "odd")
                .map_err(llvm_err)?;
            let odd_cond = self
                .builder
                .build_int_compare(IntPredicate::EQ, is_odd, one, "odd_cmp")
                .map_err(llvm_err)?;
            let mul_bb = self.context.append_basic_block(int_pow_fn, "mul");
            let _ = self
                .builder
                .build_conditional_branch(odd_cond, mul_bb, after_mul_bb);

            // mul: result *= b
            self.builder.position_at_end(mul_bb);
            let cur_result = self
                .builder
                .build_load(i64, result_alloca, "cur_r")
                .map_err(llvm_err)?
                .into_int_value();
            let cur_b = self
                .builder
                .build_load(i64, b_alloca, "cur_b")
                .map_err(llvm_err)?
                .into_int_value();
            let new_result = self
                .builder
                .build_int_mul(cur_result, cur_b, "mul_r")
                .map_err(llvm_err)?;
            self.builder
                .build_store(result_alloca, new_result)
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(after_mul_bb);

            // after_mul: b *= b; e >>= 1
            self.builder.position_at_end(after_mul_bb);
            let b_val = self
                .builder
                .build_load(i64, b_alloca, "b_val")
                .map_err(llvm_err)?
                .into_int_value();
            let b_sq = self
                .builder
                .build_int_mul(b_val, b_val, "sq")
                .map_err(llvm_err)?;
            self.builder.build_store(b_alloca, b_sq).map_err(llvm_err)?;
            let e_val2 = self
                .builder
                .build_load(i64, e_alloca, "e_val2")
                .map_err(llvm_err)?
                .into_int_value();
            let two = i64.const_int(2, false);
            let e_half = self
                .builder
                .build_int_signed_div(e_val2, two, "half")
                .map_err(llvm_err)?;
            self.builder
                .build_store(e_alloca, e_half)
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(loop_bb);

            // done: return result
            self.builder.position_at_end(done_bb);
            let done_val = self
                .builder
                .build_load(i64, result_alloca, "done_val")
                .map_err(llvm_err)?
                .into_int_value();
            let _ = self.builder.build_return(Some(&done_val));
            Ok(())
    }
}
