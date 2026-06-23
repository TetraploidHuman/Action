//! Iterator builtins: map, filter, fold, find (R4-1).

use crate::call_arg::CallArg;
use crate::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_fold(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // fold(fn, init, list) or fold(init, list) { lambda }
        if let Some(lam) = trailing {
            if args.len() == 2 {
                if let Some(result) =
                    self.try_fused_filter_map_fold_fold_args(&args[0], &args[1], &lam)?
                {
                    return Ok(result);
                }
                if let Some(result) = self.try_fused_map_fold_fold_args(&args[0], &args[1], &lam)? {
                    return Ok(result);
                }
                if let Some(result) =
                    self.try_fused_filter_fold_fold_args(&args[0], &args[1], &lam)?
                {
                    return Ok(result);
                }
            }
        }
        let (fn_ptr, init_val, list_val) = if let Some(lam) = trailing {
            if args.len() != 2 {
                return Err(
                    "fold with trailing lambda expects 2 arguments (init, list)".to_string()
                );
            }
            let iv = self.compile_call_arg(args[0])?;
            let lv = self.compile_call_arg(args[1])?;
            let fv = self.compile_call_arg(lam)?;
            (fv, iv, lv)
        } else if args.len() == 3 {
            let fv = self.compile_call_arg(args[0])?;
            let iv = self.compile_call_arg(args[1])?;
            let lv = self.compile_call_arg(args[2])?;
            (fv, iv, lv)
        } else {
            return Err("fold expects 3 arguments (fn, init, list)".to_string());
        };

        if let Some(result) = self.try_builtin_fold_direct(fn_ptr, init_val, list_val)? {
            return Ok(result);
        }

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("fold: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("fold: third argument must be a list".to_string()),
        };
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Err("fold: init must be an integer".to_string()),
        };

        let list_struct = self.load_list(list_ptr)?;
        let input_list = list_struct;

        let fold_cc = self.call_rt(
            "action_list_fold_walk",
            &[input_list.into(), fn_ptr.into(), init_i64.into()],
        )?;
        let final_acc = fold_cc
            .try_as_basic_value()
            .basic()
            .ok_or("fold_walk failed")?
            .into_int_value();
        Ok(TypedValue::Int(final_acc))
    }

    /// flatMap(fn, list) = flatten(map(fn, list))
    pub(crate) fn builtin_flat_map_list(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let mapped = self.builtin_map(args, trailing)?;
        match mapped {
            TypedValue::List(lp) => {
                let lv = self.load_list(lp)?;
                let cc = self.call_rt("action_list_flatten", &[lv.into()])?;
                let result = cc.try_as_basic_value().basic().ok_or("flatten failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "flatMap")
                    .map_err(llvm_err)?;
                self.builder.build_store(alloca, result).map_err(llvm_err)?;
                Ok(TypedValue::List(alloca))
            }
            _ => Err("flatMap: map result must be a list".to_string()),
        }
    }

    pub(crate) fn builtin_callback_list(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "any" => self.builtin_any(args, trailing),
            "all" => self.builtin_all(args, trailing),
            "find" => self.builtin_find(args, trailing),
            "findIndex" => self.builtin_find_index(args, trailing),
            "reduce" => self.builtin_reduce(args, trailing),
            "foldRight" => self.builtin_fold_right(args, trailing),
            "takeWhile" => self.builtin_take_while(args, trailing),
            "dropWhile" => self.builtin_drop_while(args, trailing),
            "sortedBy" => self.builtin_sorted_by(args, trailing),
            "partition" => self.builtin_partition(args, trailing),
            "count" => self.builtin_count(args, trailing),
            _ => Err(format!("Unknown callback list builtin: {}", name)),
        }
    }
}
