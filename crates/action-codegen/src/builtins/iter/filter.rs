//! Iterator builtins: map, filter, fold, find (R4-1).

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_filter(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // Fused map+filter optimization: if the list argument is `map(...)`,
        // fuse map and filter into a single tree walk instead of creating an
        // intermediate list.
        let list_arg = if trailing.is_some() {
            if args.len() != 1 {
                return Err("filter with trailing lambda expects 1 argument (list)".to_string());
            }
            args[0]
        } else if args.len() == 2 {
            args[1]
        } else {
            return Err("filter expects 2 arguments (fn, list)".to_string());
        };

        let CallArg::Hir(list_hir) = list_arg;
        if let Some((map_fn, inner)) = Self::extract_map_call_args_hir(list_hir) {
            let filter_fn_val = if let Some(lam) = trailing {
                self.compile_call_arg(lam)?
            } else {
                self.compile_call_arg(args[0])?
            };
            return self.fused_map_filter_hir(map_fn, inner, filter_fn_val);
        }
        if let Some((flat_fn, inner)) = Self::extract_flatmap_call_args_hir(list_hir) {
            let filter_fn_val = if let Some(lam) = trailing {
                self.compile_call_arg(lam)?
            } else {
                self.compile_call_arg(args[0])?
            };
            return self.fused_flatmap_filter_hir(flat_fn, inner, filter_fn_val);
        }

        // Standard filter path
        let (fn_ptr, list_val) = if let Some(lam) = trailing {
            let lv = self.compile_call_arg(args[0])?;
            let fv = self.compile_call_arg(lam)?;
            (fv, lv)
        } else {
            let fv = self.compile_call_arg(args[0])?;
            let lv = self.compile_call_arg(args[1])?;
            (fv, lv)
        };

        if let Some(result) = self.try_builtin_filter_direct(fn_ptr.clone(), list_val.clone())? {
            return Ok(result);
        }

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("filter: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("filter: second argument must be a list".to_string()),
        };

        let list_struct = self.load_list(list_ptr)?;
        let input_list = list_struct;

        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "filter_result")
            .map_err(llvm_err)?;

        let filter_cc = self.call_rt(
            "action_list_filter_walk",
            &[input_list.into(), fn_ptr.into()],
        )?;
        let result_bv = filter_cc
            .try_as_basic_value()
            .basic()
            .ok_or("filter_walk failed")?;
        self.builder
            .build_store(result_alloca, result_bv)
            .map_err(llvm_err)?;

        Ok(TypedValue::List(result_alloca))
    }
}
