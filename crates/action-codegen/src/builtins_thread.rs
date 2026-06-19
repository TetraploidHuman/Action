// Submodule: builtins_thread

use inkwell::IntPredicate;

use super::call_arg::CallArg;
use super::{llvm_err, CodeGen, Scope, TypedValue};

impl<'ctx> CodeGen<'ctx> {
    // ---- Coroutine builtins ----

    /// launch { body } — start a coroutine on a real pthread (default scheduler).
    /// launch(io) { body } — start with I/O scheduler.
    /// launch(cpu) { body } — start with CPU scheduler.
    /// Task struct: {pthread: i64, done: i64, cancelled: i64, result_list: {ptr, i64, i64}}
    pub(super) fn builtin_launch(
        &mut self,
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        // Parse optional scheduler argument
        let scheduler = if !args.is_empty() {
            Self::parse_launch_scheduler(args[0])?
        } else {
            0i64 // default scheduler
        };
        let body = trailing.ok_or("launch requires a trailing lambda body")?;
        let body_body = Self::extract_trailing_block_body(body)
            .map_err(|_| "launch expects a block body: launch { ... }".to_string())?;

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
        let result = self.compile_trailing_body(body_body)?;

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
        _args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        let body = trailing.ok_or("coroutineScope requires a trailing lambda body")?;
        let body_body = Self::extract_trailing_block_body(body).map_err(|_| {
            "coroutineScope expects a block body: coroutineScope { ... }".to_string()
        })?;

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
        self.compile_trailing_body(body_body)?;

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
    pub(super) fn builtin_delay(
        &mut self,
        args: &[CallArg<'_>],
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err("delay expects 1 argument (ms)".to_string());
        }
        let ms_val = self.compile_call_arg(args[0])?;
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
        // action_list_push handles rc_inc of the element data_ptr internally
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
        args: &[CallArg<'_>],
        trailing: Option<CallArg<'_>>,
    ) -> Result<TypedValue<'ctx>, String> {
        if args.len() != 1 {
            return Err(
                "withTimeout expects 2 arguments: timeout(ms) and a trailing lambda".to_string(),
            );
        }
        let timeout_ms_val = self.compile_call_arg(args[0])?;
        let timeout_ms = match &timeout_ms_val {
            TypedValue::Int(v) => *v,
            _ => return Err("withTimeout: first argument must be Int (milliseconds)".to_string()),
        };
        let body = trailing.ok_or("withTimeout requires a trailing lambda body")?;
        let body_body = Self::extract_trailing_block_body(body)
            .map_err(|_| "withTimeout expects a block body: withTimeout(ms) { ... }".to_string())?;

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

        let result = self.compile_trailing_body(body_body)?;

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
}
