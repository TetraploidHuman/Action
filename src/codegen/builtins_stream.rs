// Submodule: builtins_stream

use crate::ast::*;
use inkwell::IntPredicate;

use super::{llvm_err, CodeGen, TypedValue};

impl<'ctx> CodeGen<'ctx> {
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

}
