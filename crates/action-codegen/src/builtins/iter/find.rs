//! Iterator builtins: map, filter, fold, find (R4-1).

use inkwell::values::PointerValue;
use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// find(list, fn) or find(list) { lambda } -> Option<T>
    pub(crate) fn find_on_list_ptr(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_ptr = self.callback_fn_ptr(&fn_val, "find")?;
        if let Some(target) = self.try_direct_lambda(fn_val.clone()) {
            return self.builtin_find_with_direct_lambda(list_ptr, &target);
        }
        let fn_type = self.predicate_llvm_fn_type(&fn_val)?;
        if self.predicate_returns_fat(fn_type) {
            let list_struct = self.load_list(list_ptr)?;
            let find_cc = self.call_rt(
                "action_list_find_walk",
                &[list_struct.into(), fn_ptr.into()],
            )?;
            let found_bv = find_cc
                .try_as_basic_value()
                .basic()
                .ok_or("find_walk failed")?;
            let found_a = self
                .builder
                .build_alloca(self.string_type, "found")
                .map_err(crate::llvm_err)?;
            let found_flag_a = self
                .builder
                .build_alloca(self.bool_ty(), "found_f")
                .map_err(crate::llvm_err)?;
            self.builder
                .build_store(found_a, found_bv)
                .map_err(crate::llvm_err)?;
            let found_tag = self
                .builder
                .build_extract_value(found_bv.into_struct_value(), 0, "ft")
                .map_err(crate::llvm_err)?
                .into_int_value();
            let is_found = self
                .builder
                .build_int_compare(
                    IntPredicate::NE,
                    found_tag,
                    self.i64_ty().const_int(1, false),
                    "is_found",
                )
                .map_err(crate::llvm_err)?;
            let found_i64 = self
                .builder
                .build_int_z_extend(is_found, self.i64_ty(), "found_i64")
                .map_err(crate::llvm_err)?;
            self.builder
                .build_store(found_flag_a, found_i64)
                .map_err(crate::llvm_err)?;
            return self.build_fallible_str_from_found_flag(found_a, found_flag_a);
        }
        self.builtin_find_indexed(list_ptr, fn_ptr, fn_type, &fn_val)
    }

    /// find(list, fn) or find(list) { lambda } -> Option<T>
    pub(crate) fn builtin_find(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_val, list_val) = self.extract_callback_fn_and_list(args, trailing, 1, "find")?;
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("find: last argument must be a list".to_string()),
        };
        self.find_on_list_ptr(list_ptr, fn_val)
    }

    pub(crate) fn builtin_find_with_direct_lambda(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        target: &crate::mono::DirectLambdaTarget<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let found_a = self
            .builder
            .build_alloca(self.string_type, "fd_elem")
            .map_err(llvm_err)?;
        let found_flag_a = self
            .builder
            .build_alloca(self.bool_ty(), "fd_flag")
            .map_err(llvm_err)?;
        self.builder
            .build_store(found_flag_a, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "fd_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "fd_hdr");
        let bdy = self.context.append_basic_block(current_fn, "fd_bdy");
        let set_found = self.context.append_basic_block(current_fn, "fd_set");
        let ext = self.context.append_basic_block(current_fn, "fd_ext");
        let one_b = self.bool_ty().const_int(1, false);
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "fd_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "fd_cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "fd_et")
            .map_err(llvm_err)?
            .into_int_value();
        let pred = {
            let cc = self.emit_direct_lambda_call(target, elem_tag, "fd_call")?;
            if cc.is_struct_value() {
                self.builder
                    .build_extract_value(cc.into_struct_value(), 0, "fd_pred")
                    .map_err(llvm_err)?
                    .into_int_value()
            } else {
                cc.into_int_value()
            }
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "fd_true")
            .map_err(llvm_err)?;
        self.builder
            .build_store(found_a, elem_val)
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "fd_ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let chk = self.context.append_basic_block(current_fn, "fd_chk");
        let _ = self
            .builder
            .build_conditional_branch(is_true, set_found, chk);
        self.builder.position_at_end(set_found);
        self.builder
            .build_store(found_flag_a, one_b)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ext);
        self.builder.position_at_end(chk);
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        self.build_fallible_str_from_found_flag(found_a, found_flag_a)
    }

    pub(crate) fn builtin_find_indexed(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        fn_ptr: PointerValue<'ctx>,
        fn_type: inkwell::types::FunctionType<'ctx>,
        fn_val: &TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let found_a = self
            .builder
            .build_alloca(self.string_type, "fd_elem")
            .map_err(llvm_err)?;
        let found_flag_a = self
            .builder
            .build_alloca(self.bool_ty(), "fd_flag")
            .map_err(llvm_err)?;
        self.builder
            .build_store(found_flag_a, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "fd_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "fd_hdr");
        let bdy = self.context.append_basic_block(current_fn, "fd_bdy");
        let set_found = self.context.append_basic_block(current_fn, "fd_set");
        let ext = self.context.append_basic_block(current_fn, "fd_ext");
        let one_b = self.bool_ty().const_int(1, false);
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "fd_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "fd_cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "fd_et")
            .map_err(llvm_err)?
            .into_int_value();
        let pred =
            self.call_predicate_on_tag_for_val(fn_val, fn_ptr, fn_type, elem_tag, "fd_call")?;
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "fd_true")
            .map_err(llvm_err)?;
        self.builder
            .build_store(found_a, elem_val)
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "fd_ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let chk = self.context.append_basic_block(current_fn, "fd_chk");
        let _ = self
            .builder
            .build_conditional_branch(is_true, set_found, chk);
        self.builder.position_at_end(set_found);
        self.builder
            .build_store(found_flag_a, one_b)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ext);
        self.builder.position_at_end(chk);
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        self.build_fallible_str_from_found_flag(found_a, found_flag_a)
    }

    /// findIndex(list, fn) on an already-compiled list alloca (UFCS fast path).
    pub(crate) fn find_index_on_list_ptr(
        &mut self,
        list_ptr: PointerValue<'ctx>,
        fn_val: TypedValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_ptr = self.callback_fn_ptr(&fn_val, "findIndex")?;
        let direct_target = self.try_direct_lambda(fn_val.clone());
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let result_a = self.builder.build_alloca(i64, "fi_idx").map_err(llvm_err)?;
        self.builder
            .build_store(result_a, i64.const_int((-1i64) as u64, true))
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "fi_hdr");
        let bdy = self.context.append_basic_block(current_fn, "fi_bdy");
        let ext = self.context.append_basic_block(current_fn, "fi_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let pred = if let Some(ref target) = direct_target {
            let cc = self.emit_direct_lambda_call(target, elem_tag, "fi_call")?;
            if cc.is_struct_value() {
                self.builder
                    .build_extract_value(cc.into_struct_value(), 0, "pred")
                    .map_err(llvm_err)?
                    .into_int_value()
            } else {
                cc.into_int_value()
            }
        } else {
            let fn_type = self.predicate_llvm_fn_type(&fn_val)?;
            self.call_predicate_on_tag_for_val(&fn_val, fn_ptr, fn_type, elem_tag, "fi_call")?
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        self.builder.build_store(result_a, iv).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let fi_hdr2 = self.context.append_basic_block(current_fn, "fi_chk");
        let _ = self.builder.build_conditional_branch(is_true, ext, fi_hdr2);
        self.builder.position_at_end(fi_hdr2);
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let found_idx = self
            .builder
            .build_load(i64, result_a, "found_idx")
            .map_err(llvm_err)?
            .into_int_value();
        let is_found = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                found_idx,
                i64.const_int(0, false),
                "is_found",
            )
            .map_err(llvm_err)?;
        self.build_fallible_int_from_ok(found_idx, is_found)
    }

    /// findIndex(list, fn) or findIndex(list) { lambda } -> Option<Int>
    pub(crate) fn builtin_find_index(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_val, list_val) =
            self.extract_callback_fn_and_list(args, trailing, 1, "findIndex")?;
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("findIndex: last argument must be a list".to_string()),
        };
        self.find_index_on_list_ptr(list_ptr, fn_val)
    }
}
