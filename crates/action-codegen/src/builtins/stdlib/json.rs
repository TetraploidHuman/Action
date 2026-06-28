//! Compiler JSON builtins (`__jsonParse`, `__jsonGet`, `__jsonGetIdx`).

use inkwell::values::PointerValue;

use crate::call_arg::CallArg;
use crate::{CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn builtin_stdlib_json(
        &mut self,
        name: &str,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        if self.in_fallible_region() {
            return match name {
                "__jsonParse" if args.len() == 1 => self.compile_json_parse_fallible(args[0]),
                "__jsonGet" if args.len() == 2 => self.compile_json_get_fallible(args[0], args[1]),
                "__jsonGetIdx" if args.len() == 2 => {
                    self.compile_json_get_idx_fallible(args[0], args[1])
                }
                _ => Err(format!("Unknown JSON builtin: {}", name)),
            };
        }
        match name {
            "__jsonParse" => {
                if args.len() != 1 {
                    return Err("__jsonParse expects 1 argument".to_string());
                }
                let cstr = self.json_cstring_arg(args[0])?;
                self.json_call_ptr("action_json_parse", &[cstr.into()])
            }
            "__jsonGet" => {
                if args.len() != 2 {
                    return Err("__jsonGet expects 2 arguments".to_string());
                }
                let node = self.json_ptr_arg(args[0])?;
                let key = self.json_cstring_arg(args[1])?;
                self.json_call_ptr("action_json_get", &[node.into(), key.into()])
            }
            "__jsonGetIdx" => {
                if args.len() != 2 {
                    return Err("__jsonGetIdx expects 2 arguments".to_string());
                }
                let node = self.json_ptr_arg(args[0])?;
                let idx = self.compile_call_arg(args[1])?;
                let TypedValue::Int(idx_iv) = idx else {
                    return Err("__jsonGetIdx: index must be Int".to_string());
                };
                self.json_call_ptr("action_json_get_idx", &[node.into(), idx_iv.into()])
            }
            _ => Err(format!("Unknown JSON builtin: {}", name)),
        }
    }

    pub(crate) fn json_cstring_arg(
        &mut self,
        arg: CallArg<'_>,
    ) -> Result<PointerValue<'ctx>, String> {
        let v = self.compile_call_arg(arg)?;
        match v {
            TypedValue::CString(p) => Ok(p),
            other => match self.builtin_to_cstring_value(other)? {
                TypedValue::CString(p) => Ok(p),
                _ => Err("expected CString".to_string()),
            },
        }
    }

    pub(crate) fn json_ptr_arg(&mut self, arg: CallArg<'_>) -> Result<PointerValue<'ctx>, String> {
        let v = self.compile_call_arg(arg)?;
        match v {
            TypedValue::Ptr(p) => Ok(p),
            _ => Err("expected Ptr".to_string()),
        }
    }

    pub(crate) fn json_call_ptr(
        &mut self,
        rt: &str,
        args: &[inkwell::values::BasicMetadataValueEnum<'ctx>],
    ) -> Result<TypedValue<'ctx>, String> {
        let cc = self.call_rt(rt, args)?;
        let ptr = cc
            .try_as_basic_value()
            .basic()
            .ok_or_else(|| format!("{rt} failed"))?
            .into_pointer_value();
        Ok(TypedValue::Ptr(ptr))
    }
}
