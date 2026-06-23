//! Iterator builtins: map, filter, fold, find (R4-1).

use crate::call_arg::CallArg;
use crate::{CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    /// any(list, fn) or any(list) { lambda } -> Bool
    pub(crate) fn builtin_any(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_val, list_val) = if let Some(lam) = trailing {
            if args.len() != 1 {
                return Err("any with trailing lambda expects 1 argument (list)".to_string());
            }
            let lv = self.compile_call_arg(args[0])?;
            let fv = self.compile_call_arg(lam)?;
            (fv, lv)
        } else if args.len() == 2 {
            let fv = self.compile_call_arg(args[0])?;
            let lv = self.compile_call_arg(args[1])?;
            (fv, lv)
        } else {
            return Err("any expects 2 arguments (fn, list)".to_string());
        };

        if let Some(result) = self.try_builtin_any_direct(fn_val, list_val)? {
            return Ok(result);
        }

        let fn_ptr = match fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("any: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("any: last argument must be a list".to_string()),
        };
        let input_list = self.load_list(list_ptr)?;

        let any_cc = self.call_rt("action_list_any_walk", &[input_list.into(), fn_ptr.into()])?;
        let res = any_cc
            .try_as_basic_value()
            .basic()
            .ok_or("any_walk failed")?
            .into_int_value();
        Ok(TypedValue::Bool(res))
    }

    /// all(list, fn) or all(list) { lambda } -> Bool
    pub(crate) fn builtin_all(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_val, list_val) = if let Some(lam) = trailing {
            if args.len() != 1 {
                return Err("all with trailing lambda expects 1 argument (list)".to_string());
            }
            let lv = self.compile_call_arg(args[0])?;
            let fv = self.compile_call_arg(lam)?;
            (fv, lv)
        } else if args.len() == 2 {
            let fv = self.compile_call_arg(args[0])?;
            let lv = self.compile_call_arg(args[1])?;
            (fv, lv)
        } else {
            return Err("all expects 2 arguments (fn, list)".to_string());
        };

        if let Some(result) = self.try_builtin_all_direct(fn_val, list_val)? {
            return Ok(result);
        }

        let fn_ptr = match fn_val {
            TypedValue::Fn(p, _) => p,
            TypedValue::Closure { fn_ptr, .. } => fn_ptr,
            _ => return Err("all: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("all: last argument must be a list".to_string()),
        };
        let input_list = self.load_list(list_ptr)?;

        let all_cc = self.call_rt("action_list_all_walk", &[input_list.into(), fn_ptr.into()])?;
        let res = all_cc
            .try_as_basic_value()
            .basic()
            .ok_or("all_walk failed")?
            .into_int_value();
        Ok(TypedValue::Bool(res))
    }
}
