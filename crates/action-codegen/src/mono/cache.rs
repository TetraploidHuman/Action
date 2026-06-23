//! Monomorphic lambda direct-call specialization (R4-2).

use inkwell::types::BasicTypeEnum;
use inkwell::values::{FunctionValue, IntValue, PointerValue};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, DirectLambdaTarget, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn try_direct_lambda(
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

    pub(crate) fn fn_ptr_to_internal_lambda(
        &self,
        fn_ptr: PointerValue<'ctx>,
    ) -> Option<FunctionValue<'ctx>> {
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

    pub(crate) fn closure_has_simple_captures(
        &self,
        closure_ty: inkwell::types::StructType<'ctx>,
    ) -> bool {
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

    pub(crate) fn direct_lambda_cache_key(
        &self,
        prefix: &str,
        target: &DirectLambdaTarget<'ctx>,
    ) -> String {
        let lambda_name = match target {
            DirectLambdaTarget::Plain(f) => f.get_name().to_string_lossy().into_owned(),
            DirectLambdaTarget::WithCaptures { lambda_fn, .. } => {
                lambda_fn.get_name().to_string_lossy().into_owned()
            }
        };
        format!("{prefix}_{lambda_name}")
    }

    pub(crate) fn emit_direct_lambda_call(
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

    pub(crate) fn emit_direct_lambda_call_2(
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

    pub(crate) fn fat_tag_from_call_result(
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

    pub(crate) fn fat_struct_from_call_result(
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

    pub(crate) fn bool_from_call_result(
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
    pub(crate) fn ensure_direct_map_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_map", &target);
        if !self.mono_cache.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct map walk '{cache_key}'"));
        }
        self.define_direct_map_walk_fn(&cache_key, &target)
    }

    /// Monomorphized filter: B-tree walk with direct lambda calls.
    pub(crate) fn ensure_direct_filter_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_filter", &target);
        if !self.mono_cache.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct filter walk '{cache_key}'"));
        }
        self.define_direct_filter_walk_fn(&cache_key, &target)
    }

    /// Monomorphized fold: B-tree walk with direct lambda calls.
    pub(crate) fn ensure_direct_fold_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_fold", &target);
        if !self.mono_cache.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct fold walk '{cache_key}'"));
        }
        self.define_direct_fold_walk_fn(&cache_key, &target)
    }

    /// Monomorphized any: B-tree walk with early exit.
    pub(crate) fn ensure_direct_any_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_any", &target);
        if !self.mono_cache.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct any walk '{cache_key}'"));
        }
        self.define_direct_any_walk_fn(&cache_key, &target)
    }

    /// Monomorphized all: B-tree walk with early exit.
    pub(crate) fn ensure_direct_all_walk(
        &mut self,
        target: DirectLambdaTarget<'ctx>,
    ) -> Result<FunctionValue<'ctx>, String> {
        let cache_key = self.direct_lambda_cache_key(".mono_all", &target);
        if !self.mono_cache.monomorphized_fns.insert(cache_key.clone()) {
            return self
                .module
                .get_function(&cache_key)
                .ok_or_else(|| format!("missing cached direct all walk '{cache_key}'"));
        }
        self.define_direct_all_walk_fn(&cache_key, &target)
    }
}
