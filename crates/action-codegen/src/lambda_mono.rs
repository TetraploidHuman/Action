// P2: monomorphic lambda direct-call specialization for map/filter/fold/any/all.
//
// Capture-free (or simple scalar-capture) lambdas compile to internal LLVM
// functions; higher-order builtins call them directly via B-tree walks instead
// of passing fn ptrs into action_list_*_walk runtime helpers.

use inkwell::types::BasicTypeEnum;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

const LEAF_BATCH: u64 = 64;

/// A lambda that can be invoked with a direct LLVM call inside list walks.
pub(super) enum DirectLambdaTarget<'ctx> {
    /// No captures: `lambda(arg)` or `lambda(acc, arg)`.
    Plain(FunctionValue<'ctx>),
    /// Scalar captures only: `lambda(captures_ptr, arg)` or `lambda(captures_ptr, acc, arg)`.
    WithCaptures {
        lambda_fn: FunctionValue<'ctx>,
        captures_ptr: PointerValue<'ctx>,
    },
}

impl<'ctx> CodeGen<'ctx> {
    /// If `tv` is a monomorphic lambda eligible for direct call, return its target.
    pub(super) fn try_direct_lambda(
        &self,
        tv: TypedValue<'ctx>,
    ) -> Option<DirectLambdaTarget<'ctx>> {
        match tv {
            TypedValue::Fn(fn_ptr, _) => {
                let lambda_fn = self.fn_ptr_to_internal_lambda(fn_ptr)?;
                Some(DirectLambdaTarget::Plain(lambda_fn))
            }
            TypedValue::Closure {
                fn_ptr,
                closure_ptr,
                closure_ty,
                alloca: None,
                ..
            } => {
                if !self.closure_has_simple_captures(closure_ty) {
                    return None;
                }
                let lambda_fn = self.fn_ptr_to_internal_lambda(fn_ptr)?;
                Some(DirectLambdaTarget::WithCaptures {
                    lambda_fn,
                    captures_ptr: closure_ptr,
                })
            }
            _ => None,
        }
    }

    fn fn_ptr_to_internal_lambda(&self, fn_ptr: PointerValue<'ctx>) -> Option<FunctionValue<'ctx>> {
        for f in self.module.get_functions() {
            let name = f.get_name();
            let name = name.to_str().ok()?;
            if !name.starts_with(".lambda_") {
                continue;
            }
            if f.as_global_value().as_pointer_value() == fn_ptr {
                return Some(f);
            }
        }
        None
    }

    fn closure_has_simple_captures(&self, closure_ty: inkwell::types::StructType<'ctx>) -> bool {
        let n = closure_ty.count_fields();
        for i in 0..n {
            let field = closure_ty.get_field_type_at_index(i).unwrap();
            if !matches!(
                field,
                BasicTypeEnum::IntType(_) | BasicTypeEnum::FloatType(_)
            ) {
                return false;
            }
        }
        true
    }

    fn direct_lambda_cache_key(&self, prefix: &str, target: &DirectLambdaTarget<'ctx>) -> String {
        let lambda_name = match target {
            DirectLambdaTarget::Plain(f) => f.get_name().to_string_lossy().into_owned(),
            DirectLambdaTarget::WithCaptures { lambda_fn, .. } => {
                lambda_fn.get_name().to_string_lossy().into_owned()
            }
        };
        format!("{prefix}_{lambda_name}")
    }

    fn emit_direct_lambda_call(
        &mut self,
        target: &DirectLambdaTarget<'ctx>,
        arg: IntValue<'ctx>,
        name: &str,
    ) -> Result<inkwell::values::BasicValueEnum<'ctx>, String> {
        let cc = match target {
            DirectLambdaTarget::Plain(f) => self
                .builder
                .build_call(*f, &[arg.into()], name)
                .map_err(llvm_err)?,
            DirectLambdaTarget::WithCaptures {
                lambda_fn,
                captures_ptr,
            } => self
                .builder
                .build_call(*lambda_fn, &[(*captures_ptr).into(), arg.into()], name)
                .map_err(llvm_err)?,
        };
        cc.try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("direct lambda call '{name}' returned void"))
    }

    fn emit_direct_lambda_call_2(
        &mut self,
        target: &DirectLambdaTarget<'ctx>,
        arg0: IntValue<'ctx>,
        arg1: IntValue<'ctx>,
        name: &str,
    ) -> Result<IntValue<'ctx>, String> {
        let cc = match target {
            DirectLambdaTarget::Plain(f) => self
                .builder
                .build_call(*f, &[arg0.into(), arg1.into()], name)
                .map_err(llvm_err)?,
            DirectLambdaTarget::WithCaptures {
                lambda_fn,
                captures_ptr,
            } => self
                .builder
                .build_call(
                    *lambda_fn,
                    &[(*captures_ptr).into(), arg0.into(), arg1.into()],
                    name,
                )
                .map_err(llvm_err)?,
        };
        let bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("direct lambda call '{name}' returned void"))?;
        self.fat_tag_from_call_result(bv)
    }

    fn fat_tag_from_call_result(
        &mut self,
        bv: inkwell::values::BasicValueEnum<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        if bv.is_struct_value() {
            Ok(self
                .builder
                .build_extract_value(bv.into_struct_value(), 0, "lam_tag")
                .map_err(llvm_err)?
                .into_int_value())
        } else {
            Ok(bv.into_int_value())
        }
    }

    fn fat_struct_from_call_result(
        &mut self,
        bv: inkwell::values::BasicValueEnum<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        if bv.is_struct_value() {
            let sv = bv.into_struct_value();
            if sv.get_type() == self.string_type {
                return Ok(sv);
            }
            let tag = self
                .builder
                .build_extract_value(sv, 0, "fat_tag")
                .map_err(llvm_err)?;
            let data = self
                .builder
                .build_extract_value(sv, 1, "fat_data")
                .map_err(llvm_err)?;
            let undef = self.string_type.get_undef();
            let s1 = self
                .builder
                .build_insert_value(undef, tag, 0, "str_tag")
                .map_err(llvm_err)?;
            let s2 = self
                .builder
                .build_insert_value(s1, data, 1, "str_data")
                .map_err(llvm_err)?;
            Ok(s2.into_struct_value())
        } else {
            self.make_int_fat(bv.into_int_value())
        }
    }

    fn bool_from_call_result(
        &mut self,
        bv: inkwell::values::BasicValueEnum<'ctx>,
        zero: IntValue<'ctx>,
    ) -> Result<IntValue<'ctx>, String> {
        let tag = self.fat_tag_from_call_result(bv)?;
        self.builder
            .build_int_compare(IntPredicate::NE, tag, zero, "pred_true")
            .map_err(llvm_err)
    }

    /// Monomorphized map: B-tree walk with direct lambda calls (no fn ptr param).
    pub(super) fn ensure_direct_map_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_map", &target);
        if !self.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct map walk '{cache_key}'"));
        }
        self.define_direct_map_walk_fn(&cache_key, &target)
    }

    /// Monomorphized filter: B-tree walk with direct lambda calls.
    pub(super) fn ensure_direct_filter_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_filter", &target);
        if !self.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct filter walk '{cache_key}'"));
        }
        self.define_direct_filter_walk_fn(&cache_key, &target)
    }

    /// Monomorphized fold: B-tree walk with direct lambda calls.
    pub(super) fn ensure_direct_fold_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_fold", &target);
        if !self.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct fold walk '{cache_key}'"));
        }
        self.define_direct_fold_walk_fn(&cache_key, &target)
    }

    /// Monomorphized any: B-tree walk with early exit.
    pub(super) fn ensure_direct_any_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_any", &target);
        if !self.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct any walk '{cache_key}'"));
        }
        self.define_direct_any_walk_fn(&cache_key, &target)
    }

    /// Monomorphized all: B-tree walk with early exit.
    pub(super) fn ensure_direct_all_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_all", &target);
        if !self.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct all walk '{cache_key}'"));
        }
        self.define_direct_all_walk_fn(&cache_key, &target)
    }

    fn define_direct_map_walk_fn(
        &mut self,
        name: &str,
        target: &DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let saved = self.builder.get_insert_block();
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let void = self.void_ty();
        let zero = i64.const_int(0, false);
        let batch = i64.const_int(LEAF_BATCH, false);

        let create_fn = self.module.get_function("action_list_create").unwrap();
        let push_leaf_fn = self.module.get_function("action_list_push_leaf").unwrap();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;

        let rec_name = format!("{name}_rec");
        let rec_fn = self.module.add_function(
            &rec_name,
            void.fn_type(
                &[ptr.into(), i64.into(), ptr.into(), ptr.into(), ptr.into()],
                false,
            ),
            None,
        );

        let r_entry = self.context.append_basic_block(rec_fn, "entry");
        let r_concat = self.context.append_basic_block(rec_fn, "concat");
        let r_normal = self.context.append_basic_block(rec_fn, "normal");
        let r_leaf_hdr = self.context.append_basic_block(rec_fn, "leaf_hdr");
        let r_leaf_bdy = self.context.append_basic_block(rec_fn, "leaf_bdy");
        let r_leaf_chk = self.context.append_basic_block(rec_fn, "leaf_chk");
        let r_leaf_flush = self.context.append_basic_block(rec_fn, "leaf_flush");
        let r_leaf_next = self.context.append_basic_block(rec_fn, "leaf_next");
        let r_leaf_done = self.context.append_basic_block(rec_fn, "leaf_done");
        let r_int_hdr = self.context.append_basic_block(rec_fn, "int_hdr");
        let r_int_bdy = self.context.append_basic_block(rec_fn, "int_bdy");
        let r_int_child = self.context.append_basic_block(rec_fn, "int_child");
        let r_int_next = self.context.append_basic_block(rec_fn, "int_next");

        self.builder.position_at_end(r_entry);
        let r_node = rec_fn.get_first_param().unwrap().into_pointer_value();
        let r_height = rec_fn.get_nth_param(1).unwrap().into_int_value();
        let r_acc = rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let r_buf_p = rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let r_buf_pos_p = rec_fn.get_nth_param(4).unwrap().into_pointer_value();
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
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    left_node.into(),
                    left_h.into(),
                    r_acc.into(),
                    r_buf_p.into(),
                    r_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    right_node.into(),
                    right_h.into(),
                    r_acc.into(),
                    r_buf_p.into(),
                    r_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

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
            .build_conditional_branch(done_leaf, r_leaf_done, r_leaf_chk);

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
        let mapped_bv = self.emit_direct_lambda_call(target, elem_tag, "mapped")?;
        let mapped_fat = self.fat_struct_from_call_result(mapped_bv)?;
        let buf = self
            .builder
            .build_load(ptr, r_buf_p, "buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pos = self
            .builder
            .build_load(i64, r_buf_pos_p, "pos")
            .map_err(llvm_err)?
            .into_int_value();
        let buf_i8 = self
            .builder
            .build_pointer_cast(buf, ptr, "buf_i8")
            .map_err(llvm_err)?;
        let buf_eb = unsafe {
            self.builder
                .build_gep(i8, buf_i8, &[i64.const_int(8, false)], "buf_eb")
                .map_err(llvm_err)?
        };
        let buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, buf_eb, &[pos], "buf_ep")
                .map_err(llvm_err)?
        };
        self.builder
            .build_store(buf_ep, mapped_fat)
            .map_err(llvm_err)?;
        let pos_inc = self
            .builder
            .build_int_add(pos, i64.const_int(1, false), "pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(r_buf_pos_p, pos_inc)
            .map_err(llvm_err)?;
        let buf_full = self
            .builder
            .build_int_compare(IntPredicate::EQ, pos_inc, batch, "buf_full")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(buf_full, r_leaf_flush, r_leaf_next);

        self.builder.position_at_end(r_leaf_flush);
        let flush_cnt = i32.const_int(LEAF_BATCH, false);
        self.builder
            .build_store(buf_i8, flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[r_acc.into(), buf.into()], "")
            .map_err(llvm_err)?;
        let new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let new_buf_i8 = self
            .builder
            .build_pointer_cast(new_buf, ptr, "new_buf_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(r_buf_p, new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(r_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_leaf_next);

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

        self.builder.position_at_end(r_leaf_done);
        let _ = self.builder.build_return(None);

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
            .build_conditional_branch(done_int, r_leaf_done, r_int_child);

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
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    child_ptr.into(),
                    child_h.into(),
                    r_acc.into(),
                    r_buf_p.into(),
                    r_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_int_next);

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

        // Wrapper: list -> mapped list
        let walk_fn = self.module.add_function(
            name,
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let w_entry = self.context.append_basic_block(walk_fn, "entry");
        let w_walk = self.context.append_basic_block(walk_fn, "walk");
        let w_flush = self.context.append_basic_block(walk_fn, "flush");
        let w_done = self.context.append_basic_block(walk_fn, "done");

        self.builder.position_at_end(w_entry);
        let input = walk_fn.get_first_param().unwrap().into_struct_value();
        let node = self
            .builder
            .build_extract_value(input, 0, "node")
            .map_err(llvm_err)?
            .into_pointer_value();
        let len = self
            .builder
            .build_extract_value(input, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let height = self
            .builder
            .build_extract_value(input, 2, "height")
            .map_err(llvm_err)?
            .into_int_value();
        let acc_a = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        let buf_p = self.builder.build_alloca(ptr, "buf_p").map_err(llvm_err)?;
        let buf_pos_p = self
            .builder
            .build_alloca(i64, "buf_pos")
            .map_err(llvm_err)?;
        let init = self
            .builder
            .build_call(create_fn, &[len.into()], "init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("map mono: create failed")?;
        self.builder.build_store(acc_a, init).map_err(llvm_err)?;
        let buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let buf_init_i8 = self
            .builder
            .build_pointer_cast(buf_init, ptr, "buf_init_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(buf_p, buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(w_walk);

        self.builder.position_at_end(w_walk);
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    node.into(),
                    height.into(),
                    acc_a.into(),
                    buf_p.into(),
                    buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let rem_pos = self
            .builder
            .build_load(i64, buf_pos_p, "rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, rem_pos, zero, "has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(has_rem, w_flush, w_done);

        self.builder.position_at_end(w_flush);
        let rem_buf = self
            .builder
            .build_load(ptr, buf_p, "rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rem_buf_i8 = self
            .builder
            .build_pointer_cast(rem_buf, ptr, "rem_buf_i8")
            .map_err(llvm_err)?;
        let rem_cnt = self
            .builder
            .build_int_truncate(rem_pos, i32, "rem_cnt")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rem_buf_i8, rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[acc_a.into(), rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(w_done);

        self.builder.position_at_end(w_done);
        let result = self
            .builder
            .build_load(self.list_type, acc_a, "res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&result));

        if let Some(bb) = saved {
            self.builder.position_at_end(bb);
        }
        Ok(walk_fn)
    }

    fn define_direct_filter_walk_fn(
        &mut self,
        name: &str,
        target: &DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let saved = self.builder.get_insert_block();
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let void = self.void_ty();
        let zero = i64.const_int(0, false);
        let batch = i64.const_int(LEAF_BATCH, false);

        let create_fn = self.module.get_function("action_list_create").unwrap();
        let len_fn = self.module.get_function("action_list_len").unwrap();
        let push_leaf_fn = self.module.get_function("action_list_push_leaf").unwrap();
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let leaf_sz = self.leaf_type.size_of().ok_or("leaf size")?;

        let rec_name = format!("{name}_rec");
        let rec_fn = self.module.add_function(
            &rec_name,
            void.fn_type(
                &[ptr.into(), i64.into(), ptr.into(), ptr.into(), ptr.into()],
                false,
            ),
            None,
        );

        let r_entry = self.context.append_basic_block(rec_fn, "entry");
        let r_concat = self.context.append_basic_block(rec_fn, "concat");
        let r_normal = self.context.append_basic_block(rec_fn, "normal");
        let r_leaf_hdr = self.context.append_basic_block(rec_fn, "leaf_hdr");
        let r_leaf_bdy = self.context.append_basic_block(rec_fn, "leaf_bdy");
        let r_leaf_chk = self.context.append_basic_block(rec_fn, "leaf_chk");
        let r_leaf_push = self.context.append_basic_block(rec_fn, "leaf_push");
        let r_leaf_flush = self.context.append_basic_block(rec_fn, "leaf_flush");
        let r_leaf_next = self.context.append_basic_block(rec_fn, "leaf_next");
        let r_leaf_done = self.context.append_basic_block(rec_fn, "leaf_done");
        let r_int_hdr = self.context.append_basic_block(rec_fn, "int_hdr");
        let r_int_bdy = self.context.append_basic_block(rec_fn, "int_bdy");
        let r_int_child = self.context.append_basic_block(rec_fn, "int_child");
        let r_int_next = self.context.append_basic_block(rec_fn, "int_next");

        self.builder.position_at_end(r_entry);
        let r_node = rec_fn.get_first_param().unwrap().into_pointer_value();
        let r_height = rec_fn.get_nth_param(1).unwrap().into_int_value();
        let r_acc = rec_fn.get_nth_param(2).unwrap().into_pointer_value();
        let r_buf_p = rec_fn.get_nth_param(3).unwrap().into_pointer_value();
        let r_buf_pos_p = rec_fn.get_nth_param(4).unwrap().into_pointer_value();
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
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    left_node.into(),
                    left_h.into(),
                    r_acc.into(),
                    r_buf_p.into(),
                    r_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    right_node.into(),
                    right_h.into(),
                    r_acc.into(),
                    r_buf_p.into(),
                    r_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

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
            .build_conditional_branch(done_leaf, r_leaf_done, r_leaf_chk);

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
        let passes = self.bool_from_call_result(pred_bv, zero)?;
        let _ = self
            .builder
            .build_conditional_branch(passes, r_leaf_push, r_leaf_next);

        self.builder.position_at_end(r_leaf_push);
        let buf = self
            .builder
            .build_load(ptr, r_buf_p, "buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let pos = self
            .builder
            .build_load(i64, r_buf_pos_p, "pos")
            .map_err(llvm_err)?
            .into_int_value();
        let buf_i8 = self
            .builder
            .build_pointer_cast(buf, ptr, "buf_i8")
            .map_err(llvm_err)?;
        let buf_eb = unsafe {
            self.builder
                .build_gep(i8, buf_i8, &[i64.const_int(8, false)], "buf_eb")
                .map_err(llvm_err)?
        };
        let buf_ep = unsafe {
            self.builder
                .build_gep(self.string_type, buf_eb, &[pos], "buf_ep")
                .map_err(llvm_err)?
        };
        self.builder.build_store(buf_ep, elem).map_err(llvm_err)?;
        let pos_inc = self
            .builder
            .build_int_add(pos, i64.const_int(1, false), "pos_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(r_buf_pos_p, pos_inc)
            .map_err(llvm_err)?;
        let buf_full = self
            .builder
            .build_int_compare(IntPredicate::EQ, pos_inc, batch, "buf_full")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(buf_full, r_leaf_flush, r_leaf_next);

        self.builder.position_at_end(r_leaf_flush);
        let flush_cnt = i32.const_int(LEAF_BATCH, false);
        self.builder
            .build_store(buf_i8, flush_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[r_acc.into(), buf.into()], "")
            .map_err(llvm_err)?;
        let new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let new_buf_i8 = self
            .builder
            .build_pointer_cast(new_buf, ptr, "new_buf_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(new_buf_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(r_buf_p, new_buf)
            .map_err(llvm_err)?;
        self.builder
            .build_store(r_buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_leaf_next);

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

        self.builder.position_at_end(r_leaf_done);
        let _ = self.builder.build_return(None);

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
            .build_conditional_branch(done_int, r_leaf_done, r_int_child);

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
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    child_ptr.into(),
                    child_h.into(),
                    r_acc.into(),
                    r_buf_p.into(),
                    r_buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_int_next);

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

        let walk_fn = self.module.add_function(
            name,
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let w_entry = self.context.append_basic_block(walk_fn, "entry");
        let w_walk = self.context.append_basic_block(walk_fn, "walk");
        let w_flush = self.context.append_basic_block(walk_fn, "flush");
        let w_done = self.context.append_basic_block(walk_fn, "done");

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
        let len_cc = self
            .builder
            .build_call(len_fn, &[input.into()], "len")
            .map_err(llvm_err)?;
        let len = len_cc
            .try_as_basic_value()
            .basic()
            .ok_or("filter mono: len failed")?
            .into_int_value();
        let acc_a = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        let buf_p = self.builder.build_alloca(ptr, "buf_p").map_err(llvm_err)?;
        let buf_pos_p = self
            .builder
            .build_alloca(i64, "buf_pos")
            .map_err(llvm_err)?;
        let init = self
            .builder
            .build_call(create_fn, &[len.into()], "init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("filter mono: create failed")?;
        self.builder.build_store(acc_a, init).map_err(llvm_err)?;
        let buf_init = self
            .builder
            .build_call(malloc_rc_fn, &[leaf_sz.into()], "buf_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let buf_init_i8 = self
            .builder
            .build_pointer_cast(buf_init, ptr, "buf_init_i8")
            .map_err(llvm_err)?;
        self.builder
            .build_store(buf_init_i8, zero)
            .map_err(llvm_err)?;
        self.builder
            .build_store(buf_p, buf_init)
            .map_err(llvm_err)?;
        self.builder
            .build_store(buf_pos_p, zero)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(w_walk);

        self.builder.position_at_end(w_walk);
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[
                    node.into(),
                    height.into(),
                    acc_a.into(),
                    buf_p.into(),
                    buf_pos_p.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let rem_pos = self
            .builder
            .build_load(i64, buf_pos_p, "rem_pos")
            .map_err(llvm_err)?
            .into_int_value();
        let has_rem = self
            .builder
            .build_int_compare(IntPredicate::SGT, rem_pos, zero, "has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(has_rem, w_flush, w_done);

        self.builder.position_at_end(w_flush);
        let rem_buf = self
            .builder
            .build_load(ptr, buf_p, "rem_buf")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rem_buf_i8 = self
            .builder
            .build_pointer_cast(rem_buf, ptr, "rem_buf_i8")
            .map_err(llvm_err)?;
        let rem_cnt = self
            .builder
            .build_int_truncate(rem_pos, i32, "rem_cnt")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rem_buf_i8, rem_cnt)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(push_leaf_fn, &[acc_a.into(), rem_buf.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(w_done);

        self.builder.position_at_end(w_done);
        let result = self
            .builder
            .build_load(self.list_type, acc_a, "res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&result));

        if let Some(bb) = saved {
            self.builder.position_at_end(bb);
        }
        Ok(walk_fn)
    }

    fn define_direct_fold_walk_fn(
        &mut self,
        name: &str,
        target: &DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let saved = self.builder.get_insert_block();
        let i64 = self.i64_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        let ptr = self.ptr_ty();
        let void = self.void_ty();
        let zero = i64.const_int(0, false);

        let rec_name = format!("{name}_rec");
        let rec_fn = self.module.add_function(
            &rec_name,
            void.fn_type(&[ptr.into(), i64.into(), ptr.into()], false),
            None,
        );

        let r_entry = self.context.append_basic_block(rec_fn, "entry");
        let r_concat = self.context.append_basic_block(rec_fn, "concat");
        let r_normal = self.context.append_basic_block(rec_fn, "normal");
        let r_leaf_hdr = self.context.append_basic_block(rec_fn, "leaf_hdr");
        let r_leaf_bdy = self.context.append_basic_block(rec_fn, "leaf_bdy");
        let r_leaf_chk = self.context.append_basic_block(rec_fn, "leaf_chk");
        let r_leaf_next = self.context.append_basic_block(rec_fn, "leaf_next");
        let r_leaf_done = self.context.append_basic_block(rec_fn, "leaf_done");
        let r_int_hdr = self.context.append_basic_block(rec_fn, "int_hdr");
        let r_int_bdy = self.context.append_basic_block(rec_fn, "int_bdy");
        let r_int_child = self.context.append_basic_block(rec_fn, "int_child");
        let r_int_next = self.context.append_basic_block(rec_fn, "int_next");

        self.builder.position_at_end(r_entry);
        let r_node = rec_fn.get_first_param().unwrap().into_pointer_value();
        let r_height = rec_fn.get_nth_param(1).unwrap().into_int_value();
        let r_acc = rec_fn.get_nth_param(2).unwrap().into_pointer_value();
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
        let _ = self
            .builder
            .build_call(rec_fn, &[left_node.into(), left_h.into(), r_acc.into()], "")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[right_node.into(), right_h.into(), r_acc.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(None);

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
            .build_conditional_branch(done_leaf, r_leaf_done, r_leaf_chk);

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
        let elem_tag = self
            .builder
            .build_load(i64, ep, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let cur_acc = self
            .builder
            .build_load(i64, r_acc, "cur_acc")
            .map_err(llvm_err)?
            .into_int_value();
        let new_acc = self.emit_direct_lambda_call_2(target, cur_acc, elem_tag, "folded")?;
        self.builder.build_store(r_acc, new_acc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_leaf_next);

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

        self.builder.position_at_end(r_leaf_done);
        let _ = self.builder.build_return(None);

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
            .build_conditional_branch(done_int, r_leaf_done, r_int_child);

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
        let _ = self
            .builder
            .build_call(
                rec_fn,
                &[child_ptr.into(), child_h.into(), r_acc.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(r_int_next);

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

        let walk_fn = self.module.add_function(
            name,
            i64.fn_type(&[self.list_type.into(), i64.into()], false),
            None,
        );
        let w_entry = self.context.append_basic_block(walk_fn, "entry");
        let w_walk = self.context.append_basic_block(walk_fn, "walk");

        self.builder.position_at_end(w_entry);
        let input = walk_fn.get_first_param().unwrap().into_struct_value();
        let init = walk_fn.get_nth_param(1).unwrap().into_int_value();
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
        let acc_a = self.builder.build_alloca(i64, "acc").map_err(llvm_err)?;
        self.builder.build_store(acc_a, init).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(w_walk);

        self.builder.position_at_end(w_walk);
        let _ = self
            .builder
            .build_call(rec_fn, &[node.into(), height.into(), acc_a.into()], "")
            .map_err(llvm_err)?;
        let result = self
            .builder
            .build_load(i64, acc_a, "res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&result));

        if let Some(bb) = saved {
            self.builder.position_at_end(bb);
        }
        Ok(walk_fn)
    }

    fn define_direct_any_walk_fn(
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

    fn define_direct_all_walk_fn(
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
    pub(super) fn try_builtin_map_direct(
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
    pub(super) fn try_builtin_filter_direct(
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
    pub(super) fn try_builtin_fold_direct(
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
    pub(super) fn try_builtin_any_direct(
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
    pub(super) fn try_builtin_all_direct(
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
