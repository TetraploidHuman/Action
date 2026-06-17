// P2: monomorphic lambda direct-call specialization for map/filter.
//
// Capture-free (or simple scalar-capture) lambdas compile to internal LLVM
// functions; map/filter call them directly instead of passing fn ptrs into
// action_list_map_walk / action_list_filter_walk.

use inkwell::types::BasicTypeEnum;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

/// A lambda that can be invoked with a direct LLVM call inside map/filter loops.
pub(super) enum DirectLambdaTarget<'ctx> {
    /// No captures: `lambda(arg)`.
    Plain(FunctionValue<'ctx>),
    /// Scalar captures only: `lambda(captures_ptr, arg)`.
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
            // Lambda returns __fat_ret {i64, ptr}; list_push expects __action_str {i64, ptr}.
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

    fn define_direct_map_walk_fn(
        &mut self,
        name: &str,
        target: &DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let saved = self.builder.get_insert_block();
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);

        let create_fn = self.module.get_function("action_list_create").unwrap();
        let get_fn = self.module.get_function("action_list_get").unwrap();
        let push_fn = self.module.get_function("action_list_push").unwrap();

        let walk_fn = self.module.add_function(
            name,
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(walk_fn, "entry");
        let loop_hdr = self.context.append_basic_block(walk_fn, "loop");
        let loop_body = self.context.append_basic_block(walk_fn, "body");
        let done = self.context.append_basic_block(walk_fn, "done");

        self.builder.position_at_end(entry);
        let input = walk_fn.get_first_param().unwrap().into_struct_value();
        let len = self
            .builder
            .build_extract_value(input, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();

        let acc_a = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;

        let init = self
            .builder
            .build_call(create_fn, &[len.into()], "init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("map direct: create failed")?;
        self.builder.build_store(acc_a, init).map_err(llvm_err)?;
        self.builder.build_store(i_a, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_hdr);
        let i_val = self
            .builder
            .build_load(i64, i_a, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let cur_list = self
            .builder
            .build_load(self.list_type, acc_a, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, i_val, len, "done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_cond, done, loop_body);

        self.builder.position_at_end(loop_body);
        let get_cc = self
            .builder
            .build_call(get_fn, &[input.into(), i_val.into()], "get")
            .map_err(llvm_err)?;
        let elem_fat = get_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map direct: get failed")?
            .into_struct_value();
        let elem_tag = self
            .builder
            .build_extract_value(elem_fat, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();

        let mapped_bv = self.emit_direct_lambda_call(target, elem_tag, "mapped")?;
        let mapped_fat = self.fat_struct_from_call_result(mapped_bv)?;

        let push_cc = self
            .builder
            .build_call(push_fn, &[cur_list.into(), mapped_fat.into()], "push")
            .map_err(llvm_err)?;
        let new_list = push_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map direct: push failed")?;
        self.builder
            .build_store(acc_a, new_list)
            .map_err(llvm_err)?;

        let next_i = self
            .builder
            .build_int_add(i_val, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, next_i).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(done);
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
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);

        let create_fn = self.module.get_function("action_list_create").unwrap();
        let get_fn = self.module.get_function("action_list_get").unwrap();
        let push_fn = self.module.get_function("action_list_push").unwrap();
        let len_fn = self.module.get_function("action_list_len").unwrap();

        let walk_fn = self.module.add_function(
            name,
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(walk_fn, "entry");
        let loop_hdr = self.context.append_basic_block(walk_fn, "loop");
        let loop_body = self.context.append_basic_block(walk_fn, "body");
        let push_bb = self.context.append_basic_block(walk_fn, "push");
        let skip_bb = self.context.append_basic_block(walk_fn, "skip");
        let done = self.context.append_basic_block(walk_fn, "done");

        self.builder.position_at_end(entry);
        let input = walk_fn.get_first_param().unwrap().into_struct_value();
        let len_cc = self
            .builder
            .build_call(len_fn, &[input.into()], "len")
            .map_err(llvm_err)?;
        let len = len_cc
            .try_as_basic_value()
            .basic()
            .ok_or("filter direct: len failed")?
            .into_int_value();

        let acc_a = self
            .builder
            .build_alloca(self.list_type, "acc")
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;

        let init = self
            .builder
            .build_call(create_fn, &[len.into()], "init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("filter direct: create failed")?;
        self.builder.build_store(acc_a, init).map_err(llvm_err)?;
        self.builder.build_store(i_a, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_hdr);
        let i_val = self
            .builder
            .build_load(i64, i_a, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let cur_list = self
            .builder
            .build_load(self.list_type, acc_a, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, i_val, len, "done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(done_cond, done, loop_body);

        self.builder.position_at_end(loop_body);
        let get_cc = self
            .builder
            .build_call(get_fn, &[input.into(), i_val.into()], "get")
            .map_err(llvm_err)?;
        let elem_fat = get_cc
            .try_as_basic_value()
            .basic()
            .ok_or("filter direct: get failed")?
            .into_struct_value();

        let elem_tag = self
            .builder
            .build_extract_value(elem_fat, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();

        let pred_bv = self.emit_direct_lambda_call(target, elem_tag, "pred")?;
        let pred_tag = self.fat_tag_from_call_result(pred_bv)?;
        let passes = self
            .builder
            .build_int_compare(IntPredicate::NE, pred_tag, zero, "passes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(passes, push_bb, skip_bb);

        self.builder.position_at_end(push_bb);
        let push_cc = self
            .builder
            .build_call(push_fn, &[cur_list.into(), elem_fat.into()], "push")
            .map_err(llvm_err)?;
        let new_list = push_cc
            .try_as_basic_value()
            .basic()
            .ok_or("filter direct: push failed")?;
        self.builder
            .build_store(acc_a, new_list)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(skip_bb);

        self.builder.position_at_end(skip_bb);
        let next_i = self
            .builder
            .build_int_add(i_val, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, next_i).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(done);
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
}
