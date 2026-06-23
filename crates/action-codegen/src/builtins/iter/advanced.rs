//! Iterator builtins: map, filter, fold, find (R4-1).

use inkwell::IntPredicate;

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// dropWhile(list, fn) or dropWhile(list) { lambda } -> List<T>
    pub(crate) fn builtin_drop_while(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "dropWhile")?;
        let list_struct = self.load_list(list_ptr)?;
        let input_len = self.list_len_val(list_struct)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let res_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "dw_res")
            .map_err(llvm_err)?;
        self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
        let dropping_a = self
            .builder
            .build_alloca(self.bool_ty(), "dropping")
            .map_err(llvm_err)?;
        self.builder
            .build_store(dropping_a, self.bool_ty().const_int(1, false))
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "dw_hdr");
        let bdy = self.context.append_basic_block(current_fn, "dw_bdy");
        let ext = self.context.append_basic_block(current_fn, "dw_ext");
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
        let dropping = self
            .builder
            .build_load(self.bool_ty(), dropping_a, "dropping")
            .map_err(llvm_err)?
            .into_int_value();
        let is_dropping = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                dropping,
                self.bool_ty().const_zero(),
                "is_dropping",
            )
            .map_err(llvm_err)?;
        // Only call predicate if still dropping
        let call_bb = self.context.append_basic_block(current_fn, "dw_call");
        let push_bb = self.context.append_basic_block(current_fn, "dw_push");
        let inc_bb = self.context.append_basic_block(current_fn, "dw_inc");
        let _ = self
            .builder
            .build_conditional_branch(is_dropping, call_bb, push_bb);
        // Call predicate
        self.builder.position_at_end(call_bb);
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "dw_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        // If true, still dropping, skip element (go to inc). If false, stop dropping, push element.
        let _ = self
            .builder
            .build_conditional_branch(is_true, inc_bb, push_bb);
        // Push element
        self.builder.position_at_end(push_bb);
        self.builder
            .build_store(dropping_a, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        let rl = self
            .builder
            .build_load(self.list_type, res_a, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rp = self.call_rt("action_list_push", &[rl.into(), elem_val.into()])?;
        self.builder
            .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(inc_bb);
        // Increment
        self.builder.position_at_end(inc_bb);
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        Ok(TypedValue::List(res_a))
    }

    /// sortedBy(list, fn) or sortedBy(list) { lambda } -> List<T>
    pub(crate) fn builtin_sorted_by(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "sortedBy")?;
        let list_struct = self.load_list(list_ptr)?;
        let cc = self.call_rt(
            "action_list_sorted_by",
            &[list_struct.into(), fn_ptr.into()],
        )?;
        let result = cc.try_as_basic_value().basic().ok_or("sortedBy failed")?;
        let alloca = self
            .builder
            .build_alloca(self.list_type, "sb_res")
            .map_err(llvm_err)?;
        self.builder.build_store(alloca, result).map_err(llvm_err)?;
        Ok(TypedValue::List(alloca))
    }

    /// partition(list, fn) or partition(list) { lambda } -> (List<T>, List<T>)
    pub(crate) fn builtin_partition(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "partition")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        // Create two result lists
        let cap = i64.const_int(4, false);
        let left_cc = self.call_rt("action_list_create", &[cap.into()])?;
        let left_bv = left_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create left")?;
        let left_a = self
            .builder
            .build_alloca(self.list_type, "part_left")
            .map_err(llvm_err)?;
        self.builder
            .build_store(left_a, left_bv)
            .map_err(llvm_err)?;
        let right_cc = self.call_rt("action_list_create", &[cap.into()])?;
        let right_bv = right_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create right")?;
        let right_a = self
            .builder
            .build_alloca(self.list_type, "part_right")
            .map_err(llvm_err)?;
        self.builder
            .build_store(right_a, right_bv)
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "part_hdr");
        let bdy = self.context.append_basic_block(current_fn, "part_bdy");
        let ext = self.context.append_basic_block(current_fn, "part_ext");
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
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "part_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let left_bb = self.context.append_basic_block(current_fn, "part_left");
        let right_bb = self.context.append_basic_block(current_fn, "part_right");
        let part_merge = self.context.append_basic_block(current_fn, "part_merge2");
        let _ = self
            .builder
            .build_conditional_branch(is_true, left_bb, right_bb);
        // Push to left
        self.builder.position_at_end(left_bb);
        let ll = self.load_list(left_a)?;
        let lp = self.call_rt("action_list_push", &[ll.into(), elem_val.into()])?;
        let lp_bv = lp.try_as_basic_value().basic().ok_or("push left")?;
        self.builder.build_store(left_a, lp_bv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(part_merge);
        // Push to right
        self.builder.position_at_end(right_bb);
        let rl = self.load_list(right_a)?;
        let rp = self.call_rt("action_list_push", &[rl.into(), elem_val.into()])?;
        let rp_bv = rp.try_as_basic_value().basic().ok_or("push right")?;
        self.builder.build_store(right_a, rp_bv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(part_merge);
        self.builder.position_at_end(part_merge);
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        // Build tuple struct: {list_type, list_type}
        let lv = self
            .builder
            .build_load(self.list_type, left_a, "lv")
            .map_err(llvm_err)?;
        let rv = self
            .builder
            .build_load(self.list_type, right_a, "rv")
            .map_err(llvm_err)?;
        let tuple_ty = self
            .context
            .struct_type(&[self.list_type.into(), self.list_type.into()], false);
        let undef = tuple_ty.get_undef();
        let t1 = self
            .builder
            .build_insert_value(undef, lv, 0, "t_l")
            .map_err(llvm_err)?;
        let t2 = self
            .builder
            .build_insert_value(t1, rv, 1, "t_r")
            .map_err(llvm_err)?;
        let alloca = self
            .builder
            .build_alloca(tuple_ty, "part_tuple")
            .map_err(llvm_err)?;
        self.builder.build_store(alloca, t2).map_err(llvm_err)?;
        Ok(TypedValue::Struct(alloca, tuple_ty))
    }

    /// count(list, fn) or count(list) { lambda } -> Int
    pub(crate) fn builtin_count(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "count")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let cnt_a = self.builder.build_alloca(i64, "cnt").map_err(llvm_err)?;
        self.builder
            .build_store(cnt_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let get_cache = self.alloc_list_get_cache()?;
        let hdr = self.context.append_basic_block(current_fn, "cnt_hdr");
        let bdy = self.context.append_basic_block(current_fn, "cnt_bdy");
        let ext = self.context.append_basic_block(current_fn, "cnt_ext");
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
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "cnt_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let one_or_zero = self
            .builder
            .build_int_z_extend(is_true, i64, "one_or_zero")
            .map_err(llvm_err)?;
        let cur = self
            .builder
            .build_load(i64, cnt_a, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let inc = self
            .builder
            .build_int_add(cur, one_or_zero, "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(cnt_a, inc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let result = self
            .builder
            .build_load(i64, cnt_a, "result")
            .map_err(llvm_err)?;
        Ok(TypedValue::Int(result.into_int_value()))
    }
}
