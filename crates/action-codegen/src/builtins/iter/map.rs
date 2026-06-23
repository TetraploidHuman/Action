//! Iterator builtins: map, filter, fold, find (R4-1).

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_map(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // map(filter(map(base))) { outer } — fuse inner map+filter; skip identity outer map
        if let Some(lam) = trailing {
            if args.len() == 1 {
                let CallArg::Hir(list_hir) = &args[0];
                if let Some((filter_fn_hir, inner)) = Self::extract_filter_call_args_hir(list_hir) {
                    if let Some((map_inner_hir, base_list)) = Self::extract_map_call_args_hir(inner)
                    {
                        let filter_fn_val = self.compile_hir_expr(filter_fn_hir)?;
                        if Self::is_identity_lambda_call_arg(&lam) {
                            return self.fused_map_filter_hir(
                                map_inner_hir,
                                base_list,
                                filter_fn_val,
                            );
                        }
                        let outer_fn = self.compile_call_arg(lam)?;
                        return self.fused_map_filter_map_hir(
                            map_inner_hir,
                            base_list,
                            filter_fn_val,
                            outer_fn,
                        );
                    }
                }
            }
        }

        // map(fn, list) or map(list) { lambda }
        let (fn_ptr, list_val) = if let Some(lam) = trailing {
            // map(list) { lambda }
            if args.len() != 1 {
                return Err("map with trailing lambda expects 1 argument (list)".to_string());
            }
            let lv = self.compile_call_arg(args[0])?;
            let fv = self.compile_call_arg(lam)?;
            (fv, lv)
        } else if args.len() == 2 {
            let fv = self.compile_call_arg(args[0])?;
            let lv = self.compile_call_arg(args[1])?;
            (fv, lv)
        } else {
            return Err("map expects 2 arguments (fn, list)".to_string());
        };

        if let Some(result) = self.try_builtin_map_direct(fn_ptr, list_val)? {
            return Ok(result);
        }

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("map: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("map: second argument must be a list".to_string()),
        };

        let list_struct = self.load_list(list_ptr)?;
        let input_list = list_struct;

        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "map_result")
            .map_err(llvm_err)?;

        let map_cc = self.call_rt("action_list_map_walk", &[input_list.into(), fn_ptr.into()])?;
        let result_bv = map_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_walk failed")?;
        self.builder
            .build_store(result_alloca, result_bv)
            .map_err(llvm_err)?;

        Ok(TypedValue::List(result_alloca))
    }
}
