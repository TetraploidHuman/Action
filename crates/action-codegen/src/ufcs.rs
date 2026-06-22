// UFCS method dispatch (shared AST/HIR call-arg path).

use super::builtin_dispatch::BuiltinDispatch;
use super::call_arg::CallArg;
use action_frontend::builtin::UfcsReceiverKind;
use inkwell::values::BasicMetadataValueEnum;
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn compile_ufcs_method(
        &mut self,
        receiver: CallArg<'_>,
        method: &str,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let recv_val = self.compile_call_arg(receiver)?;

        // Auto short-circuit: nullable receiver — branch on null,
        // extract inner, and dispatch method on the non-null inner value.
        if let TypedValue::Nullable(nullable_ptr, inner_bt) = recv_val {
            return self.compile_nullable_method_call_call_args(
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
                return self.ufcs_forward_call("mapKeys", receiver, &[], None);
            }
            if method == "values" {
                self.rc_free_method_receiver(&recv_val)?;
                return self.ufcs_forward_call("mapValues", receiver, &[], None);
            }
            if method == "mapValues" {
                self.rc_free_method_receiver(&recv_val)?;
                return self.ufcs_forward_call("mapMapValues", receiver, &[], trailing);
            }
            if method == "entries" {
                self.rc_free_method_receiver(&recv_val)?;
                return self.ufcs_forward_call("mapEntries", receiver, &[], None);
            }
            if method == "union" {
                if args.len() != 1 {
                    return Err("map.union expects 1 argument (other map)".to_string());
                }
                self.rc_free_method_receiver(&recv_val)?;
                return self.ufcs_forward_call("mapUnion", receiver, args, None);
            }
            if method == "filter" {
                self.rc_free_method_receiver(&recv_val)?;
                return self.ufcs_forward_call("mapFilter", receiver, &[], trailing);
            }
            if method == "fold" {
                self.rc_free_method_receiver(&recv_val)?;
                let mut new_args = vec![receiver];
                new_args.extend(args.iter().copied());
                return self.dispatch_named_call("mapFold", &new_args, trailing);
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
                return self.ufcs_forward_call("setUnion", receiver, args, None);
            }
            if method == "intersection" {
                if args.len() != 1 {
                    return Err("set.intersection expects 1 argument (other set)".to_string());
                }
                self.rc_free_method_receiver(&recv_val)?;
                return self.ufcs_forward_call("setIntersection", receiver, args, None);
            }
            if method == "difference" {
                if args.len() != 1 {
                    return Err("set.difference expects 1 argument (other set)".to_string());
                }
                self.rc_free_method_receiver(&recv_val)?;
                return self.ufcs_forward_call("setDifference", receiver, args, None);
            }
            if method == "is_subset" {
                if args.len() != 1 {
                    return Err("set.isSubset expects 1 argument (other set)".to_string());
                }
                self.rc_free_method_receiver(&recv_val)?;
                return self.ufcs_forward_call("setIsSubset", receiver, args, None);
            }
            if method == "toList" {
                self.rc_free_method_receiver(&recv_val)?;
                return self.ufcs_forward_call("toList", receiver, &[], None);
            }
        }
        // Handle Range builtin methods inline (range is a Struct with 3 i64 fields)
        if let TypedValue::Struct(_, st) = &recv_val {
            if *st == self.range_type {
                self.rc_free_method_receiver(&recv_val)?;
                match method {
                    "contains" => {
                        if args.len() != 1 {
                            return Err("range.contains expects 1 argument".to_string());
                        }
                        return self.builtin_range_contains_call_args(receiver, args[0]);
                    }
                    "toList" => {
                        if !args.is_empty() {
                            return Err("range.toList expects no arguments".to_string());
                        }
                        return self.builtin_range_to_list_call_args(receiver);
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
            match method {
                "toList" => {
                    return self.ufcs_forward_call("toList", receiver, &[], None);
                }
                "toLazyList" => {
                    return self.ufcs_forward_call("toLazyList", receiver, &[], None);
                }
                "take" => {
                    if args.len() != 1 {
                        return Err("lazy.take expects 1 argument (n)".to_string());
                    }
                    return self.dispatch_named_call("lazyTake", &[args[0], receiver], None);
                }
                "drop" => {
                    if args.len() != 1 {
                        return Err("lazy.drop expects 1 argument (n)".to_string());
                    }
                    return self.dispatch_named_call("lazyDrop", &[args[0], receiver], None);
                }
                "map" => {
                    return self.ufcs_forward_call("lazyMap", receiver, &[], trailing);
                }
                "filter" => {
                    return self.ufcs_forward_call("lazyFilter", receiver, &[], trailing);
                }
                "takeWhile" => {
                    return self.ufcs_forward_call("lazyTakeWhile", receiver, &[], trailing);
                }
                "head" => {
                    return self.ufcs_forward_call("lazyHead", receiver, &[], None);
                }
                "zip" => {
                    if args.len() != 1 {
                        return Err("lazy.zip expects 1 argument (other)".to_string());
                    }
                    return self.ufcs_forward_call("lazyZip", receiver, args, None);
                }
                _ => return Err(format!("Method '{}' not found on LazyList", method)),
            }
        }
        // Handle String builtin methods inline
        if matches!(recv_val, TypedValue::Str(_)) {
            // All paths recompile via compile_call; free the first compilation's
            // intermediate data. Scope variables: no-op.
            self.rc_free_method_receiver(&recv_val)?;
            match method {
                // No-arg methods
                "len" | "isEmpty" | "toUpper" | "toLower" | "trim" | "trimStart" | "trimEnd"
                | "chars" | "splitLines" | "toInt" | "toFloat" => {
                    return self.ufcs_forward_call(method, receiver, &[], None);
                }
                // Single-arg methods (method(string, arg))
                "split" | "startsWith" | "endsWith" | "indexOf" | "replace" | "slice"
                | "repeat" | "contains" => {
                    if args.len() != 1 {
                        return Err(format!("string.{} expects 1 argument", method));
                    }
                    let mapped = match method {
                        "contains" => "stringContains",
                        "repeat" => "stringRepeat",
                        "slice" => "slice",
                        other => other,
                    };
                    return self.ufcs_forward_call(mapped, receiver, args, None);
                }
                // substring(string, start, len)
                "substring" => {
                    if args.len() != 2 {
                        return Err(
                            "string.substring expects 2 arguments (start, length)".to_string()
                        );
                    }
                    return self.ufcs_forward_call("substring", receiver, args, None);
                }
                "join" => {
                    // string.join(list) = join(string, list)
                    if args.len() != 1 {
                        return Err("string.join expects 1 argument (list)".to_string());
                    }
                    return self.ufcs_forward_call("join", receiver, args, None);
                }
                "toCString" => {
                    return self.ufcs_forward_call("toCString", receiver, &[], None);
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
            match method {
                "isNull" => {
                    return self.ufcs_forward_call("isNull", receiver, &[], None);
                }
                "deref" => {
                    return self.ufcs_forward_call("deref", receiver, &[], None);
                }
                _ => return Err(format!("Method '{}' not found on Ptr/CString", method)),
            }
        }
        // Handle Stream builtin methods inline
        if matches!(recv_val, TypedValue::Stream(_)) {
            match method {
                "send" => {
                    if args.len() != 1 {
                        return Err("stream.send expects 1 argument: value".to_string());
                    }
                    let stream_ptr = match recv_val {
                        TypedValue::Stream(p) => p,
                        _ => unreachable!(),
                    };
                    let value = self.compile_call_arg(args[0])?;
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
                    let wait_loop_bb = self.context.append_basic_block(cur_fn, "stream_wait_loop");
                    let got_data_bb = self.context.append_basic_block(cur_fn, "stream_got_data");
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
            match method {
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
                    return Ok(TypedValue::Int(tag));
                }
                _ => return Err(format!("Method '{}' not found on Task", method)),
            }
        }
        // Handle List builtin methods inline — UFCS: list.method(args) ≡ method(list, args...)
        if let TypedValue::List(lp) = &recv_val {
            if let Some(result) = self.compile_list_readonly_ufcs(*lp, &recv_val, method, args)? {
                return Ok(result);
            }
            match method {
                "insert" => return self.builtin_list_insert(*lp, args),
                "remove" => return self.builtin_list_remove(*lp, args),
                "append" => return self.builtin_list_append(*lp, args),
                _ => {}
            }
            // Read-only UFCS must not rc_free + AST recompile (method-chain SIGSEGV).
            if let Some(def) = action_frontend::builtin::lookup_ufcs(UfcsReceiverKind::List, method)
            {
                if def.readonly && !def.supports_trailing_lambda {
                    return Err(format!(
                        "internal: readonly list method '{}' missing UFCS fast path",
                        method
                    ));
                }
            }
            // Remaining methods: free intermediate then recompile via compile_call
            self.rc_free_method_receiver(&recv_val)?;
            match method {
                // No-arg methods: f(list) — read-only handled above
                "init" | "toList" | "toLazyList" | "flatten" | "unique" | "sorted" | "product" => {
                    return self.ufcs_forward_call(method, receiver, &[], None);
                }
                // Two-arg methods: f(list, arg1, arg2) — dispatch to builtin_stdlib
                "insert" => {
                    if args.len() != 2 {
                        return Err(format!("list.{} expects 2 arguments", method));
                    }
                    return self.ufcs_forward_call(method, receiver, args, None);
                }
                // Single-arg methods: f(list, arg) — dispatch to builtin_stdlib
                "take" | "drop" | "append" | "prepend" | "slice" | "splitAt" | "chunks"
                | "windows" | "repeat" | "withIndex" | "remove" | "zip" | "count" | "partition" => {
                    if args.len() != 1 {
                        return Err(format!("list.{} expects 1 argument", method));
                    }
                    return self.ufcs_forward_call(method, receiver, args, None);
                }
                // map, filter, fold, any, all, find, reduce, foldRight, takeWhile, dropWhile, flatMap, sortedBy
                // Named fn arg: list.op(pred). Trailing lambda: list.op { x => ... }.
                "map" | "filter" | "any" | "all" | "find" | "reduce" | "takeWhile"
                | "dropWhile" | "flatMap" | "foldRight" | "sortedBy" | "findIndex" => {
                    if trailing.is_some() {
                        return self.ufcs_forward_call(method, receiver, &[], trailing);
                    }
                    if args.len() == 1 {
                        return self.ufcs_forward_call(method, receiver, args, None);
                    }
                    return Err(format!("list.{} expects 1 function argument", method));
                }
                "fold" => {
                    if args.len() < 1 {
                        return Err("list.fold expects at least 1 argument (init)".to_string());
                    }
                    let mut new_args = vec![receiver];
                    new_args.extend(args.iter().copied());
                    return self.dispatch_named_call("fold", &new_args, trailing);
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
            let recv_bv = self.typed_value_to_bv(&recv_val);
            let casted_recv = self.coerce_arg(recv_bv, param_tys.first())?;
            ca.push(casted_recv.into());
            tracked_args.push(recv_val.clone());
            for (i, a) in args.iter().enumerate() {
                let av = self.compile_call_arg(*a)?;
                let bv = self.typed_value_to_bv(&av);
                let casted = self.coerce_arg(bv, param_tys.get(i + 1))?;
                ca.push(casted.into());
                tracked_args.push(av);
            }
            if let Some(lam) = trailing {
                let bv = self.compile_and_load_call_arg(lam)?;
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
            TypedValue::Map(_) | TypedValue::Set(_) | TypedValue::Stream(_) | TypedValue::Task(_)
        ) {
            return Err(format!(
                "Method '{}' not found on type '{}'",
                method, type_name
            ));
        }

        // UFCS fallback: receiver.method(args) → method(receiver, args)
        // Read-only collection len/isEmpty must use compiled recv_val — rc_free + AST
        // recompile double-evaluates method chains (e.g. lst.remove(0).len()) and can SIGSEGV.
        if let Some(def) = action_frontend::builtin::lookup(method) {
            if BuiltinDispatch::is_readonly_ufcs_on_collection(def) {
                if matches!(recv_val, TypedValue::Map(_) | TypedValue::Set(_)) {
                    let lp = match &recv_val {
                        TypedValue::Map(p) | TypedValue::Set(p) => *p,
                        _ => unreachable!(),
                    };
                    let lv = self.load_list(lp)?;
                    let len = self.map_len_val(lv)?;
                    if def.name == "isEmpty" {
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
        }
        if let TypedValue::List(lp) = &recv_val {
            if let Some(result) = self.compile_list_readonly_ufcs(*lp, &recv_val, method, args)? {
                return Ok(result);
            }
        }
        self.rc_free_method_receiver(&recv_val)?;
        let mut new_args = vec![receiver];
        new_args.extend(args.iter().copied());
        return self.dispatch_named_call(method, &new_args, trailing);
    }
}
