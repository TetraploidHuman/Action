// Submodule: builtins

use crate::ast::*;
use inkwell::types::{BasicMetadataTypeEnum, BasicTypeEnum};
use inkwell::values::{
    BasicMetadataValueEnum, BasicValue, FloatValue, IntValue, PointerValue, StructValue,
};
use inkwell::{FloatPredicate, IntPredicate};

use super::{llvm_err, CodeGen, InnerType, Scope, TypedValue};

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
            if name == "stream" {
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
                        Some(1)
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
                return match cc.try_as_basic_value().basic() {
                    Some(bv) => self.bv_to_typed(bv),
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
                for (i, a) in args.iter().enumerate() {
                    let bv = self.compile_and_load(a)?;
                    let casted = self.coerce_arg(bv, param_tys.get(i))?;
                    ca.push(casted.into());
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    let casted = self.coerce_arg(bv, param_tys.get(args.len()))?;
                    ca.push(casted.into());
                }

                let cc = self.builder.build_call(fn_val, &ca, "").map_err(llvm_err)?;
                return match cc.try_as_basic_value().basic() {
                    Some(bv) => self.bv_to_typed(bv),
                    None => Ok(TypedValue::Unit),
                };
            }
            // Not a module function - fall through to higher-order path (it might be a variable holding a lambda)
        }

        // Module-qualified call: module.function(args) → module_function(args)
        if let Expr::FieldAccess(module_expr, method) = func {
            if let Expr::Ident(module_name) = module_expr.as_ref() {
                // List.of(...) → List[...] (equivalent to list literal)
                if module_name == "List" && method == "of" {
                    return self.builtin_list(args);
                }
                // Set.of(...) → Set literal
                if module_name == "Set" && method == "of" {
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
                    for (i, a) in args.iter().enumerate() {
                        let bv = self.compile_and_load(a)?;
                        let casted = self.coerce_arg(bv, param_tys.get(i))?;
                        ca.push(casted.into());
                    }
                    if let Some(lam) = trailing {
                        let bv = self.compile_and_load(lam)?;
                        let casted = self.coerce_arg(bv, param_tys.get(args.len()))?;
                        ca.push(casted.into());
                    }
                    let cc = self.builder.build_call(fn_val, &ca, "").map_err(llvm_err)?;
                    return match cc.try_as_basic_value().basic() {
                        Some(bv) => self.bv_to_typed(bv),
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
                if method == "insert" {
                    return self.builtin_map_insert(receiver, args);
                }
                if method == "remove" {
                    return self.builtin_map_remove(receiver, args);
                }
                if method == "contains" {
                    return self.builtin_map_contains(receiver, args);
                }
                if method == "len" || method == "isEmpty" {
                    let new_func = Expr::Ident(method.to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
                if method == "keys" {
                    let new_func = Expr::Ident("mapKeys".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
                if method == "values" {
                    // map.values() -> get all values as a list
                    let new_func = Expr::Ident("mapValues".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
                if method == "mapValues" {
                    // map.mapValues(transform) -> mapMapValues(map, transform)
                    let new_func = Expr::Ident("mapMapValues".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], trailing);
                }
                if method == "entries" {
                    let new_func = Expr::Ident("mapEntries".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
                if method == "union" {
                    if args.len() != 1 {
                        return Err("map.union expects 1 argument (other map)".to_string());
                    }
                    let new_func = Expr::Ident("mapUnion".to_string());
                    return self.compile_call(
                        &new_func,
                        &[receiver.as_ref().clone(), args[0].clone()],
                        &None,
                    );
                }
                if method == "filter" {
                    // map.filter(predicate) -> mapFilter(map, predicate)
                    let new_func = Expr::Ident("mapFilter".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], trailing);
                }
                if method == "fold" {
                    // map.fold(init, folder) -> mapFold(map, init, folder)
                    let new_func = Expr::Ident("mapFold".to_string());
                    let mut new_args = vec![receiver.as_ref().clone()];
                    new_args.extend(args.iter().cloned());
                    return self.compile_call(&new_func, &new_args, trailing);
                }
            }
            // Handle Set builtin methods inline
            if matches!(recv_val, TypedValue::Set(_)) {
                if method == "insert" {
                    return self.builtin_set_insert(receiver, args);
                }
                if method == "remove" {
                    return self.builtin_set_remove(receiver, args);
                }
                if method == "contains" {
                    return self.builtin_set_contains(receiver, args);
                }
                if method == "len" || method == "isEmpty" {
                    let new_func = Expr::Ident(method.to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
                if method == "union" {
                    if args.len() != 1 {
                        return Err("set.union expects 1 argument (other set)".to_string());
                    }
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
                    let new_func = Expr::Ident("setIsSubset".to_string());
                    return self.compile_call(
                        &new_func,
                        &[receiver.as_ref().clone(), args[0].clone()],
                        &None,
                    );
                }
                if method == "toList" {
                    let new_func = Expr::Ident("toList".to_string());
                    return self.compile_call(&new_func, &[receiver.as_ref().clone()], &None);
                }
            }
            // Handle Range builtin methods inline (range is a Struct with 3 i64 fields)
            if let TypedValue::Struct(_, st) = &recv_val {
                if *st == self.range_type {
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
                                .build_gep(self.context.i8_type(), data_ptr, &[self.i64_ty().const_int(8, false)], "elems")
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
            if matches!(recv_val, TypedValue::List(_) | TypedValue::LazyList(_)) {
                match method.as_str() {
                    // No-arg methods: f(list)
                    "len" | "isEmpty" | "head" | "last" | "tail" | "init" | "reverse" | "sum"
                    | "product" | "sorted" | "flatten" | "unique" | "toList" | "toLazyList" => {
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
                let recv_bv = self.compile_and_load(receiver)?;
                let casted_recv = self.coerce_arg(recv_bv, param_tys.first())?;
                ca.push(casted_recv.into());
                for (i, a) in args.iter().enumerate() {
                    let bv = self.compile_and_load(a)?;
                    let casted = self.coerce_arg(bv, param_tys.get(i + 1))?;
                    ca.push(casted.into());
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    let casted = self.coerce_arg(bv, param_tys.get(args.len() + 1))?;
                    ca.push(casted.into());
                }
                let cc = self.builder.build_call(fn_val, &ca, "").map_err(llvm_err)?;
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
                for a in args {
                    let bv = self.compile_and_load(a)?;
                    ca.push(bv.into());
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    ca.push(bv.into());
                }

                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect")
                    .map_err(llvm_err)?;
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, fn_type.get_return_type()),
                    None => Ok(TypedValue::Unit),
                }
            }
            TypedValue::Closure {
                fn_ptr,
                actual_fn_type,
                closure_ptr,
                closure_ty: _,
            } => {
                let mut ca: Vec<BasicMetadataValueEnum> = Vec::new();
                // First arg: the closure struct pointer (captures context)
                ca.push(closure_ptr.into());
                for a in args {
                    let bv = self.compile_and_load(a)?;
                    ca.push(bv.into());
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    ca.push(bv.into());
                }

                let cc = self
                    .builder
                    .build_indirect_call(actual_fn_type, fn_ptr, &ca, "indirect_closure")
                    .map_err(llvm_err)?;
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
                for a in args {
                    let bv = self.compile_and_load(a)?;
                    ca.push(bv.into());
                }
                if let Some(lam) = trailing {
                    let bv = self.compile_and_load(lam)?;
                    ca.push(bv.into());
                }
                let cc = self
                    .builder
                    .build_indirect_call(fn_type, fn_ptr, &ca, "indirect_untyped")
                    .map_err(llvm_err)?;
                match cc.try_as_basic_value().basic() {
                    Some(bv) => self.unpack_fat_return(bv, Some(BasicTypeEnum::StructType(ret_ty))),
                    None => Ok(TypedValue::Unit),
                }
            }
            _ => Err("Call target is not a function".to_string()),
        }
    }

    pub(super) fn builtin_print(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.is_empty() {
            if name == "println" {
                let _ = self.call_rt("action_println", &[]);
            }
            return Ok(TypedValue::Unit);
        }
        let v = self.compile_expr(&args[0])?;
        match &v {
            TypedValue::Int(_) => {
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_int", &[bv.into()]);
                }
            }
            TypedValue::Float(_) => {
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_float", &[bv.into()]);
                }
            }
            TypedValue::Bool(_) => {
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_bool", &[bv.into()]);
                }
            }
            TypedValue::Str(ptr) => {
                let _ = self.call_rt_with_str("action_print_string", *ptr);
            }
            TypedValue::Fn(_, _) | TypedValue::Closure { .. } => {
                /* print fn/closure pointer as int */
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_int", &[bv.into()]);
                }
            }
            TypedValue::List(ptr) | TypedValue::Set(ptr) | TypedValue::Map(ptr) => {
                let list = self.load_list(*ptr)?;
                let _ = self.call_rt("action_list_print", &[list.into()]);
            }
            TypedValue::Task(ptr) => {
                let task_val = self
                    .builder
                    .build_load(self.task_type, *ptr, "print_task")
                    .map_err(llvm_err)?;
                let _ = self.call_rt("action_print_task", &[task_val.into()]);
            }
            TypedValue::Stream(ptr) => {
                // Stream is {mutex, cond, closed, list}; load list from field 3
                let list_field = self
                    .builder
                    .build_struct_gep(self.stream_type, *ptr, 3, "print_sl_field")
                    .map_err(llvm_err)?;
                let list_val = self
                    .builder
                    .build_load(self.list_type, list_field, "print_sl")
                    .map_err(llvm_err)?;
                let _ = self.call_rt("action_list_print", &[list_val.into()]);
            }
            TypedValue::LazyList(ptr) => {
                let list_val = self
                    .builder
                    .build_load(self.list_type, *ptr, "print_ll")
                    .map_err(llvm_err)?;
                let _ = self.call_rt("action_list_print", &[list_val.into()]);
            }
            TypedValue::CString(_p) | TypedValue::Ptr(_p) | TypedValue::FileHandle(_p) => {
                // Print pointer value as hex
                if let Some(bv) = v.to_bv() {
                    let _ = self.call_rt("action_print_int", &[bv.into()]);
                }
            }
            TypedValue::Struct(_, _) => {
                let _ = self.call_rt("action_print_struct", &[]);
            }
            TypedValue::Enum(ptr, _, inner_type, _) => {
                let enum_st = self
                    .context
                    .struct_type(&[self.i64_ty().into(), self.ptr_ty().into()], false);
                let loaded = self
                    .builder
                    .build_load(enum_st, *ptr, "print_enum_ld")
                    .map_err(llvm_err)?;
                if *inner_type == InnerType::Float {
                    let _ = self.call_rt("action_print_enum_float", &[loaded.into()]);
                } else {
                    let _ = self.call_rt("action_print_enum", &[loaded.into()]);
                }
            }
            TypedValue::Nullable(ptr, inner_bt) => {
                // Print nullable: check null flag, print "null" or inner value
                let loaded = self
                    .builder
                    .build_load(*inner_bt, *ptr, "print_null_ld")
                    .map_err(llvm_err)?;
                let nullable_struct = loaded.into_struct_value();
                let null_flag = self
                    .builder
                    .build_extract_value(nullable_struct, 0, "print_null_flag")
                    .map_err(llvm_err)?
                    .into_int_value();

                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("Cannot print outside function")?;
                let is_null = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        null_flag,
                        self.null_flag_ty().const_int(1, false),
                        "print_is_null",
                    )
                    .map_err(llvm_err)?;

                let null_block = self.context.append_basic_block(current_fn, "print_null");
                let val_block = self.context.append_basic_block(current_fn, "print_val");
                let merge_block = self.context.append_basic_block(current_fn, "print_merge");

                self.builder
                    .build_conditional_branch(is_null, null_block, val_block)
                    .map_err(llvm_err)?;

                // Print "null" using printf
                self.builder.position_at_end(null_block);
                if let Some(printf_fn) = self.module.get_function("printf") {
                    let null_str = self
                        .builder
                        .build_global_string_ptr("null", "null_str")
                        .map_err(llvm_err)?
                        .as_pointer_value();
                    let _ =
                        self.builder
                            .build_call(printf_fn, &[null_str.into()], "print_null_call");
                }
                self.builder
                    .build_unconditional_branch(merge_block)
                    .map_err(llvm_err)?;

                // Print inner value
                self.builder.position_at_end(val_block);
                let inner = self
                    .builder
                    .build_extract_value(nullable_struct, 1, "print_inner")
                    .map_err(llvm_err)?;
                let inner_typed = self.bv_to_typed(inner)?;
                match &inner_typed {
                    TypedValue::Int(v) => {
                        let _ = self.call_rt("action_print_int", &[v.as_basic_value_enum().into()]);
                    }
                    TypedValue::Float(v) => {
                        let _ =
                            self.call_rt("action_print_float", &[v.as_basic_value_enum().into()]);
                    }
                    TypedValue::Bool(v) => {
                        let _ =
                            self.call_rt("action_print_bool", &[v.as_basic_value_enum().into()]);
                    }
                    TypedValue::Str(v) => {
                        let _ = self.call_rt_with_str("action_print_string", *v);
                    }
                    _ => {
                        let bv = inner_typed
                            .to_bv()
                            .unwrap_or_else(|| self.i64_ty().const_int(0, false).into());
                        let _ = self.call_rt("action_print_int", &[bv.into()]);
                    }
                }
                self.builder
                    .build_unconditional_branch(merge_block)
                    .map_err(llvm_err)?;

                self.builder.position_at_end(merge_block);
            }
            TypedValue::Unit => {}
        }
        if name == "println" {
            let _ = self.call_rt("action_println", &[]);
        }
        Ok(TypedValue::Unit)
    }

    pub(super) fn builtin_list(&mut self, args: &[Expr]) -> Result<TypedValue<'ctx>, String> {
        let len = self.i64_ty().const_int(args.len() as u64, false);
        let cc = self.call_rt("action_list_create", &[len.into()])?;
        let list_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let list_alloca = self
            .builder
            .build_alloca(self.list_type, "list_tmp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(list_alloca, list_bv)
            .map_err(llvm_err)?;

        for arg in args {
            let v = self.compile_expr(arg)?;
            let elem_fat = self.to_fat_struct(&v)?;
            let list_val = self.load_list(list_alloca)?;
            let cc = self.call_rt("action_list_push", &[list_val.into(), elem_fat.into()])?;
            let new_list = cc.try_as_basic_value().basic().ok_or("list_push failed")?;
            self.builder
                .build_store(list_alloca, new_list)
                .map_err(llvm_err)?;
        }

        Ok(TypedValue::List(list_alloca))
    }

    /// lazy_list(seed) - create a lazy list with a seed value
    /// lazy_list(seed) { fn } - create a lazy list with seed and step function
    pub(super) fn builtin_lazy_list(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if args.is_empty() {
            return Err("lazy_list expects at least 1 argument (seed)".to_string());
        }
        let seed = self.compile_expr(&args[0])?;
        let seed_i64 = match &seed {
            TypedValue::Int(v) => *v,
            _ => return Err("lazy_list: seed must be an Int".to_string()),
        };

        // Compile step function if provided
        let (step_fn_ptr, state, take_count) = if let Some(lam) = trailing {
            let step_fn_val = self.compile_lambda_for_lazy(lam)?;
            // -1 means "infinite" — only bounded by explicit take()
            (
                step_fn_val,
                seed_i64,
                self.i64_ty().const_int(-1_i64 as u64, true),
            )
        } else {
            // No step function: only the seed element
            (
                self.ptr_ty().const_null(),
                self.i64_ty().const_int(0, false),
                self.i64_ty().const_int(0, false),
            )
        };

        // Build LazyList struct: {head_val: i64, step_fn: i8*, state: i64, take_count: i64, map_fn: i8*}
        let ll_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "ll")
            .map_err(llvm_err)?;
        let undef = self.lazylist_type.get_undef();
        let v0 = self
            .builder
            .build_insert_value(undef, seed_i64, 0, "ll_head")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn_ptr, 1, "ll_fn")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state, 2, "ll_state")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, take_count, 3, "ll_tc")
            .map_err(llvm_err)?;
        let v4 = self
            .builder
            .build_insert_value(v3, self.ptr_ty().const_null(), 4, "ll_map")
            .map_err(llvm_err)?;
        let v5 = self
            .builder
            .build_insert_value(v4, self.ptr_ty().const_null(), 5, "ll_filt")
            .map_err(llvm_err)?;
        self.builder.build_store(ll_alloca, v5).map_err(llvm_err)?;
        Ok(TypedValue::LazyList(ll_alloca))
    }

    /// Compile a lambda for use as a lazy list step function.
    /// Returns a function pointer that can be called with (i64 state) -> next_i64.
    fn compile_lambda_for_lazy(
        &mut self,
        lam: &Expr,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        match lam {
            Expr::Lambda { params, body, .. } => {
                if params.is_empty() {
                    return Err("lazy_list step function expects 1 parameter".to_string());
                }
                let fn_val = self.compile_lambda(params, body)?;
                match fn_val {
                    TypedValue::Fn(ptr, _) => Ok(ptr),
                    TypedValue::Closure { fn_ptr, .. } => Ok(fn_ptr),
                    _ => Err("lazy_list: step function compilation failed".to_string()),
                }
            }
            _ => Err("lazy_list: expected lambda body".to_string()),
        }
    }

    // ---- LazyList operations ----

    /// If the value is a LazyList, convert it to a List and return the list alloca pointer.
    /// If it's already a List, return the pointer directly.
    fn ensure_list_ptr(
        &self,
        val: &TypedValue<'ctx>,
        prefix: &str,
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        match val {
            TypedValue::LazyList(_) => {
                let list_sv = self.convert_lazylist_to_list(val)?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, &format!("{}_list", prefix))
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(alloca, list_sv)
                    .map_err(llvm_err)?;
                Ok(alloca)
            }
            TypedValue::List(p) => Ok(*p),
            _ => Err(format!("{}: argument must be a List or LazyList", prefix)),
        }
    }

    /// Convert a LazyList to a List struct value (i.e., the loaded StructValue of the list).
    /// This forces evaluation: iterates the step function take_count times.
    pub(super) fn convert_lazylist_to_list(
        &self,
        ll_val: &TypedValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let ll_ptr = match ll_val {
            TypedValue::LazyList(p) => *p,
            _ => return Err("convert_lazylist_to_list: expected LazyList".to_string()),
        };
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, ll_ptr, "ll_conv")
            .map_err(llvm_err)?;
        let ll_struct = ll_sv.into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_struct, 0, "ll_head")
            .map_err(llvm_err)?
            .into_int_value();
        let step_fn = self
            .builder
            .build_extract_value(ll_struct, 1, "ll_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let state_val = self
            .builder
            .build_extract_value(ll_struct, 2, "ll_state")
            .map_err(llvm_err)?
            .into_int_value();
        let take_count_val = self
            .builder
            .build_extract_value(ll_struct, 3, "ll_tc")
            .map_err(llvm_err)?
            .into_int_value();
        let map_fn = self
            .builder
            .build_extract_value(ll_struct, 4, "ll_map")
            .map_err(llvm_err)?
            .into_pointer_value();
        let filter_fn = self
            .builder
            .build_extract_value(ll_struct, 5, "ll_filt")
            .map_err(llvm_err)?
            .into_pointer_value();

        let zero = self.i64_ty().const_int(0, false);
        let one = self.i64_ty().const_int(1, false);
        let neg_one = self.i64_ty().const_int((-1_i64) as u64, true);

        let has_step = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                step_fn,
                self.ptr_ty().const_null(),
                "has_step",
            )
            .map_err(llvm_err)?;
        let state_nz = self
            .builder
            .build_int_compare(IntPredicate::NE, state_val, zero, "state_nz")
            .map_err(llvm_err)?;
        // list-backed: no step fn but state holds a valid data pointer (from toLazyList)
        let not_has_step = self
            .builder
            .build_not(has_step, "not_has_step")
            .map_err(llvm_err)?;
        let is_list_backed = self
            .builder
            .build_and(not_has_step, state_nz, "is_list_backed")
            .map_err(llvm_err)?;

        let tc_gt_zero = self
            .builder
            .build_int_compare(IntPredicate::SGT, take_count_val, zero, "tc_gt0")
            .map_err(llvm_err)?;
        let tc_is_neg1 = self
            .builder
            .build_int_compare(IntPredicate::EQ, take_count_val, neg_one, "tc_neg1")
            .map_err(llvm_err)?;
        let tc_or_inf = self
            .builder
            .build_or(tc_gt_zero, tc_is_neg1, "tc_or_inf")
            .map_err(llvm_err)?;
        let should_generate = self
            .builder
            .build_and(has_step, tc_or_inf, "should_gen")
            .map_err(llvm_err)?;

        // Compute final_count:
        //   list-backed: use take_count (at least 1 for the already-pushed head)
        //   has_step:    use max(1, take_count)
        //   head-only:   1
        let total_elems = self
            .builder
            .build_select(tc_is_neg1, one, take_count_val, "total_raw")
            .map_err(llvm_err)?
            .into_int_value();
        let step_count = self
            .builder
            .build_select(has_step, total_elems, one, "step_count")
            .map_err(llvm_err)?
            .into_int_value();
        // For list-backed, take_count is already the right count (>=0); ensure at least 1 for head
        let lb_count = self
            .builder
            .build_select(tc_gt_zero, take_count_val, one, "lb_count")
            .map_err(llvm_err)?
            .into_int_value();
        let final_count = self
            .builder
            .build_select(is_list_backed, lb_count, step_count, "final_count")
            .map_err(llvm_err)?
            .into_int_value();

        // Create result list
        let cc = self.call_rt("action_list_create", &[final_count.into()])?;
        let list_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "ll_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, list_bv)
            .map_err(llvm_err)?;

        let has_map = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                map_fn,
                self.ptr_ty().const_null(),
                "has_map",
            )
            .map_err(llvm_err)?;
        let has_filter = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                filter_fn,
                self.ptr_ty().const_null(),
                "has_filt",
            )
            .map_err(llvm_err)?;
        let map_fn_type = self.string_type.fn_type(&[self.i64_ty().into()], false);
        let filt_fn_type = self.string_type.fn_type(&[self.i64_ty().into()], false);
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;

        // ---- Head push with optional map and filter ----
        // Flow: map_head_bb / no_map_head_bb → head_check_bb
        //       head_check_bb: phi → if has_filter → call_filt_head_bb else → head_push_bb
        //       call_filt_head_bb: call filter → if pass → head_push_bb else → head_skip_bb
        //       head_push_bb: push, i=1 → after_head_bb
        //       head_skip_bb: i=0 → after_head_bb
        //       after_head_bb: check need_more → loop_hdr or loop_exit
        let map_head_bb = self.context.append_basic_block(current_fn, "map_head");
        let no_map_head_bb = self.context.append_basic_block(current_fn, "no_map_head");
        let head_check_bb = self.context.append_basic_block(current_fn, "head_check");
        let call_filt_head_bb = self
            .context
            .append_basic_block(current_fn, "call_filt_head");
        let head_push_bb = self.context.append_basic_block(current_fn, "head_push");
        let head_skip_bb = self.context.append_basic_block(current_fn, "head_skip");
        let after_head_bb = self.context.append_basic_block(current_fn, "after_head");
        let _ = self
            .builder
            .build_conditional_branch(has_map, map_head_bb, no_map_head_bb);

        // Map head
        self.builder.position_at_end(map_head_bb);
        let mapped_head = self
            .builder
            .build_indirect_call(map_fn_type, map_fn, &[head_val.into()], "mh_call")
            .map_err(llvm_err)?;
        let mapped_head_bv = mapped_head
            .try_as_basic_value()
            .basic()
            .ok_or("map head call failed")?;
        let mapped_head_val = if mapped_head_bv.is_struct_value() {
            self.builder
                .build_extract_value(mapped_head_bv.into_struct_value(), 0, "mh_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            mapped_head_bv.into_int_value()
        };
        let _ = self.builder.build_unconditional_branch(head_check_bb);

        // No map head
        self.builder.position_at_end(no_map_head_bb);
        let _ = self.builder.build_unconditional_branch(head_check_bb);

        // ---- head_check_bb: phi for head, then branch on has_filter ----
        self.builder.position_at_end(head_check_bb);
        let head_phi = self
            .builder
            .build_phi(self.i64_ty(), "head_phi")
            .map_err(llvm_err)?;
        head_phi.add_incoming(&[(&mapped_head_val, map_head_bb), (&head_val, no_map_head_bb)]);
        let candidate_head = head_phi.as_basic_value().into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(has_filter, call_filt_head_bb, head_push_bb);

        // ---- call_filt_head_bb: call filter on head ----
        self.builder.position_at_end(call_filt_head_bb);
        let filt_head_call = self
            .builder
            .build_indirect_call(filt_fn_type, filter_fn, &[candidate_head.into()], "fh_call")
            .map_err(llvm_err)?;
        let filt_head_bv = filt_head_call
            .try_as_basic_value()
            .basic()
            .ok_or("filt head call failed")?;
        let filt_head_tag = if filt_head_bv.is_struct_value() {
            self.builder
                .build_extract_value(filt_head_bv.into_struct_value(), 0, "fh_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            filt_head_bv.into_int_value()
        };
        let head_passes = self
            .builder
            .build_int_compare(IntPredicate::NE, filt_head_tag, zero, "head_passes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(head_passes, head_push_bb, head_skip_bb);

        // ---- head_push_bb: push head, i=1 ----
        self.builder.position_at_end(head_push_bb);
        let head_fat = self.make_int_fat(candidate_head)?;
        let cur_list_h = self.load_list(result_alloca)?;
        let cc_h = self.call_rt("action_list_push", &[cur_list_h.into(), head_fat.into()])?;
        let new_list_h = cc_h
            .try_as_basic_value()
            .basic()
            .ok_or("push head failed")?;
        self.builder
            .build_store(result_alloca, new_list_h)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(after_head_bb);

        // ---- head_skip_bb: head filtered out, i=0 ----
        self.builder.position_at_end(head_skip_bb);
        let _ = self.builder.build_unconditional_branch(after_head_bb);

        // ---- after_head_bb: init i counter and state, check need_more ----
        self.builder.position_at_end(after_head_bb);
        let i_init_phi = self
            .builder
            .build_phi(self.i64_ty(), "i_init")
            .map_err(llvm_err)?;
        i_init_phi.add_incoming(&[(&one, head_push_bb), (&zero, head_skip_bb)]);
        let i_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ll_i")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i_init_phi.as_basic_value().into_int_value())
            .map_err(llvm_err)?;
        let state_phi_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ll_state_phi")
            .map_err(llvm_err)?;
        self.builder
            .build_store(state_phi_alloca, state_val)
            .map_err(llvm_err)?;

        let need_more = self
            .builder
            .build_or(should_generate, is_list_backed, "need_more")
            .map_err(llvm_err)?;

        let loop_hdr = self.context.append_basic_block(current_fn, "ll_gen_hdr");
        let loop_body = self.context.append_basic_block(current_fn, "ll_gen_body");
        let loop_exit = self.context.append_basic_block(current_fn, "ll_gen_exit");
        let _ = self
            .builder
            .build_conditional_branch(need_more, loop_hdr, loop_exit);

        // ---- loop_hdr: check i < final_count ----
        self.builder.position_at_end(loop_hdr);
        let i_loaded = self
            .builder
            .build_load(self.i64_ty(), i_alloca, "ll_i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_loaded, final_count, "ll_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        // ---- loop_body: generate next element ----
        self.builder.position_at_end(loop_body);

        let data_ptr = self
            .builder
            .build_int_to_ptr(state_val, self.ptr_ty(), "data_ptr")
            .map_err(llvm_err)?;
        let step_block = self.context.append_basic_block(current_fn, "ll_step_blk");
        let lb_block = self.context.append_basic_block(current_fn, "ll_lb_blk");
        let merge_block = self.context.append_basic_block(current_fn, "ll_merge_blk");
        let _ = self
            .builder
            .build_conditional_branch(is_list_backed, lb_block, step_block);

        // Step-function path
        self.builder.position_at_end(step_block);
        let current_state = self
            .builder
            .build_load(self.i64_ty(), state_phi_alloca, "ll_cur_state")
            .map_err(llvm_err)?
            .into_int_value();
        let fn_type = self.fat_return_type.fn_type(&[self.i64_ty().into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, step_fn, &[current_state.into()], "ll_step_call")
            .map_err(llvm_err)?;
        let step_fat = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("step call returned void")?;
        let step_fat_sv = step_fat.into_struct_value();
        let step_elem = self
            .builder
            .build_extract_value(step_fat_sv, 0, "ll_next")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(state_phi_alloca, step_elem)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        // List-backed path
        self.builder.position_at_end(lb_block);
        let elem_gep = unsafe {
            self.builder
                .build_gep(self.fat_return_type, data_ptr, &[i_loaded], "lb_gep")
                .map_err(llvm_err)
        }?;
        let elem_fat = self
            .builder
            .build_load(self.fat_return_type, elem_gep, "lb_fat")
            .map_err(llvm_err)?
            .into_struct_value();
        let lb_elem = self
            .builder
            .build_extract_value(elem_fat, 0, "lb_elem")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Merge element
        self.builder.position_at_end(merge_block);
        let phi = self
            .builder
            .build_phi(self.i64_ty(), "elem_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&step_elem, step_block), (&lb_elem, lb_block)]);
        let elem_val = phi.as_basic_value().into_int_value();

        // Apply map_fn if present
        let map_elem_bb = self.context.append_basic_block(current_fn, "map_elem");
        let no_map_elem_bb = self.context.append_basic_block(current_fn, "no_map_elem");
        let filt_elem_check_bb = self
            .context
            .append_basic_block(current_fn, "filt_elem_check");
        let _ = self
            .builder
            .build_conditional_branch(has_map, map_elem_bb, no_map_elem_bb);

        self.builder.position_at_end(map_elem_bb);
        let mapped_elem_call = self
            .builder
            .build_indirect_call(map_fn_type, map_fn, &[elem_val.into()], "me_call")
            .map_err(llvm_err)?;
        let mapped_elem_bv = mapped_elem_call
            .try_as_basic_value()
            .basic()
            .ok_or("map elem call failed")?;
        let mapped_elem_val = if mapped_elem_bv.is_struct_value() {
            self.builder
                .build_extract_value(mapped_elem_bv.into_struct_value(), 0, "me_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            mapped_elem_bv.into_int_value()
        };
        let _ = self.builder.build_unconditional_branch(filt_elem_check_bb);

        self.builder.position_at_end(no_map_elem_bb);
        let _ = self.builder.build_unconditional_branch(filt_elem_check_bb);

        // ---- filt_elem_check_bb: phi for mapped/unmapped elem, branch on has_filter ----
        self.builder.position_at_end(filt_elem_check_bb);
        let elem_phi_filt = self
            .builder
            .build_phi(self.i64_ty(), "elem_phi_filt")
            .map_err(llvm_err)?;
        elem_phi_filt.add_incoming(&[(&mapped_elem_val, map_elem_bb), (&elem_val, no_map_elem_bb)]);
        let candidate_elem = elem_phi_filt.as_basic_value().into_int_value();

        let call_filt_elem_bb = self
            .context
            .append_basic_block(current_fn, "call_filt_elem");
        let elem_pass_bb = self.context.append_basic_block(current_fn, "elem_pass");
        let elem_fail_bb = self.context.append_basic_block(current_fn, "elem_fail");
        let _ = self
            .builder
            .build_conditional_branch(has_filter, call_filt_elem_bb, elem_pass_bb);

        // ---- call_filt_elem_bb: call filter on element ----
        self.builder.position_at_end(call_filt_elem_bb);
        let filt_elem_call = self
            .builder
            .build_indirect_call(filt_fn_type, filter_fn, &[candidate_elem.into()], "fe_call")
            .map_err(llvm_err)?;
        let filt_elem_bv = filt_elem_call
            .try_as_basic_value()
            .basic()
            .ok_or("filt elem call failed")?;
        let filt_elem_tag = if filt_elem_bv.is_struct_value() {
            self.builder
                .build_extract_value(filt_elem_bv.into_struct_value(), 0, "fe_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            filt_elem_bv.into_int_value()
        };
        let elem_passes = self
            .builder
            .build_int_compare(IntPredicate::NE, filt_elem_tag, zero, "elem_passes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(elem_passes, elem_pass_bb, elem_fail_bb);

        // ---- elem_pass_bb: push element, increment i ----
        self.builder.position_at_end(elem_pass_bb);
        let elem_fat = self.make_int_fat(candidate_elem)?;
        let cur_list2 = self.load_list(result_alloca)?;
        let cc2 = self.call_rt("action_list_push", &[cur_list2.into(), elem_fat.into()])?;
        let new_list2 = cc2.try_as_basic_value().basic().ok_or("push2 failed")?;
        self.builder
            .build_store(result_alloca, new_list2)
            .map_err(llvm_err)?;
        let new_i = self
            .builder
            .build_int_add(i_loaded, one, "ll_i_inc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, new_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        // ---- elem_fail_bb: skip this element, try next ----
        self.builder.position_at_end(elem_fail_bb);
        let _ = self.builder.build_unconditional_branch(loop_body);

        // ---- loop_exit ----
        self.builder.position_at_end(loop_exit);
        let final_list = self.load_list(result_alloca)?;
        Ok(final_list)
    }

    /// Create a fat struct {i64, i8*} from an i64 value (using string_type to match list_push expectations)
    fn make_int_fat(
        &self,
        val: inkwell::values::IntValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let undef = self.string_type.get_undef();
        let null_ptr = self.ptr_ty().const_null();
        let aggregate = self
            .builder
            .build_insert_value(undef, val, 0, "fat_v")
            .map_err(llvm_err)?;
        let aggregate2 = self
            .builder
            .build_insert_value(aggregate, null_ptr, 1, "fat_p")
            .map_err(llvm_err)?;
        Ok(aggregate2.into_struct_value())
    }

    /// range.contains(value): check if value is within the range [start, end) or [start, end]
    pub(super) fn builtin_range_contains(
        &mut self,
        range_expr: &Expr,
        val_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let range_val = self.compile_expr(range_expr)?;
        let val_val = self.compile_expr(val_expr)?;
        let (ptr, st) = match range_val {
            TypedValue::Struct(p, s) => (p, s),
            _ => return Err("range.contains requires a range value".to_string()),
        };
        let val_int = match val_val {
            TypedValue::Int(v) => v,
            _ => return Err("range.contains requires an integer argument".to_string()),
        };
        let bt: BasicTypeEnum = st.into();
        let loaded = self
            .builder
            .build_load(bt, ptr, "range_ld")
            .map_err(llvm_err)?
            .into_struct_value();
        let start = self
            .builder
            .build_extract_value(loaded, 0, "r_start")
            .map_err(llvm_err)?
            .into_int_value();
        let end = self
            .builder
            .build_extract_value(loaded, 1, "r_end")
            .map_err(llvm_err)?
            .into_int_value();
        let _inclusive = self
            .builder
            .build_extract_value(loaded, 2, "r_inc")
            .map_err(llvm_err)?
            .into_int_value();
        let ge_start = self
            .builder
            .build_int_compare(IntPredicate::SGE, val_int, start, "ge_s")
            .map_err(llvm_err)?;
        // If inclusive, use SLE; otherwise SLT
        let end_cmp = self
            .builder
            .build_int_compare(IntPredicate::SLE, val_int, end, "le_e")
            .map_err(llvm_err)?;
        let result = self
            .builder
            .build_and(ge_start, end_cmp, "in_range")
            .map_err(llvm_err)?;
        Ok(TypedValue::Bool(result))
    }

    /// range.toList(): expand the range into a List<Int> of all values
    pub(super) fn builtin_range_to_list(
        &mut self,
        range_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let range_val = self.compile_expr(range_expr)?;
        let (ptr, st) = match range_val {
            TypedValue::Struct(p, s) => (p, s),
            _ => return Err("range.toList requires a range value".to_string()),
        };
        let bt: BasicTypeEnum = st.into();
        let loaded = self
            .builder
            .build_load(bt, ptr, "range_ld")
            .map_err(llvm_err)?
            .into_struct_value();
        let start_val = self
            .builder
            .build_extract_value(loaded, 0, "r_start")
            .map_err(llvm_err)?
            .into_int_value();
        let end_val = self
            .builder
            .build_extract_value(loaded, 1, "r_end")
            .map_err(llvm_err)?
            .into_int_value();
        let inclusive = self
            .builder
            .build_extract_value(loaded, 2, "r_inc")
            .map_err(llvm_err)?
            .into_int_value();

        // end_bound = end + inclusive (for inclusive range, iterate up to and including end)
        let end_bound = self
            .builder
            .build_int_add(end_val, inclusive, "end_bound")
            .map_err(llvm_err)?;

        // Create list and store in alloca
        let cap_val = self.i64_ty().const_int(16, false);
        let list_cc = self.call_rt("action_list_create", &[cap_val.into()])?;
        let list_bv = list_cc
            .try_as_basic_value()
            .basic()
            .ok_or("range_toList create fail")?;
        let list_alloca = self
            .builder
            .build_alloca(self.list_type, "rtl_list")
            .map_err(llvm_err)?;
        self.builder
            .build_store(list_alloca, list_bv)
            .map_err(llvm_err)?;

        // Loop to populate list
        let current_fn = self
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();
        let entry_block = self.builder.get_insert_block().unwrap();
        let loop_block = self.context.append_basic_block(current_fn, "rtl_loop");
        let body_block = self.context.append_basic_block(current_fn, "rtl_body");
        let done_block = self.context.append_basic_block(current_fn, "rtl_done");
        self.builder
            .build_unconditional_branch(loop_block)
            .map_err(llvm_err)?;

        // Loop header: check if i < end_bound
        self.builder.position_at_end(loop_block);
        let i_phi = self
            .builder
            .build_phi(self.i64_ty(), "rtl_i")
            .map_err(llvm_err)?;
        let list_phi = self
            .builder
            .build_phi(self.list_type, "rtl_lphi")
            .map_err(llvm_err)?;
        i_phi.add_incoming(&[(&start_val, entry_block)]);
        list_phi.add_incoming(&[(&list_bv, entry_block)]);
        let done_cond = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                i_phi.as_basic_value().into_int_value(),
                end_bound,
                "rtl_done_cond",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_conditional_branch(done_cond, done_block, body_block)
            .map_err(llvm_err)?;

        // Loop body: push current value
        self.builder.position_at_end(body_block);
        let val_i = i_phi.as_basic_value().into_int_value();
        let fat = self.make_int_fat(val_i)?;
        let cur_list = list_phi.as_basic_value();
        let pushed = self.call_rt("action_list_push", &[cur_list.into(), fat.into()])?;
        let new_list = pushed.try_as_basic_value().basic().ok_or("rtl push fail")?;
        let next_i = self
            .builder
            .build_int_add(val_i, self.i64_ty().const_int(1, false), "rtl_next")
            .map_err(llvm_err)?;
        let body_end_block = self.builder.get_insert_block().unwrap();
        i_phi.add_incoming(&[(&next_i, body_end_block)]);
        list_phi.add_incoming(&[(&new_list, body_end_block)]);
        self.builder
            .build_unconditional_branch(loop_block)
            .map_err(llvm_err)?;

        self.builder.position_at_end(done_block);
        let final_list = list_phi.as_basic_value();
        self.builder
            .build_store(list_alloca, final_list)
            .map_err(llvm_err)?;
        Ok(TypedValue::List(list_alloca))
    }

    /// toList(lazy_or_set) - convert a LazyList or Set to a List
    pub(super) fn builtin_to_list(&mut self, expr: &Expr) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_expr(expr)?;
        match val {
            TypedValue::LazyList(_) => {
                let list_sv = self.convert_lazylist_to_list(&val)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "toList")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, list_sv)
                    .map_err(llvm_err)?;
                Ok(TypedValue::List(new_alloca))
            }
            TypedValue::Set(ptr) => {
                let list_val = self.load_list(ptr)?;
                let new_alloca = self
                    .builder
                    .build_alloca(self.list_type, "to_list_s")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(new_alloca, list_val)
                    .map_err(llvm_err)?;
                Ok(TypedValue::List(new_alloca))
            }
            TypedValue::List(_) => Ok(val),
            _ => Err("toList: argument must be a LazyList or Set".to_string()),
        }
    }

    /// toLazyList(list) - convert a List to a LazyList
    pub(super) fn builtin_to_lazy_list(&mut self, expr: &Expr) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_expr(expr)?;
        match val {
            TypedValue::List(ptr) => {
                // Load list, extract first element as head
                let list_sv = self.load_list(ptr)?;
                let data = self
                    .builder
                    .build_extract_value(list_sv, 0, "toll_data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let len = self
                    .builder
                    .build_extract_value(list_sv, 1, "toll_len")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Load first element (fat struct) from data[0]
                let first_fat_ptr = unsafe {
                    self.builder
                        .build_gep(
                            self.fat_return_type,
                            data,
                            &[self.i64_ty().const_int(0, false)],
                            "toll_gep",
                        )
                        .map_err(llvm_err)
                }?;
                let first_fat = self
                    .builder
                    .build_load(self.fat_return_type, first_fat_ptr, "toll_fat")
                    .map_err(llvm_err)?;
                let head_val = self
                    .builder
                    .build_extract_value(first_fat.into_struct_value(), 0, "toll_head")
                    .map_err(llvm_err)?
                    .into_int_value();

                // Store data pointer as i64 in state field so round-trip toList can recover all elements
                let data_as_i64 = self
                    .builder
                    .build_ptr_to_int(data, self.i64_ty(), "data_i64")
                    .map_err(llvm_err)?;

                // Create LazyList with head, no step fn, state = data_ptr, take_count = len
                let ll_alloca = self
                    .builder
                    .build_alloca(self.lazylist_type, "to_ll")
                    .map_err(llvm_err)?;
                let undef = self.lazylist_type.get_undef();
                let v0 = self
                    .builder
                    .build_insert_value(undef, head_val, 0, "ll_h")
                    .map_err(llvm_err)?;
                let v1 = self
                    .builder
                    .build_insert_value(v0, self.ptr_ty().const_null(), 1, "ll_fn")
                    .map_err(llvm_err)?;
                let v2 = self
                    .builder
                    .build_insert_value(v1, data_as_i64, 2, "ll_s")
                    .map_err(llvm_err)?;
                let v3 = self
                    .builder
                    .build_insert_value(v2, len, 3, "ll_tc")
                    .map_err(llvm_err)?;
                let v4 = self
                    .builder
                    .build_insert_value(v3, self.ptr_ty().const_null(), 4, "ll_map")
                    .map_err(llvm_err)?;
                let v5 = self
                    .builder
                    .build_insert_value(v4, self.ptr_ty().const_null(), 5, "ll_filt")
                    .map_err(llvm_err)?;
                self.builder.build_store(ll_alloca, v5).map_err(llvm_err)?;
                Ok(TypedValue::LazyList(ll_alloca))
            }
            TypedValue::LazyList(_) => Ok(val),
            _ => Err("toLazyList: argument must be a List".to_string()),
        }
    }

    /// lazyTake(n, lazy_list) - limit lazy list to first n elements (lazy: just updates take_count)
    pub(super) fn builtin_lazy_take(
        &mut self,
        n_expr: &Expr,
        lazy_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let n_val = self.compile_expr(n_expr)?;
        let n = match n_val {
            TypedValue::Int(v) => v,
            _ => return Err("lazyTake: first argument must be an Int".to_string()),
        };
        let lazy_val = self.compile_expr(lazy_expr)?;
        let lazy_ptr = match &lazy_val {
            TypedValue::LazyList(p) => *p,
            _ => return Err("lazyTake: second argument must be a LazyList".to_string()),
        };
        // Load the LazyList struct, copy it with updated take_count
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, lazy_ptr, "lt_ll")
            .map_err(llvm_err)?
            .into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_sv, 0, "lt_head")
            .map_err(llvm_err)?;
        let step_fn = self
            .builder
            .build_extract_value(ll_sv, 1, "lt_fn")
            .map_err(llvm_err)?;
        let state_val = self
            .builder
            .build_extract_value(ll_sv, 2, "lt_st")
            .map_err(llvm_err)?;
        let map_fn = self
            .builder
            .build_extract_value(ll_sv, 4, "lt_map")
            .map_err(llvm_err)?;
        let filter_fn = self
            .builder
            .build_extract_value(ll_sv, 5, "lt_filt")
            .map_err(llvm_err)?;

        let result_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "lt_result")
            .map_err(llvm_err)?;
        let undef = self.lazylist_type.get_undef();
        let v0 = self
            .builder
            .build_insert_value(undef, head_val, 0, "lt_h")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn, 1, "lt_f")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state_val, 2, "lt_s")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, n, 3, "lt_n")
            .map_err(llvm_err)?;
        let v4 = self
            .builder
            .build_insert_value(v3, map_fn, 4, "lt_map")
            .map_err(llvm_err)?;
        let v5 = self
            .builder
            .build_insert_value(v4, filter_fn, 5, "lt_filt")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, v5)
            .map_err(llvm_err)?;
        Ok(TypedValue::LazyList(result_alloca))
    }

    /// lazyDrop(n, lazy_list) - drop first n elements (truly lazy: advances state without materializing list)
    pub(super) fn builtin_lazy_drop(
        &mut self,
        n_expr: &Expr,
        lazy_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let n_val = self.compile_expr(n_expr)?;
        let n = match n_val {
            TypedValue::Int(v) => v,
            _ => return Err("lazyDrop: first argument must be an Int".to_string()),
        };
        let lazy_val = self.compile_expr(lazy_expr)?;
        let lazy_ptr = match &lazy_val {
            TypedValue::LazyList(p) => *p,
            _ => return Err("lazyDrop: second argument must be a LazyList".to_string()),
        };

        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, lazy_ptr, "ld_ll")
            .map_err(llvm_err)?
            .into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_sv, 0, "ld_head")
            .map_err(llvm_err)?
            .into_int_value();
        let step_fn = self
            .builder
            .build_extract_value(ll_sv, 1, "ld_fn")
            .map_err(llvm_err)?
            .into_pointer_value();
        let state_val = self
            .builder
            .build_extract_value(ll_sv, 2, "ld_st")
            .map_err(llvm_err)?
            .into_int_value();
        let take_count_val = self
            .builder
            .build_extract_value(ll_sv, 3, "ld_tc")
            .map_err(llvm_err)?
            .into_int_value();
        let map_fn = self
            .builder
            .build_extract_value(ll_sv, 4, "ld_map")
            .map_err(llvm_err)?
            .into_pointer_value();
        let filter_fn = self
            .builder
            .build_extract_value(ll_sv, 5, "ld_filt")
            .map_err(llvm_err)?
            .into_pointer_value();

        let zero = self.i64_ty().const_int(0, false);
        let one = self.i64_ty().const_int(1, false);
        let neg_one = self.i64_ty().const_int((-1_i64) as u64, true);

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;

        // Determine if list-backed (no step fn, state holds data ptr)
        let has_step = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                step_fn,
                self.ptr_ty().const_null(),
                "ld_has_step",
            )
            .map_err(llvm_err)?;
        let state_nz = self
            .builder
            .build_int_compare(IntPredicate::NE, state_val, zero, "ld_state_nz")
            .map_err(llvm_err)?;
        let not_has_step = self
            .builder
            .build_not(has_step, "ld_not_step")
            .map_err(llvm_err)?;
        let is_list_backed = self
            .builder
            .build_and(not_has_step, state_nz, "ld_is_lb")
            .map_err(llvm_err)?;

        // Check if n >= take_count (result is empty)
        let tc_is_inf = self
            .builder
            .build_int_compare(IntPredicate::EQ, take_count_val, neg_one, "ld_tc_inf")
            .map_err(llvm_err)?;
        let n_ge_tc = self
            .builder
            .build_int_compare(IntPredicate::SGE, n, take_count_val, "ld_n_ge_tc")
            .map_err(llvm_err)?;
        let not_inf = self
            .builder
            .build_not(tc_is_inf, "ld_not_inf")
            .map_err(llvm_err)?;
        let becomes_empty = self
            .builder
            .build_and(not_inf, n_ge_tc, "ld_empty")
            .map_err(llvm_err)?;

        // Branch: empty? → fast path; otherwise → drop path
        let empty_block = self.context.append_basic_block(current_fn, "ld_empty");
        let drop_block = self.context.append_basic_block(current_fn, "ld_drop");
        let merge_block = self.context.append_basic_block(current_fn, "ld_merge");
        let _ = self
            .builder
            .build_conditional_branch(becomes_empty, empty_block, drop_block);

        // Empty result: head=0, no step fn, state=0, tc=0, keep map/filter (won't matter)
        self.builder.position_at_end(empty_block);
        let e_result = self
            .builder
            .build_alloca(self.lazylist_type, "ld_e_result")
            .map_err(llvm_err)?;
        let e_undef = self.lazylist_type.get_undef();
        let e0 = self
            .builder
            .build_insert_value(e_undef, zero, 0, "e_h")
            .map_err(llvm_err)?;
        let e1 = self
            .builder
            .build_insert_value(e0, self.ptr_ty().const_null(), 1, "e_fn")
            .map_err(llvm_err)?;
        let e2 = self
            .builder
            .build_insert_value(e1, zero, 2, "e_st")
            .map_err(llvm_err)?;
        let e3 = self
            .builder
            .build_insert_value(e2, zero, 3, "e_tc")
            .map_err(llvm_err)?;
        let e4 = self
            .builder
            .build_insert_value(e3, self.ptr_ty().const_null(), 4, "e_map")
            .map_err(llvm_err)?;
        let e5 = self
            .builder
            .build_insert_value(e4, self.ptr_ty().const_null(), 5, "e_filt")
            .map_err(llvm_err)?;
        self.builder.build_store(e_result, e5).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Drop path: advance head/state by n
        self.builder.position_at_end(drop_block);

        // Branch on list-backed vs step-function
        let lb_drop_block = self.context.append_basic_block(current_fn, "ld_lb_drop");
        let step_drop_block = self.context.append_basic_block(current_fn, "ld_step_drop");
        let drop_merge_block = self.context.append_basic_block(current_fn, "ld_drop_merge");
        let _ =
            self.builder
                .build_conditional_branch(is_list_backed, lb_drop_block, step_drop_block);

        // List-backed drop: advance data ptr by n elements, load new head
        self.builder.position_at_end(lb_drop_block);
        let data_ptr = self
            .builder
            .build_int_to_ptr(state_val, self.ptr_ty(), "ld_dp")
            .map_err(llvm_err)?;
        let new_data_gep = unsafe {
            self.builder
                .build_gep(self.fat_return_type, data_ptr, &[n], "ld_ndp")
                .map_err(llvm_err)
        }?;
        let new_data_i64 = self
            .builder
            .build_ptr_to_int(new_data_gep, self.i64_ty(), "ld_ndp_i64")
            .map_err(llvm_err)?;
        let new_head_fat = self
            .builder
            .build_load(self.fat_return_type, new_data_gep, "ld_nh_fat")
            .map_err(llvm_err)?
            .into_struct_value();
        let new_head = self
            .builder
            .build_extract_value(new_head_fat, 0, "ld_nh")
            .map_err(llvm_err)?
            .into_int_value();
        let new_tc_lb = self
            .builder
            .build_int_sub(take_count_val, n, "ld_new_tc_lb")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(drop_merge_block);

        // Step-function drop: call step_fn n times to advance
        self.builder.position_at_end(step_drop_block);
        let i_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ld_i")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, zero).map_err(llvm_err)?;
        let cur_state_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ld_cs")
            .map_err(llvm_err)?;
        self.builder
            .build_store(cur_state_alloca, state_val)
            .map_err(llvm_err)?;
        let cur_head_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "ld_ch")
            .map_err(llvm_err)?;
        self.builder
            .build_store(cur_head_alloca, head_val)
            .map_err(llvm_err)?;

        let step_loop_hdr = self.context.append_basic_block(current_fn, "ld_step_hdr");
        let step_loop_body = self.context.append_basic_block(current_fn, "ld_step_body");
        let step_done = self.context.append_basic_block(current_fn, "ld_step_done");
        let _ = self.builder.build_unconditional_branch(step_loop_hdr);

        self.builder.position_at_end(step_loop_hdr);
        let i_val = self
            .builder
            .build_load(self.i64_ty(), i_alloca, "ld_i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let i_lt_n = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, n, "ld_i_lt_n")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(i_lt_n, step_loop_body, step_done);

        self.builder.position_at_end(step_loop_body);
        let cs = self
            .builder
            .build_load(self.i64_ty(), cur_state_alloca, "ld_cs_val")
            .map_err(llvm_err)?
            .into_int_value();
        let fn_type = self.fat_return_type.fn_type(&[self.i64_ty().into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, step_fn, &[cs.into()], "ld_step_call")
            .map_err(llvm_err)?;
        let step_fat = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("step call returned void")?;
        let step_fat_sv = step_fat.into_struct_value();
        let next_val = self
            .builder
            .build_extract_value(step_fat_sv, 0, "ld_next")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(cur_state_alloca, next_val)
            .map_err(llvm_err)?;
        self.builder
            .build_store(cur_head_alloca, next_val)
            .map_err(llvm_err)?;
        let new_i = self
            .builder
            .build_int_add(i_val, one, "ld_ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, new_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(step_loop_hdr);

        self.builder.position_at_end(step_done);
        let new_head_step = self
            .builder
            .build_load(self.i64_ty(), cur_head_alloca, "ld_nh_step")
            .map_err(llvm_err)?
            .into_int_value();
        let new_state = self
            .builder
            .build_load(self.i64_ty(), cur_state_alloca, "ld_ns")
            .map_err(llvm_err)?
            .into_int_value();
        let new_tc_step = self
            .builder
            .build_select(
                tc_is_inf,
                take_count_val,
                self.builder
                    .build_int_sub(take_count_val, n, "ld_tc_sub")
                    .map_err(llvm_err)?,
                "ld_new_tc_step",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(drop_merge_block);

        // Drop merge: phi for new head, new state, new take_count
        self.builder.position_at_end(drop_merge_block);
        let d_nh_phi = self
            .builder
            .build_phi(self.i64_ty(), "ld_nh_phi")
            .map_err(llvm_err)?;
        d_nh_phi.add_incoming(&[(&new_head, lb_drop_block), (&new_head_step, step_done)]);
        let d_ns_phi = self
            .builder
            .build_phi(self.i64_ty(), "ld_ns_phi")
            .map_err(llvm_err)?;
        d_ns_phi.add_incoming(&[(&new_data_i64, lb_drop_block), (&new_state, step_done)]);
        let d_ntc_phi = self
            .builder
            .build_phi(self.i64_ty(), "ld_ntc_phi")
            .map_err(llvm_err)?;
        d_ntc_phi.add_incoming(&[(&new_tc_lb, lb_drop_block), (&new_tc_step, step_done)]);

        let d_result = self
            .builder
            .build_alloca(self.lazylist_type, "ld_d_result")
            .map_err(llvm_err)?;
        let d_undef = self.lazylist_type.get_undef();
        let d0 = self
            .builder
            .build_insert_value(
                d_undef,
                d_nh_phi.as_basic_value().into_int_value(),
                0,
                "d_h",
            )
            .map_err(llvm_err)?;
        let d1 = self
            .builder
            .build_insert_value(d0, step_fn, 1, "d_fn")
            .map_err(llvm_err)?;
        let d2 = self
            .builder
            .build_insert_value(d1, d_ns_phi.as_basic_value().into_int_value(), 2, "d_st")
            .map_err(llvm_err)?;
        let d3 = self
            .builder
            .build_insert_value(d2, d_ntc_phi.as_basic_value().into_int_value(), 3, "d_tc")
            .map_err(llvm_err)?;
        let d4 = self
            .builder
            .build_insert_value(d3, map_fn, 4, "d_map")
            .map_err(llvm_err)?;
        let d5 = self
            .builder
            .build_insert_value(d4, filter_fn, 5, "d_filt")
            .map_err(llvm_err)?;
        self.builder.build_store(d_result, d5).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Final merge: phi for the result LazyList pointer
        self.builder.position_at_end(merge_block);
        let m_phi = self
            .builder
            .build_phi(self.ptr_ty(), "ld_m_phi")
            .map_err(llvm_err)?;
        m_phi.add_incoming(&[(&e_result, empty_block), (&d_result, drop_merge_block)]);
        let result_ptr = m_phi.as_basic_value().into_pointer_value();
        Ok(TypedValue::LazyList(result_ptr))
    }

    /// lazyMap(fn, lazy_list) - truly lazy: creates a wrapper step function composing map with the original step fn.
    /// Falls back to eager evaluation for list-backed lazy lists (from toLazyList).
    pub(super) fn builtin_lazy_map(
        &mut self,
        fn_expr: &Expr,
        lazy_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self.compile_expr(fn_expr)?;
        let (map_fn_ptr, _fn_type) = match fn_val {
            TypedValue::Fn(p, ft) => (p, ft),
            _ => return Err("lazyMap: first argument must be a function".to_string()),
        };
        let lazy_val = self.compile_expr(lazy_expr)?;
        match &lazy_val {
            TypedValue::LazyList(ll_ptr) => self.lazy_map_impl(map_fn_ptr, *ll_ptr),
            TypedValue::List(_) => {
                // Convert to lazy list first, then map lazily
                let ll_val = self.builtin_to_lazy_list(lazy_expr)?;
                match ll_val {
                    TypedValue::LazyList(ll_ptr) => self.lazy_map_impl(map_fn_ptr, ll_ptr),
                    _ => Err("lazyMap: toLazyList did not return LazyList".to_string()),
                }
            }
            _ => Err("lazyMap: second argument must be a LazyList or List".to_string()),
        }
    }

    /// lazy_map_impl: store map_fn in the LazyList for deferred application during toList()
    fn lazy_map_impl(
        &mut self,
        map_fn_ptr: inkwell::values::PointerValue<'ctx>,
        ll_ptr: inkwell::values::PointerValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, ll_ptr, "lm_ll")
            .map_err(llvm_err)?
            .into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_sv, 0, "lm_head")
            .map_err(llvm_err)?;
        let step_fn = self
            .builder
            .build_extract_value(ll_sv, 1, "lm_sf")
            .map_err(llvm_err)?;
        let state_val = self
            .builder
            .build_extract_value(ll_sv, 2, "lm_st")
            .map_err(llvm_err)?;
        let take_count = self
            .builder
            .build_extract_value(ll_sv, 3, "lm_tc")
            .map_err(llvm_err)?;
        let old_map_fn = self
            .builder
            .build_extract_value(ll_sv, 4, "lm_old_map")
            .map_err(llvm_err)?;
        let filter_fn = self
            .builder
            .build_extract_value(ll_sv, 5, "lm_filt")
            .map_err(llvm_err)?;

        // Compose with existing map_fn if present
        let has_old_map = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                old_map_fn.into_pointer_value(),
                self.ptr_ty().const_null(),
                "has_old_map",
            )
            .map_err(llvm_err)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let compose_block = self.context.append_basic_block(current_fn, "lm_compose");
        let no_compose_block = self.context.append_basic_block(current_fn, "lm_no_compose");
        let merge_block = self.context.append_basic_block(current_fn, "lm_merge");

        let _ = self
            .builder
            .build_conditional_branch(has_old_map, compose_block, no_compose_block);

        // Compose: new_fn(x) = map_fn_ptr(old_map_fn(x))
        self.builder.position_at_end(compose_block);
        let wrapper_name = format!("lm_compose_{}", self.wrapper_counter);
        self.wrapper_counter += 1;
        let fat_ty = self.string_type;
        let wrapper_fn = self.module.add_function(
            &wrapper_name,
            fat_ty.fn_type(&[self.i64_ty().into()], false),
            None,
        );
        let wrapper_entry = self.context.append_basic_block(wrapper_fn, "entry");
        let saved_block = self.builder.get_insert_block();

        let cap_ty = self
            .context
            .struct_type(&[self.ptr_ty().into(), self.ptr_ty().into()], false);
        let cap_global = self
            .module
            .add_global(cap_ty, None, &format!("{}_cap", wrapper_name));
        cap_global.set_initializer(&cap_ty.const_zero());
        let cap_ptr = cap_global.as_pointer_value();
        let c_gep0 = self
            .builder
            .build_struct_gep(cap_ty, cap_ptr, 0, "cg0")
            .map_err(llvm_err)?;
        self.builder
            .build_store(c_gep0, old_map_fn)
            .map_err(llvm_err)?;
        let c_gep1 = self
            .builder
            .build_struct_gep(cap_ty, cap_ptr, 1, "cg1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(c_gep1, map_fn_ptr)
            .map_err(llvm_err)?;

        self.builder.position_at_end(wrapper_entry);
        let w_state = wrapper_fn.get_first_param().unwrap().into_int_value();
        let cap_load = self
            .builder
            .build_load(cap_ty, cap_ptr, "cap_load")
            .map_err(llvm_err)?
            .into_struct_value();
        let w_old_fn = self
            .builder
            .build_extract_value(cap_load, 0, "w_old")
            .map_err(llvm_err)?
            .into_pointer_value();
        let w_new_fn = self
            .builder
            .build_extract_value(cap_load, 1, "w_new")
            .map_err(llvm_err)?
            .into_pointer_value();
        let map_fn_type = fat_ty.fn_type(&[self.i64_ty().into()], false);
        let old_call = self
            .builder
            .build_indirect_call(map_fn_type, w_old_fn, &[w_state.into()], "w_old_call")
            .map_err(llvm_err)?;
        let old_result = old_call
            .try_as_basic_value()
            .basic()
            .ok_or("old call failed")?;
        let old_val = if old_result.is_struct_value() {
            self.builder
                .build_extract_value(old_result.into_struct_value(), 0, "w_old_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            old_result.into_int_value()
        };
        let new_call = self
            .builder
            .build_indirect_call(map_fn_type, w_new_fn, &[old_val.into()], "w_new_call")
            .map_err(llvm_err)?;
        let new_result = new_call
            .try_as_basic_value()
            .basic()
            .ok_or("new call failed")?;
        self.builder
            .build_return(Some(&new_result))
            .map_err(llvm_err)?;

        self.builder.position_at_end(saved_block.unwrap());
        let composed_fn = wrapper_fn.as_global_value().as_pointer_value();
        let _ = self.builder.build_unconditional_branch(merge_block);

        self.builder.position_at_end(no_compose_block);
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Merge: pick the right map_fn
        self.builder.position_at_end(merge_block);
        let phi_map = self
            .builder
            .build_phi(self.ptr_ty(), "lm_phi_map")
            .map_err(llvm_err)?;
        phi_map.add_incoming(&[
            (&composed_fn, compose_block),
            (&map_fn_ptr, no_compose_block),
        ]);

        // Build result LazyList with updated map_fn, head unchanged (deferred mapping in toList)
        let result_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "lm_result")
            .map_err(llvm_err)?;
        let undef = self.lazylist_type.get_undef();
        let v0 = self
            .builder
            .build_insert_value(undef, head_val, 0, "lm_h")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn, 1, "lm_f")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state_val, 2, "lm_s")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, take_count, 3, "lm_t")
            .map_err(llvm_err)?;
        let v4 = self
            .builder
            .build_insert_value(
                v3,
                phi_map.as_basic_value().into_pointer_value(),
                4,
                "lm_map",
            )
            .map_err(llvm_err)?;
        let v5 = self
            .builder
            .build_insert_value(v4, filter_fn, 5, "lm_filt")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, v5)
            .map_err(llvm_err)?;
        Ok(TypedValue::LazyList(result_alloca))
    }

    /// lazyFilter(fn, lazy_list) - truly lazy: stores filter_fn in LazyList for deferred application during toList()
    pub(super) fn builtin_lazy_filter(
        &mut self,
        fn_expr: &Expr,
        lazy_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self.compile_expr(fn_expr)?;
        let (filter_fn_ptr, _) = match fn_val {
            TypedValue::Fn(p, _) => (p, fn_val),
            _ => return Err("lazyFilter: first argument must be a function".to_string()),
        };
        let lazy_val = self.compile_expr(lazy_expr)?;
        match &lazy_val {
            TypedValue::LazyList(ll_ptr) => self.lazy_filter_impl(filter_fn_ptr, *ll_ptr),
            TypedValue::List(_) => {
                let ll_val = self.builtin_to_lazy_list(lazy_expr)?;
                match ll_val {
                    TypedValue::LazyList(ll_ptr) => self.lazy_filter_impl(filter_fn_ptr, ll_ptr),
                    _ => Err("lazyFilter: toLazyList did not return LazyList".to_string()),
                }
            }
            _ => Err("lazyFilter: second argument must be a LazyList or List".to_string()),
        }
    }

    /// lazy_filter_impl: store filter_fn in the LazyList for deferred application during toList()
    fn lazy_filter_impl(
        &mut self,
        filter_fn_ptr: inkwell::values::PointerValue<'ctx>,
        ll_ptr: inkwell::values::PointerValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let ll_sv = self
            .builder
            .build_load(self.lazylist_type, ll_ptr, "lf_ll")
            .map_err(llvm_err)?
            .into_struct_value();
        let head_val = self
            .builder
            .build_extract_value(ll_sv, 0, "lf_head")
            .map_err(llvm_err)?;
        let step_fn = self
            .builder
            .build_extract_value(ll_sv, 1, "lf_sf")
            .map_err(llvm_err)?;
        let state_val = self
            .builder
            .build_extract_value(ll_sv, 2, "lf_st")
            .map_err(llvm_err)?;
        let take_count = self
            .builder
            .build_extract_value(ll_sv, 3, "lf_tc")
            .map_err(llvm_err)?;
        let map_fn = self
            .builder
            .build_extract_value(ll_sv, 4, "lf_map")
            .map_err(llvm_err)?;
        let old_filter_fn = self
            .builder
            .build_extract_value(ll_sv, 5, "lf_old_filt")
            .map_err(llvm_err)?;

        // Compose filters if there's already a filter_fn
        let has_old_filter = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                old_filter_fn.into_pointer_value(),
                self.ptr_ty().const_null(),
                "has_old_filt",
            )
            .map_err(llvm_err)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let compose_block = self.context.append_basic_block(current_fn, "lf_compose");
        let no_compose_block = self.context.append_basic_block(current_fn, "lf_no_compose");
        let merge_block = self.context.append_basic_block(current_fn, "lf_merge");

        let _ =
            self.builder
                .build_conditional_branch(has_old_filter, compose_block, no_compose_block);

        // Compose: new_filter(x) = old_filter(x) && new_filter(x)
        self.builder.position_at_end(compose_block);
        let wrapper_name = format!("lf_compose_{}", self.wrapper_counter);
        self.wrapper_counter += 1;
        let fat_ty = self.string_type;
        let wrapper_fn = self.module.add_function(
            &wrapper_name,
            fat_ty.fn_type(&[self.i64_ty().into()], false),
            None,
        );
        let wrapper_entry = self.context.append_basic_block(wrapper_fn, "entry");
        let saved_block = self.builder.get_insert_block();

        // Capture both filter functions via global
        let cap_ty = self
            .context
            .struct_type(&[self.ptr_ty().into(), self.ptr_ty().into()], false);
        let cap_global = self
            .module
            .add_global(cap_ty, None, &format!("{}_cap", wrapper_name));
        cap_global.set_initializer(&cap_ty.const_zero());
        let cap_ptr = cap_global.as_pointer_value();
        let c_gep0 = self
            .builder
            .build_struct_gep(cap_ty, cap_ptr, 0, "cg0")
            .map_err(llvm_err)?;
        self.builder
            .build_store(c_gep0, old_filter_fn)
            .map_err(llvm_err)?;
        let c_gep1 = self
            .builder
            .build_struct_gep(cap_ty, cap_ptr, 1, "cg1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(c_gep1, filter_fn_ptr)
            .map_err(llvm_err)?;

        self.builder.position_at_end(wrapper_entry);
        let w_state = wrapper_fn.get_first_param().unwrap().into_int_value();
        let cap_load = self
            .builder
            .build_load(cap_ty, cap_ptr, "cap_load")
            .map_err(llvm_err)?
            .into_struct_value();
        let w_old_fn = self
            .builder
            .build_extract_value(cap_load, 0, "w_old")
            .map_err(llvm_err)?
            .into_pointer_value();
        let w_new_fn = self
            .builder
            .build_extract_value(cap_load, 1, "w_new")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Call old_filter(state)
        let filt_fn_type = fat_ty.fn_type(&[self.i64_ty().into()], false);
        let old_call = self
            .builder
            .build_indirect_call(filt_fn_type, w_old_fn, &[w_state.into()], "w_old_call")
            .map_err(llvm_err)?;
        let old_result = old_call
            .try_as_basic_value()
            .basic()
            .ok_or("old filt call failed")?;
        let old_val = if old_result.is_struct_value() {
            self.builder
                .build_extract_value(old_result.into_struct_value(), 0, "w_old_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            old_result.into_int_value()
        };
        let old_true = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                old_val,
                self.i64_ty().const_int(0, false),
                "old_true",
            )
            .map_err(llvm_err)?;

        let then_block = self.context.append_basic_block(wrapper_fn, "then_call");
        let else_block = self.context.append_basic_block(wrapper_fn, "else_zero");
        let w_merge = self.context.append_basic_block(wrapper_fn, "w_merge");
        let _ = self
            .builder
            .build_conditional_branch(old_true, then_block, else_block);

        self.builder.position_at_end(then_block);
        let new_call = self
            .builder
            .build_indirect_call(filt_fn_type, w_new_fn, &[w_state.into()], "w_new_call")
            .map_err(llvm_err)?;
        let new_result = new_call
            .try_as_basic_value()
            .basic()
            .ok_or("new filt call failed")?;
        let new_val = if new_result.is_struct_value() {
            self.builder
                .build_extract_value(new_result.into_struct_value(), 0, "w_new_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            new_result.into_int_value()
        };
        let new_true = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                new_val,
                self.i64_ty().const_int(0, false),
                "new_true",
            )
            .map_err(llvm_err)?;
        let new_i64 = self
            .builder
            .build_int_z_extend(new_true, self.i64_ty(), "new_i64")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(w_merge);

        self.builder.position_at_end(else_block);
        let _ = self.builder.build_unconditional_branch(w_merge);

        self.builder.position_at_end(w_merge);
        let phi = self
            .builder
            .build_phi(self.i64_ty(), "filt_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[
            (&new_i64, then_block),
            (&self.i64_ty().const_int(0, false), else_block),
        ]);
        // Return as fat struct {i64, i8*}
        let undef_ret = fat_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef_ret, phi.as_basic_value().into_int_value(), 0, "fr_v")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, self.ptr_ty().const_null(), 1, "fr_p")
            .map_err(llvm_err)?;
        self.builder.build_return(Some(&r2)).map_err(llvm_err)?;

        self.builder.position_at_end(saved_block.unwrap());
        let composed_fn = wrapper_fn.as_global_value().as_pointer_value();
        let _ = self.builder.build_unconditional_branch(merge_block);

        // No composition needed
        self.builder.position_at_end(no_compose_block);
        let _ = self.builder.build_unconditional_branch(merge_block);

        // Merge: pick the right filter_fn
        self.builder.position_at_end(merge_block);
        let phi_filt = self
            .builder
            .build_phi(self.ptr_ty(), "lf_phi_filt")
            .map_err(llvm_err)?;
        phi_filt.add_incoming(&[
            (&composed_fn, compose_block),
            (&filter_fn_ptr, no_compose_block),
        ]);

        let result_alloca = self
            .builder
            .build_alloca(self.lazylist_type, "lf_result")
            .map_err(llvm_err)?;
        let undef = self.lazylist_type.get_undef();
        let v0 = self
            .builder
            .build_insert_value(undef, head_val, 0, "lf_h")
            .map_err(llvm_err)?;
        let v1 = self
            .builder
            .build_insert_value(v0, step_fn, 1, "lf_f")
            .map_err(llvm_err)?;
        let v2 = self
            .builder
            .build_insert_value(v1, state_val, 2, "lf_s")
            .map_err(llvm_err)?;
        let v3 = self
            .builder
            .build_insert_value(v2, take_count, 3, "lf_t")
            .map_err(llvm_err)?;
        let v4 = self
            .builder
            .build_insert_value(v3, map_fn, 4, "lf_map")
            .map_err(llvm_err)?;
        let v5 = self
            .builder
            .build_insert_value(
                v4,
                phi_filt.as_basic_value().into_pointer_value(),
                5,
                "lf_filt",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, v5)
            .map_err(llvm_err)?;
        Ok(TypedValue::LazyList(result_alloca))
    }

    /// lazyTakeWhile(fn, lazy_list) - take elements while predicate is true
    pub(super) fn builtin_lazy_take_while(
        &mut self,
        fn_expr: &Expr,
        lazy_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let fn_val = self.compile_expr(fn_expr)?;
        let (fn_ptr, _) = match fn_val {
            TypedValue::Fn(p, _) => (p, fn_val),
            _ => return Err("lazyTakeWhile: first argument must be a function".to_string()),
        };
        let lazy_val = self.compile_expr(lazy_expr)?;
        let lazy_ptr = self.ensure_list_ptr(&lazy_val, "ltw")?;
        let list = self.load_list(lazy_ptr)?;
        let len = self
            .builder
            .build_extract_value(list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let data = self
            .builder
            .build_extract_value(list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();

        let cc = self.call_rt("action_list_create", &[len.into()])?;
        let new_list = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "ltw_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_list)
            .map_err(llvm_err)?;

        let i64 = self.i64_ty();
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let i_alloca = self.builder.build_alloca(i64, "ltw_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_hdr = self.context.append_basic_block(current_fn, "ltw_hdr");
        let loop_bdy = self.context.append_basic_block(current_fn, "ltw_bdy");
        let loop_ins = self.context.append_basic_block(current_fn, "ltw_ins");
        let loop_ext = self.context.append_basic_block(current_fn, "ltw_ext");

        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_hdr);
        let i = self
            .builder
            .build_load(i64, i_alloca, "ltw_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i, len, "ltw_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_bdy, loop_ext);

        self.builder.position_at_end(loop_bdy);
        let src_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, data, &[i], "ltw_sp")
                .map_err(llvm_err)
        }?;
        let elem = self
            .builder
            .build_load(self.string_type, src_ptr, "ltw_el")
            .map_err(llvm_err)?
            .into_struct_value();
        let tag = self
            .builder
            .build_extract_value(elem, 0, "ltw_tag")
            .map_err(llvm_err)?
            .into_int_value();

        let fat_ty = self.string_type;
        let lam_fn_type = fat_ty.fn_type(&[i64.into()], false);
        let cc = self
            .builder
            .build_indirect_call(lam_fn_type, fn_ptr, &[tag.into()], "ltw_call")
            .map_err(llvm_err)?;
        let pred_bv = cc.try_as_basic_value().basic().ok_or("ltw call failed")?;
        let pred_tag = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let keep = self
            .builder
            .build_int_compare(IntPredicate::NE, pred_tag, i64.const_int(0, false), "keep")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(keep, loop_ins, loop_ext);

        self.builder.position_at_end(loop_ins);
        let cur = self.load_list(result_alloca)?;
        let pcc = self.call_rt("action_list_push", &[cur.into(), elem.into()])?;
        let nl = pcc.try_as_basic_value().basic().ok_or("list_push failed")?;
        self.builder
            .build_store(result_alloca, nl)
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(i, i64.const_int(1, false), "ltw_ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_ext);
        Ok(TypedValue::List(result_alloca))
    }

    /// lazyHead(lazy_list) - return first element as Some, or None if empty
    pub(super) fn builtin_lazy_head(
        &mut self,
        lazy_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let lazy_val = self.compile_expr(lazy_expr)?;
        // If LazyList, extract head directly. If List, use first element.
        let (head_val, is_empty) = match &lazy_val {
            TypedValue::LazyList(ptr) => {
                let ll_sv = self
                    .builder
                    .build_load(self.lazylist_type, *ptr, "head_ll")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let h = self
                    .builder
                    .build_extract_value(ll_sv, 0, "head_h")
                    .map_err(llvm_err)?;
                // Check take_count (field 3): 0 = empty, != 0 = has elements
                let take_count = self
                    .builder
                    .build_extract_value(ll_sv, 3, "head_tc")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_empty = self
                    .builder
                    .build_int_compare(
                        IntPredicate::EQ,
                        take_count,
                        self.i64_ty().const_int(0, false),
                        "ll_is_empty",
                    )
                    .map_err(llvm_err)?;
                (h, is_empty)
            }
            TypedValue::List(ptr) => {
                let list = self.load_list(*ptr)?;
                let len = self
                    .builder
                    .build_extract_value(list, 1, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let data = self
                    .builder
                    .build_extract_value(list, 0, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let zero = self.i64_ty().const_int(0, false);
                let is_empty_cond = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "is_empty")
                    .map_err(llvm_err)?;
                // Load first element's fat struct
                let first_ptr = unsafe {
                    self.builder
                        .build_gep(self.fat_return_type, data, &[zero], "head_gep")
                        .map_err(llvm_err)
                }?;
                let first_fat = self
                    .builder
                    .build_load(self.fat_return_type, first_ptr, "head_fat")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let h = self
                    .builder
                    .build_extract_value(first_fat, 0, "head_h")
                    .map_err(llvm_err)?;
                (h, is_empty_cond)
            }
            _ => return Err("lazyHead: argument must be a LazyList or List".to_string()),
        };

        let i64 = self.i64_ty();

        // Create nullable {i1, i64} for nullable Int
        let nullable_ty = self.get_nullable_type(i64.into(), "Nullable<Int>");
        let null_bt: BasicTypeEnum = nullable_ty.into();

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;

        let result_alloca = self
            .builder
            .build_alloca(nullable_ty, "lh_result")
            .map_err(llvm_err)?;

        let merge_block = self.context.append_basic_block(current_fn, "lh_merge");
        let some_block = self.context.append_basic_block(current_fn, "lh_some");
        let none_block = self.context.append_basic_block(current_fn, "lh_none");

        let _ = self
            .builder
            .build_conditional_branch(is_empty, none_block, some_block);

        // Some branch: head_val contains the i64 value
        self.builder.position_at_end(some_block);
        let head_i64 = head_val.into_int_value();

        // Build nullable {flag=0, value} — inline, no heap allocation
        let undef = nullable_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(
                undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "lh_some_flag",
            )
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, head_i64, 1, "lh_some_val")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, r2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        // None branch: nullable {flag=1, undef}
        self.builder.position_at_end(none_block);
        let undef2 = nullable_ty.get_undef();
        let n1 = self
            .builder
            .build_insert_value(
                undef2,
                self.null_flag_ty().const_int(1, false),
                0,
                "lh_none_flag",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, n1)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_block);

        self.builder.position_at_end(merge_block);
        Ok(TypedValue::Nullable(result_alloca, null_bt))
    }

    /// lazy.zip(lazy1, lazy2) - zip two lazy lists eagerly, return as List
    pub(super) fn builtin_lazy_zip(
        &mut self,
        lazy1_expr: &Expr,
        lazy2_expr: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        let v1 = self.compile_expr(lazy1_expr)?;
        let v2 = self.compile_expr(lazy2_expr)?;
        let p1 = self.ensure_list_ptr(&v1, "lz1")?;
        let p2 = self.ensure_list_ptr(&v2, "lz2")?;
        let l1 = self.load_list(p1)?;
        let l2 = self.load_list(p2)?;
        let len1 = self
            .builder
            .build_extract_value(l1, 1, "lz_len1")
            .map_err(llvm_err)?
            .into_int_value();
        let len2 = self
            .builder
            .build_extract_value(l2, 1, "lz_len2")
            .map_err(llvm_err)?
            .into_int_value();
        let d1 = self
            .builder
            .build_extract_value(l1, 0, "lz_d1")
            .map_err(llvm_err)?
            .into_pointer_value();
        let d2 = self
            .builder
            .build_extract_value(l2, 0, "lz_d2")
            .map_err(llvm_err)?
            .into_pointer_value();

        let i64 = self.i64_ty();
        let is_len1_lt_len2 = self
            .builder
            .build_int_compare(IntPredicate::SLT, len1, len2, "is_len1_lt_len2")
            .map_err(llvm_err)?;
        let min_len = self
            .builder
            .build_select(is_len1_lt_len2, len1, len2, "lz_min")
            .map_err(llvm_err)?
            .into_int_value();

        let cc = self.call_rt("action_list_create", &[min_len.into()])?;
        let new_list = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "lz_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_list)
            .map_err(llvm_err)?;

        // Zip elements as tuple-like: store (tag1, tag2) as two sequential entries
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let i_alloca = self.builder.build_alloca(i64, "lz_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_hdr = self.context.append_basic_block(current_fn, "lz_hdr");
        let loop_bdy = self.context.append_basic_block(current_fn, "lz_bdy");
        let loop_ext = self.context.append_basic_block(current_fn, "lz_ext");

        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_hdr);
        let i = self
            .builder
            .build_load(i64, i_alloca, "lz_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i, min_len, "lz_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_bdy, loop_ext);

        self.builder.position_at_end(loop_bdy);
        let sp1 = unsafe {
            self.builder
                .build_gep(self.string_type, d1, &[i], "lz_sp1")
                .map_err(llvm_err)
        }?;
        let e1 = self
            .builder
            .build_load(self.string_type, sp1, "lz_e1")
            .map_err(llvm_err)?;
        let sp2 = unsafe {
            self.builder
                .build_gep(self.string_type, d2, &[i], "lz_sp2")
                .map_err(llvm_err)
        }?;
        let e2 = self
            .builder
            .build_load(self.string_type, sp2, "lz_e2")
            .map_err(llvm_err)?;

        // Push both as separate elements (pair is two sequential entries)
        let cur = self.load_list(result_alloca)?;
        let cc = self.call_rt("action_list_push", &[cur.into(), e1.into()])?;
        let nl = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_push e1 failed")?;
        self.builder
            .build_store(result_alloca, nl)
            .map_err(llvm_err)?;
        let cur2 = self.load_list(result_alloca)?;
        let cc2 = self.call_rt("action_list_push", &[cur2.into(), e2.into()])?;
        let nl2 = cc2
            .try_as_basic_value()
            .basic()
            .ok_or("list_push e2 failed")?;
        self.builder
            .build_store(result_alloca, nl2)
            .map_err(llvm_err)?;

        let ni = self
            .builder
            .build_int_add(i, i64.const_int(1, false), "lz_ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_ext);
        Ok(TypedValue::List(result_alloca))
    }

    // ---- Coroutine builtins ----

    /// launch { body } — start a coroutine on a real pthread (default scheduler).
    /// launch(io) { body } — start with I/O scheduler.
    /// launch(cpu) { body } — start with CPU scheduler.
    /// Task struct: {pthread: i64, done: i64, cancelled: i64, result_list: {ptr, i64, i64}}
    pub(super) fn builtin_launch(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // Parse optional scheduler argument
        let scheduler = if !args.is_empty() {
            match &args[0] {
                Expr::Ident(s) if s == "io" => 1i64,
                Expr::Ident(s) if s == "cpu" => 2i64,
                _ => return Err("launch scheduler must be 'io' or 'cpu'".to_string()),
            }
        } else {
            0i64 // default scheduler
        };
        let body = trailing
            .as_ref()
            .ok_or("launch requires a trailing lambda body")?;
        let body_expr = match body.as_ref() {
            Expr::Lambda { params, body, .. } if params.is_empty() => body.as_ref(),
            _ => return Err("launch expects a block body: launch { ... }".to_string()),
        };

        // 1. Heap-allocate Task struct (so thread can safely write to it after main returns)
        // Compute task struct size via GEP trick
        let task_ty_ptr = self.context.ptr_type(Default::default());
        let null_task_ptr = task_ty_ptr.const_null();
        let task_size_ptr = unsafe {
            self.builder
                .build_gep(
                    self.task_type,
                    null_task_ptr,
                    &[self.i64_ty().const_int(1, false)],
                    "task_size_ptr",
                )
                .map_err(llvm_err)
        }?;
        let task_size = self
            .builder
            .build_ptr_to_int(task_size_ptr, self.i64_ty(), "task_size")
            .map_err(llvm_err)?;
        let malloc_fn = self
            .module
            .get_function("malloc")
            .ok_or("malloc not found")?;
        let task_heap = self
            .builder
            .build_call(malloc_fn, &[task_size.into()], "task_heap")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let task_undef = self.task_type.get_undef();
        let pthread_zero = self.i64_ty().const_int(0, false);
        let done_zero = self.i64_ty().const_int(0, false);
        let cancelled_zero = self.i64_ty().const_int(0, false);
        let empty_list = self.list_type.get_undef();
        let empty_list_ptr = self.ptr_ty().const_null();
        let empty_list_len = self.i64_ty().const_int(0, false);
        let empty_list_cap = self.i64_ty().const_int(0, false);
        let el0 = self
            .builder
            .build_insert_value(empty_list, empty_list_ptr, 0, "el0")
            .map_err(llvm_err)?;
        let el1 = self
            .builder
            .build_insert_value(el0, empty_list_len, 1, "el1")
            .map_err(llvm_err)?;
        let el2 = self
            .builder
            .build_insert_value(el1, empty_list_cap, 2, "el2")
            .map_err(llvm_err)?;
        let t0 = self
            .builder
            .build_insert_value(task_undef, pthread_zero, 0, "t_pt")
            .map_err(llvm_err)?;
        let t1 = self
            .builder
            .build_insert_value(t0, done_zero, 1, "t_done")
            .map_err(llvm_err)?;
        let t2 = self
            .builder
            .build_insert_value(t1, cancelled_zero, 2, "t_canc")
            .map_err(llvm_err)?;
        let sched_val = self.i64_ty().const_int(scheduler as u64, false);
        let t3 = self
            .builder
            .build_insert_value(t2, sched_val, 3, "t_sched")
            .map_err(llvm_err)?;
        let t4 = self
            .builder
            .build_insert_value(t3, el2, 4, "t_list")
            .map_err(llvm_err)?;
        self.builder.build_store(task_heap, t4).map_err(llvm_err)?;

        // 2. Compile body into a thread function that creates its own result list
        self.lambda_count += 1;
        let task_name = format!(".task_body_{}", self.lambda_count);
        let fn_type = self.ptr_ty().fn_type(&[self.ptr_ty().into()], false);
        let task_fn = self.module.add_function(&task_name, fn_type, None);
        let entry = self.context.append_basic_block(task_fn, "entry");

        let saved_pos = self.builder.get_insert_block();
        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::new();

        self.builder.position_at_end(entry);
        let task_ptr_param = task_fn.get_first_param().unwrap().into_pointer_value();

        // Compile the body expression
        let result = self.compile_expr(body_expr)?;

        // Create a fresh list INSIDE the thread (avoids cross-thread data issues)
        let cap = self.i64_ty().const_int(1, false);
        let cc = self.call_rt("action_list_create", &[cap.into()])?;
        let list_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let rl_alloca = self
            .builder
            .build_alloca(self.list_type, "rl_a")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rl_alloca, list_bv)
            .map_err(llvm_err)?;
        self.push_to_collector(rl_alloca, &result)?;

        // Write done=1 and the new list back to the task struct
        let updated_list = self
            .builder
            .build_load(self.list_type, rl_alloca, "ul")
            .map_err(llvm_err)?;
        let task_ptr_cast = self
            .builder
            .build_pointer_cast(
                task_ptr_param,
                self.context.ptr_type(Default::default()),
                "task_cast",
            )
            .map_err(llvm_err)?;
        let loaded_task = self
            .builder
            .build_load(self.task_type, task_ptr_cast, "ltask")
            .map_err(llvm_err)?
            .into_struct_value();
        let done_one = self.i64_ty().const_int(1, false);
        let cancelled_val = self
            .builder
            .build_extract_value(loaded_task, 2, "cv")
            .map_err(llvm_err)?;
        let pt_val = self
            .builder
            .build_extract_value(loaded_task, 0, "pv")
            .map_err(llvm_err)?;
        let sched_val = self
            .builder
            .build_extract_value(loaded_task, 3, "sv")
            .map_err(llvm_err)?;
        let undef2 = self.task_type.get_undef();
        let u0 = self
            .builder
            .build_insert_value(undef2, pt_val, 0, "u_pt")
            .map_err(llvm_err)?;
        let u1 = self
            .builder
            .build_insert_value(u0, done_one, 1, "u_done")
            .map_err(llvm_err)?;
        let u2 = self
            .builder
            .build_insert_value(u1, cancelled_val, 2, "u_canc")
            .map_err(llvm_err)?;
        let u3 = self
            .builder
            .build_insert_value(u2, sched_val, 3, "u_sched")
            .map_err(llvm_err)?;
        let u4 = self
            .builder
            .build_insert_value(u3, updated_list, 4, "u_list")
            .map_err(llvm_err)?;
        self.builder
            .build_store(task_ptr_cast, u4)
            .map_err(llvm_err)?;

        // Return from thread function
        let current_block = self.builder.get_insert_block().unwrap();
        if current_block.get_terminator().is_none() {
            let null_ret = self.ptr_ty().const_null();
            let _ = self.builder.build_return(Some(&null_ret));
        }

        std::mem::swap(&mut self.scope, &mut saved_scope);
        if let Some(pos) = saved_pos {
            self.builder.position_at_end(pos);
        }

        // 3. Call pthread_create
        let pthread_create_fn = self
            .module
            .get_function("action_thread_create")
            .ok_or("action_thread_create not found")?;
        let pthread_field_ptr = self
            .builder
            .build_struct_gep(self.task_type, task_heap, 0, "pt_field")
            .map_err(llvm_err)?;
        let fn_as_ptr = task_fn.as_global_value().as_pointer_value();
        let _ = self
            .builder
            .build_call(
                pthread_create_fn,
                &[
                    pthread_field_ptr.into(),
                    self.ptr_ty().const_null().into(),
                    fn_as_ptr.into(),
                    task_heap.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;

        // 5. If inside coroutineScope, track this task for later join
        if let Some(collector_alloca) = self.coroutine_collector {
            // Store task_heap pointer as i64 in a fat struct {ptr_as_i64, null}
            let task_as_i64 = self
                .builder
                .build_ptr_to_int(task_heap, self.i64_ty(), "task_i64")
                .map_err(llvm_err)?;
            let task_fat = self.make_int_fat(task_as_i64)?;
            let cl = self.load_list(collector_alloca)?;
            let cc = self.call_rt("action_list_push", &[cl.into(), task_fat.into()])?;
            let nl = cc.try_as_basic_value().basic().ok_or("push failed")?;
            self.builder
                .build_store(collector_alloca, nl)
                .map_err(llvm_err)?;
        }

        Ok(TypedValue::Task(task_heap))
    }

    /// coroutineScope { body } — structured concurrency scope with real pthread join.
    pub(super) fn builtin_coroutine_scope(
        &mut self,
        _args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let body = trailing
            .as_ref()
            .ok_or("coroutineScope requires a trailing lambda body")?;
        let body_expr = match body.as_ref() {
            Expr::Lambda { params, body, .. } if params.is_empty() => body.as_ref(),
            _ => {
                return Err(
                    "coroutineScope expects a block body: coroutineScope { ... }".to_string(),
                )
            }
        };

        // Create collector list for task pointers
        let cap = self.i64_ty().const_int(4, false);
        let cc = self.call_rt("action_list_create", &[cap.into()])?;
        let list_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let collector_alloca = self
            .builder
            .build_alloca(self.list_type, "coro_collector")
            .map_err(llvm_err)?;
        self.builder
            .build_store(collector_alloca, list_bv)
            .map_err(llvm_err)?;

        // Save previous collector and set new one
        let prev_collector = self.coroutine_collector;
        self.coroutine_collector = Some(collector_alloca);

        // Compile the body (launch calls inside will spawn threads and push task pointers to collector)
        self.compile_expr(body_expr)?;

        // Restore previous collector
        self.coroutine_collector = prev_collector;

        // Join all tasks and collect results
        let collector_list = self.load_list(collector_alloca)?;
        let task_count = self
            .builder
            .build_extract_value(collector_list, 1, "tc")
            .map_err(llvm_err)?
            .into_int_value();

        // Create result list
        let result_cap = self.i64_ty().const_int(4, false);
        let rcc = self.call_rt("action_list_create", &[result_cap.into()])?;
        let result_list_bv = rcc
            .try_as_basic_value()
            .basic()
            .ok_or("result list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "coro_results")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, result_list_bv)
            .map_err(llvm_err)?;

        // Loop: for each task in collector, join and collect result
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let i_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "cs_i")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, self.i64_ty().const_int(0, false))
            .map_err(llvm_err)?;
        // Allocate cancel-loop index alloca here (dominates all cancel blocks)
        let cj_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "cs_cj")
            .map_err(llvm_err)?;

        let loop_hdr = self.context.append_basic_block(current_fn, "cs_hdr");
        let loop_body = self.context.append_basic_block(current_fn, "cs_body");
        let loop_exit = self.context.append_basic_block(current_fn, "cs_exit");

        let _ = self.builder.build_unconditional_branch(loop_hdr);

        self.builder.position_at_end(loop_hdr);
        let i_val = self
            .builder
            .build_load(self.i64_ty(), i_alloca, "cs_iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, task_count, "cs_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        // Create blocks for fast-fail handling
        let cancel_init = self
            .context
            .append_basic_block(current_fn, "cs_cancel_init");
        let cancel_loop_hdr = self.context.append_basic_block(current_fn, "cs_cancel_hdr");
        let cancel_loop_body = self
            .context
            .append_basic_block(current_fn, "cs_cancel_body");
        let cancel_exit = self
            .context
            .append_basic_block(current_fn, "cs_cancel_exit");

        self.builder.position_at_end(loop_body);

        // Load task fat struct from collector[i] via action_list_get (tree-aware)
        let cs_get_cc = self.call_rt("action_list_get", &[collector_list.into(), i_val.into()])?;
        let elem_fat = cs_get_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_get failed")?
            .into_struct_value();
        let task_i64 = self
            .builder
            .build_extract_value(elem_fat, 0, "cs_ti64")
            .map_err(llvm_err)?
            .into_int_value();
        let task_ptr = self
            .builder
            .build_int_to_ptr(task_i64, self.context.ptr_type(Default::default()), "cs_tp")
            .map_err(llvm_err)?;

        let task_sv = self
            .builder
            .build_load(self.task_type, task_ptr, "cs_task")
            .map_err(llvm_err)?
            .into_struct_value();
        let pthread_val = self
            .builder
            .build_extract_value(task_sv, 0, "cs_pt")
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

        let task_sv2 = self
            .builder
            .build_load(self.task_type, task_ptr, "cs_task2")
            .map_err(llvm_err)?
            .into_struct_value();
        let result_list_sv = self
            .builder
            .build_extract_value(task_sv2, 4, "cs_rl")
            .map_err(llvm_err)?
            .into_struct_value();

        let rl_alloca = self
            .builder
            .build_alloca(self.list_type, "cs_rla")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rl_alloca, result_list_sv)
            .map_err(llvm_err)?;
        let rl_val = self.load_list(rl_alloca)?;
        let zero = self.i64_ty().const_int(0, false);
        let cc = self.call_rt("action_list_get", &[rl_val.into(), zero.into()])?;
        let fat = cc
            .try_as_basic_value()
            .basic()
            .ok_or("get failed")?
            .into_struct_value();

        // Fast-fail check: tag==1 && data_ptr!=null means Err variant
        let fat_tag = self
            .builder
            .build_extract_value(fat, 0, "ff_tag")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_data = self
            .builder
            .build_extract_value(fat, 1, "ff_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let is_err_tag = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                fat_tag,
                self.i64_ty().const_int(1, false),
                "isErr",
            )
            .map_err(llvm_err)?;
        let data_nonnull = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                fat_data,
                self.ptr_ty().const_null(),
                "data_ok",
            )
            .map_err(llvm_err)?;
        let is_error = self
            .builder
            .build_and(is_err_tag, data_nonnull, "is_error")
            .map_err(llvm_err)?;
        let add_ok_bb = self.context.append_basic_block(current_fn, "cs_add_ok");
        let _ = self
            .builder
            .build_conditional_branch(is_error, cancel_init, add_ok_bb);

        // Add OK result to result list
        self.builder.position_at_end(add_ok_bb);
        let cur_results = self.load_list(result_alloca)?;
        let cc2 = self.call_rt("action_list_push", &[cur_results.into(), fat.into()])?;
        let new_results = cc2.try_as_basic_value().basic().ok_or("push2 failed")?;
        self.builder
            .build_store(result_alloca, new_results)
            .map_err(llvm_err)?;
        let next_i = self
            .builder
            .build_int_add(i_val, self.i64_ty().const_int(1, false), "cs_ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);

        // Cancel init: compute start index (i+1, skip already-joined task)
        self.builder.position_at_end(cancel_init);
        let cancel_start_i = self
            .builder
            .build_int_add(i_val, self.i64_ty().const_int(1, false), "cs_csi")
            .map_err(llvm_err)?;
        self.builder
            .build_store(cj_alloca, cancel_start_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(cancel_loop_hdr);

        // Cancel loop header
        self.builder.position_at_end(cancel_loop_hdr);
        let cj_val = self
            .builder
            .build_load(self.i64_ty(), cj_alloca, "cs_cjv")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, cj_val, task_count, "cs_ccond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cc_cond, cancel_loop_body, cancel_exit);

        // Cancel loop body: cancel one task
        self.builder.position_at_end(cancel_loop_body);
        let c_get_cc = self.call_rt("action_list_get", &[collector_list.into(), cj_val.into()])?;
        let c_elem_fat = c_get_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_get failed")?
            .into_struct_value();
        let c_task_i64 = self
            .builder
            .build_extract_value(c_elem_fat, 0, "cs_cti64")
            .map_err(llvm_err)?
            .into_int_value();
        let c_task_ptr = self
            .builder
            .build_int_to_ptr(
                c_task_i64,
                self.context.ptr_type(Default::default()),
                "cs_ctp",
            )
            .map_err(llvm_err)?;
        let c_task_sv = self
            .builder
            .build_load(self.task_type, c_task_ptr, "cs_ctsk")
            .map_err(llvm_err)?
            .into_struct_value();
        let c_pt_val = self
            .builder
            .build_extract_value(c_task_sv, 0, "cs_cpt")
            .map_err(llvm_err)?
            .into_int_value();
        let pthread_cancel_fn = self
            .module
            .get_function("action_thread_cancel")
            .ok_or("action_thread_cancel not found")?;
        let _ = self
            .builder
            .build_call(pthread_cancel_fn, &[c_pt_val.into()], "")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                pthread_join_fn,
                &[c_pt_val.into(), self.ptr_ty().const_null().into()],
                "",
            )
            .map_err(llvm_err)?;
        let c_next = self
            .builder
            .build_int_add(cj_val, self.i64_ty().const_int(1, false), "cs_cn")
            .map_err(llvm_err)?;
        self.builder
            .build_store(cj_alloca, c_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(cancel_loop_hdr);

        // After cancelling all remaining, push the error to result list and exit
        self.builder.position_at_end(cancel_exit);
        let err_results = self.load_list(result_alloca)?;
        let ecc = self.call_rt("action_list_push", &[err_results.into(), fat.into()])?;
        let enew = ecc.try_as_basic_value().basic().ok_or("err push failed")?;
        self.builder
            .build_store(result_alloca, enew)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_exit);

        self.builder.position_at_end(loop_exit);
        Ok(TypedValue::List(result_alloca))
    }

    /// delay(ms) — suspend coroutine for ms milliseconds using usleep.
    pub(super) fn builtin_delay(&mut self, args: &[Expr]) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err("delay expects 1 argument (ms)".to_string());
        }
        let ms_val = self.compile_expr(&args[0])?;
        let ms = match ms_val {
            TypedValue::Int(v) => v,
            _ => return Err("delay: argument must be an Int (milliseconds)".to_string()),
        };
        // usleep takes microseconds: ms * 1000
        let thousand = self.i64_ty().const_int(1000, false);
        let us = self
            .builder
            .build_int_mul(ms, thousand, "delay_us")
            .map_err(llvm_err)?;
        // Truncate to i32 for usleep
        let us_i32 = self
            .builder
            .build_int_truncate(us, self.i32_ty(), "delay_us32")
            .map_err(llvm_err)?;
        let usleep_fn = self
            .module
            .get_function("action_sleep_us")
            .ok_or("action_sleep_us not found")?;
        let _ = self
            .builder
            .build_call(usleep_fn, &[us_i32.into()], "")
            .map_err(llvm_err)?;
        Ok(TypedValue::Unit)
    }

    /// Push a TypedValue to the collector list (used by launch inside coroutineScope).
    pub(super) fn push_to_collector(
        &mut self,
        collector_alloca: inkwell::values::PointerValue<'ctx>,
        value: &TypedValue<'ctx>,
    ) -> Result<(), String> {
        let elem_fat = self.to_fat_struct(value)?;
        let list_val = self.load_list(collector_alloca)?;
        let cc = self.call_rt("action_list_push", &[list_val.into(), elem_fat.into()])?;
        let new_list = cc.try_as_basic_value().basic().ok_or("list_push failed")?;
        self.builder
            .build_store(collector_alloca, new_list)
            .map_err(llvm_err)?;
        Ok(())
    }

    /// withTimeout(ms, { body }) — timeout-controlled coroutine execution using pthread.
    /// Spawns a real pthread for the body, polls until done or timeout.
    /// Returns Ok(result) on success, Err(Timeout) on timeout.
    pub(super) fn builtin_with_timeout(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err(
                "withTimeout expects 2 arguments: timeout(ms) and a trailing lambda".to_string(),
            );
        }
        let timeout_ms_val = self.compile_expr(&args[0])?;
        let timeout_ms = match &timeout_ms_val {
            TypedValue::Int(v) => *v,
            _ => return Err("withTimeout: first argument must be Int (milliseconds)".to_string()),
        };
        let body = trailing
            .as_ref()
            .ok_or("withTimeout requires a trailing lambda body")?;
        let body_expr = match body.as_ref() {
            Expr::Lambda { params, body, .. } if params.is_empty() => body.as_ref().clone(),
            _ => {
                return Err("withTimeout expects a block body: withTimeout(ms) { ... }".to_string())
            }
        };

        // 1. Heap-allocate Task struct for the thread to write results into
        let task_ty_ptr = self.context.ptr_type(Default::default());
        let null_task_ptr = task_ty_ptr.const_null();
        let task_size_ptr = unsafe {
            self.builder
                .build_gep(
                    self.task_type,
                    null_task_ptr,
                    &[self.i64_ty().const_int(1, false)],
                    "wtsz",
                )
                .map_err(llvm_err)
        }?;
        let task_size = self
            .builder
            .build_ptr_to_int(task_size_ptr, self.i64_ty(), "wtsz_i64")
            .map_err(llvm_err)?;
        let malloc_fn = self
            .module
            .get_function("malloc")
            .ok_or("malloc not found")?;
        let task_heap = self
            .builder
            .build_call(malloc_fn, &[task_size.into()], "wt_task")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Initialize task struct with zeroes
        let task_undef = self.task_type.get_undef();
        let pthread_zero = self.i64_ty().const_int(0, false);
        let done_zero = self.i64_ty().const_int(0, false);
        let cancelled_zero = self.i64_ty().const_int(0, false);
        let empty_list = self.list_type.get_undef();
        let empty_list_ptr = self.ptr_ty().const_null();
        let empty_list_len = self.i64_ty().const_int(0, false);
        let empty_list_cap = self.i64_ty().const_int(0, false);
        let el0 = self
            .builder
            .build_insert_value(empty_list, empty_list_ptr, 0, "el0")
            .map_err(llvm_err)?;
        let el1 = self
            .builder
            .build_insert_value(el0, empty_list_len, 1, "el1")
            .map_err(llvm_err)?;
        let el2 = self
            .builder
            .build_insert_value(el1, empty_list_cap, 2, "el2")
            .map_err(llvm_err)?;
        let t0 = self
            .builder
            .build_insert_value(task_undef, pthread_zero, 0, "t0")
            .map_err(llvm_err)?;
        let t1 = self
            .builder
            .build_insert_value(t0, done_zero, 1, "t1")
            .map_err(llvm_err)?;
        let t2 = self
            .builder
            .build_insert_value(t1, cancelled_zero, 2, "t2")
            .map_err(llvm_err)?;
        let sched_zero = self.i64_ty().const_int(0, false); // default scheduler for withTimeout
        let t3 = self
            .builder
            .build_insert_value(t2, sched_zero, 3, "t3_sched")
            .map_err(llvm_err)?;
        let t4 = self
            .builder
            .build_insert_value(t3, el2, 4, "t4_list")
            .map_err(llvm_err)?;
        self.builder.build_store(task_heap, t4).map_err(llvm_err)?;

        // 2. Compile body into a thread function
        self.lambda_count += 1;
        let task_name = format!(".wt_body_{}", self.lambda_count);
        let fn_type = self.ptr_ty().fn_type(&[self.ptr_ty().into()], false);
        let task_fn = self.module.add_function(&task_name, fn_type, None);
        let entry = self.context.append_basic_block(task_fn, "entry");

        let saved_pos = self.builder.get_insert_block();
        let mut saved_scope = Scope::new();
        std::mem::swap(&mut self.scope, &mut saved_scope);
        self.scope = Scope::new();

        self.builder.position_at_end(entry);
        let task_ptr_param = task_fn.get_first_param().unwrap().into_pointer_value();

        let result = self.compile_expr(&body_expr)?;

        // Store result in the task's result_list
        let cap = self.i64_ty().const_int(1, false);
        let cc = self.call_rt("action_list_create", &[cap.into()])?;
        let list_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("wt list_create failed")?;
        let rl_alloca = self
            .builder
            .build_alloca(self.list_type, "wt_rl")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rl_alloca, list_bv)
            .map_err(llvm_err)?;
        self.push_to_collector(rl_alloca, &result)?;

        // Write done=1 and result_list to task struct
        let updated_list = self
            .builder
            .build_load(self.list_type, rl_alloca, "wt_ul")
            .map_err(llvm_err)?;
        let task_ptr_cast = self
            .builder
            .build_pointer_cast(
                task_ptr_param,
                self.context.ptr_type(Default::default()),
                "wt_task_cast",
            )
            .map_err(llvm_err)?;
        let loaded_task = self
            .builder
            .build_load(self.task_type, task_ptr_cast, "wt_lt")
            .map_err(llvm_err)?
            .into_struct_value();
        let done_one = self.i64_ty().const_int(1, false);
        let cancelled_val = self
            .builder
            .build_extract_value(loaded_task, 2, "wt_cv")
            .map_err(llvm_err)?;
        let pt_val = self
            .builder
            .build_extract_value(loaded_task, 0, "wt_pv")
            .map_err(llvm_err)?;
        let undef2 = self.task_type.get_undef();
        let u0 = self
            .builder
            .build_insert_value(undef2, pt_val, 0, "u0")
            .map_err(llvm_err)?;
        let u1 = self
            .builder
            .build_insert_value(u0, done_one, 1, "u1")
            .map_err(llvm_err)?;
        let u2 = self
            .builder
            .build_insert_value(u1, cancelled_val, 2, "u2")
            .map_err(llvm_err)?;
        let wt_sched_val = self
            .builder
            .build_extract_value(loaded_task, 3, "wt_sv")
            .map_err(llvm_err)?;
        let u3 = self
            .builder
            .build_insert_value(u2, wt_sched_val, 3, "u3_sched")
            .map_err(llvm_err)?;
        let u4 = self
            .builder
            .build_insert_value(u3, updated_list, 4, "u4_list")
            .map_err(llvm_err)?;
        self.builder
            .build_store(task_ptr_cast, u4)
            .map_err(llvm_err)?;
        let null_ret = self.ptr_ty().const_null();
        let _ = self.builder.build_return(Some(&null_ret));

        std::mem::swap(&mut self.scope, &mut saved_scope);
        if let Some(pos) = saved_pos {
            self.builder.position_at_end(pos);
        }

        // 3. Spawn thread with pthread_create
        let pthread_create_fn = self
            .module
            .get_function("action_thread_create")
            .ok_or("action_thread_create not found")?;
        let pthread_field_ptr = self
            .builder
            .build_struct_gep(self.task_type, task_heap, 0, "wt_ptf")
            .map_err(llvm_err)?;
        let fn_as_ptr = task_fn.as_global_value().as_pointer_value();
        let _ = self
            .builder
            .build_call(
                pthread_create_fn,
                &[
                    pthread_field_ptr.into(),
                    self.ptr_ty().const_null().into(),
                    fn_as_ptr.into(),
                    task_heap.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;

        // 4. Polling loop: check done flag every 10ms until timeout
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no fn")?;
        let done_field_ptr = self
            .builder
            .build_struct_gep(self.task_type, task_heap, 1, "wt_done_ptr")
            .map_err(llvm_err)?;
        let elapsed_alloca = self
            .builder
            .build_alloca(self.i64_ty(), "wt_elapsed")
            .map_err(llvm_err)?;
        self.builder
            .build_store(elapsed_alloca, self.i64_ty().const_int(0, false))
            .map_err(llvm_err)?;
        let poll_interval = 10_000_i64; // 10ms in microseconds

        let poll_hdr = self.context.append_basic_block(current_fn, "wt_poll_hdr");
        let poll_body = self.context.append_basic_block(current_fn, "wt_poll_body");
        let poll_done = self.context.append_basic_block(current_fn, "wt_poll_done");
        let poll_timeout = self
            .context
            .append_basic_block(current_fn, "wt_poll_timeout");
        let wt_return = self.context.append_basic_block(current_fn, "wt_return");
        let wt_nullable_ty = self.get_nullable_type(self.string_type.into(), "Nullable");
        let wt_result_alloca = self
            .builder
            .build_alloca(wt_nullable_ty, "wt_res")
            .map_err(llvm_err)?;

        let _ = self.builder.build_unconditional_branch(poll_hdr);
        self.builder.position_at_end(poll_hdr);
        // Load elapsed and check if >= timeout_ms
        let elapsed = self
            .builder
            .build_load(self.i64_ty(), elapsed_alloca, "wt_el")
            .map_err(llvm_err)?
            .into_int_value();
        let timed_out = self
            .builder
            .build_int_compare(IntPredicate::SGE, elapsed, timeout_ms, "wt_to")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(timed_out, poll_timeout, poll_body);

        // Poll body: sleep 10ms, then check done flag
        self.builder.position_at_end(poll_body);
        let usleep_fn = self
            .module
            .get_function("action_sleep_us")
            .ok_or("action_sleep_us not found")?;
        let _ = self
            .builder
            .build_call(
                usleep_fn,
                &[self.i32_ty().const_int(poll_interval as u64, false).into()],
                "",
            )
            .map_err(llvm_err)?;
        // Update elapsed
        let new_elapsed = self
            .builder
            .build_int_add(elapsed, self.i64_ty().const_int(10, false), "wt_ne")
            .map_err(llvm_err)?;
        self.builder
            .build_store(elapsed_alloca, new_elapsed)
            .map_err(llvm_err)?;
        // Check done flag
        let done_val = self
            .builder
            .build_load(self.i64_ty(), done_field_ptr, "wt_dv")
            .map_err(llvm_err)?
            .into_int_value();
        let is_done = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                done_val,
                self.i64_ty().const_int(0, false),
                "wt_id",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_done, poll_done, poll_hdr);

        // Timeout: cancel thread, join, return null (nullable {i1=1, undef})
        self.builder.position_at_end(poll_timeout);
        let pthread_cancel_fn = self
            .module
            .get_function("action_thread_cancel")
            .ok_or("action_thread_cancel not found")?;
        let pthread_val_t = self
            .builder
            .build_load(self.i64_ty(), pthread_field_ptr, "wt_ptv")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(pthread_cancel_fn, &[pthread_val_t.into()], "")
            .map_err(llvm_err)?;
        let pthread_join_fn = self
            .module
            .get_function("action_thread_join")
            .ok_or("action_thread_join not found")?;
        let _ = self
            .builder
            .build_call(
                pthread_join_fn,
                &[pthread_val_t.into(), self.ptr_ty().const_null().into()],
                "",
            )
            .map_err(llvm_err)?;
        // Build nullable {i8=1, undef} for timeout (null)
        let null_undef = wt_nullable_ty.get_undef();
        let null_flag = self.null_flag_ty().const_int(1, false);
        let with_flag = self
            .builder
            .build_insert_value(null_undef, null_flag, 0, "nul_f")
            .map_err(llvm_err)?;
        let inner_undef = self.string_type.get_undef();
        let null_full = self
            .builder
            .build_insert_value(with_flag, inner_undef, 1, "nul_v")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wt_result_alloca, null_full)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(wt_return);

        // Done: pthread_join and return Ok(result)
        self.builder.position_at_end(poll_done);
        // pthread_join if not already joined
        let done_pthread_val = self
            .builder
            .build_load(self.i64_ty(), pthread_field_ptr, "wt_dpt")
            .map_err(llvm_err)?
            .into_int_value();
        // Only join from the success path (not timeout)
        let pt_is_nonzero = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                done_pthread_val,
                self.i64_ty().const_int(0, false),
                "pt_nz",
            )
            .map_err(llvm_err)?;
        let join_bb = self.context.append_basic_block(current_fn, "wt_join");
        let merge_bb = self.context.append_basic_block(current_fn, "wt_merge");
        let _ = self
            .builder
            .build_conditional_branch(pt_is_nonzero, join_bb, merge_bb);
        self.builder.position_at_end(join_bb);
        let _ = self
            .builder
            .build_call(
                pthread_join_fn,
                &[done_pthread_val.into(), self.ptr_ty().const_null().into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        self.builder.position_at_end(merge_bb);

        // Load result from task's result_list
        let task_sv = self
            .builder
            .build_load(self.task_type, task_heap, "wt_tsk")
            .map_err(llvm_err)?
            .into_struct_value();
        let result_list_sv = self
            .builder
            .build_extract_value(task_sv, 4, "wt_rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rla = self
            .builder
            .build_alloca(self.list_type, "wt_rla")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rla, result_list_sv)
            .map_err(llvm_err)?;
        let rl_val = self.load_list(rla)?;
        let zero = self.i64_ty().const_int(0, false);
        let cc = self.call_rt("action_list_get", &[rl_val.into(), zero.into()])?;
        let fat = cc
            .try_as_basic_value()
            .basic()
            .ok_or("wt get failed")?
            .into_struct_value();

        // Free task heap
        let free_fn = self.module.get_function("free").ok_or("free not found")?;
        let _ = self
            .builder
            .build_call(free_fn, &[task_heap.into()], "")
            .map_err(llvm_err)?;

        // Wrap result in nullable {i8=0, fat} (non-null)
        let ok_undef = wt_nullable_ty.get_undef();
        let ok_flag = self.null_flag_ty().const_int(0, false);
        let ok_with_flag = self
            .builder
            .build_insert_value(ok_undef, ok_flag, 0, "ok_f")
            .map_err(llvm_err)?;
        let ok_full = self
            .builder
            .build_insert_value(ok_with_flag, fat, 1, "ok_v")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wt_result_alloca, ok_full)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(wt_return);

        // Return: load nullable from alloca
        self.builder.position_at_end(wt_return);
        Ok(TypedValue::Nullable(
            wt_result_alloca,
            wt_nullable_ty.into(),
        ))
    }

    /// stream() — create a new Stream<T> channel with mutex + condvar + buffer.
    /// Stream struct (heap-allocated): {mutex: [40 x i8], cond: [48 x i8], closed: i64, list: {ptr, i64, i64}}
    pub(super) fn builtin_stream_create(&mut self) -> Result<TypedValue<'ctx>, String> {
        let stream_ty = self.stream_type;
        let null_ptr = self.context.ptr_type(Default::default()).const_null();
        let size_ptr = unsafe {
            self.builder
                .build_gep(
                    stream_ty,
                    null_ptr,
                    &[self.i64_ty().const_int(1, false)],
                    "stream_size_ptr",
                )
                .map_err(llvm_err)
        }?;
        let stream_size = self
            .builder
            .build_ptr_to_int(size_ptr, self.i64_ty(), "stream_size")
            .map_err(llvm_err)?;
        let malloc_fn = self
            .module
            .get_function("malloc")
            .ok_or("malloc not found")?;
        let stream_buf = self
            .builder
            .build_call(malloc_fn, &[stream_size.into()], "stream_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let stream_ptr = self
            .builder
            .build_pointer_cast(
                stream_buf,
                self.context.ptr_type(Default::default()),
                "stream_ptr",
            )
            .map_err(llvm_err)?;

        // Initialize mutex (field 0)
        let pthread_mutex_init_fn = self
            .module
            .get_function("action_mutex_init")
            .ok_or("action_mutex_init not found")?;
        let mutex_field_ptr = self
            .builder
            .build_struct_gep(stream_ty, stream_ptr, 0, "mutex_field")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                pthread_mutex_init_fn,
                &[mutex_field_ptr.into(), self.ptr_ty().const_null().into()],
                "",
            )
            .map_err(llvm_err)?;

        // Initialize condvar (field 1)
        let pthread_cond_init_fn = self
            .module
            .get_function("action_cond_init")
            .ok_or("action_cond_init not found")?;
        let cond_field_ptr = self
            .builder
            .build_struct_gep(stream_ty, stream_ptr, 1, "cond_field")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                pthread_cond_init_fn,
                &[cond_field_ptr.into(), self.ptr_ty().const_null().into()],
                "",
            )
            .map_err(llvm_err)?;

        // Initialize closed flag to 0 (field 2)
        let closed_field_ptr = self
            .builder
            .build_struct_gep(stream_ty, stream_ptr, 2, "closed_field")
            .map_err(llvm_err)?;
        self.builder
            .build_store(closed_field_ptr, self.i64_ty().const_int(0, false))
            .map_err(llvm_err)?;

        // Initialize list (field 3)
        let cap = self.i64_ty().const_int(4, false);
        let cc = self.call_rt("action_list_create", &[cap.into()])?;
        let list_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("stream list_create failed")?;
        let list_field_ptr = self
            .builder
            .build_struct_gep(stream_ty, stream_ptr, 3, "list_field")
            .map_err(llvm_err)?;
        self.builder
            .build_store(list_field_ptr, list_bv)
            .map_err(llvm_err)?;

        Ok(TypedValue::Stream(stream_ptr))
    }

    /// Stream operations: send(stream, value), receive(stream), close(stream)
    pub(super) fn builtin_stream_op(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "send" => {
                if args.len() != 2 {
                    return Err("send expects 2 arguments: stream and value".to_string());
                }
                let stream_val = self.compile_expr(&args[0])?;
                let stream_ptr = match stream_val {
                    TypedValue::Stream(p) => p,
                    _ => return Err("send: first argument must be a Stream".to_string()),
                };
                let value = self.compile_expr(&args[1])?;
                let mutex_ptr = self
                    .builder
                    .build_struct_gep(self.stream_type, stream_ptr, 0, "sm")
                    .map_err(llvm_err)?;
                let _ = self
                    .builder
                    .build_call(
                        self.module.get_function("action_mutex_lock").unwrap(),
                        &[mutex_ptr.into()],
                        "",
                    )
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
                let _ = self
                    .builder
                    .build_call(
                        self.module.get_function("action_cond_signal").unwrap(),
                        &[cond_ptr.into()],
                        "",
                    )
                    .map_err(llvm_err)?;
                // Unlock
                let _ = self
                    .builder
                    .build_call(
                        self.module.get_function("action_mutex_unlock").unwrap(),
                        &[mutex_ptr.into()],
                        "",
                    )
                    .map_err(llvm_err)?;
                Ok(TypedValue::Unit)
            }
            "receive" => {
                if args.len() != 1 {
                    return Err("receive expects 1 argument: stream".to_string());
                }
                let stream_val = self.compile_expr(&args[0])?;
                let stream_ptr = match stream_val {
                    TypedValue::Stream(p) => p,
                    _ => return Err("receive: argument must be a Stream".to_string()),
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
                    .build_alloca(self.i64_ty(), "sop_result")
                    .map_err(llvm_err)?;
                let lock_fn = self.module.get_function("action_mutex_lock").unwrap();
                let unlock_fn = self.module.get_function("action_mutex_unlock").unwrap();
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
                let merge_bb = self.context.append_basic_block(cur_fn, "sop_merge");
                let _ = self
                    .builder
                    .build_call(lock_fn, &[mutex_ptr.into()], "")
                    .map_err(llvm_err)?;
                // Wait loop: while list is empty and not closed, cond_wait
                let wait_loop_bb = self.context.append_basic_block(cur_fn, "sop_wait_loop");
                let got_data_bb = self.context.append_basic_block(cur_fn, "sop_got_data");
                let empty_bb = self.context.append_basic_block(cur_fn, "sop_empty");
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
                let _ = self
                    .builder
                    .build_conditional_branch(has_data, got_data_bb, empty_bb);
                // Empty: check closed
                self.builder.position_at_end(empty_bb);
                let closed_val = self
                    .builder
                    .build_load(self.i64_ty(), closed_ptr, "closed_val")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_closed = self
                    .builder
                    .build_int_compare(IntPredicate::NE, closed_val, zero, "is_closed")
                    .map_err(llvm_err)?;
                let do_wait_bb = self.context.append_basic_block(cur_fn, "sop_cond_wait");
                let ret_zero_bb = self.context.append_basic_block(cur_fn, "sop_ret_zero");
                let _ = self
                    .builder
                    .build_conditional_branch(is_closed, ret_zero_bb, do_wait_bb);
                self.builder.position_at_end(do_wait_bb);
                let _ = self
                    .builder
                    .build_call(cond_wait_fn, &[cond_ptr.into(), mutex_ptr.into()], "")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(wait_loop_bb);
                // Closed & empty: return 0
                self.builder.position_at_end(ret_zero_bb);
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
                // Re-load list_val in this block (can't use value from wait_loop across cond_wait)
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
                let shift_bb = self.context.append_basic_block(cur_fn, "sop_shift_bb");
                let done_bb = self.context.append_basic_block(cur_fn, "sop_shift_done");
                let _ = self
                    .builder
                    .build_conditional_branch(has_more, shift_bb, done_bb);
                self.builder.position_at_end(shift_bb);
                let mm_fn = self
                    .module
                    .get_function("memmove")
                    .ok_or("memmove not found")?;
                let src_ptr = unsafe {
                    self.builder
                        .build_gep(self.string_type, data_ptr, &[one], "src")
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
                        &[data_ptr.into(), src_ptr.into(), move_bytes.into()],
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
                // Merge: load result and return
                self.builder.position_at_end(merge_bb);
                let result = self
                    .builder
                    .build_load(self.i64_ty(), result_alloca, "sop_load_result")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::Int(result))
            }
            "close" => {
                if args.len() != 1 {
                    return Err("close expects 1 argument: stream".to_string());
                }
                let stream_val = self.compile_expr(&args[0])?;
                let stream_ptr = match stream_val {
                    TypedValue::Stream(p) => p,
                    _ => return Err("close: argument must be a Stream".to_string()),
                };
                // Lock mutex, set closed=1, broadcast to wake all waiters, unlock
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
                Ok(TypedValue::Unit)
            }
            _ => Err(format!("Unknown Stream operation: {}", name)),
        }
    }

    /// Task operations: cancel(task), is_done(task), is_cancelled(task), wait(task)
    /// Task struct: {pthread: i64, done: i64, cancelled: i64, result_list: {ptr, i64, i64}}
    pub(super) fn builtin_task_op(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err(format!("{} expects 1 argument: task", name));
        }
        let task_val = self.compile_expr(&args[0])?;
        let task_ptr = match task_val {
            TypedValue::Task(p) => p,
            _ => return Err(format!("{}: argument must be a Task", name)),
        };
        let tv = self
            .builder
            .build_load(self.task_type, task_ptr, "task_val")
            .map_err(llvm_err)?
            .into_struct_value();
        match name {
            "cancel" => {
                let cancelled_one = self.i64_ty().const_int(1, false);
                let updated = self
                    .builder
                    .build_insert_value(tv, cancelled_one, 2, "t_canc_set")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(task_ptr, updated)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Unit)
            }
            "is_done" => {
                let done = self
                    .builder
                    .build_extract_value(tv, 1, "is_done")
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
                Ok(TypedValue::Bool(is_true))
            }
            "is_cancelled" => {
                let cancelled = self
                    .builder
                    .build_extract_value(tv, 2, "is_canc")
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
                Ok(TypedValue::Bool(is_true))
            }
            "wait" => {
                // pthread_join the task, then extract result
                let pthread_val = self
                    .builder
                    .build_extract_value(tv, 0, "pt")
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
                // Reload task struct after join (thread updated done, result_list fields)
                let tv2 = self
                    .builder
                    .build_load(self.task_type, task_ptr, "task_val2")
                    .map_err(llvm_err)?
                    .into_struct_value();
                // Extract result list from task struct field 4
                let result_list = self
                    .builder
                    .build_extract_value(tv2, 4, "wait_list")
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
                let cc = self.call_rt("action_list_get", &[list_val.into(), zero.into()])?;
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
                Ok(TypedValue::Int(tag))
            }
            _ => Err(format!("Unknown Task operation: {}", name)),
        }
    }

    pub(super) fn builtin_map(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // map(fn, list) or map(list) { lambda }
        let (fn_ptr, list_val) = if let Some(lam) = trailing {
            // map(list) { lambda }
            if args.len() != 1 {
                return Err("map with trailing lambda expects 1 argument (list)".to_string());
            }
            let lv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv)
        } else if args.len() == 2 {
            let fv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            (fv, lv)
        } else {
            return Err("map expects 2 arguments (fn, list)".to_string());
        };

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("map: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("map: second argument must be a list".to_string()),
        };

        // Build the result list
        let list_struct = self.load_list(list_ptr)?;
        let input_len = self.list_len_val(list_struct)?;

        // Create new list with same capacity
        let new_list_cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let new_list_bv = new_list_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "map_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_list_bv)
            .map_err(llvm_err)?;

        // Build loop
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile map outside function")?;

        let i64 = self.i64_ty();
        let i_alloca = self.builder.build_alloca(i64, "map_i").map_err(llvm_err)?;
        let zero = i64.const_int(0, false);
        self.builder.build_store(i_alloca, zero).map_err(llvm_err)?;

        let loop_header = self.context.append_basic_block(current_fn, "map_header");
        let loop_body = self.context.append_basic_block(current_fn, "map_body");
        let loop_exit = self.context.append_basic_block(current_fn, "map_exit");

        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_header);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, input_len, "map_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        self.builder.position_at_end(loop_body);

        // Get element from input list (fat {i64,ptr} struct)
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), i_val.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        // Extract tag (first field) to pass to lambda (lambdas still take i64)
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "elem_tag")
            .map_err(llvm_err)?;

        // Call the lambda with the element tag (returns fat {i64,ptr})
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "map_call")
            .map_err(llvm_err)?;
        let mapped_bv = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("map call failed")?;

        // Push lambda result (fat {i64,ptr}) to result list
        let result_list = self.load_list(result_alloca)?;
        let push_cc = self.call_rt("action_list_push", &[result_list.into(), mapped_bv.into()])?;
        let pushed = push_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_push failed")?;
        self.builder
            .build_store(result_alloca, pushed)
            .map_err(llvm_err)?;

        // Increment counter
        let one = i64.const_int(1, false);
        let next = self
            .builder
            .build_int_add(i_val, one, "i_next")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, next).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_exit);
        Ok(TypedValue::List(result_alloca))
    }

    pub(super) fn builtin_filter(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_val) = if let Some(lam) = trailing {
            if args.len() != 1 {
                return Err("filter with trailing lambda expects 1 argument (list)".to_string());
            }
            let lv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv)
        } else if args.len() == 2 {
            let fv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            (fv, lv)
        } else {
            return Err("filter expects 2 arguments (fn, list)".to_string());
        };

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("filter: first argument must be a function".to_string()),
        };
        let list_ptr = match list_val {
            TypedValue::List(p) => p,
            _ => return Err("filter: second argument must be a list".to_string()),
        };

        let list_struct = self.load_list(list_ptr)?;
        let input_len = self.list_len_val(list_struct)?;

        let new_list_cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let new_list_bv = new_list_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "filter_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_list_bv)
            .map_err(llvm_err)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile filter outside function")?;

        let i64 = self.i64_ty();
        let i_alloca = self
            .builder
            .build_alloca(i64, "filter_i")
            .map_err(llvm_err)?;
        let zero = i64.const_int(0, false);
        self.builder.build_store(i_alloca, zero).map_err(llvm_err)?;

        let loop_header = self.context.append_basic_block(current_fn, "filter_header");
        let loop_body = self.context.append_basic_block(current_fn, "filter_body");
        let loop_push = self.context.append_basic_block(current_fn, "filter_push");
        let loop_inc = self.context.append_basic_block(current_fn, "filter_inc");
        let loop_exit = self.context.append_basic_block(current_fn, "filter_exit");

        let _ = self.builder.build_unconditional_branch(loop_header);

        // Header: check i < len
        self.builder.position_at_end(loop_header);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, input_len, "filter_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        // Body: get element, call predicate
        self.builder.position_at_end(loop_body);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), i_val.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        // Extract tag to pass to predicate (lambdas still take i64)
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "elem_tag")
            .map_err(llvm_err)?;

        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "filter_call")
            .map_err(llvm_err)?;
        let pred_bv = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("filter call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "filter_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, zero, "is_true")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_true, loop_push, loop_inc);

        // Push: add original fat struct element to result list
        self.builder.position_at_end(loop_push);
        let result_list = self.load_list(result_alloca)?;
        let push_cc = self.call_rt("action_list_push", &[result_list.into(), elem_val.into()])?;
        let pushed = push_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_push failed")?;
        self.builder
            .build_store(result_alloca, pushed)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_inc);

        // Increment: i++ then back to header
        self.builder.position_at_end(loop_inc);
        let i_next = self
            .builder
            .build_load(i64, i_alloca, "i_next")
            .map_err(llvm_err)?
            .into_int_value();
        let one = i64.const_int(1, false);
        let next = self
            .builder
            .build_int_add(i_next, one, "i_inc")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, next).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_exit);
        Ok(TypedValue::List(result_alloca))
    }

    pub(super) fn builtin_fold(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // fold(fn, init, list) or fold(init, list) { lambda }
        let (fn_ptr, init_val, list_val) = if let Some(lam) = trailing {
            if args.len() != 2 {
                return Err(
                    "fold with trailing lambda expects 2 arguments (init, list)".to_string()
                );
            }
            let iv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            let fv = self.compile_expr(lam)?;
            (fv, iv, lv)
        } else if args.len() == 3 {
            let fv = self.compile_expr(&args[0])?;
            let iv = self.compile_expr(&args[1])?;
            let lv = self.compile_expr(&args[2])?;
            (fv, iv, lv)
        } else {
            return Err("fold expects 3 arguments (fn, init, list)".to_string());
        };

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
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
        let input_len = self.list_len_val(list_struct)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile fold outside function")?;

        let i64 = self.i64_ty();
        let acc_alloca = self
            .builder
            .build_alloca(i64, "fold_acc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(acc_alloca, init_i64)
            .map_err(llvm_err)?;

        let i_alloca = self.builder.build_alloca(i64, "fold_i").map_err(llvm_err)?;
        let zero = i64.const_int(0, false);
        self.builder.build_store(i_alloca, zero).map_err(llvm_err)?;

        let loop_header = self.context.append_basic_block(current_fn, "fold_header");
        let loop_body = self.context.append_basic_block(current_fn, "fold_body");
        let loop_exit = self.context.append_basic_block(current_fn, "fold_exit");

        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_header);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, input_len, "fold_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        self.builder.position_at_end(loop_body);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), i_val.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        // Extract tag to pass to fold lambda
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "elem_tag")
            .map_err(llvm_err)?;
        let acc = self
            .builder
            .build_load(i64, acc_alloca, "acc")
            .map_err(llvm_err)?;

        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[acc.into(), elem_tag.into()], "fold_call")
            .map_err(llvm_err)?;
        let new_acc_bv = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("fold call failed")?;
        let new_acc = if new_acc_bv.is_struct_value() {
            self.builder
                .build_extract_value(new_acc_bv.into_struct_value(), 0, "fold_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            new_acc_bv.into_int_value()
        };
        self.builder
            .build_store(acc_alloca, new_acc)
            .map_err(llvm_err)?;

        let one = i64.const_int(1, false);
        let next = self
            .builder
            .build_int_add(i_val, one, "i_next")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, next).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_exit);
        let final_acc = self
            .builder
            .build_load(i64, acc_alloca, "final_acc")
            .map_err(llvm_err)?;
        Ok(TypedValue::Int(final_acc.into_int_value()))
    }

    /// flatMap(fn, list) = flatten(map(fn, list))
    pub(super) fn builtin_flat_map_list(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
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

    pub(super) fn builtin_callback_list(
        &mut self,
        name: &str,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
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

    /// any(list, fn) or any(list) { lambda } -> Bool
    pub(super) fn builtin_any(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "any")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let result_a = self
            .builder
            .build_alloca(self.bool_ty(), "any_res")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_a, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "any_hdr");
        let bdy = self.context.append_basic_block(current_fn, "any_bdy");
        let ext = self.context.append_basic_block(current_fn, "any_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "any_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        // Accumulate: result = result OR is_true
        let cur = self
            .builder
            .build_load(self.bool_ty(), result_a, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let new_res = self
            .builder
            .build_or(cur, is_true, "new_res")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_a, new_res)
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let res = self
            .builder
            .build_load(self.bool_ty(), result_a, "res")
            .map_err(llvm_err)?;
        Ok(TypedValue::Bool(res.into_int_value()))
    }

    /// all(list, fn) or all(list) { lambda } -> Bool
    pub(super) fn builtin_all(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "all")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let result_a = self
            .builder
            .build_alloca(self.bool_ty(), "all_res")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_a, self.bool_ty().const_int(1, false))
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "all_hdr");
        let bdy = self.context.append_basic_block(current_fn, "all_bdy");
        let ext = self.context.append_basic_block(current_fn, "all_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "all_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        // Accumulate: result = result AND is_true
        let cur = self
            .builder
            .build_load(self.bool_ty(), result_a, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let new_res = self
            .builder
            .build_and(cur, is_true, "new_res")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_a, new_res)
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let res = self
            .builder
            .build_load(self.bool_ty(), result_a, "res")
            .map_err(llvm_err)?;
        Ok(TypedValue::Bool(res.into_int_value()))
    }

    /// find(list, fn) or find(list) { lambda } -> Option<T>
    pub(super) fn builtin_find(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "find")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        // Allocate fat struct slot for found element
        let found_a = self
            .builder
            .build_alloca(self.string_type, "found")
            .map_err(llvm_err)?;
        let found_flag_a = self
            .builder
            .build_alloca(self.bool_ty(), "found_f")
            .map_err(llvm_err)?;
        self.builder
            .build_store(found_flag_a, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "find_hdr");
        let bdy = self.context.append_basic_block(current_fn, "find_bdy");
        let found_bb = self.context.append_basic_block(current_fn, "find_found");
        let ext = self.context.append_basic_block(current_fn, "find_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "find_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_true, found_bb, hdr);
        self.builder.position_at_end(found_bb);
        self.builder
            .build_store(found_a, elem_val)
            .map_err(llvm_err)?;
        self.builder
            .build_store(found_flag_a, self.bool_ty().const_int(1, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ext);
        self.builder.position_at_end(ext);
        // Build nullable String: set flag 0 + found value, or flag 1 (null).
        // InnerType defaults to Int — list elements are fat structs whose type
        // is only known at runtime. Fixing this requires adding element type info
        // to List/Map/Set TypedValue variants.
        self.build_nullable_str(found_a, found_flag_a)
    }

    /// findIndex(list, fn) or findIndex(list) { lambda } -> Option<Int>
    pub(super) fn builtin_find_index(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "findIndex")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let result_a = self.builder.build_alloca(i64, "fi_idx").map_err(llvm_err)?;
        self.builder
            .build_store(result_a, i64.const_int((-1i64) as u64, true))
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "fi_hdr");
        let bdy = self.context.append_basic_block(current_fn, "fi_bdy");
        let ext = self.context.append_basic_block(current_fn, "fi_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "fi_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        self.builder.build_store(result_a, iv).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let fi_hdr2 = self.context.append_basic_block(current_fn, "fi_chk");
        let _ = self.builder.build_conditional_branch(is_true, ext, fi_hdr2);
        self.builder.position_at_end(fi_hdr2);
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let found_idx = self
            .builder
            .build_load(i64, result_a, "found_idx")
            .map_err(llvm_err)?
            .into_int_value();
        let is_found = self
            .builder
            .build_int_compare(
                IntPredicate::SGE,
                found_idx,
                i64.const_int(0, false),
                "is_found",
            )
            .map_err(llvm_err)?;
        // Build nullable Int: set flag 0 + found_idx, or flag 1 (null)
        self.build_nullable_int(found_idx, is_found)
    }

    /// reduce(list, fn) or reduce(list) { lambda } -> Option<T>
    pub(super) fn builtin_reduce(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "reduce")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let is_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, input_len, zero, "is_empty")
            .map_err(llvm_err)?;
        // Accumulator: fat {i64,ptr}
        let acc_a = self
            .builder
            .build_alloca(self.string_type, "reduce_acc")
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(i_a, one).map_err(llvm_err)?;
        // Init: load first element into acc
        let init_bb = self.context.append_basic_block(current_fn, "reduce_init");
        let loop_hdr = self.context.append_basic_block(current_fn, "reduce_hdr");
        let loop_bdy = self.context.append_basic_block(current_fn, "reduce_bdy");
        let loop_ext = self.context.append_basic_block(current_fn, "reduce_ext");
        let empty_bb = self.context.append_basic_block(current_fn, "reduce_empty");
        let merge_bb = self.context.append_basic_block(current_fn, "reduce_merge");
        let _ = self
            .builder
            .build_conditional_branch(is_empty, empty_bb, init_bb);
        // Init: load first element
        self.builder.position_at_end(init_bb);
        let input_list0 = self.load_list(list_ptr)?;
        let first = self.call_rt("action_list_get", &[input_list0.into(), zero.into()])?;
        let first_val = first
            .try_as_basic_value()
            .basic()
            .ok_or("list_get failed")?;
        self.builder
            .build_store(acc_a, first_val)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);
        // Loop
        self.builder.position_at_end(loop_hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_bdy, loop_ext);
        self.builder.position_at_end(loop_bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let acc_fat = self
            .builder
            .build_load(self.string_type, acc_a, "acc")
            .map_err(llvm_err)?;
        let acc_tag = self
            .builder
            .build_extract_value(acc_fat.into_struct_value(), 0, "acc_tag")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(
                fn_type,
                fn_ptr,
                &[acc_tag.into(), elem_tag.into()],
                "reduce_call",
            )
            .map_err(llvm_err)?;
        let new_acc = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        self.builder.build_store(acc_a, new_acc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_hdr);
        self.builder.position_at_end(loop_ext);
        let final_acc = self
            .builder
            .build_load(self.string_type, acc_a, "final_acc")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Empty: build None
        self.builder.position_at_end(empty_bb);
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge: build Option from fat struct or None
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(self.string_type, "reduce_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[
            (&final_acc, loop_ext),
            (&self.string_type.get_undef(), empty_bb),
        ]);
        let phi_val = phi.as_basic_value();
        let found_flag_a = self
            .builder
            .build_alloca(self.bool_ty(), "red_found")
            .map_err(llvm_err)?;
        let phi_flag = self
            .builder
            .build_phi(self.bool_ty(), "red_flag")
            .map_err(llvm_err)?;
        phi_flag.add_incoming(&[
            (&self.bool_ty().const_int(1, false), loop_ext),
            (&self.bool_ty().const_zero(), empty_bb),
        ]);
        self.builder
            .build_store(found_flag_a, phi_flag.as_basic_value())
            .map_err(llvm_err)?;
        let acc_alloca = self
            .builder
            .build_alloca(self.string_type, "red_acc_s")
            .map_err(llvm_err)?;
        self.builder
            .build_store(acc_alloca, phi_val)
            .map_err(llvm_err)?;
        // InnerType defaults to Int — accumulator is a fat struct whose type
        // is only known at runtime. See comment at builtin_find for details.
        self.build_nullable_str(acc_alloca, found_flag_a)
    }

    /// foldRight(list, init, fn) or foldRight(list, init) { lambda } -> T
    pub(super) fn builtin_fold_right(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr, init_val) = self.extract_fold_right_args(args, trailing)?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        let acc_a = self.builder.build_alloca(i64, "fr_acc").map_err(llvm_err)?;
        self.builder
            .build_store(acc_a, init_val)
            .map_err(llvm_err)?;
        // Iterate backwards: i = len-1 down to 0
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        let start_i = self
            .builder
            .build_int_sub(input_len, one, "start_i")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, start_i).map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "fr_hdr");
        let bdy = self.context.append_basic_block(current_fn, "fr_bdy");
        let ext = self.context.append_basic_block(current_fn, "fr_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, iv, zero, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let acc = self
            .builder
            .build_load(i64, acc_a, "acc")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into(), acc.into()], "fr_call")
            .map_err(llvm_err)?;
        let new_acc_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let new_acc = if new_acc_bv.is_struct_value() {
            self.builder
                .build_extract_value(new_acc_bv.into_struct_value(), 0, "fr_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            new_acc_bv.into_int_value()
        };
        self.builder.build_store(acc_a, new_acc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_sub(iv, one, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let final_acc = self
            .builder
            .build_load(i64, acc_a, "final_acc")
            .map_err(llvm_err)?
            .into_int_value();
        Ok(TypedValue::Int(final_acc))
    }

    /// takeWhile(list, fn) or takeWhile(list) { lambda } -> List<T>
    pub(super) fn builtin_take_while(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "takeWhile")?;
        let list_struct = self.load_list(list_ptr)?;
        let input_len = self.list_len_val(list_struct)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        // Create result list
        let cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let res_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "tw_res")
            .map_err(llvm_err)?;
        self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "tw_hdr");
        let bdy = self.context.append_basic_block(current_fn, "tw_bdy");
        let ext = self.context.append_basic_block(current_fn, "tw_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "tw_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let push_bb = self.context.append_basic_block(current_fn, "tw_push");
        let _ = self.builder.build_conditional_branch(is_true, push_bb, ext);
        self.builder.position_at_end(push_bb);
        let rl = self
            .builder
            .build_load(self.list_type, res_a, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rp = self.call_rt("action_list_push", &[rl.into(), elem_val.into()])?;
        self.builder
            .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        Ok(TypedValue::List(res_a))
    }

    /// dropWhile(list, fn) or dropWhile(list) { lambda } -> List<T>
    pub(super) fn builtin_drop_while(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "dropWhile")?;
        let list_struct = self.load_list(list_ptr)?;
        let input_len = self.list_len_val(list_struct)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let res_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "dw_res")
            .map_err(llvm_err)?;
        self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
        let dropping_a = self
            .builder
            .build_alloca(self.bool_ty(), "dropping")
            .map_err(llvm_err)?;
        self.builder
            .build_store(dropping_a, self.bool_ty().const_int(1, false))
            .map_err(llvm_err)?;
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "dw_hdr");
        let bdy = self.context.append_basic_block(current_fn, "dw_bdy");
        let ext = self.context.append_basic_block(current_fn, "dw_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let dropping = self
            .builder
            .build_load(self.bool_ty(), dropping_a, "dropping")
            .map_err(llvm_err)?
            .into_int_value();
        let is_dropping = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                dropping,
                self.bool_ty().const_zero(),
                "is_dropping",
            )
            .map_err(llvm_err)?;
        // Only call predicate if still dropping
        let call_bb = self.context.append_basic_block(current_fn, "dw_call");
        let push_bb = self.context.append_basic_block(current_fn, "dw_push");
        let inc_bb = self.context.append_basic_block(current_fn, "dw_inc");
        let _ = self
            .builder
            .build_conditional_branch(is_dropping, call_bb, push_bb);
        // Call predicate
        self.builder.position_at_end(call_bb);
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "dw_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        // If true, still dropping, skip element (go to inc). If false, stop dropping, push element.
        let _ = self
            .builder
            .build_conditional_branch(is_true, inc_bb, push_bb);
        // Push element
        self.builder.position_at_end(push_bb);
        self.builder
            .build_store(dropping_a, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        let rl = self
            .builder
            .build_load(self.list_type, res_a, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rp = self.call_rt("action_list_push", &[rl.into(), elem_val.into()])?;
        self.builder
            .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(inc_bb);
        // Increment
        self.builder.position_at_end(inc_bb);
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        Ok(TypedValue::List(res_a))
    }

    /// sortedBy(list, fn) or sortedBy(list) { lambda } -> List<T>
    /// Uses insertion sort since we can't easily do merge sort with callbacks in LLVM IR
    pub(super) fn builtin_sorted_by(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "sortedBy")?;
        let list_struct = self.load_list(list_ptr)?;
        let input_len = self.list_len_val(list_struct)?;
        let _input_data = self.list_data_ptr(list_struct)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let zero = i64.const_int(0, false);
        let one = i64.const_int(1, false);
        // Create result list (copy of input)
        let cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let res_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let res_a = self
            .builder
            .build_alloca(self.list_type, "sb_res")
            .map_err(llvm_err)?;
        self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
        // Copy all elements to result
        let i_copy_a = self.builder.build_alloca(i64, "i_copy").map_err(llvm_err)?;
        self.builder.build_store(i_copy_a, zero).map_err(llvm_err)?;
        let copy_hdr = self.context.append_basic_block(current_fn, "sb_copy_hdr");
        let copy_bdy = self.context.append_basic_block(current_fn, "sb_copy_bdy");
        let copy_ext = self.context.append_basic_block(current_fn, "sb_copy_ext");
        let _ = self.builder.build_unconditional_branch(copy_hdr);
        self.builder.position_at_end(copy_hdr);
        let ic = self
            .builder
            .build_load(i64, i_copy_a, "ic")
            .map_err(llvm_err)?
            .into_int_value();
        let cc_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ic, input_len, "c_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cc_cond, copy_bdy, copy_ext);
        self.builder.position_at_end(copy_bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), ic.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let rl = self
            .builder
            .build_load(self.list_type, res_a, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rp = self.call_rt("action_list_push", &[rl.into(), elem_val.into()])?;
        self.builder
            .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let nic = self
            .builder
            .build_int_add(ic, one, "nic")
            .map_err(llvm_err)?;
        self.builder.build_store(i_copy_a, nic).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(copy_hdr);
        self.builder.position_at_end(copy_ext);
        // Insertion sort: for i=1..len, for j=i..>0, compare res[j-1] > res[j], swap if so
        let i_a = self.builder.build_alloca(i64, "sb_i").map_err(llvm_err)?;
        self.builder.build_store(i_a, one).map_err(llvm_err)?;
        let outer_hdr = self.context.append_basic_block(current_fn, "sb_outer_hdr");
        let outer_bdy = self.context.append_basic_block(current_fn, "sb_outer_bdy");
        let outer_ext = self.context.append_basic_block(current_fn, "sb_outer_ext");
        let _ = self.builder.build_unconditional_branch(outer_hdr);
        self.builder.position_at_end(outer_hdr);
        let iv_o = self
            .builder
            .build_load(i64, i_a, "iv_o")
            .map_err(llvm_err)?
            .into_int_value();
        let o_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv_o, input_len, "o_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(o_cond, outer_bdy, outer_ext);
        self.builder.position_at_end(outer_bdy);
        let j_a = self.builder.build_alloca(i64, "sb_j").map_err(llvm_err)?;
        self.builder.build_store(j_a, iv_o).map_err(llvm_err)?;
        let inner_hdr = self.context.append_basic_block(current_fn, "sb_inner_hdr");
        let inner_bdy = self.context.append_basic_block(current_fn, "sb_inner_bdy");
        let inner_ext = self.context.append_basic_block(current_fn, "sb_inner_ext");
        let _ = self.builder.build_unconditional_branch(inner_hdr);
        self.builder.position_at_end(inner_hdr);
        let jv = self
            .builder
            .build_load(i64, j_a, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let j_cond = self
            .builder
            .build_int_compare(IntPredicate::SGT, jv, zero, "j_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(j_cond, inner_bdy, inner_ext);
        self.builder.position_at_end(inner_bdy);
        let jm1 = self
            .builder
            .build_int_sub(jv, one, "jm1")
            .map_err(llvm_err)?;
        let res_list_jm1 = self.load_list(res_a)?;
        let elem_jm1 = self.call_rt("action_list_get", &[res_list_jm1.into(), jm1.into()])?;
        let ev_jm1 = elem_jm1
            .try_as_basic_value()
            .basic()
            .ok_or("list_get failed")?;
        let tag_jm1 = self
            .builder
            .build_extract_value(ev_jm1.into_struct_value(), 0, "t_jm1")
            .map_err(llvm_err)?
            .into_int_value();
        let res_list_j = self.load_list(res_a)?;
        let elem_j = self.call_rt("action_list_get", &[res_list_j.into(), jv.into()])?;
        let ev_j = elem_j
            .try_as_basic_value()
            .basic()
            .ok_or("list_get failed")?;
        let tag_j = self
            .builder
            .build_extract_value(ev_j.into_struct_value(), 0, "t_j")
            .map_err(llvm_err)?
            .into_int_value();
        // Call comparator: fn(a, b) -> Bool, returns true if a > b (need swap)
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[tag_jm1.into(), tag_j.into()], "sb_cmp")
            .map_err(llvm_err)?;
        let cmp_bv = call_r.try_as_basic_value().basic().ok_or("cmp failed")?;
        let cmp = if cmp_bv.is_struct_value() {
            self.builder
                .build_extract_value(cmp_bv.into_struct_value(), 0, "cmp")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            cmp_bv.into_int_value()
        };
        let should_swap = self
            .builder
            .build_int_compare(IntPredicate::NE, cmp, zero, "should_swap")
            .map_err(llvm_err)?;
        let swap_bb = self.context.append_basic_block(current_fn, "sb_swap");
        let no_swap_bb = self.context.append_basic_block(current_fn, "sb_noswap");
        let _ = self
            .builder
            .build_conditional_branch(should_swap, swap_bb, no_swap_bb);
        // Swap: use action_list_set
        self.builder.position_at_end(swap_bb);
        let rl_sw = self.load_list(res_a)?;
        let _set1 = self.call_rt("action_list_set", &[rl_sw.into(), jm1.into(), ev_j.into()])?;
        let rl2_sw = self.load_list(res_a)?;
        let set2 = self.call_rt(
            "action_list_set",
            &[rl2_sw.into(), jv.into(), ev_jm1.into()],
        )?;
        let set_bv = set2.try_as_basic_value().basic().ok_or("list_set failed")?;
        self.builder.build_store(res_a, set_bv).map_err(llvm_err)?;
        self.builder.build_store(j_a, jm1).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(inner_hdr);
        self.builder.position_at_end(no_swap_bb);
        let _ = self.builder.build_unconditional_branch(inner_ext);
        self.builder.position_at_end(inner_ext);
        let ni_o = self
            .builder
            .build_int_add(iv_o, one, "ni_o")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni_o).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(outer_hdr);
        self.builder.position_at_end(outer_ext);
        Ok(TypedValue::List(res_a))
    }

    /// partition(list, fn) or partition(list) { lambda } -> (List<T>, List<T>)
    pub(super) fn builtin_partition(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "partition")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        // Create two result lists
        let cap = i64.const_int(4, false);
        let left_cc = self.call_rt("action_list_create", &[cap.into()])?;
        let left_bv = left_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create left")?;
        let left_a = self
            .builder
            .build_alloca(self.list_type, "part_left")
            .map_err(llvm_err)?;
        self.builder
            .build_store(left_a, left_bv)
            .map_err(llvm_err)?;
        let right_cc = self.call_rt("action_list_create", &[cap.into()])?;
        let right_bv = right_cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create right")?;
        let right_a = self
            .builder
            .build_alloca(self.list_type, "part_right")
            .map_err(llvm_err)?;
        self.builder
            .build_store(right_a, right_bv)
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "part_hdr");
        let bdy = self.context.append_basic_block(current_fn, "part_bdy");
        let ext = self.context.append_basic_block(current_fn, "part_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "part_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let left_bb = self.context.append_basic_block(current_fn, "part_left");
        let right_bb = self.context.append_basic_block(current_fn, "part_right");
        let part_merge = self.context.append_basic_block(current_fn, "part_merge2");
        let _ = self
            .builder
            .build_conditional_branch(is_true, left_bb, right_bb);
        // Push to left
        self.builder.position_at_end(left_bb);
        let ll = self.load_list(left_a)?;
        let lp = self.call_rt("action_list_push", &[ll.into(), elem_val.into()])?;
        let lp_bv = lp.try_as_basic_value().basic().ok_or("push left")?;
        self.builder.build_store(left_a, lp_bv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(part_merge);
        // Push to right
        self.builder.position_at_end(right_bb);
        let rl = self.load_list(right_a)?;
        let rp = self.call_rt("action_list_push", &[rl.into(), elem_val.into()])?;
        let rp_bv = rp.try_as_basic_value().basic().ok_or("push right")?;
        self.builder.build_store(right_a, rp_bv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(part_merge);
        self.builder.position_at_end(part_merge);
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        // Build tuple struct: {list_type, list_type}
        let lv = self
            .builder
            .build_load(self.list_type, left_a, "lv")
            .map_err(llvm_err)?;
        let rv = self
            .builder
            .build_load(self.list_type, right_a, "rv")
            .map_err(llvm_err)?;
        let tuple_ty = self
            .context
            .struct_type(&[self.list_type.into(), self.list_type.into()], false);
        let undef = tuple_ty.get_undef();
        let t1 = self
            .builder
            .build_insert_value(undef, lv, 0, "t_l")
            .map_err(llvm_err)?;
        let t2 = self
            .builder
            .build_insert_value(t1, rv, 1, "t_r")
            .map_err(llvm_err)?;
        let alloca = self
            .builder
            .build_alloca(tuple_ty, "part_tuple")
            .map_err(llvm_err)?;
        self.builder.build_store(alloca, t2).map_err(llvm_err)?;
        Ok(TypedValue::Struct(alloca, tuple_ty))
    }

    /// count(list, fn) or count(list) { lambda } -> Int
    pub(super) fn builtin_count(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, list_ptr) = self.extract_callback_args(args, trailing, 1, "count")?;
        let input_len = self.list_len_val(self.load_list(list_ptr)?)?;
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let i64 = self.i64_ty();
        let i_a = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let cnt_a = self.builder.build_alloca(i64, "cnt").map_err(llvm_err)?;
        self.builder
            .build_store(cnt_a, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let hdr = self.context.append_basic_block(current_fn, "cnt_hdr");
        let bdy = self.context.append_basic_block(current_fn, "cnt_bdy");
        let ext = self.context.append_basic_block(current_fn, "cnt_ext");
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(hdr);
        let iv = self
            .builder
            .build_load(i64, i_a, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, iv, input_len, "cond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(cond, bdy, ext);
        self.builder.position_at_end(bdy);
        let input_list = self.load_list(list_ptr)?;
        let elem = self.call_rt("action_list_get", &[input_list.into(), iv.into()])?;
        let elem_val = elem.try_as_basic_value().basic().ok_or("list_get failed")?;
        let elem_tag = self
            .builder
            .build_extract_value(elem_val.into_struct_value(), 0, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_r = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[elem_tag.into()], "cnt_call")
            .map_err(llvm_err)?;
        let pred_bv = call_r.try_as_basic_value().basic().ok_or("call failed")?;
        let pred = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let is_true = self
            .builder
            .build_int_compare(IntPredicate::NE, pred, i64.const_int(0, false), "is_true")
            .map_err(llvm_err)?;
        let one_or_zero = self
            .builder
            .build_int_z_extend(is_true, i64, "one_or_zero")
            .map_err(llvm_err)?;
        let cur = self
            .builder
            .build_load(i64, cnt_a, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let inc = self
            .builder
            .build_int_add(cur, one_or_zero, "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(cnt_a, inc).map_err(llvm_err)?;
        let ni = self
            .builder
            .build_int_add(iv, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_a, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(hdr);
        self.builder.position_at_end(ext);
        let result = self
            .builder
            .build_load(i64, cnt_a, "result")
            .map_err(llvm_err)?;
        Ok(TypedValue::Int(result.into_int_value()))
    }

    /// Helper: extract (fn_ptr, list_ptr) from args for callback-based list functions
    pub(super) fn extract_callback_args(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
        expected_args: usize,
        name: &str,
    ) -> Result<(PointerValue<'ctx>, PointerValue<'ctx>), String> {
        let (fn_expr, list_expr) = if let Some(lam) = trailing {
            if args.len() != expected_args {
                return Err(format!(
                    "{} with trailing lambda expects {} argument(s) (list)",
                    name, expected_args
                ));
            }
            let lv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv)
        } else if args.len() == expected_args + 1 {
            let fv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[expected_args])?;
            (fv, lv)
        } else {
            return Err(format!(
                "{} expects {} argument(s) (fn, list)",
                name,
                expected_args + 1
            ));
        };
        let fn_ptr = match fn_expr {
            TypedValue::Fn(p, _) => p,
            _ => return Err(format!("{}: first argument must be a function", name)),
        };
        let list_ptr = match list_expr {
            TypedValue::List(p) => p,
            _ => return Err(format!("{}: last argument must be a list", name)),
        };
        Ok((fn_ptr, list_ptr))
    }

    /// Helper: extract (fn_ptr, list_ptr, init_i64) for foldRight
    pub(super) fn extract_fold_right_args(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<(PointerValue<'ctx>, PointerValue<'ctx>, IntValue<'ctx>), String> {
        let (fn_expr, list_expr, init_expr) = if let Some(lam) = trailing {
            if args.len() != 2 {
                return Err(
                    "foldRight with trailing lambda expects 2 arguments (init, list)".to_string(),
                );
            }
            let iv = self.compile_expr(&args[0])?;
            let lv = self.compile_expr(&args[1])?;
            let fv = self.compile_expr(lam)?;
            (fv, lv, iv)
        } else if args.len() == 3 {
            let fv = self.compile_expr(&args[0])?;
            let iv = self.compile_expr(&args[1])?;
            let lv = self.compile_expr(&args[2])?;
            (fv, lv, iv)
        } else {
            return Err("foldRight expects 3 arguments (fn, init, list)".to_string());
        };
        let fn_ptr = match fn_expr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("foldRight: first argument must be a function".to_string()),
        };
        let list_ptr = match list_expr {
            TypedValue::List(p) => p,
            _ => return Err("foldRight: last argument must be a list".to_string()),
        };
        let init_val = match init_expr {
            TypedValue::Int(v) => v,
            _ => return Err("foldRight: init must be an integer".to_string()),
        };
        Ok((fn_ptr, list_ptr, init_val))
    }

    /// Callback-based map functions: mapFilter, mapMapValues, mapFold
    pub(super) fn builtin_callback_map(
        &mut self,
        name: &str,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "mapFilter" => self.builtin_map_filter(args, trailing),
            "mapMapValues" => self.builtin_map_map_values(args, trailing),
            "mapFold" => self.builtin_map_fold(args, trailing),
            _ => Err(format!("Unknown callback map builtin: {}", name)),
        }
    }

    /// mapFilter(map, predicate) or mapFilter(predicate, map) or mapFilter(map) { k, v -> ... }
    /// Predicate takes (key_tag, val_tag) -> Bool (fat {i64,ptr} with tag=1 true, 0 false)
    pub(super) fn builtin_map_filter(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, map_ptr) = if let Some(lam) = trailing {
            if args.len() != 1 {
                return Err("mapFilter with trailing lambda expects 1 argument (map)".to_string());
            }
            let mv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, mv)
        } else if args.len() == 2 {
            // Could be mapFilter(map, fn) or mapFilter(fn, map) - check types
            let a0 = self.compile_expr(&args[0])?;
            let a1 = self.compile_expr(&args[1])?;
            if matches!(a0, TypedValue::Map(_)) {
                (a1, a0)
            } else {
                (a0, a1)
            }
        } else {
            return Err("mapFilter expects 2 arguments (map, predicate)".to_string());
        };

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("mapFilter: predicate must be a function".to_string()),
        };
        let map_ptr = match map_ptr {
            TypedValue::Map(p) => p,
            _ => return Err("mapFilter: first argument must be a map".to_string()),
        };

        let map_struct = self.load_list(map_ptr)?;
        let input_len = self.list_len_val(map_struct)?;
        let data_ptr = self.list_data_ptr(map_struct)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile mapFilter outside function")?;

        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;

        // Create new empty map (use input_len as capacity)
        let cc = self.call_rt("action_list_create", &[input_len.into()])?;
        let new_map_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "mf_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_map_bv)
            .map_err(llvm_err)?;

        let i_alloca = self.builder.build_alloca(i64, "mf_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_header = self.context.append_basic_block(current_fn, "mf_hdr");
        let loop_body = self.context.append_basic_block(current_fn, "mf_bdy");
        let loop_insert = self.context.append_basic_block(current_fn, "mf_ins");
        let loop_next = self.context.append_basic_block(current_fn, "mf_nxt");
        let loop_exit = self.context.append_basic_block(current_fn, "mf_ext");

        let _ = self.builder.build_unconditional_branch(loop_header);

        // Header: check i < len
        self.builder.position_at_end(loop_header);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, input_len, "mf_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        // Body: read entry, call predicate
        self.builder.position_at_end(loop_body);
        let off = self
            .builder
            .build_int_mul(i_val, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let di64 = self
            .builder
            .build_pointer_cast(data_ptr, ptr, "di64")
            .map_err(llvm_err)?;

        let kt_ptr = unsafe {
            self.builder
                .build_gep(i64, di64, &[off], "kt_ptr")
                .map_err(llvm_err)
        }?;
        let kt = self
            .builder
            .build_load(i64, kt_ptr, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let off1 = self
            .builder
            .build_int_add(off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let kp_ptr = unsafe {
            self.builder
                .build_gep(i64, di64, &[off1], "kp_ptr")
                .map_err(llvm_err)
        }?;
        let kp = self
            .builder
            .build_load(i64, kp_ptr, "kp")
            .map_err(llvm_err)?
            .into_int_value();
        let off2 = self
            .builder
            .build_int_add(off, i64.const_int(2, false), "off2")
            .map_err(llvm_err)?;
        let vt_ptr = unsafe {
            self.builder
                .build_gep(i64, di64, &[off2], "vt_ptr")
                .map_err(llvm_err)
        }?;
        let vt = self
            .builder
            .build_load(i64, vt_ptr, "vt")
            .map_err(llvm_err)?
            .into_int_value();
        let off3 = self
            .builder
            .build_int_add(off, i64.const_int(3, false), "off3")
            .map_err(llvm_err)?;
        let vp_ptr = unsafe {
            self.builder
                .build_gep(i64, di64, &[off3], "vp_ptr")
                .map_err(llvm_err)
        }?;
        let vp = self
            .builder
            .build_load(i64, vp_ptr, "vp")
            .map_err(llvm_err)?
            .into_int_value();

        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[kt.into(), vt.into()], "mf_call")
            .map_err(llvm_err)?;
        let pred_bv = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("mf call failed")?;
        let pred_tag = if pred_bv.is_struct_value() {
            self.builder
                .build_extract_value(pred_bv.into_struct_value(), 0, "pred")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            pred_bv.into_int_value()
        };
        let keep = self
            .builder
            .build_int_compare(IntPredicate::NE, pred_tag, i64.const_int(0, false), "keep")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(keep, loop_insert, loop_next);

        // Insert: add entry to result map, then go to next
        self.builder.position_at_end(loop_insert);
        let key_undef = str_ty.get_undef();
        let key1 = self
            .builder
            .build_insert_value(key_undef, kt, 0, "key1")
            .map_err(llvm_err)?;
        let kp_val = self
            .builder
            .build_int_to_ptr(kp, ptr, "kp_val")
            .map_err(llvm_err)?;
        let key_fat = self
            .builder
            .build_insert_value(key1, kp_val, 1, "key_fat")
            .map_err(llvm_err)?;
        let val_undef = str_ty.get_undef();
        let val1 = self
            .builder
            .build_insert_value(val_undef, vt, 0, "val1")
            .map_err(llvm_err)?;
        let vp_val = self
            .builder
            .build_int_to_ptr(vp, ptr, "vp_val")
            .map_err(llvm_err)?;
        let val_fat = self
            .builder
            .build_insert_value(val1, vp_val, 1, "val_fat")
            .map_err(llvm_err)?;

        let cur_map = self
            .builder
            .build_load(self.list_type, result_alloca, "cur_map")
            .map_err(llvm_err)?
            .into_struct_value();
        let ins_cc = self.call_rt(
            "action_map_insert",
            &[
                cur_map.into(),
                key_fat.as_basic_value_enum().into(),
                val_fat.as_basic_value_enum().into(),
            ],
        )?;
        let new_map = ins_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_insert failed")?;
        self.builder
            .build_store(result_alloca, new_map)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_next);

        // Next: increment i, go back to header
        self.builder.position_at_end(loop_next);
        let ni = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_header);

        // Exit
        self.builder.position_at_end(loop_exit);
        Ok(TypedValue::Map(result_alloca))
    }

    /// mapMapValues(map, transform) or mapMapValues(transform, map) or mapMapValues(map) { v -> ... }
    /// Transform takes val_tag -> new_val (fat {i64, ptr})
    pub(super) fn builtin_map_map_values(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, map_ptr) = if let Some(lam) = trailing {
            if args.len() != 1 {
                return Err(
                    "mapMapValues with trailing lambda expects 1 argument (map)".to_string()
                );
            }
            let mv = self.compile_expr(&args[0])?;
            let fv = self.compile_expr(lam)?;
            (fv, mv)
        } else if args.len() == 2 {
            let a0 = self.compile_expr(&args[0])?;
            let a1 = self.compile_expr(&args[1])?;
            if matches!(a0, TypedValue::Map(_)) {
                (a1, a0)
            } else {
                (a0, a1)
            }
        } else {
            return Err("mapMapValues expects 2 arguments (map, transform)".to_string());
        };

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("mapMapValues: transform must be a function".to_string()),
        };
        let map_ptr = match map_ptr {
            TypedValue::Map(p) => p,
            _ => return Err("mapMapValues: first argument must be a map".to_string()),
        };

        let map_struct = self.load_list(map_ptr)?;
        let input_len = self.list_len_val(map_struct)?;
        let data_ptr = self.list_data_ptr(map_struct)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile mapMapValues outside function")?;

        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;

        // Create new empty map for transformed values
        let cap = self
            .builder
            .build_int_add(input_len, i64.const_int(4, false), "mmv_cap")
            .map_err(llvm_err)?;
        let cc = self.call_rt("action_list_create", &[cap.into()])?;
        let new_map_bv = cc
            .try_as_basic_value()
            .basic()
            .ok_or("list_create failed")?;
        let result_alloca = self
            .builder
            .build_alloca(self.list_type, "mmv_result")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_map_bv)
            .map_err(llvm_err)?;

        let i_alloca = self.builder.build_alloca(i64, "mmv_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_header = self.context.append_basic_block(current_fn, "mmv_hdr");
        let loop_body = self.context.append_basic_block(current_fn, "mmv_bdy");
        let loop_exit = self.context.append_basic_block(current_fn, "mmv_ext");

        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_header);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, input_len, "mmv_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        self.builder.position_at_end(loop_body);
        let off = self
            .builder
            .build_int_mul(i_val, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let di64 = self
            .builder
            .build_pointer_cast(data_ptr, ptr, "di64")
            .map_err(llvm_err)?;

        // Read key tag, key ptr
        let kt_ptr = unsafe {
            self.builder
                .build_gep(i64, di64, &[off], "kt_ptr")
                .map_err(llvm_err)
        }?;
        let kt = self
            .builder
            .build_load(i64, kt_ptr, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let off1 = self
            .builder
            .build_int_add(off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let kp_ptr = unsafe {
            self.builder
                .build_gep(i64, di64, &[off1], "kp_ptr")
                .map_err(llvm_err)
        }?;
        let kp = self
            .builder
            .build_load(i64, kp_ptr, "kp")
            .map_err(llvm_err)?
            .into_int_value();

        // Read val tag, val ptr
        let off2 = self
            .builder
            .build_int_add(off, i64.const_int(2, false), "off2")
            .map_err(llvm_err)?;
        let vt_ptr = unsafe {
            self.builder
                .build_gep(i64, di64, &[off2], "vt_ptr")
                .map_err(llvm_err)
        }?;
        let vt = self
            .builder
            .build_load(i64, vt_ptr, "vt")
            .map_err(llvm_err)?
            .into_int_value();

        // Call transform(val_tag) -> fat {i64, ptr} (new value)
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into()], false);
        let call_result = self
            .builder
            .build_indirect_call(fn_type, fn_ptr, &[vt.into()], "mmv_call")
            .map_err(llvm_err)?;
        let new_val_bv = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("mmv call failed")?;
        let new_val = new_val_bv.into_struct_value();
        let new_vt = self
            .builder
            .build_extract_value(new_val, 0, "new_vt")
            .map_err(llvm_err)?
            .into_int_value();
        let new_vp = self
            .builder
            .build_extract_value(new_val, 1, "new_vp")
            .map_err(llvm_err)?
            .into_pointer_value();

        // Build key fat {i64, ptr}
        let key_undef = str_ty.get_undef();
        let key1 = self
            .builder
            .build_insert_value(key_undef, kt, 0, "key1")
            .map_err(llvm_err)?;
        let kp_val = self
            .builder
            .build_int_to_ptr(kp, ptr, "kp_val")
            .map_err(llvm_err)?;
        let key_fat = self
            .builder
            .build_insert_value(key1, kp_val, 1, "key_fat")
            .map_err(llvm_err)?;

        // Build new val fat {i64, ptr}
        let val_undef = str_ty.get_undef();
        let val1 = self
            .builder
            .build_insert_value(val_undef, new_vt, 0, "val1")
            .map_err(llvm_err)?;
        let val_fat = self
            .builder
            .build_insert_value(val1, new_vp, 1, "val_fat")
            .map_err(llvm_err)?;

        let cur_map = self
            .builder
            .build_load(self.list_type, result_alloca, "cur_map")
            .map_err(llvm_err)?
            .into_struct_value();
        let ins_cc = self.call_rt(
            "action_map_insert",
            &[
                cur_map.into(),
                key_fat.as_basic_value_enum().into(),
                val_fat.as_basic_value_enum().into(),
            ],
        )?;
        let new_map = ins_cc
            .try_as_basic_value()
            .basic()
            .ok_or("map_insert failed")?;
        self.builder
            .build_store(result_alloca, new_map)
            .map_err(llvm_err)?;

        let ni = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_exit);
        Ok(TypedValue::Map(result_alloca))
    }

    /// mapFold(map, init, folder) or mapFold(init, folder, map) or mapFold(init, map) { acc, k, v -> ... }
    /// Folder takes (acc_tag, key_tag, val_tag) -> new_acc (fat {i64, ptr})
    pub(super) fn builtin_map_fold(
        &mut self,
        args: &[Expr],
        trailing: &Option<Box<Expr>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let (fn_ptr, init_val, map_ptr) = if let Some(lam) = trailing {
            if args.len() != 2 {
                return Err(
                    "mapFold with trailing lambda expects 2 arguments (map, init)".to_string(),
                );
            }
            let a0 = self.compile_expr(&args[0])?;
            let a1 = self.compile_expr(&args[1])?;
            let fv = self.compile_expr(lam)?;
            if matches!(a0, TypedValue::Map(_)) {
                (fv, a1, a0)
            } else {
                (fv, a0, a1)
            }
        } else if args.len() == 3 {
            // Could be mapFold(fn, init, map) or mapFold(init, fn, map) or mapFold(init, map, fn)
            // Try to determine by checking which arg is a map
            let a0 = self.compile_expr(&args[0])?;
            let a1 = self.compile_expr(&args[1])?;
            let a2 = self.compile_expr(&args[2])?;
            if matches!(a2, TypedValue::Map(_)) {
                // Last is map, first two are fn+init or init+fn
                if matches!(a1, TypedValue::Fn(_, _)) {
                    (a1, a0, a2) // fn, init, map
                } else {
                    (a0, a1, a2) // fn(assume a0), init(a1), map(a2)
                }
            } else if matches!(a1, TypedValue::Map(_)) {
                (a0, a2, a1) // fn(a0), init(a2), map(a1)
            } else if matches!(a0, TypedValue::Map(_)) {
                (a1, a2, a0) // fn(a1), init(a2), map(a0)
            } else {
                return Err("mapFold: one argument must be a map".to_string());
            }
        } else {
            return Err("mapFold expects 3 arguments (map, init, folder)".to_string());
        };

        let fn_ptr = match fn_ptr {
            TypedValue::Fn(p, _) => p,
            _ => return Err("mapFold: folder must be a function".to_string()),
        };
        let map_ptr = match map_ptr {
            TypedValue::Map(p) => p,
            _ => return Err("mapFold: map argument must be a map".to_string()),
        };
        let init_i64 = match init_val {
            TypedValue::Int(v) => v,
            _ => return Err("mapFold: init must be an integer".to_string()),
        };

        let map_struct = self.load_list(map_ptr)?;
        let input_len = self.list_len_val(map_struct)?;
        let data_ptr = self.list_data_ptr(map_struct)?;

        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("Cannot compile mapFold outside function")?;

        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();

        let acc_alloca = self
            .builder
            .build_alloca(i64, "mfld_acc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(acc_alloca, init_i64)
            .map_err(llvm_err)?;

        let i_alloca = self.builder.build_alloca(i64, "mfld_i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let loop_header = self.context.append_basic_block(current_fn, "mfld_hdr");
        let loop_body = self.context.append_basic_block(current_fn, "mfld_bdy");
        let loop_exit = self.context.append_basic_block(current_fn, "mfld_ext");

        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_header);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, i_val, input_len, "mfld_cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(cond, loop_body, loop_exit);

        self.builder.position_at_end(loop_body);
        let off = self
            .builder
            .build_int_mul(i_val, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let di64 = self
            .builder
            .build_pointer_cast(data_ptr, ptr, "di64")
            .map_err(llvm_err)?;

        let kt_ptr = unsafe {
            self.builder
                .build_gep(i64, di64, &[off], "kt_ptr")
                .map_err(llvm_err)
        }?;
        let kt = self
            .builder
            .build_load(i64, kt_ptr, "kt")
            .map_err(llvm_err)?
            .into_int_value();

        let off2 = self
            .builder
            .build_int_add(off, i64.const_int(2, false), "off2")
            .map_err(llvm_err)?;
        let vt_ptr = unsafe {
            self.builder
                .build_gep(i64, di64, &[off2], "vt_ptr")
                .map_err(llvm_err)
        }?;
        let vt = self
            .builder
            .build_load(i64, vt_ptr, "vt")
            .map_err(llvm_err)?
            .into_int_value();

        // Call folder(acc_tag, key_tag, val_tag) -> fat {i64, ptr} (new acc)
        let acc = self
            .builder
            .build_load(i64, acc_alloca, "acc")
            .map_err(llvm_err)?
            .into_int_value();
        let fat_ret_ty = self.string_type;
        let fn_type = fat_ret_ty.fn_type(&[i64.into(), i64.into(), i64.into()], false);
        let call_result = self
            .builder
            .build_indirect_call(
                fn_type,
                fn_ptr,
                &[acc.into(), kt.into(), vt.into()],
                "mfld_call",
            )
            .map_err(llvm_err)?;
        let new_acc_bv = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("mfld call failed")?;
        let new_acc = if new_acc_bv.is_struct_value() {
            self.builder
                .build_extract_value(new_acc_bv.into_struct_value(), 0, "mfld_val")
                .map_err(llvm_err)?
                .into_int_value()
        } else {
            new_acc_bv.into_int_value()
        };
        self.builder
            .build_store(acc_alloca, new_acc)
            .map_err(llvm_err)?;

        let ni = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(i_alloca, ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_header);

        self.builder.position_at_end(loop_exit);
        let final_acc = self
            .builder
            .build_load(i64, acc_alloca, "final_acc")
            .map_err(llvm_err)?;
        Ok(TypedValue::Int(final_acc.into_int_value()))
    }

    /// Build nullable String? ({i1, {i64, ptr}}): flag=0 valid(fat_struct), flag=1 null(undef)
    /// The fat string struct is inlined — no heap allocation needed.
    pub(super) fn build_nullable_str(
        &mut self,
        fat_alloca: PointerValue<'ctx>,
        found_flag_a: PointerValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let is_found = self
            .builder
            .build_load(self.bool_ty(), found_flag_a, "is_found")
            .map_err(llvm_err)?
            .into_int_value();
        let nullable_ty = self.get_nullable_type(self.string_type.into(), "Nullable<Str>");
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let some_bb = self.context.append_basic_block(current_fn, "nls_some");
        let none_bb = self.context.append_basic_block(current_fn, "nls_none");
        let merge_bb = self.context.append_basic_block(current_fn, "nls_merge");
        let is_found_cond = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                is_found,
                self.bool_ty().const_zero(),
                "is_found_cond",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_found_cond, some_bb, none_bb);
        // Some: {flag=0, fat_val} — value inlined
        self.builder.position_at_end(some_bb);
        let fat_val = self
            .builder
            .build_load(self.string_type, fat_alloca, "fat_val")
            .map_err(llvm_err)?
            .into_struct_value();
        let some_undef = nullable_ty.get_undef();
        let s1 = self
            .builder
            .build_insert_value(
                some_undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "s_flag",
            )
            .map_err(llvm_err)?;
        let some_val = self
            .builder
            .build_insert_value(s1, fat_val, 1, "s_val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // None: {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let none_undef = nullable_ty.get_undef();
        let none_val = self
            .builder
            .build_insert_value(
                none_undef,
                self.null_flag_ty().const_int(1, false),
                0,
                "n_flag",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(nullable_ty, "nls_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);
        let alloca = self
            .builder
            .build_alloca(nullable_ty, "nls_alloca")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
    }

    /// Build nullable Int? ({i1, i64}): flag=0 valid(val), flag=1 null(undef)
    pub(super) fn build_nullable_int(
        &mut self,
        val: IntValue<'ctx>,
        is_some: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let nullable_ty = self.get_nullable_type(self.i64_ty().into(), "Nullable<Int>");
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let some_bb = self.context.append_basic_block(current_fn, "nli_some");
        let none_bb = self.context.append_basic_block(current_fn, "nli_none");
        let merge_bb = self.context.append_basic_block(current_fn, "nli_merge");
        let is_some_cond = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                is_some,
                self.bool_ty().const_zero(),
                "is_some_cond",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_some_cond, some_bb, none_bb);
        // Some: {flag=0, val}
        self.builder.position_at_end(some_bb);
        let some_undef = nullable_ty.get_undef();
        let s1 = self
            .builder
            .build_insert_value(
                some_undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "s_flag",
            )
            .map_err(llvm_err)?;
        let some_val = self
            .builder
            .build_insert_value(s1, val, 1, "s_val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // None: {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let none_undef = nullable_ty.get_undef();
        let none_val = self
            .builder
            .build_insert_value(
                none_undef,
                self.null_flag_ty().const_int(1, false),
                0,
                "n_flag",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(nullable_ty, "nli_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);
        let alloca = self
            .builder
            .build_alloca(nullable_ty, "nli_alloca")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
    }

    /// Build nullable Float? ({i1, f64}): flag=0 valid(val), flag=1 null(undef)
    pub(super) fn build_nullable_float(
        &mut self,
        val: FloatValue<'ctx>,
        is_some: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let nullable_ty = self.get_nullable_type(self.f64_ty().into(), "Nullable<Float>");
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let some_bb = self.context.append_basic_block(current_fn, "nlf_some");
        let none_bb = self.context.append_basic_block(current_fn, "nlf_none");
        let merge_bb = self.context.append_basic_block(current_fn, "nlf_merge");
        let is_some_cond = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                is_some,
                self.bool_ty().const_zero(),
                "is_some_cond",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_some_cond, some_bb, none_bb);
        // Some: {flag=0, val}
        self.builder.position_at_end(some_bb);
        let some_undef = nullable_ty.get_undef();
        let s1 = self
            .builder
            .build_insert_value(
                some_undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "s_flag",
            )
            .map_err(llvm_err)?;
        let some_val = self
            .builder
            .build_insert_value(s1, val, 1, "s_val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // None: {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let none_undef = nullable_ty.get_undef();
        let none_val = self
            .builder
            .build_insert_value(
                none_undef,
                self.null_flag_ty().const_int(1, false),
                0,
                "n_flag",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(nullable_ty, "nlf_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);
        let alloca = self
            .builder
            .build_alloca(nullable_ty, "nlf_alloca")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
    }

    /// Build nullable List? ({i1, list_struct}): flag=0 valid(list_val), flag=1 null(undef)
    pub(super) fn build_nullable_list(
        &mut self,
        list_val: StructValue<'ctx>,
        is_empty_value: IntValue<'ctx>,
    ) -> Result<TypedValue<'ctx>, String> {
        let nullable_ty = self.get_nullable_type(self.list_type.into(), "Nullable<List>");
        let current_fn = self
            .builder
            .get_insert_block()
            .and_then(|b| b.get_parent())
            .ok_or("no function")?;
        let some_bb = self.context.append_basic_block(current_fn, "nll_some");
        let none_bb = self.context.append_basic_block(current_fn, "nll_none");
        let merge_bb = self.context.append_basic_block(current_fn, "nll_merge");
        let is_empty_cond = self
            .builder
            .build_int_compare(
                IntPredicate::NE,
                is_empty_value,
                self.bool_ty().const_zero(),
                "is_empty_cond",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_empty_cond, none_bb, some_bb);
        // Some: {flag=0, list_val} — value inlined, no heap allocation
        self.builder.position_at_end(some_bb);
        let some_undef = nullable_ty.get_undef();
        let s1 = self
            .builder
            .build_insert_value(
                some_undef,
                self.null_flag_ty().const_int(0, false),
                0,
                "s_flag",
            )
            .map_err(llvm_err)?;
        let some_val = self
            .builder
            .build_insert_value(s1, list_val, 1, "s_val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // None: {flag=1, undef}
        self.builder.position_at_end(none_bb);
        let none_undef = nullable_ty.get_undef();
        let none_val = self
            .builder
            .build_insert_value(
                none_undef,
                self.null_flag_ty().const_int(1, false),
                0,
                "n_flag",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        // Merge
        self.builder.position_at_end(merge_bb);
        let phi = self
            .builder
            .build_phi(nullable_ty, "nll_phi")
            .map_err(llvm_err)?;
        phi.add_incoming(&[(&some_val, some_bb), (&none_val, none_bb)]);
        let alloca = self
            .builder
            .build_alloca(nullable_ty, "nll_alloca")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, phi.as_basic_value())
            .map_err(llvm_err)?;
        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
    }

    /// Builtin stdlib functions: len, is_empty, append, concat
    pub(super) fn builtin_stdlib(
        &mut self,
        name: &str,
        args: &[Expr],
    ) -> Result<TypedValue<'ctx>, String> {
        match name {
            "to" => {
                if args.len() != 2 {
                    return Err("to expects 2 arguments".to_string());
                }
                self.compile_tuple(&[(None, args[0].clone()), (None, args[1].clone())])
            }
            "len" => {
                if args.len() != 1 {
                    return Err("len expects 1 argument".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                match val {
                    TypedValue::List(ptr) => {
                        let list = self.load_list(ptr)?;
                        let len = self
                            .builder
                            .build_extract_value(list, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(len))
                    }
                    TypedValue::LazyList(ptr) => {
                        let ll_sv = self
                            .builder
                            .build_load(self.lazylist_type, ptr, "len_ll")
                            .map_err(llvm_err)?
                            .into_struct_value();
                        let take_count = self
                            .builder
                            .build_extract_value(ll_sv, 3, "len_tc")
                            .map_err(llvm_err)?
                            .into_int_value();
                        // If take_count > 0, that's the length. If 0 (no step fn), it's 1.
                        // If -1 (infinite), return -1.
                        let zero = self.i64_ty().const_int(0, false);
                        let one = self.i64_ty().const_int(1, false);
                        let is_zero = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, take_count, zero, "tc_zero")
                            .map_err(llvm_err)?;
                        let result_len = self
                            .builder
                            .build_select(is_zero, one, take_count, "ll_len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result_len))
                    }
                    TypedValue::Str(ptr) => {
                        let str_val = self.load_string(ptr)?;
                        let len = self
                            .builder
                            .build_extract_value(str_val, 0, "slen")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(len))
                    }
                    TypedValue::Map(ptr) => {
                        let m = self.load_list(ptr)?;
                        let raw_len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let two = self.i64_ty().const_int(2, false);
                        let len = self
                            .builder
                            .build_int_signed_div(raw_len, two, "entries")
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Int(len))
                    }
                    TypedValue::Set(ptr) => {
                        let m = self.load_list(ptr)?;
                        let raw_len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let two = self.i64_ty().const_int(2, false);
                        let len = self
                            .builder
                            .build_int_signed_div(raw_len, two, "entries")
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Int(len))
                    }
                    _ => Err(
                        "len: argument must be a list, string, map, set, or lazy list".to_string(),
                    ),
                }
            }
            "isEmpty" => {
                if args.len() != 1 {
                    return Err("isEmpty expects 1 argument".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                let len = match val {
                    TypedValue::List(ptr) => {
                        let list = self.load_list(ptr)?;
                        self.builder
                            .build_extract_value(list, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value()
                    }
                    TypedValue::LazyList(_) => {
                        // A LazyList always has at least the head element, so never empty
                        self.i64_ty().const_int(1, false)
                    }
                    TypedValue::Str(ptr) => {
                        let str_val = self.load_string(ptr)?;
                        self.builder
                            .build_extract_value(str_val, 0, "slen")
                            .map_err(llvm_err)?
                            .into_int_value()
                    }
                    TypedValue::Map(ptr) => {
                        let m = self.load_list(ptr)?;
                        let raw_len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let two = self.i64_ty().const_int(2, false);
                        self.builder
                            .build_int_signed_div(raw_len, two, "entries")
                            .map_err(llvm_err)?
                    }
                    TypedValue::Set(ptr) => {
                        let m = self.load_list(ptr)?;
                        let raw_len = self
                            .builder
                            .build_extract_value(m, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let two = self.i64_ty().const_int(2, false);
                        self.builder
                            .build_int_signed_div(raw_len, two, "entries")
                            .map_err(llvm_err)?
                    }
                    _ => {
                        return Err(
                            "is_empty: argument must be a list, string, map, set, or lazy list"
                                .to_string(),
                        )
                    }
                };
                let zero = self.i64_ty().const_int(0, false);
                let is_empty = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, len, zero, "is_empty")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_empty))
            }
            "append" => {
                if args.len() != 2 {
                    return Err("append expects 2 arguments (list, element)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("append: first argument must be a list".to_string()),
                };
                let elem_val = self.compile_expr(&args[1])?;
                let elem_fat = self.to_fat_struct(&elem_val)?;
                let list = self.load_list(list_ptr)?;
                let cc = self.call_rt("action_list_push", &[list.into(), elem_fat.into()])?;
                let new_list = cc.try_as_basic_value().basic().ok_or("list_push failed")?;
                let alloca = self
                    .builder
                    .build_alloca(self.list_type, "appended")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(alloca, new_list)
                    .map_err(llvm_err)?;
                Ok(TypedValue::List(alloca))
            }
            "concat" => {
                if args.len() != 2 {
                    return Err("concat expects 2 arguments".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Str(p1), TypedValue::Str(p2)) => {
                        let s1 = self.load_string(*p1)?;
                        let s2 = self.load_string(*p2)?;
                        let cc = self.call_rt("action_string_concat", &[s1.into(), s2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("string_concat failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "concat_str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    (TypedValue::List(p1), TypedValue::List(p2)) => {
                        let l1 = self.load_list(*p1)?;
                        let l2 = self.load_list(*p2)?;
                        let result = self
                            .call_rt("action_list_concat", &[l1.into(), l2.into()])?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("list_concat failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "concat_list")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    (TypedValue::Map(p1), TypedValue::Map(p2)) => {
                        let m1 = self.load_list(*p1)?;
                        let m2 = self.load_list(*p2)?;
                        let result = self
                            .call_rt("action_map_union", &[m1.into(), m2.into()])?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("map_union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "concat_map")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Map(alloca))
                    }
                    (TypedValue::Set(p1), TypedValue::Set(p2)) => {
                        let s1 = self.load_list(*p1)?;
                        let s2 = self.load_list(*p2)?;
                        let result = self
                            .call_rt("action_set_union", &[s1.into(), s2.into()])?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set_union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "concat_set")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("concat: arguments must be both strings, both lists, both maps, or both sets".to_string()),
                }
            }
            "toUpper" => {
                if args.len() != 1 {
                    return Err("toUpper expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_to_upper", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("toUpper failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "upper")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("toUpper: argument must be a string".to_string()),
                }
            }
            "toLower" => {
                if args.len() != 1 {
                    return Err("toLower expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_to_lower", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("toLower failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "lower")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("toLower: argument must be a string".to_string()),
                }
            }
            "trim" => {
                if args.len() != 1 {
                    return Err("trim expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_trim", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("trim failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "trimmed")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("trim: argument must be a string".to_string()),
                }
            }
            "readLine" => {
                if !args.is_empty() {
                    return Err("readLine expects no arguments".to_string());
                }
                if self.module.get_function("action_read_line").is_none() {
                    self.emit_read_line_runtime()?;
                }
                let cc = self.call_rt("action_read_line", &[])?;
                let result_struct = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("readLine failed")?
                    .into_struct_value();
                // Extract string {i64, ptr} and success flag i1
                let str_len = self
                    .builder
                    .build_extract_value(result_struct, 0, "slen")
                    .map_err(llvm_err)?
                    .into_int_value();
                let str_ptr = self
                    .builder
                    .build_extract_value(result_struct, 1, "sptr")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                let ok = self
                    .builder
                    .build_extract_value(result_struct, 2, "ok")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Build the string fat struct and store in alloca
                let line_undef = self.string_type.get_undef();
                let line1 = self
                    .builder
                    .build_insert_value(line_undef, str_len, 0, "l_len")
                    .map_err(llvm_err)?;
                let line_val = self
                    .builder
                    .build_insert_value(line1, str_ptr, 1, "l_ptr")
                    .map_err(llvm_err)?;
                let fat_alloca = self
                    .builder
                    .build_alloca(self.string_type, "line")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(fat_alloca, line_val)
                    .map_err(llvm_err)?;
                let flag_alloca = self
                    .builder
                    .build_alloca(self.bool_ty(), "line_ok")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(flag_alloca, ok)
                    .map_err(llvm_err)?;
                self.build_nullable_str(fat_alloca, flag_alloca)
            }
            "startsWith" => {
                if args.len() != 2 {
                    return Err("startsWith expects 2 arguments".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let prefix = self.compile_expr(&args[1])?;
                match (&s, &prefix) {
                    (TypedValue::Str(sp), TypedValue::Str(pp)) => {
                        let sv = self.load_string(*sp)?;
                        let pv = self.load_string(*pp)?;
                        let cc =
                            self.call_rt("action_string_starts_with", &[sv.into(), pv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("startsWith failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("startsWith: arguments must be strings".to_string()),
                }
            }
            "endsWith" => {
                if args.len() != 2 {
                    return Err("endsWith expects 2 arguments".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let suffix = self.compile_expr(&args[1])?;
                match (&s, &suffix) {
                    (TypedValue::Str(sp), TypedValue::Str(sup)) => {
                        let sv = self.load_string(*sp)?;
                        let suv = self.load_string(*sup)?;
                        let cc =
                            self.call_rt("action_string_ends_with", &[sv.into(), suv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("endsWith failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("endsWith: arguments must be strings".to_string()),
                }
            }
            "substring" => {
                if args.len() != 3 {
                    return Err("substring expects 3 arguments (str, start, len)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let start = self.compile_expr(&args[1])?;
                let len = self.compile_expr(&args[2])?;
                match s {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let start_bv = start.to_bv().ok_or("start must be a basic value")?;
                        let len_bv = len.to_bv().ok_or("len must be a basic value")?;
                        let cc = self.call_rt(
                            "action_string_substring",
                            &[sv.into(), start_bv.into(), len_bv.into()],
                        )?;
                        let result = cc.try_as_basic_value().basic().ok_or("substring failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "substr")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("substring: first argument must be a string".to_string()),
                }
            }
            "parseInt" => {
                if args.len() != 1 {
                    return Err("parseInt expects 1 argument".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                match s {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let cc = self.call_rt("action_parse_int", &[sv.into()])?;
                        let result_struct = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("parseInt failed")?
                            .into_struct_value();
                        let val = self
                            .builder
                            .build_extract_value(result_struct, 0, "val")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let ok = self
                            .builder
                            .build_extract_value(result_struct, 1, "ok")
                            .map_err(llvm_err)?
                            .into_int_value();
                        self.build_nullable_int(val, ok)
                    }
                    _ => Err("parseInt: argument must be a string".to_string()),
                }
            }
            "readFile" => {
                if args.len() != 1 {
                    return Err("readFile expects 1 argument (path)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                match path {
                    TypedValue::Str(pp) => {
                        let pv = self.load_string(pp)?;
                        let cc = self.call_rt("action_read_file", &[pv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("readFile failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "content")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("readFile: argument must be a string".to_string()),
                }
            }
            "writeFile" => {
                if args.len() != 2 {
                    return Err("writeFile expects 2 arguments (path, content)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                let content = self.compile_expr(&args[1])?;
                match (&path, &content) {
                    (TypedValue::Str(pp), TypedValue::Str(cp)) => {
                        let pv = self.load_string(*pp)?;
                        let cv = self.load_string(*cp)?;
                        let cc = self.call_rt("action_write_file", &[pv.into(), cv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("writeFile failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("writeFile: arguments must be strings".to_string()),
                }
            }
            "appendFile" => {
                if args.len() != 2 {
                    return Err("appendFile expects 2 arguments (path, content)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                let content = self.compile_expr(&args[1])?;
                match (&path, &content) {
                    (TypedValue::Str(pp), TypedValue::Str(cp)) => {
                        let pv = self.load_string(*pp)?;
                        let cv = self.load_string(*cp)?;
                        let cc = self.call_rt("action_file_append", &[pv.into(), cv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("appendFile failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("appendFile: arguments must be strings".to_string()),
                }
            }
            "exists" => {
                if args.len() != 1 {
                    return Err("exists expects 1 argument (path)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                match path {
                    TypedValue::Str(pp) => {
                        let pv = self.load_string(pp)?;
                        let cc = self.call_rt("action_file_exists", &[pv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("exists failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("exists: argument must be a string".to_string()),
                }
            }
            "deleteFile" => {
                if args.len() != 1 {
                    return Err("deleteFile expects 1 argument (path)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                match path {
                    TypedValue::Str(pp) => {
                        let pv = self.load_string(pp)?;
                        let cc = self.call_rt("action_file_delete", &[pv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("deleteFile failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("deleteFile: argument must be a string".to_string()),
                }
            }
            // ---- Streaming File I/O ----
            "openFile" => {
                if args.len() != 2 {
                    return Err("openFile expects 2 arguments (path, mode)".to_string());
                }
                let path = self.compile_expr(&args[0])?;
                let mode = self.compile_expr(&args[1])?;
                match (&path, &mode) {
                    (TypedValue::Str(pp), TypedValue::Str(mp)) => {
                        let path_s = self.load_string(*pp)?;
                        let mode_s = self.load_string(*mp)?;
                        let cc =
                            self.call_rt("action_file_open", &[path_s.into(), mode_s.into()])?;
                        let file_ptr = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("openFile failed")?
                            .into_pointer_value();
                        Ok(TypedValue::FileHandle(file_ptr))
                    }
                    _ => Err("openFile: arguments must be strings (path, mode)".to_string()),
                }
            }
            "closeFile" => {
                if args.len() != 1 {
                    return Err("closeFile expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_close", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("closeFile failed")?
                            .into_int_value();
                        let ok = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                result,
                                self.i32_ty().const_int(0, false),
                                "ok",
                            )
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Bool(ok))
                    }
                    _ => Err("closeFile: argument must be a FileHandle".to_string()),
                }
            }
            "isEof" => {
                if args.len() != 1 {
                    return Err("isEof expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_eof", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("isEof failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("isEof: argument must be a FileHandle".to_string()),
                }
            }
            "fileReadLine" => {
                if args.len() != 1 {
                    return Err("fileReadLine expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_read_line", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileReadLine failed")?
                            .into_struct_value();
                        // Build string from len+ptr
                        let len = self
                            .builder
                            .build_extract_value(result, 0, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let data = self
                            .builder
                            .build_extract_value(result, 1, "data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let str_struct =
                            self.call_rt("action_string_create", &[data.into(), len.into()])?;
                        let str_val = str_struct
                            .try_as_basic_value()
                            .basic()
                            .ok_or("string_create failed")?;
                        let str_alloca = self
                            .builder
                            .build_alloca(self.string_type, "str_tmp")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(str_alloca, str_val)
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Str(str_alloca))
                    }
                    _ => Err("fileReadLine: argument must be a FileHandle".to_string()),
                }
            }
            "fileReadBytes" => {
                if args.len() != 2 {
                    return Err("fileReadBytes expects 2 arguments (file, size)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                let size = self.compile_expr(&args[1])?;
                match (&file, &size) {
                    (TypedValue::FileHandle(p), TypedValue::Int(s)) => {
                        let cc =
                            self.call_rt("action_file_read_bytes", &[(*p).into(), (*s).into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileReadBytes failed")?
                            .into_struct_value();
                        let len = self
                            .builder
                            .build_extract_value(result, 0, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let data = self
                            .builder
                            .build_extract_value(result, 1, "data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let str_struct =
                            self.call_rt("action_string_create", &[data.into(), len.into()])?;
                        let str_val = str_struct
                            .try_as_basic_value()
                            .basic()
                            .ok_or("string_create failed")?;
                        let str_alloca = self
                            .builder
                            .build_alloca(self.string_type, "rb_tmp")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(str_alloca, str_val)
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Str(str_alloca))
                    }
                    _ => Err("fileReadBytes: arguments must be (FileHandle, Int)".to_string()),
                }
            }
            "fileWrite" => {
                if args.len() != 2 {
                    return Err("fileWrite expects 2 arguments (file, data)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                let data = self.compile_expr(&args[1])?;
                match (&file, &data) {
                    (TypedValue::FileHandle(fp), TypedValue::Str(dp)) => {
                        let data_s = self.load_string(*dp)?;
                        let data_len = self
                            .builder
                            .build_extract_value(data_s, 0, "dlen")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let data_ptr = self
                            .builder
                            .build_extract_value(data_s, 1, "dptr")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let cc = self.call_rt(
                            "action_file_write_bytes",
                            &[(*fp).into(), data_ptr.into(), data_len.into()],
                        )?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileWrite failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("fileWrite: arguments must be (FileHandle, String)".to_string()),
                }
            }
            "fileWriteLine" => {
                if args.len() != 2 {
                    return Err("fileWriteLine expects 2 arguments (file, data)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                let data = self.compile_expr(&args[1])?;
                match (&file, &data) {
                    (TypedValue::FileHandle(fp), TypedValue::Str(dp)) => {
                        let data_s = self.load_string(*dp)?;
                        let data_len = self
                            .builder
                            .build_extract_value(data_s, 0, "dlen")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let data_ptr = self
                            .builder
                            .build_extract_value(data_s, 1, "dptr")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        // Write data first
                        let cc1 = self.call_rt(
                            "action_file_write_bytes",
                            &[(*fp).into(), data_ptr.into(), data_len.into()],
                        )?;
                        // Write newline: create a buffer with "\n\0"
                        let malloc_fn = self.module.get_function("malloc").unwrap();
                        let nl_len = self.i64_ty().const_int(1, false);
                        let nl_buf = self
                            .builder
                            .build_call(
                                malloc_fn,
                                &[self.i64_ty().const_int(2, false).into()],
                                "nl_buf",
                            )
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_pointer_value();
                        self.builder
                            .build_store(nl_buf, self.context.i8_type().const_int(10, false))
                            .map_err(llvm_err)?;
                        let _ = self.call_rt(
                            "action_file_write_bytes",
                            &[(*fp).into(), nl_buf.into(), nl_len.into()],
                        )?;
                        let result = cc1
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileWriteLine failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("fileWriteLine: arguments must be (FileHandle, String)".to_string()),
                }
            }
            "fileFlush" => {
                if args.len() != 1 {
                    return Err("fileFlush expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_flush", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileFlush failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("fileFlush: argument must be a FileHandle".to_string()),
                }
            }
            "fileSeek" => {
                if args.len() != 3 {
                    return Err("fileSeek expects 3 arguments (file, offset, whence)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                let offset = self.compile_expr(&args[1])?;
                let whence = self.compile_expr(&args[2])?;
                match (&file, &offset, &whence) {
                    (TypedValue::FileHandle(p), TypedValue::Int(o), TypedValue::Int(w)) => {
                        let w32 = self
                            .builder
                            .build_int_truncate(*w, self.i32_ty(), "w32")
                            .map_err(llvm_err)?;
                        let cc = self
                            .call_rt("action_file_seek", &[(*p).into(), (*o).into(), w32.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileSeek failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("fileSeek: arguments must be (FileHandle, Int, Int)".to_string()),
                }
            }
            "fileTell" => {
                if args.len() != 1 {
                    return Err("fileTell expects 1 argument (file)".to_string());
                }
                let file = self.compile_expr(&args[0])?;
                match file {
                    TypedValue::FileHandle(p) => {
                        let cc = self.call_rt("action_file_tell", &[p.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("fileTell failed")?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    _ => Err("fileTell: argument must be a FileHandle".to_string()),
                }
            }
            "randInt" => {
                if args.len() != 2 {
                    return Err("randInt expects 2 arguments (min, max)".to_string());
                }
                let min = self.compile_expr(&args[0])?;
                let max = self.compile_expr(&args[1])?;
                let min_bv = min.to_bv().ok_or("min must be a basic value")?;
                let max_bv = max.to_bv().ok_or("max must be a basic value")?;
                let cc = self.call_rt("action_rand_int", &[min_bv.into(), max_bv.into()])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("randInt failed")?
                    .into_int_value();
                Ok(TypedValue::Int(result))
            }
            "randFloat" => {
                if !args.is_empty() {
                    return Err("randFloat expects no arguments".to_string());
                }
                let cc = self.call_rt("action_rand_float", &[])?;
                let result = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("randFloat failed")?
                    .into_float_value();
                Ok(TypedValue::Float(result))
            }
            "split" => {
                if args.len() != 2 {
                    return Err("split expects 2 arguments (string, delimiter)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let delim = self.compile_expr(&args[1])?;
                match (&s, &delim) {
                    (TypedValue::Str(sp), TypedValue::Str(dp)) => {
                        let sv = self.load_string(*sp)?;
                        let dv = self.load_string(*dp)?;
                        let cc = self.call_rt("action_string_split", &[sv.into(), dv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("split failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "split_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("split: arguments must be strings".to_string()),
                }
            }
            "join" => {
                if args.len() != 2 {
                    return Err("join expects 2 arguments (list, delimiter)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let delim = self.compile_expr(&args[1])?;
                match (&list_val, &delim) {
                    (TypedValue::List(lp), TypedValue::Str(dp)) => {
                        let lv = self.load_list(*lp)?;
                        let dv = self.load_string(*dp)?;
                        let cc = self.call_rt("action_string_join", &[lv.into(), dv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("join failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "join_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("join: first argument must be a list, second a string".to_string()),
                }
            }
            "replace" => {
                if args.len() != 3 {
                    return Err("replace expects 3 arguments (string, from, to)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let from = self.compile_expr(&args[1])?;
                let to = self.compile_expr(&args[2])?;
                match (&s, &from, &to) {
                    (TypedValue::Str(sp), TypedValue::Str(fp), TypedValue::Str(tp)) => {
                        let sv = self.load_string(*sp)?;
                        let fv = self.load_string(*fp)?;
                        let tv = self.load_string(*tp)?;
                        let cc = self
                            .call_rt("action_string_replace", &[sv.into(), fv.into(), tv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("replace failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "replace_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("replace: arguments must be strings".to_string()),
                }
            }
            "abs" => {
                if args.len() != 1 {
                    return Err("abs expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => {
                        let zero = self.i64_ty().const_int(0, false);
                        let neg = self.builder.build_int_neg(iv, "neg").map_err(llvm_err)?;
                        let is_neg = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, iv, zero, "is_neg")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_neg, neg, iv, "abs_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    TypedValue::Float(fv) => {
                        let zero = self.f64_ty().const_float(0.0);
                        let neg = self.builder.build_float_neg(fv, "neg").map_err(llvm_err)?;
                        let is_neg = self
                            .builder
                            .build_float_compare(FloatPredicate::OLT, fv, zero, "is_neg")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_neg, neg, fv, "fabs_result")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("abs: argument must be Int or Float".to_string()),
                }
            }
            "min" => {
                if args.len() != 2 {
                    return Err("min expects 2 arguments".to_string());
                }
                let a = self.compile_expr(&args[0])?;
                let b = self.compile_expr(&args[1])?;
                match (&a, &b) {
                    (TypedValue::Int(av), TypedValue::Int(bv)) => {
                        let is_lt = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, *av, *bv, "is_lt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_lt, *av, *bv, "min_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    (TypedValue::Float(av), TypedValue::Float(bv)) => {
                        let is_lt = self
                            .builder
                            .build_float_compare(FloatPredicate::OLT, *av, *bv, "is_lt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_lt, *av, *bv, "fmin_result")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("min: arguments must be both Int or both Float".to_string()),
                }
            }
            "max" => {
                if args.len() != 2 {
                    return Err("max expects 2 arguments".to_string());
                }
                let a = self.compile_expr(&args[0])?;
                let b = self.compile_expr(&args[1])?;
                match (&a, &b) {
                    (TypedValue::Int(av), TypedValue::Int(bv)) => {
                        let is_gt = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, *av, *bv, "is_gt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_gt, *av, *bv, "max_result")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(result))
                    }
                    (TypedValue::Float(av), TypedValue::Float(bv)) => {
                        let is_gt = self
                            .builder
                            .build_float_compare(FloatPredicate::OGT, *av, *bv, "is_gt")
                            .map_err(llvm_err)?;
                        let result = self
                            .builder
                            .build_select(is_gt, *av, *bv, "fmax_result")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("max: arguments must be both Int or both Float".to_string()),
                }
            }
            "sqrt" => {
                if args.len() != 1 {
                    return Err("sqrt expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let sqrt_fn = self.module.get_function("sqrt").unwrap();
                let r = self
                    .builder
                    .build_call(sqrt_fn, &[fv.into()], "sqrt")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("sqrt failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "cbrt" => {
                if args.len() != 1 {
                    return Err("cbrt expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let cbrt_fn = self.module.get_function("cbrt").unwrap();
                let r = self
                    .builder
                    .build_call(cbrt_fn, &[fv.into()], "cbrt")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("cbrt failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "sin" => {
                if args.len() != 1 {
                    return Err("sin expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("sin").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "sin")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("sin failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "cos" => {
                if args.len() != 1 {
                    return Err("cos expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("cos").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "cos")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("cos failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "tan" => {
                if args.len() != 1 {
                    return Err("tan expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("tan").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "tan")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("tan failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "asin" => {
                if args.len() != 1 {
                    return Err("asin expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("asin").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "asin")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("asin failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "acos" => {
                if args.len() != 1 {
                    return Err("acos expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("acos").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "acos")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("acos failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "atan" => {
                if args.len() != 1 {
                    return Err("atan expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("atan").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "atan")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("atan failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "atan2" => {
                if args.len() != 2 {
                    return Err("atan2 expects 2 arguments".to_string());
                }
                let y = self.compile_expr(&args[0])?;
                let x = self.compile_expr(&args[1])?;
                let yv = self.typed_to_float(&y)?;
                let xv = self.typed_to_float(&x)?;
                let f = self.module.get_function("atan2").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[yv.into(), xv.into()], "atan2")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("atan2 failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "log" => {
                if args.len() != 1 {
                    return Err("log expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("log").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "log")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("log failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "log2" => {
                if args.len() != 1 {
                    return Err("log2 expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("log2").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "log2")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("log2 failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "log10" => {
                if args.len() != 1 {
                    return Err("log10 expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("log10").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "log10")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("log10 failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "exp" => {
                if args.len() != 1 {
                    return Err("exp expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("exp").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "exp")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("exp failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "floor" => {
                if args.len() != 1 {
                    return Err("floor expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("floor").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "floor")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("floor failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "ceil" => {
                if args.len() != 1 {
                    return Err("ceil expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("ceil").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "ceil")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("ceil failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "round" => {
                if args.len() != 1 {
                    return Err("round expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let f = self.module.get_function("round").unwrap();
                let r = self
                    .builder
                    .build_call(f, &[fv.into()], "round")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .basic()
                    .ok_or("round failed")?
                    .into_float_value();
                Ok(TypedValue::Float(r))
            }
            "pi" => {
                if !args.is_empty() {
                    return Err("pi expects no arguments".to_string());
                }
                let pi_val = self.f64_ty().const_float(std::f64::consts::PI);
                Ok(TypedValue::Float(pi_val))
            }
            "e" => {
                if !args.is_empty() {
                    return Err("e expects no arguments".to_string());
                }
                let e_val = self.f64_ty().const_float(std::f64::consts::E);
                Ok(TypedValue::Float(e_val))
            }
            "clamp" => {
                if args.len() != 3 {
                    return Err("clamp expects 3 arguments (value, min, max)".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                let min = self.compile_expr(&args[1])?;
                let max = self.compile_expr(&args[2])?;
                match (&val, &min, &max) {
                    (TypedValue::Int(vv), TypedValue::Int(mn), TypedValue::Int(mx)) => {
                        let lt_min = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, *vv, *mn, "lt_min")
                            .map_err(llvm_err)?;
                        let r1 = self
                            .builder
                            .build_select(lt_min, *mn, *vv, "clamp1")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let gt_max = self
                            .builder
                            .build_int_compare(IntPredicate::SGT, r1, *mx, "gt_max")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_select(gt_max, *mx, r1, "clamp2")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(r2))
                    }
                    (TypedValue::Float(vv), TypedValue::Float(mn), TypedValue::Float(mx)) => {
                        let lt_min = self
                            .builder
                            .build_float_compare(FloatPredicate::OLT, *vv, *mn, "lt_min")
                            .map_err(llvm_err)?;
                        let r1 = self
                            .builder
                            .build_select(lt_min, *mn, *vv, "clamp1")
                            .map_err(llvm_err)?
                            .into_float_value();
                        let gt_max = self
                            .builder
                            .build_float_compare(FloatPredicate::OGT, r1, *mx, "gt_max")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_select(gt_max, *mx, r1, "clamp2")
                            .map_err(llvm_err)?
                            .into_float_value();
                        Ok(TypedValue::Float(r2))
                    }
                    _ => Err("clamp: arguments must be all Int or all Float".to_string()),
                }
            }
            "isNaN" => {
                if args.len() != 1 {
                    return Err("isNaN expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let is_nan = self
                    .builder
                    .build_float_compare(FloatPredicate::UNO, fv, fv, "isNaN")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_nan))
            }
            "isInfinite" => {
                if args.len() != 1 {
                    return Err("isInfinite expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let fv = self.typed_to_float(&v)?;
                let inf = self.f64_ty().const_float(f64::INFINITY);
                let is_pos_inf = self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, fv, inf, "is_pos_inf")
                    .map_err(llvm_err)?;
                let neg_inf = self.f64_ty().const_float(f64::NEG_INFINITY);
                let is_neg_inf = self
                    .builder
                    .build_float_compare(FloatPredicate::OEQ, fv, neg_inf, "is_neg_inf")
                    .map_err(llvm_err)?;
                let is_inf = self
                    .builder
                    .build_or(is_pos_inf, is_neg_inf, "is_inf")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_inf))
            }
            "panic" => {
                if args.len() != 1 {
                    return Err("panic expects 1 argument (message)".to_string());
                }
                let msg = self.compile_expr(&args[0])?;
                match msg {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let _ = self.call_rt("action_print_string", &[sv.into()])?;
                        let _ = self.call_rt("action_println", &[])?;
                        // Call exit(1)
                        let exit_fn = self.module.get_function("exit");
                        if exit_fn.is_none() {
                            let _ = self.module.add_function(
                                "exit",
                                self.void_ty().fn_type(&[self.i32_ty().into()], false),
                                None,
                            );
                        }
                        let exit_fn = self.module.get_function("exit").unwrap();
                        let one = self.i32_ty().const_int(1, false);
                        let _ = self
                            .builder
                            .build_call(exit_fn, &[one.into()], "")
                            .map_err(llvm_err)?;
                        self.builder.build_unreachable().map_err(llvm_err)?;
                        Ok(TypedValue::Unit)
                    }
                    _ => Err("panic: argument must be a string".to_string()),
                }
            }
            "assert" => {
                if args.len() != 2 {
                    return Err("assert expects 2 arguments (condition, message)".to_string());
                }
                let cond = self.compile_expr(&args[0])?;
                let cond_bool = match cond {
                    TypedValue::Bool(b) => b,
                    _ => return Err("assert: first argument must be a Bool".to_string()),
                };
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .unwrap()
                    .get_parent()
                    .unwrap();
                let assert_ok_bb = self.context.append_basic_block(current_fn, "assert_ok");
                let assert_fail_bb = self.context.append_basic_block(current_fn, "assert_fail");
                let assert_merge_bb = self.context.append_basic_block(current_fn, "assert_merge");
                let _ =
                    self.builder
                        .build_conditional_branch(cond_bool, assert_ok_bb, assert_fail_bb);
                // Fail: print message and exit
                self.builder.position_at_end(assert_fail_bb);
                let msg = self.compile_expr(&args[1])?;
                match msg {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let prefix = self.compile_string_literal("Assertion failed: ")?;
                        let prefix_sv = match prefix {
                            TypedValue::Str(pp) => self.load_string(pp)?,
                            _ => return Err("internal error".to_string()),
                        };
                        let cc =
                            self.call_rt("action_string_concat", &[prefix_sv.into(), sv.into()])?;
                        let full = cc.try_as_basic_value().basic().ok_or("concat failed")?;
                        let _ = self.call_rt("action_print_string", &[full.into()])?;
                        let _ = self.call_rt("action_println", &[])?;
                        let exit_fn = self.module.get_function("exit");
                        if exit_fn.is_none() {
                            let _ = self.module.add_function(
                                "exit",
                                self.void_ty().fn_type(&[self.i32_ty().into()], false),
                                None,
                            );
                        }
                        let exit_fn = self.module.get_function("exit").unwrap();
                        let _ = self
                            .builder
                            .build_call(exit_fn, &[self.i32_ty().const_int(1, false).into()], "")
                            .map_err(llvm_err)?;
                        self.builder.build_unreachable().map_err(llvm_err)?;
                    }
                    _ => return Err("assert: second argument must be a string".to_string()),
                }
                // Ok: continue
                self.builder.position_at_end(assert_ok_bb);
                let _ = self.builder.build_unconditional_branch(assert_merge_bb);
                self.builder.position_at_end(assert_merge_bb);
                Ok(TypedValue::Unit)
            }
            "toString" => {
                if args.len() != 1 {
                    return Err("toString expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => {
                        let cc = self.call_rt("action_int_to_string", &[iv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("intToString failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    TypedValue::Float(fv) => {
                        let cc = self.call_rt("action_float_to_string", &[fv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("floatToString failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "fstr")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    TypedValue::Bool(bv) => {
                        let true_lit = self.compile_string_literal("true")?;
                        let false_lit = self.compile_string_literal("false")?;
                        let true_sv = match true_lit {
                            TypedValue::Str(tp) => self.load_string(tp)?,
                            _ => return Err("internal".to_string()),
                        };
                        let false_sv = match false_lit {
                            TypedValue::Str(fp) => self.load_string(fp)?,
                            _ => return Err("internal".to_string()),
                        };
                        let result = self
                            .builder
                            .build_select(bv, true_sv, false_sv, "bool_str")
                            .map_err(llvm_err)?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "bstr")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, result.into_struct_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "idstr")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, sv).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => {
                        let placeholder = self.compile_string_literal("[Object]")?;
                        match placeholder {
                            TypedValue::Str(pp) => {
                                let pv = self.load_string(pp)?;
                                let alloca = self
                                    .builder
                                    .build_alloca(self.string_type, "objstr")
                                    .map_err(llvm_err)?;
                                self.builder.build_store(alloca, pv).map_err(llvm_err)?;
                                Ok(TypedValue::Str(alloca))
                            }
                            _ => Err("internal error".to_string()),
                        }
                    }
                }
            }
            "head" => {
                if args.len() != 1 {
                    return Err("head expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) | TypedValue::LazyList(lp) => {
                        let list_val = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(list_val, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let zero = self.i64_ty().const_int(0, false);
                        let empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        let nullable_ty =
                            self.get_nullable_type(self.string_type.into(), "Nullable<Str>");
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "head_some");
                        let none_bb = self.context.append_basic_block(current_fn, "head_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "head_merge");
                        let _ = self
                            .builder
                            .build_conditional_branch(empty, none_bb, some_bb);
                        // Some: {flag=0, elem} — value inlined, no heap alloc
                        self.builder.position_at_end(some_bb);
                        let elem =
                            self.call_rt("action_list_get", &[list_val.into(), zero.into()])?;
                        let elem_bv = elem.try_as_basic_value().basic().ok_or("get failed")?;
                        let some_struct = {
                            let undef = nullable_ty.get_undef();
                            let r1 = self
                                .builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(0, false),
                                    0,
                                    "s_flag",
                                )
                                .map_err(llvm_err)?;
                            self.builder
                                .build_insert_value(r1, elem_bv, 1, "s_val")
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None: {flag=1, undef}
                        self.builder.position_at_end(none_bb);
                        let none_struct = {
                            let undef = nullable_ty.get_undef();
                            self.builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(1, false),
                                    0,
                                    "n_flag",
                                )
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(nullable_ty, "head_result")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                        let alloca = self
                            .builder
                            .build_alloca(nullable_ty, "head")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
                    }
                    _ => Err("head: argument must be a list".to_string()),
                }
            }
            "last" => {
                if args.len() != 1 {
                    return Err("last expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) | TypedValue::LazyList(lp) => {
                        let list_val = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(list_val, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let zero = self.i64_ty().const_int(0, false);
                        let empty = self
                            .builder
                            .build_int_compare(IntPredicate::EQ, len, zero, "empty")
                            .map_err(llvm_err)?;
                        let last_idx = self
                            .builder
                            .build_int_sub(len, self.i64_ty().const_int(1, false), "last_idx")
                            .map_err(llvm_err)?;
                        let nullable_ty =
                            self.get_nullable_type(self.string_type.into(), "Nullable<Str>");
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "last_some");
                        let none_bb = self.context.append_basic_block(current_fn, "last_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "last_merge");
                        let _ = self
                            .builder
                            .build_conditional_branch(empty, none_bb, some_bb);
                        // Some: {flag=0, elem} — value inlined, no heap alloc
                        self.builder.position_at_end(some_bb);
                        let elem =
                            self.call_rt("action_list_get", &[list_val.into(), last_idx.into()])?;
                        let elem_bv = elem.try_as_basic_value().basic().ok_or("get failed")?;
                        let some_struct = {
                            let undef = nullable_ty.get_undef();
                            let r1 = self
                                .builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(0, false),
                                    0,
                                    "s_flag",
                                )
                                .map_err(llvm_err)?;
                            self.builder
                                .build_insert_value(r1, elem_bv, 1, "s_val")
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None: {flag=1, undef}
                        self.builder.position_at_end(none_bb);
                        let none_struct = {
                            let undef = nullable_ty.get_undef();
                            self.builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(1, false),
                                    0,
                                    "n_flag",
                                )
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(nullable_ty, "last_result")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                        let alloca = self
                            .builder
                            .build_alloca(nullable_ty, "last")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
                    }
                    _ => Err("last: argument must be a list".to_string()),
                }
            }
            "get" => {
                if args.len() != 2 {
                    return Err("get expects 2 arguments (list, index)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let idx_val = self.compile_expr(&args[1])?;
                match (&list_val, &idx_val) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let zero = self.i64_ty().const_int(0, false);
                        let neg = self
                            .builder
                            .build_int_compare(IntPredicate::SLT, *iv, zero, "neg")
                            .map_err(llvm_err)?;
                        let ge_len = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, *iv, len, "ge_len")
                            .map_err(llvm_err)?;
                        let oob = self
                            .builder
                            .build_or(neg, ge_len, "oob")
                            .map_err(llvm_err)?;
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "get_some");
                        let none_bb = self.context.append_basic_block(current_fn, "get_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "get_merge");
                        let _ = self.builder.build_conditional_branch(oob, none_bb, some_bb);
                        // Some: {flag=0, elem} — value inlined, no heap alloc
                        self.builder.position_at_end(some_bb);
                        let elem = self.call_rt("action_list_get", &[lv.into(), (*iv).into()])?;
                        let elem_bv = elem.try_as_basic_value().basic().ok_or("get failed")?;
                        let nullable_ty =
                            self.get_nullable_type(self.string_type.into(), "Nullable<Str>");
                        let some_struct = {
                            let undef = nullable_ty.get_undef();
                            let r1 = self
                                .builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(0, false),
                                    0,
                                    "s_flag",
                                )
                                .map_err(llvm_err)?;
                            self.builder
                                .build_insert_value(r1, elem_bv, 1, "s_val")
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None: {flag=1, undef}
                        self.builder.position_at_end(none_bb);
                        let none_struct = {
                            let undef = nullable_ty.get_undef();
                            self.builder
                                .build_insert_value(
                                    undef,
                                    self.null_flag_ty().const_int(1, false),
                                    0,
                                    "n_flag",
                                )
                                .map_err(llvm_err)?
                        };
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(nullable_ty, "get_result")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&some_struct, some_bb), (&none_struct, none_bb)]);
                        let alloca = self
                            .builder
                            .build_alloca(nullable_ty, "get")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Nullable(alloca, nullable_ty.into()))
                    }
                    _ => Err("get: first argument must be a list, second an Int".to_string()),
                }
            }
            "remove" => {
                if args.len() != 2 {
                    return Err("remove expects 2 arguments (list, index)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let idx_val = self.compile_expr(&args[1])?;
                match (&list_val, &idx_val) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let result =
                            self.call_rt("action_list_remove", &[lv.into(), (*iv).into()])?;
                        let rv = result.try_as_basic_value().basic().ok_or("remove failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "remove_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, rv).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("remove expects (List, Int)".to_string()),
                }
            }
            "reverse" => {
                if args.len() != 1 {
                    return Err("reverse expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_reverse", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("reverse failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "rev")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("reverse: argument must be a list".to_string()),
                }
            }
            "contains" => {
                if args.len() != 2 {
                    return Err("contains expects 2 arguments (list, element)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let elem_val = self.compile_expr(&args[1])?;
                match (&list_val, &elem_val) {
                    (TypedValue::List(lp), _) => {
                        let lv = self.load_list(*lp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let cc = self.call_rt("action_list_contains", &[lv.into(), fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("contains failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    (TypedValue::Set(sp), _) => {
                        let lv = self.load_list(*sp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let cc = self.call_rt("action_list_contains", &[lv.into(), fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("contains failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("contains: first argument must be a list or set".to_string()),
                }
            }
            "containsKey" => {
                if args.len() != 2 {
                    return Err("containsKey expects 2 arguments (map, key)".to_string());
                }
                let map_val = self.compile_expr(&args[0])?;
                let key_val = self.compile_expr(&args[1])?;
                match &map_val {
                    TypedValue::Map(mp) => {
                        let lv = self.load_list(*mp)?;
                        let key_fat = self.to_fat_struct(&key_val)?;
                        let cc =
                            self.call_rt("action_map_contains", &[lv.into(), key_fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("map_contains failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("containsKey: first argument must be a map".to_string()),
                }
            }
            "prepend" => {
                if args.len() != 2 {
                    return Err("prepend expects 2 arguments (element, list)".to_string());
                }
                let elem_val = self.compile_expr(&args[0])?;
                let list_val = self.compile_expr(&args[1])?;
                match list_val {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len_bv = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let new_cap = self
                            .builder
                            .build_int_add(len_bv, self.i64_ty().const_int(4, false), "new_cap")
                            .map_err(llvm_err)?;
                        let new_list = self.call_rt("action_list_create", &[new_cap.into()])?;
                        let new_list_bv = new_list
                            .try_as_basic_value()
                            .basic()
                            .ok_or("create failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "prepend")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, new_list_bv)
                            .map_err(llvm_err)?;
                        // Push element first
                        let fat = self.to_fat_struct(&elem_val)?;
                        let lv2 = self.load_list(alloca)?;
                        let pushed1 =
                            self.call_rt("action_list_push", &[lv2.into(), fat.into()])?;
                        let pb1 = pushed1.try_as_basic_value().basic().ok_or("push1 failed")?;
                        self.builder.build_store(alloca, pb1).map_err(llvm_err)?;
                        // Then push all original elements
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let entry_block = current_fn.get_last_basic_block().unwrap();
                        let loop_bb = self.context.append_basic_block(current_fn, "prepend_loop");
                        let done_bb = self.context.append_basic_block(current_fn, "prepend_done");
                        let _ = self.builder.build_unconditional_branch(loop_bb);
                        self.builder.position_at_end(loop_bb);
                        let i = self
                            .builder
                            .build_phi(self.i64_ty(), "pp_i")
                            .map_err(llvm_err)?;
                        let lv_orig = self.load_list(lp)?;
                        let lv_cur = self.load_list(alloca)?;
                        let elem = self.call_rt(
                            "action_list_get",
                            &[lv_orig.into(), i.as_basic_value().into_int_value().into()],
                        )?;
                        let elem_bv = elem.try_as_basic_value().basic().ok_or("get failed")?;
                        let pushed =
                            self.call_rt("action_list_push", &[lv_cur.into(), elem_bv.into()])?;
                        let pb = pushed.try_as_basic_value().basic().ok_or("push2 failed")?;
                        self.builder.build_store(alloca, pb).map_err(llvm_err)?;
                        let ni = self
                            .builder
                            .build_int_add(
                                i.as_basic_value().into_int_value(),
                                self.i64_ty().const_int(1, false),
                                "pp_ni",
                            )
                            .map_err(llvm_err)?;
                        let done_cond = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, ni, len_bv, "pp_done")
                            .map_err(llvm_err)?;
                        let loop_block = self.builder.get_insert_block().unwrap();
                        i.add_incoming(&[
                            (&self.i64_ty().const_int(0, false), entry_block),
                            (&ni, loop_block),
                        ]);
                        let _ = self
                            .builder
                            .build_conditional_branch(done_cond, done_bb, loop_bb);
                        self.builder.position_at_end(done_bb);
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("prepend: second argument must be a list".to_string()),
                }
            }
            "take" => {
                if args.len() != 2 {
                    return Err("take expects 2 arguments (list, n)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let n_val = self.compile_expr(&args[1])?;
                match (&list_val, &n_val) {
                    (TypedValue::List(lp), TypedValue::Int(nv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_take", &[lv.into(), (*nv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("take failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "take")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("take: first argument must be a list, second an Int".to_string()),
                }
            }
            "drop" => {
                if args.len() != 2 {
                    return Err("drop expects 2 arguments (list, n)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let n_val = self.compile_expr(&args[1])?;
                match (&list_val, &n_val) {
                    (TypedValue::List(lp), TypedValue::Int(nv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_drop", &[lv.into(), (*nv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("drop failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "drop")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("drop: first argument must be a list, second an Int".to_string()),
                }
            }
            "range" => {
                if args.len() != 2 {
                    return Err("range expects 2 arguments (start, end)".to_string());
                }
                let start = self.compile_expr(&args[0])?;
                let end = self.compile_expr(&args[1])?;
                match (&start, &end) {
                    (TypedValue::Int(sv), TypedValue::Int(ev)) => {
                        let cc =
                            self.call_rt("action_list_range", &[(*sv).into(), (*ev).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("range failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "range")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("range: arguments must be Int".to_string()),
                }
            }
            "repeat" => {
                if args.len() != 2 {
                    return Err("repeat expects 2 arguments (value, count)".to_string());
                }
                let val = self.compile_expr(&args[0])?;
                let count = self.compile_expr(&args[1])?;
                match count {
                    TypedValue::Int(cv) => {
                        let cap = self.i64_ty().const_int(4, false);
                        let new_list = self.call_rt("action_list_create", &[cap.into()])?;
                        let new_list_bv = new_list
                            .try_as_basic_value()
                            .basic()
                            .ok_or("create failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "repeat")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(alloca, new_list_bv)
                            .map_err(llvm_err)?;
                        let fat = self.to_fat_struct(&val)?;
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let entry_block = current_fn.get_last_basic_block().unwrap();
                        let loop_bb = self.context.append_basic_block(current_fn, "repeat_loop");
                        let done_bb = self.context.append_basic_block(current_fn, "repeat_done");
                        let _ = self.builder.build_unconditional_branch(loop_bb);
                        self.builder.position_at_end(loop_bb);
                        let i = self
                            .builder
                            .build_phi(self.i64_ty(), "rep_i")
                            .map_err(llvm_err)?;
                        let lv = self.load_list(alloca)?;
                        let pushed = self.call_rt("action_list_push", &[lv.into(), fat.into()])?;
                        let pb = pushed.try_as_basic_value().basic().ok_or("push failed")?;
                        self.builder.build_store(alloca, pb).map_err(llvm_err)?;
                        let ni = self
                            .builder
                            .build_int_add(
                                i.as_basic_value().into_int_value(),
                                self.i64_ty().const_int(1, false),
                                "rep_ni",
                            )
                            .map_err(llvm_err)?;
                        let done_cond = self
                            .builder
                            .build_int_compare(IntPredicate::SGE, ni, cv, "rep_done")
                            .map_err(llvm_err)?;
                        let loop_block = self.builder.get_insert_block().unwrap();
                        i.add_incoming(&[
                            (&self.i64_ty().const_int(0, false), entry_block),
                            (&ni, loop_block),
                        ]);
                        let _ = self
                            .builder
                            .build_conditional_branch(done_cond, done_bb, loop_bb);
                        self.builder.position_at_end(done_bb);
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("repeat: second argument must be Int".to_string()),
                }
            }
            "trimStart" => {
                if args.len() != 1 {
                    return Err("trimStart expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_trim_start", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("trimStart failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "trimmed_start")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("trimStart: argument must be a string".to_string()),
                }
            }
            "trimEnd" => {
                if args.len() != 1 {
                    return Err("trimEnd expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_string_trim_end", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("trimEnd failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "trimmed_end")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("trimEnd: argument must be a string".to_string()),
                }
            }
            "stringContains" => {
                if args.len() != 2 {
                    return Err("stringContains expects 2 arguments (str, substr)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let sub = self.compile_expr(&args[1])?;
                match (&s, &sub) {
                    (TypedValue::Str(sp), TypedValue::Str(subp)) => {
                        let sv = self.load_string(*sp)?;
                        let subv = self.load_string(*subp)?;
                        let cc =
                            self.call_rt("action_string_contains", &[sv.into(), subv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("stringContains failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("stringContains: arguments must be strings".to_string()),
                }
            }
            "stringRepeat" => {
                if args.len() != 2 {
                    return Err("stringRepeat expects 2 arguments (str, count)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let count = self.compile_expr(&args[1])?;
                match (s, count) {
                    (TypedValue::Str(sp), TypedValue::Int(cv)) => {
                        let sv = self.load_string(sp)?;
                        let cc = self.call_rt("action_string_repeat", &[sv.into(), cv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("stringRepeat failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "str_repeat")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err(
                        "stringRepeat: first argument must be a string, second an Int".to_string(),
                    ),
                }
            }
            "tail" => {
                if args.len() != 1 {
                    return Err("tail expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let is_empty = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                len,
                                self.i64_ty().const_int(0, false),
                                "empty",
                            )
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_list_tail", &[lv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("tail failed")?
                            .into_struct_value();
                        self.build_nullable_list(result, is_empty)
                    }
                    _ => Err("tail: argument must be a list".to_string()),
                }
            }
            "zip" => {
                if args.len() != 2 {
                    return Err("zip expects 2 arguments (list1, list2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::List(lp1), TypedValue::List(lp2)) => {
                        let lv1 = self.load_list(*lp1)?;
                        let lv2 = self.load_list(*lp2)?;
                        let cc = self.call_rt("action_list_zip", &[lv1.into(), lv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("zip failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "zip")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("zip: arguments must be lists".to_string()),
                }
            }
            "splitLines" => {
                if args.len() != 1 {
                    return Err("splitLines expects 1 argument (string)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                match s {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let cc = self.call_rt("action_string_split_lines", &[sv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("splitLines failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "lines")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("splitLines: argument must be a string".to_string()),
                }
            }
            "indexOf" => {
                if args.len() != 2 {
                    return Err("indexOf expects 2 arguments".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    // indexOf(element, list) -> Option<Int>
                    (elem, TypedValue::List(lp)) => {
                        let lv = self.load_list(*lp)?;
                        let fat = self.to_fat_struct(elem)?;
                        let cc = self.call_rt("action_list_index_of", &[lv.into(), fat.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("indexOf failed")?
                            .into_int_value();
                        let found = self
                            .builder
                            .build_int_compare(
                                IntPredicate::SGE,
                                result,
                                self.i64_ty().const_int(0, false),
                                "found",
                            )
                            .map_err(llvm_err)?;
                        self.build_nullable_int(result, found)
                    }
                    // indexOf(substring, string) -> Option<Int>
                    (TypedValue::Str(sp1), TypedValue::Str(sp2)) => {
                        let sv1 = self.load_string(*sp1)?;
                        let sv2 = self.load_string(*sp2)?;
                        // runtime expects (haystack, needle), so swap: sv2 is haystack, sv1 is needle
                        let cc =
                            self.call_rt("action_string_index_of", &[sv2.into(), sv1.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("indexOf failed")?
                            .into_int_value();
                        let neg_one = self.i64_ty().const_int((-1i64) as u64, true);
                        let found = self
                            .builder
                            .build_int_compare(IntPredicate::NE, result, neg_one, "found")
                            .map_err(llvm_err)?;
                        self.build_nullable_int(result, found)
                    }
                    _ => Err(
                        "indexOf: first arg must be (element, list) or (substring, string)"
                            .to_string(),
                    ),
                }
            }
            "insert" => {
                if args.len() != 3 {
                    return Err("insert expects 3 arguments (list, index, elem)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let idx_val = self.compile_expr(&args[1])?;
                let elem_val = self.compile_expr(&args[2])?;
                match (&list_val, &idx_val) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let fat = self.to_fat_struct(&elem_val)?;
                        let result = self.call_rt(
                            "action_list_insert",
                            &[lv.into(), (*iv).into(), fat.into()],
                        )?;
                        let rv = result.try_as_basic_value().basic().ok_or("insert failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "insert_result")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, rv).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("insert expects (List, Int, Any)".to_string()),
                }
            }
            "init" => {
                if args.len() != 1 {
                    return Err("init expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let is_empty = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                len,
                                self.i64_ty().const_int(0, false),
                                "empty",
                            )
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_list_init", &[lv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("init failed")?
                            .into_struct_value();
                        self.build_nullable_list(result, is_empty)
                    }
                    _ => Err("init: argument must be a list".to_string()),
                }
            }
            "chars" => {
                if args.len() != 1 {
                    return Err("chars expects 1 argument (string)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                match s {
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let cc = self.call_rt("action_string_chars", &[sv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("chars failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "chars")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("chars: argument must be a string".to_string()),
                }
            }
            "setToList" => {
                if args.len() != 1 {
                    return Err("setToList expects 1 argument (set)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Set(p) => Ok(TypedValue::List(p)),
                    _ => Err("setToList: argument must be a set".to_string()),
                }
            }
            "setFromList" => {
                if args.len() != 1 {
                    return Err("setFromList expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(p) => Ok(TypedValue::Set(p)),
                    _ => Err("setFromList: argument must be a list".to_string()),
                }
            }
            "fromList" => {
                if args.len() != 1 {
                    return Err("fromList expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(p) => Ok(TypedValue::Set(p)),
                    _ => Err("fromList: argument must be a list".to_string()),
                }
            }
            "today" => {
                if !args.is_empty() {
                    return Err("today expects no arguments".to_string());
                }
                // Call C time() and localtime_r() to get real current date
                self.emit_today_now(false)
            }
            "now" => {
                if !args.is_empty() {
                    return Err("now expects no arguments".to_string());
                }
                self.emit_today_now(true)
            }
            // DateTime/Date field accessors
            "year" | "month" | "day" | "hour" | "minute" | "second" => {
                if args.len() != 1 {
                    return Err(format!("{} expects 1 argument", name));
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Struct(p, st) => {
                        let field_idx = match name {
                            "year" => 0,
                            "month" => 1,
                            "day" => 2,
                            "hour" => 3,
                            "minute" => 4,
                            "second" => 5,
                            _ => return Err("bad field".to_string()),
                        };
                        let fptr = self
                            .builder
                            .build_struct_gep(st, p, field_idx, "fptr")
                            .map_err(llvm_err)?;
                        let val = self
                            .builder
                            .build_load(self.i64_ty(), fptr, "val")
                            .map_err(llvm_err)?
                            .into_int_value();
                        Ok(TypedValue::Int(val))
                    }
                    _ => Err(format!(
                        "{}: argument must be a Date or DateTime struct",
                        name
                    )),
                }
            }
            "addDays" => {
                if args.len() != 2 {
                    return Err("addDays expects 2 arguments (date, days)".to_string());
                }
                let d = self.compile_expr(&args[0])?;
                let days = self.compile_expr(&args[1])?;
                let days_bv = days.to_bv().ok_or("days must be Int")?;
                match d {
                    TypedValue::Struct(p, st) => {
                        // Create a new Date struct with added days
                        let alloca = self
                            .builder
                            .build_alloca(st, "new_date")
                            .map_err(llvm_err)?;
                        for i in 0..3u32 {
                            let fptr = self
                                .builder
                                .build_struct_gep(st, p, i, "fptr")
                                .map_err(llvm_err)?;
                            let fval = self
                                .builder
                                .build_load(self.i64_ty(), fptr, "fval")
                                .map_err(llvm_err)?
                                .into_int_value();
                            let new_val = if i == 2 {
                                self.builder
                                    .build_int_add(fval, days_bv.into_int_value(), "new_day")
                                    .map_err(llvm_err)?
                                    .into()
                            } else {
                                fval
                            };
                            let dfptr = self
                                .builder
                                .build_struct_gep(st, alloca, i, "dfptr")
                                .map_err(llvm_err)?;
                            self.builder.build_store(dfptr, new_val).map_err(llvm_err)?;
                        }
                        Ok(TypedValue::Struct(alloca, st))
                    }
                    _ => Err("addDays: first argument must be a Date struct".to_string()),
                }
            }
            "addHours" => {
                if args.len() != 2 {
                    return Err("addHours expects 2 arguments (datetime, hours)".to_string());
                }
                let d = self.compile_expr(&args[0])?;
                let hours = self.compile_expr(&args[1])?;
                let hours_bv = hours.to_bv().ok_or("hours must be Int")?;
                match d {
                    TypedValue::Struct(p, st) => {
                        let alloca = self.builder.build_alloca(st, "new_dt").map_err(llvm_err)?;
                        for i in 0..6u32 {
                            let fptr = self
                                .builder
                                .build_struct_gep(st, p, i, "fptr")
                                .map_err(llvm_err)?;
                            let fval = self
                                .builder
                                .build_load(self.i64_ty(), fptr, "fval")
                                .map_err(llvm_err)?
                                .into_int_value();
                            let new_val = if i == 3 {
                                self.builder
                                    .build_int_add(fval, hours_bv.into_int_value(), "new_hour")
                                    .map_err(llvm_err)?
                                    .into()
                            } else {
                                fval
                            };
                            let dfptr = self
                                .builder
                                .build_struct_gep(st, alloca, i, "dfptr")
                                .map_err(llvm_err)?;
                            self.builder.build_store(dfptr, new_val).map_err(llvm_err)?;
                        }
                        Ok(TypedValue::Struct(alloca, st))
                    }
                    _ => Err("addHours: first argument must be a DateTime struct".to_string()),
                }
            }
            "randChoice" => {
                if args.len() != 1 {
                    return Err("randChoice expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let len = self
                            .builder
                            .build_extract_value(lv, 1, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let empty = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                len,
                                self.i64_ty().const_int(0, false),
                                "empty",
                            )
                            .map_err(llvm_err)?;
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .unwrap()
                            .get_parent()
                            .unwrap();
                        let has_elem = self.context.append_basic_block(current_fn, "has_elem");
                        let no_elem = self.context.append_basic_block(current_fn, "no_elem");
                        let merge = self.context.append_basic_block(current_fn, "merge");
                        let _ = self
                            .builder
                            .build_conditional_branch(empty, no_elem, has_elem);
                        // No element: return None (tag=0)
                        self.builder.position_at_end(no_elem);
                        let none_fat = self.string_type.get_undef();
                        let none1 = self
                            .builder
                            .build_insert_value(
                                none_fat,
                                self.i64_ty().const_int(0, false),
                                0,
                                "none_tag",
                            )
                            .map_err(llvm_err)?;
                        let none2 = self
                            .builder
                            .build_insert_value(none1, self.ptr_ty().const_zero(), 1, "none_data")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge);
                        let none_block = self.builder.get_insert_block().unwrap();
                        // Has element: pick random index
                        self.builder.position_at_end(has_elem);
                        let idx = self
                            .builder
                            .build_int_sub(len, self.i64_ty().const_int(1, false), "max_idx")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt(
                            "action_rand_int",
                            &[self.i64_ty().const_int(0, false).into(), idx.into()],
                        )?;
                        let ri = cc.try_as_basic_value().unwrap_basic().into_int_value();
                        let data = self
                            .builder
                            .build_extract_value(lv, 0, "data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let ep = unsafe {
                            self.builder
                                .build_gep(self.string_type, data, &[ri], "ep")
                                .map_err(llvm_err)
                        }?;
                        let elem = self
                            .builder
                            .build_load(self.string_type, ep, "elem")
                            .map_err(llvm_err)?
                            .into_struct_value();
                        // Wrap in Some: tag=1, data=ptr to elem copy
                        let malloc = self.module.get_function("malloc").unwrap();
                        let some_ptr = self
                            .builder
                            .build_call(
                                malloc,
                                &[self.i64_ty().const_int(16, false).into()],
                                "some",
                            )
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_pointer_value();
                        self.builder.build_store(some_ptr, elem).map_err(llvm_err)?;
                        let some_fat = self.string_type.get_undef();
                        let some1 = self
                            .builder
                            .build_insert_value(
                                some_fat,
                                self.i64_ty().const_int(1, false),
                                0,
                                "some_tag",
                            )
                            .map_err(llvm_err)?;
                        let some2 = self
                            .builder
                            .build_insert_value(some1, some_ptr, 1, "some_data")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge);
                        let some_block = self.builder.get_insert_block().unwrap();
                        // Merge
                        self.builder.position_at_end(merge);
                        let phi = self
                            .builder
                            .build_phi(self.string_type, "choice")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[
                            (&none2.as_basic_value_enum(), none_block),
                            (&some2.as_basic_value_enum(), some_block),
                        ]);
                        // Return as fat struct (Tag=EnumKind(3), data=ptr to fat value)
                        let opt_alloca = self
                            .builder
                            .build_alloca(self.string_type, "opt")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(opt_alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::List(opt_alloca)) // Reuse List type for the result
                    }
                    _ => Err("randChoice: argument must be a list".to_string()),
                }
            }
            "toChar" => {
                if args.len() != 1 {
                    return Err("toChar expects 1 argument (int)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => {
                        // Validate: code point must be in valid Unicode range
                        let max_cp = self.i64_ty().const_int(0x10FFFF, false);
                        let in_range = self
                            .builder
                            .build_int_compare(IntPredicate::ULE, iv, max_cp, "valid_cp")
                            .map_err(llvm_err)?;
                        let valid = self.build_nullable_int(iv, in_range);
                        valid
                    }
                    _ => Err("toChar: argument must be an Int".to_string()),
                }
            }
            "charCode" => {
                if args.len() != 1 {
                    return Err("charCode expects 1 argument (char)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => Ok(TypedValue::Int(iv)),
                    _ => Err("charCode: argument must be a Char".to_string()),
                }
            }
            "toInt" => {
                if args.len() != 1 {
                    return Err("toInt expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Int(iv) => {
                        self.build_nullable_int(iv, self.bool_ty().const_int(1, false))
                    }
                    TypedValue::Float(fv) => {
                        let i = self
                            .builder
                            .build_float_to_signed_int(fv, self.i64_ty(), "ftoi")
                            .map_err(llvm_err)?;
                        self.build_nullable_int(i, self.bool_ty().const_int(1, false))
                    }
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let cc = self.call_rt("action_parse_int", &[sv.into()])?;
                        let result_struct = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("parseInt failed")?
                            .into_struct_value();
                        let val = self
                            .builder
                            .build_extract_value(result_struct, 0, "val")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let ok = self
                            .builder
                            .build_extract_value(result_struct, 1, "ok")
                            .map_err(llvm_err)?
                            .into_int_value();
                        self.build_nullable_int(val, ok)
                    }
                    _ => Err("toInt: cannot convert to Int".to_string()),
                }
            }
            "toFloat" => {
                if args.len() != 1 {
                    return Err("toFloat expects 1 argument".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let always_true = self.bool_ty().const_int(1, false);
                match v {
                    TypedValue::Float(fv) => self.build_nullable_float(fv, always_true),
                    TypedValue::Int(iv) => {
                        let f = self
                            .builder
                            .build_signed_int_to_float(iv, self.f64_ty(), "itof")
                            .map_err(llvm_err)?;
                        self.build_nullable_float(f, always_true)
                    }
                    TypedValue::Str(sp) => {
                        let sv = self.load_string(sp)?;
                        let len = self
                            .builder
                            .build_extract_value(sv, 0, "len")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let has_chars = self
                            .builder
                            .build_int_compare(
                                IntPredicate::UGT,
                                len,
                                self.i64_ty().const_int(0, false),
                                "has_chars",
                            )
                            .map_err(llvm_err)?;
                        // Check first char is digit, '-', '+', or '.'
                        let data_ptr = self
                            .builder
                            .build_extract_value(sv, 1, "dptr")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        let first_char = self
                            .builder
                            .build_load(self.context.i8_type(), data_ptr, "first_char")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let is_digit = self
                            .builder
                            .build_int_compare(
                                IntPredicate::UGE,
                                first_char,
                                self.context.i8_type().const_int(b'0' as u64, false),
                                "isd",
                            )
                            .map_err(llvm_err)?;
                        let le9 = self
                            .builder
                            .build_int_compare(
                                IntPredicate::ULE,
                                first_char,
                                self.context.i8_type().const_int(b'9' as u64, false),
                                "le9",
                            )
                            .map_err(llvm_err)?;
                        let is_d = self
                            .builder
                            .build_and(is_digit, le9, "is_digit")
                            .map_err(llvm_err)?;
                        let is_minus = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                first_char,
                                self.context.i8_type().const_int(b'-' as u64, false),
                                "is_minus",
                            )
                            .map_err(llvm_err)?;
                        let is_plus = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                first_char,
                                self.context.i8_type().const_int(b'+' as u64, false),
                                "is_plus",
                            )
                            .map_err(llvm_err)?;
                        let is_dot = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                first_char,
                                self.context.i8_type().const_int(b'.' as u64, false),
                                "is_dot",
                            )
                            .map_err(llvm_err)?;
                        let is_sign = self
                            .builder
                            .build_or(is_minus, is_plus, "is_sign")
                            .map_err(llvm_err)?;
                        let is_num_start = self
                            .builder
                            .build_or(is_d, is_sign, "is_num1")
                            .map_err(llvm_err)?;
                        let is_valid = self
                            .builder
                            .build_or(is_num_start, is_dot, "is_valid")
                            .map_err(llvm_err)?;
                        let ok = self
                            .builder
                            .build_and(has_chars, is_valid, "ok")
                            .map_err(llvm_err)?;
                        let strtod_fn = self.module.get_function("strtod").unwrap();
                        let null_ptr = self.ptr_ty().const_zero();
                        let result = self
                            .builder
                            .build_call(strtod_fn, &[data_ptr.into(), null_ptr.into()], "fval")
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .basic()
                            .ok_or("strtod failed")?
                            .into_float_value();
                        self.build_nullable_float(result, ok)
                    }
                    _ => Err("toFloat: cannot convert to Float".to_string()),
                }
            }
            "withIndex" => {
                if args.len() != 1 {
                    return Err("withIndex expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_with_index", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("withIndex failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "wi")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("withIndex: argument must be a list".to_string()),
                }
            }
            "unique" => {
                if args.len() != 1 {
                    return Err("unique expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_unique", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("unique failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "unique")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("unique: argument must be a list".to_string()),
                }
            }
            "slice" => {
                if args.len() != 3 {
                    return Err("slice expects 3 arguments (collection, start, end)".to_string());
                }
                let coll_v = self.compile_expr(&args[0])?;
                let start_v = self.compile_expr(&args[1])?;
                let end_v = self.compile_expr(&args[2])?;
                match (&coll_v, &start_v, &end_v) {
                    // slice(List<T>, Int, Int) -> List<T>  with [start, end) semantics
                    (TypedValue::List(lp), TypedValue::Int(sv), TypedValue::Int(ev)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt(
                            "action_list_slice",
                            &[lv.into(), (*sv).into(), (*ev).into()],
                        )?;
                        let result = cc.try_as_basic_value().basic().ok_or("slice failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "slice")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    // slice(String, Int, Int) -> String  with [start, end) semantics
                    (TypedValue::Str(sp), TypedValue::Int(sv), TypedValue::Int(ev)) => {
                        let str_val = self.load_string(*sp)?;
                        let len = self
                            .builder
                            .build_int_sub(*ev, *sv, "slice_len")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt(
                            "action_string_substring",
                            &[str_val.into(), (*sv).into(), len.into()],
                        )?;
                        let result = cc.try_as_basic_value().basic().ok_or("slice failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "slice_str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err(
                        "slice: first argument must be a list or string, second and third Int"
                            .to_string(),
                    ),
                }
            }
            "flatten" => {
                if args.len() != 1 {
                    return Err("flatten expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_flatten", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("flatten failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "flatten")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("flatten: argument must be a list".to_string()),
                }
            }
            "splitAt" => {
                if args.len() != 2 {
                    return Err("splitAt expects 2 arguments (list, index)".to_string());
                }
                let list_v = self.compile_expr(&args[0])?;
                let idx_v = self.compile_expr(&args[1])?;
                match (&list_v, &idx_v) {
                    (TypedValue::List(lp), TypedValue::Int(iv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc =
                            self.call_rt("action_list_split_at", &[lv.into(), (*iv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("splitAt failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "splitAt")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("splitAt: first argument must be a list, second Int".to_string()),
                }
            }
            "chunks" => {
                if args.len() != 2 {
                    return Err("chunks expects 2 arguments (list, size)".to_string());
                }
                let list_v = self.compile_expr(&args[0])?;
                let size_v = self.compile_expr(&args[1])?;
                match (&list_v, &size_v) {
                    (TypedValue::List(lp), TypedValue::Int(sv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_chunks", &[lv.into(), (*sv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("chunks failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "chunks")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("chunks: first argument must be a list, second Int".to_string()),
                }
            }
            "windows" => {
                if args.len() != 2 {
                    return Err("windows expects 2 arguments (list, size)".to_string());
                }
                let list_v = self.compile_expr(&args[0])?;
                let size_v = self.compile_expr(&args[1])?;
                match (&list_v, &size_v) {
                    (TypedValue::List(lp), TypedValue::Int(sv)) => {
                        let lv = self.load_list(*lp)?;
                        let cc = self.call_rt("action_list_windows", &[lv.into(), (*sv).into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("windows failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "windows")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("windows: first argument must be a list, second Int".to_string()),
                }
            }
            "pow" => {
                if args.len() != 2 {
                    return Err("pow expects 2 arguments".to_string());
                }
                let base = self.compile_expr(&args[0])?;
                let exp = self.compile_expr(&args[1])?;
                match (&base, &exp) {
                    (TypedValue::Float(bv), TypedValue::Float(ev)) => {
                        let cc = self.call_rt("action_pow", &[(*bv).into(), (*ev).into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    (TypedValue::Int(bv), TypedValue::Int(ev)) => {
                        let bf = self
                            .builder
                            .build_signed_int_to_float(*bv, self.f64_ty(), "bf")
                            .map_err(llvm_err)?;
                        let ef = self
                            .builder
                            .build_signed_int_to_float(*ev, self.f64_ty(), "ef")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_pow", &[bf.into(), ef.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    // Mixed Int/Float → promote Int to Float
                    (TypedValue::Int(bv), TypedValue::Float(ev)) => {
                        let bf = self
                            .builder
                            .build_signed_int_to_float(*bv, self.f64_ty(), "bf")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_pow", &[bf.into(), (*ev).into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    (TypedValue::Float(bv), TypedValue::Int(ev)) => {
                        let ef = self
                            .builder
                            .build_signed_int_to_float(*ev, self.f64_ty(), "ef")
                            .map_err(llvm_err)?;
                        let cc = self.call_rt("action_pow", &[(*bv).into(), ef.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("pow failed")?
                            .into_float_value();
                        Ok(TypedValue::Float(result))
                    }
                    _ => Err("pow: arguments must be numeric".to_string()),
                }
            }
            "mapKeys" => {
                if args.len() != 1 {
                    return Err("mapKeys expects 1 argument (map)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_keys", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapKeys failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "keys")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("mapKeys: argument must be a map".to_string()),
                }
            }
            "mapValues" => {
                if args.len() != 1 {
                    return Err("mapValues expects 1 argument (map)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_values", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapValues failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "values")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("mapValues: argument must be a map".to_string()),
                }
            }
            "mapEntries" => {
                if args.len() != 1 {
                    return Err("mapEntries expects 1 argument (map)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Map(mp) => {
                        let mv = self.load_list(mp)?;
                        let cc = self.call_rt("action_map_entries", &[mv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("mapEntries failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "entries")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("mapEntries: argument must be a map".to_string()),
                }
            }
            "mapUnion" => {
                if args.len() != 2 {
                    return Err("map.union expects 2 arguments (map1, map2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Map(mp1), TypedValue::Map(mp2)) => {
                        let mv1 = self.load_list(*mp1)?;
                        let mv2 = self.load_list(*mp2)?;
                        let cc = self.call_rt("action_map_union", &[mv1.into(), mv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("map.union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "mapUnion")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Map(alloca))
                    }
                    _ => Err("map.union: arguments must be maps".to_string()),
                }
            }
            "setUnion" => {
                if args.len() != 2 {
                    return Err("set.union expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc = self.call_rt("action_set_union", &[sv1.into(), sv2.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("set.union failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "union")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("set.union: arguments must be sets".to_string()),
                }
            }
            "setIntersection" => {
                if args.len() != 2 {
                    return Err("set.intersection expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc =
                            self.call_rt("action_set_intersection", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.intersection failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "intersection")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("set.intersection: arguments must be sets".to_string()),
                }
            }
            "setDifference" => {
                if args.len() != 2 {
                    return Err("set.difference expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc =
                            self.call_rt("action_set_difference", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.difference failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "difference")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::Set(alloca))
                    }
                    _ => Err("set.difference: arguments must be sets".to_string()),
                }
            }
            "setIsSubset" => {
                if args.len() != 2 {
                    return Err("set.isSubset expects 2 arguments (set1, set2)".to_string());
                }
                let v1 = self.compile_expr(&args[0])?;
                let v2 = self.compile_expr(&args[1])?;
                match (&v1, &v2) {
                    (TypedValue::Set(sp1), TypedValue::Set(sp2)) => {
                        let sv1 = self.load_list(*sp1)?;
                        let sv2 = self.load_list(*sp2)?;
                        let cc = self.call_rt("action_set_is_subset", &[sv1.into(), sv2.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("set.isSubset failed")?
                            .into_int_value();
                        Ok(TypedValue::Bool(result))
                    }
                    _ => Err("set.isSubset: arguments must be sets".to_string()),
                }
            }
            "randShuffle" => {
                if args.len() != 1 {
                    return Err("randShuffle expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_rand_shuffle", &[lv.into()])?;
                        let result = cc
                            .try_as_basic_value()
                            .basic()
                            .ok_or("randShuffle failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "shuffled")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("randShuffle: argument must be a list".to_string()),
                }
            }
            "sorted" => {
                if args.len() != 1 {
                    return Err("sorted expects 1 argument (list)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::List(lp) => {
                        let lv = self.load_list(lp)?;
                        let cc = self.call_rt("action_list_sorted", &[lv.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("sorted failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "sorted")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("sorted: argument must be a list".to_string()),
                }
            }
            "readDir" => {
                if args.len() != 1 {
                    return Err("readDir expects 1 argument (path)".to_string());
                }
                if self.module.get_function("action_read_dir").is_none() {
                    self.emit_read_dir_runtime()?;
                }
                let v = self.compile_expr(&args[0])?;
                match v {
                    TypedValue::Str(p) => {
                        let s = self.load_string(p)?;
                        let cc = self.call_rt("action_read_dir", &[s.into()])?;
                        let result = cc.try_as_basic_value().basic().ok_or("readDir failed")?;
                        let alloca = self
                            .builder
                            .build_alloca(self.list_type, "readDir")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, result).map_err(llvm_err)?;
                        Ok(TypedValue::List(alloca))
                    }
                    _ => Err("readDir: argument must be a string".to_string()),
                }
            }
            "identity" => {
                if args.len() != 1 {
                    return Err("identity expects 1 argument".to_string());
                }
                self.compile_expr(&args[0])
            }
            "compose" => {
                if args.len() != 3 {
                    return Err("compose expects 3 arguments (f, g, x)".to_string());
                }
                // compose(f, g, x) = f(g(x))
                let inner = Expr::Call {
                    func: Box::new(args[1].clone()),
                    args: vec![args[2].clone()],
                    trailing_lambda: None,
                };
                let outer = Expr::Call {
                    func: Box::new(args[0].clone()),
                    args: vec![inner],
                    trailing_lambda: None,
                };
                self.compile_expr(&outer)
            }
            "diffDays" => {
                if args.len() != 2 {
                    return Err("diffDays expects 2 arguments (date1, date2)".to_string());
                }
                let d1 = self.compile_expr(&args[0])?;
                let d2 = self.compile_expr(&args[1])?;
                let (p1, st1) = match d1 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffDays: arguments must be Date structs".to_string()),
                };
                let (p2, st2) = match d2 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffDays: arguments must be Date structs".to_string()),
                };
                let i64_ty = self.i64_ty();
                // Julian Day Number: JDN = D + (153*m+2)/5 + 365*y + y/4 - y/100 + y/400 - 32045
                // where a = (14-M)/12, y = Y+4800-a, m = M+12*a-3
                let jdn = |yp: PointerValue<'ctx>,
                           sty: inkwell::types::StructType<'ctx>|
                 -> Result<IntValue<'ctx>, String> {
                    let y_ptr = self
                        .builder
                        .build_struct_gep(sty, yp, 0, "j_y")
                        .map_err(llvm_err)?;
                    let y_val = self
                        .builder
                        .build_load(i64_ty, y_ptr, "j_yv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let m_ptr = self
                        .builder
                        .build_struct_gep(sty, yp, 1, "j_m")
                        .map_err(llvm_err)?;
                    let m_val = self
                        .builder
                        .build_load(i64_ty, m_ptr, "j_mv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let d_ptr = self
                        .builder
                        .build_struct_gep(sty, yp, 2, "j_d")
                        .map_err(llvm_err)?;
                    let d_val = self
                        .builder
                        .build_load(i64_ty, d_ptr, "j_dv")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let c12 = i64_ty.const_int(12, false);
                    let c14 = i64_ty.const_int(14, false);
                    let c4800 = i64_ty.const_int(4800, false);
                    let c3 = i64_ty.const_int(3, false);
                    let c4 = i64_ty.const_int(4, false);
                    let c100 = i64_ty.const_int(100, false);
                    let c400 = i64_ty.const_int(400, false);
                    let c153 = i64_ty.const_int(153, false);
                    let c2 = i64_ty.const_int(2, false);
                    let c5 = i64_ty.const_int(5, false);
                    let c365 = i64_ty.const_int(365, false);
                    let c32045 = i64_ty.const_int(32045, false);
                    // a = (14 - M) / 12
                    let a = self
                        .builder
                        .build_int_signed_div(
                            self.builder
                                .build_int_sub(c14, m_val, "t_a1")
                                .map_err(llvm_err)?,
                            c12,
                            "a",
                        )
                        .map_err(llvm_err)?;
                    // y = Y + 4800 - a
                    let y = self
                        .builder
                        .build_int_sub(
                            self.builder
                                .build_int_add(y_val, c4800, "t_y1")
                                .map_err(llvm_err)?,
                            a,
                            "y",
                        )
                        .map_err(llvm_err)?;
                    // m = M + 12*a - 3
                    let m = self
                        .builder
                        .build_int_sub(
                            self.builder
                                .build_int_add(
                                    m_val,
                                    self.builder
                                        .build_int_mul(c12, a, "t_m1")
                                        .map_err(llvm_err)?,
                                    "t_m2",
                                )
                                .map_err(llvm_err)?,
                            c3,
                            "m",
                        )
                        .map_err(llvm_err)?;
                    // term1 = (153*m + 2) / 5
                    let term1 = self
                        .builder
                        .build_int_signed_div(
                            self.builder
                                .build_int_add(
                                    self.builder
                                        .build_int_mul(c153, m, "t_t1a")
                                        .map_err(llvm_err)?,
                                    c2,
                                    "t_t1b",
                                )
                                .map_err(llvm_err)?,
                            c5,
                            "term1",
                        )
                        .map_err(llvm_err)?;
                    // term2 = 365*y
                    let term2 = self
                        .builder
                        .build_int_mul(c365, y, "term2")
                        .map_err(llvm_err)?;
                    // term3 = y/4
                    let term3 = self
                        .builder
                        .build_int_signed_div(y, c4, "term3")
                        .map_err(llvm_err)?;
                    // term4 = y/100
                    let term4 = self
                        .builder
                        .build_int_signed_div(y, c100, "term4")
                        .map_err(llvm_err)?;
                    // term5 = y/400
                    let term5 = self
                        .builder
                        .build_int_signed_div(y, c400, "term5")
                        .map_err(llvm_err)?;
                    // JDN = D + term1 + term2 + term3 - term4 + term5 - 32045
                    let s1 = self
                        .builder
                        .build_int_add(d_val, term1, "s1")
                        .map_err(llvm_err)?;
                    let s2 = self
                        .builder
                        .build_int_add(s1, term2, "s2")
                        .map_err(llvm_err)?;
                    let s3 = self
                        .builder
                        .build_int_add(s2, term3, "s3")
                        .map_err(llvm_err)?;
                    let s4 = self
                        .builder
                        .build_int_sub(s3, term4, "s4")
                        .map_err(llvm_err)?;
                    let s5 = self
                        .builder
                        .build_int_add(s4, term5, "s5")
                        .map_err(llvm_err)?;
                    let jdn_val = self
                        .builder
                        .build_int_sub(s5, c32045, "jdn")
                        .map_err(llvm_err)?;
                    Ok(jdn_val)
                };
                let j1 = jdn(p1, st1)?;
                let j2 = jdn(p2, st2)?;
                let diff = self
                    .builder
                    .build_int_sub(j1, j2, "diff")
                    .map_err(llvm_err)?;
                let zero = i64_ty.const_int(0, false);
                let nd = self.builder.build_int_neg(diff, "nd").map_err(llvm_err)?;
                let is_neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, diff, zero, "is_neg")
                    .map_err(llvm_err)?;
                let abs_diff = self
                    .builder
                    .build_select(is_neg, nd, diff, "abs_diff")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::Int(abs_diff))
            }
            "weekday" => {
                if args.len() != 1 {
                    return Err("weekday expects 1 argument (date)".to_string());
                }
                let d = self.compile_expr(&args[0])?;
                match d {
                    TypedValue::Struct(p, st) => {
                        // Use mktime to compute proper weekday
                        // Build struct tm: {i32 x 9}
                        let i32_ty = self.context.i32_type();
                        let tm_ty = self.context.struct_type(&[i32_ty.into(); 9], false);
                        let tm_a = self.builder.build_alloca(tm_ty, "tm").map_err(llvm_err)?;
                        let i64_ty = self.i64_ty();
                        // Extract year, month, day from Date struct
                        let yp = self
                            .builder
                            .build_struct_gep(st, p, 0, "w_yp")
                            .map_err(llvm_err)?;
                        let yv = self
                            .builder
                            .build_load(i64_ty, yp, "w_yv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let mp = self
                            .builder
                            .build_struct_gep(st, p, 1, "w_mp")
                            .map_err(llvm_err)?;
                        let mv = self
                            .builder
                            .build_load(i64_ty, mp, "w_mv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let dp = self
                            .builder
                            .build_struct_gep(st, p, 2, "w_dp")
                            .map_err(llvm_err)?;
                        let dv = self
                            .builder
                            .build_load(i64_ty, dp, "w_dv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        // tm_sec = 0
                        let f0 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 0, "f0")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f0, i32_ty.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_min = 0
                        let f1 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 1, "f1")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f1, i32_ty.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_hour = 12 (noon, avoid DST issues)
                        let f2 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 2, "f2")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f2, i32_ty.const_int(12, false))
                            .map_err(llvm_err)?;
                        // tm_mday = day
                        let f3 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 3, "f3")
                            .map_err(llvm_err)?;
                        let dv32 = self
                            .builder
                            .build_int_truncate(dv, i32_ty, "dv32")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f3, dv32).map_err(llvm_err)?;
                        // tm_mon = month - 1
                        let f4 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 4, "f4")
                            .map_err(llvm_err)?;
                        let mon_minus = self
                            .builder
                            .build_int_sub(mv, i64_ty.const_int(1, false), "mon_minus")
                            .map_err(llvm_err)?;
                        let mon32 = self
                            .builder
                            .build_int_truncate(mon_minus, i32_ty, "mon32")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f4, mon32).map_err(llvm_err)?;
                        // tm_year = year - 1900
                        let f5 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 5, "f5")
                            .map_err(llvm_err)?;
                        let y_minus = self
                            .builder
                            .build_int_sub(yv, i64_ty.const_int(1900, false), "y_minus")
                            .map_err(llvm_err)?;
                        let y32 = self
                            .builder
                            .build_int_truncate(y_minus, i32_ty, "y32")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f5, y32).map_err(llvm_err)?;
                        // Remaining fields init to 0
                        for i in 6..9u32 {
                            let f = self
                                .builder
                                .build_struct_gep(tm_ty, tm_a, i, "f")
                                .map_err(llvm_err)?;
                            self.builder
                                .build_store(f, i32_ty.const_int(0, false))
                                .map_err(llvm_err)?;
                        }
                        // Call mktime
                        let mktime_fn = self.module.get_function("mktime").unwrap_or_else(|| {
                            self.module.add_function(
                                "mktime",
                                self.i64_ty().fn_type(&[self.ptr_ty().into()], false),
                                None,
                            )
                        });
                        let _ = self
                            .builder
                            .build_call(mktime_fn, &[tm_a.into()], "")
                            .map_err(llvm_err)?;
                        // Read tm_wday (field 6)
                        let wf = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 6, "wf")
                            .map_err(llvm_err)?;
                        let wday32 = self
                            .builder
                            .build_load(i32_ty, wf, "wday")
                            .map_err(llvm_err)?
                            .into_int_value();
                        // Convert: C wday 0=Sunday -> Atomic 1=Monday..7=Sunday
                        // Atomic weekday: 1=Monday, 7=Sunday
                        // C: 0=Sun,1=Mon,2=Tue,3=Wed,4=Thu,5=Fri,6=Sat
                        // Map: C=0->7, C=1->1, C=2->2, C=3->3, C=4->4, C=5->5, C=6->6
                        let wd_c0 = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                wday32,
                                i32_ty.const_int(0, false),
                                "wd_c0",
                            )
                            .map_err(llvm_err)?;
                        let wd32 = self
                            .builder
                            .build_select(wd_c0, i32_ty.const_int(7, false), wday32, "wd")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let wd = self
                            .builder
                            .build_int_s_extend(wd32, i64_ty, "wd64")
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Int(wd))
                    }
                    _ => Err("weekday: argument must be a Date struct".to_string()),
                }
            }
            "sum" => {
                if args.len() != 1 {
                    return Err("sum expects 1 argument (list)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("sum: argument must be a list".to_string()),
                };
                let list = self.load_list(list_ptr)?;
                let len = self.list_len_val(list)?;
                let data = self.list_data_ptr(list)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                let sum_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "sum")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(sum_a, self.i64_ty().const_int(0, false))
                    .map_err(llvm_err)?;
                let i_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "i")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(i_a, self.i64_ty().const_int(0, false))
                    .map_err(llvm_err)?;
                let hdr = self.context.append_basic_block(current, "sum_hdr");
                let bdy = self.context.append_basic_block(current, "sum_bdy");
                let ext = self.context.append_basic_block(current, "sum_ext");
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(hdr);
                let iv = self
                    .builder
                    .build_load(self.i64_ty(), i_a, "iv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, iv, len, "cond")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(cond, bdy, ext);
                self.builder.position_at_end(bdy);
                let ep = unsafe {
                    self.builder
                        .build_gep(self.string_type, data, &[iv], "ep")
                        .map_err(llvm_err)
                }?;
                let ev = self
                    .builder
                    .build_load(self.string_type, ep, "ev")
                    .map_err(llvm_err)?;
                let etag = self
                    .builder
                    .build_extract_value(ev.into_struct_value(), 0, "etag")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cur = self
                    .builder
                    .build_load(self.i64_ty(), sum_a, "cur")
                    .map_err(llvm_err)?
                    .into_int_value();
                let new_sum = self
                    .builder
                    .build_int_add(cur, etag, "new_sum")
                    .map_err(llvm_err)?;
                self.builder.build_store(sum_a, new_sum).map_err(llvm_err)?;
                let ni = self
                    .builder
                    .build_int_add(iv, self.i64_ty().const_int(1, false), "ni")
                    .map_err(llvm_err)?;
                self.builder.build_store(i_a, ni).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(ext);
                let result = self
                    .builder
                    .build_load(self.i64_ty(), sum_a, "result")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Int(result.into_int_value()))
            }
            "product" => {
                if args.len() != 1 {
                    return Err("product expects 1 argument (list)".to_string());
                }
                let list_val = self.compile_expr(&args[0])?;
                let list_ptr = match list_val {
                    TypedValue::List(p) => p,
                    _ => return Err("product: argument must be a list".to_string()),
                };
                let list = self.load_list(list_ptr)?;
                let len = self.list_len_val(list)?;
                let data = self.list_data_ptr(list)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                let prod_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "prod")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(prod_a, self.i64_ty().const_int(1, false))
                    .map_err(llvm_err)?;
                let i_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "i")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(i_a, self.i64_ty().const_int(0, false))
                    .map_err(llvm_err)?;
                let hdr = self.context.append_basic_block(current, "prod_hdr");
                let bdy = self.context.append_basic_block(current, "prod_bdy");
                let ext = self.context.append_basic_block(current, "prod_ext");
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(hdr);
                let iv = self
                    .builder
                    .build_load(self.i64_ty(), i_a, "iv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cond = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, iv, len, "cond")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(cond, bdy, ext);
                self.builder.position_at_end(bdy);
                let ep = unsafe {
                    self.builder
                        .build_gep(self.string_type, data, &[iv], "ep")
                        .map_err(llvm_err)
                }?;
                let ev = self
                    .builder
                    .build_load(self.string_type, ep, "ev")
                    .map_err(llvm_err)?;
                let etag = self
                    .builder
                    .build_extract_value(ev.into_struct_value(), 0, "etag")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cur = self
                    .builder
                    .build_load(self.i64_ty(), prod_a, "cur")
                    .map_err(llvm_err)?
                    .into_int_value();
                let new_prod = self
                    .builder
                    .build_int_mul(cur, etag, "new_prod")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(prod_a, new_prod)
                    .map_err(llvm_err)?;
                let ni = self
                    .builder
                    .build_int_add(iv, self.i64_ty().const_int(1, false), "ni")
                    .map_err(llvm_err)?;
                self.builder.build_store(i_a, ni).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(hdr);
                self.builder.position_at_end(ext);
                let result = self
                    .builder
                    .build_load(self.i64_ty(), prod_a, "result")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Int(result.into_int_value()))
            }
            "digits" => {
                // digits(n) -> List<Int>: decimal digits of abs(n), MSD first. 0 -> [0].
                if args.len() != 1 {
                    return Err("digits expects 1 argument (int)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let n = match v {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("digits: argument must be an int".to_string()),
                };
                let ten = self.i64_ty().const_int(10, false);
                let zero = self.i64_ty().const_int(0, false);
                let one = self.i64_ty().const_int(1, false);
                // abs_n = n < 0 ? -n : n
                let neg = self.builder.build_int_neg(n, "neg").map_err(llvm_err)?;
                let is_neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, n, zero, "is_neg")
                    .map_err(llvm_err)?;
                let abs_n = self
                    .builder
                    .build_select(is_neg, neg, n, "abs_n")
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_zero = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, n, zero, "is0")
                    .map_err(llvm_err)?;
                let current = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no function")?;
                // Count digits via repeated division
                let dc_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "dc")
                    .map_err(llvm_err)?;
                self.builder.build_store(dc_a, zero).map_err(llvm_err)?;
                let tmp_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "tmp")
                    .map_err(llvm_err)?;
                self.builder.build_store(tmp_a, abs_n).map_err(llvm_err)?;
                let cnt_hdr = self.context.append_basic_block(current, "dc_hdr");
                let cnt_bdy = self.context.append_basic_block(current, "dc_bdy");
                let cnt_ext = self.context.append_basic_block(current, "dc_ext");
                let _ = self.builder.build_unconditional_branch(cnt_hdr);
                self.builder.position_at_end(cnt_hdr);
                let tv = self
                    .builder
                    .build_load(self.i64_ty(), tmp_a, "tv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let gt0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGT, tv, zero, "gt0")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(gt0, cnt_bdy, cnt_ext);
                self.builder.position_at_end(cnt_bdy);
                let dv = self
                    .builder
                    .build_load(self.i64_ty(), dc_a, "dv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let nd = self
                    .builder
                    .build_int_add(dv, one, "nd")
                    .map_err(llvm_err)?;
                self.builder.build_store(dc_a, nd).map_err(llvm_err)?;
                let nt = self
                    .builder
                    .build_int_signed_div(tv, ten, "nt")
                    .map_err(llvm_err)?;
                self.builder.build_store(tmp_a, nt).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(cnt_hdr);
                self.builder.position_at_end(cnt_ext);
                let ndigits = self
                    .builder
                    .build_load(self.i64_ty(), dc_a, "nd")
                    .map_err(llvm_err)?
                    .into_int_value();
                // 0 -> 1 digit
                let final_dc = self
                    .builder
                    .build_select(is_zero, one, ndigits, "fdc")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Create result list with capacity = final_dc
                let cc = self.call_rt("action_list_create", &[final_dc.into()])?;
                let res_bv = cc
                    .try_as_basic_value()
                    .basic()
                    .ok_or("list_create failed")?;
                let res_a = self
                    .builder
                    .build_alloca(self.list_type, "digits_res")
                    .map_err(llvm_err)?;
                self.builder.build_store(res_a, res_bv).map_err(llvm_err)?;
                // Compute 10^(ndigits-1) iteratively
                let pow_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "pow10")
                    .map_err(llvm_err)?;
                self.builder.build_store(pow_a, one).map_err(llvm_err)?;
                let pi_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "pi")
                    .map_err(llvm_err)?;
                self.builder.build_store(pi_a, one).map_err(llvm_err)?;
                let pow_hdr = self.context.append_basic_block(current, "pow_hdr");
                let pow_bdy = self.context.append_basic_block(current, "pow_bdy");
                let pow_ext = self.context.append_basic_block(current, "pow_ext");
                let _ = self.builder.build_unconditional_branch(pow_hdr);
                self.builder.position_at_end(pow_hdr);
                let piv = self
                    .builder
                    .build_load(self.i64_ty(), pi_a, "piv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let plt = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, piv, final_dc, "plt")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_conditional_branch(plt, pow_bdy, pow_ext);
                self.builder.position_at_end(pow_bdy);
                let pv = self
                    .builder
                    .build_load(self.i64_ty(), pow_a, "pv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let npv = self
                    .builder
                    .build_int_mul(pv, ten, "npv")
                    .map_err(llvm_err)?;
                self.builder.build_store(pow_a, npv).map_err(llvm_err)?;
                let npi = self
                    .builder
                    .build_int_add(piv, one, "npi")
                    .map_err(llvm_err)?;
                self.builder.build_store(pi_a, npi).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(pow_hdr);
                self.builder.position_at_end(pow_ext);
                let pow10 = self
                    .builder
                    .build_load(self.i64_ty(), pow_a, "pow10")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Extract digits MSD-first: for i in 0..ndigits { d = (abs_n / pow10) % 10; push; pow10 /= 10 }
                self.builder.build_store(tmp_a, abs_n).map_err(llvm_err)?;
                let di_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "di")
                    .map_err(llvm_err)?;
                self.builder.build_store(di_a, zero).map_err(llvm_err)?;
                let p10_a = self
                    .builder
                    .build_alloca(self.i64_ty(), "p10")
                    .map_err(llvm_err)?;
                self.builder.build_store(p10_a, pow10).map_err(llvm_err)?;
                let fill_hdr = self.context.append_basic_block(current, "fill_hdr");
                let fill_bdy = self.context.append_basic_block(current, "fill_bdy");
                let fill_ext = self.context.append_basic_block(current, "fill_ext");
                let _ = self.builder.build_unconditional_branch(fill_hdr);
                self.builder.position_at_end(fill_hdr);
                let div = self
                    .builder
                    .build_load(self.i64_ty(), di_a, "div")
                    .map_err(llvm_err)?
                    .into_int_value();
                let flt = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, div, final_dc, "flt")
                    .map_err(llvm_err)?;
                let _ = self
                    .builder
                    .build_conditional_branch(flt, fill_bdy, fill_ext);
                self.builder.position_at_end(fill_bdy);
                let cur_pow = self
                    .builder
                    .build_load(self.i64_ty(), p10_a, "cur_pow")
                    .map_err(llvm_err)?
                    .into_int_value();
                let cur_n = self
                    .builder
                    .build_load(self.i64_ty(), tmp_a, "cur_n")
                    .map_err(llvm_err)?
                    .into_int_value();
                let q = self
                    .builder
                    .build_int_signed_div(cur_n, cur_pow, "q")
                    .map_err(llvm_err)?;
                let digit = self
                    .builder
                    .build_int_signed_rem(q, ten, "digit")
                    .map_err(llvm_err)?;
                // Build fat struct {digit, null} and push
                let undef = self.string_type.get_undef();
                let d1 = self
                    .builder
                    .build_insert_value(undef, digit, 0, "d1")
                    .map_err(llvm_err)?;
                let d2 = self
                    .builder
                    .build_insert_value(d1, self.ptr_ty().const_zero(), 1, "d2")
                    .map_err(llvm_err)?;
                let rl = self
                    .builder
                    .build_load(self.list_type, res_a, "rl")
                    .map_err(llvm_err)?
                    .into_struct_value();
                let rp = self.call_rt(
                    "action_list_push",
                    &[rl.into(), d2.as_basic_value_enum().into()],
                )?;
                self.builder
                    .build_store(res_a, rp.try_as_basic_value().unwrap_basic())
                    .map_err(llvm_err)?;
                // Advance: i++, pow10 /= 10
                let ndi = self
                    .builder
                    .build_int_add(div, one, "ndi")
                    .map_err(llvm_err)?;
                self.builder.build_store(di_a, ndi).map_err(llvm_err)?;
                let np10 = self
                    .builder
                    .build_int_signed_div(cur_pow, ten, "np10")
                    .map_err(llvm_err)?;
                self.builder.build_store(p10_a, np10).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(fill_hdr);
                self.builder.position_at_end(fill_ext);
                Ok(TypedValue::List(res_a))
            }
            "charAt" => {
                if args.len() != 2 {
                    return Err("charAt expects 2 arguments (string, index)".to_string());
                }
                let s = self.compile_expr(&args[0])?;
                let idx = self.compile_expr(&args[1])?;
                let s_ptr = match s {
                    TypedValue::Str(p) => p,
                    _ => return Err("charAt: first argument must be a string".to_string()),
                };
                let idx_val = match idx {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("charAt: second argument must be an int".to_string()),
                };
                let ss = self.load_string(s_ptr)?;
                let slen = self
                    .builder
                    .build_extract_value(ss, 0, "slen")
                    .map_err(llvm_err)?
                    .into_int_value();
                let sdata = self
                    .builder
                    .build_extract_value(ss, 1, "sdata")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                // Clamp negative index
                let zero = self.i64_ty().const_int(0, false);
                let neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, idx_val, zero, "neg")
                    .map_err(llvm_err)?;
                let adj_idx = self
                    .builder
                    .build_int_add(slen, idx_val, "adj")
                    .map_err(llvm_err)?;
                let real_idx = self
                    .builder
                    .build_select(neg, adj_idx, idx_val, "real_idx")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Read leading byte and determine UTF-8 byte count
                let gep = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), sdata, &[real_idx], "gep")
                        .map_err(llvm_err)?
                };
                let ch = self
                    .builder
                    .build_load(self.context.i8_type(), gep, "ch")
                    .map_err(llvm_err)?
                    .into_int_value();
                let nbytes = self
                    .call_rt("action_utf8_byte_len", &[ch.into()])?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();
                // Allocate nbytes+1 (for null terminator)
                let alloc_sz = self
                    .builder
                    .build_int_add(nbytes, self.i64_ty().const_int(1, false), "alloc_sz")
                    .map_err(llvm_err)?;
                let malloc_fn = self.module.get_function("malloc").unwrap();
                let buf = self
                    .builder
                    .build_call(malloc_fn, &[alloc_sz.into()], "buf")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
                // memcpy nbytes from sdata+real_idx to buf
                let memcpy_fn = self.module.get_function("memcpy").unwrap();
                let src = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), sdata, &[real_idx], "src")
                        .map_err(llvm_err)
                }?;
                let _ = self
                    .builder
                    .build_call(memcpy_fn, &[buf.into(), src.into(), nbytes.into()], "")
                    .map_err(llvm_err)?;
                // Null terminate
                let null_pos = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), buf, &[nbytes], "null_pos")
                        .map_err(llvm_err)
                }?;
                self.builder
                    .build_store(null_pos, self.context.i8_type().const_int(0, false))
                    .map_err(llvm_err)?;
                // Build string struct
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, nbytes, 0, "r1")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, buf, 1, "r2")
                    .map_err(llvm_err)?;
                let sa = self
                    .builder
                    .build_alloca(self.string_type, "char_s")
                    .map_err(llvm_err)?;
                self.builder.build_store(sa, r2).map_err(llvm_err)?;
                Ok(TypedValue::Str(sa))
            }
            "isAlpha" => {
                if args.len() != 1 {
                    return Err("isAlpha expects 1 argument (char)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let ch = match v {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("isAlpha: argument must be a char code (int)".to_string()),
                };
                let a_lower = self.i64_ty().const_int('a' as u64, false);
                let z_lower = self.i64_ty().const_int('z' as u64, false);
                let a_upper = self.i64_ty().const_int('A' as u64, false);
                let z_upper = self.i64_ty().const_int('Z' as u64, false);
                let is_lower = self
                    .builder
                    .build_and(
                        self.builder
                            .build_int_compare(IntPredicate::SGE, ch, a_lower, "ge_a")
                            .map_err(llvm_err)?,
                        self.builder
                            .build_int_compare(IntPredicate::SLE, ch, z_lower, "le_z")
                            .map_err(llvm_err)?,
                        "is_lower",
                    )
                    .map_err(llvm_err)?;
                let is_upper = self
                    .builder
                    .build_and(
                        self.builder
                            .build_int_compare(IntPredicate::SGE, ch, a_upper, "ge_A")
                            .map_err(llvm_err)?,
                        self.builder
                            .build_int_compare(IntPredicate::SLE, ch, z_upper, "le_Z")
                            .map_err(llvm_err)?,
                        "is_upper",
                    )
                    .map_err(llvm_err)?;
                let result = self
                    .builder
                    .build_or(is_lower, is_upper, "isAlpha")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(result))
            }
            "codeToChar" => {
                if args.len() != 1 {
                    return Err("codeToChar expects 1 argument (int)".to_string());
                }
                let v = self.compile_expr(&args[0])?;
                let code = match v {
                    TypedValue::Int(iv) => iv,
                    _ => return Err("codeToChar: argument must be an int".to_string()),
                };
                let i64 = self.i64_ty();
                let i8 = self.context.i8_type();
                // Allocate 5 bytes (max 4 byte UTF-8 + null terminator)
                let malloc_fn = self.module.get_function("malloc").unwrap();
                let alloc_sz = i64.const_int(5, false);
                let buf = self
                    .builder
                    .build_call(malloc_fn, &[alloc_sz.into()], "buf")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
                // Call runtime UTF-8 encoder: nbytes = action_utf8_encode(code, buf)
                let nbytes = self
                    .call_rt("action_utf8_encode", &[code.into(), buf.into()])?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value();
                // Null terminate at position nbytes
                let null_g = unsafe {
                    self.builder
                        .build_gep(i8, buf, &[nbytes], "null_g")
                        .map_err(llvm_err)
                }?;
                self.builder
                    .build_store(null_g, i8.const_int(0, false))
                    .map_err(llvm_err)?;
                // Build string struct: { len: i64, data: i8* }
                let undef = self.string_type.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, nbytes, 0, "slen")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, buf, 1, "sdata")
                    .map_err(llvm_err)?;
                let sa = self
                    .builder
                    .build_alloca(self.string_type, "code_s")
                    .map_err(llvm_err)?;
                self.builder.build_store(sa, r2).map_err(llvm_err)?;
                Ok(TypedValue::Str(sa))
            }
            "nowUtc" => {
                if !args.is_empty() {
                    return Err("nowUtc expects no arguments".to_string());
                }
                let sty = self.context.struct_type(&[self.i64_ty().into(); 6], false);
                let alloca = self.builder.build_alloca(sty, "nowUtc").map_err(llvm_err)?;
                let time_fn = self
                    .module
                    .get_function("time")
                    .ok_or("time function not found")?;
                let null_ptr = self.ptr_ty().const_null();
                let ts = self
                    .builder
                    .build_call(time_fn, &[null_ptr.into()], "ts")
                    .map_err(llvm_err)?;
                let ts_val = ts.try_as_basic_value().unwrap_basic().into_int_value();
                let gmtime_fn = self
                    .module
                    .get_function("gmtime_r")
                    .ok_or("gmtime_r function not found")?;
                let tm_ptr = self.builder.build_alloca(sty, "tm").map_err(llvm_err)?;
                let gmtime_call = self
                    .builder
                    .build_call(gmtime_fn, &[ts_val.into(), tm_ptr.into()], "")
                    .map_err(llvm_err)?;
                let _ = gmtime_call.try_as_basic_value().basic();
                // Copy tm struct to result (year+1900, month, day, hour, min, sec)
                for i in 0..6u32 {
                    let src_p = self
                        .builder
                        .build_struct_gep(sty, tm_ptr, i, "tm_f")
                        .map_err(llvm_err)?;
                    let val = self
                        .builder
                        .build_load(self.i64_ty(), src_p, "val")
                        .map_err(llvm_err)?;
                    let dst_p = self
                        .builder
                        .build_struct_gep(sty, alloca, i, "dst_f")
                        .map_err(llvm_err)?;
                    self.builder.build_store(dst_p, val).map_err(llvm_err)?;
                }
                // Fix year: tm_year is years since 1900
                let yp = self
                    .builder
                    .build_struct_gep(sty, alloca, 0, "yp")
                    .map_err(llvm_err)?;
                let yv = self
                    .builder
                    .build_load(self.i64_ty(), yp, "yv")
                    .map_err(llvm_err)?
                    .into_int_value();
                let ya = self
                    .builder
                    .build_int_add(yv, self.i64_ty().const_int(1900, false), "ya")
                    .map_err(llvm_err)?;
                self.builder.build_store(yp, ya).map_err(llvm_err)?;
                Ok(TypedValue::Struct(alloca, sty))
            }
            "diffSeconds" => {
                if args.len() != 2 {
                    return Err("diffSeconds expects 2 arguments (dt1, dt2)".to_string());
                }
                let d1 = self.compile_expr(&args[0])?;
                let d2 = self.compile_expr(&args[1])?;
                let (p1, st1) = match d1 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffSeconds: arguments must be DateTime structs".to_string()),
                };
                let (p2, _st2) = match d2 {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("diffSeconds: arguments must be DateTime structs".to_string()),
                };
                let i64_ty = self.i64_ty();
                // Approximate seconds from year/month/day/hour/min/sec
                let extract = |builder: &inkwell::builder::Builder<'ctx>,
                               p: PointerValue<'ctx>,
                               st: inkwell::types::StructType<'ctx>|
                 -> Result<IntValue<'ctx>, String> {
                    let yp = builder.build_struct_gep(st, p, 0, "yp").map_err(llvm_err)?;
                    let y = builder
                        .build_load(i64_ty, yp, "y")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let mp = builder.build_struct_gep(st, p, 1, "mp").map_err(llvm_err)?;
                    let m = builder
                        .build_load(i64_ty, mp, "m")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let dp = builder.build_struct_gep(st, p, 2, "dp").map_err(llvm_err)?;
                    let d = builder
                        .build_load(i64_ty, dp, "d")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let hp = builder.build_struct_gep(st, p, 3, "hp").map_err(llvm_err)?;
                    let h = builder
                        .build_load(i64_ty, hp, "h")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let minp = builder
                        .build_struct_gep(st, p, 4, "minp")
                        .map_err(llvm_err)?;
                    let minv = builder
                        .build_load(i64_ty, minp, "min")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let sp = builder.build_struct_gep(st, p, 5, "sp").map_err(llvm_err)?;
                    let s = builder
                        .build_load(i64_ty, sp, "s")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let d365 = builder
                        .build_int_mul(y, i64_ty.const_int(365, false), "d365")
                        .map_err(llvm_err)?;
                    let d30 = builder
                        .build_int_mul(m, i64_ty.const_int(30, false), "d30")
                        .map_err(llvm_err)?;
                    let days = builder
                        .build_int_add(
                            builder.build_int_add(d365, d30, "d1").map_err(llvm_err)?,
                            d,
                            "d2",
                        )
                        .map_err(llvm_err)?;
                    let secs_per_day = i64_ty.const_int(86400, false);
                    let ds = builder
                        .build_int_mul(days, secs_per_day, "ds")
                        .map_err(llvm_err)?;
                    let hs = builder
                        .build_int_mul(h, i64_ty.const_int(3600, false), "hs")
                        .map_err(llvm_err)?;
                    let ms = builder
                        .build_int_mul(minv, i64_ty.const_int(60, false), "ms")
                        .map_err(llvm_err)?;
                    let total = builder
                        .build_int_add(
                            builder
                                .build_int_add(
                                    builder.build_int_add(ds, hs, "t1").map_err(llvm_err)?,
                                    ms,
                                    "t2",
                                )
                                .map_err(llvm_err)?,
                            s,
                            "t3",
                        )
                        .map_err(llvm_err)?;
                    Ok(total)
                };
                let t1 = extract(&self.builder, p1, st1)?;
                let t2 = extract(&self.builder, p2, st1)?;
                let diff = self
                    .builder
                    .build_int_sub(t1, t2, "diff")
                    .map_err(llvm_err)?;
                // Absolute value
                let zero = self.i64_ty().const_int(0, false);
                let nd = self.builder.build_int_neg(diff, "nd").map_err(llvm_err)?;
                let is_neg = self
                    .builder
                    .build_int_compare(IntPredicate::SLT, diff, zero, "is_neg")
                    .map_err(llvm_err)?;
                let abs_diff = self
                    .builder
                    .build_select(is_neg, nd, diff, "abs_diff")
                    .map_err(llvm_err)?
                    .into_int_value();
                Ok(TypedValue::Int(abs_diff))
            }
            "format" => {
                if args.len() != 2 {
                    return Err("format expects 2 arguments (datetime, format_str)".to_string());
                }
                let dt = self.compile_expr(&args[0])?;
                let fmt = self.compile_expr(&args[1])?;
                match (&dt, &fmt) {
                    (TypedValue::Struct(dt_ptr, dt_st), TypedValue::Str(fmt_ptr)) => {
                        let fmt_val = self.load_string(*fmt_ptr)?;
                        let fmt_data = self
                            .builder
                            .build_extract_value(fmt_val, 1, "fmt_data")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        // Extract DateTime fields: {year, month, day, hour, minute, second}
                        let extract_field = |i: u32| -> Result<IntValue, String> {
                            let fptr = self
                                .builder
                                .build_struct_gep(*dt_st, *dt_ptr, i, "dt_f")
                                .map_err(llvm_err)?;
                            let val = self
                                .builder
                                .build_load(self.i64_ty(), fptr, "dt_v")
                                .map_err(llvm_err)?
                                .into_int_value();
                            Ok(val)
                        };
                        let year = extract_field(0)?;
                        let month = extract_field(1)?;
                        let day = extract_field(2)?;
                        let hour = extract_field(3)?;
                        let minute = extract_field(4)?;
                        let second = extract_field(5)?;
                        // Build struct tm: {i32 x 9}
                        let i32 = self.context.i32_type();
                        let tm_ty = self.context.struct_type(&[i32.into(); 9], false);
                        let tm_a = self.builder.build_alloca(tm_ty, "tm").map_err(llvm_err)?;
                        // tm_sec = second
                        let tm_sec = self
                            .builder
                            .build_int_truncate(second, i32, "tm_sec")
                            .map_err(llvm_err)?;
                        let f0 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 0, "f0")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f0, tm_sec).map_err(llvm_err)?;
                        // tm_min = minute
                        let tm_min = self
                            .builder
                            .build_int_truncate(minute, i32, "tm_min")
                            .map_err(llvm_err)?;
                        let f1 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 1, "f1")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f1, tm_min).map_err(llvm_err)?;
                        // tm_hour = hour
                        let tm_hour = self
                            .builder
                            .build_int_truncate(hour, i32, "tm_hour")
                            .map_err(llvm_err)?;
                        let f2 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 2, "f2")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f2, tm_hour).map_err(llvm_err)?;
                        // tm_mday = day
                        let tm_mday = self
                            .builder
                            .build_int_truncate(day, i32, "tm_mday")
                            .map_err(llvm_err)?;
                        let f3 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 3, "f3")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f3, tm_mday).map_err(llvm_err)?;
                        // tm_mon = month - 1
                        let mon_minus = self
                            .builder
                            .build_int_sub(month, self.i64_ty().const_int(1, false), "mon_minus")
                            .map_err(llvm_err)?;
                        let tm_mon = self
                            .builder
                            .build_int_truncate(mon_minus, i32, "tm_mon")
                            .map_err(llvm_err)?;
                        let f4 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 4, "f4")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f4, tm_mon).map_err(llvm_err)?;
                        // tm_year = year - 1900
                        let year_minus = self
                            .builder
                            .build_int_sub(year, self.i64_ty().const_int(1900, false), "year_minus")
                            .map_err(llvm_err)?;
                        let tm_year = self
                            .builder
                            .build_int_truncate(year_minus, i32, "tm_year")
                            .map_err(llvm_err)?;
                        let f5 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 5, "f5")
                            .map_err(llvm_err)?;
                        self.builder.build_store(f5, tm_year).map_err(llvm_err)?;
                        // tm_wday = 0
                        let f6 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 6, "f6")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f6, i32.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_yday = 0
                        let f7 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 7, "f7")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f7, i32.const_int(0, false))
                            .map_err(llvm_err)?;
                        // tm_isdst = -1
                        let f8 = self
                            .builder
                            .build_struct_gep(tm_ty, tm_a, 8, "f8")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(f8, i32.const_int(0xFFFFFFFFu64 as u64, false))
                            .map_err(llvm_err)?;
                        // Allocate buffer and call strftime
                        let buf_size = self.i64_ty().const_int(256, false);
                        let malloc_fn = self.module.get_function("malloc").unwrap();
                        let buf = self
                            .builder
                            .build_call(malloc_fn, &[buf_size.into()], "fmt_buf")
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_pointer_value();
                        let strftime_fn = self
                            .module
                            .get_function("strftime")
                            .ok_or("strftime not found")?;
                        let _ = self
                            .builder
                            .build_call(
                                strftime_fn,
                                &[buf.into(), buf_size.into(), fmt_data.into(), tm_a.into()],
                                "",
                            )
                            .map_err(llvm_err)?;
                        // Build Atomic string: {i64, i8*} with strlen
                        let strlen_fn = self
                            .module
                            .get_function("strlen")
                            .ok_or("strlen not found")?;
                        let len = self
                            .builder
                            .build_call(strlen_fn, &[buf.into()], "fmt_len")
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_int_value();
                        let fat = self.string_type.get_undef();
                        let r1 = self
                            .builder
                            .build_insert_value(fat, len, 0, "r1")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_insert_value(r1, buf, 1, "r2")
                            .map_err(llvm_err)?;
                        let alloca = self
                            .builder
                            .build_alloca(self.string_type, "fmt_str")
                            .map_err(llvm_err)?;
                        self.builder.build_store(alloca, r2).map_err(llvm_err)?;
                        Ok(TypedValue::Str(alloca))
                    }
                    _ => Err("format: expects (DateTime, String)".to_string()),
                }
            }
            "parseDate" => {
                if args.len() != 2 {
                    return Err("parseDate expects 2 arguments (format_str, date_str)".to_string());
                }
                let fmt_v = self.compile_expr(&args[0])?;
                let date_v = self.compile_expr(&args[1])?;
                match (&fmt_v, &date_v) {
                    (TypedValue::Str(_fmt_ptr), TypedValue::Str(date_ptr)) => {
                        let date_val = self.load_string(*date_ptr)?;
                        let date_data = self
                            .builder
                            .build_extract_value(date_val, 1, "pd_date")
                            .map_err(llvm_err)?
                            .into_pointer_value();
                        // Use sscanf to parse the date string with format "%d-%d-%d"
                        let i32_ty = self.context.i32_type();
                        let sscanf_ty = self
                            .i32_ty()
                            .fn_type(&[self.ptr_ty().into(), self.ptr_ty().into()], true);
                        let sscanf_fn = self
                            .module
                            .get_function("sscanf")
                            .unwrap_or_else(|| self.module.add_function("sscanf", sscanf_ty, None));
                        // Stack-allocate year, month, day as i32
                        let y_ptr = self
                            .builder
                            .build_alloca(i32_ty, "pd_y")
                            .map_err(llvm_err)?;
                        let m_ptr = self
                            .builder
                            .build_alloca(i32_ty, "pd_m")
                            .map_err(llvm_err)?;
                        let d_ptr = self
                            .builder
                            .build_alloca(i32_ty, "pd_d")
                            .map_err(llvm_err)?;
                        let fmt_str = self
                            .builder
                            .build_global_string_ptr("%d-%d-%d", "pd_fmt")
                            .map_err(llvm_err)?;
                        let ret = self
                            .builder
                            .build_call(
                                sscanf_fn,
                                &[
                                    date_data.into(),
                                    fmt_str.as_pointer_value().into(),
                                    y_ptr.into(),
                                    m_ptr.into(),
                                    d_ptr.into(),
                                ],
                                "pd_ret",
                            )
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_int_value();
                        let ok = self
                            .builder
                            .build_int_compare(
                                IntPredicate::EQ,
                                ret,
                                i32_ty.const_int(3, false),
                                "pd_ok",
                            )
                            .map_err(llvm_err)?;
                        // Build Option<Date>
                        let enum_ty = self
                            .context
                            .struct_type(&[self.i64_ty().into(), self.ptr_ty().into()], false);
                        let some_sty =
                            self.named_structs.get("Date").copied().unwrap_or_else(|| {
                                self.context.struct_type(&[self.i64_ty().into(); 3], false)
                            });
                        let current_fn = self
                            .builder
                            .get_insert_block()
                            .and_then(|b| b.get_parent())
                            .ok_or("no fn")?;
                        let some_bb = self.context.append_basic_block(current_fn, "pd_some");
                        let none_bb = self.context.append_basic_block(current_fn, "pd_none");
                        let merge_bb = self.context.append_basic_block(current_fn, "pd_merge");
                        let _ = self.builder.build_conditional_branch(ok, some_bb, none_bb);
                        // Some branch
                        self.builder.position_at_end(some_bb);
                        let y_val = self
                            .builder
                            .build_load(i32_ty, y_ptr, "pd_yv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let m_val = self
                            .builder
                            .build_load(i32_ty, m_ptr, "pd_mv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let d_val = self
                            .builder
                            .build_load(i32_ty, d_ptr, "pd_dv")
                            .map_err(llvm_err)?
                            .into_int_value();
                        let year_i64 = self
                            .builder
                            .build_int_s_extend(y_val, self.i64_ty(), "py")
                            .map_err(llvm_err)?;
                        let month_i64 = self
                            .builder
                            .build_int_s_extend(m_val, self.i64_ty(), "pm")
                            .map_err(llvm_err)?;
                        let day_i64 = self
                            .builder
                            .build_int_s_extend(d_val, self.i64_ty(), "pd")
                            .map_err(llvm_err)?;
                        let date_size = self.i64_ty().const_int(24, false);
                        let malloc_fn = self.module.get_function("malloc").unwrap();
                        let heap = self
                            .builder
                            .build_call(malloc_fn, &[date_size.into()], "pd_heap")
                            .map_err(llvm_err)?
                            .try_as_basic_value()
                            .unwrap_basic()
                            .into_pointer_value();
                        let dp = self
                            .builder
                            .build_pointer_cast(heap, self.ptr_ty(), "dp")
                            .map_err(llvm_err)?;
                        let yp = self
                            .builder
                            .build_struct_gep(some_sty, dp, 0, "yp")
                            .map_err(llvm_err)?;
                        self.builder.build_store(yp, year_i64).map_err(llvm_err)?;
                        let mp = self
                            .builder
                            .build_struct_gep(some_sty, dp, 1, "mp")
                            .map_err(llvm_err)?;
                        self.builder.build_store(mp, month_i64).map_err(llvm_err)?;
                        let dap = self
                            .builder
                            .build_struct_gep(some_sty, dp, 2, "dap")
                            .map_err(llvm_err)?;
                        self.builder.build_store(dap, day_i64).map_err(llvm_err)?;
                        let undef = enum_ty.get_undef();
                        let r1 = self
                            .builder
                            .build_insert_value(undef, self.i64_ty().const_int(0, false), 0, "r1")
                            .map_err(llvm_err)?;
                        let r2 = self
                            .builder
                            .build_insert_value(r1, heap, 1, "r2")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // None branch
                        self.builder.position_at_end(none_bb);
                        let undef2 = enum_ty.get_undef();
                        let r3 = self
                            .builder
                            .build_insert_value(undef2, self.i64_ty().const_int(1, false), 0, "r3")
                            .map_err(llvm_err)?;
                        let r4 = self
                            .builder
                            .build_insert_value(r3, self.ptr_ty().const_null(), 1, "r4")
                            .map_err(llvm_err)?;
                        let _ = self.builder.build_unconditional_branch(merge_bb);
                        // Merge with phi
                        self.builder.position_at_end(merge_bb);
                        let phi = self
                            .builder
                            .build_phi(enum_ty, "pd_phi")
                            .map_err(llvm_err)?;
                        phi.add_incoming(&[(&r2, some_bb), (&r4, none_bb)]);
                        let result_alloca = self
                            .builder
                            .build_alloca(enum_ty, "pd_result")
                            .map_err(llvm_err)?;
                        self.builder
                            .build_store(result_alloca, phi.as_basic_value())
                            .map_err(llvm_err)?;
                        Ok(TypedValue::Enum(
                            result_alloca,
                            enum_ty,
                            InnerType::Int,
                            false,
                        ))
                    }
                    _ => Err("parseDate: expects (String, String)".to_string()),
                }
            }
            "date" => {
                if args.len() != 3 {
                    return Err("date expects 3 arguments (year, month, day)".to_string());
                }
                let yv = self.compile_expr(&args[0])?;
                let mv = self.compile_expr(&args[1])?;
                let dv = self.compile_expr(&args[2])?;
                let y = yv.to_bv().ok_or("year must be Int")?.into_int_value();
                let m = mv.to_bv().ok_or("month must be Int")?.into_int_value();
                let d = dv.to_bv().ok_or("day must be Int")?.into_int_value();
                let i64_ty = self.i64_ty();
                let zero = i64_ty.const_int(0, false);
                let one = i64_ty.const_int(1, false);
                // year >= 1
                let y_ok = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, y, one, "y_ok")
                    .map_err(llvm_err)?;
                // 1 <= month <= 12
                let m_ge1 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, m, one, "m_ge")
                    .map_err(llvm_err)?;
                let m_le12 = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, m, i64_ty.const_int(12, false), "m_le")
                    .map_err(llvm_err)?;
                let m_ok = self
                    .builder
                    .build_and(m_ge1, m_le12, "m_ok")
                    .map_err(llvm_err)?;
                // Leap year: (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
                let y_mod4 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(4, false), "ym4")
                    .map_err(llvm_err)?;
                let y_mod100 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(100, false), "ym100")
                    .map_err(llvm_err)?;
                let y_mod400 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(400, false), "ym400")
                    .map_err(llvm_err)?;
                let div4_ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, y_mod4, zero, "d4")
                    .map_err(llvm_err)?;
                let div100_ok = self
                    .builder
                    .build_int_compare(IntPredicate::NE, y_mod100, zero, "d100")
                    .map_err(llvm_err)?;
                let div400_ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, y_mod400, zero, "d400")
                    .map_err(llvm_err)?;
                let leap_part1 = self
                    .builder
                    .build_and(div4_ok, div100_ok, "lp1")
                    .map_err(llvm_err)?;
                let is_leap = self
                    .builder
                    .build_or(leap_part1, div400_ok, "is_leap")
                    .map_err(llvm_err)?;
                // feb_days = is_leap ? 29 : 28
                let feb_days = self
                    .builder
                    .build_select(
                        is_leap,
                        i64_ty.const_int(29, false),
                        i64_ty.const_int(28, false),
                        "feb",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                // max_days based on month:
                // month 2 -> feb_days
                // month 4,6,9,11 -> 30
                // month 1,3,5,7,8,10,12 -> 31
                let is_feb = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(2, false), "is_feb")
                    .map_err(llvm_err)?;
                let is_30d = {
                    let m4 = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(4, false), "m4")
                        .map_err(llvm_err)?;
                    let m6 = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(6, false), "m6")
                        .map_err(llvm_err)?;
                    let m9 = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(9, false), "m9")
                        .map_err(llvm_err)?;
                    let m11 = self
                        .builder
                        .build_int_compare(IntPredicate::EQ, m, i64_ty.const_int(11, false), "m11")
                        .map_err(llvm_err)?;
                    let t1 = self.builder.build_or(m4, m6, "t1").map_err(llvm_err)?;
                    let t2 = self.builder.build_or(m9, m11, "t2").map_err(llvm_err)?;
                    self.builder.build_or(t1, t2, "is_30d").map_err(llvm_err)?
                };
                let max_days_30or31 = self
                    .builder
                    .build_select(
                        is_30d,
                        i64_ty.const_int(30, false),
                        i64_ty.const_int(31, false),
                        "md_30or31",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                let max_days = self
                    .builder
                    .build_select(is_feb, feb_days, max_days_30or31, "max_days")
                    .map_err(llvm_err)?
                    .into_int_value();
                let d_ge1 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, d, one, "d_ge")
                    .map_err(llvm_err)?;
                let d_le_max = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, d, max_days, "d_le")
                    .map_err(llvm_err)?;
                let d_ok = self
                    .builder
                    .build_and(d_ge1, d_le_max, "d_ok")
                    .map_err(llvm_err)?;
                let ym_ok = self
                    .builder
                    .build_and(y_ok, m_ok, "ym_ok")
                    .map_err(llvm_err)?;
                let is_valid = self
                    .builder
                    .build_and(ym_ok, d_ok, "is_valid")
                    .map_err(llvm_err)?;
                // Build Option<Date>
                let enum_ty = self
                    .context
                    .struct_type(&[i64_ty.into(), self.ptr_ty().into()], false);
                let date_sty = self
                    .named_structs
                    .get("Date")
                    .copied()
                    .unwrap_or_else(|| self.context.struct_type(&[i64_ty.into(); 3], false));
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no fn")?;
                let some_bb = self.context.append_basic_block(current_fn, "d_some");
                let none_bb = self.context.append_basic_block(current_fn, "d_none");
                let merge_bb = self.context.append_basic_block(current_fn, "d_merge");
                let _ = self
                    .builder
                    .build_conditional_branch(is_valid, some_bb, none_bb);
                self.builder.position_at_end(some_bb);
                let date_size = i64_ty.const_int(24, false);
                let malloc_fn = self.module.get_function("malloc").unwrap();
                let heap = self
                    .builder
                    .build_call(malloc_fn, &[date_size.into()], "d_heap")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
                let yp = self
                    .builder
                    .build_struct_gep(date_sty, heap, 0, "d_yp")
                    .map_err(llvm_err)?;
                self.builder.build_store(yp, y).map_err(llvm_err)?;
                let mp = self
                    .builder
                    .build_struct_gep(date_sty, heap, 1, "d_mp")
                    .map_err(llvm_err)?;
                self.builder.build_store(mp, m).map_err(llvm_err)?;
                let dp = self
                    .builder
                    .build_struct_gep(date_sty, heap, 2, "d_dp")
                    .map_err(llvm_err)?;
                self.builder.build_store(dp, d).map_err(llvm_err)?;
                let undef = enum_ty.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, i64_ty.const_int(0, false), 0, "r1")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, heap, 1, "r2")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(none_bb);
                let undef2 = enum_ty.get_undef();
                let r3 = self
                    .builder
                    .build_insert_value(undef2, i64_ty.const_int(1, false), 0, "r3")
                    .map_err(llvm_err)?;
                let r4 = self
                    .builder
                    .build_insert_value(r3, self.ptr_ty().const_null(), 1, "r4")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(merge_bb);
                let phi = self.builder.build_phi(enum_ty, "d_phi").map_err(llvm_err)?;
                phi.add_incoming(&[(&r2, some_bb), (&r4, none_bb)]);
                let result_alloca = self
                    .builder
                    .build_alloca(enum_ty, "d_result")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(result_alloca, phi.as_basic_value())
                    .map_err(llvm_err)?;
                Ok(TypedValue::Enum(
                    result_alloca,
                    enum_ty,
                    InnerType::Int,
                    false,
                ))
            }
            "datetime" => {
                if args.len() != 6 {
                    return Err(
                        "datetime expects 6 arguments (year, month, day, hour, minute, second)"
                            .to_string(),
                    );
                }
                let yv = self.compile_expr(&args[0])?;
                let mov = self.compile_expr(&args[1])?;
                let dv = self.compile_expr(&args[2])?;
                let hv = self.compile_expr(&args[3])?;
                let minv = self.compile_expr(&args[4])?;
                let sv = self.compile_expr(&args[5])?;
                let y = yv.to_bv().ok_or("year must be Int")?.into_int_value();
                let mo = mov.to_bv().ok_or("month must be Int")?.into_int_value();
                let d = dv.to_bv().ok_or("day must be Int")?.into_int_value();
                let h = hv.to_bv().ok_or("hour must be Int")?.into_int_value();
                let min = minv.to_bv().ok_or("minute must be Int")?.into_int_value();
                let s = sv.to_bv().ok_or("second must be Int")?.into_int_value();
                let i64_ty = self.i64_ty();
                let zero = i64_ty.const_int(0, false);
                let one = i64_ty.const_int(1, false);
                // Validate year, month, day (same as date)
                let y_ok = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, y, one, "y_ok")
                    .map_err(llvm_err)?;
                let m_ge1 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, mo, one, "m_ge")
                    .map_err(llvm_err)?;
                let m_le12 = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, mo, i64_ty.const_int(12, false), "m_le")
                    .map_err(llvm_err)?;
                let m_ok = self
                    .builder
                    .build_and(m_ge1, m_le12, "m_ok")
                    .map_err(llvm_err)?;
                let y_mod4 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(4, false), "ym4")
                    .map_err(llvm_err)?;
                let y_mod100 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(100, false), "ym100")
                    .map_err(llvm_err)?;
                let y_mod400 = self
                    .builder
                    .build_int_signed_rem(y, i64_ty.const_int(400, false), "ym400")
                    .map_err(llvm_err)?;
                let div4_ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, y_mod4, zero, "d4")
                    .map_err(llvm_err)?;
                let div100_ok = self
                    .builder
                    .build_int_compare(IntPredicate::NE, y_mod100, zero, "d100")
                    .map_err(llvm_err)?;
                let div400_ok = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, y_mod400, zero, "d400")
                    .map_err(llvm_err)?;
                let leap_part1 = self
                    .builder
                    .build_and(div4_ok, div100_ok, "lp1")
                    .map_err(llvm_err)?;
                let is_leap = self
                    .builder
                    .build_or(leap_part1, div400_ok, "is_leap")
                    .map_err(llvm_err)?;
                let feb_days = self
                    .builder
                    .build_select(
                        is_leap,
                        i64_ty.const_int(29, false),
                        i64_ty.const_int(28, false),
                        "feb",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                let is_feb = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(2, false), "is_feb")
                    .map_err(llvm_err)?;
                let m4 = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(4, false), "m4")
                    .map_err(llvm_err)?;
                let m6 = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(6, false), "m6")
                    .map_err(llvm_err)?;
                let m9 = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(9, false), "m9")
                    .map_err(llvm_err)?;
                let m11 = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, mo, i64_ty.const_int(11, false), "m11")
                    .map_err(llvm_err)?;
                let t1 = self.builder.build_or(m4, m6, "t1").map_err(llvm_err)?;
                let t2 = self.builder.build_or(m9, m11, "t2").map_err(llvm_err)?;
                let is_30d = self.builder.build_or(t1, t2, "is_30d").map_err(llvm_err)?;
                let max_days_30or31 = self
                    .builder
                    .build_select(
                        is_30d,
                        i64_ty.const_int(30, false),
                        i64_ty.const_int(31, false),
                        "md_30or31",
                    )
                    .map_err(llvm_err)?
                    .into_int_value();
                let max_days = self
                    .builder
                    .build_select(is_feb, feb_days, max_days_30or31, "max_days")
                    .map_err(llvm_err)?
                    .into_int_value();
                let d_ge1 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, d, one, "d_ge")
                    .map_err(llvm_err)?;
                let d_le_max = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, d, max_days, "d_le")
                    .map_err(llvm_err)?;
                let d_ok = self
                    .builder
                    .build_and(d_ge1, d_le_max, "d_ok")
                    .map_err(llvm_err)?;
                // hour 0-23, minute 0-59, second 0-59
                let h_ge0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, h, zero, "h_ge")
                    .map_err(llvm_err)?;
                let h_le23 = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, h, i64_ty.const_int(23, false), "h_le")
                    .map_err(llvm_err)?;
                let h_ok = self
                    .builder
                    .build_and(h_ge0, h_le23, "h_ok")
                    .map_err(llvm_err)?;
                let min_ge0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, min, zero, "min_ge")
                    .map_err(llvm_err)?;
                let min_le59 = self
                    .builder
                    .build_int_compare(
                        IntPredicate::SLE,
                        min,
                        i64_ty.const_int(59, false),
                        "min_le",
                    )
                    .map_err(llvm_err)?;
                let min_ok = self
                    .builder
                    .build_and(min_ge0, min_le59, "min_ok")
                    .map_err(llvm_err)?;
                let s_ge0 = self
                    .builder
                    .build_int_compare(IntPredicate::SGE, s, zero, "s_ge")
                    .map_err(llvm_err)?;
                let s_le59 = self
                    .builder
                    .build_int_compare(IntPredicate::SLE, s, i64_ty.const_int(59, false), "s_le")
                    .map_err(llvm_err)?;
                let s_ok = self
                    .builder
                    .build_and(s_ge0, s_le59, "s_ok")
                    .map_err(llvm_err)?;
                let ym_ok = self
                    .builder
                    .build_and(y_ok, m_ok, "ym_ok")
                    .map_err(llvm_err)?;
                let ymd_ok = self
                    .builder
                    .build_and(ym_ok, d_ok, "ymd_ok")
                    .map_err(llvm_err)?;
                let hms_ok = self
                    .builder
                    .build_and(
                        self.builder
                            .build_and(h_ok, min_ok, "hm_ok")
                            .map_err(llvm_err)?,
                        s_ok,
                        "hms_ok",
                    )
                    .map_err(llvm_err)?;
                let is_valid = self
                    .builder
                    .build_and(ymd_ok, hms_ok, "is_valid")
                    .map_err(llvm_err)?;
                // Build Option<DateTime>
                let enum_ty = self
                    .context
                    .struct_type(&[i64_ty.into(), self.ptr_ty().into()], false);
                let dt_sty = self
                    .named_structs
                    .get("DateTime")
                    .copied()
                    .unwrap_or_else(|| self.context.struct_type(&[i64_ty.into(); 6], false));
                let current_fn = self
                    .builder
                    .get_insert_block()
                    .and_then(|b| b.get_parent())
                    .ok_or("no fn")?;
                let some_bb = self.context.append_basic_block(current_fn, "dt_some");
                let none_bb = self.context.append_basic_block(current_fn, "dt_none");
                let merge_bb = self.context.append_basic_block(current_fn, "dt_merge");
                let _ = self
                    .builder
                    .build_conditional_branch(is_valid, some_bb, none_bb);
                self.builder.position_at_end(some_bb);
                let dt_size = i64_ty.const_int(48, false); // 6 * 8 bytes
                let malloc_fn = self.module.get_function("malloc").unwrap();
                let heap = self
                    .builder
                    .build_call(malloc_fn, &[dt_size.into()], "dt_heap")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value();
                for (i, val) in [y, mo, d, h, min, s].iter().enumerate() {
                    let fp = self
                        .builder
                        .build_struct_gep(dt_sty, heap, i as u32, "dt_f")
                        .map_err(llvm_err)?;
                    self.builder.build_store(fp, *val).map_err(llvm_err)?;
                }
                let undef = enum_ty.get_undef();
                let r1 = self
                    .builder
                    .build_insert_value(undef, i64_ty.const_int(0, false), 0, "r1")
                    .map_err(llvm_err)?;
                let r2 = self
                    .builder
                    .build_insert_value(r1, heap, 1, "r2")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(none_bb);
                let undef2 = enum_ty.get_undef();
                let r3 = self
                    .builder
                    .build_insert_value(undef2, i64_ty.const_int(1, false), 0, "r3")
                    .map_err(llvm_err)?;
                let r4 = self
                    .builder
                    .build_insert_value(r3, self.ptr_ty().const_null(), 1, "r4")
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(enum_ty, "dt_phi")
                    .map_err(llvm_err)?;
                phi.add_incoming(&[(&r2, some_bb), (&r4, none_bb)]);
                let result_alloca = self
                    .builder
                    .build_alloca(enum_ty, "dt_result")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(result_alloca, phi.as_basic_value())
                    .map_err(llvm_err)?;
                Ok(TypedValue::Enum(
                    result_alloca,
                    enum_ty,
                    InnerType::Int,
                    false,
                ))
            }
            "Random_new" => {
                if args.len() != 1 {
                    return Err("Random_new expects 1 argument (seed)".to_string());
                }
                let seed_v = self.compile_expr(&args[0])?;
                let seed = seed_v.to_bv().ok_or("seed must be Int")?.into_int_value();
                // Random struct is just {i64} wrapping the seed
                let rand_sty = self.context.struct_type(&[self.i64_ty().into()], false);
                let alloca = self
                    .builder
                    .build_alloca(rand_sty, "rand")
                    .map_err(llvm_err)?;
                let f0 = self
                    .builder
                    .build_struct_gep(rand_sty, alloca, 0, "f0")
                    .map_err(llvm_err)?;
                self.builder.build_store(f0, seed).map_err(llvm_err)?;
                Ok(TypedValue::Struct(alloca, rand_sty))
            }
            "nextInt" => {
                if args.len() != 3 {
                    return Err("nextInt expects 3 arguments (random, min, max)".to_string());
                }
                let rng_v = self.compile_expr(&args[0])?;
                let min_v = self.compile_expr(&args[1])?;
                let max_v = self.compile_expr(&args[2])?;
                let (rng_ptr, rng_st) = match rng_v {
                    TypedValue::Struct(p, st) => (p, st),
                    _ => return Err("nextInt: first argument must be a Random struct".to_string()),
                };
                let min = min_v.to_bv().ok_or("min must be Int")?.into_int_value();
                let max = max_v.to_bv().ok_or("max must be Int")?.into_int_value();
                let i64_ty = self.i64_ty();
                // Load current seed
                let f0 = self
                    .builder
                    .build_struct_gep(rng_st, rng_ptr, 0, "f0")
                    .map_err(llvm_err)?;
                let seed = self
                    .builder
                    .build_load(i64_ty, f0, "seed")
                    .map_err(llvm_err)?
                    .into_int_value();
                // xorshift64 PRNG
                // x ^= x << 13; x ^= x >> 7; x ^= x << 17
                let c13 = i64_ty.const_int(13, false);
                let c7 = i64_ty.const_int(7, false);
                let c17 = i64_ty.const_int(17, false);
                let x1 = self
                    .builder
                    .build_xor(
                        seed,
                        self.builder
                            .build_left_shift(seed, c13, "s1")
                            .map_err(llvm_err)?,
                        "x1",
                    )
                    .map_err(llvm_err)?;
                let x2 = self
                    .builder
                    .build_xor(
                        x1,
                        self.builder
                            .build_right_shift(x1, c7, false, "s2")
                            .map_err(llvm_err)?,
                        "x2",
                    )
                    .map_err(llvm_err)?;
                let x3 = self
                    .builder
                    .build_xor(
                        x2,
                        self.builder
                            .build_left_shift(x2, c17, "s3")
                            .map_err(llvm_err)?,
                        "x3",
                    )
                    .map_err(llvm_err)?;
                // Ensure non-zero (degenerates to 0 otherwise)
                let zero = i64_ty.const_int(0, false);
                let is_zero = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, x3, zero, "is_zero")
                    .map_err(llvm_err)?;
                let new_seed = self
                    .builder
                    .build_select(is_zero, i64_ty.const_int(1, false), x3, "new_seed")
                    .map_err(llvm_err)?
                    .into_int_value();
                // Compute value in [min, max] range
                let range = self
                    .builder
                    .build_int_sub(max, min, "range")
                    .map_err(llvm_err)?;
                let range_plus_1 = self
                    .builder
                    .build_int_add(range, i64_ty.const_int(1, false), "rp1")
                    .map_err(llvm_err)?;
                // Use unsigned remainder for proper range mapping
                let value = self
                    .builder
                    .build_int_unsigned_rem(new_seed, range_plus_1, "val_mod")
                    .map_err(llvm_err)?;
                let result = self
                    .builder
                    .build_int_add(value, min, "result")
                    .map_err(llvm_err)?;
                // Build result tuple (Random, Int)
                let rand_sty = rng_st;
                let tuple_sty = self
                    .context
                    .struct_type(&[rand_sty.into(), i64_ty.into()], false);
                let tup_alloca = self
                    .builder
                    .build_alloca(tuple_sty, "tup")
                    .map_err(llvm_err)?;
                // Store new Random
                let rng_field = self
                    .builder
                    .build_struct_gep(tuple_sty, tup_alloca, 0, "rf")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(rng_field, new_seed)
                    .map_err(llvm_err)?;
                // Store int result
                let int_field = self
                    .builder
                    .build_struct_gep(tuple_sty, tup_alloca, 1, "inf")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(int_field, result)
                    .map_err(llvm_err)?;
                Ok(TypedValue::Struct(tup_alloca, tuple_sty))
            }
            "flip" => {
                if args.len() != 3 {
                    return Err("flip expects 3 arguments (f, a, b)".to_string());
                }
                // flip(f, a, b) = f(b, a)
                let call = Expr::Call {
                    func: Box::new(args[0].clone()),
                    args: vec![args[2].clone(), args[1].clone()],
                    trailing_lambda: None,
                };
                self.compile_expr(&call)
            }
            "constant" => {
                if args.len() != 2 {
                    return Err("constant expects 2 arguments (a, b)".to_string());
                }
                // constant(a, b) = a (returns first argument, ignores second)
                self.compile_expr(&args[0])
            }
            "uncurry" => {
                if args.len() != 3 {
                    return Err("uncurry expects 3 arguments (f, a, b)".to_string());
                }
                // uncurry(f, a, b) = f(a)(b)
                let inner = Expr::Call {
                    func: Box::new(args[0].clone()),
                    args: vec![args[1].clone()],
                    trailing_lambda: None,
                };
                let outer = Expr::Call {
                    func: Box::new(inner),
                    args: vec![args[2].clone()],
                    trailing_lambda: None,
                };
                self.compile_expr(&outer)
            }
            "curry" => {
                if args.len() != 2 {
                    return Err("curry expects 2 arguments (f, a)".to_string());
                }
                // curry(f, a) → creates a lambda |b| f(a, b)
                // We implement this by compiling the partial application as a lambda expression
                let lambda = Expr::Lambda {
                    params: vec!["b".to_string()],
                    body: Box::new(Expr::Call {
                        func: Box::new(args[0].clone()),
                        args: vec![args[1].clone(), Expr::Ident("b".to_string())],
                        trailing_lambda: None,
                    }),
                    implicit_it: false,
                };
                self.compile_expr(&lambda)
            }
            // ---- LazyList operations ----
            "toList" => {
                if args.len() != 1 {
                    return Err("toList expects 1 argument (lazy_list or set)".to_string());
                }
                self.builtin_to_list(&args[0])
            }
            "toLazyList" => {
                if args.len() != 1 {
                    return Err("toLazyList expects 1 argument (list)".to_string());
                }
                self.builtin_to_lazy_list(&args[0])
            }
            "lazyTake" => {
                if args.len() != 2 {
                    return Err("lazyTake expects 2 arguments (n, lazy_list)".to_string());
                }
                self.builtin_lazy_take(&args[0], &args[1])
            }
            "lazyDrop" => {
                if args.len() != 2 {
                    return Err("lazyDrop expects 2 arguments (n, lazy_list)".to_string());
                }
                self.builtin_lazy_drop(&args[0], &args[1])
            }
            "lazyMap" => {
                if args.len() != 2 {
                    return Err("lazyMap expects 2 arguments (fn, lazy_list)".to_string());
                }
                self.builtin_lazy_map(&args[0], &args[1])
            }
            "lazyFilter" => {
                if args.len() != 2 {
                    return Err("lazyFilter expects 2 arguments (fn, lazy_list)".to_string());
                }
                self.builtin_lazy_filter(&args[0], &args[1])
            }
            "lazyTakeWhile" => {
                if args.len() != 2 {
                    return Err("lazyTakeWhile expects 2 arguments (fn, lazy_list)".to_string());
                }
                self.builtin_lazy_take_while(&args[0], &args[1])
            }
            "lazyHead" => {
                if args.len() != 1 {
                    return Err("lazyHead expects 1 argument (lazy_list)".to_string());
                }
                self.builtin_lazy_head(&args[0])
            }
            "lazyZip" => {
                if args.len() != 2 {
                    return Err("lazy.zip expects 2 arguments (lazy1, lazy2)".to_string());
                }
                self.builtin_lazy_zip(&args[0], &args[1])
            }
            "toCString" => {
                if args.len() != 1 {
                    return Err("toCString expects 1 argument".to_string());
                }
                self.builtin_to_cstring(&args[0])
            }
            "fromCString" => {
                if args.len() != 1 {
                    return Err("fromCString expects 1 argument".to_string());
                }
                self.builtin_from_cstring(&args[0])
            }
            "isNull" => {
                if args.len() != 1 {
                    return Err("isNull expects 1 argument".to_string());
                }
                self.builtin_is_null(&args[0])
            }
            "deref" => {
                if args.len() != 1 {
                    return Err("deref expects 1 argument".to_string());
                }
                self.builtin_deref(&args[0])
            }
            "ping" => {
                let result = self.call_rt("action_test_ping", &[])?;
                let val = result
                    .try_as_basic_value()
                    .basic()
                    .ok_or("ping call failed")?
                    .into_int_value();
                Ok(TypedValue::Int(val))
            }
            "httpRequest" => {
                if args.len() != 4 {
                    return Err(
                        "httpRequest expects 4 arguments (method, url, headers, body)".to_string(),
                    );
                }
                self.builtin_http_request(&args[0], &args[1], &args[2], &args[3])
            }
            _ => Err(format!("Unknown builtin: {}", name)),
        }
    }

    /// Emit real date/time by calling C time() and localtime_r().
    /// When `include_time` is true, returns DateTime {year, month, day, hour, minute, second};
    /// otherwise returns Date {year, month, day}.
    pub(super) fn emit_today_now(
        &mut self,
        include_time: bool,
    ) -> Result<TypedValue<'ctx>, String> {
        let i64 = self.i64_ty();
        let i32 = self.i32_ty();
        let ptr = self.ptr_ty();

        // Declare time(3) if not already declared: time_t time(time_t *tloc)
        let time_fn = self.module.get_function("time").unwrap_or_else(|| {
            self.module
                .add_function("time", i64.fn_type(&[ptr.into()], false), None)
        });

        // Declare localtime_r(3) if not already declared: struct tm *localtime_r(const time_t *timep, struct tm *result)
        let loc_fn = self.module.get_function("localtime_r").unwrap_or_else(|| {
            self.module.add_function(
                "localtime_r",
                ptr.fn_type(&[ptr.into(), ptr.into()], false),
                None,
            )
        });

        // struct tm = {i32, i32, i32, i32, i32, i32, i32, i32, i32}
        let tm_ty = self.context.struct_type(
            &[
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
                i32.into(),
            ],
            false,
        );

        // Call time(NULL) — pass null for tloc
        let null_ptr = ptr.const_zero();
        let now_ts = self
            .builder
            .build_call(time_fn, &[null_ptr.into()], "now_ts")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("time() call failed")?;

        // Allocate struct tm on stack, zero-init
        let tm_a = self
            .builder
            .build_alloca(tm_ty, "tm_buf")
            .map_err(llvm_err)?;
        let zero_i32 = i32.const_int(0, false);
        for i in 0..9u32 {
            let fp = self
                .builder
                .build_struct_gep(tm_ty, tm_a, i, "tm_f")
                .map_err(llvm_err)?;
            self.builder.build_store(fp, zero_i32).map_err(llvm_err)?;
        }

        // Allocate time_t for passing to localtime_r
        let ts_a = self.builder.build_alloca(i64, "ts_buf").map_err(llvm_err)?;
        self.builder.build_store(ts_a, now_ts).map_err(llvm_err)?;

        // Call localtime_r(&ts, &tm)
        let _ = self
            .builder
            .build_call(loc_fn, &[ts_a.into(), tm_a.into()], "")
            .map_err(llvm_err)?;

        // Load fields from struct tm
        // tm_year: years since 1900 → actual year = tm_year + 1900
        let tm_year_p = self
            .builder
            .build_struct_gep(tm_ty, tm_a, 5, "tm_year_p")
            .map_err(llvm_err)?;
        let tm_year = self
            .builder
            .build_load(i32, tm_year_p, "tm_year")
            .map_err(llvm_err)?
            .into_int_value();
        let year = self
            .builder
            .build_int_add(
                self.builder
                    .build_int_s_extend(tm_year, i64, "year_ext")
                    .map_err(llvm_err)?,
                i64.const_int(1900, false),
                "year",
            )
            .map_err(llvm_err)?;

        // tm_mon: 0-11 → month = tm_mon + 1
        let tm_mon_p = self
            .builder
            .build_struct_gep(tm_ty, tm_a, 4, "tm_mon_p")
            .map_err(llvm_err)?;
        let tm_mon = self
            .builder
            .build_load(i32, tm_mon_p, "tm_mon")
            .map_err(llvm_err)?
            .into_int_value();
        let month = self
            .builder
            .build_int_add(
                self.builder
                    .build_int_s_extend(tm_mon, i64, "mon_ext")
                    .map_err(llvm_err)?,
                i64.const_int(1, false),
                "month",
            )
            .map_err(llvm_err)?;

        // tm_mday: 1-31
        let tm_day_p = self
            .builder
            .build_struct_gep(tm_ty, tm_a, 3, "tm_day_p")
            .map_err(llvm_err)?;
        let tm_day = self
            .builder
            .build_load(i32, tm_day_p, "tm_day")
            .map_err(llvm_err)?
            .into_int_value();
        let day = self
            .builder
            .build_int_s_extend(tm_day, i64, "day_ext")
            .map_err(llvm_err)?;

        if include_time {
            let dt_struct = self.named_structs.get("DateTime").or_else(|| {
                self.anon_structs
                    .values()
                    .find(|s| s.get_field_types().len() == 6)
            });
            match dt_struct {
                Some(sty) => {
                    let sty = *sty;
                    let alloca = self.builder.build_alloca(sty, "now").map_err(llvm_err)?;
                    // Store year, month, day
                    for (i, val) in [(0u32, year), (1, month), (2, day)].iter() {
                        let fp = self
                            .builder
                            .build_struct_gep(sty, alloca, *i, "f")
                            .map_err(llvm_err)?;
                        self.builder.build_store(fp, *val).map_err(llvm_err)?;
                    }
                    // tm_hour: 0-23
                    let tm_h_p = self
                        .builder
                        .build_struct_gep(tm_ty, tm_a, 2, "tm_h_p")
                        .map_err(llvm_err)?;
                    let tm_h = self
                        .builder
                        .build_load(i32, tm_h_p, "tm_h")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let hour = self
                        .builder
                        .build_int_s_extend(tm_h, i64, "h_ext")
                        .map_err(llvm_err)?;
                    // tm_min: 0-59
                    let tm_m_p = self
                        .builder
                        .build_struct_gep(tm_ty, tm_a, 1, "tm_min_p")
                        .map_err(llvm_err)?;
                    let tm_m = self
                        .builder
                        .build_load(i32, tm_m_p, "tm_m")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let min = self
                        .builder
                        .build_int_s_extend(tm_m, i64, "m_ext")
                        .map_err(llvm_err)?;
                    // tm_sec: 0-60
                    let tm_s_p = self
                        .builder
                        .build_struct_gep(tm_ty, tm_a, 0, "tm_s_p")
                        .map_err(llvm_err)?;
                    let tm_s = self
                        .builder
                        .build_load(i32, tm_s_p, "tm_s")
                        .map_err(llvm_err)?
                        .into_int_value();
                    let sec = self
                        .builder
                        .build_int_s_extend(tm_s, i64, "s_ext")
                        .map_err(llvm_err)?;
                    for (i, val) in [(3u32, hour), (4, min), (5, sec)].iter() {
                        let fp = self
                            .builder
                            .build_struct_gep(sty, alloca, *i, "f")
                            .map_err(llvm_err)?;
                        self.builder.build_store(fp, *val).map_err(llvm_err)?;
                    }
                    Ok(TypedValue::Struct(alloca, sty))
                }
                None => Err("now: DateTime type not defined".to_string()),
            }
        } else {
            let date_struct = self.named_structs.get("Date").or_else(|| {
                self.anon_structs
                    .values()
                    .find(|s| s.get_field_types().len() == 3)
            });
            match date_struct {
                Some(sty) => {
                    let sty = *sty;
                    let alloca = self.builder.build_alloca(sty, "today").map_err(llvm_err)?;
                    for (i, val) in [(0u32, year), (1, month), (2, day)].iter() {
                        let fp = self
                            .builder
                            .build_struct_gep(sty, alloca, *i, "f")
                            .map_err(llvm_err)?;
                        self.builder.build_store(fp, *val).map_err(llvm_err)?;
                    }
                    Ok(TypedValue::Struct(alloca, sty))
                }
                None => Err("today: Date type not defined".to_string()),
            }
        }
    }

    /// toCString(str) -> CString: allocate a null-terminated copy of the string
    pub(super) fn builtin_to_cstring(&mut self, expr: &Expr) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_expr(expr)?;
        match val {
            TypedValue::Str(ptr) => {
                let str_val = self.load_string(ptr)?;
                let len = self
                    .builder
                    .build_extract_value(str_val, 0, "len")
                    .map_err(llvm_err)?
                    .into_int_value();
                let data = self
                    .builder
                    .build_extract_value(str_val, 1, "data")
                    .map_err(llvm_err)?
                    .into_pointer_value();
                // allocate len + 1 bytes for null-terminated copy
                let size = self
                    .builder
                    .build_int_add(len, self.i64_ty().const_int(1, false), "cstr_size")
                    .map_err(llvm_err)?;
                let cstr = self.call_rt("malloc", &[size.into()])?;
                let cstr_ptr = cstr
                    .try_as_basic_value()
                    .basic()
                    .ok_or("malloc failed")?
                    .into_pointer_value();
                // memcpy the string data (dest, src, len)
                let _ = self
                    .builder
                    .build_memcpy(cstr_ptr, 1, data, 1, len)
                    .map_err(llvm_err)?;
                // null terminate
                let null_pos = unsafe {
                    self.builder
                        .build_gep(self.context.i8_type(), cstr_ptr, &[len], "null_pos")
                }
                .map_err(llvm_err)?;
                self.builder
                    .build_store(null_pos, self.context.i8_type().const_int(0, false))
                    .map_err(llvm_err)?;
                Ok(TypedValue::CString(cstr_ptr))
            }
            _ => Err("toCString: argument must be a String".to_string()),
        }
    }

    /// fromCString(cstr) -> String: read a null-terminated C string
    pub(super) fn builtin_from_cstring(&mut self, expr: &Expr) -> Result<TypedValue<'ctx>, String> {
        let val = self.compile_expr(expr)?;
        match val {
            TypedValue::CString(ptr) | TypedValue::Ptr(ptr) | TypedValue::FileHandle(ptr) => {
                if matches!(val, TypedValue::FileHandle(_)) {
                    return Err("fromCString: cannot convert FileHandle to string".to_string());
                }
                // Null check: return empty string for null pointers
                let null_ptr = self
                    .context
                    .ptr_type(inkwell::AddressSpace::default())
                    .const_zero();
                let is_null = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, ptr, null_ptr, "is_null")
                    .map_err(llvm_err)?;
                let is_null_bb = self.context.append_basic_block(
                    self.builder
                        .get_insert_block()
                        .and_then(|b| b.get_parent())
                        .ok_or("no function")?,
                    "fcs_null",
                );
                let ok_bb = self.context.append_basic_block(
                    self.builder
                        .get_insert_block()
                        .and_then(|b| b.get_parent())
                        .ok_or("no function")?,
                    "fcs_ok",
                );
                let merge_bb = self.context.append_basic_block(
                    self.builder
                        .get_insert_block()
                        .and_then(|b| b.get_parent())
                        .ok_or("no function")?,
                    "fcs_merge",
                );
                let _ = self
                    .builder
                    .build_conditional_branch(is_null, is_null_bb, ok_bb);

                // Null path: return empty string ""
                self.builder.position_at_end(is_null_bb);
                let empty_str = self
                    .builder
                    .build_alloca(self.string_type, "empty")
                    .map_err(llvm_err)?;
                let empty_undef = self.string_type.get_undef();
                let e1 = self
                    .builder
                    .build_insert_value(empty_undef, self.i64_ty().const_int(0, false), 0, "e_len")
                    .map_err(llvm_err)?;
                // Allocate a zero byte for the empty string data
                let zero_byte = self
                    .builder
                    .build_alloca(self.context.i8_type(), "zero_byte")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(zero_byte, self.context.i8_type().const_int(0, false))
                    .map_err(llvm_err)?;
                let e2 = self
                    .builder
                    .build_insert_value(e1, zero_byte, 1, "e_ptr")
                    .map_err(llvm_err)?;
                self.builder.build_store(empty_str, e2).map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);

                // OK path: strlen + allocate
                self.builder.position_at_end(ok_bb);
                let len_val = self.call_rt("strlen", &[ptr.into()])?;
                let len = len_val
                    .try_as_basic_value()
                    .basic()
                    .ok_or("strlen failed")?
                    .into_int_value();
                let str_struct = self.call_rt("action_string_create", &[ptr.into(), len.into()])?;
                let str_val = str_struct
                    .try_as_basic_value()
                    .basic()
                    .ok_or("string_create failed")?;
                let ok_alloca = self
                    .builder
                    .build_alloca(self.string_type, "from_cstr")
                    .map_err(llvm_err)?;
                self.builder
                    .build_store(ok_alloca, str_val)
                    .map_err(llvm_err)?;
                let _ = self.builder.build_unconditional_branch(merge_bb);

                // Merge with phi
                self.builder.position_at_end(merge_bb);
                let phi = self
                    .builder
                    .build_phi(
                        self.context.ptr_type(inkwell::AddressSpace::default()),
                        "fcs_phi",
                    )
                    .map_err(llvm_err)?;
                phi.add_incoming(&[
                    (&empty_str.as_basic_value_enum(), is_null_bb),
                    (&ok_alloca.as_basic_value_enum(), ok_bb),
                ]);
                let result_alloca = phi.as_basic_value().into_pointer_value();
                Ok(TypedValue::Str(result_alloca))
            }
            _ => Err("fromCString: argument must be a CString or Ptr".to_string()),
        }
    }

    /// isNull(ptr) -> Bool: check if a Ptr or CString is null
    pub(super) fn builtin_is_null(&mut self, expr: &Expr) -> Result<TypedValue<'ctx>, String> {
        if !self.in_unsafe {
            return Err("isNull can only be used inside an unsafe block".to_string());
        }
        let val = self.compile_expr(expr)?;
        match val {
            TypedValue::Ptr(p) | TypedValue::CString(p) | TypedValue::FileHandle(p) => {
                let null_ptr = self
                    .context
                    .ptr_type(inkwell::AddressSpace::default())
                    .const_zero();
                let is_null = self
                    .builder
                    .build_int_compare(IntPredicate::EQ, p, null_ptr, "isNull")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Bool(is_null))
            }
            _ => Err("isNull: argument must be a Ptr, CString, or FileHandle".to_string()),
        }
    }

    /// deref(ptr) -> T: dereference a typed pointer (unsafe)
    pub(super) fn builtin_deref(&mut self, expr: &Expr) -> Result<TypedValue<'ctx>, String> {
        if !self.in_unsafe {
            return Err("deref can only be used inside an unsafe block".to_string());
        }
        let val = self.compile_expr(expr)?;
        match val {
            TypedValue::Ptr(p) => {
                // Load as i64 (most common FFI use case)
                let loaded = self
                    .builder
                    .build_load(self.i64_ty(), p, "deref")
                    .map_err(llvm_err)?;
                Ok(TypedValue::Int(loaded.into_int_value()))
            }
            _ => Err("deref: argument must be a Ptr".to_string()),
        }
    }

    /// httpRequest(method: String, url: String, headers: String, body: String) -> String
    /// Converts each String arg to CString, calls action_http_request, returns result as String.
    pub(super) fn builtin_http_request(
        &mut self,
        method: &Expr,
        url: &Expr,
        headers: &Expr,
        body: &Expr,
    ) -> Result<TypedValue<'ctx>, String> {
        // Delegate to existing toCString for each arg
        let method_cstr = self.builtin_to_cstring(method)?;
        let url_cstr = self.builtin_to_cstring(url)?;
        let headers_cstr = self.builtin_to_cstring(headers)?;
        let body_cstr = self.builtin_to_cstring(body)?;

        let method_ptr = match method_cstr {
            TypedValue::CString(p) => p,
            _ => return Err("httpRequest: method must be String".to_string()),
        };
        let url_ptr = match url_cstr {
            TypedValue::CString(p) => p,
            _ => return Err("httpRequest: url must be String".to_string()),
        };
        let headers_ptr = match headers_cstr {
            TypedValue::CString(p) => p,
            _ => return Err("httpRequest: headers must be String".to_string()),
        };
        let body_ptr = match body_cstr {
            TypedValue::CString(p) => p,
            _ => return Err("httpRequest: body must be String".to_string()),
        };

        // Use strlen to get body length (safe since we just null-terminated it)
        let body_len_val = self.call_rt("strlen", &[body_ptr.into()])?;
        let body_len = body_len_val
            .try_as_basic_value()
            .basic()
            .ok_or("strlen failed")?
            .into_int_value();

        // Call action_http_request(method, url, headers, body, body_len)
        let req_fn = self
            .module
            .get_function("action_http_request")
            .ok_or("action_http_request not found")?;
        let call_result = self
            .builder
            .build_call(
                req_fn,
                &[
                    method_ptr.into(),
                    url_ptr.into(),
                    headers_ptr.into(),
                    body_ptr.into(),
                    body_len.into(),
                ],
                "http_result",
            )
            .map_err(llvm_err)?;
        let result_ptr = call_result
            .try_as_basic_value()
            .basic()
            .ok_or("call failed")?
            .into_pointer_value();

        // Free temp CStrings
        let free_fn = self
            .module
            .get_function("free")
            .ok_or("free not found in module")?;
        for ptr in &[method_ptr, url_ptr, headers_ptr, body_ptr] {
            let _ = self.builder.build_call(free_fn, &[(*ptr).into()], "");
        }

        // Convert result CString -> String (fromCString logic inline)
        let res_len_val = self.call_rt("strlen", &[result_ptr.into()])?;
        let res_len = res_len_val
            .try_as_basic_value()
            .basic()
            .ok_or("strlen failed")?
            .into_int_value();
        let str_struct =
            self.call_rt("action_string_create", &[result_ptr.into(), res_len.into()])?;
        let str_val = str_struct
            .try_as_basic_value()
            .basic()
            .ok_or("string_create failed")?;
        let alloca = self
            .builder
            .build_alloca(self.string_type, "http_resp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(alloca, str_val)
            .map_err(llvm_err)?;

        // Free C result string via action_http_free
        let http_free_fn = self
            .module
            .get_function("action_http_free")
            .ok_or("action_http_free not found")?;
        let _ = self
            .builder
            .build_call(http_free_fn, &[result_ptr.into()], "");

        Ok(TypedValue::Str(alloca))
    }
}
