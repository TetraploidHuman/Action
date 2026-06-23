//! Monomorphic lambda direct-call specialization (R4-2).

use inkwell::values::FunctionValue;
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, DirectLambdaTarget, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn define_direct_any_walk_fn(
        &mut self,
        name: &str,
        target: &DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let saved = self.builder.get_insert_block();
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let b1 = self.bool_ty();
        let zero = i64.const_int(0, false);
        let one = b1.const_int(1, false);
        let false_val = b1.const_int(0, false);

        let rec_name = format!("{name}_rec");
        let rec_fn = self.module.add_function(
            &rec_name,
            b1.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );

        let r_entry = self.context.append_basic_block(rec_fn, "entry");
        let r_true = self.context.append_basic_block(rec_fn, "any_true");
        let r_false = self.context.append_basic_block(rec_fn, "any_false");
        let r_concat = self.context.append_basic_block(rec_fn, "concat");
        let r_concat_right = self.context.append_basic_block(rec_fn, "concat_right");
        let r_normal = self.context.append_basic_block(rec_fn, "normal");
        let r_leaf_hdr = self.context.append_basic_block(rec_fn, "leaf_hdr");
        let r_leaf_bdy = self.context.append_basic_block(rec_fn, "leaf_bdy");
        let r_leaf_chk = self.context.append_basic_block(rec_fn, "leaf_chk");
        let r_leaf_next = self.context.append_basic_block(rec_fn, "leaf_next");
        let r_int_hdr = self.context.append_basic_block(rec_fn, "int_hdr");
        let r_int_bdy = self.context.append_basic_block(rec_fn, "int_bdy");
        let r_int_child = self.context.append_basic_block(rec_fn, "int_child");
        let r_int_next = self.context.append_basic_block(rec_fn, "int_next");

        self.builder.position_at_end(r_entry);
        let r_node = rec_fn.get_first_param().unwrap().into_pointer_value();
        let r_height = rec_fn.get_nth_param(1).unwrap().into_int_value();
        let neg1 = i64.const_int(-1i64 as u64, true);
        let is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, r_height, neg1, "is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_concat, r_concat, r_normal);

        self.builder.position_at_end(r_concat);
        let ln_p = unsafe {
            self.builder
                .build_gep(ptr, r_node, &[i64.const_int(2, false)], "ln_p")
                .map_err(llvm_err)?
        };
        let left_node = self
            .builder
            .build_load(ptr, ln_p, "ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lh_p = unsafe {
            self.builder
                .build_gep(i64, r_node, &[i64.const_int(4, false)], "lh_p")
                .map_err(llvm_err)?
        };
        let left_h = self
            .builder
            .build_load(i64, lh_p, "lh")
            .map_err(llvm_err)?
            .into_int_value();
        let rn_p = unsafe {
            self.builder
                .build_gep(ptr, r_node, &[i64.const_int(5, false)], "rn_p")
                .map_err(llvm_err)?
        };
        let right_node = self
            .builder
            .build_load(ptr, rn_p, "rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rh_p = unsafe {
            self.builder
                .build_gep(i64, r_node, &[i64.const_int(7, false)], "rh_p")
                .map_err(llvm_err)?
        };
        let right_h = self
            .builder
            .build_load(i64, rh_p, "rh")
            .map_err(llvm_err)?
            .into_int_value();
        let lhit = self
            .builder
            .build_call(rec_fn, &[left_node.into(), left_h.into()], "lhit")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(lhit, r_true, r_concat_right);

        self.builder.position_at_end(r_concat_right);
        let rhit = self
            .builder
            .build_call(rec_fn, &[right_node.into(), right_h.into()], "rhit")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&rhit));

        self.builder.position_at_end(r_normal);
        let is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, r_height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_leaf, r_leaf_hdr, r_int_hdr);

        self.builder.position_at_end(r_leaf_hdr);
        let leaf_i8 = self
            .builder
            .build_pointer_cast(r_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let count_raw = self
            .builder
            .build_load(i32, leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let count = self
            .builder
            .build_int_z_extend(count_raw, i64, "count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_leaf_bdy);

        self.builder.position_at_end(r_leaf_bdy);
        let li = self.builder.build_phi(i64, "li").map_err(llvm_err)?;
        let done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                li.as_basic_value().into_int_value(),
                count,
                "done_leaf",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_leaf, r_false, r_leaf_chk);

        self.builder.position_at_end(r_leaf_chk);
        let eb = unsafe {
            self.builder
                .build_gep(i8, leaf_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)?
        };
        let ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    eb,
                    &[li.as_basic_value().into_int_value()],
                    "ep",
                )
                .map_err(llvm_err)?
        };
        let elem = self
            .builder
            .build_load(self.string_type, ep, "elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let elem_tag = self
            .builder
            .build_extract_value(elem, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let pred_bv = self.emit_direct_lambda_call(target, elem_tag, "pred")?;
        let is_true = self.bool_from_call_result(pred_bv, zero)?;
        let _ = self
            .builder
            .build_conditional_branch(is_true, r_true, r_leaf_next);

        self.builder.position_at_end(r_leaf_next);
        let next_i = self
            .builder
            .build_int_add(
                li.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ni",
            )
            .map_err(llvm_err)?;
        let leaf_next_bb = self.builder.get_insert_block().unwrap();
        li.add_incoming(&[(&zero, r_leaf_hdr), (&next_i, leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(r_leaf_bdy);

        self.builder.position_at_end(r_int_hdr);
        let int_i8 = self
            .builder
            .build_pointer_cast(r_node, ptr, "int_i8")
            .map_err(llvm_err)?;
        let child_count_raw = self
            .builder
            .build_load(i32, int_i8, "cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let child_count = self
            .builder
            .build_int_z_extend(child_count_raw, i64, "cc")
            .map_err(llvm_err)?;
        let child_h = self
            .builder
            .build_int_sub(r_height, i64.const_int(1, false), "ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_int_bdy);

        self.builder.position_at_end(r_int_bdy);
        let ci = self.builder.build_phi(i64, "ci").map_err(llvm_err)?;
        let done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                ci.as_basic_value().into_int_value(),
                child_count,
                "done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_int, r_false, r_int_child);

        self.builder.position_at_end(r_int_child);
        let children_base = unsafe {
            self.builder
                .build_gep(i8, int_i8, &[i64.const_int(16, false)], "cb")
                .map_err(llvm_err)?
        };
        let child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[ci.as_basic_value().into_int_value()],
                    "cep",
                )
                .map_err(llvm_err)?
        };
        let child_entry = self
            .builder
            .build_load(self.child_entry_type, child_ep, "ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let child_ptr = self
            .builder
            .build_extract_value(child_entry, 0, "cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let child_hit = self
            .builder
            .build_call(rec_fn, &[child_ptr.into(), child_h.into()], "hit")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(child_hit, r_true, r_int_next);

        self.builder.position_at_end(r_int_next);
        let next_ci = self
            .builder
            .build_int_add(
                ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "nci",
            )
            .map_err(llvm_err)?;
        let int_next_bb = self.builder.get_insert_block().unwrap();
        ci.add_incoming(&[(&zero, r_int_hdr), (&next_ci, int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(r_int_bdy);

        self.builder.position_at_end(r_true);
        let _ = self.builder.build_return(Some(&one));
        self.builder.position_at_end(r_false);
        let _ = self.builder.build_return(Some(&false_val));

        let walk_fn =
            self.module
                .add_function(name, b1.fn_type(&[self.list_type.into()], false), None);
        let w_entry = self.context.append_basic_block(walk_fn, "entry");
        let w_walk = self.context.append_basic_block(walk_fn, "walk");

        self.builder.position_at_end(w_entry);
        let input = walk_fn.get_first_param().unwrap().into_struct_value();
        let node = self
            .builder
            .build_extract_value(input, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let height = self
            .builder
            .build_extract_value(input, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(w_walk);

        self.builder.position_at_end(w_walk);
        let hit = self
            .builder
            .build_call(rec_fn, &[node.into(), height.into()], "hit")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&hit));

        if let Some(bb) = saved {
            self.builder.position_at_end(bb);
        }
        Ok(walk_fn)
    }

    pub(crate) fn define_direct_all_walk_fn(
        &mut self,
        name: &str,
        target: &DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let saved = self.builder.get_insert_block();
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let b1 = self.bool_ty();
        let zero = i64.const_int(0, false);
        let one = b1.const_int(1, false);
        let false_val = b1.const_int(0, false);

        let rec_name = format!("{name}_rec");
        let rec_fn = self.module.add_function(
            &rec_name,
            b1.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );

        let r_entry = self.context.append_basic_block(rec_fn, "entry");
        let r_true = self.context.append_basic_block(rec_fn, "all_true");
        let r_false = self.context.append_basic_block(rec_fn, "all_false");
        let r_concat = self.context.append_basic_block(rec_fn, "concat");
        let r_concat_right = self.context.append_basic_block(rec_fn, "concat_right");
        let r_normal = self.context.append_basic_block(rec_fn, "normal");
        let r_leaf_hdr = self.context.append_basic_block(rec_fn, "leaf_hdr");
        let r_leaf_bdy = self.context.append_basic_block(rec_fn, "leaf_bdy");
        let r_leaf_chk = self.context.append_basic_block(rec_fn, "leaf_chk");
        let r_leaf_next = self.context.append_basic_block(rec_fn, "leaf_next");
        let r_int_hdr = self.context.append_basic_block(rec_fn, "int_hdr");
        let r_int_bdy = self.context.append_basic_block(rec_fn, "int_bdy");
        let r_int_child = self.context.append_basic_block(rec_fn, "int_child");
        let r_int_next = self.context.append_basic_block(rec_fn, "int_next");

        self.builder.position_at_end(r_entry);
        let r_node = rec_fn.get_first_param().unwrap().into_pointer_value();
        let r_height = rec_fn.get_nth_param(1).unwrap().into_int_value();
        let neg1 = i64.const_int(-1i64 as u64, true);
        let is_concat = self
            .builder
            .build_int_compare(IntPredicate::EQ, r_height, neg1, "is_concat")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_concat, r_concat, r_normal);

        self.builder.position_at_end(r_concat);
        let ln_p = unsafe {
            self.builder
                .build_gep(ptr, r_node, &[i64.const_int(2, false)], "ln_p")
                .map_err(llvm_err)?
        };
        let left_node = self
            .builder
            .build_load(ptr, ln_p, "ln")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lh_p = unsafe {
            self.builder
                .build_gep(i64, r_node, &[i64.const_int(4, false)], "lh_p")
                .map_err(llvm_err)?
        };
        let left_h = self
            .builder
            .build_load(i64, lh_p, "lh")
            .map_err(llvm_err)?
            .into_int_value();
        let rn_p = unsafe {
            self.builder
                .build_gep(ptr, r_node, &[i64.const_int(5, false)], "rn_p")
                .map_err(llvm_err)?
        };
        let right_node = self
            .builder
            .build_load(ptr, rn_p, "rn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rh_p = unsafe {
            self.builder
                .build_gep(i64, r_node, &[i64.const_int(7, false)], "rh_p")
                .map_err(llvm_err)?
        };
        let right_h = self
            .builder
            .build_load(i64, rh_p, "rh")
            .map_err(llvm_err)?
            .into_int_value();
        let lpass = self
            .builder
            .build_call(rec_fn, &[left_node.into(), left_h.into()], "lpass")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(lpass, r_concat_right, r_false);

        self.builder.position_at_end(r_concat_right);
        let rpass = self
            .builder
            .build_call(rec_fn, &[right_node.into(), right_h.into()], "rpass")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&rpass));

        self.builder.position_at_end(r_normal);
        let is_leaf = self
            .builder
            .build_int_compare(IntPredicate::EQ, r_height, zero, "is_leaf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_leaf, r_leaf_hdr, r_int_hdr);

        self.builder.position_at_end(r_leaf_hdr);
        let leaf_i8 = self
            .builder
            .build_pointer_cast(r_node, ptr, "leaf_i8")
            .map_err(llvm_err)?;
        let count_raw = self
            .builder
            .build_load(i32, leaf_i8, "count_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let count = self
            .builder
            .build_int_z_extend(count_raw, i64, "count")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_leaf_bdy);

        self.builder.position_at_end(r_leaf_bdy);
        let li = self.builder.build_phi(i64, "li").map_err(llvm_err)?;
        let done_leaf = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                li.as_basic_value().into_int_value(),
                count,
                "done_leaf",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_leaf, r_true, r_leaf_chk);

        self.builder.position_at_end(r_leaf_chk);
        let eb = unsafe {
            self.builder
                .build_gep(i8, leaf_i8, &[i64.const_int(8, false)], "eb")
                .map_err(llvm_err)?
        };
        let ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    eb,
                    &[li.as_basic_value().into_int_value()],
                    "ep",
                )
                .map_err(llvm_err)?
        };
        let elem = self
            .builder
            .build_load(self.string_type, ep, "elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let elem_tag = self
            .builder
            .build_extract_value(elem, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let pred_bv = self.emit_direct_lambda_call(target, elem_tag, "pred")?;
        let is_true = self.bool_from_call_result(pred_bv, zero)?;
        let _ = self
            .builder
            .build_conditional_branch(is_true, r_leaf_next, r_false);

        self.builder.position_at_end(r_leaf_next);
        let next_i = self
            .builder
            .build_int_add(
                li.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ni",
            )
            .map_err(llvm_err)?;
        let leaf_next_bb = self.builder.get_insert_block().unwrap();
        li.add_incoming(&[(&zero, r_leaf_hdr), (&next_i, leaf_next_bb)]);
        let _ = self.builder.build_unconditional_branch(r_leaf_bdy);

        self.builder.position_at_end(r_int_hdr);
        let int_i8 = self
            .builder
            .build_pointer_cast(r_node, ptr, "int_i8")
            .map_err(llvm_err)?;
        let child_count_raw = self
            .builder
            .build_load(i32, int_i8, "cc_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let child_count = self
            .builder
            .build_int_z_extend(child_count_raw, i64, "cc")
            .map_err(llvm_err)?;
        let child_h = self
            .builder
            .build_int_sub(r_height, i64.const_int(1, false), "ch")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_int_bdy);

        self.builder.position_at_end(r_int_bdy);
        let ci = self.builder.build_phi(i64, "ci").map_err(llvm_err)?;
        let done_int = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                ci.as_basic_value().into_int_value(),
                child_count,
                "done_int",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_int, r_true, r_int_child);

        self.builder.position_at_end(r_int_child);
        let children_base = unsafe {
            self.builder
                .build_gep(i8, int_i8, &[i64.const_int(16, false)], "cb")
                .map_err(llvm_err)?
        };
        let child_ep = unsafe {
            self.builder
                .build_gep(
                    self.child_entry_type,
                    children_base,
                    &[ci.as_basic_value().into_int_value()],
                    "cep",
                )
                .map_err(llvm_err)?
        };
        let child_entry = self
            .builder
            .build_load(self.child_entry_type, child_ep, "ce")
            .map_err(llvm_err)?
            .into_struct_value();
        let child_ptr = self
            .builder
            .build_extract_value(child_entry, 0, "cp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let child_pass = self
            .builder
            .build_call(rec_fn, &[child_ptr.into(), child_h.into()], "pass")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(child_pass, r_int_next, r_false);

        self.builder.position_at_end(r_int_next);
        let next_ci = self
            .builder
            .build_int_add(
                ci.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "nci",
            )
            .map_err(llvm_err)?;
        let int_next_bb = self.builder.get_insert_block().unwrap();
        ci.add_incoming(&[(&zero, r_int_hdr), (&next_ci, int_next_bb)]);
        let _ = self.builder.build_unconditional_branch(r_int_bdy);

        self.builder.position_at_end(r_true);
        let _ = self.builder.build_return(Some(&one));
        self.builder.position_at_end(r_false);
        let _ = self.builder.build_return(Some(&false_val));

        let walk_fn =
            self.module
                .add_function(name, b1.fn_type(&[self.list_type.into()], false), None);
        let w_entry = self.context.append_basic_block(walk_fn, "entry");
        let w_walk = self.context.append_basic_block(walk_fn, "walk");

        self.builder.position_at_end(w_entry);
        let input = walk_fn.get_first_param().unwrap().into_struct_value();
        let node = self
            .builder
            .build_extract_value(input, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let height = self
            .builder
            .build_extract_value(input, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(w_walk);

        self.builder.position_at_end(w_walk);
        let pass = self
            .builder
            .build_call(rec_fn, &[node.into(), height.into()], "pass")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&pass));

        if let Some(bb) = saved {
            self.builder.position_at_end(bb);
        }
        Ok(walk_fn)
    }

    /// Run map via monomorphized direct-call walk when eligible; otherwise None.
    pub(crate) fn try_builtin_map_direct(
        &mut self,
        fn_val: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let target = match self.try_direct_lambda(fn_val) {
            Some(t) => t,
            None => return Ok(None),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Ok(None),
        };
        let list_struct = self.load_list(list_ptr)?;
        let walk_fn = self.ensure_direct_map_walk(target)?;
        let cc = self
            .builder
            .build_call(walk_fn, &[list_struct.into()], "mono_map")
            .map_err(llvm_err)?;
        let result_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("mono map call failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "map_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, result_bv)
            .map_err(llvm_err)?;
        Ok(Some(TypedValue::List(result_alloca)))
    }

    /// Run filter via monomorphized direct-call walk when eligible; otherwise None.
    pub(crate) fn try_builtin_filter_direct(
        &mut self,
        fn_val: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let target = match self.try_direct_lambda(fn_val) {
            Some(t) => t,
            None => return Ok(None),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Ok(None),
        };
        let list_struct = self.load_list(list_ptr)?;
        let walk_fn = self.ensure_direct_filter_walk(target)?;
        let cc = self
            .builder
            .build_call(walk_fn, &[list_struct.into()], "mono_filter")
            .map_err(llvm_err)?;
        let result_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("mono filter call failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "filter_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, result_bv)
            .map_err(llvm_err)?;
        Ok(Some(TypedValue::List(result_alloca)))
    }

    /// Run fold via monomorphized direct-call walk when eligible; otherwise None.
    pub(crate) fn try_builtin_fold_direct(
        &mut self,
        fn_val: TypedValue<'ctx>,
        init_val: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let target = match self.try_direct_lambda(fn_val) {
            Some(t) => t,
            None => return Ok(None),
        };
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Ok(None),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Ok(None),
        };
        let list_struct = self.load_list(list_ptr)?;
        let walk_fn = self.ensure_direct_fold_walk(target)?;
        let cc = self
            .builder
            .build_call(walk_fn, &[list_struct.into(), init_i64.into()], "mono_fold")
            .map_err(llvm_err)?;
        let final_acc = cc
            .try_as_basic_value()
            .basic()
            .ok_or("mono fold call failed")?
            .into_int_value();
        Ok(Some(TypedValue::Int(final_acc)))
    }

    /// Run any via monomorphized direct-call walk when eligible; otherwise None.
    pub(crate) fn try_builtin_any_direct(
        &mut self,
        fn_val: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let target = match self.try_direct_lambda(fn_val) {
            Some(t) => t,
            None => return Ok(None),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Ok(None),
        };
        let list_struct = self.load_list(list_ptr)?;
        let walk_fn = self.ensure_direct_any_walk(target)?;
        let cc = self
            .builder
            .build_call(walk_fn, &[list_struct.into()], "mono_any")
            .map_err(llvm_err)?;
        let res = cc
            .try_as_basic_value()
            .basic()
            .ok_or("mono any call failed")?
            .into_int_value();
        Ok(Some(TypedValue::Bool(res)))
    }

    /// Run all via monomorphized direct-call walk when eligible; otherwise None.
    pub(crate) fn try_builtin_all_direct(
        &mut self,
        fn_val: TypedValue<'ctx>,
        list_val: TypedValue<'ctx>,
    ) -> Result<Option<TypedValue<'ctx>>, String> {
        let target = match self.try_direct_lambda(fn_val) {
            Some(t) => t,
            None => return Ok(None),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Ok(None),
        };
        let list_struct = self.load_list(list_ptr)?;
        let walk_fn = self.ensure_direct_all_walk(target)?;
        let cc = self
            .builder
            .build_call(walk_fn, &[list_struct.into()], "mono_all")
            .map_err(llvm_err)?;
        let res = cc
            .try_as_basic_value()
            .basic()
            .ok_or("mono all call failed")?
            .into_int_value();
        Ok(Some(TypedValue::Bool(res)))
    }
}
