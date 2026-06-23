//! Expression codegen (R4-3).

use inkwell::types::BasicMetadataTypeEnum;
use inkwell::values::{BasicValue, BasicValueEnum};

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(crate) fn type_name_from_typed_value(&self, v: &TypedValue<'ctx>) -> String {
        match v {
            TypedValue::Int(_) => "Int".to_string(),
            TypedValue::Float(_) => "Float".to_string(),
            TypedValue::Bool(_) => "Bool".to_string(),
            TypedValue::Str(_) => "String".to_string(),
            TypedValue::Struct(_, st) => {
                for (name, ty) in &self.type_layout.named_structs {
                    if *ty == *st {
                        return name.clone();
                    }
                }
                "Struct".to_string()
            }
            TypedValue::Enum(..) => "Enum".to_string(),
            TypedValue::Unit => "Unit".to_string(),
            TypedValue::Fn(_, _) | TypedValue::Closure { .. } => "Fn".to_string(),
            TypedValue::List(_) => "list".to_string(),
            TypedValue::Map(_) => "map".to_string(),
            TypedValue::Set(_) => "set".to_string(),
            TypedValue::Task(_) => "Task".to_string(),
            TypedValue::Stream(_) => "Stream".to_string(),
            TypedValue::LazyList(_) => "LazyList".to_string(),
            TypedValue::CString(_) => "CString".to_string(),
            TypedValue::Ptr(_) => "Ptr".to_string(),
            TypedValue::FileHandle(_) => "FileHandle".to_string(),
            TypedValue::Nullable(_, _) => "Nullable".to_string(),
        }
    }

    /// Compile a HIR expression and load for use as a call argument.
    pub(crate) fn compile_and_load_hir(
        &mut self,
        expr: &action_frontend::hir::HirExpr,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let v = self.compile_hir_expr(expr)?;
        self.typed_value_to_bv_for_call(&v)
    }

    pub(crate) fn typed_value_to_bv_for_call(
        &mut self,
        v: &TypedValue<'ctx>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        match v {
            TypedValue::Enum(ptr, ty, ..) => {
                let bt: inkwell::types::BasicTypeEnum = (*ty).into();
                Ok(self
                    .builder
                    .build_load(bt, *ptr, "arg_enum")
                    .map_err(llvm_err)?)
            }
            TypedValue::Struct(ptr, ty) => {
                let bt: inkwell::types::BasicTypeEnum = (*ty).into();
                Ok(self
                    .builder
                    .build_load(bt, *ptr, "arg_struct")
                    .map_err(llvm_err)?)
            }
            TypedValue::Str(ptr) => Ok(self.load_string(*ptr)?.into()),
            TypedValue::List(ptr) => Ok(self.load_list(*ptr)?.into()),
            TypedValue::Map(ptr) => Ok(self.load_list(*ptr)?.into()),
            TypedValue::Set(ptr) => Ok(self.load_list(*ptr)?.into()),
            TypedValue::CString(p) | TypedValue::Ptr(p) | TypedValue::FileHandle(p) => {
                Ok((*p).into())
            }
            TypedValue::Nullable(ptr, ty) => Ok(self
                .builder
                .build_load(*ty, *ptr, "arg_nullable")
                .map_err(llvm_err)?),
            _ => v
                .to_bv()
                .ok_or_else(|| format!("Cannot pass value as argument")),
        }
    }

    /// Compile an expression and load the result as a BasicValueEnum for passing as a call argument.
    /// Handles loading from alloca pointers for enum, struct, and string types.

    /// Coerce an argument value to match the expected parameter type
    pub(crate) fn coerce_arg(
        &mut self,
        val: BasicValueEnum<'ctx>,
        expected_ty: Option<&BasicMetadataTypeEnum<'ctx>>,
    ) -> Result<BasicValueEnum<'ctx>, String> {
        let expected_ty = match expected_ty {
            Some(t) => t,
            None => return Ok(val),
        };
        let actual_is_ptr = matches!(val, BasicValueEnum::PointerValue(_));
        let expected_is_i64 =
            matches!(expected_ty, BasicMetadataTypeEnum::IntType(t) if t.get_bit_width() == 64);
        let expected_is_f64 = matches!(expected_ty, BasicMetadataTypeEnum::FloatType(_));
        let expected_is_ptr = matches!(expected_ty, BasicMetadataTypeEnum::PointerType(_));
        let actual_is_i64 =
            matches!(val, BasicValueEnum::IntValue(i) if i.get_type().get_bit_width() == 64);
        let actual_is_f64 = matches!(val, BasicValueEnum::FloatValue(_));

        if actual_is_ptr && expected_is_i64 {
            // ptr → i64: function pointer passed to untyped parameter
            let ptr_val = val.into_pointer_value();
            let i64_val = self
                .builder
                .build_ptr_to_int(ptr_val, self.i64_ty(), "ptr2int")
                .map_err(llvm_err)?;
            Ok(i64_val.as_basic_value_enum())
        } else if !actual_is_ptr && expected_is_ptr {
            // i64 → ptr: int passed to function pointer parameter
            let int_val = val.into_int_value();
            let ptr_val = self
                .builder
                .build_int_to_ptr(int_val, self.ptr_ty(), "int2ptr")
                .map_err(llvm_err)?;
            Ok(ptr_val.as_basic_value_enum())
        } else if actual_is_i64 && expected_is_f64 {
            // Int → Float promotion
            let int_val = val.into_int_value();
            let float_val = self
                .builder
                .build_signed_int_to_float(int_val, self.f64_ty(), "int2float")
                .map_err(llvm_err)?;
            Ok(float_val.as_basic_value_enum())
        } else if actual_is_f64 && expected_is_i64 {
            // Float → Int truncation
            let float_val = val.into_float_value();
            let int_val = self
                .builder
                .build_float_to_signed_int(float_val, self.i64_ty(), "float2int")
                .map_err(llvm_err)?;
            Ok(int_val.as_basic_value_enum())
        } else if let BasicMetadataTypeEnum::StructType(expected_struct) = expected_ty {
            // Wrap non-struct scalar into nullable struct {i1=0, T} when target is nullable.
            // Only wrap if the value is not already a struct (which would be double-wrapping).
            let field_types = expected_struct.get_field_types();
            if field_types.len() == 2 && !matches!(&val, BasicValueEnum::StructValue(_)) {
                let undef = expected_struct.get_undef();
                let null_flag = self.null_flag_ty().const_int(0, false);
                let with_flag = self
                    .builder
                    .build_insert_value(undef, null_flag, 0, "wrap_flag")
                    .map_err(llvm_err)?;
                let wrapped = self
                    .builder
                    .build_insert_value(with_flag, val, 1, "wrap_val")
                    .map_err(llvm_err)?;
                Ok(wrapped.as_basic_value_enum())
            } else {
                Ok(val)
            }
        } else {
            Ok(val)
        }
    }

    /// Compile function reference: ::function_name, ::Type.method, ::module::func
    pub(crate) fn compile_function_ref(&mut self, name: &str) -> Result<TypedValue<'ctx>, String> {
        // Resolve :: separators in path (e.g., "math::add" or "Type::method")
        let resolved = name.replace("::", "_").replace('.', "_");

        // Try the resolved name first (handles module::function -> module_function)
        if let Some(fn_val) = self.module.get_function(&resolved) {
            let fn_ptr = fn_val.as_global_value().as_pointer_value();
            let fn_type = fn_val.get_type();
            return Ok(TypedValue::Fn(fn_ptr, fn_type));
        }

        // Try the original name (handles simple ::function_name)
        if let Some(fn_val) = self.module.get_function(name) {
            let fn_ptr = fn_val.as_global_value().as_pointer_value();
            let fn_type = fn_val.get_type();
            return Ok(TypedValue::Fn(fn_ptr, fn_type));
        }

        // Handle Type.method pattern: ::Int.toString -> action_int_to_string
        if let Some((type_part, method)) = name.rsplit_once('.') {
            let type_name = type_part;
            // Map type-method to runtime function name
            // Many builtins have corresponding action_* runtime functions
            let rt_name = match (type_name, method) {
                // Int/Float/Bool -> String conversions
                ("Int", "toString") | ("Bool", "toString") => "action_int_to_string",
                ("Float", "toString") => "action_float_to_string",
                // String methods
                ("String", "len") | ("String", "length") => "action_string_len",
                ("String", "toUpper") => "action_string_to_upper",
                ("String", "toLower") => "action_string_to_lower",
                ("String", "trim") => "action_string_trim",
                ("String", "substring") => "action_string_substring",
                ("String", "startsWith") => "action_string_starts_with",
                ("String", "endsWith") => "action_string_ends_with",
                ("String", "split") => "action_string_split",
                ("String", "contains") => "action_string_contains",
                ("String", "toInt") | ("String", "toFloat") => {
                    return Err(format!(
                        "::{}::{} cannot be used as a function reference (nullable parse result)",
                        type_name, method
                    ));
                }
                ("String", "chars") => "action_string_chars",
                ("String", "join") => "action_string_join",
                ("String", "replace") => "action_string_replace",
                ("String", "repeat") => "action_string_repeat",
                ("String", "trimStart") => "action_string_trim_start",
                ("String", "trimEnd") => "action_string_trim_end",
                // List methods
                ("list", "len") | ("map", "len") | ("set", "len") => "action_list_len",
                ("list", "head") => "action_list_head",
                ("list", "last") => "action_list_last",
                ("list", "tail") => "action_list_tail",
                ("list", "init") => "action_list_init",
                ("list", "reverse") => "action_list_reverse",
                ("list", "take") => "action_list_take",
                ("list", "drop") => "action_list_drop",
                ("list", "contains") => "action_list_contains",
                ("list", "zip") => "action_list_zip",
                ("list", "get") => "action_list_get",
                ("list", "append") | ("list", "push") => "action_list_push",
                ("list", "range") => "action_list_range",
                ("list", "sorted") => "action_list_sorted",
                ("list", "unique") => "action_list_unique",
                ("list", "flatten") => "action_list_flatten",
                // Map methods
                ("map", "contains") => "action_map_contains",
                ("map", "get") => "action_map_get",
                ("map", "insert") => "action_map_insert",
                ("map", "remove") => "action_map_remove",
                // Methods without simple runtime function counterparts
                ("list", "map")
                | ("list", "filter")
                | ("list", "fold")
                | ("list", "flatMap")
                | ("list", "withIndex")
                | ("list", "sum")
                | ("list", "product")
                | ("list", "prepend")
                | ("list", "isEmpty")
                | ("list", "any")
                | ("list", "all")
                | ("list", "find")
                | ("list", "reduce")
                | ("list", "splitLines")
                | ("LazyList", _)
                | ("Task", _)
                | ("Stream", _)
                | ("Ptr", _)
                | ("CString", _) => {
                    // These either take function arguments or operate on complex types —
                    // register as wrapper-needed and create a placeholder
                    self.builtin_wrappers_needed.insert(method.to_string());
                    return Err(format!(
                        "::{}::{} requires runtime support not yet available as function reference",
                        type_name, method
                    ));
                }
                _ => {
                    // Try Type_method mangling for extension methods
                    let mangled = format!("{}_{}", type_name, method);
                    if let Some(fn_val) = self.module.get_function(&mangled) {
                        let fn_ptr = fn_val.as_global_value().as_pointer_value();
                        let fn_type = fn_val.get_type();
                        return Ok(TypedValue::Fn(fn_ptr, fn_type));
                    }
                    let alt_mangled = format!("{}_{}", type_part.replace("::", "_"), method);
                    if mangled != alt_mangled {
                        if let Some(fn_val) = self.module.get_function(&alt_mangled) {
                            let fn_ptr = fn_val.as_global_value().as_pointer_value();
                            let fn_type = fn_val.get_type();
                            return Ok(TypedValue::Fn(fn_ptr, fn_type));
                        }
                    }
                    return Err(format!(
                        "Function reference '::{}' could not be resolved",
                        name
                    ));
                }
            };
            // Look up the runtime function
            if let Some(fn_val) = self.module.get_function(rt_name) {
                let fn_ptr = fn_val.as_global_value().as_pointer_value();
                let fn_type = fn_val.get_type();
                return Ok(TypedValue::Fn(fn_ptr, fn_type));
            }
            Err(format!(
                "Runtime function '{}' not found for ::{}",
                rt_name, name
            ))
        } else {
            Err(format!("Undefined function reference: ::{}", name))
        }
    }
}
