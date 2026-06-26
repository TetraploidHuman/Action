//! Iterator builtins: map, filter, fold, find (R4-1).

use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// reduce(list, fn) or reduce(list) { lambda } -> Option<T>
    pub(crate) fn builtin_reduce(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "reduce")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let is_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, input_len, zero, "is_empty")
            .map_err(llvm_err)?;
        // Accumulator: fat {i64,ptr}
        let acc_a = self
            .builder
            .build_alloca(self.string_type, "reduce_acc")
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(i_a, one).map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        // Init: load first element into acc
        let init_bb = self.context.append_basic_block(current_fn, "reduce_init");
        let loop_hdr = self.context.append_basic_block(current_fn, "reduce_hdr");
        let loop_bdy = self.context.append_basic_block(current_fn, "reduce_bdy");
        let loop_ext = self.context.append_basic_block(current_fn, "reduce_ext");
        let empty_bb = self.context.append_basic_block(current_fn, "reduce_empty");
        let merge_bb = self.context.append_basic_block(current_fn, "reduce_merge");
        let _ = self
            .builder
            .build_conditional_branch(is_empty, empty_bb, init_bb);
        // Init: load first element
        self.builder.position_at_end(init_bb);
        let first_val = self.list_get_cached_fat(list_ptr, zero, get_cache)?;
        self.builder
            .build_store(acc_a, first_val)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);
        // Loop
        self.builder.position_at_end(loop_hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_bdy, loop_ext);
        self.builder.position_at_end(loop_bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let acc_fat = self
            .builder
            .build_load(self.string_type, acc_a, "acc")
            .map_err(llvm_err)?;
        let acc_tag = self
            .builder
            .build_extract_value(acc_fat.into_struct_value(), 0, "acc_tag")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(
                fn_type,
                fn_ptr,
                &[acc_tag.into(), elem_tag.into()],
                "reduce_call",
            )
            .map_err(llvm_err)?;
        let new_acc = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        self.builder.build_store(acc_a, new_acc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);
        self.builder.position_at_end(loop_ext);
        let final_acc = self
            .builder
            .build_load(self.string_type, acc_a, "final_acc")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Empty: build None
        self.builder.position_at_end(empty_bb);
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge: build Option from fat struct or None
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.string_type, "reduce_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[
            (&final_acc, loop_ext),
            (&self.string_type.get_undef(), empty_bb),
        ]);
        let phi_val = phi.as_basic_value();
        let found_flag_a = self
            .builder
            .build_alloca(self.bool_ty(), "red_found")
            .map_err(llvm_err)?;
        let phi_flag = self
            .builder
            .build_phi(self.bool_ty(), "red_flag")
            .map_err(llvm_err)?;
        phi_flag.add_incoming(&[
            (&self.bool_ty().const_int(1, false), loop_ext),
            (&self.bool_ty().const_zero(), empty_bb),
        ]);
        self.builder
            .build_store(found_flag_a, phi_flag.as_basic_value())
            .map_err(llvm_err)?;
        let acc_alloca = self
            .builder
            .build_alloca(self.string_type, "red_acc_s")
            .map_err(llvm_err)?;
        self.builder
            .build_store(acc_alloca, phi_val)
            .map_err(llvm_err)?;
        // InnerType defaults to Int — accumulator is a fat struct whose type
        // is only known at runtime. See comment at builtin_find for details.
        let list_arg = if trailing.is_some() {
            args[0]
        } else if args.len() == 2 {
            args[1]
        } else {
            args[0]
        };
        let elem_ty = self.list_element_ast_type(list_arg);
        self.build_fallible_from_fat_found_flag(acc_alloca, found_flag_a, &elem_ty)
    }

    /// foldRight(list, init, fn) or foldRight(list, init) { lambda } -> T
    pub(crate) fn builtin_fold_right(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr, init_val) = self.extract_fold_right_args(args, trailing)?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let acc_a = self.builder.build_alloca(i64, "fr_acc").map_err(llvm_err)?;
        self.builder
            .build_store(acc_a, init_val)
            .map_err(llvm_err)?;
        // Iterate backwards: i = len-1 down to 0
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        let start_i = self
            .builder
            .build_int_sub(input_len, one, "start_i")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, start_i).map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "fr_hdr");
        let bdy = self.context.append_basic_block(current_fn, "fr_bdy");
        let ext = self.context.append_basic_block(current_fn, "fr_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, iv, zero, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let elem_val = self.list_get_cached_fat(list_ptr, iv, get_cache)?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let acc = self
            .builder
            .build_load(i64, acc_a, "acc")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into(), acc.into()], "fr_call")
            .map_err(llvm_err)?;
        let new_acc_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let new_acc = if new_acc_bv.is_struct_value() {
            self.builder
                .build_extract_value(new_acc_bv.into_struct_value(), 0, "fr_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            new_acc_bv.into_int_value()
        };
        self.builder.build_store(acc_a, new_acc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_sub(iv, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let final_acc = self
            .builder
            .build_load(i64, acc_a, "final_acc")
            .map_err(llvm_err)?
            .into_int_value();
        Ok(TypedValue::Int(final_acc))
    }

    /// takeWhile(list, fn) or takeWhile(list) { lambda } -> List<T>
    pub(crate) fn builtin_take_while(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "takeWhile")?;
        let list_struct = self.load_list(list_ptr)?;
        let tw_cc = self.call_rt(
            "action_list_take_while_walk",
            &[list_struct.into(), fn_ptr.into()],
        )?;
        let result_bv = tw_cc
            .try_as_basic_value()
            .basic()
            .ok_or("take_while_walk failed")?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "tw_res")
            .map_err(llvm_err)?;
        self.builder
            .build_store(res_a, result_bv)
            .map_err(llvm_err)?;
        Ok(TypedValue::List(res_a))
    }
}
