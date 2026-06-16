// Submodule: builtins_call

use crate::ast::*;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::{BasicMetadataValueEnum, BasicValueEnum};
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_call(
        &mut self,
        func: &Expr,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // Handle named function calls (including builtins)
        if let Expr::Ident(name) = func {
            // If this name is a function variable in scope, dispatch via indirect call
            // (takes precedence over builtins to allow passing builtins as function references)
            if let Some(scope_var) = self.scope.get(name) {
                if scope_var.kind == super::ValKind::Fn {
                    // Fall through to higher-order call path below
                    let target = self.compile_expr(func)?;
                    return self.compile_indirect_call(target, args, trailing);
                }
            }
            if name == "print" || name == "println" {
                return self.builtin_print(name, args);
            }
            if name == "__list" {
                return self.builtin_list(args);
            }
            if name == "lazy_list" {
                return self.builtin_lazy_list(args, trailing);
            }
            if name == "launch" {
                return self.builtin_launch(args, trailing);
            }
            if name == "coroutineScope" {
                return self.builtin_coroutine_scope(args, trailing);
            }
            if name == "delay" {
                return self.builtin_delay(args);
            }
            if name == "withTimeout" {
                return self.builtin_with_timeout(args, trailing);
            }
            // Stream<T> operations
            if name == "Stream" {
                return self.builtin_stream_create();
            }
            if name == "send" || name == "receive" || name == "close" {
                return self.builtin_stream_op(name, args);
            }
            // Task<T> operations
            if name == "cancel" || name == "is_done" || name == "is_cancelled" || name == "wait" {
                return self.builtin_task_op(name, args);
            }
            if name == "len"
                || name == "isEmpty"
                || name == "append"
                || name == "concat"
                || name == "toUpper"
                || name == "toLower"
                || name == "trim"
                || name == "readLine"
                || name == "startsWith"
                || name == "endsWith"
                || name == "substring"
                || name == "parseInt"
                || name == "readFile"
                || name == "writeFile"
                || name == "appendFile"
                || name == "exists"
                || name == "deleteFile"
                || name == "openFile"
                || name == "closeFile"
                || name == "isEof"
                || name == "fileReadLine"
                || name == "fileReadBytes"
                || name == "fileWrite"
                || name == "fileWriteLine"
                || name == "fileFlush"
                || name == "fileSeek"
                || name == "fileTell"
                || name == "randInt"
                || name == "randFloat"
                || name == "split"
                || name == "join"
                || name == "replace"
                || name == "abs"
                || name == "min"
                || name == "max"
                || name == "sqrt"
                || name == "cbrt"
                || name == "sin"
                || name == "cos"
                || name == "tan"
                || name == "asin"
                || name == "acos"
                || name == "atan"
                || name == "atan2"
                || name == "log"
                || name == "log2"
                || name == "log10"
                || name == "exp"
                || name == "floor"
                || name == "ceil"
                || name == "round"
                || name == "pi"
                || name == "e"
                || name == "clamp"
                || name == "isNaN"
                || name == "isInfinite"
                || name == "panic"
                || name == "assert"
                || name == "toString"
                || name == "head"
                || name == "last"
                || name == "get"
                || name == "reverse"
                || name == "contains"
                || name == "containsKey"
                || name == "prepend"
                || name == "take"
                || name == "drop"
                || name == "range"
                || name == "repeat"
                || name == "trimStart"
                || name == "trimEnd"
                || name == "stringContains"
                || name == "stringRepeat"
                || name == "now"
                || name == "today"
                || name == "tail"
                || name == "zip"
                || name == "splitLines"
                || name == "indexOf"
                || name == "year"
                || name == "month"
                || name == "day"
                || name == "hour"
                || name == "minute"
                || name == "second"
                || name == "addDays"
                || name == "addHours"
                || name == "randChoice"
                || name == "randShuffle"
                || name == "toChar"
                || name == "charCode"
                || name == "toInt"
                || name == "toFloat"
                || name == "init"
                || name == "insert"
                || name == "remove"
                || name == "chars"
                || name == "setToList"
                || name == "setFromList"
                || name == "fromList"
                || name == "withIndex"
                || name == "unique"
                || name == "slice"
                || name == "flatten"
                || name == "splitAt"
                || name == "chunks"
                || name == "windows"
                || name == "pow"
                || name == "mapKeys"
                || name == "mapValues"
                || name == "mapEntries"
                || name == "mapUnion"
                || name == "setUnion"
                || name == "setIntersection"
                || name == "setDifference"
                || name == "setIsSubset"
                || name == "setInsert"
                || name == "setRemove"
                || name == "sorted"
                || name == "readDir"
                || name == "identity"
                || name == "compose"
                || name == "diffDays"
                || name == "weekday"
                || name == "sum"
                || name == "product"
                || name == "digits"
                || name == "charAt"
                || name == "isAlpha"
                || name == "codeToChar"
                || name == "nowUtc"
                || name == "diffSeconds"
                || name == "flip"
                || name == "constant"
                || name == "uncurry"
                || name == "curry"
                || name == "toLazyList"
                || name == "lazyTake"
                || name == "lazyDrop"
                || name == "lazyMap"
                || name == "lazyFilter"
                || name == "lazyTakeWhile"
                || name == "lazyHead"
                || name == "lazyZip"
                || name == "toList"
                || name == "format"
                || name == "parseDate"
                || name == "date"
                || name == "datetime"
                || name == "Random_new"
                || name == "nextInt"
                || name == "toCString"
                || name == "fromCString"
                || name == "isNull"
                || name == "deref"
                || name == "to"
                || name == "httpRequest"
                || name == "ping"
            {
                // Handle trailing lambda for lazyMap/filter/takeWhile:
                // lazyMap(ll) { fn } → args becomes [fn, ll]
                if trailing.is_some()
                    && (name == "lazyMap" || name == "lazyFilter" || name == "lazyTakeWhile")
                {
                    let mut new_args = vec![*trailing.clone().unwrap()];
                    new_args.extend_from_slice(args);
                    return self.builtin_stdlib(name, &new_args);
                }
                return self.builtin_stdlib(name, args);
            }
            // Handle enum variant constructors: Some(42), Ok(val), Err(e), etc.
            if let Some((enum_info, variant)) = self
                .registry
                .lookup_variant(name)
                .map(|(ei, vi)| (ei.clone(), vi.clone()))
            {
                if !variant.params.is_empty() {
                    return self.compile_enum_construct(&enum_info, &variant, args);
                }
                // Unit variant without args: simply construct
                if args.is_empty() {
                    return self.compile_enum_construct(&enum_info, &variant, &[]);
                }
                return Err(format!(
                    "Variant '{}' takes no arguments but {} were given",
                    name,
                    args.len()
                ));
            }
            // flatMap/flatMapResult no longer handle Option/Result enums
            // (nullable types replace Option/Result)
            if name == "map" || name == "filter" || name == "fold" {
                let list_arg_idx: Option<usize> = if name == "map" || name == "filter" {
                    if trailing.is_some() {
                        Some(0)
                    } else if args.len() >= 2 {
                        Some(1)
                    } else {
                        None
                    }
                } else if name == "fold" {
                    if trailing.is_some() && args.len() >= 2 {
                        Some(1)
                    } else if args.len() >= 3 {
                        Some(1)
                    } else {
                        None
                    }
                } else {
                    None
                };
                let is_list_op = list_arg_idx.map_or(false, |idx| {
                    idx < args.len()
                        && matches!(self.compile_expr(&args[idx]), Ok(TypedValue::List(_)))
                });
                if is_list_op {
                    if name == "map" {
                        return self.builtin_map(args, trailing);
                    } else if name == "filter" {
                        return self.builtin_filter(args, trailing);
                    } else if name == "fold" {
                        return self.builtin_fold(args, trailing);
                    }
                }
                // enum map (Option/Result) has been removed — nullable types replace them
            }
            // flatMap for lists: flatMap(fn, list) or flatMap(list) { lambda }
            if name == "flatMap" {
                let list_arg_idx: Option<usize> = if trailing.is_some() {
                    Some(0)
                } else if args.len() >= 2 {
                    Some(1)
                } else {
                    None
                };
                let is_list_op = list_arg_idx.map_or(false, |idx| {
                    idx < args.len()
                        && matches!(self.compile_expr(&args[idx]), Ok(TypedValue::List(_)))
                });
                if is_list_op {
                    return self.builtin_flat_map_list(args, trailing);
                }
            }
            // Callback-based list functions
            if name == "any"
                || name == "all"
                || name == "find"
                || name == "findIndex"
                || name == "reduce"
                || name == "foldRight"
                || name == "takeWhile"
                || name == "dropWhile"
                || name == "sortedBy"
                || name == "partition"
                || name == "count"
            {
                let list_arg_idx: Option<usize> = if name == "foldRight" {
                    if trailing.is_some() && args.len() >= 2 {
                        Some(0)
                    } else if args.len() >= 3 {
                        Some(1)
                    } else {
                        None
                    }
                } else {
                    if trailing.is_some() {
                        Some(0)
                    } else if args.len() >= 2 {
                        Some(1)
                    } else {
                        None
                    }
                };
                let is_list_op = list_arg_idx.map_or(false, |idx| {
                    idx < args.len()
                        && matches!(self.compile_expr(&args[idx]), Ok(TypedValue::List(_)))
                });
                if is_list_op {
                    return self.builtin_callback_list(name, args, trailing);
                }
            }
            // Callback-based map functions
            if name == "mapFilter" || name == "mapMapValues" || name == "mapFold" {
                // Find which argument is a Map
                let map_idx = (0..args.len()).find(|&i| {
                    self.compile_expr(&args[i])
                        .map_or(false, |v| matches!(v, TypedValue::Map(_)))
                });
                if map_idx.is_some() {
                    return self.builtin_callback_map(name, args, trailing);
                }
            }

            // Check if it's an enum variant constructor: Some(42), None, etc.
            let variant_info = self
                .registry
                .lookup_variant(name)
                .map(|(ei, vi)| (ei.clone(), vi.clone()));
            if let Some((enum_info, variant)) = variant_info {
                return self.compile_enum_construct(&enum_info, &variant, args);
            }

            // Try overloaded dispatch first if the name has overloads
            if let Some(overloads) = self.overloaded_functions.get(name).cloned() {
                // Compile args to determine their runtime types
                let arg_vals: Vec<TypedValue<'ctx>> = args
                    .iter()
                    .map(|a| self.compile_expr(a))
                    .collect::<Result<_, _>>()?;

                // Map TypedValue to type name for mangling
                let arg_type_names: Vec<String> = arg_vals
                    .iter()
                    .map(|v| self.typed_value_type_name(v))
                    .collect();
                let mangled = if arg_type_names.is_empty() {
                    name.clone()
                } else {
                    format!("{}_{}", name, arg_type_names.join("_"))
                };

                // Find matching overload
                let fn_name = overloads
                    .iter()
                    .find(|(_, mn)| *mn == mangled)
                    .map(|(_, mn)| mn)
                    .or_else(|| {
                        // Exact match not found; try fallback: if all args are Int,
                        // it might be an untyped call — use the first overload
                        overloads.first().map(|(_, mn)| mn)
                    })
                    .ok_or_else(|| {
                        format!(
                            "No matching overload of '{}' for argument types: {:?}",
                            name, arg_type_names
                        )
                    })?;

                let fn_val = self
                    .module
                    .get_function(fn_name)
                    .ok_or_else(|| format!("Overloaded function '{}' not found", fn_name))?;
                let fn_type = fn_val.get_type();
                let param_tys = fn_type.get_param_types();
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                for (i, av) in arg_vals.iter().enumerate() {
                    let bv = av.to_bv().unwrap_or_else(|| {
                        // For complex types, we need to load from alloca
                        match av {
                            TypedValue::Str(ptr) => {
                                let ld = self
                                    .builder
                                    .build_load(self.string_type, *ptr, "arg_str")
                                    .unwrap();
                                ld.into()
                            }
                            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                                let ld = self
                                    .builder
                                    .build_load(self.list_type, *ptr, "arg_list")
                                    .unwrap();
                                ld.into()
                            }
                            TypedValue::LazyList(ptr) => {
                                let ld = self
                                    .builder
                                    .build_load(self.lazylist_type, *ptr, "arg_ll")
                                    .unwrap();
                                ld.into()
                            }
                            TypedValue::Task(ptr) => {
                                let ld = self
                                    .builder
                                    .build_load(self.task_type, *ptr, "arg_task")
                                    .unwrap();
                                ld.into()
                            }
                            TypedValue::Stream(ptr) => {
                                // Stream is a heap pointer; extract list from field 1 for arg passing
                                let lf = self
                                    .builder
                                    .build_struct_gep(self.stream_type, *ptr, 3, "arg_slf")
                                    .unwrap();
                                let ld = self
                                    .builder
                                    .build_load(self.list_type, lf, "arg_sl")
                                    .unwrap();
                                ld.into()
                            }
                            TypedValue::Struct(ptr, st) => {
                                let ld = self.builder.build_load(*st, *ptr, "arg_struct").unwrap();
                                ld.into()
                            }
                            TypedValue::Enum(ptr, et, ..) => {
                                let ld = self.builder.build_load(*et, *ptr, "arg_enum").unwrap();
                                ld.into()
                            }
                            TypedValue::CString(p)
                            | TypedValue::Ptr(p)
                            | TypedValue::FileHandle(p) => (*p).into(),
                            _ => {
                                // Fallback: use zero int
                                self.i64_ty().const_int(0, false).into()
                            }
                        }
                    });
                    let casted = self.coerce_arg(bv, param_tys.get(i))?;
                    ca.push(casted.into());
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    let casted = self.coerce_arg(bv, param_tys.get(args.len()))?;
                    ca.push(casted.into());
                }

                let cc = self.builder.build_call(fn_val, &ca, "").map_err(llvm_err)?;
                // Free intermediate arguments (not scope variables) after the call.
                for av in &arg_vals {
                    self.rc_free_intermediate(av)?;
                }
                let ast_ret = self.fun_return_types.get(fn_name).cloned();
                return match cc.try_as_basic_value().basic() {
                    Some(bv) => {
                        self.unpack_call_return(bv, fn_type.get_return_type(), ast_ret.as_ref())
                    }
                    None => Ok(TypedValue::Unit),
                };
            }

            // Generic function dispatch (monomorphization)
            if let Some(generic_stmt) = self.generic_fun_defs.get(name).cloned() {
                if let Stmt::Fun {
                    params: _,
                    type_params,
                    ..
                } = &generic_stmt
                {
                    if !type_params.is_empty() {
                        return self.compile_generic_call(
                            &generic_stmt,
                            name,
                            args,
                            trailing.clone(),
                        );
                    }
                }
            }

            // Try direct call if function exists in module
            if self.module.get_function(name).is_some() {
                let fn_val = self.module.get_function(name).unwrap();
                let fn_type = fn_val.get_type();
                let param_tys = fn_type.get_param_types();
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut direct_arg_vals: Vec<TypedValue<'ctx>> = Vec::new();
                for (i, a) in args.iter().enumerate() {
                    let av = self.compile_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    let casted = self.coerce_arg(bv, param_tys.get(i))?;
                    ca.push(casted.into());
                    direct_arg_vals.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    let casted = self.coerce_arg(bv, param_tys.get(args.len()))?;
                    ca.push(casted.into());
                }

                let cc = self.builder.build_call(fn_val, &ca, "").map_err(llvm_err)?;
                for av in &direct_arg_vals {
                    self.rc_free_intermediate(av)?;
                }
                let ast_ret = self.fun_return_types.get(name).cloned();
                return match cc.try_as_basic_value().basic() {
                    Some(bv) => {
                        self.unpack_call_return(bv, fn_type.get_return_type(), ast_ret.as_ref())
                    }
                    None => Ok(TypedValue::Unit),
                };
            }
            // Not a module function - fall through to higher-order path (it might be a variable holding a lambda)
        }

        // Module-qualified call: module.function(args) → module_function(args)
        if let Expr::FieldAccess(module_expr, method) = func {
            if let Expr::Ident(module_name) = module_expr.as_ref() {
                // List.of(...) → List[...] (equivalent to list literal)
                if module_name == "list" && method == "of" {
                    return self.builtin_list(args);
                }
                // Set.of(...) → Set literal
                if module_name == "set" && method == "of" {
                    return self.builtin_set_of(args);
                }
                let mangled = format!("{}_{}", module_name, method);
                // Check if mangled name is a builtin
                if mangled == "Random_new" || mangled == "Random_next_int" {
                    let new_func = Expr::Ident(mangled);
                    return self.compile_call(&new_func, args, trailing);
                }
                if self.module.get_function(&mangled).is_some() {
                    let fn_val = self.module.get_function(&mangled).unwrap();
                    let fn_type = fn_val.get_type();
                    let param_tys = fn_type.get_param_types();
                    let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                    let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                    for (i, a) in args.iter().enumerate() {
                        let av = self.compile_expr(a)?;
                        let bv = self.typed_value_to_bv(&av);
                        let casted = self.coerce_arg(bv, param_tys.get(i))?;
                        ca.push(casted.into());
                        tracked_args.push(av);
                    }
                    if let Some(lam) = trailing {
                        let bv = self.compile_and_load(lam)?;
                        let casted = self.coerce_arg(bv, param_tys.get(args.len()))?;
                        ca.push(casted.into());
                    }
                    let cc = self.builder.build_call(fn_val, &ca, "").map_err(llvm_err)?;
                    for av in &tracked_args {
                        self.rc_free_intermediate(av)?;
                    }
                    let ast_ret = self.fun_return_types.get(&mangled).cloned();
                    return match cc.try_as_basic_value().basic() {
                        Some(bv) => {
                            self.unpack_call_return(bv, fn_type.get_return_type(), ast_ret.as_ref())
                        }
                        None => Ok(TypedValue::Unit),
                    };
                }
            }
        }

        // UFCS method call: receiver.method(args) → TypeName_method(receiver, args)
        if let Expr::FieldAccess(receiver, method) = func {
            let recv_val = self.compile_expr(receiver)?;

            // Auto short-circuit: nullable receiver — branch on null,
            // extract inner, and dispatch method on the non-null inner value.
            if let TypedValue::Nullable(nullable_ptr, inner_bt) = recv_val {
                return self.compile_nullable_method_call(
                    nullable_ptr,
                    inner_bt,
                    receiver,
                    method,
                    args,
                    trailing,
                );
            }

            let type_name = self.type_name_from_typed_value(&recv_val);

            // Handle Map builtin methods inline
            if matches!(recv_val, TypedValue::Map(_)) {
                let map_ptr = match &recv_val {
                    TypedValue::Map(p) => *p,
                    _ => unreachable!(),
                };
                if method == "insert" {
                    return self.builtin_map_insert(map_ptr, args);
                }
                if method == "remove" {
                    return self.builtin_map_remove(map_ptr, args);
                }
                if method == "contains" {
                    return self.builtin_map_contains(map_ptr, args);
                }
                if method == "len" || method == "isEmpty" {
                    let map_loaded = self.load_list(map_ptr)?;
                    let len = self.map_len_val(map_loaded)?;
                    if method == "isEmpty" {
                        let zero = self.i64_ty().const_int(0, false);
                        let is_empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        self.rc_free_intermediate(&recv_val)?;
                        return Ok(TypedValue::Bool(is_empty));
                    }
                    self.rc_free_intermediate(&recv_val)?;
                    return Ok(TypedValue::Int(len));
                }
                if method == "keys" {
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("mapKeys".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
                if method == "values" {
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("mapValues".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
                if method == "mapValues" {
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("mapMapValues".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], trailing);
                }
                if method == "entries" {
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("mapEntries".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
                if method == "union" {
                    if args.len() != 1 {
                        return Err("map.union expects 1 argument (other map)".to_string());
                    }
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("mapUnion".to_string());
                    return self.compile_call(
                        &new_func,
                        &[receiver.as_ref().clone(), args[0].clone()],
                        &None,
                    );
                }
                if method == "filter" {
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("mapFilter".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], trailing);
                }
                if method == "fold" {
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("mapFold".to_string());
                    let mut new_args = vec![receiver.as_ref().clone()];
                    new_args.extend(args.iter().cloned());
                    return self.compile_call(&new_func, &new_args, trailing);
                }
            }
            // Handle Set builtin methods inline
            if matches!(recv_val, TypedValue::Set(_)) {
                let set_ptr = match &recv_val {
                    TypedValue::Set(p) => *p,
                    _ => unreachable!(),
                };
                if method == "insert" {
                    return self.builtin_set_insert(set_ptr, args);
                }
                if method == "remove" {
                    return self.builtin_set_remove(set_ptr, args);
                }
                if method == "contains" {
                    return self.builtin_set_contains(set_ptr, args);
                }
                if method == "len" || method == "isEmpty" {
                    let set_loaded = self.load_list(set_ptr)?;
                    let len = self.map_len_val(set_loaded)?;
                    if method == "isEmpty" {
                        let zero = self.i64_ty().const_int(0, false);
                        let is_empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        self.rc_free_intermediate(&recv_val)?;
                        return Ok(TypedValue::Bool(is_empty));
                    }
                    self.rc_free_intermediate(&recv_val)?;
                    return Ok(TypedValue::Int(len));
                }
                if method == "union" {
                    if args.len() != 1 {
                        return Err("set.union expects 1 argument (other set)".to_string());
                    }
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("setUnion".to_string());
                    return self.compile_call(
                        &new_func,
                        &[receiver.as_ref().clone(), args[0].clone()],
                        &None,
                    );
                }
                if method == "intersection" {
                    if args.len() != 1 {
                        return Err("set.intersection expects 1 argument (other set)".to_string());
                    }
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("setIntersection".to_string());
                    return self.compile_call(
                        &new_func,
                        &[receiver.as_ref().clone(), args[0].clone()],
                        &None,
                    );
                }
                if method == "difference" {
                    if args.len() != 1 {
                        return Err("set.difference expects 1 argument (other set)".to_string());
                    }
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("setDifference".to_string());
                    return self.compile_call(
                        &new_func,
                        &[receiver.as_ref().clone(), args[0].clone()],
                        &None,
                    );
                }
                if method == "is_subset" {
                    if args.len() != 1 {
                        return Err("set.isSubset expects 1 argument (other set)".to_string());
                    }
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("setIsSubset".to_string());
                    return self.compile_call(
                        &new_func,
                        &[receiver.as_ref().clone(), args[0].clone()],
                        &None,
                    );
                }
                if method == "toList" {
                    self.rc_free_method_receiver(&recv_val)?;
                    let new_func = Expr::Ident("toList".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
            }
            // Handle Range builtin methods inline (range is a Struct with 3 i64 fields)
            if let TypedValue::Struct(_, st) = &recv_val {
                if *st == self.range_type {
                    self.rc_free_method_receiver(&recv_val)?;
                    match method.as_str() {
                        "contains" => {
                            if args.len() != 1 {
                                return Err("range.contains expects 1 argument".to_string());
                            }
                            return self.builtin_range_contains(receiver, &args[0]);
                        }
                        "toList" => {
                            if !args.is_empty() {
                                return Err("range.toList expects no arguments".to_string());
                            }
                            return self.builtin_range_to_list(receiver);
                        }
                        _ => return Err(format!("Method '{}' not found on Range", method)),
                    }
                }
            }
            // Option/Result enum builtins have been removed — nullable types replace them
            // Enum dispatch for user-defined enums only
            // Handle LazyList builtin methods inline
            if matches!(recv_val, TypedValue::LazyList(_)) {
                self.rc_free_method_receiver(&recv_val)?;
                match method.as_str() {
                    "toList" => {
                        let new_func = Expr::Ident("toList".to_string());
                        return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                    }
                    "toLazyList" => {
                        let new_func = Expr::Ident("toLazyList".to_string());
                        return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                    }
                    "take" => {
                        if args.len() != 1 {
                            return Err("lazy.take expects 1 argument (n)".to_string());
                        }
                        let new_func = Expr::Ident("lazyTake".to_string());
                        return self.compile_call(
                            &new_func,
                            &[args[0].clone(), receiver.as_ref().clone()],
                            &None,
                        );
                    }
                    "drop" => {
                        if args.len() != 1 {
                            return Err("lazy.drop expects 1 argument (n)".to_string());
                        }
                        let new_func = Expr::Ident("lazyDrop".to_string());
                        return self.compile_call(
                            &new_func,
                            &[args[0].clone(), receiver.as_ref().clone()],
                            &None,
                        );
                    }
                    "map" => {
                        let new_func = Expr::Ident("lazyMap".to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone()],
                            trailing,
                        );
                    }
                    "filter" => {
                        let new_func = Expr::Ident("lazyFilter".to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone()],
                            trailing,
                        );
                    }
                    "takeWhile" => {
                        let new_func = Expr::Ident("lazyTakeWhile".to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone()],
                            trailing,
                        );
                    }
                    "head" => {
                        let new_func = Expr::Ident("lazyHead".to_string());
                        return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                    }
                    "zip" => {
                        if args.len() != 1 {
                            return Err("lazy.zip expects 1 argument (other)".to_string());
                        }
                        let new_func = Expr::Ident("lazyZip".to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone(), args[0].clone()],
                            &None,
                        );
                    }
                    _ => return Err(format!("Method '{}' not found on LazyList", method)),
                }
            }
            // Handle String builtin methods inline
            if matches!(recv_val, TypedValue::Str(_)) {
                // All paths recompile via compile_call; free the first compilation's
                // intermediate data. Scope variables: no-op.
                self.rc_free_method_receiver(&recv_val)?;
                match method.as_str() {
                    // No-arg methods
                    "len" | "isEmpty" | "toUpper" | "toLower" | "trim" | "trimStart"
                    | "trimEnd" | "chars" | "splitLines" | "toInt" | "toFloat" => {
                        let new_func = Expr::Ident(method.to_string());
                        return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                    }
                    // Single-arg methods (method(string, arg))
                    "split" | "startsWith" | "endsWith" | "indexOf" | "replace" | "slice"
                    | "repeat" | "contains" => {
                        if args.len() != 1 {
                            return Err(format!("string.{} expects 1 argument", method));
                        }
                        let mapped = match method.as_str() {
                            "contains" => "stringContains",
                            "repeat" => "stringRepeat",
                            "slice" => "slice",
                            other => other,
                        };
                        let new_func = Expr::Ident(mapped.to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone(), args[0].clone()],
                            &None,
                        );
                    }
                    // substring(string, start, len)
                    "substring" => {
                        if args.len() != 2 {
                            return Err(
                                "string.substring expects 2 arguments (start, length)".to_string()
                            );
                        }
                        let new_func = Expr::Ident("substring".to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone(), args[0].clone(), args[1].clone()],
                            &None,
                        );
                    }
                    "join" => {
                        // string.join(list) = join(string, list)
                        if args.len() != 1 {
                            return Err("string.join expects 1 argument (list)".to_string());
                        }
                        let new_func = Expr::Ident("join".to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone(), args[0].clone()],
                            &None,
                        );
                    }
                    "toCString" => {
                        let new_func = Expr::Ident("toCString".to_string());
                        return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                    }
                    _ => return Err(format!("Method '{}' not found on String", method)),
                }
            }
            // Handle Ptr/CString builtin methods inline
            if matches!(
                recv_val,
                TypedValue::Ptr(_) | TypedValue::CString(_) | TypedValue::FileHandle(_)
            ) {
                self.rc_free_method_receiver(&recv_val)?;
                match method.as_str() {
                    "isNull" => {
                        let new_func = Expr::Ident("isNull".to_string());
                        return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                    }
                    "deref" => {
                        let new_func = Expr::Ident("deref".to_string());
                        return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                    }
                    _ => return Err(format!("Method '{}' not found on Ptr/CString", method)),
                }
            }
            // Handle Stream builtin methods inline
            if matches!(recv_val, TypedValue::Stream(_)) {
                match method.as_str() {
                    "send" => {
                        if args.len() != 1 {
                            return Err("stream.send expects 1 argument: value".to_string());
                        }
                        let stream_ptr = match recv_val {
                            TypedValue::Stream(p) => p,
                            _ => unreachable!(),
                        };
                        let value = self.compile_expr(&args[0])?;
                        // Lock mutex (field 0)
                        let mutex_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 0, "sm")
                            .map_err(llvm_err)?;
                        let lock_fn = self
                            .module
                            .get_function("action_mutex_lock")
                            .ok_or("action_mutex_lock not found")?;
                        let _ = self
                            .builder
                            .build_call(lock_fn, &[mutex_ptr.into()], "")
                            .map_err(llvm_err)?;
                        // Push to list (field 3)
                        let list_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 3, "sl")
                            .map_err(llvm_err)?;
                        self.push_to_collector(list_ptr, &value)?;
                        // Signal condvar to wake up waiting receivers
                        let cond_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 1, "sc")
                            .map_err(llvm_err)?;
                        let cond_sig_fn = self
                            .module
                            .get_function("action_cond_signal")
                            .ok_or("action_cond_signal not found")?;
                        let _ = self
                            .builder
                            .build_call(cond_sig_fn, &[cond_ptr.into()], "")
                            .map_err(llvm_err)?;
                        // Unlock mutex
                        let unlock_fn = self
                            .module
                            .get_function("action_mutex_unlock")
                            .ok_or("action_mutex_unlock not found")?;
                        let _ = self
                            .builder
                            .build_call(unlock_fn, &[mutex_ptr.into()], "")
                            .map_err(llvm_err)?;
                        return Ok(TypedValue::Unit);
                    }
                    "receive" => {
                        let stream_ptr = match recv_val {
                            TypedValue::Stream(p) => p,
                            _ => unreachable!(),
                        };
                        let zero = self.i64_ty().const_int(0, false);
                        let one = self.i64_ty().const_int(1, false);
                        let cur_fn = self
                            .builder
                            .get_insert_block()
                            .ok_or("no insert block")?
                            .get_parent()
                            .ok_or("no current fn")?;
                        let result_alloca = self
                            .builder
                            .build_alloca(self.i64_ty(), "ufcs_recv_result")
                            .map_err(llvm_err)?;
                        let lock_fn = self
                            .module
                            .get_function("action_mutex_lock")
                            .ok_or("action_mutex_lock not found")?;
                        let unlock_fn = self
                            .module
                            .get_function("action_mutex_unlock")
                            .ok_or("action_mutex_unlock not found")?;
                        let cond_wait_fn = self
                            .module
                            .get_function("action_cond_wait")
                            .ok_or("action_cond_wait not found")?;
                        let mutex_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 0, "rm")
                            .map_err(llvm_err)?;
                        let cond_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 1, "rc")
                            .map_err(llvm_err)?;
                        let closed_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 2, "rc_closed")
                            .map_err(llvm_err)?;
                        let list_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 3, "rl")
                            .map_err(llvm_err)?;
                        let merge_bb = self.context.append_basic_block(cur_fn, "ufcs_merge");
                        let _ = self
                            .builder
                            .build_call(lock_fn, &[mutex_ptr.into()], "")
                            .map_err(llvm_err)?;
                        // Wait loop: while list is empty and not closed, cond_wait
                        let wait_loop_bb =
                            self.context.append_basic_block(cur_fn, "stream_wait_loop");
                        let got_data_bb =
                            self.context.append_basic_block(cur_fn, "stream_got_data");
                        let empty_closed_bb = self
                            .context
                            .append_basic_block(cur_fn, "stream_empty_closed");
                        let _ = self.builder.build_unconditional_branch(wait_loop_bb);
                        self.builder.position_at_end(wait_loop_bb);
                        let list_val = self.load_list(list_ptr)?;
                        let len = self
                            .builder
                            .build_extract_value(list_val, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let has_data = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, len, zero, "has_data")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_conditional_branch(
                            has_data,
                            got_data_bb,
                            empty_closed_bb,
                        );
                        // Empty: check if closed
                        self.builder.position_at_end(empty_closed_bb);
                        let closed_val = self
                            .builder
                            .build_load(self.i64_ty(), closed_ptr, "closed_val")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let is_closed = self
                            .builder
                            .build_int_compare(IntPredicate::NE, closed_val, zero, "is_closed")
                            .map_err(llvm_err)?;
                        let do_wait_bb = self.context.append_basic_block(cur_fn, "do_cond_wait");
                        let return_zero_bb = self.context.append_basic_block(cur_fn, "ret_closed");
                        let _ = self.builder.build_conditional_branch(
                            is_closed,
                            return_zero_bb,
                            do_wait_bb,
                        );
                        self.builder.position_at_end(do_wait_bb);
                        let _ = self
                            .builder
                            .build_call(cond_wait_fn, &[cond_ptr.into(), mutex_ptr.into()], "")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(wait_loop_bb);
                        // Return 0 when closed & empty
                        self.builder.position_at_end(return_zero_bb);
                        let _ = self
                            .builder
                            .build_call(unlock_fn, &[mutex_ptr.into()], "")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(result_alloca, zero)
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Got data: extract, shift, unlock
                        self.builder.position_at_end(got_data_bb);
                        let lv2 = self.load_list(list_ptr)?;
                        let fat = self.call_rt("action_list_get", &[lv2.into(), zero.into()])?;
                        let fat = fat
                            .try_as_basic_value()
                            .basic()
                            .ok_or("receive get failed")?
                            .into_struct_value();
                        let tag = self
                            .builder
                            .build_extract_value(fat, 0, "tag")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let data_ptr = self
                            .builder
                            .build_extract_value(lv2, 0, "data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let len2 = self
                            .builder
                            .build_extract_value(lv2, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let cap = self
                            .builder
                            .build_extract_value(lv2, 2, "cap")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let new_len = self
                            .builder
                            .build_int_sub(len2, one, "new_len")
                            .map_err(llvm_err)?;
                        let has_more = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, len2, one, "has_more")
                            .map_err(llvm_err)?;
                        let shift_bb = self.context.append_basic_block(cur_fn, "shift_bb");
                        let done_bb = self.context.append_basic_block(cur_fn, "shift_done");
                        let _ = self
                            .builder
                            .build_conditional_branch(has_more, shift_bb, done_bb);
                        self.builder.position_at_end(shift_bb);
                        let mm_fn = self
                            .module
                            .get_function("memmove")
                            .ok_or("memmove not found")?;
                        // data_ptr points to the leaf node start (count+pad header).
                        // Shift elements within the elements array (offset 8), preserving the header.
                        let elems_ptr = unsafe {
                            self.builder
                                .build_gep(
                                    self.context.i8_type(),
                                    data_ptr,
                                    &[self.i64_ty().const_int(8, false)],
                                    "elems",
                                )
                                .map_err(llvm_err)
                        }?;
                        let src_ptr = unsafe {
                            self.builder
                                .build_gep(self.string_type, elems_ptr, &[one], "src")
                                .map_err(llvm_err)
                        }?;
                        let elem_size = self.i64_ty().const_int(16, false);
                        let move_bytes = self
                            .builder
                            .build_int_mul(new_len, elem_size, "move_bytes")
                            .map_err(llvm_err)?;
                        let _ = self
                            .builder
                            .build_call(
                                mm_fn,
                                &[elems_ptr.into(), src_ptr.into(), move_bytes.into()],
                                "",
                            )
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(done_bb);
                        self.builder.position_at_end(done_bb);
                        let undef = self.list_type.get_undef();
                        let r1 = self
                            .builder
                            .build_insert_value(undef, data_ptr, 0, "sr1")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_insert_value(r1, new_len, 1, "sr2")
                            .map_err(llvm_err)?;
                        let r3 = self
                            .builder
                            .build_insert_value(r2, cap, 2, "sr3")
                            .map_err(llvm_err)?;
                        self.builder.build_store(list_ptr, r3).map_err(llvm_err)?;
                        let _ = self
                            .builder
                            .build_call(unlock_fn, &[mutex_ptr.into()], "")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(result_alloca, tag)
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge: load result
                        self.builder.position_at_end(merge_bb);
                        let result = self
                            .builder
                            .build_load(self.i64_ty(), result_alloca, "ufcs_load_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        return Ok(TypedValue::Int(result));
                    }
                    "close" => {
                        let stream_ptr = match recv_val {
                            TypedValue::Stream(p) => p,
                            _ => unreachable!(),
                        };
                        let mutex_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 0, "cm")
                            .map_err(llvm_err)?;
                        let _ = self
                            .builder
                            .build_call(
                                self.module.get_function("action_mutex_lock").unwrap(),
                                &[mutex_ptr.into()],
                                "",
                            )
                            .map_err(llvm_err)?;
                        let closed_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 2, "cc")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(closed_ptr, self.i64_ty().const_int(1, false))
                            .map_err(llvm_err)?;
                        let cond_ptr = self
                            .builder
                            .build_struct_gep(self.stream_type, stream_ptr, 1, "ccond")
                            .map_err(llvm_err)?;
                        let _ = self
                            .builder
                            .build_call(
                                self.module.get_function("action_cond_broadcast").unwrap(),
                                &[cond_ptr.into()],
                                "",
                            )
                            .map_err(llvm_err)?;
                        let _ = self
                            .builder
                            .build_call(
                                self.module.get_function("action_mutex_unlock").unwrap(),
                                &[mutex_ptr.into()],
                                "",
                            )
                            .map_err(llvm_err)?;
                        return Ok(TypedValue::Unit);
                    }
                    _ => return Err(format!("Method '{}' not found on Stream", method)),
                }
            }
            // Handle Task builtin methods inline
            // Task struct: {pthread: i64, done: i64, cancelled: i64, result_list: {ptr, i64, i64}}
            if matches!(recv_val, TypedValue::Task(_)) {
                let task_ptr = match recv_val {
                    TypedValue::Task(p) => p,
                    _ => unreachable!(),
                };
                let task_val = self
                    .builder
                    .build_load(self.task_type, task_ptr, "task_val")
                    .map_err(llvm_err)?
                    .into_struct_value();
                match method.as_str() {
                    "cancel" => {
                        let cancelled_one = self.i64_ty().const_int(1, false);
                        let updated = self
                            .builder
                            .build_insert_value(task_val, cancelled_one, 2, "t_canc_set")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(task_ptr, updated)
                            .map_err(llvm_err)?;
                        return Ok(TypedValue::Unit);
                    }
                    "is_done" => {
                        let done = self
                            .builder
                            .build_extract_value(task_val, 1, "is_done")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let is_true = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                done,
                                self.i64_ty().const_int(0, false),
                                "done_bool",
                            )
                            .map_err(llvm_err)?;
                        return Ok(TypedValue::Bool(is_true));
                    }
                    "is_cancelled" => {
                        let cancelled = self
                            .builder
                            .build_extract_value(task_val, 2, "is_canc")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let is_true = self
                            .builder
                            .build_int_compare(
                                IntPredicate::NE,
                                cancelled,
                                self.i64_ty().const_int(0, false),
                                "canc_bool",
                            )
                            .map_err(llvm_err)?;
                        return Ok(TypedValue::Bool(is_true));
                    }
                    "wait" => {
                        // pthread_join then reload task (thread updates result_list)
                        let pthread_val = self
                            .builder
                            .build_extract_value(task_val, 0, "pt")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let pthread_join_fn = self
                            .module
                            .get_function("action_thread_join")
                            .ok_or("action_thread_join not found")?;
                        let null_ptr = self.ptr_ty().const_null();
                        let _ = self
                            .builder
                            .build_call(pthread_join_fn, &[pthread_val.into(), null_ptr.into()], "")
                            .map_err(llvm_err)?;
                        let task_val2 = self
                            .builder
                            .build_load(self.task_type, task_ptr, "task_val2")
                            .map_err(llvm_err)?
                            .into_struct_value();
                        let result_list = self
                            .builder
                            .build_extract_value(task_val2, 4, "wait_list")
                            .map_err(llvm_err)?
                            .into_struct_value();
                        let list_alloca = self
                            .builder
                            .build_alloca(self.list_type, "wait_l")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(list_alloca, result_list)
                            .map_err(llvm_err)?;
                        let list_val = self.load_list(list_alloca)?;
                        let zero = self.i64_ty().const_int(0, false);
                        let cc =
                            self.call_rt("action_list_get", &[list_val.into(), zero.into()])?;
                        let fat = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("wait get failed")?
                            .into_struct_value();
                        let tag = self
                            .builder
                            .build_extract_value(fat, 0, "tag")
                            .map_err(llvm_err)?
                            .into_int_value();
                        return Ok(TypedValue::Int(tag));
                    }
                    _ => return Err(format!("Method '{}' not found on Task", method)),
                }
            }
            // Handle List builtin methods inline — UFCS: list.method(args) ≡ method(list, args...)
            if let TypedValue::List(lp) = &recv_val {
                match method.as_str() {
                    "insert" => return self.builtin_list_insert(*lp, args),
                    "remove" => return self.builtin_list_remove(*lp, args),
                    "append" => return self.builtin_list_append(*lp, args),
                    "len" => {
                        let lv = self.load_list(*lp)?;
                        let len = self.list_len_val(lv)?;
                        self.rc_free_intermediate(&recv_val)?;
                        return Ok(TypedValue::Int(len));
                    }
                    "isEmpty" => {
                        let lv = self.load_list(*lp)?;
                        let len = self.list_len_val(lv)?;
                        let zero = self.i64_ty().const_int(0, false);
                        let is_empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        self.rc_free_intermediate(&recv_val)?;
                        return Ok(TypedValue::Bool(is_empty));
                    }
                    _ => {}
                }
                // Remaining methods: free intermediate then recompile via compile_call
                self.rc_free_method_receiver(&recv_val)?;
                match method.as_str() {
                    // No-arg methods: f(list) — len/isEmpty handled above
                    "head" | "last" | "tail" | "init" | "reverse" | "sum" | "product"
                    | "sorted" | "flatten" | "unique" | "toList" | "toLazyList" => {
                        let new_func = Expr::Ident(method.to_string());
                        return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                    }
                    // Two-arg methods: f(list, arg1, arg2) — dispatch to builtin_stdlib
                    "insert" => {
                        if args.len() != 2 {
                            return Err(format!("list.{} expects 2 arguments", method));
                        }
                        let new_func = Expr::Ident(method.to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone(), args[0].clone(), args[1].clone()],
                            &None,
                        );
                    }
                    // Single-arg methods: f(list, arg) — dispatch to builtin_stdlib
                    "get" | "contains" | "take" | "drop" | "append" | "prepend" | "indexOf"
                    | "slice" | "splitAt" | "chunks" | "windows" | "repeat" | "withIndex"
                    | "remove" | "zip" | "count" | "partition" => {
                        if args.len() != 1 {
                            return Err(format!("list.{} expects 1 argument", method));
                        }
                        let new_func = Expr::Ident(method.to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone(), args[0].clone()],
                            &None,
                        );
                    }
                    // map, filter, fold, any, all, find, reduce, foldRight, takeWhile, dropWhile, flatMap, sortedBy
                    "map" | "filter" | "any" | "all" | "find" | "reduce" | "takeWhile"
                    | "dropWhile" | "flatMap" | "foldRight" | "sortedBy" | "findIndex" => {
                        let new_func = Expr::Ident(method.to_string());
                        return self.compile_call(
                            &new_func,
                            &[receiver.as_ref().clone()],
                            trailing,
                        );
                    }
                    "fold" => {
                        if args.len() < 1 {
                            return Err("list.fold expects at least 1 argument (init)".to_string());
                        }
                        let new_func = Expr::Ident("fold".to_string());
                        let mut new_args = vec![receiver.as_ref().clone()];
                        new_args.extend(args.iter().cloned());
                        return self.compile_call(&new_func, &new_args, trailing);
                    }
                    _ => return Err(format!("Method '{}' not found on List", method)),
                }
            }

            let lookup_key = format!("{}.{}", type_name, method);
            if let Some(fn_name) = self.extension_methods.get(&lookup_key).cloned() {
                let fn_val = self
                    .module
                    .get_function(&fn_name)
                    .ok_or_else(|| format!("Extension method '{}' not found", fn_name))?;
                let fn_type = fn_val.get_type();
                let param_tys = fn_type.get_param_types();
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                let recv_val = self.compile_expr(receiver)?;
                let recv_bv = self.typed_value_to_bv(&recv_val);
                let casted_recv = self.coerce_arg(recv_bv, param_tys.first())?;
                ca.push(casted_recv.into());
                tracked_args.push(recv_val);
                for (i, a) in args.iter().enumerate() {
                    let av = self.compile_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    let casted = self.coerce_arg(bv, param_tys.get(i + 1))?;
                    ca.push(casted.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    let casted = self.coerce_arg(bv, param_tys.get(args.len() + 1))?;
                    ca.push(casted.into());
                }
                let cc = self.builder.build_call(fn_val, &ca, "").map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                return match cc.try_as_basic_value().basic() {
                    Some(bv) => self.bv_to_typed(bv),
                    None => Ok(TypedValue::Unit),
                };
            }
            // If receiver is Map/Set/Stream/Task and no builtin/extension method matched, error out
            if matches!(
                recv_val,
                TypedValue::Map(_)
                    | TypedValue::Set(_)
                    | TypedValue::Stream(_)
                    | TypedValue::Task(_)
            ) {
                return Err(format!(
                    "Method '{}' not found on type '{}'",
                    method, type_name
                ));
            }

            // UFCS fallback: receiver.method(args) → method(receiver, args)
            // Avoid rc_free + AST recompile for List len/isEmpty — that double-evaluates
            // method chains (e.g. lst.remove(0).len()) and can free shared nodes early.
            if matches!(method.as_str(), "len" | "isEmpty") {
                if let TypedValue::List(lp) = &recv_val {
                    let lv = self.load_list(*lp)?;
                    let len = self.list_len_val(lv)?;
                    if method == "isEmpty" {
                        let zero = self.i64_ty().const_int(0, false);
                        let is_empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        self.rc_free_intermediate(&recv_val)?;
                        return Ok(TypedValue::Bool(is_empty));
                    }
                    self.rc_free_intermediate(&recv_val)?;
                    return Ok(TypedValue::Int(len));
                }
                if matches!(recv_val, TypedValue::Map(_) | TypedValue::Set(_)) {
                    let lp = match &recv_val {
                        TypedValue::Map(p) | TypedValue::Set(p) => *p,
                        _ => unreachable!(),
                    };
                    let lv = self.load_list(lp)?;
                    let len = self.map_len_val(lv)?;
                    if method == "isEmpty" {
                        let zero = self.i64_ty().const_int(0, false);
                        let is_empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        self.rc_free_intermediate(&recv_val)?;
                        return Ok(TypedValue::Bool(is_empty));
                    }
                    self.rc_free_intermediate(&recv_val)?;
                    return Ok(TypedValue::Int(len));
                }
            }
            self.rc_free_method_receiver(&recv_val)?;
            let new_func = Expr::Ident(method.to_string());
            let mut new_args = vec![receiver.as_ref().clone()];
            new_args.extend(args.iter().cloned());
            return self.compile_call(&new_func, &new_args, trailing);
        }

        // Higher-order call: compile the call target expression
        let target = self.compile_expr(func)?;
        self.compile_indirect_call(target, args, trailing)
    }

    /// Perform an indirect function call through a TypedValue::Fn, TypedValue::Closure, or TypedValue::Int.
    pub(super) fn compile_indirect_call(
        &mut self,
        target: TypedValue<'ctx>,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        match target {
            TypedValue::Fn(fn_ptr, fn_type) => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                for a in args {
                    let av = self.compile_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    ca.push(bv.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    ca.push(bv.into());
                }

                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect")
                    .map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            TypedValue::Closure {
                fn_ptr,
                actual_fn_type,
                closure_ptr,
                closure_ty,
                alloca,
            } => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                // First arg: the closure struct pointer (captures context)
                ca.push(closure_ptr.into());
                for a in args {
                    let av = self.compile_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    ca.push(bv.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    ca.push(bv.into());
                }

                let cc = self
                    .builder
                    .build_indirect_call(actual_fn_type, fn_ptr, &ca, "indirect_closure")
                    .map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                // Free intermediate closure's captures struct after the call.
                // Scope-variable closures (alloca = Some) are handled by scope cleanup.
                if alloca.is_none() {
                    self.rc_inc(closure_ptr)?;
                    self.rc_dec_closure_captures(closure_ptr, closure_ty)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, actual_fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            // Handle untyped parameters (fallback to Int) used as function callbacks.
            // Use fat return type to preserve enum/string/struct values through the
            // untyped boundary. The named fat_return_type is distinct from enum types,
            // so bv_to_typed won't confuse packed scalars with enums.
            TypedValue::Int(iv) => {
                let total_args = args.len() + trailing.as_ref().map_or(0, |_| 1);
                let param_tys: Vec<BasicMetadataTypeEnum<'ctx>> =
                    (0..total_args).map(|_| self.i64_ty().into()).collect();
                let ret_ty = self.fat_return_type;
                let fn_type = ret_ty.fn_type(&param_tys, false);
                let ptr_type = self.context.ptr_type(inkwell::AddressSpace::default());
                let fn_ptr = self
                    .builder
                    .build_int_to_ptr(iv, ptr_type, "fn_ptr_cast")
                    .map_err(llvm_err)?;
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                let mut tracked_args: Vec<TypedValue<'ctx>> = Vec::new();
                for a in args {
                    let av = self.compile_expr(a)?;
                    let bv = self.typed_value_to_bv(&av);
                    ca.push(bv.into());
                    tracked_args.push(av);
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    ca.push(bv.into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect_untyped")
                    .map_err(llvm_err)?;
                for av in &tracked_args {
                    self.rc_free_intermediate(av)?;
                }
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, Some(BasicTypeEnum::StructType(ret_ty))),
                    None => Ok(TypedValue::Unit),
                }
            }
            _ => Err("Call target is not a function".to_string()),
        }
    }

    /// Convert a TypedValue to a BasicValueEnum suitable for passing as a
    /// function call argument, without re-compiling the expression.
    fn typed_value_to_bv(&self, av: &TypedValue<'ctx>) -> BasicValueEnum<'ctx> {
        av.to_bv().unwrap_or_else(|| match av {
            TypedValue::Str(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.string_type, *ptr, "arg_str")
                    .unwrap();
                ld.into()
            }
            TypedValue::List(ptr) | TypedValue::Map(ptr) | TypedValue::Set(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.list_type, *ptr, "arg_list")
                    .unwrap();
                ld.into()
            }
            TypedValue::LazyList(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "arg_ll")
                    .unwrap();
                ld.into()
            }
            TypedValue::Task(ptr) => {
                let ld = self
                    .builder
                    .build_load(self.task_type, *ptr, "arg_task")
                    .unwrap();
                ld.into()
            }
            TypedValue::Stream(ptr) => {
                let lf = self
                    .builder
                    .build_struct_gep(self.stream_type, *ptr, 3, "arg_slf")
                    .unwrap();
                let ld = self
                    .builder
                    .build_load(self.list_type, lf, "arg_sl")
                    .unwrap();
                ld.into()
            }
            TypedValue::Struct(ptr, st) => {
                let ld = self.builder.build_load(*st, *ptr, "arg_struct").unwrap();
                ld.into()
            }
            TypedValue::Enum(ptr, et, ..) => {
                let ld = self.builder.build_load(*et, *ptr, "arg_enum").unwrap();
                ld.into()
            }
            TypedValue::Nullable(ptr, ty) => {
                self.builder.build_load(*ty, *ptr, "arg_nullable").unwrap()
            }
            TypedValue::CString(p) | TypedValue::Ptr(p) | TypedValue::FileHandle(p) => (*p).into(),
            _ => self.i64_ty().const_int(0, false).into(),
        })
    }
}
