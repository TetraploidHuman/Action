// Submodule: runtime

use inkwell::values::{BasicValue, IntValue, PointerValue};
use inkwell::{FloatPredicate, IntPredicate};

use super::{llvm_err, CodeGen};

impl<'ctx> CodeGen<'ctx> {
    #[allow(unused_variables)]
    pub(super) fn define_runtime(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let str_ty = self.string_type;
        let b1 = self.bool_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();

        // Declare external C functions
        let printf_fn = self
            .module
            .add_function("printf", i32.fn_type(&[ptr.into()], true), None);
        let malloc_fn = self
            .module
            .add_function("malloc", ptr.fn_type(&[i64.into()], false), None);
        let realloc_fn = self.module.add_function(
            "realloc",
            ptr.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let free_fn = self
            .module
            .add_function("free", void.fn_type(&[ptr.into()], false), None);
        // Declare RC functions early (defined at end of define_runtime)
        let malloc_rc_fn: inkwell::values::FunctionValue<'ctx> =
            self.module
                .add_function("action_malloc_rc", ptr.fn_type(&[i64.into()], false), None);
        let memcmp_fn = self.module.add_function(
            "memcmp",
            i32.fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
            None,
        );
        let utf8_encode_fn = self.module.add_function(
            "action_utf8_encode",
            i64.fn_type(&[i64.into(), ptr.into()], false),
            None,
        );
        let utf8_byte_len_fn = self.module.add_function(
            "action_utf8_byte_len",
            i64.fn_type(&[i8.into()], false),
            None,
        );
        let sprintf_fn = self.module.add_function(
            "sprintf",
            i32.fn_type(&[ptr.into(), ptr.into()], true),
            None,
        );
        let strlen_fn = self
            .module
            .add_function("strlen", i64.fn_type(&[ptr.into()], false), None);
        let memcpy_fn = self.module.add_function(
            "memcpy",
            ptr.fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
            None,
        );
        let _pow_fn =
            self.module
                .add_function("pow", f64.fn_type(&[f64.into(), f64.into()], false), None);
        let fopen_fn =
            self.module
                .add_function("fopen", ptr.fn_type(&[ptr.into(), ptr.into()], false), None);
        let fclose_fn = self
            .module
            .add_function("fclose", i32.fn_type(&[ptr.into()], false), None);
        let _fgets_fn = self.module.add_function(
            "fgets",
            ptr.fn_type(&[ptr.into(), i32.into(), ptr.into()], false),
            None,
        );
        let fread_fn = self.module.add_function(
            "fread",
            i64.fn_type(&[ptr.into(), i64.into(), i64.into(), ptr.into()], false),
            None,
        );
        let fwrite_fn = self.module.add_function(
            "fwrite",
            i64.fn_type(&[ptr.into(), i64.into(), i64.into(), ptr.into()], false),
            None,
        );
        let fseek_fn = self.module.add_function(
            "fseek",
            i32.fn_type(&[ptr.into(), i64.into(), i32.into()], false),
            None,
        );
        let ftell_fn = self
            .module
            .add_function("ftell", i64.fn_type(&[ptr.into()], false), None);
        let _remove_fn =
            self.module
                .add_function("remove", self.i32_ty().fn_type(&[ptr.into()], false), None);
        let _strtod_fn = self.module.add_function(
            "strtod",
            f64.fn_type(&[ptr.into(), ptr.into()], false),
            None,
        );
        let _strftime_fn = self.module.add_function(
            "strftime",
            i64.fn_type(&[ptr.into(), i64.into(), ptr.into(), ptr.into()], false),
            None,
        );
        let _strptime_fn = self.module.add_function(
            "strptime",
            ptr.fn_type(&[ptr.into(), ptr.into(), ptr.into()], false),
            None,
        );
        // C math functions
        let _sqrt_fn = self
            .module
            .add_function("sqrt", f64.fn_type(&[f64.into()], false), None);
        let _sin_fn = self
            .module
            .add_function("sin", f64.fn_type(&[f64.into()], false), None);
        let _cos_fn = self
            .module
            .add_function("cos", f64.fn_type(&[f64.into()], false), None);
        let _tan_fn = self
            .module
            .add_function("tan", f64.fn_type(&[f64.into()], false), None);
        let _asin_fn = self
            .module
            .add_function("asin", f64.fn_type(&[f64.into()], false), None);
        let _acos_fn = self
            .module
            .add_function("acos", f64.fn_type(&[f64.into()], false), None);
        let _atan_fn = self
            .module
            .add_function("atan", f64.fn_type(&[f64.into()], false), None);
        let _atan2_fn =
            self.module
                .add_function("atan2", f64.fn_type(&[f64.into(), f64.into()], false), None);
        let _log_fn = self
            .module
            .add_function("log", f64.fn_type(&[f64.into()], false), None);
        let _log2_fn = self
            .module
            .add_function("log2", f64.fn_type(&[f64.into()], false), None);
        let _log10_fn = self
            .module
            .add_function("log10", f64.fn_type(&[f64.into()], false), None);
        let _exp_fn = self
            .module
            .add_function("exp", f64.fn_type(&[f64.into()], false), None);
        let _floor_fn = self
            .module
            .add_function("floor", f64.fn_type(&[f64.into()], false), None);
        let _ceil_fn = self
            .module
            .add_function("ceil", f64.fn_type(&[f64.into()], false), None);
        let _round_fn = self
            .module
            .add_function("round", f64.fn_type(&[f64.into()], false), None);
        let _cbrt_fn = self
            .module
            .add_function("cbrt", f64.fn_type(&[f64.into()], false), None);

        // ---- action_* concurrency extern declarations ----
        // These are defined in src/runtime_threading.rs with #[no_mangle]
        // and registered to the JIT via add_global_mapping() in jit.rs.
        // On Linux they delegate to pthread; on Windows to kernel32.dll.

        let _action_mutex_init_fn = self.module.add_function(
            "action_mutex_init",
            i32.fn_type(&[ptr.into(), ptr.into()], false),
            None,
        );
        let _action_mutex_lock_fn =
            self.module
                .add_function("action_mutex_lock", i32.fn_type(&[ptr.into()], false), None);
        let _action_mutex_unlock_fn = self.module.add_function(
            "action_mutex_unlock",
            i32.fn_type(&[ptr.into()], false),
            None,
        );
        let _action_mutex_destroy_fn = self.module.add_function(
            "action_mutex_destroy",
            i32.fn_type(&[ptr.into()], false),
            None,
        );
        let _action_cond_init_fn = self.module.add_function(
            "action_cond_init",
            i32.fn_type(&[ptr.into(), ptr.into()], false),
            None,
        );
        let _action_cond_wait_fn = self.module.add_function(
            "action_cond_wait",
            i32.fn_type(&[ptr.into(), ptr.into()], false),
            None,
        );
        let _action_cond_signal_fn = self.module.add_function(
            "action_cond_signal",
            i32.fn_type(&[ptr.into()], false),
            None,
        );
        let _action_cond_broadcast_fn = self.module.add_function(
            "action_cond_broadcast",
            i32.fn_type(&[ptr.into()], false),
            None,
        );
        let _action_cond_destroy_fn = self.module.add_function(
            "action_cond_destroy",
            i32.fn_type(&[ptr.into()], false),
            None,
        );
        let _action_thread_create_fn = self.module.add_function(
            "action_thread_create",
            i32.fn_type(&[ptr.into(), ptr.into(), ptr.into(), ptr.into()], false),
            None,
        );
        let _action_thread_join_fn = self.module.add_function(
            "action_thread_join",
            i32.fn_type(&[i64.into(), ptr.into()], false),
            None,
        );
        let _action_thread_detach_fn = self.module.add_function(
            "action_thread_detach",
            i32.fn_type(&[i64.into()], false),
            None,
        );
        let _action_thread_cancel_fn = self.module.add_function(
            "action_thread_cancel",
            i32.fn_type(&[i64.into()], false),
            None,
        );
        let _action_sleep_us_fn =
            self.module
                .add_function("action_sleep_us", i32.fn_type(&[i32.into()], false), None);
        let _action_clock_gettime_fn = self.module.add_function(
            "action_clock_gettime",
            i32.fn_type(&[i32.into(), ptr.into()], false),
            None,
        );

        // memmove(dest, src, n) -> void* — for shifting list elements
        let _memmove_fn = self.module.add_function(
            "memmove",
            ptr.fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
            None,
        );

        // ---- HTTP / networking runtime functions ----
        // action_http_request(method: ptr, url: ptr, headers: ptr, body: ptr, body_len: i64) -> ptr
        let _http_request_fn = self.module.add_function(
            "action_http_request",
            ptr.fn_type(
                &[ptr.into(), ptr.into(), ptr.into(), ptr.into(), i64.into()],
                false,
            ),
            None,
        );
        // action_http_free(ptr)
        let _http_free_fn =
            self.module
                .add_function("action_http_free", void.fn_type(&[ptr.into()], false), None);
        // action_test_ping() -> i64
        let _ping_fn = self
            .module
            .add_function("action_test_ping", i64.fn_type(&[], false), None);

        // ---- JSON runtime functions ----
        // action_json_parse(json_str: ptr) -> ptr (returns null on error)
        let _json_parse_fn =
            self.module
                .add_function("action_json_parse", ptr.fn_type(&[ptr.into()], false), None);
        // action_json_stringify(node: ptr) -> ptr
        let _json_stringify_fn = self.module.add_function(
            "action_json_stringify",
            ptr.fn_type(&[ptr.into()], false),
            None,
        );
        // action_json_free(node: ptr)
        let _json_free_fn =
            self.module
                .add_function("action_json_free", void.fn_type(&[ptr.into()], false), None);
        // action_json_type(node: ptr) -> i64
        let _json_type_fn =
            self.module
                .add_function("action_json_type", i64.fn_type(&[ptr.into()], false), None);
        // action_json_get(node: ptr, key: ptr) -> ptr
        let _json_get_fn = self.module.add_function(
            "action_json_get",
            ptr.fn_type(&[ptr.into(), ptr.into()], false),
            None,
        );
        // action_json_get_idx(node: ptr, idx: i64) -> ptr
        let _json_get_idx_fn = self.module.add_function(
            "action_json_get_idx",
            ptr.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        // action_json_as_str(node: ptr) -> ptr
        let _json_as_str_fn = self.module.add_function(
            "action_json_as_str",
            ptr.fn_type(&[ptr.into()], false),
            None,
        );
        // action_json_as_float(node: ptr) -> f64
        let _json_as_float_fn = self.module.add_function(
            "action_json_as_float",
            f64.fn_type(&[ptr.into()], false),
            None,
        );
        // action_json_as_bool(node: ptr) -> i64
        let _json_as_bool_fn = self.module.add_function(
            "action_json_as_bool",
            i64.fn_type(&[ptr.into()], false),
            None,
        );
        // action_json_len(node: ptr) -> i64
        let _json_len_fn =
            self.module
                .add_function("action_json_len", i64.fn_type(&[ptr.into()], false), None);

        // Helper to create a global string constant
        let make_global_str = |name: &str, content: &[u8]| -> PointerValue<'ctx> {
            let arr_ty = i8.array_type(content.len() as u32);
            let global = self.module.add_global(arr_ty, None, name);
            let arr = self.context.const_string(content, false);
            global.set_initializer(&arr);
            global.as_pointer_value()
        };

        // Create format string globals (all null-terminated)
        let fmt_int_ptr = make_global_str(".fmt_int", b"%ld\0");
        let fmt_float_ptr = make_global_str(".fmt_float", b"%g\0");
        let fmt_str_ptr = make_global_str(".fmt_str", b"%s\0");
        let fmt_nl_ptr = make_global_str(".fmt_nl", b"\n\0");
        let str_true_ptr = make_global_str(".str_true", b"true\0");
        let str_false_ptr = make_global_str(".str_false", b"false\0");
        let fmt_lb_ptr = make_global_str(".fmt_lb", b"[\0");
        let fmt_sep_ptr = make_global_str(".fmt_sep", b", \0");
        let fmt_rb_ptr = make_global_str(".fmt_rb", b"]\0");
        let fmt_task_pre_ptr = make_global_str(".fmt_task_pre", b"Task(done=\0");
        let fmt_task_mid_ptr = make_global_str(".fmt_task_mid", b", cancelled=\0");
        let fmt_task_suf_ptr = make_global_str(".fmt_task_suf", b")\0");
        let fmt_struct_ptr = make_global_str(".fmt_struct", b"<struct>\0");
        let fmt_ev_pre = make_global_str(".fmt_ev_pre", b"EnumVariant<\0");
        let fmt_ev_gt = make_global_str(".fmt_ev_gt", b">\0");
        let fmt_ev_lp = make_global_str(".fmt_ev_lp", b">(\0");
        let fmt_ev_rp = make_global_str(".fmt_ev_rp", b")\0");

        // Save builder position (might be None since no function has been positioned yet)
        let saved_pos = self.builder.get_insert_block();

        // ---- action_print_int(i64) ----
        let print_int_fn =
            self.module
                .add_function("action_print_int", void.fn_type(&[i64.into()], false), None);
        let entry = self.context.append_basic_block(print_int_fn, "entry");
        self.builder.position_at_end(entry);
        let n = print_int_fn.get_first_param().unwrap();
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), n.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_float(double) ----
        let print_float_fn = self.module.add_function(
            "action_print_float",
            void.fn_type(&[f64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(print_float_fn, "entry");
        self.builder.position_at_end(entry);
        let n = print_float_fn.get_first_param().unwrap();
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_float_ptr.into(), n.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_bool(i1) ----
        let print_bool_fn =
            self.module
                .add_function("action_print_bool", void.fn_type(&[b1.into()], false), None);
        let entry = self.context.append_basic_block(print_bool_fn, "entry");
        let true_block = self
            .context
            .append_basic_block(print_bool_fn, "true_branch");
        let false_block = self
            .context
            .append_basic_block(print_bool_fn, "false_branch");
        self.builder.position_at_end(entry);
        let b = print_bool_fn.get_first_param().unwrap().into_int_value();
        let _ = self
            .builder
            .build_conditional_branch(b, true_block, false_block);
        self.builder.position_at_end(true_block);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_str_ptr.into(), str_true_ptr.into()], "");
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(false_block);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_str_ptr.into(), str_false_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_string({i64, ptr}) ----
        // Handles both: String (non-null data ptr) and Int (null data ptr, value in tag)
        let print_str_fn = self.module.add_function(
            "action_print_string",
            void.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(print_str_fn, "entry");
        self.builder.position_at_end(entry);
        let s = print_str_fn.get_first_param().unwrap().into_struct_value();
        let data = self
            .builder
            .build_extract_value(s, 1, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let is_null = self
            .builder
            .build_is_null(data, "is_null")
            .map_err(llvm_err)?;
        let str_bb = self.context.append_basic_block(print_str_fn, "print_str");
        let int_bb = self.context.append_basic_block(print_str_fn, "print_int");
        let _ = self
            .builder
            .build_conditional_branch(is_null, int_bb, str_bb);
        self.builder.position_at_end(str_bb);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_str_ptr.into(), data.into()], "");
        let _ = self.builder.build_return(None);
        self.builder.position_at_end(int_bb);
        let tag = self
            .builder
            .build_extract_value(s, 0, "tag")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), tag.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_println() ----
        let println_fn = self
            .module
            .add_function("action_println", void.fn_type(&[], false), None);
        let entry = self.context.append_basic_block(println_fn, "entry");
        self.builder.position_at_end(entry);
        let _ = self.builder.build_call(printf_fn, &[fmt_nl_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_list_print({ptr, i64, i64}) ----
        let list_print_fn = self.module.add_function(
            "action_list_print",
            void.fn_type(&[self.list_type.into()], false),
            None,
        );
        let lp_entry = self.context.append_basic_block(list_print_fn, "entry");
        self.builder.position_at_end(lp_entry);
        let lp_list = list_print_fn.get_first_param().unwrap().into_struct_value();
        let lp_data = self
            .builder
            .build_extract_value(lp_list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lp_len = self
            .builder
            .build_extract_value(lp_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        // Print "["
        let _ = self.builder.build_call(printf_fn, &[fmt_lb_ptr.into()], "");
        let lp_i = self.builder.build_alloca(i64, "lpi").map_err(llvm_err)?;
        self.builder
            .build_store(lp_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let lp_hdr = self.context.append_basic_block(list_print_fn, "lphdr");
        let lp_bdy = self.context.append_basic_block(list_print_fn, "lpbdy");
        let lp_ext = self.context.append_basic_block(list_print_fn, "lpext");
        let _ = self.builder.build_unconditional_branch(lp_hdr);
        self.builder.position_at_end(lp_hdr);
        let lp_iv = self
            .builder
            .build_load(i64, lp_i, "lpiv")
            .map_err(llvm_err)?
            .into_int_value();
        let lp_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lp_iv, lp_len, "lpcond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lp_cond, lp_bdy, lp_ext);
        self.builder.position_at_end(lp_bdy);
        // Print ", " if not first
        let lp_is_first = self
            .builder
            .build_int_compare(IntPredicate::EQ, lp_iv, i64.const_int(0, false), "is_first")
            .map_err(llvm_err)?;
        let lp_sep_bb = self.context.append_basic_block(list_print_fn, "lpsep");
        let lp_val_bb = self.context.append_basic_block(list_print_fn, "lpval");
        let _ = self
            .builder
            .build_conditional_branch(lp_is_first, lp_val_bb, lp_sep_bb);
        self.builder.position_at_end(lp_sep_bb);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_sep_ptr.into()], "");
        let _ = self.builder.build_unconditional_branch(lp_val_bb);
        self.builder.position_at_end(lp_val_bb);
        // Load element fat struct {tag, ptr}
        let lp_elem_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, lp_data, &[lp_iv], "lpep")
                .map_err(llvm_err)
        }?;
        let lp_elem = self
            .builder
            .build_load(self.string_type, lp_elem_ptr, "lpe")
            .map_err(llvm_err)?
            .into_struct_value();
        let lp_tag = self
            .builder
            .build_extract_value(lp_elem, 0, "lptag")
            .map_err(llvm_err)?
            .into_int_value();
        // Print integer tag for now (simplified: shows the value for int/bool/char tagged elements)
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), lp_tag.into()], "");
        // Next
        let lp_next = self
            .builder
            .build_int_add(lp_iv, i64.const_int(1, false), "lpnext")
            .map_err(llvm_err)?;
        self.builder.build_store(lp_i, lp_next).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_hdr);
        self.builder.position_at_end(lp_ext);
        let _ = self.builder.build_call(printf_fn, &[fmt_rb_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_task({pthread: i64, done: i64, cancelled: i64, result_list: list_type}) ----
        let task_print_fn = self.module.add_function(
            "action_print_task",
            void.fn_type(&[self.task_type.into()], false),
            None,
        );
        let tp_entry = self.context.append_basic_block(task_print_fn, "entry");
        self.builder.position_at_end(tp_entry);
        let tp_task = task_print_fn.get_first_param().unwrap().into_struct_value();
        let tp_done = self
            .builder
            .build_extract_value(tp_task, 1, "done")
            .map_err(llvm_err)?;
        let tp_canc = self
            .builder
            .build_extract_value(tp_task, 2, "canc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_task_pre_ptr.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), tp_done.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_task_mid_ptr.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), tp_canc.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_task_suf_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_struct() ----
        let struct_print_fn =
            self.module
                .add_function("action_print_struct", void.fn_type(&[], false), None);
        let sp_entry = self.context.append_basic_block(struct_print_fn, "entry");
        self.builder.position_at_end(sp_entry);
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_struct_ptr.into()], "");
        let _ = self.builder.build_return(None);

        // ---- action_print_enum({i64, ptr}) ----
        // Prints EnumVariant<tag> for nullary variants or EnumVariant<tag>(val) for data-carrying ones.
        let enum_ty = self.context.struct_type(&[i64.into(), ptr.into()], false);
        let enum_print_fn = self.module.add_function(
            "action_print_enum",
            void.fn_type(&[enum_ty.into()], false),
            None,
        );
        let ep_entry = self.context.append_basic_block(enum_print_fn, "entry");
        self.builder.position_at_end(ep_entry);
        let ep_enum = enum_print_fn.get_first_param().unwrap().into_struct_value();
        let ep_tag = self
            .builder
            .build_extract_value(ep_enum, 0, "tag")
            .map_err(llvm_err)?;
        let ep_data = self
            .builder
            .build_extract_value(ep_enum, 1, "data")
            .map_err(llvm_err)?;
        let ep_data_ptr = self
            .builder
            .build_pointer_cast(ep_data.into_pointer_value(), ptr, "vp")
            .map_err(llvm_err)?;
        let is_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, ep_data_ptr, ptr.const_zero(), "is_null")
            .map_err(llvm_err)?;
        let ep_data_bb = self.context.append_basic_block(enum_print_fn, "has_data");
        let ep_no_data_bb = self.context.append_basic_block(enum_print_fn, "no_data");
        let ep_merge_bb = self.context.append_basic_block(enum_print_fn, "merge");
        let _ = self
            .builder
            .build_conditional_branch(is_null, ep_no_data_bb, ep_data_bb);
        // Has data: print EnumVariant<tag>(val)
        self.builder.position_at_end(ep_data_bb);
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_pre.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), ep_tag.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_lp.into()], "");
        let ep_val = self
            .builder
            .build_load(i64, ep_data_ptr, "val")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), ep_val.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_rp.into()], "");
        let _ = self.builder.build_unconditional_branch(ep_merge_bb);
        // No data: print EnumVariant<tag>
        self.builder.position_at_end(ep_no_data_bb);
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_pre.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), ep_tag.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_gt.into()], "");
        let _ = self.builder.build_unconditional_branch(ep_merge_bb);
        self.builder.position_at_end(ep_merge_bb);
        let _ = self.builder.build_return(None);

        // ---- action_print_enum_float({i64, ptr}) ----
        // Same as action_print_enum but loads f64 from the heap instead of i64
        let epf_fn = self.module.add_function(
            "action_print_enum_float",
            void.fn_type(&[enum_ty.into()], false),
            None,
        );
        let epf_entry = self.context.append_basic_block(epf_fn, "entry");
        self.builder.position_at_end(epf_entry);
        let epf_enum = epf_fn.get_first_param().unwrap().into_struct_value();
        let epf_tag = self
            .builder
            .build_extract_value(epf_enum, 0, "tag")
            .map_err(llvm_err)?;
        let epf_data = self
            .builder
            .build_extract_value(epf_enum, 1, "data")
            .map_err(llvm_err)?;
        let epf_data_ptr = self
            .builder
            .build_pointer_cast(epf_data.into_pointer_value(), ptr, "vpf")
            .map_err(llvm_err)?;
        let epf_is_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                epf_data_ptr,
                ptr.const_zero(),
                "is_null_f",
            )
            .map_err(llvm_err)?;
        let epf_data_bb = self.context.append_basic_block(epf_fn, "has_data");
        let epf_no_data_bb = self.context.append_basic_block(epf_fn, "no_data");
        let epf_merge_bb = self.context.append_basic_block(epf_fn, "merge");
        let _ = self
            .builder
            .build_conditional_branch(epf_is_null, epf_no_data_bb, epf_data_bb);
        // Has data: print EnumVariant<tag>(val) with float
        self.builder.position_at_end(epf_data_bb);
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_pre.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), epf_tag.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_lp.into()], "");
        let epf_val = self
            .builder
            .build_load(f64, epf_data_ptr, "valf")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_float_ptr.into(), epf_val.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_rp.into()], "");
        let _ = self.builder.build_unconditional_branch(epf_merge_bb);
        // No data: print EnumVariant<tag>
        self.builder.position_at_end(epf_no_data_bb);
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_pre.into()], "");
        let _ = self
            .builder
            .build_call(printf_fn, &[fmt_int_ptr.into(), epf_tag.into()], "");
        let _ = self.builder.build_call(printf_fn, &[fmt_ev_gt.into()], "");
        let _ = self.builder.build_unconditional_branch(epf_merge_bb);
        self.builder.position_at_end(epf_merge_bb);
        let _ = self.builder.build_return(None);

        // ---- action_string_create(ptr, i64) -> {i64, ptr} ----
        let str_create_fn = self.module.add_function(
            "action_string_create",
            str_ty.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(str_create_fn, "entry");
        self.builder.position_at_end(entry);
        let data = str_create_fn
            .get_first_param()
            .unwrap()
            .into_pointer_value();
        let len = str_create_fn.get_nth_param(1).unwrap().into_int_value();
        // Allocate len+1 bytes with RC header
        let one = i64.const_int(1, false);
        let alloc_size = self
            .builder
            .build_int_add(len, one, "alloc_size")
            .map_err(llvm_err)?;
        let buf = self
            .builder
            .build_call(malloc_rc_fn, &[alloc_size.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_memcpy(buf, 1, data, 1, len)
            .map_err(llvm_err)?;
        // Null-terminate at buf[len]
        let null_pos = unsafe {
            self.builder
                .build_gep(i8, buf, &[len], "null_pos")
                .map_err(llvm_err)
        }?;
        let zero_byte = i8.const_int(0, false);
        let _ = self
            .builder
            .build_store(null_pos, zero_byte)
            .map_err(llvm_err)?;
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, len, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        // ---- action_string_concat({i64, ptr}, {i64, ptr}) -> {i64, ptr} ----
        let str_concat_fn = self.module.add_function(
            "action_string_concat",
            str_ty.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(str_concat_fn, "entry");
        self.builder.position_at_end(entry);
        let s1 = str_concat_fn.get_first_param().unwrap().into_struct_value();
        let s2 = str_concat_fn.get_nth_param(1).unwrap().into_struct_value();
        let len1 = self
            .builder
            .build_extract_value(s1, 0, "len1")
            .map_err(llvm_err)?
            .into_int_value();
        let data1 = self
            .builder
            .build_extract_value(s1, 1, "data1")
            .map_err(llvm_err)?
            .into_pointer_value();
        let len2 = self
            .builder
            .build_extract_value(s2, 0, "len2")
            .map_err(llvm_err)?
            .into_int_value();
        let data2 = self
            .builder
            .build_extract_value(s2, 1, "data2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let total = self
            .builder
            .build_int_add(len1, len2, "total")
            .map_err(llvm_err)?;
        let alloc_size = self
            .builder
            .build_int_add(total, i64.const_int(1, false), "alloc_size")
            .map_err(llvm_err)?;
        let buf = self
            .builder
            .build_call(malloc_rc_fn, &[alloc_size.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_memcpy(buf, 1, data1, 1, len1)
            .map_err(llvm_err)?;
        let offset = unsafe {
            self.builder
                .build_gep(i8, buf, &[len1], "offset")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_memcpy(offset, 1, data2, 1, len2)
            .map_err(llvm_err)?;
        // Null terminate
        let null_pos = unsafe {
            self.builder
                .build_gep(i8, buf, &[total], "null_pos")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(null_pos, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, total, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        // ---- action_string_eq({i64, ptr}, {i64, ptr}) -> i1 ----
        let str_eq_fn = self.module.add_function(
            "action_string_eq",
            b1.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry_bb = self.context.append_basic_block(str_eq_fn, "entry");
        let compare_bb = self.context.append_basic_block(str_eq_fn, "compare");
        let check_ptr_bb = self.context.append_basic_block(str_eq_fn, "check_ptr");
        let do_memcmp_bb = self.context.append_basic_block(str_eq_fn, "do_memcmp");
        let true_bb = self.context.append_basic_block(str_eq_fn, "true");
        let false_bb = self.context.append_basic_block(str_eq_fn, "false");
        let end_bb = self.context.append_basic_block(str_eq_fn, "end");
        let s1 = str_eq_fn.get_first_param().unwrap().into_struct_value();
        let s2 = str_eq_fn.get_nth_param(1).unwrap().into_struct_value();

        self.builder.position_at_end(entry_bb);
        let len1 = self
            .builder
            .build_extract_value(s1, 0, "len1")
            .map_err(llvm_err)?
            .into_int_value();
        let len2 = self
            .builder
            .build_extract_value(s2, 0, "len2")
            .map_err(llvm_err)?
            .into_int_value();
        let len_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, len1, len2, "len_eq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(len_eq, compare_bb, false_bb);

        self.builder.position_at_end(compare_bb);
        let zero_len = self.i64_ty().const_int(0, false);
        let is_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, len1, zero_len, "is_empty")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_empty, true_bb, check_ptr_bb);

        // Check for null pointers: if either is null, it's a scalar comparison — tags already match, so equal
        self.builder.position_at_end(check_ptr_bb);
        let data1 = self
            .builder
            .build_extract_value(s1, 1, "data1")
            .map_err(llvm_err)?
            .into_pointer_value();
        let data2 = self
            .builder
            .build_extract_value(s2, 1, "data2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let null_ptr = self.ptr_ty().const_zero();
        let d1_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, data1, null_ptr, "d1_null")
            .map_err(llvm_err)?;
        let d2_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, data2, null_ptr, "d2_null")
            .map_err(llvm_err)?;
        let any_null = self
            .builder
            .build_or(d1_null, d2_null, "any_null")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(any_null, true_bb, do_memcmp_bb);

        self.builder.position_at_end(do_memcmp_bb);
        let memcmp_call = self
            .builder
            .build_call(memcmp_fn, &[data1.into(), data2.into(), len1.into()], "cmp")
            .map_err(llvm_err)?;
        let cmp_result = memcmp_call
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let zero_i32 = i32.const_int(0, false);
        let content_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, cmp_result, zero_i32, "content_eq")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(end_bb);

        self.builder.position_at_end(true_bb);
        let _ = self.builder.build_unconditional_branch(end_bb);

        self.builder.position_at_end(false_bb);
        let _ = self.builder.build_unconditional_branch(end_bb);

        self.builder.position_at_end(end_bb);
        let phi = self.builder.build_phi(b1, "eq_result").map_err(llvm_err)?;
        phi.add_incoming(&[
            (&b1.const_int(1, false), true_bb),
            (&b1.const_int(0, false), false_bb),
            (&content_eq, do_memcmp_bb),
        ]);
        let _ = self.builder.build_return(Some(&phi.as_basic_value()));

        // ---- action_string_len({i64, ptr}) -> i64 ----
        let str_len_fn = self.module.add_function(
            "action_string_len",
            i64.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(str_len_fn, "entry");
        self.builder.position_at_end(entry);
        let sl_s = str_len_fn.get_first_param().unwrap().into_struct_value();
        let sl_len = self
            .builder
            .build_extract_value(sl_s, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_return(Some(&sl_len));

        // ---- action_int_to_string(i64) -> {i64, ptr} ----
        let int_to_str_fn = self.module.add_function(
            "action_int_to_string",
            str_ty.fn_type(&[i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(int_to_str_fn, "entry");
        self.builder.position_at_end(entry);
        let n = int_to_str_fn.get_first_param().unwrap().into_int_value();
        // Allocate 32-byte buffer with RC header
        let buf32 = self.i64_ty().const_int(32, false);
        let buf = self
            .builder
            .build_call(malloc_rc_fn, &[buf32.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // sprintf(buf, "%ld", n)
        let fmt_int = make_global_str(".fmt_int_str", b"%ld\0");
        let _ = self
            .builder
            .build_call(sprintf_fn, &[buf.into(), fmt_int.into(), n.into()], "")
            .map_err(llvm_err)?;
        // len = strlen(buf)
        let len = self
            .builder
            .build_call(strlen_fn, &[buf.into()], "len")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        // Return {len, buf}
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, len, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        // ---- action_float_to_string(f64) -> {i64, ptr} ----
        let float_to_str_fn = self.module.add_function(
            "action_float_to_string",
            str_ty.fn_type(&[f64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(float_to_str_fn, "entry");
        self.builder.position_at_end(entry);
        let n = float_to_str_fn
            .get_first_param()
            .unwrap()
            .into_float_value();
        let buf32 = self.i64_ty().const_int(32, false);
        let buf = self
            .builder
            .build_call(malloc_rc_fn, &[buf32.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fmt_float = make_global_str(".fmt_float_str", b"%g\0");
        let _ = self
            .builder
            .build_call(sprintf_fn, &[buf.into(), fmt_float.into(), n.into()], "")
            .map_err(llvm_err)?;
        let len = self
            .builder
            .build_call(strlen_fn, &[buf.into()], "len")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, len, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        // ---- action_int_pow(i64, i64) -> i64 (exponentiation by squaring) ----
        let int_pow_fn = self.module.add_function(
            "action_int_pow",
            i64.fn_type(&[i64.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(int_pow_fn, "entry");
        let loop_bb = self.context.append_basic_block(int_pow_fn, "loop");
        let odd_bb = self.context.append_basic_block(int_pow_fn, "odd");
        let after_mul_bb = self.context.append_basic_block(int_pow_fn, "after_mul");
        let done_bb = self.context.append_basic_block(int_pow_fn, "done");

        let base = int_pow_fn.get_first_param().unwrap().into_int_value();
        let exp = int_pow_fn.get_nth_param(1).unwrap().into_int_value();

        self.builder.position_at_end(entry);
        let result_alloca = self.builder.build_alloca(i64, "result").map_err(llvm_err)?;
        let b_alloca = self.builder.build_alloca(i64, "b").map_err(llvm_err)?;
        let e_alloca = self.builder.build_alloca(i64, "e").map_err(llvm_err)?;
        let one = i64.const_int(1, false);
        let zero = i64.const_int(0, false);
        self.builder
            .build_store(result_alloca, one)
            .map_err(llvm_err)?;
        self.builder.build_store(b_alloca, base).map_err(llvm_err)?;
        self.builder.build_store(e_alloca, exp).map_err(llvm_err)?;
        let exp_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, exp, zero, "neg")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(exp_neg, done_bb, loop_bb);

        // loop: while e > 0
        self.builder.position_at_end(loop_bb);
        let e_cur = self
            .builder
            .build_load(i64, e_alloca, "e_cur")
            .map_err(llvm_err)?
            .into_int_value();
        let e_gt_zero = self
            .builder
            .build_int_compare(IntPredicate::SGT, e_cur, zero, "gt")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(e_gt_zero, odd_bb, done_bb);

        // odd: if e & 1 then result *= b
        self.builder.position_at_end(odd_bb);
        let e_val = self
            .builder
            .build_load(i64, e_alloca, "e_val")
            .map_err(llvm_err)?
            .into_int_value();
        let is_odd = self
            .builder
            .build_and(e_val, one, "odd")
            .map_err(llvm_err)?;
        let odd_cond = self
            .builder
            .build_int_compare(IntPredicate::EQ, is_odd, one, "odd_cmp")
            .map_err(llvm_err)?;
        let mul_bb = self.context.append_basic_block(int_pow_fn, "mul");
        let _ = self
            .builder
            .build_conditional_branch(odd_cond, mul_bb, after_mul_bb);

        // mul: result *= b
        self.builder.position_at_end(mul_bb);
        let cur_result = self
            .builder
            .build_load(i64, result_alloca, "cur_r")
            .map_err(llvm_err)?
            .into_int_value();
        let cur_b = self
            .builder
            .build_load(i64, b_alloca, "cur_b")
            .map_err(llvm_err)?
            .into_int_value();
        let new_result = self
            .builder
            .build_int_mul(cur_result, cur_b, "mul_r")
            .map_err(llvm_err)?;
        self.builder
            .build_store(result_alloca, new_result)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(after_mul_bb);

        // after_mul: b *= b; e >>= 1
        self.builder.position_at_end(after_mul_bb);
        let b_val = self
            .builder
            .build_load(i64, b_alloca, "b_val")
            .map_err(llvm_err)?
            .into_int_value();
        let b_sq = self
            .builder
            .build_int_mul(b_val, b_val, "sq")
            .map_err(llvm_err)?;
        self.builder.build_store(b_alloca, b_sq).map_err(llvm_err)?;
        let e_val2 = self
            .builder
            .build_load(i64, e_alloca, "e_val2")
            .map_err(llvm_err)?
            .into_int_value();
        let two = i64.const_int(2, false);
        let e_half = self
            .builder
            .build_int_signed_div(e_val2, two, "half")
            .map_err(llvm_err)?;
        self.builder
            .build_store(e_alloca, e_half)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_bb);

        // done: return result
        self.builder.position_at_end(done_bb);
        let done_val = self
            .builder
            .build_load(i64, result_alloca, "done_val")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_return(Some(&done_val));
        let list_ty = self.list_type;
        let list_create_fn = self.module.add_function(
            "action_list_create",
            list_ty.fn_type(&[i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(list_create_fn, "entry");
        self.builder.position_at_end(entry);
        let cap = list_create_fn.get_first_param().unwrap().into_int_value();
        // capacity * 16 bytes per element ({i64, ptr} fat struct)
        let elem_size = i64.const_int(16, false);
        let data_size = self
            .builder
            .build_int_mul(cap, elem_size, "data_size")
            .map_err(llvm_err)?;
        let data = self
            .builder
            .build_call(malloc_rc_fn, &[data_size.into()], "data")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let zero = i64.const_int(0, false);
        let undef = list_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, data, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, zero, 1, "r2")
            .map_err(llvm_err)?;
        let r3 = self
            .builder
            .build_insert_value(r2, cap, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r3));

        // ---- action_list_push({ptr, i64, i64}, {i64, ptr}) -> {ptr, i64, i64} ----
        let list_push_fn = self.module.add_function(
            "action_list_push",
            list_ty.fn_type(&[list_ty.into(), self.string_type.into()], false),
            None,
        );
        let lp_entry = self.context.append_basic_block(list_push_fn, "entry");
        let lp_cow = self.context.append_basic_block(list_push_fn, "cow_clone");
        let lp_check_grow = self.context.append_basic_block(list_push_fn, "check_grow");
        let lp_grow = self.context.append_basic_block(list_push_fn, "grow");
        let lp_done = self.context.append_basic_block(list_push_fn, "done");
        self.builder.position_at_end(lp_entry);
        let list = list_push_fn.get_first_param().unwrap().into_struct_value();
        let elem = list_push_fn.get_nth_param(1).unwrap().into_struct_value();
        let data_ptr = self
            .builder
            .build_extract_value(list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let len = self
            .builder
            .build_extract_value(list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let cap = self
            .builder
            .build_extract_value(list, 2, "cap")
            .map_err(llvm_err)?
            .into_int_value();
        // CoW check: if refcount > 1, clone the data buffer
        let data_int = self
            .builder
            .build_ptr_to_int(data_ptr, i64, "data_int")
            .map_err(llvm_err)?;
        let rc_addr = self
            .builder
            .build_int_sub(data_int, i64.const_int(8, false), "rc_addr")
            .map_err(llvm_err)?;
        let rc_ptr = self
            .builder
            .build_int_to_ptr(rc_addr, ptr, "rc_ptr")
            .map_err(llvm_err)?;
        let rc_val = self
            .builder
            .build_load(self.i64_ty(), rc_ptr, "rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let need_cow = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                rc_val,
                i64.const_int(1, false),
                "need_cow",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(need_cow, lp_cow, lp_check_grow);

        // CoW clone block: allocate new buffer, copy data, dec old refcount
        self.builder.position_at_end(lp_cow);
        let elem_size_16 = i64.const_int(16, false);
        let cow_data_size = self
            .builder
            .build_int_mul(cap, elem_size_16, "cow_data_size")
            .map_err(llvm_err)?;
        let new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[cow_data_size.into()], "cow_new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let cow_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                cow_memcpy_fn,
                &[new_buf.into(), data_ptr.into(), cow_data_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_check_grow);

        // Check grow block: phi for data, then grow check
        self.builder.position_at_end(lp_check_grow);
        let phi_data = self.builder.build_phi(ptr, "phi_data").map_err(llvm_err)?;
        phi_data.add_incoming(&[(&data_ptr, lp_entry), (&new_buf, lp_cow)]);
        let check_data = phi_data.as_basic_value().into_pointer_value();
        let need_grow = self
            .builder
            .build_int_compare(IntPredicate::SGE, len, cap, "need_grow")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(need_grow, lp_grow, lp_done);

        // Grow block: double capacity (min 4), realloc with RC header, then branch to done
        self.builder.position_at_end(lp_grow);
        let min_cap = i64.const_int(4, false);
        let cap_small = self
            .builder
            .build_int_compare(IntPredicate::SLT, cap, min_cap, "cap_small")
            .map_err(llvm_err)?;
        let cap2x = self
            .builder
            .build_int_mul(cap, i64.const_int(2, false), "cap2x")
            .map_err(llvm_err)?;
        let new_cap = self
            .builder
            .build_select(cap_small, min_cap, cap2x, "new_cap")
            .map_err(llvm_err)?
            .into_int_value();
        let grow_data_size = self
            .builder
            .build_int_mul(new_cap, i64.const_int(16, false), "data_size")
            .map_err(llvm_err)?;
        let total_size = self
            .builder
            .build_int_add(grow_data_size, i64.const_int(8, false), "total_size")
            .map_err(llvm_err)?;
        let grow_data_int = self
            .builder
            .build_ptr_to_int(check_data, i64, "data_int")
            .map_err(llvm_err)?;
        let rc_offset = i64.const_int(8, false);
        let orig_int = self
            .builder
            .build_int_sub(grow_data_int, rc_offset, "orig_int")
            .map_err(llvm_err)?;
        let orig_ptr = self
            .builder
            .build_int_to_ptr(orig_int, ptr, "orig_ptr")
            .map_err(llvm_err)?;
        let new_orig = self
            .builder
            .build_call(
                realloc_fn,
                &[orig_ptr.into(), total_size.into()],
                "new_orig",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let new_orig_int = self
            .builder
            .build_ptr_to_int(new_orig, i64, "new_orig_int")
            .map_err(llvm_err)?;
        let new_data_int = self
            .builder
            .build_int_add(new_orig_int, rc_offset, "new_data_int")
            .map_err(llvm_err)?;
        let new_data = self
            .builder
            .build_int_to_ptr(new_data_int, ptr, "new_data")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lp_done);

        // Done block: phi for data/cap, store element, return
        self.builder.position_at_end(lp_done);
        let phi_final_data = self
            .builder
            .build_phi(ptr, "phi_final_data")
            .map_err(llvm_err)?;
        phi_final_data.add_incoming(&[(&check_data, lp_check_grow), (&new_data, lp_grow)]);
        let phi_final_cap = self
            .builder
            .build_phi(i64, "phi_final_cap")
            .map_err(llvm_err)?;
        phi_final_cap.add_incoming(&[(&cap, lp_check_grow), (&new_cap, lp_grow)]);
        let final_data = phi_final_data.as_basic_value().into_pointer_value();
        let final_cap = phi_final_cap.as_basic_value().into_int_value();
        // Store element at data[len] — element is {i64, ptr} fat struct (16 bytes)
        let data_i8 = self
            .builder
            .build_pointer_cast(
                final_data,
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "data_i8",
            )
            .map_err(llvm_err)?;
        let elem_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, data_i8, &[len], "elem_ptr")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(elem_ptr, elem).map_err(llvm_err)?;
        let new_len = self
            .builder
            .build_int_add(len, i64.const_int(1, false), "new_len")
            .map_err(llvm_err)?;
        let undef = list_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, final_data, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, new_len, 1, "r2")
            .map_err(llvm_err)?;
        let r3 = self
            .builder
            .build_insert_value(r2, final_cap, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r3));

        // ---- action_list_get({ptr, i64, i64}, i64) -> {i64, ptr} ----
        let list_get_fn = self.module.add_function(
            "action_list_get",
            self.string_type
                .fn_type(&[list_ty.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(list_get_fn, "entry");
        self.builder.position_at_end(entry);
        let list = list_get_fn.get_first_param().unwrap().into_struct_value();
        let idx = list_get_fn.get_nth_param(1).unwrap().into_int_value();
        let data_ptr = self
            .builder
            .build_extract_value(list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let data_i8 = self
            .builder
            .build_pointer_cast(
                data_ptr,
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "data_i8",
            )
            .map_err(llvm_err)?;
        let elem_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, data_i8, &[idx], "elem_ptr")
                .map_err(llvm_err)
        }?;
        let val = self
            .builder
            .build_load(self.string_type, elem_ptr, "val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&val));

        // ---- action_list_head({ptr, i64, i64}) -> {i64, ptr} ----
        let list_head_fn = self.module.add_function(
            "action_list_head",
            self.string_type.fn_type(&[list_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(list_head_fn, "entry");
        self.builder.position_at_end(entry);
        let lh_list = list_head_fn.get_first_param().unwrap().into_struct_value();
        let lh_len = self
            .builder
            .build_extract_value(lh_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let lh_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, lh_len, i64.const_int(0, false), "empty")
            .map_err(llvm_err)?;
        let lh_has = self.context.append_basic_block(list_head_fn, "has");
        let lh_none = self.context.append_basic_block(list_head_fn, "none");
        let _ = self
            .builder
            .build_conditional_branch(lh_empty, lh_none, lh_has);
        // Empty list: return {0, null}
        self.builder.position_at_end(lh_none);
        let lh_none_val = self.string_type.const_zero();
        let _ = self.builder.build_return(Some(&lh_none_val));
        // Has elements: return first
        self.builder.position_at_end(lh_has);
        let lh_data = self
            .builder
            .build_extract_value(lh_list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lh_data_i8 = self
            .builder
            .build_pointer_cast(
                lh_data,
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "data_i8",
            )
            .map_err(llvm_err)?;
        let lh_elem_ptr = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    lh_data_i8,
                    &[i64.const_int(0, false)],
                    "elem_ptr",
                )
                .map_err(llvm_err)
        }?;
        let lh_val = self
            .builder
            .build_load(self.string_type, lh_elem_ptr, "val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lh_val));

        // ---- action_list_len({ptr, i64, i64}) -> i64 ----
        let list_len_fn = self.module.add_function(
            "action_list_len",
            i64.fn_type(&[list_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(list_len_fn, "entry");
        self.builder.position_at_end(entry);
        let list = list_len_fn.get_first_param().unwrap().into_struct_value();
        let len = self
            .builder
            .build_extract_value(list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_return(Some(&len));

        // ---- action_list_contains({ptr, i64, i64}, {i64, ptr}) -> i1 ----
        let lc_fn = self.module.add_function(
            "action_list_contains",
            b1.fn_type(&[list_ty.into(), self.string_type.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(lc_fn, "entry");
        self.builder.position_at_end(entry);
        let lc_list = lc_fn.get_first_param().unwrap().into_struct_value();
        let lc_data = self
            .builder
            .build_extract_value(lc_list, 0, "lc_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lc_len = self
            .builder
            .build_extract_value(lc_list, 1, "lc_len")
            .map_err(llvm_err)?
            .into_int_value();
        let lc_key = lc_fn.get_nth_param(1).unwrap().into_struct_value();
        let lc_key_tag = self
            .builder
            .build_extract_value(lc_key, 0, "lc_ktag")
            .map_err(llvm_err)?
            .into_int_value();
        let lc_key_data = self
            .builder
            .build_extract_value(lc_key, 1, "lc_kdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Loop through elements
        let lc_loop_bb = self.context.append_basic_block(lc_fn, "lc_loop");
        let lc_done_bb = self.context.append_basic_block(lc_fn, "lc_done");
        let _ = self.builder.build_unconditional_branch(lc_loop_bb);
        self.builder.position_at_end(lc_loop_bb);
        let lc_i = self.builder.build_phi(i64, "lc_i").map_err(llvm_err)?;
        let lc_data_ptr = self
            .builder
            .build_pointer_cast(lc_data, self.ptr_ty(), "lc_dp")
            .map_err(llvm_err)?;
        let lc_elem_ptr = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    lc_data_ptr,
                    &[lc_i.as_basic_value().into_int_value()],
                    "lc_ep",
                )
                .map_err(llvm_err)
        }?;
        let lc_elem = self
            .builder
            .build_load(self.string_type, lc_elem_ptr, "lc_elem")
            .map_err(llvm_err)?;
        let lc_elem_ss = lc_elem.into_struct_value();
        let lc_elem_tag = self
            .builder
            .build_extract_value(lc_elem_ss, 0, "lc_etag")
            .map_err(llvm_err)?
            .into_int_value();
        let lc_elem_data = self
            .builder
            .build_extract_value(lc_elem_ss, 1, "lc_edata")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Compare first field (value for ints, length for strings)
        let tag_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, lc_elem_tag, lc_key_tag, "lc_teq")
            .map_err(llvm_err)?;
        let lc_next_bb = self.context.append_basic_block(lc_fn, "lc_next");
        let lc_check_bb = self.context.append_basic_block(lc_fn, "lc_check");
        let _ = self
            .builder
            .build_conditional_branch(tag_eq, lc_check_bb, lc_next_bb);
        // Check if both data pointers are null (scalars) or need content comparison
        self.builder.position_at_end(lc_check_bb);
        let null_ptr = self.ptr_ty().const_zero();
        let ed_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, lc_elem_data, null_ptr, "ed_null")
            .map_err(llvm_err)?;
        let kd_null = self
            .builder
            .build_int_compare(IntPredicate::EQ, lc_key_data, null_ptr, "kd_null")
            .map_err(llvm_err)?;
        let both_null = self
            .builder
            .build_and(ed_null, kd_null, "both_null")
            .map_err(llvm_err)?;
        let lc_found_bb = self.context.append_basic_block(lc_fn, "lc_found");
        let lc_content_bb = self.context.append_basic_block(lc_fn, "lc_content");
        let _ = self
            .builder
            .build_conditional_branch(both_null, lc_found_bb, lc_content_bb);
        self.builder.position_at_end(lc_found_bb);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        // One or both pointers non-null: both must be non-null for string comparison
        self.builder.position_at_end(lc_content_bb);
        let ed_nn = self.builder.build_not(ed_null, "ed_nn").map_err(llvm_err)?;
        let kd_nn = self.builder.build_not(kd_null, "kd_nn").map_err(llvm_err)?;
        let both_non_null = self
            .builder
            .build_and(ed_nn, kd_nn, "both_nn")
            .map_err(llvm_err)?;
        let lc_str_check_bb = self.context.append_basic_block(lc_fn, "lc_str_check");
        let _ = self
            .builder
            .build_conditional_branch(both_non_null, lc_str_check_bb, lc_next_bb);
        // Compare string content
        self.builder.position_at_end(lc_str_check_bb);
        let str_eq_call = self.call_rt(
            "action_string_eq",
            &[
                lc_elem_ss.as_basic_value_enum().into(),
                lc_key.as_basic_value_enum().into(),
            ],
        )?;
        let str_eq_val = str_eq_call
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let lc_str_found_bb = self.context.append_basic_block(lc_fn, "lc_str_found");
        let _ = self
            .builder
            .build_conditional_branch(str_eq_val, lc_str_found_bb, lc_next_bb);
        self.builder.position_at_end(lc_str_found_bb);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(lc_next_bb);
        let lc_next_i = self
            .builder
            .build_int_add(
                lc_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "lc_ni",
            )
            .map_err(llvm_err)?;
        let lc_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, lc_next_i, lc_len, "lc_done")
            .map_err(llvm_err)?;
        let lc_next_block = self.builder.get_insert_block().unwrap();
        lc_i.add_incoming(&[
            (
                &i64.const_int(0, false),
                lc_fn.get_first_basic_block().unwrap(),
            ),
            (&lc_next_i, lc_next_block),
        ]);
        let _ = self
            .builder
            .build_conditional_branch(lc_done, lc_done_bb, lc_loop_bb);
        self.builder.position_at_end(lc_done_bb);
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        // ---- action_list_reverse({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let lr_fn = self.module.add_function(
            "action_list_reverse",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let lr_entry = self.context.append_basic_block(lr_fn, "entry");
        self.builder.position_at_end(lr_entry);
        let lr_list = lr_fn.get_first_param().unwrap().into_struct_value();
        let lr_data = self
            .builder
            .build_extract_value(lr_list, 0, "lr_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lr_len = self
            .builder
            .build_extract_value(lr_list, 1, "lr_len")
            .map_err(llvm_err)?
            .into_int_value();
        let lr_cap = self
            .builder
            .build_extract_value(lr_list, 2, "lr_cap")
            .map_err(llvm_err)?
            .into_int_value();
        // Create new list with same capacity
        let lr_new = self
            .builder
            .build_call(list_create_fn, &[lr_cap.into()], "lr_new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        // Loop from 0 to len, get len-1-i element and push
        let lr_loop_bb = self.context.append_basic_block(lr_fn, "lr_loop");
        let lr_done_bb = self.context.append_basic_block(lr_fn, "lr_done");
        let _ = self.builder.build_unconditional_branch(lr_loop_bb);
        self.builder.position_at_end(lr_loop_bb);
        let lr_i = self.builder.build_phi(i64, "lr_i").map_err(llvm_err)?;
        let lr_list2 = self
            .builder
            .build_phi(list_ty, "lr_list2")
            .map_err(llvm_err)?;
        let lr_rev_idx = self
            .builder
            .build_int_sub(
                lr_len,
                self.builder
                    .build_int_add(
                        lr_i.as_basic_value().into_int_value(),
                        i64.const_int(1, false),
                        "lr_plus1",
                    )
                    .map_err(llvm_err)?,
                "lr_rev_idx",
            )
            .map_err(llvm_err)?;
        let lr_dp = self
            .builder
            .build_pointer_cast(lr_data, self.ptr_ty(), "lr_dp")
            .map_err(llvm_err)?;
        let lr_ep = unsafe {
            self.builder
                .build_gep(self.string_type, lr_dp, &[lr_rev_idx], "lr_ep")
                .map_err(llvm_err)
        }?;
        let lr_elem = self
            .builder
            .build_load(self.string_type, lr_ep, "lr_elem")
            .map_err(llvm_err)?;
        let lr_new2 = self
            .builder
            .build_call(
                list_push_fn,
                &[lr_list2.as_basic_value().into(), lr_elem.into()],
                "lr_push",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("push failed")?;
        let lr_next_i = self
            .builder
            .build_int_add(
                lr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "lr_ni",
            )
            .map_err(llvm_err)?;
        let lr_done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, lr_next_i, lr_len, "lr_done")
            .map_err(llvm_err)?;
        let lr_next_block = self.builder.get_insert_block().unwrap();
        lr_i.add_incoming(&[
            (&i64.const_int(0, false), lr_entry),
            (&lr_next_i, lr_next_block),
        ]);
        lr_list2.add_incoming(&[(&lr_new, lr_entry), (&lr_new2, lr_next_block)]);
        let _ = self
            .builder
            .build_conditional_branch(lr_done_cond, lr_done_bb, lr_loop_bb);
        self.builder.position_at_end(lr_done_bb);
        let lr_final = self
            .builder
            .build_phi(list_ty, "lr_final")
            .map_err(llvm_err)?;
        lr_final.add_incoming(&[(&lr_new2, lr_next_block)]);
        let _ = self.builder.build_return(Some(&lr_final.as_basic_value()));

        // ---- action_list_range(i64, i64) -> {ptr, i64, i64} ----
        let range_fn = self.module.add_function(
            "action_list_range",
            list_ty.fn_type(&[i64.into(), i64.into()], false),
            None,
        );
        let rg_entry = self.context.append_basic_block(range_fn, "entry");
        self.builder.position_at_end(rg_entry);
        let rg_start = range_fn.get_first_param().unwrap().into_int_value();
        let rg_end = range_fn.get_nth_param(1).unwrap().into_int_value();
        let rg_len = self
            .builder
            .build_int_sub(rg_end, rg_start, "rg_len")
            .map_err(llvm_err)?;
        let rg_cap = self
            .builder
            .build_int_add(rg_len, i64.const_int(1, false), "rg_cap")
            .map_err(llvm_err)?;
        let rg_list = self
            .builder
            .build_call(list_create_fn, &[rg_cap.into()], "rg_list")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let rg_loop_bb = self.context.append_basic_block(range_fn, "rg_loop");
        let rg_done_bb = self.context.append_basic_block(range_fn, "rg_done");
        let rg_check = self
            .builder
            .build_int_compare(IntPredicate::SLT, rg_start, rg_end, "rg_check")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rg_check, rg_loop_bb, rg_done_bb);
        self.builder.position_at_end(rg_loop_bb);
        let rg_i = self.builder.build_phi(i64, "rg_i").map_err(llvm_err)?;
        let rg_list2 = self
            .builder
            .build_phi(list_ty, "rg_list2")
            .map_err(llvm_err)?;
        // Create fat struct {i64 value, ptr null} for this Int
        let rg_fat_undef = self.string_type.get_undef();
        let rg_fat_val = self
            .builder
            .build_insert_value(
                rg_fat_undef,
                rg_i.as_basic_value().into_int_value(),
                0,
                "rg_fat_val",
            )
            .map_err(llvm_err)?;
        let rg_fat = self
            .builder
            .build_insert_value(rg_fat_val, self.ptr_ty().const_zero(), 1, "rg_fat")
            .map_err(llvm_err)?;
        let rg_list3 = self
            .builder
            .build_call(
                list_push_fn,
                &[
                    rg_list2.as_basic_value().into(),
                    rg_fat.as_basic_value_enum().into(),
                ],
                "rg_push",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("push failed")?;
        let rg_next = self
            .builder
            .build_int_add(
                rg_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "rg_next",
            )
            .map_err(llvm_err)?;
        let rg_done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, rg_next, rg_end, "rg_done_cond")
            .map_err(llvm_err)?;
        let rg_next_block = self.builder.get_insert_block().unwrap();
        rg_i.add_incoming(&[(&rg_start, rg_entry), (&rg_next, rg_next_block)]);
        rg_list2.add_incoming(&[(&rg_list, rg_entry), (&rg_list3, rg_next_block)]);
        let _ = self
            .builder
            .build_conditional_branch(rg_done_cond, rg_done_bb, rg_loop_bb);
        self.builder.position_at_end(rg_done_bb);
        let rg_final = self
            .builder
            .build_phi(list_ty, "rg_final")
            .map_err(llvm_err)?;
        rg_final.add_incoming(&[(&rg_list, rg_entry), (&rg_list3, rg_next_block)]);
        let _ = self.builder.build_return(Some(&rg_final.as_basic_value()));

        // ---- action_list_take({ptr, i64, i64}, i64) -> {ptr, i64, i64} ----
        let lt_fn = self.module.add_function(
            "action_list_take",
            list_ty.fn_type(&[list_ty.into(), i64.into()], false),
            None,
        );
        let lt_entry = self.context.append_basic_block(lt_fn, "entry");
        self.builder.position_at_end(lt_entry);
        let lt_list = lt_fn.get_first_param().unwrap().into_struct_value();
        let lt_n = lt_fn.get_nth_param(1).unwrap().into_int_value();
        let lt_len = self
            .builder
            .build_extract_value(lt_list, 1, "lt_len")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_actual = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, lt_n, lt_len, "lt_cmp")
                    .map_err(llvm_err)?,
                lt_n,
                lt_len,
                "lt_actual",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let lt_new = self
            .builder
            .build_call(list_create_fn, &[lt_actual.into()], "lt_new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let lt_data = self
            .builder
            .build_extract_value(lt_list, 0, "lt_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lt_dp = self
            .builder
            .build_pointer_cast(lt_data, self.ptr_ty(), "lt_dp")
            .map_err(llvm_err)?;
        let lt_loop_bb = self.context.append_basic_block(lt_fn, "lt_loop");
        let lt_done_bb = self.context.append_basic_block(lt_fn, "lt_done");
        let _ = self.builder.build_unconditional_branch(lt_loop_bb);
        self.builder.position_at_end(lt_loop_bb);
        let lt_i = self.builder.build_phi(i64, "lt_i").map_err(llvm_err)?;
        let lt_cur = self
            .builder
            .build_phi(list_ty, "lt_cur")
            .map_err(llvm_err)?;
        let lt_ep = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    lt_dp,
                    &[lt_i.as_basic_value().into_int_value()],
                    "lt_ep",
                )
                .map_err(llvm_err)
        }?;
        let lt_elem = self
            .builder
            .build_load(self.string_type, lt_ep, "lt_elem")
            .map_err(llvm_err)?;
        let lt_cur2 = self
            .builder
            .build_call(
                list_push_fn,
                &[lt_cur.as_basic_value().into(), lt_elem.into()],
                "lt_push",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("push failed")?;
        let lt_ni = self
            .builder
            .build_int_add(
                lt_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "lt_ni",
            )
            .map_err(llvm_err)?;
        let lt_done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, lt_ni, lt_actual, "lt_done")
            .map_err(llvm_err)?;
        let lt_next_block = self.builder.get_insert_block().unwrap();
        lt_i.add_incoming(&[
            (&i64.const_int(0, false), lt_entry),
            (&lt_ni, lt_next_block),
        ]);
        lt_cur.add_incoming(&[(&lt_new, lt_entry), (&lt_cur2, lt_next_block)]);
        let _ = self
            .builder
            .build_conditional_branch(lt_done_cond, lt_done_bb, lt_loop_bb);
        self.builder.position_at_end(lt_done_bb);
        let lt_final = self
            .builder
            .build_phi(list_ty, "lt_final")
            .map_err(llvm_err)?;
        lt_final.add_incoming(&[(&lt_cur2, lt_next_block)]);
        let _ = self.builder.build_return(Some(&lt_final.as_basic_value()));

        // ---- action_list_drop({ptr, i64, i64}, i64) -> {ptr, i64, i64} ----
        let ld_fn = self.module.add_function(
            "action_list_drop",
            list_ty.fn_type(&[list_ty.into(), i64.into()], false),
            None,
        );
        let ld_entry = self.context.append_basic_block(ld_fn, "entry");
        self.builder.position_at_end(ld_entry);
        let ld_list = ld_fn.get_first_param().unwrap().into_struct_value();
        let ld_n = ld_fn.get_nth_param(1).unwrap().into_int_value();
        let ld_len = self
            .builder
            .build_extract_value(ld_list, 1, "ld_len")
            .map_err(llvm_err)?
            .into_int_value();
        let ld_data = self
            .builder
            .build_extract_value(ld_list, 0, "ld_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ld_start = self
            .builder
            .build_select(
                self.builder
                    .build_int_compare(IntPredicate::SLT, ld_n, ld_len, "ld_cmp")
                    .map_err(llvm_err)?,
                ld_n,
                ld_len,
                "ld_start",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let ld_remaining = self
            .builder
            .build_int_sub(ld_len, ld_start, "ld_rem")
            .map_err(llvm_err)?;
        let ld_cap = self
            .builder
            .build_int_add(ld_remaining, i64.const_int(1, false), "ld_cap")
            .map_err(llvm_err)?;
        let ld_new = self
            .builder
            .build_call(list_create_fn, &[ld_cap.into()], "ld_new")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("create failed")?;
        let ld_dp = self
            .builder
            .build_pointer_cast(ld_data, self.ptr_ty(), "ld_dp")
            .map_err(llvm_err)?;
        let ld_loop_bb = self.context.append_basic_block(ld_fn, "ld_loop");
        let ld_done_bb = self.context.append_basic_block(ld_fn, "ld_done");
        let _ = self.builder.build_unconditional_branch(ld_loop_bb);
        self.builder.position_at_end(ld_loop_bb);
        let ld_i = self.builder.build_phi(i64, "ld_i").map_err(llvm_err)?;
        let ld_cur = self
            .builder
            .build_phi(list_ty, "ld_cur")
            .map_err(llvm_err)?;
        let ld_idx = self
            .builder
            .build_int_add(ld_i.as_basic_value().into_int_value(), ld_start, "ld_idx")
            .map_err(llvm_err)?;
        let ld_ep = unsafe {
            self.builder
                .build_gep(self.string_type, ld_dp, &[ld_idx], "ld_ep")
                .map_err(llvm_err)
        }?;
        let ld_elem = self
            .builder
            .build_load(self.string_type, ld_ep, "ld_elem")
            .map_err(llvm_err)?;
        let ld_cur2 = self
            .builder
            .build_call(
                list_push_fn,
                &[ld_cur.as_basic_value().into(), ld_elem.into()],
                "ld_push",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("push failed")?;
        let ld_ni = self
            .builder
            .build_int_add(
                ld_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ld_ni",
            )
            .map_err(llvm_err)?;
        let ld_done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, ld_ni, ld_remaining, "ld_done")
            .map_err(llvm_err)?;
        let ld_next_block = self.builder.get_insert_block().unwrap();
        ld_i.add_incoming(&[
            (&i64.const_int(0, false), ld_entry),
            (&ld_ni, ld_next_block),
        ]);
        ld_cur.add_incoming(&[(&ld_new, ld_entry), (&ld_cur2, ld_next_block)]);
        let _ = self
            .builder
            .build_conditional_branch(ld_done_cond, ld_done_bb, ld_loop_bb);
        self.builder.position_at_end(ld_done_bb);
        let ld_final = self
            .builder
            .build_phi(list_ty, "ld_final")
            .map_err(llvm_err)?;
        ld_final.add_incoming(&[(&ld_cur2, ld_next_block)]);
        let _ = self.builder.build_return(Some(&ld_final.as_basic_value()));

        // ---- abs(i64) -> i64 ----
        let abs_fn = self
            .module
            .add_function("abs", i64.fn_type(&[i64.into()], false), None);
        let entry = self.context.append_basic_block(abs_fn, "entry");
        self.builder.position_at_end(entry);
        let x = abs_fn.get_first_param().unwrap().into_int_value();
        let neg = self.builder.build_int_neg(x, "neg").map_err(llvm_err)?;
        let is_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, x, i64.const_int(0, false), "is_neg")
            .map_err(llvm_err)?;
        let result = self
            .builder
            .build_select(is_neg, neg, x, "abs_result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&result.into_int_value()));

        // ---- min(i64, i64) -> i64 ----
        let min_fn =
            self.module
                .add_function("min", i64.fn_type(&[i64.into(), i64.into()], false), None);
        let entry = self.context.append_basic_block(min_fn, "entry");
        self.builder.position_at_end(entry);
        let a = min_fn.get_first_param().unwrap().into_int_value();
        let b = min_fn.get_nth_param(1).unwrap().into_int_value();
        let lt = self
            .builder
            .build_int_compare(IntPredicate::SLT, a, b, "lt")
            .map_err(llvm_err)?;
        let min_result = self
            .builder
            .build_select(lt, a, b, "min_result")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_return(Some(&min_result.into_int_value()));

        // ---- max(i64, i64) -> i64 ----
        let max_fn =
            self.module
                .add_function("max", i64.fn_type(&[i64.into(), i64.into()], false), None);
        let entry = self.context.append_basic_block(max_fn, "entry");
        self.builder.position_at_end(entry);
        let ma = max_fn.get_first_param().unwrap().into_int_value();
        let mb = max_fn.get_nth_param(1).unwrap().into_int_value();
        let gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, ma, mb, "gt")
            .map_err(llvm_err)?;
        let max_result = self
            .builder
            .build_select(gt, ma, mb, "max_result")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_return(Some(&max_result.into_int_value()));

        // ---- action_string_to_upper({i64, ptr}) -> {i64, ptr} ----
        let to_upper_fn = self.module.add_function(
            "action_string_to_upper",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(to_upper_fn, "entry");
        self.builder.position_at_end(entry);
        let str_param = to_upper_fn.get_first_param().unwrap().into_struct_value();
        let str_len = self
            .builder
            .build_extract_value(str_param, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let str_data = self
            .builder
            .build_extract_value(str_param, 1, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alloc_len = self
            .builder
            .build_int_add(str_len, i64.const_int(1, false), "alloc_len")
            .map_err(llvm_err)?;
        let new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[alloc_len.into()], "new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Loop: for i in 0..len, copy byte, convert if lowercase
        let loop_bb = self.context.append_basic_block(to_upper_fn, "loop");
        let body_bb = self.context.append_basic_block(to_upper_fn, "body");
        let done_bb = self.context.append_basic_block(to_upper_fn, "done");
        let i_alloca = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_bb);
        self.builder.position_at_end(loop_bb);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let not_done = self
            .builder
            .build_int_compare(IntPredicate::ULT, i_val, str_len, "not_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(not_done, body_bb, done_bb);
        self.builder.position_at_end(body_bb);
        let src_ptr = unsafe {
            self.builder
                .build_gep(i8, str_data, &[i_val], "src_ptr")
                .map_err(llvm_err)
        }?;
        let c = self
            .builder
            .build_load(i8, src_ptr, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let is_lower = self
            .builder
            .build_int_compare(
                IntPredicate::UGE,
                c,
                i8.const_int('a' as u64, false),
                "ge_a",
            )
            .map_err(llvm_err)?;
        let is_lower2 = self
            .builder
            .build_int_compare(
                IntPredicate::ULE,
                c,
                i8.const_int('z' as u64, false),
                "le_z",
            )
            .map_err(llvm_err)?;
        let is_lower_final = self
            .builder
            .build_and(is_lower, is_lower2, "is_lower")
            .map_err(llvm_err)?;
        let upper_c = self
            .builder
            .build_int_sub(c, i8.const_int(32, false), "upper_c")
            .map_err(llvm_err)?;
        let conv = self
            .builder
            .build_select(is_lower_final, upper_c, c, "conv")
            .map_err(llvm_err)?
            .into_int_value();
        let dst_ptr = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[i_val], "dst_ptr")
                .map_err(llvm_err)
        }?;
        self.builder.build_store(dst_ptr, conv).map_err(llvm_err)?;
        let next_i = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "next_i")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_bb);
        self.builder.position_at_end(done_bb);
        let null_gep = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[str_len], "null_ptr")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(null_gep, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, str_len, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, new_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        // ---- action_string_to_lower({i64, ptr}) -> {i64, ptr} ----
        let to_lower_fn = self.module.add_function(
            "action_string_to_lower",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(to_lower_fn, "entry");
        self.builder.position_at_end(entry);
        let str_param = to_lower_fn.get_first_param().unwrap().into_struct_value();
        let str_len = self
            .builder
            .build_extract_value(str_param, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let str_data = self
            .builder
            .build_extract_value(str_param, 1, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alloc_len = self
            .builder
            .build_int_add(str_len, i64.const_int(1, false), "alloc_len")
            .map_err(llvm_err)?;
        let new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[alloc_len.into()], "new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let loop_bb = self.context.append_basic_block(to_lower_fn, "loop");
        let body_bb = self.context.append_basic_block(to_lower_fn, "body");
        let done_bb = self.context.append_basic_block(to_lower_fn, "done");
        let i_alloca = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_bb);
        self.builder.position_at_end(loop_bb);
        let i_val = self
            .builder
            .build_load(i64, i_alloca, "i_val")
            .map_err(llvm_err)?
            .into_int_value();
        let not_done = self
            .builder
            .build_int_compare(IntPredicate::ULT, i_val, str_len, "not_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(not_done, body_bb, done_bb);
        self.builder.position_at_end(body_bb);
        let src_ptr = unsafe {
            self.builder
                .build_gep(i8, str_data, &[i_val], "src_ptr")
                .map_err(llvm_err)
        }?;
        let c = self
            .builder
            .build_load(i8, src_ptr, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let is_upper = self
            .builder
            .build_int_compare(
                IntPredicate::UGE,
                c,
                i8.const_int('A' as u64, false),
                "ge_A",
            )
            .map_err(llvm_err)?;
        let is_upper2 = self
            .builder
            .build_int_compare(
                IntPredicate::ULE,
                c,
                i8.const_int('Z' as u64, false),
                "le_Z",
            )
            .map_err(llvm_err)?;
        let is_upper_final = self
            .builder
            .build_and(is_upper, is_upper2, "is_upper")
            .map_err(llvm_err)?;
        let lower_c = self
            .builder
            .build_int_add(c, i8.const_int(32, false), "lower_c")
            .map_err(llvm_err)?;
        let conv = self
            .builder
            .build_select(is_upper_final, lower_c, c, "conv")
            .map_err(llvm_err)?
            .into_int_value();
        let dst_ptr = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[i_val], "dst_ptr")
                .map_err(llvm_err)
        }?;
        self.builder.build_store(dst_ptr, conv).map_err(llvm_err)?;
        let next_i = self
            .builder
            .build_int_add(i_val, i64.const_int(1, false), "next_i")
            .map_err(llvm_err)?;
        self.builder
            .build_store(i_alloca, next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(loop_bb);
        self.builder.position_at_end(done_bb);
        let null_gep = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[str_len], "null_ptr")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(null_gep, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, str_len, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, new_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        // ---- action_string_trim({i64, ptr}) -> {i64, ptr} ----
        let trim_fn = self.module.add_function(
            "action_string_trim",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(trim_fn, "entry");
        self.builder.position_at_end(entry);
        let str_param = trim_fn.get_first_param().unwrap().into_struct_value();
        let str_len = self
            .builder
            .build_extract_value(str_param, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let str_data = self
            .builder
            .build_extract_value(str_param, 1, "data")
            .map_err(llvm_err)?
            .into_pointer_value();

        // Helper to build is-whitespace check for a char value
        let build_is_ws = |builder: &inkwell::builder::Builder<'ctx>,
                           c: IntValue<'ctx>|
         -> Result<IntValue<'ctx>, String> {
            let is_sp = builder
                .build_int_compare(
                    IntPredicate::EQ,
                    c,
                    i8.const_int(b' ' as u64, false),
                    "is_sp",
                )
                .map_err(llvm_err)?;
            let is_tab = builder
                .build_int_compare(
                    IntPredicate::EQ,
                    c,
                    i8.const_int(b'\t' as u64, false),
                    "is_tab",
                )
                .map_err(llvm_err)?;
            let is_nl = builder
                .build_int_compare(
                    IntPredicate::EQ,
                    c,
                    i8.const_int(b'\n' as u64, false),
                    "is_nl",
                )
                .map_err(llvm_err)?;
            let is_cr = builder
                .build_int_compare(
                    IntPredicate::EQ,
                    c,
                    i8.const_int(b'\r' as u64, false),
                    "is_cr",
                )
                .map_err(llvm_err)?;
            let ws1 = builder.build_or(is_sp, is_tab, "ws1").map_err(llvm_err)?;
            let ws2 = builder.build_or(is_nl, is_cr, "ws2").map_err(llvm_err)?;
            builder.build_or(ws1, ws2, "is_ws").map_err(llvm_err)
        };

        // Find start (left trim)
        let find_start_hdr = self.context.append_basic_block(trim_fn, "find_start_hdr");
        let find_start_body = self.context.append_basic_block(trim_fn, "find_start_body");
        let start_done = self.context.append_basic_block(trim_fn, "start_done");
        let start_idx = self
            .builder
            .build_alloca(i64, "start_idx")
            .map_err(llvm_err)?;
        self.builder
            .build_store(start_idx, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(find_start_hdr);

        // find_start_hdr: while start < len
        self.builder.position_at_end(find_start_hdr);
        let si = self
            .builder
            .build_load(i64, start_idx, "si")
            .map_err(llvm_err)?
            .into_int_value();
        let si_lt_len = self
            .builder
            .build_int_compare(IntPredicate::ULT, si, str_len, "si_lt_len")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(si_lt_len, find_start_body, start_done);

        self.builder.position_at_end(find_start_body);
        let sp = unsafe {
            self.builder
                .build_gep(i8, str_data, &[si], "sp")
                .map_err(llvm_err)
        }?;
        let sc = self
            .builder
            .build_load(i8, sp, "sc")
            .map_err(llvm_err)?
            .into_int_value();
        let is_ws = build_is_ws(&self.builder, sc)?;
        let si_plus1 = self
            .builder
            .build_int_add(si, i64.const_int(1, false), "si_plus1")
            .map_err(llvm_err)?;
        let new_si = self
            .builder
            .build_select(is_ws, si_plus1, si, "new_si")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(start_idx, new_si)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_ws, find_start_hdr, start_done);

        // Find end (right trim) - similar loop going backwards
        self.builder.position_at_end(start_done);
        let find_end_hdr = self.context.append_basic_block(trim_fn, "find_end_hdr");
        let find_end_body = self.context.append_basic_block(trim_fn, "find_end_body");
        let end_done = self.context.append_basic_block(trim_fn, "end_done");
        let end_idx = self
            .builder
            .build_alloca(i64, "end_idx")
            .map_err(llvm_err)?;
        self.builder
            .build_store(end_idx, str_len)
            .map_err(llvm_err)?;
        // Load start value here so it dominates uses in end_done
        let final_si = self
            .builder
            .build_load(i64, start_idx, "final_si")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_unconditional_branch(find_end_hdr);

        // find_end_hdr: while end > start
        self.builder.position_at_end(find_end_hdr);
        let ei = self
            .builder
            .build_load(i64, end_idx, "ei")
            .map_err(llvm_err)?
            .into_int_value();
        let ei_gt_si = self
            .builder
            .build_int_compare(IntPredicate::UGT, ei, final_si, "ei_gt_si")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ei_gt_si, find_end_body, end_done);

        self.builder.position_at_end(find_end_body);
        let ei_minus1 = self
            .builder
            .build_int_sub(ei, i64.const_int(1, false), "ei_minus1")
            .map_err(llvm_err)?;
        let ep = unsafe {
            self.builder
                .build_gep(i8, str_data, &[ei_minus1], "ep")
                .map_err(llvm_err)
        }?;
        let ec = self
            .builder
            .build_load(i8, ep, "ec")
            .map_err(llvm_err)?
            .into_int_value();
        let is_ws = build_is_ws(&self.builder, ec)?;
        let new_ei = self
            .builder
            .build_select(is_ws, ei_minus1, ei, "new_ei")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(end_idx, new_ei)
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_ws, find_end_hdr, end_done);

        // end_done: allocate and copy
        self.builder.position_at_end(end_done);
        // Reload end since it might have changed in the loop
        let final_ei = self
            .builder
            .build_load(i64, end_idx, "final_ei")
            .map_err(llvm_err)?
            .into_int_value();
        let new_len = self
            .builder
            .build_int_sub(final_ei, final_si, "new_len")
            .map_err(llvm_err)?;
        // Allocate new_len + 1 for null terminator
        let alloc_len = self
            .builder
            .build_int_add(new_len, i64.const_int(1, false), "alloc_len")
            .map_err(llvm_err)?;
        let new_buf = self
            .builder
            .build_call(malloc_rc_fn, &[alloc_len.into()], "new_buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let src_offset = unsafe {
            self.builder
                .build_gep(i8, str_data, &[final_si], "src_offset")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[new_buf.into(), src_offset.into(), new_len.into()],
                "",
            )
            .map_err(llvm_err)?;
        // Null terminate
        let null_gep = unsafe {
            self.builder
                .build_gep(i8, new_buf, &[new_len], "null_ptr")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(null_gep, i8.const_int(0, false))
            .map_err(llvm_err)?;
        // Return {new_len, new_buf}
        let undef = str_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, new_len, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, new_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r2));

        // ---- action_map_create(i64 capacity) -> {ptr, i64, i64} ----
        // Same layout as list but 32 bytes per entry (4 * i64 for key+val fat structs)
        let map_create_fn = self.module.add_function(
            "action_map_create",
            list_ty.fn_type(&[i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(map_create_fn, "entry");
        self.builder.position_at_end(entry);
        let cap = map_create_fn.get_first_param().unwrap().into_int_value();
        let thirty_two = i64.const_int(32, false);
        let data_size = self
            .builder
            .build_int_mul(cap, thirty_two, "m_data_size")
            .map_err(llvm_err)?;
        let data = self
            .builder
            .build_call(malloc_rc_fn, &[data_size.into()], "m_data")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let zero = i64.const_int(0, false);
        let undef = list_ty.get_undef();
        let r1 = self
            .builder
            .build_insert_value(undef, data, 0, "r1")
            .map_err(llvm_err)?;
        let r2 = self
            .builder
            .build_insert_value(r1, zero, 1, "r2")
            .map_err(llvm_err)?;
        let r3 = self
            .builder
            .build_insert_value(r2, cap, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&r3));

        // ---- action_map_insert / action_map_get / action_map_contains ----
        // These are implemented as simple linear-search functions.
        // For simplicity, all three are thin wrappers over a shared pattern:
        // iterate entries, compare fat-struct keys, then either update/get/check.

        // action_map_insert({ptr,i64,i64}, {i64,ptr}, {i64,ptr}) -> {ptr,i64,i64}
        let mi_fn = self.module.add_function(
            "action_map_insert",
            list_ty.fn_type(&[list_ty.into(), str_ty.into(), str_ty.into()], false),
            None,
        );
        let mi_entry = self.context.append_basic_block(mi_fn, "entry");
        let mi_cow = self.context.append_basic_block(mi_fn, "cow_clone");
        let mi_merge = self.context.append_basic_block(mi_fn, "merge");
        let mi_search = self.context.append_basic_block(mi_fn, "search");
        let mi_body = self.context.append_basic_block(mi_fn, "body");
        let mi_ckey = self.context.append_basic_block(mi_fn, "ckey");
        let mi_update = self.context.append_basic_block(mi_fn, "update");
        let mi_next = self.context.append_basic_block(mi_fn, "next");
        let mi_append_check = self.context.append_basic_block(mi_fn, "append_ck");
        let mi_grow = self.context.append_basic_block(mi_fn, "append_grow");
        let mi_append_store = self.context.append_basic_block(mi_fn, "append_store");

        self.builder.position_at_end(mi_entry);
        let mi_map = mi_fn.get_first_param().unwrap().into_struct_value();
        let mi_key = mi_fn.get_nth_param(1).unwrap().into_struct_value();
        let mi_val = mi_fn.get_nth_param(2).unwrap().into_struct_value();
        let mi_data = self
            .builder
            .build_extract_value(mi_map, 0, "d")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mi_len = self
            .builder
            .build_extract_value(mi_map, 1, "l")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_cap = self
            .builder
            .build_extract_value(mi_map, 2, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_ktag = self
            .builder
            .build_extract_value(mi_key, 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_kptr = self
            .builder
            .build_extract_value(mi_key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mi_vtag = self
            .builder
            .build_extract_value(mi_val, 0, "vt")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_vptr = self
            .builder
            .build_extract_value(mi_val, 1, "vp")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Convert pointers to i64 for storage/compare
        let mi_kp_i64 = self
            .builder
            .build_ptr_to_int(mi_kptr, i64, "kp_i64")
            .map_err(llvm_err)?;
        let mi_vp_i64 = self
            .builder
            .build_ptr_to_int(mi_vptr, i64, "vp_i64")
            .map_err(llvm_err)?;
        // CoW check for map: if refcount > 1, clone the data buffer
        let mi_data_int = self
            .builder
            .build_ptr_to_int(mi_data, i64, "mi_data_int")
            .map_err(llvm_err)?;
        let mi_rc_addr = self
            .builder
            .build_int_sub(mi_data_int, i64.const_int(8, false), "mi_rc_addr")
            .map_err(llvm_err)?;
        let mi_rc_ptr = self
            .builder
            .build_int_to_ptr(mi_rc_addr, ptr, "mi_rc_ptr")
            .map_err(llvm_err)?;
        let mi_rc_val = self
            .builder
            .build_load(self.i64_ty(), mi_rc_ptr, "mi_rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_need_cow = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                mi_rc_val,
                i64.const_int(1, false),
                "mi_need_cow",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_need_cow, mi_cow, mi_merge);

        // CoW clone for map: clone cap*32 bytes (4*i64 per entry)
        self.builder.position_at_end(mi_cow);
        let mi_cow_size = self
            .builder
            .build_int_mul(mi_cap, i64.const_int(32, false), "mi_cow_size")
            .map_err(llvm_err)?;
        let mi_new_data = self
            .builder
            .build_call(malloc_rc_fn, &[mi_cow_size.into()], "mi_new_data")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mi_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                mi_memcpy_fn,
                &[mi_new_data.into(), mi_data.into(), mi_cow_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        let mi_cow_old_rc = self
            .builder
            .build_load(self.i64_ty(), mi_rc_ptr, "mi_cow_old_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_cow_new_rc = self
            .builder
            .build_int_sub(mi_cow_old_rc, i64.const_int(1, false), "mi_cow_new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mi_rc_ptr, mi_cow_new_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_merge);

        // Merge block: phi for data, then continue
        self.builder.position_at_end(mi_merge);
        let mi_data_phi = self
            .builder
            .build_phi(ptr, "mi_data_phi")
            .map_err(llvm_err)?;
        mi_data_phi.add_incoming(&[(&mi_data, mi_entry), (&mi_new_data, mi_cow)]);
        let mi_data = mi_data_phi.as_basic_value().into_pointer_value();
        let mi_di64 = self
            .builder
            .build_pointer_cast(mi_data, ptr, "di64")
            .map_err(llvm_err)?;
        let mi_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(mi_i, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_search);

        self.builder.position_at_end(mi_search);
        let mi_iv = self
            .builder
            .build_load(i64, mi_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mi_iv, mi_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_cond, mi_body, mi_append_check);

        self.builder.position_at_end(mi_body);
        let mi_off = self
            .builder
            .build_int_mul(mi_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        // Compare stored key tag with arg key tag
        let mi_etp = unsafe {
            self.builder
                .build_gep(i64, mi_di64, &[mi_off], "etp")
                .map_err(llvm_err)
        }?;
        let mi_et = self
            .builder
            .build_load(i64, mi_etp, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let mi_teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, mi_et, mi_ktag, "teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_teq, mi_ckey, mi_next);

        self.builder.position_at_end(mi_ckey);
        let mi_off1 = self
            .builder
            .build_int_add(mi_off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let mi_epp = unsafe {
            self.builder
                .build_gep(i64, mi_di64, &[mi_off1], "epp")
                .map_err(llvm_err)
        }?;
        let mi_ep = self
            .builder
            .build_load(i64, mi_epp, "ep")
            .map_err(llvm_err)?
            .into_int_value();
        // If key ptr is 0 (value type), tag match is enough.
        // Otherwise use action_string_eq for proper content comparison (handles strings, enums, structs).
        let mi_kpz = self
            .builder
            .build_int_compare(IntPredicate::EQ, mi_kp_i64, zero, "kpz")
            .map_err(llvm_err)?;
        // Build entry key fat struct for string_eq call
        let mi_ek_undef = str_ty.get_undef();
        let mi_ek1 = self
            .builder
            .build_insert_value(mi_ek_undef, mi_et, 0, "ek1")
            .map_err(llvm_err)?;
        let mi_ep_ptr = self
            .builder
            .build_int_to_ptr(mi_ep, ptr, "ep_ptr")
            .map_err(llvm_err)?;
        let mi_ek2 = self
            .builder
            .build_insert_value(mi_ek1, mi_ep_ptr, 1, "ek2")
            .map_err(llvm_err)?;
        let seq_fn = self.module.get_function("action_string_eq").unwrap();
        let mi_seq = self
            .builder
            .build_call(
                seq_fn,
                &[mi_ek2.as_basic_value_enum().into(), mi_key.into()],
                "seq",
            )
            .map_err(llvm_err)?;
        let mi_seq_r = mi_seq.try_as_basic_value().unwrap_basic().into_int_value();
        let mi_feq = self
            .builder
            .build_select(mi_kpz, mi_teq, mi_seq_r, "feq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mi_feq.into_int_value(), mi_update, mi_next);

        self.builder.position_at_end(mi_update);
        let mi_off2 = self
            .builder
            .build_int_add(mi_off, i64.const_int(2, false), "off2")
            .map_err(llvm_err)?;
        let mi_vtp = unsafe {
            self.builder
                .build_gep(i64, mi_di64, &[mi_off2], "vtp")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(mi_vtp, mi_vtag)
            .map_err(llvm_err)?;
        let mi_off3 = self
            .builder
            .build_int_add(mi_off, i64.const_int(3, false), "off3")
            .map_err(llvm_err)?;
        let mi_vpp = unsafe {
            self.builder
                .build_gep(i64, mi_di64, &[mi_off3], "vpp")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(mi_vpp, mi_vp_i64)
            .map_err(llvm_err)?;
        let mi_ur = list_ty.get_undef();
        let mi_r1 = self
            .builder
            .build_insert_value(mi_ur, mi_data, 0, "r1")
            .map_err(llvm_err)?;
        let mi_r2 = self
            .builder
            .build_insert_value(mi_r1, mi_len, 1, "r2")
            .map_err(llvm_err)?;
        let mi_r3 = self
            .builder
            .build_insert_value(mi_r2, mi_cap, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mi_r3));

        self.builder.position_at_end(mi_next);
        let mi_niv = self
            .builder
            .build_int_add(mi_iv, i64.const_int(1, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(mi_i, mi_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_search);

        // Capacity check before appending
        self.builder.position_at_end(mi_append_check);
        let need_grow = self
            .builder
            .build_int_compare(IntPredicate::SGE, mi_len, mi_cap, "need_grow")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(need_grow, mi_grow, mi_append_store);

        // Grow block: double capacity (min 4), realloc with RC header, then branch to store
        self.builder.position_at_end(mi_grow);
        let min_cap = i64.const_int(4, false);
        let cap_small = self
            .builder
            .build_int_compare(IntPredicate::SLT, mi_cap, min_cap, "cap_small")
            .map_err(llvm_err)?;
        let cap2x = self
            .builder
            .build_int_mul(mi_cap, i64.const_int(2, false), "cap2x")
            .map_err(llvm_err)?;
        let new_cap = self
            .builder
            .build_select(cap_small, min_cap, cap2x, "new_cap")
            .map_err(llvm_err)?
            .into_int_value();
        let data_size = self
            .builder
            .build_int_mul(new_cap, i64.const_int(32, false), "data_size")
            .map_err(llvm_err)?;
        let total_size = self
            .builder
            .build_int_add(data_size, i64.const_int(8, false), "total_size")
            .map_err(llvm_err)?;
        // Adjust mi_data back to original allocation (RC header at -8)
        let data_int = self
            .builder
            .build_ptr_to_int(mi_data, i64, "mi_data_int")
            .map_err(llvm_err)?;
        let rc_offset = i64.const_int(8, false);
        let orig_int = self
            .builder
            .build_int_sub(data_int, rc_offset, "mi_orig_int")
            .map_err(llvm_err)?;
        let orig_ptr = self
            .builder
            .build_int_to_ptr(orig_int, ptr, "mi_orig_ptr")
            .map_err(llvm_err)?;
        let new_orig = self
            .builder
            .build_call(
                realloc_fn,
                &[orig_ptr.into(), total_size.into()],
                "mi_new_orig",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let new_orig_int = self
            .builder
            .build_ptr_to_int(new_orig, i64, "mi_new_orig_int")
            .map_err(llvm_err)?;
        let new_data_int = self
            .builder
            .build_int_add(new_orig_int, rc_offset, "mi_new_data_int")
            .map_err(llvm_err)?;
        let new_data = self
            .builder
            .build_int_to_ptr(new_data_int, ptr, "mi_new_data")
            .map_err(llvm_err)?;
        let new_di64 = self
            .builder
            .build_pointer_cast(new_data, ptr, "new_di64")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mi_append_store);

        // Store block: phi for data/cap, then write entry
        self.builder.position_at_end(mi_append_store);
        let phi_data = self.builder.build_phi(ptr, "phi_data").map_err(llvm_err)?;
        phi_data.add_incoming(&[(&mi_data, mi_append_check), (&new_data, mi_grow)]);
        let phi_di64 = self.builder.build_phi(ptr, "phi_di64").map_err(llvm_err)?;
        phi_di64.add_incoming(&[(&mi_di64, mi_append_check), (&new_di64, mi_grow)]);
        let phi_cap = self.builder.build_phi(i64, "phi_cap").map_err(llvm_err)?;
        phi_cap.add_incoming(&[(&mi_cap, mi_append_check), (&new_cap, mi_grow)]);
        let final_data = phi_data.as_basic_value().into_pointer_value();
        let final_di64 = phi_di64.as_basic_value().into_pointer_value();
        let final_cap = phi_cap.as_basic_value().into_int_value();
        let mi_lo = self
            .builder
            .build_int_mul(mi_len, i64.const_int(4, false), "lo")
            .map_err(llvm_err)?;
        let mi_nkt = unsafe {
            self.builder
                .build_gep(i64, final_di64, &[mi_lo], "nkt")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(mi_nkt, mi_ktag)
            .map_err(llvm_err)?;
        let mi_lo1 = self
            .builder
            .build_int_add(mi_lo, i64.const_int(1, false), "lo1")
            .map_err(llvm_err)?;
        let mi_nkp = unsafe {
            self.builder
                .build_gep(i64, final_di64, &[mi_lo1], "nkp")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(mi_nkp, mi_kp_i64)
            .map_err(llvm_err)?;
        let mi_lo2 = self
            .builder
            .build_int_add(mi_lo, i64.const_int(2, false), "lo2")
            .map_err(llvm_err)?;
        let mi_nvt = unsafe {
            self.builder
                .build_gep(i64, final_di64, &[mi_lo2], "nvt")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(mi_nvt, mi_vtag)
            .map_err(llvm_err)?;
        let mi_lo3 = self
            .builder
            .build_int_add(mi_lo, i64.const_int(3, false), "lo3")
            .map_err(llvm_err)?;
        let mi_nvp = unsafe {
            self.builder
                .build_gep(i64, final_di64, &[mi_lo3], "nvp")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(mi_nvp, mi_vp_i64)
            .map_err(llvm_err)?;
        let mi_nl = self
            .builder
            .build_int_add(mi_len, i64.const_int(1, false), "nl")
            .map_err(llvm_err)?;
        let mi_ur2 = list_ty.get_undef();
        let mi_rr1 = self
            .builder
            .build_insert_value(mi_ur2, final_data, 0, "rr1")
            .map_err(llvm_err)?;
        let mi_rr2 = self
            .builder
            .build_insert_value(mi_rr1, mi_nl, 1, "rr2")
            .map_err(llvm_err)?;
        let mi_rr3 = self
            .builder
            .build_insert_value(mi_rr2, final_cap, 2, "rr3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mi_rr3));

        // action_map_get({ptr,i64,i64}, {i64,ptr}) -> {i64,ptr}
        let mg_fn = self.module.add_function(
            "action_map_get",
            str_ty.fn_type(&[list_ty.into(), str_ty.into()], false),
            None,
        );
        let mg_blocks: Vec<_> = (0..7)
            .map(|i| self.context.append_basic_block(mg_fn, &format!("b{}", i)))
            .collect();
        self.builder.position_at_end(mg_blocks[0]); // entry
        let mg_map = mg_fn.get_first_param().unwrap().into_struct_value();
        let mg_key = mg_fn.get_nth_param(1).unwrap().into_struct_value();
        let mg_data = self
            .builder
            .build_extract_value(mg_map, 0, "d")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mg_len = self
            .builder
            .build_extract_value(mg_map, 1, "l")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_ktag = self
            .builder
            .build_extract_value(mg_key, 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_kptr = self
            .builder
            .build_extract_value(mg_key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mg_kp_i64 = self
            .builder
            .build_ptr_to_int(mg_kptr, i64, "kp_i64")
            .map_err(llvm_err)?;
        let mg_di64 = self
            .builder
            .build_pointer_cast(mg_data, ptr, "di64")
            .map_err(llvm_err)?;
        let mg_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(mg_i, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mg_blocks[1]); // search

        self.builder.position_at_end(mg_blocks[1]); // search
        let mg_iv = self
            .builder
            .build_load(i64, mg_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mg_iv, mg_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mg_cond, mg_blocks[2], mg_blocks[6]); // body or not_found

        self.builder.position_at_end(mg_blocks[2]); // body
        let mg_off = self
            .builder
            .build_int_mul(mg_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let mg_etp = unsafe {
            self.builder
                .build_gep(i64, mg_di64, &[mg_off], "etp")
                .map_err(llvm_err)
        }?;
        let mg_et = self
            .builder
            .build_load(i64, mg_etp, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, mg_et, mg_ktag, "teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mg_teq, mg_blocks[3], mg_blocks[5]); // ckey or next

        self.builder.position_at_end(mg_blocks[3]); // ckey
        let mg_off1 = self
            .builder
            .build_int_add(mg_off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let mg_epp = unsafe {
            self.builder
                .build_gep(i64, mg_di64, &[mg_off1], "epp")
                .map_err(llvm_err)
        }?;
        let mg_ep = self
            .builder
            .build_load(i64, mg_epp, "ep")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_kpz = self
            .builder
            .build_int_compare(IntPredicate::EQ, mg_kp_i64, zero, "kpz")
            .map_err(llvm_err)?;
        let mg_ek_undef = str_ty.get_undef();
        let mg_ek1 = self
            .builder
            .build_insert_value(mg_ek_undef, mg_et, 0, "ek1")
            .map_err(llvm_err)?;
        let mg_ep_ptr = self
            .builder
            .build_int_to_ptr(mg_ep, ptr, "ep_ptr")
            .map_err(llvm_err)?;
        let mg_ek2 = self
            .builder
            .build_insert_value(mg_ek1, mg_ep_ptr, 1, "ek2")
            .map_err(llvm_err)?;
        let seq_fn2 = self.module.get_function("action_string_eq").unwrap();
        let mg_seq = self
            .builder
            .build_call(
                seq_fn2,
                &[mg_ek2.as_basic_value_enum().into(), mg_key.into()],
                "seq",
            )
            .map_err(llvm_err)?;
        let mg_seq_r = mg_seq.try_as_basic_value().unwrap_basic().into_int_value();
        let mg_feq = self
            .builder
            .build_select(mg_kpz, mg_teq, mg_seq_r, "feq")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            mg_feq.into_int_value(),
            mg_blocks[4],
            mg_blocks[5],
        ); // found or next

        self.builder.position_at_end(mg_blocks[4]); // found
        let mg_off2 = self
            .builder
            .build_int_add(mg_off, i64.const_int(2, false), "off2")
            .map_err(llvm_err)?;
        let mg_vtp = unsafe {
            self.builder
                .build_gep(i64, mg_di64, &[mg_off2], "vtp")
                .map_err(llvm_err)
        }?;
        let mg_vt = self
            .builder
            .build_load(i64, mg_vtp, "vt")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_off3 = self
            .builder
            .build_int_add(mg_off, i64.const_int(3, false), "off3")
            .map_err(llvm_err)?;
        let mg_vpp = unsafe {
            self.builder
                .build_gep(i64, mg_di64, &[mg_off3], "vpp")
                .map_err(llvm_err)
        }?;
        let mg_vp = self
            .builder
            .build_load(i64, mg_vpp, "vp")
            .map_err(llvm_err)?
            .into_int_value();
        let mg_ur = str_ty.get_undef();
        let mg_r1 = self
            .builder
            .build_insert_value(mg_ur, mg_vt, 0, "r1")
            .map_err(llvm_err)?;
        let mg_vp_ptr = self
            .builder
            .build_int_to_ptr(mg_vp, ptr, "vp_ptr")
            .map_err(llvm_err)?;
        let mg_r2 = self
            .builder
            .build_insert_value(mg_r1, mg_vp_ptr, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mg_r2));

        self.builder.position_at_end(mg_blocks[5]); // next
        let mg_niv = self
            .builder
            .build_int_add(mg_iv, i64.const_int(1, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(mg_i, mg_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mg_blocks[1]);

        self.builder.position_at_end(mg_blocks[6]); // not_found
        let mg_ur2 = str_ty.get_undef();
        let mg_nf1 = self
            .builder
            .build_insert_value(mg_ur2, zero, 0, "nf1")
            .map_err(llvm_err)?;
        let mg_nf2 = self
            .builder
            .build_insert_value(mg_nf1, ptr.const_zero(), 1, "nf2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mg_nf2));

        // action_map_contains({ptr,i64,i64}, {i64,ptr}) -> i1
        let mc_fn = self.module.add_function(
            "action_map_contains",
            b1.fn_type(&[list_ty.into(), str_ty.into()], false),
            None,
        );
        let mc_blocks: Vec<_> = (0..7)
            .map(|i| self.context.append_basic_block(mc_fn, &format!("b{}", i)))
            .collect();
        self.builder.position_at_end(mc_blocks[0]); // entry
        let mc_map = mc_fn.get_first_param().unwrap().into_struct_value();
        let mc_key = mc_fn.get_nth_param(1).unwrap().into_struct_value();
        let mc_data = self
            .builder
            .build_extract_value(mc_map, 0, "d")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mc_len = self
            .builder
            .build_extract_value(mc_map, 1, "l")
            .map_err(llvm_err)?
            .into_int_value();
        let mc_ktag = self
            .builder
            .build_extract_value(mc_key, 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let mc_kptr = self
            .builder
            .build_extract_value(mc_key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mc_kp_i64 = self
            .builder
            .build_ptr_to_int(mc_kptr, i64, "kp_i64")
            .map_err(llvm_err)?;
        let mc_di64 = self
            .builder
            .build_pointer_cast(mc_data, ptr, "di64")
            .map_err(llvm_err)?;
        let mc_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(mc_i, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mc_blocks[1]); // search

        self.builder.position_at_end(mc_blocks[1]); // search
        let mc_iv = self
            .builder
            .build_load(i64, mc_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mc_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mc_iv, mc_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mc_cond, mc_blocks[2], mc_blocks[6]); // body or not_found

        self.builder.position_at_end(mc_blocks[2]); // body
        let mc_off = self
            .builder
            .build_int_mul(mc_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let mc_etp = unsafe {
            self.builder
                .build_gep(i64, mc_di64, &[mc_off], "etp")
                .map_err(llvm_err)
        }?;
        let mc_et = self
            .builder
            .build_load(i64, mc_etp, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let mc_teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, mc_et, mc_ktag, "teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mc_teq, mc_blocks[3], mc_blocks[5]); // ckey or next

        self.builder.position_at_end(mc_blocks[3]); // ckey
        let mc_off1 = self
            .builder
            .build_int_add(mc_off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let mc_epp = unsafe {
            self.builder
                .build_gep(i64, mc_di64, &[mc_off1], "epp")
                .map_err(llvm_err)
        }?;
        let mc_ep = self
            .builder
            .build_load(i64, mc_epp, "ep")
            .map_err(llvm_err)?
            .into_int_value();
        let mc_kpz = self
            .builder
            .build_int_compare(IntPredicate::EQ, mc_kp_i64, zero, "kpz")
            .map_err(llvm_err)?;
        let mc_ek_undef = str_ty.get_undef();
        let mc_ek1 = self
            .builder
            .build_insert_value(mc_ek_undef, mc_et, 0, "ek1")
            .map_err(llvm_err)?;
        let mc_ep_ptr = self
            .builder
            .build_int_to_ptr(mc_ep, ptr, "ep_ptr")
            .map_err(llvm_err)?;
        let mc_ek2 = self
            .builder
            .build_insert_value(mc_ek1, mc_ep_ptr, 1, "ek2")
            .map_err(llvm_err)?;
        let seq_fn3 = self.module.get_function("action_string_eq").unwrap();
        let mc_seq = self
            .builder
            .build_call(
                seq_fn3,
                &[mc_ek2.as_basic_value_enum().into(), mc_key.into()],
                "seq",
            )
            .map_err(llvm_err)?;
        let mc_seq_r = mc_seq.try_as_basic_value().unwrap_basic().into_int_value();
        let mc_feq = self
            .builder
            .build_select(mc_kpz, mc_teq, mc_seq_r, "feq")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            mc_feq.into_int_value(),
            mc_blocks[4],
            mc_blocks[5],
        ); // found or next

        self.builder.position_at_end(mc_blocks[4]); // found
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));

        self.builder.position_at_end(mc_blocks[5]); // next
        let mc_niv = self
            .builder
            .build_int_add(mc_iv, i64.const_int(1, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(mc_i, mc_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mc_blocks[1]);

        self.builder.position_at_end(mc_blocks[6]); // not_found
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        // ---- action_map_remove({ptr,i64,i64}, {i64,ptr}) -> {ptr,i64,i64} ----
        let mr_fn = self.module.add_function(
            "action_map_remove",
            list_ty.fn_type(&[list_ty.into(), str_ty.into()], false),
            None,
        );
        let mr_entry = self.context.append_basic_block(mr_fn, "entry");
        let mr_cow = self.context.append_basic_block(mr_fn, "cow_clone");
        let mr_merge = self.context.append_basic_block(mr_fn, "merge");
        let mr_blocks: Vec<_> = (0..6)
            .map(|i| self.context.append_basic_block(mr_fn, &format!("b{}", i)))
            .collect();
        // mr_blocks: 0=search, 1=body, 2=ckey, 3=remove, 4=shift, 5=next
        let mr_done = self.context.append_basic_block(mr_fn, "done");

        self.builder.position_at_end(mr_entry);
        let mr_map = mr_fn.get_first_param().unwrap().into_struct_value();
        let mr_key = mr_fn.get_nth_param(1).unwrap().into_struct_value();
        let mr_data = self
            .builder
            .build_extract_value(mr_map, 0, "d")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mr_len = self
            .builder
            .build_extract_value(mr_map, 1, "l")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_cap = self
            .builder
            .build_extract_value(mr_map, 2, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_ktag = self
            .builder
            .build_extract_value(mr_key, 0, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_kptr = self
            .builder
            .build_extract_value(mr_key, 1, "kp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mr_kp_i64 = self
            .builder
            .build_ptr_to_int(mr_kptr, i64, "kp_i64")
            .map_err(llvm_err)?;
        // CoW check: if refcount > 1, clone the data buffer
        let mr_data_int = self
            .builder
            .build_ptr_to_int(mr_data, i64, "mr_data_int")
            .map_err(llvm_err)?;
        let mr_rc_addr = self
            .builder
            .build_int_sub(mr_data_int, i64.const_int(8, false), "mr_rc_addr")
            .map_err(llvm_err)?;
        let mr_rc_ptr = self
            .builder
            .build_int_to_ptr(mr_rc_addr, ptr, "mr_rc_ptr")
            .map_err(llvm_err)?;
        let mr_rc_val = self
            .builder
            .build_load(self.i64_ty(), mr_rc_ptr, "mr_rc_val")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_need_cow = self
            .builder
            .build_int_compare(
                IntPredicate::SGT,
                mr_rc_val,
                i64.const_int(1, false),
                "mr_need_cow",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mr_need_cow, mr_cow, mr_merge);

        // CoW clone: clone cap*32 bytes
        self.builder.position_at_end(mr_cow);
        let mr_cow_size = self
            .builder
            .build_int_mul(mr_cap, i64.const_int(32, false), "mr_cow_size")
            .map_err(llvm_err)?;
        let mr_new_data = self
            .builder
            .build_call(malloc_rc_fn, &[mr_cow_size.into()], "mr_new_data")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let mr_memcpy_fn = self.module.get_function("memcpy").unwrap();
        let _ = self
            .builder
            .build_call(
                mr_memcpy_fn,
                &[mr_new_data.into(), mr_data.into(), mr_cow_size.into()],
                "",
            )
            .map_err(llvm_err)?;
        let mr_cow_old_rc = self
            .builder
            .build_load(self.i64_ty(), mr_rc_ptr, "mr_cow_old_rc")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_cow_new_rc = self
            .builder
            .build_int_sub(mr_cow_old_rc, i64.const_int(1, false), "mr_cow_new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(mr_rc_ptr, mr_cow_new_rc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_merge);

        // Merge block: phi for data, then setup search state
        self.builder.position_at_end(mr_merge);
        let mr_data_phi = self
            .builder
            .build_phi(ptr, "mr_data_phi")
            .map_err(llvm_err)?;
        mr_data_phi.add_incoming(&[(&mr_data, mr_entry), (&mr_new_data, mr_cow)]);
        let mr_data = mr_data_phi.as_basic_value().into_pointer_value();
        let mr_di64 = self
            .builder
            .build_pointer_cast(mr_data, ptr, "di64")
            .map_err(llvm_err)?;
        let mr_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder.build_store(mr_i, zero).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_blocks[0]); // search

        self.builder.position_at_end(mr_blocks[0]); // search
        let mr_iv = self
            .builder
            .build_load(i64, mr_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mr_iv, mr_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mr_cond, mr_blocks[1], mr_done); // body or not_found

        self.builder.position_at_end(mr_blocks[1]); // body
        let mr_off = self
            .builder
            .build_int_mul(mr_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let mr_etp = unsafe {
            self.builder
                .build_gep(i64, mr_di64, &[mr_off], "etp")
                .map_err(llvm_err)
        }?;
        let mr_et = self
            .builder
            .build_load(i64, mr_etp, "et")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, mr_et, mr_ktag, "teq")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mr_teq, mr_blocks[2], mr_blocks[5]); // ckey or next

        self.builder.position_at_end(mr_blocks[2]); // ckey
        let mr_off1 = self
            .builder
            .build_int_add(mr_off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let mr_epp = unsafe {
            self.builder
                .build_gep(i64, mr_di64, &[mr_off1], "epp")
                .map_err(llvm_err)
        }?;
        let mr_ep = self
            .builder
            .build_load(i64, mr_epp, "ep")
            .map_err(llvm_err)?
            .into_int_value();
        let mr_kpz = self
            .builder
            .build_int_compare(IntPredicate::EQ, mr_kp_i64, zero, "kpz")
            .map_err(llvm_err)?;
        let mr_ek_undef = str_ty.get_undef();
        let mr_ek1 = self
            .builder
            .build_insert_value(mr_ek_undef, mr_et, 0, "ek1")
            .map_err(llvm_err)?;
        let mr_ep_ptr = self
            .builder
            .build_int_to_ptr(mr_ep, ptr, "ep_ptr")
            .map_err(llvm_err)?;
        let mr_ek2 = self
            .builder
            .build_insert_value(mr_ek1, mr_ep_ptr, 1, "ek2")
            .map_err(llvm_err)?;
        let seq_fn4 = self.module.get_function("action_string_eq").unwrap();
        let mr_seq = self
            .builder
            .build_call(
                seq_fn4,
                &[mr_ek2.as_basic_value_enum().into(), mr_key.into()],
                "seq",
            )
            .map_err(llvm_err)?;
        let mr_seq_r = mr_seq.try_as_basic_value().unwrap_basic().into_int_value();
        let mr_feq = self
            .builder
            .build_select(mr_kpz, mr_teq, mr_seq_r, "feq")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(
            mr_feq.into_int_value(),
            mr_blocks[3],
            mr_blocks[5],
        ); // remove or next

        // Remove: shift remaining entries down by one slot (4 i64s = 32 bytes)
        self.builder.position_at_end(mr_blocks[3]); // remove
        let mr_len_dec = self
            .builder
            .build_int_sub(mr_len, i64.const_int(1, false), "len_dec")
            .map_err(llvm_err)?;
        // Number of entries after this one: len - iv - 1
        let mr_iv_p1 = self
            .builder
            .build_int_add(mr_iv, i64.const_int(1, false), "iv_p1")
            .map_err(llvm_err)?;
        let mr_remaining = self
            .builder
            .build_int_sub(mr_len, mr_iv_p1, "remaining")
            .map_err(llvm_err)?;
        let mr_has_remaining = self
            .builder
            .build_int_compare(IntPredicate::SGT, mr_remaining, zero, "has_rem")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mr_has_remaining, mr_blocks[4], mr_done); // shift or done

        self.builder.position_at_end(mr_blocks[4]); // shift
                                                    // Source: data + (iv + 1) * 32 bytes → dest: data + iv * 32 bytes
        let mr_src_off = self
            .builder
            .build_int_mul(mr_iv_p1, i64.const_int(32, false), "src_off")
            .map_err(llvm_err)?;
        let mr_dst_off = self
            .builder
            .build_int_mul(mr_iv, i64.const_int(32, false), "dst_off")
            .map_err(llvm_err)?;
        let mr_src = unsafe {
            self.builder
                .build_gep(i8, mr_data, &[mr_src_off], "src")
                .map_err(llvm_err)
        }?;
        let mr_dst = unsafe {
            self.builder
                .build_gep(i8, mr_data, &[mr_dst_off], "dst")
                .map_err(llvm_err)
        }?;
        let mr_rem_bytes = self
            .builder
            .build_int_mul(mr_remaining, i64.const_int(32, false), "rem_bytes")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[mr_dst.into(), mr_src.into(), mr_rem_bytes.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_done); // done

        self.builder.position_at_end(mr_blocks[5]); // next
        let mr_niv = self
            .builder
            .build_int_add(mr_iv, i64.const_int(1, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(mr_i, mr_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mr_blocks[0]);

        self.builder.position_at_end(mr_done); // done (not found, or already removed)
                                               // Return {data, len_dec or len, cap}
        let mr_ret_len = self.builder.build_phi(i64, "ret_len").map_err(llvm_err)?;
        mr_ret_len.add_incoming(&[
            (&mr_len, mr_blocks[0]),
            (&mr_len_dec, mr_blocks[3]),
            (&mr_len_dec, mr_blocks[4]),
        ]);
        let mr_ret_len_val = mr_ret_len.as_basic_value().into_int_value();
        let mr_ur = list_ty.get_undef();
        let mr_r1 = self
            .builder
            .build_insert_value(mr_ur, mr_data, 0, "r1")
            .map_err(llvm_err)?;
        let mr_r2 = self
            .builder
            .build_insert_value(mr_r1, mr_ret_len_val, 1, "r2")
            .map_err(llvm_err)?;
        let mr_r3 = self
            .builder
            .build_insert_value(mr_r2, mr_cap, 2, "r3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mr_r3));

        // ---- action_string_starts_with({i64, ptr}, {i64, ptr}) -> i1 ----
        let sw_fn = self.module.add_function(
            "action_string_starts_with",
            self.bool_ty()
                .fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let sw_entry = self.context.append_basic_block(sw_fn, "entry");
        self.builder.position_at_end(sw_entry);
        let sw_s = sw_fn.get_first_param().unwrap().into_struct_value();
        let sw_pre = sw_fn.get_nth_param(1).unwrap().into_struct_value();
        let sw_slen = self
            .builder
            .build_extract_value(sw_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sw_plen = self
            .builder
            .build_extract_value(sw_pre, 0, "plen")
            .map_err(llvm_err)?
            .into_int_value();
        let sw_sdata = self
            .builder
            .build_extract_value(sw_s, 1, "sdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sw_pdata = self
            .builder
            .build_extract_value(sw_pre, 1, "pdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sw_len_ok = self
            .builder
            .build_int_compare(IntPredicate::UGE, sw_slen, sw_plen, "len_ok")
            .map_err(llvm_err)?;
        let sw_check = self.context.append_basic_block(sw_fn, "check");
        let sw_cmp = self.context.append_basic_block(sw_fn, "cmp");
        let sw_false = self.context.append_basic_block(sw_fn, "false");
        let sw_done = self.context.append_basic_block(sw_fn, "done");
        let _ = self
            .builder
            .build_conditional_branch(sw_len_ok, sw_check, sw_false);
        // check: empty prefix → true, else → cmp
        self.builder.position_at_end(sw_check);
        let sw_pz = self
            .builder
            .build_int_compare(IntPredicate::EQ, sw_plen, i64.const_int(0, false), "pz")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sw_pz, sw_done, sw_cmp);
        // cmp: memcmp
        self.builder.position_at_end(sw_cmp);
        let sw_mc = self
            .builder
            .build_call(
                memcmp_fn,
                &[sw_sdata.into(), sw_pdata.into(), sw_plen.into()],
                "mc",
            )
            .map_err(llvm_err)?;
        let sw_mcr = sw_mc.try_as_basic_value().unwrap_basic().into_int_value();
        let sw_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, sw_mcr, i32.const_int(0, false), "eq")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sw_done);
        // false
        self.builder.position_at_end(sw_false);
        let _ = self.builder.build_unconditional_branch(sw_done);
        // done: phi [pz from check, eq from cmp, false from false]
        self.builder.position_at_end(sw_done);
        let sw_phi = self
            .builder
            .build_phi(self.bool_ty(), "sw_result")
            .map_err(llvm_err)?;
        sw_phi.add_incoming(&[
            (&sw_pz, sw_check),
            (&sw_eq, sw_cmp),
            (&self.bool_ty().const_int(0, false), sw_false),
        ]);
        let _ = self.builder.build_return(Some(&sw_phi.as_basic_value()));

        // ---- action_string_ends_with({i64, ptr}, {i64, ptr}) -> i1 ----
        let ew_fn = self.module.add_function(
            "action_string_ends_with",
            self.bool_ty()
                .fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let ew_entry = self.context.append_basic_block(ew_fn, "entry");
        self.builder.position_at_end(ew_entry);
        let ew_s = ew_fn.get_first_param().unwrap().into_struct_value();
        let ew_suf = ew_fn.get_nth_param(1).unwrap().into_struct_value();
        let ew_slen = self
            .builder
            .build_extract_value(ew_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let ew_suflen = self
            .builder
            .build_extract_value(ew_suf, 0, "suflen")
            .map_err(llvm_err)?
            .into_int_value();
        let ew_sdata = self
            .builder
            .build_extract_value(ew_s, 1, "sdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ew_sufdata = self
            .builder
            .build_extract_value(ew_suf, 1, "sufdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ew_len_ok = self
            .builder
            .build_int_compare(IntPredicate::UGE, ew_slen, ew_suflen, "len_ok")
            .map_err(llvm_err)?;
        let ew_check = self.context.append_basic_block(ew_fn, "check");
        let ew_cmp = self.context.append_basic_block(ew_fn, "cmp");
        let ew_false = self.context.append_basic_block(ew_fn, "false");
        let ew_done = self.context.append_basic_block(ew_fn, "done");
        let _ = self
            .builder
            .build_conditional_branch(ew_len_ok, ew_check, ew_false);
        // check: empty suffix → true, else → cmp
        self.builder.position_at_end(ew_check);
        let ew_sufz = self
            .builder
            .build_int_compare(IntPredicate::EQ, ew_suflen, i64.const_int(0, false), "sufz")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ew_sufz, ew_done, ew_cmp);
        // cmp: memcmp from offset len-suffixlen
        self.builder.position_at_end(ew_cmp);
        let ew_off = self
            .builder
            .build_int_sub(ew_slen, ew_suflen, "off")
            .map_err(llvm_err)?;
        let ew_sp = unsafe {
            self.builder
                .build_gep(i8, ew_sdata, &[ew_off], "sp")
                .map_err(llvm_err)
        }?;
        let ew_mc = self
            .builder
            .build_call(
                memcmp_fn,
                &[ew_sp.into(), ew_sufdata.into(), ew_suflen.into()],
                "mc",
            )
            .map_err(llvm_err)?;
        let ew_mcr = ew_mc.try_as_basic_value().unwrap_basic().into_int_value();
        let ew_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, ew_mcr, i32.const_int(0, false), "eq")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ew_done);
        // false
        self.builder.position_at_end(ew_false);
        let _ = self.builder.build_unconditional_branch(ew_done);
        // done: phi [sufz from check, eq from cmp, false from false]
        self.builder.position_at_end(ew_done);
        let ew_phi = self
            .builder
            .build_phi(self.bool_ty(), "ew_result")
            .map_err(llvm_err)?;
        ew_phi.add_incoming(&[
            (&ew_sufz, ew_check),
            (&ew_eq, ew_cmp),
            (&self.bool_ty().const_int(0, false), ew_false),
        ]);
        let _ = self.builder.build_return(Some(&ew_phi.as_basic_value()));

        // ---- action_string_substring({i64, ptr}, i64 start, i64 len) -> {i64, ptr} ----
        let sub_fn = self.module.add_function(
            "action_string_substring",
            str_ty.fn_type(&[str_ty.into(), i64.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(sub_fn, "entry");
        self.builder.position_at_end(entry);
        let sub_s = sub_fn.get_first_param().unwrap().into_struct_value();
        let sub_start = sub_fn.get_nth_param(1).unwrap().into_int_value();
        let sub_len = sub_fn.get_nth_param(2).unwrap().into_int_value();
        let sub_slen = self
            .builder
            .build_extract_value(sub_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sub_sdata = self
            .builder
            .build_extract_value(sub_s, 1, "sdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Clamp: if start >= slen, return empty string
        let sub_start_ok = self
            .builder
            .build_int_compare(IntPredicate::ULT, sub_start, sub_slen, "start_ok")
            .map_err(llvm_err)?;
        let sub_end = self
            .builder
            .build_int_add(sub_start, sub_len, "end")
            .map_err(llvm_err)?;
        let sub_end_ok = self
            .builder
            .build_int_compare(IntPredicate::ULE, sub_end, sub_slen, "end_ok")
            .map_err(llvm_err)?;
        let sub_clamped_end = self
            .builder
            .build_select(sub_end_ok, sub_end, sub_slen, "clamped_end")
            .map_err(llvm_err)?
            .into_int_value();
        let sub_actual_len = self
            .builder
            .build_int_sub(sub_clamped_end, sub_start, "actual_len")
            .map_err(llvm_err)?;
        let sub_clamped_start = self
            .builder
            .build_select(sub_start_ok, sub_start, sub_slen, "clamped_start")
            .map_err(llvm_err)?
            .into_int_value();
        let _sub_zero_len = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sub_actual_len,
                i64.const_int(0, false),
                "zero_len",
            )
            .map_err(llvm_err)?;
        // Allocate and copy
        let sub_alc = self
            .builder
            .build_int_add(sub_actual_len, i64.const_int(1, false), "alc")
            .map_err(llvm_err)?;
        let sub_buf = self
            .builder
            .build_call(malloc_rc_fn, &[sub_alc.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sub_src = unsafe {
            self.builder
                .build_gep(i8, sub_sdata, &[sub_clamped_start], "src")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sub_buf.into(), sub_src.into(), sub_actual_len.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sub_null = unsafe {
            self.builder
                .build_gep(i8, sub_buf, &[sub_actual_len], "null")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(sub_null, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let sub_undef = str_ty.get_undef();
        let sub_r1 = self
            .builder
            .build_insert_value(sub_undef, sub_actual_len, 0, "r1")
            .map_err(llvm_err)?;
        let sub_r2 = self
            .builder
            .build_insert_value(sub_r1, sub_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sub_r2));

        // ---- action_parse_int({i64, ptr}) -> {i64, i1} (value, success) ----
        let pi_ret_ty = self
            .context
            .struct_type(&[i64.into(), self.bool_ty().into()], false);
        let pi_fn = self.module.add_function(
            "action_parse_int",
            pi_ret_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(pi_fn, "entry");
        self.builder.position_at_end(entry);
        let pi_s = pi_fn.get_first_param().unwrap().into_struct_value();
        let pi_len = self
            .builder
            .build_extract_value(pi_s, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_data = self
            .builder
            .build_extract_value(pi_s, 1, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Initialize result=0, sign=1, i=0, valid=0
        let pi_result = self.builder.build_alloca(i64, "result").map_err(llvm_err)?;
        let pi_sign = self.builder.build_alloca(i64, "sign").map_err(llvm_err)?;
        let pi_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        let pi_valid = self
            .builder
            .build_alloca(self.bool_ty(), "valid")
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_result, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_sign, i64.const_int(1, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_valid, self.bool_ty().const_zero())
            .map_err(llvm_err)?;
        // Check for leading '-'
        let pi_has_chars = self
            .builder
            .build_int_compare(
                IntPredicate::UGT,
                pi_len,
                i64.const_int(0, false),
                "has_chars",
            )
            .map_err(llvm_err)?;
        let pi_ck = self.context.append_basic_block(pi_fn, "check_sign");
        let pi_setup = self.context.append_basic_block(pi_fn, "setup");
        let pi_loop_hdr = self.context.append_basic_block(pi_fn, "loop_hdr");
        let pi_loop_body = self.context.append_basic_block(pi_fn, "loop_body");
        let pi_done = self.context.append_basic_block(pi_fn, "done");
        let _ = self
            .builder
            .build_conditional_branch(pi_has_chars, pi_ck, pi_done);

        // check_sign: check first char for '-', then branch to setup
        self.builder.position_at_end(pi_ck);
        let pi_first = self
            .builder
            .build_load(i8, pi_data, "first")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_is_minus = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                pi_first,
                i8.const_int(b'-' as u64, false),
                "is_minus",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pi_setup);

        // setup: set sign and start index based on whether first char is '-'
        self.builder.position_at_end(pi_setup);
        let pi_sign_val = self
            .builder
            .build_select(
                pi_is_minus,
                i64.const_int(0xffffffffffffffffu64, true),
                i64.const_int(1, false),
                "sign_val",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let pi_start_i = self
            .builder
            .build_select(
                pi_is_minus,
                i64.const_int(1, false),
                i64.const_int(0, false),
                "start_i",
            )
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(pi_sign, pi_sign_val)
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_i, pi_start_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pi_loop_hdr);

        self.builder.position_at_end(pi_loop_hdr);
        let pi_iv = self
            .builder
            .build_load(i64, pi_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_not_done = self
            .builder
            .build_int_compare(IntPredicate::ULT, pi_iv, pi_len, "not_done")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(pi_not_done, pi_loop_body, pi_done);

        self.builder.position_at_end(pi_loop_body);
        let pi_chp = unsafe {
            self.builder
                .build_gep(i8, pi_data, &[pi_iv], "chp")
                .map_err(llvm_err)
        }?;
        let pi_ch = self
            .builder
            .build_load(i8, pi_chp, "ch")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_is_digit = self
            .builder
            .build_int_compare(
                IntPredicate::UGE,
                pi_ch,
                i8.const_int(b'0' as u64, false),
                "ge0",
            )
            .map_err(llvm_err)?;
        let pi_is_digit2 = self
            .builder
            .build_int_compare(
                IntPredicate::ULE,
                pi_ch,
                i8.const_int(b'9' as u64, false),
                "le9",
            )
            .map_err(llvm_err)?;
        let pi_is_d = self
            .builder
            .build_and(pi_is_digit, pi_is_digit2, "is_digit")
            .map_err(llvm_err)?;
        let pi_body_ck = self.context.append_basic_block(pi_fn, "body_ck");
        let pi_body_next = self.context.append_basic_block(pi_fn, "body_next");
        let _ = self
            .builder
            .build_conditional_branch(pi_is_d, pi_body_ck, pi_done);

        self.builder.position_at_end(pi_body_ck);
        let pi_cur = self
            .builder
            .build_load(i64, pi_result, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_mul = self
            .builder
            .build_int_mul(pi_cur, i64.const_int(10, false), "mul")
            .map_err(llvm_err)?;
        let pi_dval = self
            .builder
            .build_int_sub(pi_ch, i8.const_int(b'0' as u64, false), "dval")
            .map_err(llvm_err)?;
        let pi_dval64 = self
            .builder
            .build_int_z_extend(pi_dval, i64, "dval64")
            .map_err(llvm_err)?;
        let pi_add = self
            .builder
            .build_int_add(pi_mul, pi_dval64, "add")
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_result, pi_add)
            .map_err(llvm_err)?;
        self.builder
            .build_store(pi_valid, self.bool_ty().const_int(1, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pi_body_next);

        self.builder.position_at_end(pi_body_next);
        let pi_niv = self
            .builder
            .build_int_add(pi_iv, i64.const_int(1, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(pi_i, pi_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(pi_loop_hdr);

        self.builder.position_at_end(pi_done);
        let pi_final = self
            .builder
            .build_load(i64, pi_result, "final")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_final_sign = self
            .builder
            .build_load(i64, pi_sign, "final_sign")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_mul_sign = self
            .builder
            .build_int_mul(pi_final, pi_final_sign, "mul_sign")
            .map_err(llvm_err)?;
        let pi_valid_val = self
            .builder
            .build_load(self.bool_ty(), pi_valid, "valid_val")
            .map_err(llvm_err)?
            .into_int_value();
        let pi_ret_undef = pi_ret_ty.get_undef();
        let pi_ret1 = self
            .builder
            .build_insert_value(pi_ret_undef, pi_mul_sign, 0, "ret_val")
            .map_err(llvm_err)?;
        let pi_ret2 = self
            .builder
            .build_insert_value(pi_ret1, pi_valid_val, 1, "ret_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&pi_ret2));

        // ---- action_read_file({i64, ptr}) -> {i64, ptr} ----
        let rf_fn = self.module.add_function(
            "action_read_file",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(rf_fn, "entry");
        self.builder.position_at_end(entry);
        let rf_path_s = rf_fn.get_first_param().unwrap().into_struct_value();
        let rf_path_data = self
            .builder
            .build_extract_value(rf_path_s, 1, "path_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rf_mode = make_global_str(".rf_mode", b"rb\0");
        let rf_file = self
            .builder
            .build_call(fopen_fn, &[rf_path_data.into(), rf_mode.into()], "file")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let rf_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_ptr_to_int(rf_file, i64, "rf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(0, false),
                "rf_null",
            )
            .map_err(llvm_err)?;
        let rf_open_ok = self.context.append_basic_block(rf_fn, "open_ok");
        let rf_fail = self.context.append_basic_block(rf_fn, "fail");
        let _ = self
            .builder
            .build_conditional_branch(rf_null, rf_fail, rf_open_ok);

        // Fail: return empty string
        self.builder.position_at_end(rf_fail);
        let rf_e_undef = str_ty.get_undef();
        let rf_e_r1 = self
            .builder
            .build_insert_value(rf_e_undef, i64.const_int(0, false), 0, "r1")
            .map_err(llvm_err)?;
        let rf_e_r2 = self
            .builder
            .build_insert_value(
                rf_e_r1,
                self.builder
                    .build_int_to_ptr(i64.const_int(0, false), ptr, "nullp")
                    .map_err(llvm_err)?,
                1,
                "r2",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&rf_e_r2));

        // Open ok: seek to end, get size, read, return
        self.builder.position_at_end(rf_open_ok);
        // fseek(file, 0, 2) from end
        let _ = self
            .builder
            .build_call(
                fseek_fn,
                &[
                    rf_file.into(),
                    i64.const_int(0, false).into(),
                    i32.const_int(2, false).into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let rf_size = self
            .builder
            .build_call(ftell_fn, &[rf_file.into()], "size")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        // Rewind
        let _ = self
            .builder
            .build_call(
                fseek_fn,
                &[
                    rf_file.into(),
                    i64.const_int(0, false).into(),
                    i32.const_int(0, false).into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        // Allocate size+1, read, null-terminate
        let rf_alc = self
            .builder
            .build_int_add(rf_size, i64.const_int(1, false), "alc")
            .map_err(llvm_err)?;
        let rf_buf = self
            .builder
            .build_call(malloc_fn, &[rf_alc.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                fread_fn,
                &[
                    rf_buf.into(),
                    i64.const_int(1, false).into(),
                    rf_size.into(),
                    rf_file.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let rf_null_gep = unsafe {
            self.builder
                .build_gep(i8, rf_buf, &[rf_size], "null_gep")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(rf_null_gep, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(fclose_fn, &[rf_file.into()], "")
            .map_err(llvm_err)?;
        let rf_und = str_ty.get_undef();
        let rf_r1 = self
            .builder
            .build_insert_value(rf_und, rf_size, 0, "r1")
            .map_err(llvm_err)?;
        let rf_r2 = self
            .builder
            .build_insert_value(rf_r1, rf_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&rf_r2));

        // ---- action_write_file({i64, ptr}, {i64, ptr}) -> i1 ----
        let wf_fn = self.module.add_function(
            "action_write_file",
            self.bool_ty()
                .fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(wf_fn, "entry");
        self.builder.position_at_end(entry);
        let wf_path = wf_fn.get_first_param().unwrap().into_struct_value();
        let wf_content = wf_fn.get_nth_param(1).unwrap().into_struct_value();
        let wf_pdata = self
            .builder
            .build_extract_value(wf_path, 1, "pdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let wf_clen = self
            .builder
            .build_extract_value(wf_content, 0, "clen")
            .map_err(llvm_err)?
            .into_int_value();
        let wf_cdata = self
            .builder
            .build_extract_value(wf_content, 1, "cdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let wf_wmode = make_global_str(".wf_mode", b"wb\0");
        let wf_file = self
            .builder
            .build_call(fopen_fn, &[wf_pdata.into(), wf_wmode.into()], "file")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let wf_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_ptr_to_int(wf_file, i64, "wf_i64")
                    .map_err(llvm_err)?,
                i64.const_int(0, false),
                "wf_null",
            )
            .map_err(llvm_err)?;
        let wf_open_ok = self.context.append_basic_block(wf_fn, "open_ok");
        let wf_fail = self.context.append_basic_block(wf_fn, "wf_fail");
        let wf_done = self.context.append_basic_block(wf_fn, "wf_done");
        let _ = self
            .builder
            .build_conditional_branch(wf_null, wf_fail, wf_open_ok);
        self.builder.position_at_end(wf_fail);
        let _ = self.builder.build_unconditional_branch(wf_done);
        self.builder.position_at_end(wf_open_ok);
        let _ = self
            .builder
            .build_call(
                fwrite_fn,
                &[
                    wf_cdata.into(),
                    i64.const_int(1, false).into(),
                    wf_clen.into(),
                    wf_file.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(fclose_fn, &[wf_file.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(wf_done);
        self.builder.position_at_end(wf_done);
        let wf_phi = self
            .builder
            .build_phi(self.bool_ty(), "wf_ok")
            .map_err(llvm_err)?;
        wf_phi.add_incoming(&[
            (&self.bool_ty().const_int(0, false), wf_fail),
            (&self.bool_ty().const_int(1, false), wf_open_ok),
        ]);
        let _ = self.builder.build_return(Some(&wf_phi.as_basic_value()));

        // ---- action_file_exists({i64, ptr}) -> i1 ----
        let fe_fn = self.module.add_function(
            "action_file_exists",
            self.bool_ty().fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fe_fn, "entry");
        self.builder.position_at_end(entry);
        let fe_path = fe_fn.get_first_param().unwrap().into_struct_value();
        let fe_pdata = self
            .builder
            .build_extract_value(fe_path, 1, "pdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fe_mode = make_global_str(".fe_mode", b"r\0");
        let fe_file = self
            .builder
            .build_call(fopen_fn, &[fe_pdata.into(), fe_mode.into()], "file")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fe_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_ptr_to_int(fe_file, i64, "fe_i64")
                    .map_err(llvm_err)?,
                i64.const_int(0, false),
                "fe_null",
            )
            .map_err(llvm_err)?;
        let fe_exists_bb = self.context.append_basic_block(fe_fn, "exists_ok");
        let fe_not_bb = self.context.append_basic_block(fe_fn, "fe_done");
        let _ = self
            .builder
            .build_conditional_branch(fe_null, fe_not_bb, fe_exists_bb);
        self.builder.position_at_end(fe_exists_bb);
        let _ = self
            .builder
            .build_call(fclose_fn, &[fe_file.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fe_not_bb);
        self.builder.position_at_end(fe_not_bb);
        let fe_phi = self
            .builder
            .build_phi(self.bool_ty(), "fe_exists")
            .map_err(llvm_err)?;
        fe_phi.add_incoming(&[
            (&self.bool_ty().const_int(0, false), entry),
            (&self.bool_ty().const_int(1, false), fe_exists_bb),
        ]);
        let _ = self.builder.build_return(Some(&fe_phi.as_basic_value()));

        // ---- action_file_append({i64, ptr}, {i64, ptr}) -> i1 ----
        let fa_fn = self.module.add_function(
            "action_file_append",
            self.bool_ty()
                .fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fa_fn, "entry");
        self.builder.position_at_end(entry);
        let fa_path = fa_fn.get_first_param().unwrap().into_struct_value();
        let fa_content = fa_fn.get_nth_param(1).unwrap().into_struct_value();
        let fa_pdata = self
            .builder
            .build_extract_value(fa_path, 1, "pdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fa_clen = self
            .builder
            .build_extract_value(fa_content, 0, "clen")
            .map_err(llvm_err)?
            .into_int_value();
        let fa_cdata = self
            .builder
            .build_extract_value(fa_content, 1, "cdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fa_amode = make_global_str(".fa_mode", b"a\0");
        let fa_file = self
            .builder
            .build_call(fopen_fn, &[fa_pdata.into(), fa_amode.into()], "file")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let fa_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_ptr_to_int(fa_file, i64, "fa_i64")
                    .map_err(llvm_err)?,
                i64.const_int(0, false),
                "fa_null",
            )
            .map_err(llvm_err)?;
        let fa_open_ok = self.context.append_basic_block(fa_fn, "open_ok");
        let fa_fail = self.context.append_basic_block(fa_fn, "fa_fail");
        let fa_done = self.context.append_basic_block(fa_fn, "fa_done");
        let _ = self
            .builder
            .build_conditional_branch(fa_null, fa_fail, fa_open_ok);
        self.builder.position_at_end(fa_fail);
        let _ = self.builder.build_unconditional_branch(fa_done);
        self.builder.position_at_end(fa_open_ok);
        let _ = self
            .builder
            .build_call(
                fwrite_fn,
                &[
                    fa_cdata.into(),
                    i64.const_int(1, false).into(),
                    fa_clen.into(),
                    fa_file.into(),
                ],
                "",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(fclose_fn, &[fa_file.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fa_done);
        self.builder.position_at_end(fa_done);
        let fa_phi = self
            .builder
            .build_phi(self.bool_ty(), "fa_ok")
            .map_err(llvm_err)?;
        fa_phi.add_incoming(&[
            (&self.bool_ty().const_int(0, false), fa_fail),
            (&self.bool_ty().const_int(1, false), fa_open_ok),
        ]);
        let _ = self.builder.build_return(Some(&fa_phi.as_basic_value()));

        // ---- action_file_delete({i64, ptr}) -> i1 ----
        let fd_fn = self.module.add_function(
            "action_file_delete",
            self.bool_ty().fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fd_fn, "entry");
        self.builder.position_at_end(entry);
        let fd_path = fd_fn.get_first_param().unwrap().into_struct_value();
        let fd_pdata = self
            .builder
            .build_extract_value(fd_path, 1, "pdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let remove_fn = self.module.get_function("remove").unwrap();
        let fd_ret = self
            .builder
            .build_call(remove_fn, &[fd_pdata.into()], "ret")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fd_ok = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                fd_ret,
                self.i32_ty().const_int(0, false),
                "fd_ok",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fd_ok));

        // ---- Streaming File I/O Runtime Functions ----

        // ---- action_file_open({i64, ptr}, {i64, ptr}) -> ptr (FILE*) ----
        // Opens a file at path with mode. Returns FILE* (null on failure).
        let fo_fn = self.module.add_function(
            "action_file_open",
            ptr.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fo_fn, "entry");
        self.builder.position_at_end(entry);
        let fo_path = fo_fn.get_first_param().unwrap().into_struct_value();
        let fo_mode = fo_fn.get_nth_param(1).unwrap().into_struct_value();
        let fo_pdata = self
            .builder
            .build_extract_value(fo_path, 1, "pdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fo_mdata = self
            .builder
            .build_extract_value(fo_mode, 1, "mdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fo_file = self
            .builder
            .build_call(fopen_fn, &[fo_pdata.into(), fo_mdata.into()], "file")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self.builder.build_return(Some(&fo_file));

        // ---- action_file_close(ptr) -> i32 ----
        // Closes a file handle. Returns 0 on success, EOF on failure.
        let fc_fn =
            self.module
                .add_function("action_file_close", i32.fn_type(&[ptr.into()], false), None);
        let entry = self.context.append_basic_block(fc_fn, "entry");
        self.builder.position_at_end(entry);
        let fc_handle = fc_fn.get_first_param().unwrap().into_pointer_value();
        let fc_ret = self
            .builder
            .build_call(fclose_fn, &[fc_handle.into()], "ret")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&fc_ret));

        // ---- action_file_eof(ptr) -> i1 ----
        // Checks if file handle is at EOF. Uses feof().
        let feof_c_fn = self
            .module
            .add_function("feof", i32.fn_type(&[ptr.into()], false), None);
        let fe_fn = self.module.add_function(
            "action_file_eof",
            self.bool_ty().fn_type(&[ptr.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fe_fn, "entry");
        self.builder.position_at_end(entry);
        let fe_handle = fe_fn.get_first_param().unwrap().into_pointer_value();
        let fe_ret = self
            .builder
            .build_call(feof_c_fn, &[fe_handle.into()], "ret")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fe_ok = self
            .builder
            .build_int_compare(IntPredicate::NE, fe_ret, i32.const_int(0, false), "is_eof")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fe_ok));

        // ---- action_file_read_line(ptr) -> {i64, ptr, i1} (len, data, success) ----
        // Reads one line from file handle. Returns string + success flag (0 on EOF).
        // Uses fgets with a 4096-byte buffer.
        let frl_ret_ty = self
            .context
            .struct_type(&[i64.into(), ptr.into(), self.bool_ty().into()], false);
        let frl_fn = self.module.add_function(
            "action_file_read_line",
            frl_ret_ty.fn_type(&[ptr.into()], false),
            None,
        );
        let fgets_fn = self.module.get_function("fgets").unwrap();
        let entry = self.context.append_basic_block(frl_fn, "entry");
        self.builder.position_at_end(entry);
        let frl_handle = frl_fn.get_first_param().unwrap().into_pointer_value();
        let frl_buf_size = i64.const_int(4096, false);
        let frl_buf = self
            .builder
            .build_call(malloc_fn, &[frl_buf_size.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let frl_ret = self
            .builder
            .build_call(
                fgets_fn,
                &[
                    frl_buf.into(),
                    i32.const_int(4096, false).into(),
                    frl_handle.into(),
                ],
                "",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Check if fgets returned NULL (EOF/error)
        let frl_is_eof = self
            .builder
            .build_int_compare(IntPredicate::EQ, frl_ret, ptr.const_zero(), "is_eof")
            .map_err(llvm_err)?;
        let frl_eof_bb = self.context.append_basic_block(frl_fn, "eof");
        let frl_ok_bb = self.context.append_basic_block(frl_fn, "ok");
        let frl_merge_bb = self.context.append_basic_block(frl_fn, "merge");
        let _ = self
            .builder
            .build_conditional_branch(frl_is_eof, frl_eof_bb, frl_ok_bb);
        // EOF path
        self.builder.position_at_end(frl_eof_bb);
        let frl_e_undef = frl_ret_ty.get_undef();
        let frl_e1 = self
            .builder
            .build_insert_value(frl_e_undef, i64.const_int(0, false), 0, "e_len")
            .map_err(llvm_err)?;
        let frl_e2 = self
            .builder
            .build_insert_value(frl_e1, ptr.const_zero(), 1, "e_ptr")
            .map_err(llvm_err)?;
        let frl_e3 = self
            .builder
            .build_insert_value(frl_e2, self.bool_ty().const_zero(), 2, "e_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(frl_merge_bb);
        // OK path: compute length, strip newline
        self.builder.position_at_end(frl_ok_bb);
        let frl_str_len = self
            .builder
            .build_call(strlen_fn, &[frl_buf.into()], "len")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let frl_last = self
            .builder
            .build_int_sub(frl_str_len, i64.const_int(1, false), "last_idx")
            .map_err(llvm_err)?;
        let frl_last_ptr = unsafe {
            self.builder
                .build_gep(i8, frl_buf, &[frl_last], "last_ptr")
                .map_err(llvm_err)
        }?;
        let frl_last_ch = self
            .builder
            .build_load(i8, frl_last_ptr, "last_ch")
            .map_err(llvm_err)?
            .into_int_value();
        let frl_is_nl = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                frl_last_ch,
                i8.const_int(10, false),
                "is_nl",
            )
            .map_err(llvm_err)?;
        let frl_adj_len = self
            .builder
            .build_select(frl_is_nl, frl_last, frl_str_len, "adj_len")
            .map_err(llvm_err)?;
        let frl_o_undef = frl_ret_ty.get_undef();
        let frl_o1 = self
            .builder
            .build_insert_value(frl_o_undef, frl_adj_len.into_int_value(), 0, "o_len")
            .map_err(llvm_err)?;
        let frl_o2 = self
            .builder
            .build_insert_value(frl_o1, frl_buf, 1, "o_ptr")
            .map_err(llvm_err)?;
        let frl_o3 = self
            .builder
            .build_insert_value(frl_o2, self.bool_ty().const_int(1, false), 2, "o_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(frl_merge_bb);
        // Merge
        self.builder.position_at_end(frl_merge_bb);
        let frl_phi = self
            .builder
            .build_phi(frl_ret_ty, "frl_ret")
            .map_err(llvm_err)?;
        frl_phi.add_incoming(&[(&frl_e3, frl_eof_bb), (&frl_o3, frl_ok_bb)]);
        let _ = self.builder.build_return(Some(&frl_phi.as_basic_value()));

        // ---- action_file_read_bytes(ptr, i64) -> {i64, ptr} (actual_len, data) ----
        // Reads up to size bytes from file handle. Returns 0 length on EOF.
        let frb_ret_ty = self.context.struct_type(&[i64.into(), ptr.into()], false);
        let frb_fn = self.module.add_function(
            "action_file_read_bytes",
            frb_ret_ty.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(frb_fn, "entry");
        self.builder.position_at_end(entry);
        let frb_handle = frb_fn.get_first_param().unwrap().into_pointer_value();
        let frb_size = frb_fn.get_nth_param(1).unwrap().into_int_value();
        let frb_buf = self
            .builder
            .build_call(malloc_fn, &[frb_size.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let frb_read = self
            .builder
            .build_call(
                fread_fn,
                &[
                    frb_buf.into(),
                    i64.const_int(1, false).into(),
                    frb_size.into(),
                    frb_handle.into(),
                ],
                "read",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let frb_undef = frb_ret_ty.get_undef();
        let frb_r1 = self
            .builder
            .build_insert_value(frb_undef, frb_read, 0, "r_len")
            .map_err(llvm_err)?;
        let frb_r2 = self
            .builder
            .build_insert_value(frb_r1, frb_buf, 1, "r_ptr")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&frb_r2));

        // ---- action_file_write_bytes(ptr, ptr, i64) -> i1 ----
        // Writes data_len bytes from data to file. Returns true on success.
        let fwb_fn = self.module.add_function(
            "action_file_write_bytes",
            self.bool_ty()
                .fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fwb_fn, "entry");
        self.builder.position_at_end(entry);
        let fwb_handle = fwb_fn.get_first_param().unwrap().into_pointer_value();
        let fwb_data = fwb_fn.get_nth_param(1).unwrap().into_pointer_value();
        let fwb_len = fwb_fn.get_nth_param(2).unwrap().into_int_value();
        let fwb_written = self
            .builder
            .build_call(
                fwrite_fn,
                &[
                    fwb_data.into(),
                    i64.const_int(1, false).into(),
                    fwb_len.into(),
                    fwb_handle.into(),
                ],
                "written",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fwb_ok = self
            .builder
            .build_int_compare(IntPredicate::EQ, fwb_written, fwb_len, "ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fwb_ok));

        // ---- action_file_seek(ptr, i64, i32) -> i1 ----
        // Seeks to position (offset from whence: 0=SET, 1=CUR, 2=END). Returns true on success.
        let fs_fn = self.module.add_function(
            "action_file_seek",
            self.bool_ty()
                .fn_type(&[ptr.into(), i64.into(), i32.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(fs_fn, "entry");
        self.builder.position_at_end(entry);
        let fs_handle = fs_fn.get_first_param().unwrap().into_pointer_value();
        let fs_offset = fs_fn.get_nth_param(1).unwrap().into_int_value();
        let fs_whence = fs_fn.get_nth_param(2).unwrap().into_int_value();
        let fs_ret = self
            .builder
            .build_call(
                fseek_fn,
                &[fs_handle.into(), fs_offset.into(), fs_whence.into()],
                "ret",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let fs_ok = self
            .builder
            .build_int_compare(IntPredicate::EQ, fs_ret, i32.const_int(0, false), "ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fs_ok));

        // ---- action_file_tell(ptr) -> i64 ----
        // Returns current file position.
        let ft_fn =
            self.module
                .add_function("action_file_tell", i64.fn_type(&[ptr.into()], false), None);
        let entry = self.context.append_basic_block(ft_fn, "entry");
        self.builder.position_at_end(entry);
        let ft_handle = ft_fn.get_first_param().unwrap().into_pointer_value();
        let ft_ret = self
            .builder
            .build_call(ftell_fn, &[ft_handle.into()], "ret")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let _ = self.builder.build_return(Some(&ft_ret));

        // ---- action_file_flush(ptr) -> i1 ----
        // Flushes file handle. Returns true on success.
        let fflush_fn = self
            .module
            .add_function("fflush", i32.fn_type(&[ptr.into()], false), None);
        let ff_fn = self.module.add_function(
            "action_file_flush",
            self.bool_ty().fn_type(&[ptr.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(ff_fn, "entry");
        self.builder.position_at_end(entry);
        let ff_handle = ff_fn.get_first_param().unwrap().into_pointer_value();
        let ff_ret = self
            .builder
            .build_call(fflush_fn, &[ff_handle.into()], "ret")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let ff_ok = self
            .builder
            .build_int_compare(IntPredicate::EQ, ff_ret, i32.const_int(0, false), "ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ff_ok));

        // ---- action_rand_init() ----
        // Simple LCG state: uses a global i64 seed initialized to 1
        let rand_seed_g = self.module.add_global(i64, None, "action_rand_seed");
        rand_seed_g.set_initializer(&i64.const_int(123456789, false));

        // ---- action_rand_int(i64 min, i64 max) -> i64 ----
        let ri_fn = self.module.add_function(
            "action_rand_int",
            i64.fn_type(&[i64.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(ri_fn, "entry");
        self.builder.position_at_end(entry);
        let ri_min = ri_fn.get_first_param().unwrap().into_int_value();
        let ri_max = ri_fn.get_nth_param(1).unwrap().into_int_value();
        // LCG: seed = seed * 1103515245 + 12345
        let ri_seed_ptr = rand_seed_g.as_pointer_value();
        let ri_old_seed = self
            .builder
            .build_load(i64, ri_seed_ptr, "old_seed")
            .map_err(llvm_err)?
            .into_int_value();
        let ri_mul = self
            .builder
            .build_int_mul(ri_old_seed, i64.const_int(1103515245, false), "mul")
            .map_err(llvm_err)?;
        let ri_new_seed = self
            .builder
            .build_int_add(ri_mul, i64.const_int(12345, false), "new_seed")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ri_seed_ptr, ri_new_seed)
            .map_err(llvm_err)?;
        // range = max - min + 1
        let ri_range = self
            .builder
            .build_int_sub(ri_max, ri_min, "sub")
            .map_err(llvm_err)?;
        let ri_range1 = self
            .builder
            .build_int_add(ri_range, i64.const_int(1, false), "range1")
            .map_err(llvm_err)?;
        // result = min + (new_seed % range)
        let _ri_range_pos = self
            .builder
            .build_int_compare(IntPredicate::SGT, ri_range1, i64.const_int(0, false), "pos")
            .map_err(llvm_err)?;
        // Use unsigned remainder to avoid negative issues
        let ri_rem = self
            .builder
            .build_int_unsigned_rem(ri_new_seed, ri_range1, "rem")
            .map_err(llvm_err)?;
        let ri_zero = self
            .builder
            .build_int_compare(
                IntPredicate::ULE,
                ri_range1,
                i64.const_int(0, false),
                "zero_range",
            )
            .map_err(llvm_err)?;
        // If range <= 0, return min
        let ri_result = self
            .builder
            .build_select(
                ri_zero,
                ri_min,
                self.builder
                    .build_int_add(ri_min, ri_rem, "add")
                    .map_err(llvm_err)?,
                "result",
            )
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_return(Some(&ri_result));

        // ---- action_rand_float() -> f64 ----
        let rf_fn = self
            .module
            .add_function("action_rand_float", f64.fn_type(&[], false), None);
        let entry = self.context.append_basic_block(rf_fn, "entry");
        self.builder.position_at_end(entry);
        // Use the same LCG seed, return value in [0, 1)
        let rf_seed_ptr = rand_seed_g.as_pointer_value();
        let rf_old_seed = self
            .builder
            .build_load(i64, rf_seed_ptr, "old_seed")
            .map_err(llvm_err)?
            .into_int_value();
        let rf_mul = self
            .builder
            .build_int_mul(rf_old_seed, i64.const_int(1103515245, false), "mul")
            .map_err(llvm_err)?;
        let rf_new_seed = self
            .builder
            .build_int_add(rf_mul, i64.const_int(12345, false), "new_seed")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rf_seed_ptr, rf_new_seed)
            .map_err(llvm_err)?;
        // Convert to float: (new_seed & 0x7fffffffffffffff) / 0x7fffffffffffffff
        let rf_mask = i64.const_int(0x7fffffffffffffff_u64, false);
        let rf_masked = self
            .builder
            .build_and(rf_new_seed, rf_mask, "masked")
            .map_err(llvm_err)?;
        let rf_f64 = self
            .builder
            .build_unsigned_int_to_float(rf_masked, f64, "f64")
            .map_err(llvm_err)?;
        let rf_divisor = f64.const_float(0x7fffffffffffffff_u64 as f64);
        let rf_result = self
            .builder
            .build_float_div(rf_f64, rf_divisor, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&rf_result));

        // ---- action_string_split({i64, ptr}, {i64, ptr}) -> {ptr, i64, i64} ----
        // Returns a list of strings by splitting the input on delimiter occurrences.
        let sp_fn = self.module.add_function(
            "action_string_split",
            list_ty.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let sp_entry = self.context.append_basic_block(sp_fn, "entry");
        self.builder.position_at_end(sp_entry);
        let sp_s = sp_fn.get_first_param().unwrap().into_struct_value();
        let sp_delim = sp_fn.get_nth_param(1).unwrap().into_struct_value();
        let sp_slen = self
            .builder
            .build_extract_value(sp_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_sdata = self
            .builder
            .build_extract_value(sp_s, 1, "sdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sp_dlen = self
            .builder
            .build_extract_value(sp_delim, 0, "dlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_ddata = self
            .builder
            .build_extract_value(sp_delim, 1, "ddata")
            .map_err(llvm_err)?
            .into_pointer_value();

        // Count delimiters
        let sp_count = self.builder.build_alloca(i64, "count").map_err(llvm_err)?;
        let sp_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(sp_count, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        // Need to check dlen > 0 to avoid infinite loops
        let sp_dzero = self
            .builder
            .build_int_compare(IntPredicate::EQ, sp_dlen, i64.const_int(0, false), "dzero")
            .map_err(llvm_err)?;
        let sp_cnt_hdr = self.context.append_basic_block(sp_fn, "cnt_hdr");
        let sp_cnt_body = self.context.append_basic_block(sp_fn, "cnt_body");
        let sp_cnt_ck = self.context.append_basic_block(sp_fn, "cnt_ck");
        let sp_cnt_next = self.context.append_basic_block(sp_fn, "cnt_next");
        let sp_cnt_done = self.context.append_basic_block(sp_fn, "cnt_done");
        let _ = self
            .builder
            .build_conditional_branch(sp_dzero, sp_cnt_done, sp_cnt_hdr);

        // cnt_hdr: while i + dlen <= slen
        self.builder.position_at_end(sp_cnt_hdr);
        let sp_iv = self
            .builder
            .build_load(i64, sp_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_end = self
            .builder
            .build_int_add(sp_iv, sp_dlen, "end")
            .map_err(llvm_err)?;
        let sp_in_range = self
            .builder
            .build_int_compare(IntPredicate::ULE, sp_end, sp_slen, "in_range")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sp_in_range, sp_cnt_body, sp_cnt_done);

        self.builder.position_at_end(sp_cnt_body);
        // Check if substring at pos i matches delimiter
        let sp_src = unsafe {
            self.builder
                .build_gep(i8, sp_sdata, &[sp_iv], "src")
                .map_err(llvm_err)
        }?;
        let sp_mc = self
            .builder
            .build_call(
                memcmp_fn,
                &[sp_src.into(), sp_ddata.into(), sp_dlen.into()],
                "mc",
            )
            .map_err(llvm_err)?;
        let sp_mcr = sp_mc.try_as_basic_value().unwrap_basic().into_int_value();
        let sp_match = self
            .builder
            .build_int_compare(IntPredicate::EQ, sp_mcr, i32.const_int(0, false), "match")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sp_match, sp_cnt_ck, sp_cnt_next);

        self.builder.position_at_end(sp_cnt_ck);
        let sp_cur = self
            .builder
            .build_load(i64, sp_count, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_nc = self
            .builder
            .build_int_add(sp_cur, i64.const_int(1, false), "nc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_count, sp_nc)
            .map_err(llvm_err)?;
        // Skip past delimiter
        let sp_ni = self
            .builder
            .build_int_add(sp_iv, sp_dlen, "ni")
            .map_err(llvm_err)?;
        self.builder.build_store(sp_i, sp_ni).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sp_cnt_hdr);

        self.builder.position_at_end(sp_cnt_next);
        let sp_ni2 = self
            .builder
            .build_int_add(sp_iv, i64.const_int(1, false), "ni2")
            .map_err(llvm_err)?;
        self.builder.build_store(sp_i, sp_ni2).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sp_cnt_hdr);

        // cnt_done: create list with capacity = count + 1, then fill
        self.builder.position_at_end(sp_cnt_done);
        let sp_final_cnt = self
            .builder
            .build_load(i64, sp_count, "final_cnt")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_cap = self
            .builder
            .build_int_add(sp_final_cnt, i64.const_int(1, false), "cap")
            .map_err(llvm_err)?;
        let _sp_cc = self
            .builder
            .build_call(malloc_fn, &[i64.const_int(8, false).into()], "list_alloc")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Use the inline list_create approach: allocate list + data in one go
        // Create data array: capacity * 32 bytes per entry (2 * i64 for fat struct)
        let sp_dsize = self
            .builder
            .build_int_mul(sp_cap, i64.const_int(16, false), "dsize")
            .map_err(llvm_err)?;
        let sp_data = self
            .builder
            .build_call(malloc_fn, &[sp_dsize.into()], "data")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // List struct: {data_ptr, len:0, capacity}
        let sp_und = list_ty.get_undef();
        let sp_lr1 = self
            .builder
            .build_insert_value(sp_und, sp_data, 0, "lr1")
            .map_err(llvm_err)?;
        let sp_lr2 = self
            .builder
            .build_insert_value(sp_lr1, i64.const_int(0, false), 1, "lr2")
            .map_err(llvm_err)?;
        let sp_list_base = self
            .builder
            .build_insert_value(sp_lr2, sp_cap, 2, "lr3")
            .map_err(llvm_err)?;
        let sp_list_ptr = self
            .builder
            .build_alloca(list_ty, "list_ptr")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_list_ptr, sp_list_base)
            .map_err(llvm_err)?;

        // Reset i to 0, last_start = 0
        let sp_last = self.builder.build_alloca(i64, "last").map_err(llvm_err)?;
        self.builder
            .build_store(sp_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_last, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sp_fill_hdr = self.context.append_basic_block(sp_fn, "fill_hdr");
        let sp_fill_body = self.context.append_basic_block(sp_fn, "fill_body");
        let sp_fill_ck2 = self.context.append_basic_block(sp_fn, "fill_ck2");
        let sp_fill_push = self.context.append_basic_block(sp_fn, "fill_push");
        let sp_fill_next = self.context.append_basic_block(sp_fn, "fill_next");
        let sp_fill_last = self.context.append_basic_block(sp_fn, "fill_last");
        let sp_fill_done = self.context.append_basic_block(sp_fn, "fill_done");
        let _ = self
            .builder
            .build_conditional_branch(sp_dzero, sp_fill_last, sp_fill_hdr);

        // fill_hdr: while i + dlen <= slen
        self.builder.position_at_end(sp_fill_hdr);
        let sp_iv2 = self
            .builder
            .build_load(i64, sp_i, "iv2")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_end2 = self
            .builder
            .build_int_add(sp_iv2, sp_dlen, "end2")
            .map_err(llvm_err)?;
        let sp_in2 = self
            .builder
            .build_int_compare(IntPredicate::ULE, sp_end2, sp_slen, "in2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sp_in2, sp_fill_body, sp_fill_last);

        self.builder.position_at_end(sp_fill_body);
        let sp_src2 = unsafe {
            self.builder
                .build_gep(i8, sp_sdata, &[sp_iv2], "src2")
                .map_err(llvm_err)
        }?;
        let sp_mc2 = self
            .builder
            .build_call(
                memcmp_fn,
                &[sp_src2.into(), sp_ddata.into(), sp_dlen.into()],
                "mc2",
            )
            .map_err(llvm_err)?;
        let sp_mcr2 = sp_mc2.try_as_basic_value().unwrap_basic().into_int_value();
        let sp_m2 = self
            .builder
            .build_int_compare(IntPredicate::EQ, sp_mcr2, i32.const_int(0, false), "m2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sp_m2, sp_fill_ck2, sp_fill_next);

        // fill_ck2: push segment from last to i
        self.builder.position_at_end(sp_fill_ck2);
        let sp_last_v = self
            .builder
            .build_load(i64, sp_last, "last_v")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_seg_len = self
            .builder
            .build_int_sub(sp_iv2, sp_last_v, "seg_len")
            .map_err(llvm_err)?;
        // Create substring for this segment
        let sp_salc = self
            .builder
            .build_int_add(sp_seg_len, i64.const_int(1, false), "salc")
            .map_err(llvm_err)?;
        let sp_sbuf = self
            .builder
            .build_call(malloc_fn, &[sp_salc.into()], "sbuf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sp_ssrc = unsafe {
            self.builder
                .build_gep(i8, sp_sdata, &[sp_last_v], "ssrc")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sp_sbuf.into(), sp_ssrc.into(), sp_seg_len.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sp_snull = unsafe {
            self.builder
                .build_gep(i8, sp_sbuf, &[sp_seg_len], "snull")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(sp_snull, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sp_fill_push);

        // fill_push: push the new string to list
        self.builder.position_at_end(sp_fill_push);
        // Build fat struct {seg_len, sbuf} for the string
        let sp_fat_undef = str_ty.get_undef();
        let sp_fat1 = self
            .builder
            .build_insert_value(sp_fat_undef, sp_seg_len, 0, "fat1")
            .map_err(llvm_err)?;
        let sp_fat2 = self
            .builder
            .build_insert_value(sp_fat1, sp_sbuf, 1, "fat2")
            .map_err(llvm_err)?;
        // Load list, push fat struct
        let sp_ll = self
            .builder
            .build_load(list_ty, sp_list_ptr, "ll")
            .map_err(llvm_err)?
            .into_struct_value();
        // Inline list push: get len, check capacity, store at data[len], increment len
        let sp_llen = self
            .builder
            .build_extract_value(sp_ll, 1, "llen")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_ldata = self
            .builder
            .build_extract_value(sp_ll, 0, "ldata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sp_offset = self
            .builder
            .build_int_mul(sp_llen, i64.const_int(16, false), "offset")
            .map_err(llvm_err)?;
        let sp_dst = unsafe {
            self.builder
                .build_gep(i8, sp_ldata, &[sp_offset], "dst")
                .map_err(llvm_err)
        }?;
        let sp_dst_i64 = self
            .builder
            .build_pointer_cast(sp_dst, ptr, "dst_i64")
            .map_err(llvm_err)?;
        // Store tag
        let sp_ftag = self
            .builder
            .build_extract_value(sp_fat2, 0, "ftag")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(sp_dst_i64, sp_ftag)
            .map_err(llvm_err)?;
        // Store ptr
        let sp_fp = self
            .builder
            .build_extract_value(sp_fat2, 1, "fp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sp_off1 = self
            .builder
            .build_int_add(sp_offset, i64.const_int(8, false), "off1")
            .map_err(llvm_err)?;
        let sp_pp = unsafe {
            self.builder
                .build_gep(i8, sp_ldata, &[sp_off1], "pp")
                .map_err(llvm_err)
        }?;
        let sp_ppi64 = self
            .builder
            .build_pointer_cast(sp_pp, ptr, "ppi64")
            .map_err(llvm_err)?;
        let sp_fp_i64 = self
            .builder
            .build_ptr_to_int(sp_fp, i64, "fp_i64")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_ppi64, sp_fp_i64)
            .map_err(llvm_err)?;
        // Increment len
        let sp_nlen = self
            .builder
            .build_int_add(sp_llen, i64.const_int(1, false), "nlen")
            .map_err(llvm_err)?;
        let sp_nlist_und = list_ty.get_undef();
        let sp_nl1 = self
            .builder
            .build_insert_value(sp_nlist_und, sp_ldata, 0, "nl1")
            .map_err(llvm_err)?;
        let sp_nl2 = self
            .builder
            .build_insert_value(sp_nl1, sp_nlen, 1, "nl2")
            .map_err(llvm_err)?;
        let sp_nl3 = self
            .builder
            .build_insert_value(sp_nl2, sp_cap, 2, "nl3")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_list_ptr, sp_nl3)
            .map_err(llvm_err)?;
        // Update last = i + dlen
        let sp_nlast = self
            .builder
            .build_int_add(sp_iv2, sp_dlen, "nlast")
            .map_err(llvm_err)?;
        self.builder.build_store(sp_i, sp_nlast).map_err(llvm_err)?;
        self.builder
            .build_store(sp_last, sp_nlast)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sp_fill_hdr);

        // fill_next: i += 1
        self.builder.position_at_end(sp_fill_next);
        let sp_ni3 = self
            .builder
            .build_int_add(sp_iv2, i64.const_int(1, false), "ni3")
            .map_err(llvm_err)?;
        self.builder.build_store(sp_i, sp_ni3).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sp_fill_hdr);

        // fill_last: push remaining segment from last to slen
        self.builder.position_at_end(sp_fill_last);
        let sp_last_v2 = self
            .builder
            .build_load(i64, sp_last, "last_v2")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_seg_len2 = self
            .builder
            .build_int_sub(sp_slen, sp_last_v2, "seg_len2")
            .map_err(llvm_err)?;
        let sp_salc2 = self
            .builder
            .build_int_add(sp_seg_len2, i64.const_int(1, false), "salc2")
            .map_err(llvm_err)?;
        let sp_sbuf2 = self
            .builder
            .build_call(malloc_fn, &[sp_salc2.into()], "sbuf2")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sp_ssrc2 = unsafe {
            self.builder
                .build_gep(i8, sp_sdata, &[sp_last_v2], "ssrc2")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sp_sbuf2.into(), sp_ssrc2.into(), sp_seg_len2.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sp_snull2 = unsafe {
            self.builder
                .build_gep(i8, sp_sbuf2, &[sp_seg_len2], "snull2")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(sp_snull2, i8.const_int(0, false))
            .map_err(llvm_err)?;
        // Build fat struct
        let sp_fat_undef2 = str_ty.get_undef();
        let sp_fat1b = self
            .builder
            .build_insert_value(sp_fat_undef2, sp_seg_len2, 0, "fat1b")
            .map_err(llvm_err)?;
        let sp_fat2b = self
            .builder
            .build_insert_value(sp_fat1b, sp_sbuf2, 1, "fat2b")
            .map_err(llvm_err)?;
        // Push to list
        let sp_ll2 = self
            .builder
            .build_load(list_ty, sp_list_ptr, "ll2")
            .map_err(llvm_err)?
            .into_struct_value();
        let sp_llen2 = self
            .builder
            .build_extract_value(sp_ll2, 1, "llen2")
            .map_err(llvm_err)?
            .into_int_value();
        let sp_ldata2 = self
            .builder
            .build_extract_value(sp_ll2, 0, "ldata2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sp_offset2 = self
            .builder
            .build_int_mul(sp_llen2, i64.const_int(16, false), "offset2")
            .map_err(llvm_err)?;
        let sp_dst2 = unsafe {
            self.builder
                .build_gep(i8, sp_ldata2, &[sp_offset2], "dst2")
                .map_err(llvm_err)
        }?;
        let sp_dst2_i64 = self
            .builder
            .build_pointer_cast(sp_dst2, ptr, "dst2_i64")
            .map_err(llvm_err)?;
        let sp_ftag2 = self
            .builder
            .build_extract_value(sp_fat2b, 0, "ftag2")
            .map_err(llvm_err)?
            .into_int_value();
        self.builder
            .build_store(sp_dst2_i64, sp_ftag2)
            .map_err(llvm_err)?;
        let sp_fp2 = self
            .builder
            .build_extract_value(sp_fat2b, 1, "fp2")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sp_off1b = self
            .builder
            .build_int_add(sp_offset2, i64.const_int(8, false), "off1b")
            .map_err(llvm_err)?;
        let sp_pp2 = unsafe {
            self.builder
                .build_gep(i8, sp_ldata2, &[sp_off1b], "pp2")
                .map_err(llvm_err)
        }?;
        let sp_pp2i64 = self
            .builder
            .build_pointer_cast(sp_pp2, ptr, "pp2i64")
            .map_err(llvm_err)?;
        let sp_fp2_i64 = self
            .builder
            .build_ptr_to_int(sp_fp2, i64, "fp2_i64")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_pp2i64, sp_fp2_i64)
            .map_err(llvm_err)?;
        let sp_nlen2 = self
            .builder
            .build_int_add(sp_llen2, i64.const_int(1, false), "nlen2")
            .map_err(llvm_err)?;
        let sp_nlist_und2 = list_ty.get_undef();
        let sp_nl1b = self
            .builder
            .build_insert_value(sp_nlist_und2, sp_ldata2, 0, "nl1b")
            .map_err(llvm_err)?;
        let sp_nl2b = self
            .builder
            .build_insert_value(sp_nl1b, sp_nlen2, 1, "nl2b")
            .map_err(llvm_err)?;
        let sp_nl3b = self
            .builder
            .build_insert_value(sp_nl2b, sp_cap, 2, "nl3b")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sp_list_ptr, sp_nl3b)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sp_fill_done);

        // fill_done: return list
        self.builder.position_at_end(sp_fill_done);
        let sp_result = self
            .builder
            .build_load(list_ty, sp_list_ptr, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sp_result));

        // ---- action_string_join({ptr, i64, i64}, {i64, ptr}) -> {i64, ptr} ----
        let jn_fn = self.module.add_function(
            "action_string_join",
            str_ty.fn_type(&[list_ty.into(), str_ty.into()], false),
            None,
        );
        let jn_entry = self.context.append_basic_block(jn_fn, "entry");
        self.builder.position_at_end(jn_entry);
        let jn_list = jn_fn.get_first_param().unwrap().into_struct_value();
        let jn_delim = jn_fn.get_nth_param(1).unwrap().into_struct_value();
        let jn_ldata = self
            .builder
            .build_extract_value(jn_list, 0, "ldata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let jn_llen = self
            .builder
            .build_extract_value(jn_list, 1, "llen")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_dlen = self
            .builder
            .build_extract_value(jn_delim, 0, "dlen")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_ddata = self
            .builder
            .build_extract_value(jn_delim, 1, "ddata")
            .map_err(llvm_err)?
            .into_pointer_value();

        // Compute total size
        let jn_total = self.builder.build_alloca(i64, "total").map_err(llvm_err)?;
        self.builder
            .build_store(jn_total, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let jn_ji = self.builder.build_alloca(i64, "ji").map_err(llvm_err)?;
        self.builder
            .build_store(jn_ji, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let jn_hdr = self.context.append_basic_block(jn_fn, "hdr");
        let jn_body = self.context.append_basic_block(jn_fn, "body");
        let jn_after = self.context.append_basic_block(jn_fn, "after");
        let _ = self.builder.build_unconditional_branch(jn_hdr);

        // Sum all string lengths + delimiter lengths
        self.builder.position_at_end(jn_hdr);
        let jn_iv = self
            .builder
            .build_load(i64, jn_ji, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_more = self
            .builder
            .build_int_compare(IntPredicate::ULT, jn_iv, jn_llen, "more")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(jn_more, jn_body, jn_after);

        self.builder.position_at_end(jn_body);
        let jn_off = self
            .builder
            .build_int_mul(jn_iv, i64.const_int(16, false), "off")
            .map_err(llvm_err)?;
        let jn_ep = unsafe {
            self.builder
                .build_gep(i8, jn_ldata, &[jn_off], "ep")
                .map_err(llvm_err)
        }?;
        let jn_epi64 = self
            .builder
            .build_pointer_cast(jn_ep, ptr, "epi64")
            .map_err(llvm_err)?;
        let jn_sslen = self
            .builder
            .build_load(i64, jn_epi64, "sslen")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cur = self
            .builder
            .build_load(i64, jn_total, "cur")
            .map_err(llvm_err)?
            .into_int_value();
        let _jn_add = self
            .builder
            .build_int_add(jn_cur, jn_sslen, "add")
            .map_err(llvm_err)?;
        // Add delimiter length if not last element
        let jn_is_last = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_int_add(jn_iv, i64.const_int(1, false), "ivp1")
                    .map_err(llvm_err)?,
                jn_llen,
                "is_last",
            )
            .map_err(llvm_err)?;
        let jn_with_delim = self
            .builder
            .build_int_add(jn_sslen, jn_dlen, "with_delim")
            .map_err(llvm_err)?;
        let jn_delta_sv = self
            .builder
            .build_select(jn_is_last, jn_sslen, jn_with_delim, "delta")
            .map_err(llvm_err)?;
        let jn_delta = jn_delta_sv.into_int_value();
        let jn_new_total = self
            .builder
            .build_int_add(jn_cur, jn_delta, "new_total")
            .map_err(llvm_err)?;
        self.builder
            .build_store(jn_total, jn_new_total)
            .map_err(llvm_err)?;
        let jn_niv = self
            .builder
            .build_int_add(jn_iv, i64.const_int(1, false), "niv")
            .map_err(llvm_err)?;
        self.builder.build_store(jn_ji, jn_niv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(jn_hdr);

        // Allocate and copy
        self.builder.position_at_end(jn_after);
        let jn_final_total = self
            .builder
            .build_load(i64, jn_total, "final_total")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_jalc = self
            .builder
            .build_int_add(jn_final_total, i64.const_int(1, false), "jalc")
            .map_err(llvm_err)?;
        let jn_buf = self
            .builder
            .build_call(malloc_fn, &[jn_jalc.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Reset i, reset write cursor
        let jn_wpos = self.builder.build_alloca(i64, "wpos").map_err(llvm_err)?;
        self.builder
            .build_store(jn_ji, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(jn_wpos, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let jn_chdr = self.context.append_basic_block(jn_fn, "chdr");
        let jn_cbody = self.context.append_basic_block(jn_fn, "cbody");
        let jn_cdone = self.context.append_basic_block(jn_fn, "cdone");
        let _ = self.builder.build_unconditional_branch(jn_chdr);

        self.builder.position_at_end(jn_chdr);
        let jn_civ = self
            .builder
            .build_load(i64, jn_ji, "civ")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cmore = self
            .builder
            .build_int_compare(IntPredicate::ULT, jn_civ, jn_llen, "cmore")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(jn_cmore, jn_cbody, jn_cdone);

        self.builder.position_at_end(jn_cbody);
        let jn_coff = self
            .builder
            .build_int_mul(jn_civ, i64.const_int(16, false), "coff")
            .map_err(llvm_err)?;
        let jn_cep = unsafe {
            self.builder
                .build_gep(i8, jn_ldata, &[jn_coff], "cep")
                .map_err(llvm_err)
        }?;
        let jn_cepi64 = self
            .builder
            .build_pointer_cast(jn_cep, ptr, "cepi64")
            .map_err(llvm_err)?;
        let jn_csslen = self
            .builder
            .build_load(i64, jn_cepi64, "csslen")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_coff1 = self
            .builder
            .build_int_add(jn_coff, i64.const_int(8, false), "coff1")
            .map_err(llvm_err)?;
        let jn_cpp = unsafe {
            self.builder
                .build_gep(i8, jn_ldata, &[jn_coff1], "cpp")
                .map_err(llvm_err)
        }?;
        let jn_cppi64 = self
            .builder
            .build_pointer_cast(jn_cpp, ptr, "cppi64")
            .map_err(llvm_err)?;
        let jn_cpval = self
            .builder
            .build_load(i64, jn_cppi64, "cpval")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cp = self
            .builder
            .build_int_to_ptr(jn_cpval, ptr, "cp")
            .map_err(llvm_err)?;
        // Copy string data to output at wpos
        let jn_cwp = self
            .builder
            .build_load(i64, jn_wpos, "cwp")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cdst = unsafe {
            self.builder
                .build_gep(i8, jn_buf, &[jn_cwp], "cdst")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[jn_cdst.into(), jn_cp.into(), jn_csslen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let jn_nwp = self
            .builder
            .build_int_add(jn_cwp, jn_csslen, "nwp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(jn_wpos, jn_nwp)
            .map_err(llvm_err)?;
        // Copy delimiter if not last
        let jn_cis_last = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_int_add(jn_civ, i64.const_int(1, false), "civp1")
                    .map_err(llvm_err)?,
                jn_llen,
                "cis_last",
            )
            .map_err(llvm_err)?;
        let jn_cdel_bb = self.context.append_basic_block(jn_fn, "cdel");
        let jn_cnext_bb = self.context.append_basic_block(jn_fn, "cnext");
        let _ = self
            .builder
            .build_conditional_branch(jn_cis_last, jn_cnext_bb, jn_cdel_bb);

        self.builder.position_at_end(jn_cdel_bb);
        let jn_cwp2 = self
            .builder
            .build_load(i64, jn_wpos, "cwp2")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_cdst2 = unsafe {
            self.builder
                .build_gep(i8, jn_buf, &[jn_cwp2], "cdst2")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[jn_cdst2.into(), jn_ddata.into(), jn_dlen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let jn_nwp2 = self
            .builder
            .build_int_add(jn_cwp2, jn_dlen, "nwp2")
            .map_err(llvm_err)?;
        self.builder
            .build_store(jn_wpos, jn_nwp2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(jn_cnext_bb);

        self.builder.position_at_end(jn_cnext_bb);
        let jn_cniv = self
            .builder
            .build_int_add(jn_civ, i64.const_int(1, false), "cniv")
            .map_err(llvm_err)?;
        self.builder.build_store(jn_ji, jn_cniv).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(jn_chdr);

        // Done: null-terminate and return
        self.builder.position_at_end(jn_cdone);
        let jn_fwp = self
            .builder
            .build_load(i64, jn_wpos, "fwp")
            .map_err(llvm_err)?
            .into_int_value();
        let jn_nullp = unsafe {
            self.builder
                .build_gep(i8, jn_buf, &[jn_fwp], "nullp")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(jn_nullp, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let jn_und = str_ty.get_undef();
        let jn_r1 = self
            .builder
            .build_insert_value(jn_und, jn_fwp, 0, "r1")
            .map_err(llvm_err)?;
        let jn_r2 = self
            .builder
            .build_insert_value(jn_r1, jn_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&jn_r2));

        // ---- action_string_replace({i64, ptr}, {i64, ptr}, {i64, ptr}) -> {i64, ptr} ----
        let rp_fn = self.module.add_function(
            "action_string_replace",
            str_ty.fn_type(&[str_ty.into(), str_ty.into(), str_ty.into()], false),
            None,
        );
        let rp_entry = self.context.append_basic_block(rp_fn, "entry");
        self.builder.position_at_end(rp_entry);
        let rp_s = rp_fn.get_first_param().unwrap().into_struct_value();
        let rp_from = rp_fn.get_nth_param(1).unwrap().into_struct_value();
        let rp_to = rp_fn.get_nth_param(2).unwrap().into_struct_value();
        let rp_slen = self
            .builder
            .build_extract_value(rp_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_sdata = self
            .builder
            .build_extract_value(rp_s, 1, "sdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rp_flen = self
            .builder
            .build_extract_value(rp_from, 0, "flen")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_fdata = self
            .builder
            .build_extract_value(rp_from, 1, "fdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rp_tlen = self
            .builder
            .build_extract_value(rp_to, 0, "tlen")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_tdata = self
            .builder
            .build_extract_value(rp_to, 1, "tdata")
            .map_err(llvm_err)?
            .into_pointer_value();

        // If from is empty, return copy of original
        let rp_fzero = self
            .builder
            .build_int_compare(IntPredicate::EQ, rp_flen, i64.const_int(0, false), "fzero")
            .map_err(llvm_err)?;
        let rp_have_from = self.context.append_basic_block(rp_fn, "have_from");
        let rp_copy_ret = self.context.append_basic_block(rp_fn, "copy_ret");
        let _ = self
            .builder
            .build_conditional_branch(rp_fzero, rp_copy_ret, rp_have_from);

        // Copy return: just duplicate the original string
        self.builder.position_at_end(rp_copy_ret);
        let rp_calc = self
            .builder
            .build_int_add(rp_slen, i64.const_int(1, false), "calc")
            .map_err(llvm_err)?;
        let rp_cbuf = self
            .builder
            .build_call(malloc_fn, &[rp_calc.into()], "cbuf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[rp_cbuf.into(), rp_sdata.into(), rp_slen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let rp_cnull = unsafe {
            self.builder
                .build_gep(i8, rp_cbuf, &[rp_slen], "cnull")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(rp_cnull, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let rp_cund = str_ty.get_undef();
        let rp_cr1 = self
            .builder
            .build_insert_value(rp_cund, rp_slen, 0, "cr1")
            .map_err(llvm_err)?;
        let rp_cr2 = self
            .builder
            .build_insert_value(rp_cr1, rp_cbuf, 1, "cr2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&rp_cr2));

        // have_from: count occurrences and compute result size
        self.builder.position_at_end(rp_have_from);
        let rp_ri = self.builder.build_alloca(i64, "ri").map_err(llvm_err)?;
        let rp_rlast = self.builder.build_alloca(i64, "rlast").map_err(llvm_err)?;
        let rp_count = self.builder.build_alloca(i64, "rcount").map_err(llvm_err)?;
        self.builder
            .build_store(rp_ri, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_rlast, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_count, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let rp_hdr = self.context.append_basic_block(rp_fn, "hdr");
        let rp_body = self.context.append_basic_block(rp_fn, "body");
        let rp_ck = self.context.append_basic_block(rp_fn, "ck");
        let rp_nxt = self.context.append_basic_block(rp_fn, "nxt");
        let rp_build = self.context.append_basic_block(rp_fn, "build");
        let _ = self.builder.build_unconditional_branch(rp_hdr);

        // Scan loop: find matches, count them
        self.builder.position_at_end(rp_hdr);
        let rp_riv = self
            .builder
            .build_load(i64, rp_ri, "riv")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_end = self
            .builder
            .build_int_add(rp_riv, rp_flen, "end")
            .map_err(llvm_err)?;
        let rp_ok = self
            .builder
            .build_int_compare(IntPredicate::ULE, rp_end, rp_slen, "ok")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rp_ok, rp_body, rp_build);

        self.builder.position_at_end(rp_body);
        let rp_rsrc = unsafe {
            self.builder
                .build_gep(i8, rp_sdata, &[rp_riv], "rsrc")
                .map_err(llvm_err)
        }?;
        let rp_rmc = self
            .builder
            .build_call(
                memcmp_fn,
                &[rp_rsrc.into(), rp_fdata.into(), rp_flen.into()],
                "rmc",
            )
            .map_err(llvm_err)?;
        let rp_rmcr = rp_rmc.try_as_basic_value().unwrap_basic().into_int_value();
        let rp_rm = self
            .builder
            .build_int_compare(IntPredicate::EQ, rp_rmcr, i32.const_int(0, false), "rm")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(rp_rm, rp_ck, rp_nxt);

        self.builder.position_at_end(rp_ck);
        let rp_rc = self
            .builder
            .build_load(i64, rp_count, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_nc = self
            .builder
            .build_int_add(rp_rc, i64.const_int(1, false), "nc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_count, rp_nc)
            .map_err(llvm_err)?;
        let rp_nri = self
            .builder
            .build_int_add(rp_riv, rp_flen, "nri")
            .map_err(llvm_err)?;
        self.builder.build_store(rp_ri, rp_nri).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_hdr);

        self.builder.position_at_end(rp_nxt);
        let rp_nri2 = self
            .builder
            .build_int_add(rp_riv, i64.const_int(1, false), "nri2")
            .map_err(llvm_err)?;
        self.builder.build_store(rp_ri, rp_nri2).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_hdr);

        // build: allocate and copy with replacements
        self.builder.position_at_end(rp_build);
        let rp_fc = self
            .builder
            .build_load(i64, rp_count, "fc")
            .map_err(llvm_err)?
            .into_int_value();
        // new_len = slen + count * (tlen - flen)
        let rp_diff = self
            .builder
            .build_int_sub(rp_tlen, rp_flen, "diff")
            .map_err(llvm_err)?;
        let rp_extra = self
            .builder
            .build_int_mul(rp_fc, rp_diff, "extra")
            .map_err(llvm_err)?;
        let rp_nlen = self
            .builder
            .build_int_add(rp_slen, rp_extra, "nlen")
            .map_err(llvm_err)?;
        let rp_nalc = self
            .builder
            .build_int_add(rp_nlen, i64.const_int(1, false), "nalc")
            .map_err(llvm_err)?;
        let rp_nbuf = self
            .builder
            .build_call(malloc_fn, &[rp_nalc.into()], "nbuf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();

        // Reset scan
        self.builder
            .build_store(rp_ri, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_rlast, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let rp_wpos = self.builder.build_alloca(i64, "wpos").map_err(llvm_err)?;
        self.builder
            .build_store(rp_wpos, i64.const_int(0, false))
            .map_err(llvm_err)?;

        let rp_bhdr = self.context.append_basic_block(rp_fn, "bhdr");
        let rp_bbody = self.context.append_basic_block(rp_fn, "bbody");
        let rp_bck = self.context.append_basic_block(rp_fn, "bck");
        let rp_bnxt = self.context.append_basic_block(rp_fn, "bnxt");
        let rp_bfinal = self.context.append_basic_block(rp_fn, "bfinal");
        let rp_bdone = self.context.append_basic_block(rp_fn, "bdone");
        let _ = self.builder.build_unconditional_branch(rp_bhdr);

        self.builder.position_at_end(rp_bhdr);
        let rp_briv = self
            .builder
            .build_load(i64, rp_ri, "briv")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_bend = self
            .builder
            .build_int_add(rp_briv, rp_flen, "bend")
            .map_err(llvm_err)?;
        let rp_bok = self
            .builder
            .build_int_compare(IntPredicate::ULE, rp_bend, rp_slen, "bok")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rp_bok, rp_bbody, rp_bfinal);

        self.builder.position_at_end(rp_bbody);
        let rp_brsrc = unsafe {
            self.builder
                .build_gep(i8, rp_sdata, &[rp_briv], "brsrc")
                .map_err(llvm_err)
        }?;
        let rp_bmc = self
            .builder
            .build_call(
                memcmp_fn,
                &[rp_brsrc.into(), rp_fdata.into(), rp_flen.into()],
                "bmc",
            )
            .map_err(llvm_err)?;
        let rp_bmcr = rp_bmc.try_as_basic_value().unwrap_basic().into_int_value();
        let rp_bm = self
            .builder
            .build_int_compare(IntPredicate::EQ, rp_bmcr, i32.const_int(0, false), "bm")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rp_bm, rp_bck, rp_bnxt);

        // Match found: copy any non-matched part before it, then copy replacement
        self.builder.position_at_end(rp_bck);
        let rp_blast = self
            .builder
            .build_load(i64, rp_rlast, "blast")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_bgap = self
            .builder
            .build_int_sub(rp_briv, rp_blast, "bgap")
            .map_err(llvm_err)?;
        let rp_bwp = self
            .builder
            .build_load(i64, rp_wpos, "bwp")
            .map_err(llvm_err)?
            .into_int_value();
        // Copy gap (non-matched chars before this match)
        let rp_bgsrc = unsafe {
            self.builder
                .build_gep(i8, rp_sdata, &[rp_blast], "bgsrc")
                .map_err(llvm_err)
        }?;
        let rp_bgdst = unsafe {
            self.builder
                .build_gep(i8, rp_nbuf, &[rp_bwp], "bgdst")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[rp_bgdst.into(), rp_bgsrc.into(), rp_bgap.into()],
                "",
            )
            .map_err(llvm_err)?;
        let rp_bnwp1 = self
            .builder
            .build_int_add(rp_bwp, rp_bgap, "bnwp1")
            .map_err(llvm_err)?;
        // Copy replacement
        let rp_brdst = unsafe {
            self.builder
                .build_gep(i8, rp_nbuf, &[rp_bnwp1], "brdst")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[rp_brdst.into(), rp_tdata.into(), rp_tlen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let rp_bnwp2 = self
            .builder
            .build_int_add(rp_bnwp1, rp_tlen, "bnwp2")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_wpos, rp_bnwp2)
            .map_err(llvm_err)?;
        let rp_bnri = self
            .builder
            .build_int_add(rp_briv, rp_flen, "bnri")
            .map_err(llvm_err)?;
        self.builder.build_store(rp_ri, rp_bnri).map_err(llvm_err)?;
        self.builder
            .build_store(rp_rlast, rp_bnri)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_bhdr);

        self.builder.position_at_end(rp_bnxt);
        let rp_bnri2 = self
            .builder
            .build_int_add(rp_briv, i64.const_int(1, false), "bnri2")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rp_ri, rp_bnri2)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_bhdr);

        // Copy remaining after last match
        self.builder.position_at_end(rp_bfinal);
        let rp_blast2 = self
            .builder
            .build_load(i64, rp_rlast, "blast2")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_brem = self
            .builder
            .build_int_sub(rp_slen, rp_blast2, "brem")
            .map_err(llvm_err)?;
        let rp_bwp2 = self
            .builder
            .build_load(i64, rp_wpos, "bwp2")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_brsrc2 = unsafe {
            self.builder
                .build_gep(i8, rp_sdata, &[rp_blast2], "brsrc2")
                .map_err(llvm_err)
        }?;
        let rp_brdst2 = unsafe {
            self.builder
                .build_gep(i8, rp_nbuf, &[rp_bwp2], "brdst2")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[rp_brdst2.into(), rp_brsrc2.into(), rp_brem.into()],
                "",
            )
            .map_err(llvm_err)?;
        let _rp_bnwp3 = self
            .builder
            .build_int_add(rp_bwp2, rp_brem, "bnwp3")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rp_bdone);

        self.builder.position_at_end(rp_bdone);
        let rp_fwpos = self
            .builder
            .build_load(i64, rp_wpos, "fwpos")
            .map_err(llvm_err)?
            .into_int_value();
        let rp_bnull = unsafe {
            self.builder
                .build_gep(i8, rp_nbuf, &[rp_fwpos], "bnull")
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(rp_bnull, i8.const_int(0, false))
            .map_err(llvm_err)?;
        let rp_rund = str_ty.get_undef();
        let rp_rr1 = self
            .builder
            .build_insert_value(rp_rund, rp_fwpos, 0, "rr1")
            .map_err(llvm_err)?;
        let rp_rr2 = self
            .builder
            .build_insert_value(rp_rr1, rp_nbuf, 1, "rr2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&rp_rr2));

        // ---- action_string_contains({i64, ptr}, {i64, ptr}) -> i1 ----
        let sc_fn = self.module.add_function(
            "action_string_contains",
            b1.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let sc_entry = self.context.append_basic_block(sc_fn, "entry");
        self.builder.position_at_end(sc_entry);
        let sc_haystack = sc_fn.get_first_param().unwrap().into_struct_value();
        let sc_needle = sc_fn.get_nth_param(1).unwrap().into_struct_value();
        let sc_hlen = self
            .builder
            .build_extract_value(sc_haystack, 0, "hlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sc_hptr = self
            .builder
            .build_extract_value(sc_haystack, 1, "hptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sc_nlen = self
            .builder
            .build_extract_value(sc_needle, 0, "nlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sc_nptr = self
            .builder
            .build_extract_value(sc_needle, 1, "nptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        // If needle is empty, return true
        let sc_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, sc_nlen, i64.const_int(0, false), "nempty")
            .map_err(llvm_err)?;
        let sc_len_ok = self
            .builder
            .build_int_compare(IntPredicate::SLE, sc_nlen, sc_hlen, "lenok")
            .map_err(llvm_err)?;
        let _sc_can_search = self
            .builder
            .build_and(
                sc_len_ok,
                self.builder
                    .build_not(sc_empty, "not_empty")
                    .map_err(llvm_err)?,
                "can_search",
            )
            .map_err(llvm_err)?;
        // Brute-force search
        let sc_max = self
            .builder
            .build_int_sub(sc_hlen, sc_nlen, "max")
            .map_err(llvm_err)?;
        let sc_loop_bb = self.context.append_basic_block(sc_fn, "sc_loop");
        let sc_found_bb = self.context.append_basic_block(sc_fn, "sc_found");
        let sc_notfound_bb = self.context.append_basic_block(sc_fn, "sc_notfound");
        let _ = self.builder.build_unconditional_branch(sc_loop_bb);
        self.builder.position_at_end(sc_loop_bb);
        let sc_i = self.builder.build_phi(i64, "sc_i").map_err(llvm_err)?;
        // Compare character by character
        let sc_j_loop_bb = self.context.append_basic_block(sc_fn, "sc_jloop");
        let sc_match_bb = self.context.append_basic_block(sc_fn, "sc_match");
        let sc_mismatch_bb = self.context.append_basic_block(sc_fn, "sc_mismatch");
        let _ = self.builder.build_unconditional_branch(sc_j_loop_bb);
        self.builder.position_at_end(sc_j_loop_bb);
        let sc_j = self.builder.build_phi(i64, "sc_j").map_err(llvm_err)?;
        let sc_hidx = self
            .builder
            .build_int_add(
                sc_i.as_basic_value().into_int_value(),
                sc_j.as_basic_value().into_int_value(),
                "hidx",
            )
            .map_err(llvm_err)?;
        let sc_hp = unsafe {
            self.builder
                .build_gep(i8, sc_hptr, &[sc_hidx], "hp")
                .map_err(llvm_err)
        }?;
        let sc_hc = self
            .builder
            .build_load(i8, sc_hp, "hc")
            .map_err(llvm_err)?
            .into_int_value();
        let sc_np = unsafe {
            self.builder
                .build_gep(i8, sc_nptr, &[sc_j.as_basic_value().into_int_value()], "np")
                .map_err(llvm_err)
        }?;
        let sc_nc = self
            .builder
            .build_load(i8, sc_np, "nc")
            .map_err(llvm_err)?
            .into_int_value();
        let sc_char_match = self
            .builder
            .build_int_compare(IntPredicate::EQ, sc_hc, sc_nc, "char_match")
            .map_err(llvm_err)?;
        let sc_j_next = self
            .builder
            .build_int_add(
                sc_j.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "jnext",
            )
            .map_err(llvm_err)?;
        let sc_j_done = self
            .builder
            .build_int_compare(IntPredicate::SGE, sc_j_next, sc_nlen, "jdone")
            .map_err(llvm_err)?;
        sc_j.add_incoming(&[(&i64.const_int(0, false), sc_loop_bb)]);
        let _ = self
            .builder
            .build_conditional_branch(sc_char_match, sc_match_bb, sc_mismatch_bb);
        self.builder.position_at_end(sc_match_bb);
        sc_j.add_incoming(&[(&sc_j_next, sc_match_bb)]);
        let _ = self
            .builder
            .build_conditional_branch(sc_j_done, sc_found_bb, sc_j_loop_bb);
        self.builder.position_at_end(sc_mismatch_bb);
        let sc_i_next = self
            .builder
            .build_int_add(
                sc_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "inext",
            )
            .map_err(llvm_err)?;
        let sc_i_done = self
            .builder
            .build_int_compare(IntPredicate::SGT, sc_i_next, sc_max, "idone")
            .map_err(llvm_err)?;
        let sc_i_block = self.builder.get_insert_block().unwrap();
        sc_i.add_incoming(&[
            (&i64.const_int(0, false), sc_entry),
            (&sc_i_next, sc_i_block),
        ]);
        let _ = self
            .builder
            .build_conditional_branch(sc_i_done, sc_notfound_bb, sc_loop_bb);
        self.builder.position_at_end(sc_found_bb);
        let _ = self.builder.build_return(Some(&b1.const_int(1, false)));
        self.builder.position_at_end(sc_notfound_bb);
        let _ = self.builder.build_return(Some(&b1.const_int(0, false)));

        // ---- action_string_repeat({i64, ptr}, i64) -> {i64, ptr} ----
        let sr_fn = self.module.add_function(
            "action_string_repeat",
            str_ty.fn_type(&[str_ty.into(), i64.into()], false),
            None,
        );
        let sr_entry = self.context.append_basic_block(sr_fn, "entry");
        self.builder.position_at_end(sr_entry);
        let sr_str = sr_fn.get_first_param().unwrap().into_struct_value();
        let sr_n = sr_fn.get_nth_param(1).unwrap().into_int_value();
        let sr_slen = self
            .builder
            .build_extract_value(sr_str, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sr_sptr = self
            .builder
            .build_extract_value(sr_str, 1, "sptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sr_total = self
            .builder
            .build_int_mul(sr_slen, sr_n, "total")
            .map_err(llvm_err)?;
        let sr_buf = self
            .builder
            .build_call(malloc_fn, &[sr_total.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .basic()
            .ok_or("malloc")?
            .into_pointer_value();
        // Loop: copy s into buffer n times
        let sr_loop_bb = self.context.append_basic_block(sr_fn, "sr_loop");
        let sr_done_bb = self.context.append_basic_block(sr_fn, "sr_done");
        let _ = self.builder.build_unconditional_branch(sr_loop_bb);
        self.builder.position_at_end(sr_loop_bb);
        let sr_i = self.builder.build_phi(i64, "sr_i").map_err(llvm_err)?;
        let sr_offset = self
            .builder
            .build_int_mul(sr_i.as_basic_value().into_int_value(), sr_slen, "offset")
            .map_err(llvm_err)?;
        let sr_dst = unsafe {
            self.builder
                .build_gep(i8, sr_buf, &[sr_offset], "dst")
                .map_err(llvm_err)
        }?;
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sr_dst.into(), sr_sptr.into(), sr_slen.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sr_i_next = self
            .builder
            .build_int_add(
                sr_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "sri_next",
            )
            .map_err(llvm_err)?;
        let sr_done_cond = self
            .builder
            .build_int_compare(IntPredicate::SGE, sr_i_next, sr_n, "srdone")
            .map_err(llvm_err)?;
        let sr_loop_block = self.builder.get_insert_block().unwrap();
        sr_i.add_incoming(&[
            (&i64.const_int(0, false), sr_entry),
            (&sr_i_next, sr_loop_block),
        ]);
        let _ = self
            .builder
            .build_conditional_branch(sr_done_cond, sr_done_bb, sr_loop_bb);
        self.builder.position_at_end(sr_done_bb);
        let sr_undef = str_ty.get_undef();
        let sr_r1 = self
            .builder
            .build_insert_value(sr_undef, sr_total, 0, "r1")
            .map_err(llvm_err)?;
        let sr_r2 = self
            .builder
            .build_insert_value(sr_r1, sr_buf, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sr_r2));

        // ---- action_string_trim_start({i64, ptr}) -> {i64, ptr} ----
        let ts_fn = self.module.add_function(
            "action_string_trim_start",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let ts_entry = self.context.append_basic_block(ts_fn, "entry");
        self.builder.position_at_end(ts_entry);
        let ts_str = ts_fn.get_first_param().unwrap().into_struct_value();
        let ts_len = self
            .builder
            .build_extract_value(ts_str, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let ts_ptr = self
            .builder
            .build_extract_value(ts_str, 1, "ptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ts_loop_bb = self.context.append_basic_block(ts_fn, "ts_loop");
        let ts_done_bb = self.context.append_basic_block(ts_fn, "ts_done");
        let _ = self.builder.build_unconditional_branch(ts_loop_bb);
        self.builder.position_at_end(ts_loop_bb);
        let ts_i = self.builder.build_phi(i64, "ts_i").map_err(llvm_err)?;
        let ts_cp = unsafe {
            self.builder
                .build_gep(i8, ts_ptr, &[ts_i.as_basic_value().into_int_value()], "cp")
                .map_err(llvm_err)
        }?;
        let ts_c = self
            .builder
            .build_load(i8, ts_cp, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let ts_space = i8.const_int(0x20, false);
        let ts_tab = i8.const_int(0x09, false);
        let ts_nl = i8.const_int(0x0a, false);
        let ts_cr = i8.const_int(0x0d, false);
        let ts_is_space = self
            .builder
            .build_int_compare(IntPredicate::EQ, ts_c, ts_space, "is_space")
            .map_err(llvm_err)?;
        let ts_is_tab = self
            .builder
            .build_int_compare(IntPredicate::EQ, ts_c, ts_tab, "is_tab")
            .map_err(llvm_err)?;
        let ts_is_nl = self
            .builder
            .build_int_compare(IntPredicate::EQ, ts_c, ts_nl, "is_nl")
            .map_err(llvm_err)?;
        let ts_is_cr = self
            .builder
            .build_int_compare(IntPredicate::EQ, ts_c, ts_cr, "is_cr")
            .map_err(llvm_err)?;
        let ts_is_ws1 = self
            .builder
            .build_or(ts_is_space, ts_is_tab, "ws1")
            .map_err(llvm_err)?;
        let ts_is_ws2 = self
            .builder
            .build_or(ts_is_nl, ts_is_cr, "ws2")
            .map_err(llvm_err)?;
        let ts_is_ws = self
            .builder
            .build_or(ts_is_ws1, ts_is_ws2, "is_ws")
            .map_err(llvm_err)?;
        let ts_i_next = self
            .builder
            .build_int_add(
                ts_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "ts_inext",
            )
            .map_err(llvm_err)?;
        let ts_at_end = self
            .builder
            .build_int_compare(IntPredicate::SGE, ts_i_next, ts_len, "at_end")
            .map_err(llvm_err)?;
        let ts_stop = self
            .builder
            .build_or(
                ts_at_end,
                self.builder
                    .build_not(ts_is_ws, "not_ws")
                    .map_err(llvm_err)?,
                "stop",
            )
            .map_err(llvm_err)?;
        let ts_loop_block = self.builder.get_insert_block().unwrap();
        ts_i.add_incoming(&[
            (&i64.const_int(0, false), ts_entry),
            (&ts_i_next, ts_loop_block),
        ]);
        let _ = self
            .builder
            .build_conditional_branch(ts_stop, ts_done_bb, ts_loop_bb);
        self.builder.position_at_end(ts_done_bb);
        let ts_start = self.builder.build_phi(i64, "ts_start").map_err(llvm_err)?;
        ts_start.add_incoming(&[(&ts_i.as_basic_value().into_int_value(), ts_loop_block)]);
        // Use start idx as the new start; if start == len, return empty string
        let ts_new_len = self
            .builder
            .build_int_sub(
                ts_len,
                ts_start.as_basic_value().into_int_value(),
                "new_len",
            )
            .map_err(llvm_err)?;
        let ts_nptr = unsafe {
            self.builder
                .build_gep(
                    i8,
                    ts_ptr,
                    &[ts_start.as_basic_value().into_int_value()],
                    "nptr",
                )
                .map_err(llvm_err)
        }?;
        let ts_undef = str_ty.get_undef();
        let ts_r1 = self
            .builder
            .build_insert_value(ts_undef, ts_new_len, 0, "r1")
            .map_err(llvm_err)?;
        let ts_r2 = self
            .builder
            .build_insert_value(ts_r1, ts_nptr, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ts_r2));

        // ---- action_string_trim_end({i64, ptr}) -> {i64, ptr} ----
        let te_fn = self.module.add_function(
            "action_string_trim_end",
            str_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let te_entry = self.context.append_basic_block(te_fn, "entry");
        self.builder.position_at_end(te_entry);
        let te_str = te_fn.get_first_param().unwrap().into_struct_value();
        let te_len = self
            .builder
            .build_extract_value(te_str, 0, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let te_ptr = self
            .builder
            .build_extract_value(te_str, 1, "ptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Start from len-1 and go backwards
        let te_start = self
            .builder
            .build_int_sub(te_len, i64.const_int(1, false), "last")
            .map_err(llvm_err)?;
        let te_loop_bb = self.context.append_basic_block(te_fn, "te_loop");
        let te_done_bb = self.context.append_basic_block(te_fn, "te_done");
        let _ = self.builder.build_unconditional_branch(te_loop_bb);
        self.builder.position_at_end(te_loop_bb);
        let te_i = self.builder.build_phi(i64, "te_i").map_err(llvm_err)?;
        let te_cp = unsafe {
            self.builder
                .build_gep(i8, te_ptr, &[te_i.as_basic_value().into_int_value()], "cp")
                .map_err(llvm_err)
        }?;
        let te_c = self
            .builder
            .build_load(i8, te_cp, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let te_is_space = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                te_c,
                i8.const_int(0x20, false),
                "is_space",
            )
            .map_err(llvm_err)?;
        let te_is_tab = self
            .builder
            .build_int_compare(IntPredicate::EQ, te_c, i8.const_int(0x09, false), "is_tab")
            .map_err(llvm_err)?;
        let te_is_nl = self
            .builder
            .build_int_compare(IntPredicate::EQ, te_c, i8.const_int(0x0a, false), "is_nl")
            .map_err(llvm_err)?;
        let te_is_cr = self
            .builder
            .build_int_compare(IntPredicate::EQ, te_c, i8.const_int(0x0d, false), "is_cr")
            .map_err(llvm_err)?;
        let te_is_ws1 = self
            .builder
            .build_or(te_is_space, te_is_tab, "ws1")
            .map_err(llvm_err)?;
        let te_is_ws2 = self
            .builder
            .build_or(te_is_nl, te_is_cr, "ws2")
            .map_err(llvm_err)?;
        let te_is_ws = self
            .builder
            .build_or(te_is_ws1, te_is_ws2, "is_ws")
            .map_err(llvm_err)?;
        let te_i_next = self
            .builder
            .build_int_sub(
                te_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "te_inext",
            )
            .map_err(llvm_err)?;
        let te_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, te_i_next, i64.const_int(0, false), "neg")
            .map_err(llvm_err)?;
        let te_stop = self
            .builder
            .build_or(
                te_neg,
                self.builder
                    .build_not(te_is_ws, "not_ws")
                    .map_err(llvm_err)?,
                "stop",
            )
            .map_err(llvm_err)?;
        let te_loop_block = self.builder.get_insert_block().unwrap();
        te_i.add_incoming(&[(&te_start, te_entry), (&te_i_next, te_loop_block)]);
        let _ = self
            .builder
            .build_conditional_branch(te_stop, te_done_bb, te_loop_bb);
        self.builder.position_at_end(te_done_bb);
        // te_i is the index of the character we just checked.
        // If it was not whitespace, new_len = te_i + 1.
        // If te_neg was true (all whitespace), te_i = 0 but we need new_len = 0.
        // Check te_neg by checking if te_i_next < 0
        let _te_neg_check = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                te_i.as_basic_value().into_int_value(),
                i64.const_int(0, false),
                "neg_check",
            )
            .map_err(llvm_err)?;
        // Re-check: was the character at te_i whitespace?
        // Easier: just re-load and check
        let te_final_cp = unsafe {
            self.builder
                .build_gep(i8, te_ptr, &[te_i.as_basic_value().into_int_value()], "fcp")
                .map_err(llvm_err)
        }?;
        let te_final_c = self
            .builder
            .build_load(i8, te_final_cp, "fc")
            .map_err(llvm_err)?
            .into_int_value();
        let te_final_ws1 = self
            .builder
            .build_or(
                self.builder
                    .build_int_compare(IntPredicate::EQ, te_final_c, i8.const_int(0x20, false), "")
                    .map_err(llvm_err)?,
                self.builder
                    .build_int_compare(IntPredicate::EQ, te_final_c, i8.const_int(0x09, false), "")
                    .map_err(llvm_err)?,
                "",
            )
            .map_err(llvm_err)?;
        let te_final_ws2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_compare(IntPredicate::EQ, te_final_c, i8.const_int(0x0a, false), "")
                    .map_err(llvm_err)?,
                self.builder
                    .build_int_compare(IntPredicate::EQ, te_final_c, i8.const_int(0x0d, false), "")
                    .map_err(llvm_err)?,
                "",
            )
            .map_err(llvm_err)?;
        let te_final_ws = self
            .builder
            .build_or(te_final_ws1, te_final_ws2, "fws")
            .map_err(llvm_err)?;
        let te_zero_len = i64.const_int(0, false);
        let te_plus1 = self
            .builder
            .build_int_add(
                te_i.as_basic_value().into_int_value(),
                i64.const_int(1, false),
                "plus1",
            )
            .map_err(llvm_err)?;
        let te_new_len = self
            .builder
            .build_select(te_final_ws, te_zero_len, te_plus1, "new_len")
            .map_err(llvm_err)?
            .into_int_value();
        let te_undef = str_ty.get_undef();
        let te_r1 = self
            .builder
            .build_insert_value(te_undef, te_new_len, 0, "r1")
            .map_err(llvm_err)?;
        let te_r2 = self
            .builder
            .build_insert_value(te_r1, te_ptr, 1, "r2")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&te_r2));

        // ---- action_list_tail({ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Returns a new list without the first element (empty list if input is empty)
        let lt_fn = self.module.add_function(
            "action_list_tail",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(lt_fn, "entry");
        self.builder.position_at_end(entry);
        let lt_list = lt_fn.get_first_param().unwrap().into_struct_value();
        let lt_len = self
            .builder
            .build_extract_value(lt_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let _lt_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, lt_len, i64.const_int(0, false), "empty")
            .map_err(llvm_err)?;
        let lt_empty_or_one = self
            .builder
            .build_int_compare(
                IntPredicate::SLE,
                lt_len,
                i64.const_int(1, false),
                "empty_or_one",
            )
            .map_err(llvm_err)?;
        let lt_do = self.context.append_basic_block(lt_fn, "do");
        let lt_empty_bb = self.context.append_basic_block(lt_fn, "empty_ret");
        let _ = self
            .builder
            .build_conditional_branch(lt_empty_or_one, lt_empty_bb, lt_do);
        self.builder.position_at_end(lt_empty_bb);
        // Return empty list
        let cc0 = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let lte_r = cc0.try_as_basic_value().unwrap_basic();
        let _ = self.builder.build_return(Some(&lte_r));
        // Copy elements [1..len)
        self.builder.position_at_end(lt_do);
        let lt_nlen = self
            .builder
            .build_int_sub(lt_len, i64.const_int(1, false), "nlen")
            .map_err(llvm_err)?;
        let cc = self.call_rt("action_list_create", &[lt_nlen.into()])?;
        let lt_new = cc.try_as_basic_value().unwrap_basic().into_struct_value();
        let lt_data = self
            .builder
            .build_extract_value(lt_list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Loop from i=1 to len
        let lt_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lt_new_alloc, lt_new)
            .map_err(llvm_err)?;
        let lt_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(lt_i_alloc, i64.const_int(1, false))
            .map_err(llvm_err)?;
        let lt_loop = self.context.append_basic_block(lt_fn, "loop");
        let lt_body = self.context.append_basic_block(lt_fn, "body");
        let lt_done = self.context.append_basic_block(lt_fn, "done");
        let _ = self.builder.build_unconditional_branch(lt_loop);
        self.builder.position_at_end(lt_loop);
        let lt_i = self
            .builder
            .build_load(i64, lt_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let lt_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lt_i, lt_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lt_cond, lt_body, lt_done);
        self.builder.position_at_end(lt_body);
        let lt_ep = unsafe {
            self.builder
                .build_gep(self.string_type, lt_data, &[lt_i], "ep")
                .map_err(llvm_err)
        }?;
        let lt_fv = self
            .builder
            .build_load(self.string_type, lt_ep, "fv")
            .map_err(llvm_err)?;
        let lt_cur = self
            .builder
            .build_load(self.list_type, lt_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let cc2 = self.call_rt("action_list_push", &[lt_cur.into(), lt_fv.into()])?;
        let lt_nv = cc2.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(lt_new_alloc, lt_nv)
            .map_err(llvm_err)?;
        let lt_ni = self
            .builder
            .build_int_add(lt_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lt_i_alloc, lt_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lt_loop);
        self.builder.position_at_end(lt_done);
        let lt_rv = self
            .builder
            .build_load(self.list_type, lt_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lt_rv));

        // ---- action_list_zip({ptr,i64,i64}, {ptr,i64,i64}) -> {ptr,i64,i64} ----
        let lz_fn = self.module.add_function(
            "action_list_zip",
            list_ty.fn_type(&[list_ty.into(), list_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(lz_fn, "entry");
        self.builder.position_at_end(entry);
        let lz_a = lz_fn.get_first_param().unwrap().into_struct_value();
        let lz_b = lz_fn.get_nth_param(1).unwrap().into_struct_value();
        let lz_alen = self
            .builder
            .build_extract_value(lz_a, 1, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let lz_blen = self
            .builder
            .build_extract_value(lz_b, 1, "blen")
            .map_err(llvm_err)?
            .into_int_value();
        let lz_altb = self
            .builder
            .build_int_compare(IntPredicate::SLT, lz_alen, lz_blen, "altb")
            .map_err(llvm_err)?;
        let lz_min = self
            .builder
            .build_select(lz_altb, lz_alen, lz_blen, "min")
            .map_err(llvm_err)?
            .into_int_value();
        let cc3 = self.call_rt("action_list_create", &[lz_min.into()])?;
        let lz_new = cc3.try_as_basic_value().unwrap_basic().into_struct_value();
        let lz_adata = self
            .builder
            .build_extract_value(lz_a, 0, "adata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lz_bdata = self
            .builder
            .build_extract_value(lz_b, 0, "bdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lz_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_new_alloc, lz_new)
            .map_err(llvm_err)?;
        let lz_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(lz_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let lz_loop = self.context.append_basic_block(lz_fn, "loop");
        let lz_body = self.context.append_basic_block(lz_fn, "body");
        let lz_done = self.context.append_basic_block(lz_fn, "done");
        let _ = self.builder.build_unconditional_branch(lz_loop);
        self.builder.position_at_end(lz_loop);
        let lz_i = self
            .builder
            .build_load(i64, lz_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let lz_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lz_i, lz_min, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lz_cond, lz_body, lz_done);
        self.builder.position_at_end(lz_body);
        let lz_afat = unsafe {
            self.builder
                .build_gep(self.string_type, lz_adata, &[lz_i], "afat")
                .map_err(llvm_err)
        }?;
        let lz_bfat = unsafe {
            self.builder
                .build_gep(self.string_type, lz_bdata, &[lz_i], "bfat")
                .map_err(llvm_err)
        }?;
        let lz_av = self
            .builder
            .build_load(self.string_type, lz_afat, "av")
            .map_err(llvm_err)?;
        let lz_bv = self
            .builder
            .build_load(self.string_type, lz_bfat, "bv")
            .map_err(llvm_err)?;
        // Allocate tuple struct {fat_a, fat_b}
        let lz_tup_ty = self
            .context
            .struct_type(&[self.string_type.into(), self.string_type.into()], false);
        let lz_tup_size = i64.const_int(32, false);
        let lz_tup = self
            .builder
            .build_call(malloc_fn, &[lz_tup_size.into()], "tup")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let lz_tup_a = self
            .builder
            .build_struct_gep(lz_tup_ty, lz_tup, 0, "ta")
            .map_err(llvm_err)?;
        let lz_tup_b = self
            .builder
            .build_struct_gep(lz_tup_ty, lz_tup, 1, "tb")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_tup_a, lz_av)
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_tup_b, lz_bv)
            .map_err(llvm_err)?;
        // Fat struct: tag=5 (Struct), data=ptr to tuple
        let lz_fat_und = self.string_type.get_undef();
        let lz_fat1 = self
            .builder
            .build_insert_value(lz_fat_und, self.i64_ty().const_int(5, false), 0, "tag")
            .map_err(llvm_err)?;
        let lz_fat2 = self
            .builder
            .build_insert_value(lz_fat1, lz_tup, 1, "data")
            .map_err(llvm_err)?;
        // Push into result list
        let lz_cur = self
            .builder
            .build_load(self.list_type, lz_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let lz_push_cc = self.call_rt(
            "action_list_push",
            &[lz_cur.into(), lz_fat2.as_basic_value_enum().into()],
        )?;
        let lz_nv = lz_push_cc.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(lz_new_alloc, lz_nv)
            .map_err(llvm_err)?;
        let lz_ni = self
            .builder
            .build_int_add(lz_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(lz_i_alloc, lz_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lz_loop);
        self.builder.position_at_end(lz_done);
        let lz_rv = self
            .builder
            .build_load(self.list_type, lz_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&lz_rv));

        // ---- action_list_init({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let li_fn = self.module.add_function(
            "action_list_init",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(li_fn, "entry");
        self.builder.position_at_end(entry);
        let li_list = li_fn.get_first_param().unwrap().into_struct_value();
        let li_len = self
            .builder
            .build_extract_value(li_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let li_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, li_len, i64.const_int(0, false), "empty")
            .map_err(llvm_err)?;
        let li_do = self.context.append_basic_block(li_fn, "do");
        let li_empty_bb = self.context.append_basic_block(li_fn, "empty_ret");
        let _ = self
            .builder
            .build_conditional_branch(li_empty, li_empty_bb, li_do);
        self.builder.position_at_end(li_empty_bb);
        let cce = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let li_er = cce.try_as_basic_value().unwrap_basic();
        let _ = self.builder.build_return(Some(&li_er));
        self.builder.position_at_end(li_do);
        let li_nlen = self
            .builder
            .build_int_sub(li_len, i64.const_int(1, false), "nlen")
            .map_err(llvm_err)?;
        let cc = self.call_rt("action_list_create", &[li_nlen.into()])?;
        let li_new_init = cc.try_as_basic_value().unwrap_basic().into_struct_value();
        let li_data = self
            .builder
            .build_extract_value(li_list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let li_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(li_new_alloc, li_new_init)
            .map_err(llvm_err)?;
        let li_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(li_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let li_loop = self.context.append_basic_block(li_fn, "loop");
        let li_body = self.context.append_basic_block(li_fn, "body");
        let li_done = self.context.append_basic_block(li_fn, "done");
        let _ = self.builder.build_unconditional_branch(li_loop);
        self.builder.position_at_end(li_loop);
        let li_i = self
            .builder
            .build_load(i64, li_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let li_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, li_i, li_nlen, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(li_cond, li_body, li_done);
        self.builder.position_at_end(li_body);
        let li_ep = unsafe {
            self.builder
                .build_gep(self.string_type, li_data, &[li_i], "ep")
                .map_err(llvm_err)
        }?;
        let li_fv = self
            .builder
            .build_load(self.string_type, li_ep, "fv")
            .map_err(llvm_err)?;
        let li_cur = self
            .builder
            .build_load(self.list_type, li_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let cc2 = self.call_rt("action_list_push", &[li_cur.into(), li_fv.into()])?;
        let li_nv = cc2.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(li_new_alloc, li_nv)
            .map_err(llvm_err)?;
        let li_ni = self
            .builder
            .build_int_add(li_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(li_i_alloc, li_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(li_loop);
        self.builder.position_at_end(li_done);
        let li_rv = self
            .builder
            .build_load(self.list_type, li_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&li_rv));

        // ---- action_list_last({ptr, i64, i64}) -> {i64, ptr} ----
        let llast_fn = self.module.add_function(
            "action_list_last",
            self.string_type.fn_type(&[list_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(llast_fn, "entry");
        self.builder.position_at_end(entry);
        let ll_list = llast_fn.get_first_param().unwrap().into_struct_value();
        let ll_len = self
            .builder
            .build_extract_value(ll_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let ll_empty = self
            .builder
            .build_int_compare(IntPredicate::EQ, ll_len, i64.const_int(0, false), "empty")
            .map_err(llvm_err)?;
        let ll_has = self.context.append_basic_block(llast_fn, "has");
        let ll_none = self.context.append_basic_block(llast_fn, "none");
        let _ = self
            .builder
            .build_conditional_branch(ll_empty, ll_none, ll_has);
        self.builder.position_at_end(ll_none);
        let ll_none_val = self.string_type.const_zero();
        let _ = self.builder.build_return(Some(&ll_none_val));
        self.builder.position_at_end(ll_has);
        let ll_data = self
            .builder
            .build_extract_value(ll_list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ll_data_i8 = self
            .builder
            .build_pointer_cast(
                ll_data,
                self.context.ptr_type(inkwell::AddressSpace::default()),
                "data_i8",
            )
            .map_err(llvm_err)?;
        let ll_last_idx = self
            .builder
            .build_int_sub(ll_len, i64.const_int(1, false), "last_idx")
            .map_err(llvm_err)?;
        let ll_elem_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, ll_data_i8, &[ll_last_idx], "elem_ptr")
                .map_err(llvm_err)
        }?;
        let ll_val = self
            .builder
            .build_load(self.string_type, ll_elem_ptr, "val")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ll_val));

        // ---- action_string_chars({i64, ptr}) -> {ptr, i64, i64} ----
        let ch_fn = self.module.add_function(
            "action_string_chars",
            list_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(ch_fn, "entry");
        self.builder.position_at_end(entry);
        let ch_s = ch_fn.get_first_param().unwrap().into_struct_value();
        let ch_len = self
            .builder
            .build_extract_value(ch_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_ptr = self
            .builder
            .build_extract_value(ch_s, 1, "sptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc0 = self.call_rt("action_list_create", &[ch_len.into()])?;
        let ch_list_init = cc0.try_as_basic_value().unwrap_basic().into_struct_value();
        let ch_list_alloc = self
            .builder
            .build_alloca(self.list_type, "list_acc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_list_alloc, ch_list_init)
            .map_err(llvm_err)?;
        let ch_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(ch_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let ch_loop = self.context.append_basic_block(ch_fn, "loop");
        let ch_body = self.context.append_basic_block(ch_fn, "body");
        let ch_done = self.context.append_basic_block(ch_fn, "done");
        let _ = self.builder.build_unconditional_branch(ch_loop);
        self.builder.position_at_end(ch_loop);
        let ch_i = self
            .builder
            .build_load(i64, ch_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ch_i, ch_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ch_cond, ch_body, ch_done);
        self.builder.position_at_end(ch_body);
        let ch_cp = unsafe {
            self.builder
                .build_gep(i8, ch_ptr, &[ch_i], "cp")
                .map_err(llvm_err)
        }?;
        let ch_c = self
            .builder
            .build_load(i8, ch_cp, "c")
            .map_err(llvm_err)?
            .into_int_value();
        // Create a 1-byte string from this character
        let ch_salloc = self
            .builder
            .build_call(malloc_fn, &[i64.const_int(1, false).into()], "salloc")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        self.builder
            .build_store(ch_salloc, ch_c)
            .map_err(llvm_err)?;
        let ch_fat = self.string_type.get_undef();
        let ch_fat_tag = self
            .builder
            .build_insert_value(ch_fat, self.i64_ty().const_int(1, false), 0, "tag")
            .map_err(llvm_err)?;
        let ch_fat_val = self
            .builder
            .build_insert_value(ch_fat_tag, ch_salloc, 1, "data")
            .map_err(llvm_err)?;
        let ch_cur = self
            .builder
            .build_load(self.list_type, ch_list_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let ch_push = self.call_rt(
            "action_list_push",
            &[ch_cur.into(), ch_fat_val.as_basic_value_enum().into()],
        )?;
        let ch_new = ch_push.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(ch_list_alloc, ch_new)
            .map_err(llvm_err)?;
        let ch_ni = self
            .builder
            .build_int_add(ch_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_i_alloc, ch_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ch_loop);
        self.builder.position_at_end(ch_done);
        let ch_rv = self
            .builder
            .build_load(self.list_type, ch_list_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ch_rv));

        // ---- action_list_with_index({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let wi_fn = self.module.add_function(
            "action_list_with_index",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(wi_fn, "entry");
        self.builder.position_at_end(entry);
        let wi_list = wi_fn.get_first_param().unwrap().into_struct_value();
        let wi_len = self
            .builder
            .build_extract_value(wi_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let wi_data = self
            .builder
            .build_extract_value(wi_list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc = self.call_rt("action_list_create", &[wi_len.into()])?;
        let wi_new_init = cc.try_as_basic_value().unwrap_basic().into_struct_value();
        let wi_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wi_new_alloc, wi_new_init)
            .map_err(llvm_err)?;
        let wi_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(wi_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let wi_loop = self.context.append_basic_block(wi_fn, "loop");
        let wi_body = self.context.append_basic_block(wi_fn, "body");
        let wi_done = self.context.append_basic_block(wi_fn, "done");
        let _ = self.builder.build_unconditional_branch(wi_loop);
        self.builder.position_at_end(wi_loop);
        let wi_i = self
            .builder
            .build_load(i64, wi_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let wi_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, wi_i, wi_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(wi_cond, wi_body, wi_done);
        self.builder.position_at_end(wi_body);
        let wi_ep = unsafe {
            self.builder
                .build_gep(self.string_type, wi_data, &[wi_i], "ep")
                .map_err(llvm_err)
        }?;
        let wi_ev = self
            .builder
            .build_load(self.string_type, wi_ep, "ev")
            .map_err(llvm_err)?
            .into_struct_value();
        // Create pair tuple {i64 index, fat_elem}
        let wi_tup_ty = self
            .context
            .struct_type(&[i64.into(), self.string_type.into()], false);
        let wi_tup = self
            .builder
            .build_call(malloc_fn, &[i64.const_int(24, false).into()], "tup")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let wi_tup_i = self
            .builder
            .build_struct_gep(wi_tup_ty, wi_tup, 0, "ti")
            .map_err(llvm_err)?;
        let wi_tup_e = self
            .builder
            .build_struct_gep(wi_tup_ty, wi_tup, 1, "te")
            .map_err(llvm_err)?;
        self.builder.build_store(wi_tup_i, wi_i).map_err(llvm_err)?;
        self.builder
            .build_store(wi_tup_e, wi_ev)
            .map_err(llvm_err)?;
        // Wrap in fat struct tag=5 (Struct)
        let wi_fat = self.string_type.get_undef();
        let wi_fat1 = self
            .builder
            .build_insert_value(wi_fat, i64.const_int(5, false), 0, "tag")
            .map_err(llvm_err)?;
        let wi_fat2 = self
            .builder
            .build_insert_value(wi_fat1, wi_tup, 1, "data")
            .map_err(llvm_err)?;
        let wi_cur = self
            .builder
            .build_load(self.list_type, wi_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let cc2 = self.call_rt(
            "action_list_push",
            &[wi_cur.into(), wi_fat2.as_basic_value_enum().into()],
        )?;
        let wi_nv = cc2.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(wi_new_alloc, wi_nv)
            .map_err(llvm_err)?;
        let wi_ni = self
            .builder
            .build_int_add(wi_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wi_i_alloc, wi_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(wi_loop);
        self.builder.position_at_end(wi_done);
        let wi_rv = self
            .builder
            .build_load(self.list_type, wi_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&wi_rv));

        // ---- action_list_unique({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let unq_fn = self.module.add_function(
            "action_list_unique",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(unq_fn, "entry");
        self.builder.position_at_end(entry);
        let unq_list = unq_fn.get_first_param().unwrap().into_struct_value();
        let unq_len = self
            .builder
            .build_extract_value(unq_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let unq_data = self
            .builder
            .build_extract_value(unq_list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc3 = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let unq_new_init = cc3.try_as_basic_value().unwrap_basic().into_struct_value();
        let unq_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(unq_new_alloc, unq_new_init)
            .map_err(llvm_err)?;
        let unq_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(unq_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let unq_loop = self.context.append_basic_block(unq_fn, "loop");
        let unq_body = self.context.append_basic_block(unq_fn, "body");
        let unq_done = self.context.append_basic_block(unq_fn, "done");
        let _ = self.builder.build_unconditional_branch(unq_loop);
        self.builder.position_at_end(unq_loop);
        let unq_i = self
            .builder
            .build_load(i64, unq_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let unq_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, unq_i, unq_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(unq_cond, unq_body, unq_done);
        self.builder.position_at_end(unq_body);
        let unq_ep = unsafe {
            self.builder
                .build_gep(self.string_type, unq_data, &[unq_i], "ep")
                .map_err(llvm_err)
        }?;
        let unq_ev = self
            .builder
            .build_load(self.string_type, unq_ep, "ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let unq_cur = self
            .builder
            .build_load(self.list_type, unq_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        // Check if already in result: call action_list_contains
        let cc4 = self.call_rt(
            "action_list_contains",
            &[unq_cur.into(), unq_ev.as_basic_value_enum().into()],
        )?;
        let unq_found = cc4.try_as_basic_value().unwrap_basic().into_int_value();
        let unq_push_bb = self.context.append_basic_block(unq_fn, "push");
        let unq_skip_bb = self.context.append_basic_block(unq_fn, "skip");
        let _ = self
            .builder
            .build_conditional_branch(unq_found, unq_skip_bb, unq_push_bb);
        self.builder.position_at_end(unq_push_bb);
        let cc5 = self.call_rt(
            "action_list_push",
            &[unq_cur.into(), unq_ev.as_basic_value_enum().into()],
        )?;
        let unq_nv = cc5.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(unq_new_alloc, unq_nv)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(unq_skip_bb);
        self.builder.position_at_end(unq_skip_bb);
        let unq_ni = self
            .builder
            .build_int_add(unq_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(unq_i_alloc, unq_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(unq_loop);
        self.builder.position_at_end(unq_done);
        let unq_rv = self
            .builder
            .build_load(self.list_type, unq_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&unq_rv));

        // ---- action_list_slice({ptr, i64, i64}, i64 start, i64 end) -> {ptr, i64, i64} ----
        let slc_fn = self.module.add_function(
            "action_list_slice",
            list_ty.fn_type(&[list_ty.into(), i64.into(), i64.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(slc_fn, "entry");
        self.builder.position_at_end(entry);
        let slc_list = slc_fn.get_first_param().unwrap().into_struct_value();
        let slc_start = slc_fn.get_nth_param(1).unwrap().into_int_value();
        let slc_end = slc_fn.get_nth_param(2).unwrap().into_int_value();
        let slc_len = self
            .builder
            .build_extract_value(slc_list, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_data = self
            .builder
            .build_extract_value(slc_list, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        // Clamp start to [0, len]
        let slc_s_neg = self
            .builder
            .build_int_compare(
                IntPredicate::SLT,
                slc_start,
                i64.const_int(0, false),
                "sneg",
            )
            .map_err(llvm_err)?;
        let slc_s_clamp = self
            .builder
            .build_select(slc_s_neg, i64.const_int(0, false), slc_start, "sclamp")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_s_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_s_clamp, slc_len, "sgt")
            .map_err(llvm_err)?;
        let slc_s_final = self
            .builder
            .build_select(slc_s_gt, slc_len, slc_s_clamp, "sfinal")
            .map_err(llvm_err)?
            .into_int_value();
        // Clamp end
        let slc_e_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_end, i64.const_int(0, false), "eneg")
            .map_err(llvm_err)?;
        let slc_e_clamp = self
            .builder
            .build_select(slc_e_neg, i64.const_int(0, false), slc_end, "eclamp")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_e_gt = self
            .builder
            .build_int_compare(IntPredicate::SGT, slc_e_clamp, slc_len, "egt")
            .map_err(llvm_err)?;
        let slc_e_final = self
            .builder
            .build_select(slc_e_gt, slc_len, slc_e_clamp, "efinal")
            .map_err(llvm_err)?
            .into_int_value();
        // Compute result length
        let slc_rlen = self
            .builder
            .build_int_sub(slc_e_final, slc_s_final, "rlen")
            .map_err(llvm_err)?;
        let slc_rlen_neg = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_rlen, i64.const_int(0, false), "rneg")
            .map_err(llvm_err)?;
        let slc_rlen_final = self
            .builder
            .build_select(slc_rlen_neg, i64.const_int(0, false), slc_rlen, "rlenf")
            .map_err(llvm_err)?
            .into_int_value();
        let cc6 = self.call_rt("action_list_create", &[slc_rlen_final.into()])?;
        let slc_new_init = cc6.try_as_basic_value().unwrap_basic().into_struct_value();
        let slc_new_alloc = self
            .builder
            .build_alloca(self.list_type, "newacc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slc_new_alloc, slc_new_init)
            .map_err(llvm_err)?;
        let slc_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(slc_i_alloc, slc_s_final)
            .map_err(llvm_err)?;
        let slc_loop = self.context.append_basic_block(slc_fn, "loop");
        let slc_body = self.context.append_basic_block(slc_fn, "body");
        let slc_done = self.context.append_basic_block(slc_fn, "done");
        let _ = self.builder.build_unconditional_branch(slc_loop);
        self.builder.position_at_end(slc_loop);
        let slc_i = self
            .builder
            .build_load(i64, slc_i_alloc, "i")
            .map_err(llvm_err)?
            .into_int_value();
        let slc_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, slc_i, slc_e_final, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(slc_cond, slc_body, slc_done);
        self.builder.position_at_end(slc_body);
        let slc_ep = unsafe {
            self.builder
                .build_gep(self.string_type, slc_data, &[slc_i], "ep")
                .map_err(llvm_err)
        }?;
        let slc_ev = self
            .builder
            .build_load(self.string_type, slc_ep, "ev")
            .map_err(llvm_err)?;
        let slc_cur = self
            .builder
            .build_load(self.list_type, slc_new_alloc, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let cc7 = self.call_rt("action_list_push", &[slc_cur.into(), slc_ev.into()])?;
        let slc_nv = cc7.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(slc_new_alloc, slc_nv)
            .map_err(llvm_err)?;
        let slc_ni = self
            .builder
            .build_int_add(slc_i, i64.const_int(1, false), "ni")
            .map_err(llvm_err)?;
        self.builder
            .build_store(slc_i_alloc, slc_ni)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(slc_loop);
        self.builder.position_at_end(slc_done);
        let slc_rv = self
            .builder
            .build_load(self.list_type, slc_new_alloc, "rv")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&slc_rv));

        // ---- action_string_split_lines({i64, ptr}) -> {ptr, i64, i64} ----
        let sl_fn = self.module.add_function(
            "action_string_split_lines",
            list_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(sl_fn, "entry");
        self.builder.position_at_end(entry);
        let sl_s = sl_fn.get_first_param().unwrap().into_struct_value();
        let sl_len = self
            .builder
            .build_extract_value(sl_s, 0, "slen")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_ptr = self
            .builder
            .build_extract_value(sl_s, 1, "sptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let cc4 = self.call_rt("action_list_create", &[i64.const_int(0, false).into()])?;
        let sl_list_init = cc4.try_as_basic_value().unwrap_basic().into_struct_value();
        // Use alloca to accumulate list across loop iterations
        let sl_list_alloc = self
            .builder
            .build_alloca(self.list_type, "list_acc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_list_alloc, sl_list_init)
            .map_err(llvm_err)?;
        // Scan through string, splitting on '\n'
        let sl_start_alloc = self.builder.build_alloca(i64, "start").map_err(llvm_err)?;
        let sl_i_alloc = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(sl_start_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_i_alloc, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sl_loop = self.context.append_basic_block(sl_fn, "loop");
        let sl_body_bb = self.context.append_basic_block(sl_fn, "body");
        let sl_done = self.context.append_basic_block(sl_fn, "done");
        let _ = self.builder.build_unconditional_branch(sl_loop);
        self.builder.position_at_end(sl_loop);
        let sl_i = self
            .builder
            .build_load(i64, sl_i_alloc, "sl_i")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_cond = self
            .builder
            .build_int_compare(IntPredicate::SLE, sl_i, sl_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sl_cond, sl_body_bb, sl_done);
        self.builder.position_at_end(sl_body_bb);
        // Check if at end or char is '\n'
        let sl_at_end = self
            .builder
            .build_int_compare(IntPredicate::EQ, sl_i, sl_len, "atend")
            .map_err(llvm_err)?;
        let sl_cp = unsafe {
            self.builder
                .build_gep(i8, sl_ptr, &[sl_i], "cp")
                .map_err(llvm_err)
        }?;
        let sl_c = self
            .builder
            .build_load(i8, sl_cp, "c")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_is_nl = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sl_c,
                i8.const_int(b'\n' as u64, false),
                "isnl",
            )
            .map_err(llvm_err)?;
        let sl_cr = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sl_c,
                i8.const_int(b'\r' as u64, false),
                "iscr",
            )
            .map_err(llvm_err)?;
        let sl_split = self
            .builder
            .build_or(
                sl_at_end,
                self.builder
                    .build_or(sl_is_nl, sl_cr, "")
                    .map_err(llvm_err)?,
                "split",
            )
            .map_err(llvm_err)?;
        let sl_cont = self.context.append_basic_block(sl_fn, "cont");
        let sl_extract = self.context.append_basic_block(sl_fn, "extract");
        let _ = self
            .builder
            .build_conditional_branch(sl_split, sl_extract, sl_cont);
        // Extract line from start to i
        self.builder.position_at_end(sl_extract);
        let sl_start = self
            .builder
            .build_load(i64, sl_start_alloc, "slstart")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_seg_len = self
            .builder
            .build_int_sub(sl_i, sl_start, "seg_len")
            .map_err(llvm_err)?;
        let sl_seg_data = unsafe {
            self.builder
                .build_gep(i8, sl_ptr, &[sl_start], "segp")
                .map_err(llvm_err)
        }?;
        // Skip \r if next char is \n
        let sl_next_i = self
            .builder
            .build_int_add(sl_i, i64.const_int(1, false), "nexti")
            .map_err(llvm_err)?;
        // Create string for this segment: malloc + memcpy
        let sl_salloc = self
            .builder
            .build_call(malloc_fn, &[sl_seg_len.into()], "seg")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let _ = self
            .builder
            .build_call(
                memcpy_fn,
                &[sl_salloc.into(), sl_seg_data.into(), sl_seg_len.into()],
                "",
            )
            .map_err(llvm_err)?;
        let sl_fat = self.string_type.get_undef();
        let sl_fat_tag = self
            .builder
            .build_insert_value(sl_fat, self.i64_ty().const_int(1, false), 0, "tag")
            .map_err(llvm_err)?;
        let sl_fat_val = self
            .builder
            .build_insert_value(sl_fat_tag, sl_salloc, 1, "data")
            .map_err(llvm_err)?;
        let sl_cur_list = self
            .builder
            .build_load(self.list_type, sl_list_alloc, "cur_list")
            .map_err(llvm_err)?
            .into_struct_value();
        let sl_push_cc = self.call_rt(
            "action_list_push",
            &[sl_cur_list.into(), sl_fat_val.as_basic_value_enum().into()],
        )?;
        let sl_new_list = sl_push_cc.try_as_basic_value().unwrap_basic();
        self.builder
            .build_store(sl_list_alloc, sl_new_list)
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_start_alloc, sl_next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sl_cont);
        // Continue scanning
        self.builder.position_at_end(sl_cont);
        let sl_i2 = self
            .builder
            .build_load(i64, sl_i_alloc, "i2")
            .map_err(llvm_err)?
            .into_int_value();
        let sl_i_next = self
            .builder
            .build_int_add(sl_i2, i64.const_int(1, false), "inext")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sl_i_alloc, sl_i_next)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sl_loop);
        self.builder.position_at_end(sl_done);
        let sl_result = self
            .builder
            .build_load(self.list_type, sl_list_alloc, "result")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sl_result));

        // ---- action_string_index_of({i64, ptr}, {i64, ptr}) -> i64 (returns -1 if not found) ----
        let sio_fn = self.module.add_function(
            "action_string_index_of",
            i64.fn_type(&[str_ty.into(), str_ty.into()], false),
            None,
        );
        let entry = self.context.append_basic_block(sio_fn, "entry");
        self.builder.position_at_end(entry);
        let sio_hay = sio_fn.get_first_param().unwrap().into_struct_value();
        let sio_nee = sio_fn.get_nth_param(1).unwrap().into_struct_value();
        let sio_hlen = self
            .builder
            .build_extract_value(sio_hay, 0, "hlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sio_hptr = self
            .builder
            .build_extract_value(sio_hay, 1, "hptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sio_nlen = self
            .builder
            .build_extract_value(sio_nee, 0, "nlen")
            .map_err(llvm_err)?
            .into_int_value();
        let sio_nptr = self
            .builder
            .build_extract_value(sio_nee, 1, "nptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        // If needle empty, return 0
        let sio_nempty = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sio_nlen,
                i64.const_int(0, false),
                "nempty",
            )
            .map_err(llvm_err)?;
        let sio_nok = self
            .builder
            .build_int_compare(IntPredicate::SLE, sio_nlen, sio_hlen, "nok")
            .map_err(llvm_err)?;
        let _sio_can = self
            .builder
            .build_and(
                sio_nok,
                self.builder.build_not(sio_nempty, "").map_err(llvm_err)?,
                "",
            )
            .map_err(llvm_err)?;
        let sio_max = self
            .builder
            .build_int_sub(sio_hlen, sio_nlen, "max")
            .map_err(llvm_err)?;
        // Outer loop
        let sio_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(sio_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sio_oloop = self.context.append_basic_block(sio_fn, "oloop");
        let sio_obody = self.context.append_basic_block(sio_fn, "obody");
        let sio_notfound = self.context.append_basic_block(sio_fn, "notfound");
        let _ = self.builder.build_unconditional_branch(sio_oloop);
        self.builder.position_at_end(sio_oloop);
        let sio_iv = self
            .builder
            .build_load(i64, sio_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let sio_cond = self
            .builder
            .build_int_compare(IntPredicate::SLE, sio_iv, sio_max, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sio_cond, sio_obody, sio_notfound);
        self.builder.position_at_end(sio_obody);
        let sio_hp = unsafe {
            self.builder
                .build_gep(i8, sio_hptr, &[sio_iv], "hp")
                .map_err(llvm_err)
        }?;
        let sio_eq = self
            .builder
            .build_call(
                memcmp_fn,
                &[sio_hp.into(), sio_nptr.into(), sio_nlen.into()],
                "eq",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let sio_match = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                sio_eq,
                self.i32_ty().const_int(0, false),
                "match",
            )
            .map_err(llvm_err)?;
        let sio_match_bb = self.context.append_basic_block(sio_fn, "match");
        let sio_next_bb = self.context.append_basic_block(sio_fn, "next");
        let _ = self
            .builder
            .build_conditional_branch(sio_match, sio_match_bb, sio_next_bb);
        self.builder.position_at_end(sio_match_bb);
        let _ = self.builder.build_return(Some(&sio_iv));
        self.builder.position_at_end(sio_next_bb);
        let sio_next_i = self
            .builder
            .build_int_add(sio_iv, i64.const_int(1, false), "nexti")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sio_i, sio_next_i)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sio_oloop);
        self.builder.position_at_end(sio_notfound);
        let _ = self
            .builder
            .build_return(Some(&i64.const_int(-1i64 as u64, true)));

        // ---- action_list_flatten({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let fl_fn = self.module.add_function(
            "action_list_flatten",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let fl_entry = self.context.append_basic_block(fl_fn, "entry");
        self.builder.position_at_end(fl_entry);
        let fl_input = fl_fn.get_first_param().unwrap().into_struct_value();
        let fl_data = self
            .builder
            .build_extract_value(fl_input, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_len = self
            .builder
            .build_extract_value(fl_input, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_result = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let fl_rb = fl_result.try_as_basic_value().unwrap_basic();
        let fl_ra = self
            .builder
            .build_alloca(self.list_type, "fl_res")
            .map_err(llvm_err)?;
        self.builder.build_store(fl_ra, fl_rb).map_err(llvm_err)?;
        let fl_oi = self.builder.build_alloca(i64, "fl_oi").map_err(llvm_err)?;
        self.builder
            .build_store(fl_oi, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let fl_oloop = self.context.append_basic_block(fl_fn, "oloop");
        let fl_obody = self.context.append_basic_block(fl_fn, "obody");
        let fl_odone = self.context.append_basic_block(fl_fn, "odone");
        let _ = self.builder.build_unconditional_branch(fl_oloop);
        self.builder.position_at_end(fl_oloop);
        let fl_oi_val = self
            .builder
            .build_load(i64, fl_oi, "oi")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_ocond = self
            .builder
            .build_int_compare(IntPredicate::SLT, fl_oi_val, fl_len, "ocond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fl_ocond, fl_obody, fl_odone);
        self.builder.position_at_end(fl_obody);
        let fl_elem_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, fl_data, &[fl_oi_val], "ep")
                .map_err(llvm_err)
        }?;
        let fl_elem = self
            .builder
            .build_load(self.string_type, fl_elem_ptr, "elem")
            .map_err(llvm_err)?
            .into_struct_value();
        let fl_elem_tag = self
            .builder
            .build_extract_value(fl_elem, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_is_list = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                fl_elem_tag,
                i64.const_int(6, false),
                "islist",
            )
            .map_err(llvm_err)?;
        let fl_push_flat = self.context.append_basic_block(fl_fn, "push_flat");
        let fl_push_direct = self.context.append_basic_block(fl_fn, "push_direct");
        let fl_push_next = self.context.append_basic_block(fl_fn, "push_next");
        let _ = self
            .builder
            .build_conditional_branch(fl_is_list, fl_push_flat, fl_push_direct);
        self.builder.position_at_end(fl_push_flat);
        let fl_edata = self
            .builder
            .build_extract_value(fl_elem, 1, "edata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_inner = self
            .builder
            .build_load(self.list_type, fl_edata, "inner")
            .map_err(llvm_err)?
            .into_struct_value();
        let fl_idata = self
            .builder
            .build_extract_value(fl_inner, 0, "idata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let fl_ilen = self
            .builder
            .build_extract_value(fl_inner, 1, "ilen")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_ii = self.builder.build_alloca(i64, "fl_ii").map_err(llvm_err)?;
        self.builder
            .build_store(fl_ii, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let fl_iloop = self.context.append_basic_block(fl_fn, "iloop");
        let fl_ibody = self.context.append_basic_block(fl_fn, "ibody");
        let fl_idone = self.context.append_basic_block(fl_fn, "idone");
        let _ = self.builder.build_unconditional_branch(fl_iloop);
        self.builder.position_at_end(fl_iloop);
        let fl_ii_val = self
            .builder
            .build_load(i64, fl_ii, "ii")
            .map_err(llvm_err)?
            .into_int_value();
        let fl_icond = self
            .builder
            .build_int_compare(IntPredicate::SLT, fl_ii_val, fl_ilen, "icond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(fl_icond, fl_ibody, fl_idone);
        self.builder.position_at_end(fl_ibody);
        let fl_ie_ptr = unsafe {
            self.builder
                .build_gep(self.string_type, fl_idata, &[fl_ii_val], "iep")
                .map_err(llvm_err)
        }?;
        let fl_ie = self
            .builder
            .build_load(self.string_type, fl_ie_ptr, "ie")
            .map_err(llvm_err)?
            .into_struct_value();
        let fl_cl = self
            .builder
            .build_load(self.list_type, fl_ra, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let fl_ps = self.call_rt(
            "action_list_push",
            &[fl_cl.into(), fl_ie.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(fl_ra, fl_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let fl_ii_inc = self
            .builder
            .build_int_add(fl_ii_val, i64.const_int(1, false), "iiinc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(fl_ii, fl_ii_inc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fl_iloop);
        self.builder.position_at_end(fl_idone);
        let _ = self.builder.build_unconditional_branch(fl_push_next);
        self.builder.position_at_end(fl_push_direct);
        let fl_cl2 = self
            .builder
            .build_load(self.list_type, fl_ra, "cl2")
            .map_err(llvm_err)?
            .into_struct_value();
        let fl_ps2 = self.call_rt(
            "action_list_push",
            &[fl_cl2.into(), fl_elem.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(fl_ra, fl_ps2.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fl_push_next);
        self.builder.position_at_end(fl_push_next);
        let fl_oi_inc = self
            .builder
            .build_int_add(fl_oi_val, i64.const_int(1, false), "oiinc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(fl_oi, fl_oi_inc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(fl_oloop);
        self.builder.position_at_end(fl_odone);
        let fl_res = self
            .builder
            .build_load(self.list_type, fl_ra, "fl_res")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&fl_res));

        // ---- action_list_split_at({ptr, i64, i64}, i64) -> {ptr, i64, i64} ----
        let sa_fn = self.module.add_function(
            "action_list_split_at",
            list_ty.fn_type(&[list_ty.into(), i64.into()], false),
            None,
        );
        let sa_entry = self.context.append_basic_block(sa_fn, "entry");
        self.builder.position_at_end(sa_entry);
        let sa_in = sa_fn.get_first_param().unwrap().into_struct_value();
        let sa_idx = sa_fn.get_nth_param(1).unwrap().into_int_value();
        let sa_data = self
            .builder
            .build_extract_value(sa_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sa_len = self
            .builder
            .build_extract_value(sa_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let sa_clamped = self
            .builder
            .build_int_compare(IntPredicate::SLT, sa_idx, i64.const_int(0, false), "cl")
            .map_err(llvm_err)?;
        let sa_idx0 = self
            .builder
            .build_select(sa_clamped, i64.const_int(0, false), sa_idx, "idx0")
            .map_err(llvm_err)?
            .into_int_value();
        let sa_cl2 = self
            .builder
            .build_int_compare(IntPredicate::SGT, sa_idx0, sa_len, "cl2")
            .map_err(llvm_err)?;
        let sa_idx_safe = self
            .builder
            .build_select(sa_cl2, sa_len, sa_idx0, "idx_safe")
            .map_err(llvm_err)?
            .into_int_value();
        let sa_r1 = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let sa_r1v = sa_r1.try_as_basic_value().unwrap_basic();
        let sa_a1 = self
            .builder
            .build_alloca(self.list_type, "sa_a1")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_a1, sa_r1v).map_err(llvm_err)?;
        let sa_r2 = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let sa_r2v = sa_r2.try_as_basic_value().unwrap_basic();
        let sa_a2 = self
            .builder
            .build_alloca(self.list_type, "sa_a2")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_a2, sa_r2v).map_err(llvm_err)?;
        let sa_i = self.builder.build_alloca(i64, "sa_i").map_err(llvm_err)?;
        self.builder
            .build_store(sa_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sa_loop = self.context.append_basic_block(sa_fn, "loop");
        let sa_body = self.context.append_basic_block(sa_fn, "body");
        let sa_done = self.context.append_basic_block(sa_fn, "done");
        let _ = self.builder.build_unconditional_branch(sa_loop);
        self.builder.position_at_end(sa_loop);
        let sa_iv = self
            .builder
            .build_load(i64, sa_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let sa_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, sa_iv, sa_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sa_cond, sa_body, sa_done);
        self.builder.position_at_end(sa_body);
        let sa_ep = unsafe {
            self.builder
                .build_gep(self.string_type, sa_data, &[sa_iv], "ep")
                .map_err(llvm_err)
        }?;
        let sa_ev = self
            .builder
            .build_load(self.string_type, sa_ep, "ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let sa_before = self
            .builder
            .build_int_compare(IntPredicate::SLT, sa_iv, sa_idx_safe, "before")
            .map_err(llvm_err)?;
        let sa_l1 = self
            .builder
            .build_load(self.list_type, sa_a1, "l1")
            .map_err(llvm_err)?
            .into_struct_value();
        let sa_l2 = self
            .builder
            .build_load(self.list_type, sa_a2, "l2")
            .map_err(llvm_err)?
            .into_struct_value();
        let sa_ps1 = self.call_rt(
            "action_list_push",
            &[sa_l1.into(), sa_ev.as_basic_value_enum().into()],
        )?;
        let sa_ps2 = self.call_rt(
            "action_list_push",
            &[sa_l2.into(), sa_ev.as_basic_value_enum().into()],
        )?;
        let sa_l1_sel = self
            .builder
            .build_select(
                sa_before,
                sa_ps1.try_as_basic_value().unwrap_basic(),
                sa_l1.into(),
                "l1s",
            )
            .map_err(llvm_err)?;
        let sa_l2_sel = self
            .builder
            .build_select(
                sa_before,
                sa_l2.into(),
                sa_ps2.try_as_basic_value().unwrap_basic(),
                "l2s",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_a1, sa_l1_sel)
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_a2, sa_l2_sel)
            .map_err(llvm_err)?;
        let sa_inc = self
            .builder
            .build_int_add(sa_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_i, sa_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sa_loop);
        self.builder.position_at_end(sa_done);
        // Return as list of 2 lists
        let sa_malloc = self
            .builder
            .build_call(malloc_fn, &[i64.const_int(16, false).into()], "sa_m")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let sa_l1f = self
            .builder
            .build_load(self.list_type, sa_a1, "l1f")
            .map_err(llvm_err)?
            .into_struct_value();
        let sa_fat1 = self.string_type.get_undef();
        let sa_fat1t = self
            .builder
            .build_insert_value(sa_fat1, i64.const_int(6, false), 0, "t1")
            .map_err(llvm_err)?;
        let sa_l1p = self
            .builder
            .build_alloca(self.list_type, "l1p")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_l1p, sa_l1f).map_err(llvm_err)?;
        let sa_fat1v = self
            .builder
            .build_insert_value(sa_fat1t, sa_l1p, 1, "v1")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_malloc, sa_fat1v)
            .map_err(llvm_err)?;
        let sa_slot2 = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    sa_malloc,
                    &[i64.const_int(1, false)],
                    "s2",
                )
                .map_err(llvm_err)
        }?;
        let sa_l2f = self
            .builder
            .build_load(self.list_type, sa_a2, "l2f")
            .map_err(llvm_err)?
            .into_struct_value();
        let sa_fat2 = self.string_type.get_undef();
        let sa_fat2t = self
            .builder
            .build_insert_value(sa_fat2, i64.const_int(6, false), 0, "t2")
            .map_err(llvm_err)?;
        let sa_l2p = self
            .builder
            .build_alloca(self.list_type, "l2p")
            .map_err(llvm_err)?;
        self.builder.build_store(sa_l2p, sa_l2f).map_err(llvm_err)?;
        let sa_fat2v = self
            .builder
            .build_insert_value(sa_fat2t, sa_l2p, 1, "v2")
            .map_err(llvm_err)?;
        self.builder
            .build_store(sa_slot2, sa_fat2v)
            .map_err(llvm_err)?;
        let sa_rt = self.list_type.get_undef();
        let sa_rtd = self
            .builder
            .build_insert_value(sa_rt, sa_malloc, 0, "d")
            .map_err(llvm_err)?;
        let sa_rtl = self
            .builder
            .build_insert_value(sa_rtd, i64.const_int(2, false), 1, "l")
            .map_err(llvm_err)?;
        let sa_rtc = self
            .builder
            .build_insert_value(sa_rtl, i64.const_int(2, false), 2, "c")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sa_rtc));

        // ---- action_list_chunks({ptr, i64, i64}, i64 chunk_size) -> {ptr, i64, i64} ----
        let ch_fn = self.module.add_function(
            "action_list_chunks",
            list_ty.fn_type(&[list_ty.into(), i64.into()], false),
            None,
        );
        let ch_entry = self.context.append_basic_block(ch_fn, "entry");
        self.builder.position_at_end(ch_entry);
        let ch_in = ch_fn.get_first_param().unwrap().into_struct_value();
        let ch_csize = ch_fn.get_nth_param(1).unwrap().into_int_value();
        let ch_data = self
            .builder
            .build_extract_value(ch_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let ch_len = self
            .builder
            .build_extract_value(ch_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_cz = self
            .builder
            .build_int_compare(IntPredicate::SLT, ch_csize, i64.const_int(1, false), "cz")
            .map_err(llvm_err)?;
        let ch_csafe = self
            .builder
            .build_select(ch_cz, i64.const_int(1, false), ch_csize, "csafe")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let ch_resv = ch_res.try_as_basic_value().unwrap_basic();
        let ch_ra = self
            .builder
            .build_alloca(self.list_type, "ch_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(ch_ra, ch_resv).map_err(llvm_err)?;
        let ch_i = self.builder.build_alloca(i64, "ch_i").map_err(llvm_err)?;
        self.builder
            .build_store(ch_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let ch_loop = self.context.append_basic_block(ch_fn, "loop");
        let ch_body = self.context.append_basic_block(ch_fn, "body");
        let ch_done = self.context.append_basic_block(ch_fn, "done");
        let _ = self.builder.build_unconditional_branch(ch_loop);
        self.builder.position_at_end(ch_loop);
        let ch_iv = self
            .builder
            .build_load(i64, ch_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ch_iv, ch_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ch_cond, ch_body, ch_done);
        self.builder.position_at_end(ch_body);
        let ch_subl = self.call_rt("action_list_create", &[ch_csafe.into()])?;
        let ch_sublv = ch_subl.try_as_basic_value().unwrap_basic();
        let ch_sa = self
            .builder
            .build_alloca(self.list_type, "ch_sa")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_sa, ch_sublv)
            .map_err(llvm_err)?;
        let ch_j = self.builder.build_alloca(i64, "ch_j").map_err(llvm_err)?;
        self.builder
            .build_store(ch_j, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let ch_iloop = self.context.append_basic_block(ch_fn, "iloop");
        let ch_ibody = self.context.append_basic_block(ch_fn, "ibody");
        let ch_idone = self.context.append_basic_block(ch_fn, "idone");
        let _ = self.builder.build_unconditional_branch(ch_iloop);
        self.builder.position_at_end(ch_iloop);
        let ch_jv = self
            .builder
            .build_load(i64, ch_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_jc = self
            .builder
            .build_int_compare(IntPredicate::SLT, ch_jv, ch_csafe, "jc")
            .map_err(llvm_err)?;
        let ch_end = self
            .builder
            .build_int_compare(IntPredicate::SGE, ch_iv, ch_len, "end")
            .map_err(llvm_err)?;
        let ch_ic = self
            .builder
            .build_and(
                ch_jc,
                self.builder.build_not(ch_end, "").map_err(llvm_err)?,
                "ic",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(ch_ic, ch_ibody, ch_idone);
        self.builder.position_at_end(ch_ibody);
        let ch_cur_i = self
            .builder
            .build_load(i64, ch_i, "cur_i")
            .map_err(llvm_err)?
            .into_int_value();
        let ch_ep = unsafe {
            self.builder
                .build_gep(self.string_type, ch_data, &[ch_cur_i], "ep")
                .map_err(llvm_err)
        }?;
        let ch_ev = self
            .builder
            .build_load(self.string_type, ch_ep, "ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let ch_cl = self
            .builder
            .build_load(self.list_type, ch_sa, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let ch_ps = self.call_rt(
            "action_list_push",
            &[ch_cl.into(), ch_ev.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(ch_sa, ch_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let ch_ivi = self
            .builder
            .build_int_add(ch_cur_i, i64.const_int(1, false), "ivi")
            .map_err(llvm_err)?;
        self.builder.build_store(ch_i, ch_ivi).map_err(llvm_err)?;
        let ch_jvi = self
            .builder
            .build_int_add(ch_jv, i64.const_int(1, false), "jvi")
            .map_err(llvm_err)?;
        self.builder.build_store(ch_j, ch_jvi).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ch_iloop);
        self.builder.position_at_end(ch_idone);
        let ch_subl_fat = self.string_type.get_undef();
        let ch_sublft = self
            .builder
            .build_insert_value(ch_subl_fat, i64.const_int(6, false), 0, "st")
            .map_err(llvm_err)?;
        let ch_subl_l = self
            .builder
            .build_load(self.list_type, ch_sa, "sl")
            .map_err(llvm_err)?
            .into_struct_value();
        let ch_sp = self
            .builder
            .build_alloca(self.list_type, "ch_sp")
            .map_err(llvm_err)?;
        self.builder
            .build_store(ch_sp, ch_subl_l)
            .map_err(llvm_err)?;
        let ch_sublfv = self
            .builder
            .build_insert_value(ch_sublft, ch_sp, 1, "sv")
            .map_err(llvm_err)?;
        let ch_rl = self
            .builder
            .build_load(self.list_type, ch_ra, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let ch_rps = self.call_rt(
            "action_list_push",
            &[ch_rl.into(), ch_sublfv.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(ch_ra, ch_rps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(ch_loop);
        self.builder.position_at_end(ch_done);
        let ch_rt = self
            .builder
            .build_load(self.list_type, ch_ra, "ch_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&ch_rt));

        // ---- action_list_windows({ptr, i64, i64}, i64 win_size) -> {ptr, i64, i64} ----
        let wn_fn = self.module.add_function(
            "action_list_windows",
            list_ty.fn_type(&[list_ty.into(), i64.into()], false),
            None,
        );
        let wn_entry = self.context.append_basic_block(wn_fn, "entry");
        self.builder.position_at_end(wn_entry);
        let wn_in = wn_fn.get_first_param().unwrap().into_struct_value();
        let wn_wsize = wn_fn.get_nth_param(1).unwrap().into_int_value();
        let wn_data = self
            .builder
            .build_extract_value(wn_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let wn_len = self
            .builder
            .build_extract_value(wn_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_wz = self
            .builder
            .build_int_compare(IntPredicate::SLT, wn_wsize, i64.const_int(1, false), "wz")
            .map_err(llvm_err)?;
        let wn_wsafe = self
            .builder
            .build_select(wn_wz, i64.const_int(1, false), wn_wsize, "wsafe")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_tmp = self
            .builder
            .build_int_sub(wn_len, wn_wsafe, "tmp")
            .map_err(llvm_err)?;
        let wn_nw1 = self
            .builder
            .build_int_add(wn_tmp, i64.const_int(1, false), "nw1")
            .map_err(llvm_err)?;
        let wn_nz = self
            .builder
            .build_int_compare(IntPredicate::SLT, wn_nw1, i64.const_int(0, false), "nz")
            .map_err(llvm_err)?;
        let wn_nwin = self
            .builder
            .build_select(wn_nz, i64.const_int(0, false), wn_nw1, "nwin")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let wn_resv = wn_res.try_as_basic_value().unwrap_basic();
        let wn_ra = self
            .builder
            .build_alloca(self.list_type, "wn_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(wn_ra, wn_resv).map_err(llvm_err)?;
        let wn_i = self.builder.build_alloca(i64, "wn_i").map_err(llvm_err)?;
        self.builder
            .build_store(wn_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let wn_loop = self.context.append_basic_block(wn_fn, "loop");
        let wn_body = self.context.append_basic_block(wn_fn, "body");
        let wn_done = self.context.append_basic_block(wn_fn, "done");
        let _ = self.builder.build_unconditional_branch(wn_loop);
        self.builder.position_at_end(wn_loop);
        let wn_iv = self
            .builder
            .build_load(i64, wn_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, wn_iv, wn_nwin, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(wn_cond, wn_body, wn_done);
        self.builder.position_at_end(wn_body);
        let wn_subl = self.call_rt("action_list_create", &[wn_wsafe.into()])?;
        let wn_sublv = wn_subl.try_as_basic_value().unwrap_basic();
        let wn_sa = self
            .builder
            .build_alloca(self.list_type, "wn_sa")
            .map_err(llvm_err)?;
        self.builder
            .build_store(wn_sa, wn_sublv)
            .map_err(llvm_err)?;
        let wn_j = self.builder.build_alloca(i64, "wn_j").map_err(llvm_err)?;
        self.builder
            .build_store(wn_j, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let wn_iloop = self.context.append_basic_block(wn_fn, "iloop");
        let wn_ibody = self.context.append_basic_block(wn_fn, "ibody");
        let wn_idone = self.context.append_basic_block(wn_fn, "idone");
        let _ = self.builder.build_unconditional_branch(wn_iloop);
        self.builder.position_at_end(wn_iloop);
        let wn_jv = self
            .builder
            .build_load(i64, wn_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let wn_jc = self
            .builder
            .build_int_compare(IntPredicate::SLT, wn_jv, wn_wsafe, "jc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(wn_jc, wn_ibody, wn_idone);
        self.builder.position_at_end(wn_ibody);
        let wn_ep_idx = self
            .builder
            .build_int_add(wn_iv, wn_jv, "epi")
            .map_err(llvm_err)?;
        let wn_ep = unsafe {
            self.builder
                .build_gep(self.string_type, wn_data, &[wn_ep_idx], "ep")
                .map_err(llvm_err)
        }?;
        let wn_ev = self
            .builder
            .build_load(self.string_type, wn_ep, "ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let wn_cl = self
            .builder
            .build_load(self.list_type, wn_sa, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let wn_ps = self.call_rt(
            "action_list_push",
            &[wn_cl.into(), wn_ev.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(wn_sa, wn_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let wn_jvi = self
            .builder
            .build_int_add(wn_jv, i64.const_int(1, false), "jvi")
            .map_err(llvm_err)?;
        self.builder.build_store(wn_j, wn_jvi).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(wn_iloop);
        self.builder.position_at_end(wn_idone);
        let wn_fat = self.string_type.get_undef();
        let wn_ft = self
            .builder
            .build_insert_value(wn_fat, i64.const_int(6, false), 0, "ft")
            .map_err(llvm_err)?;
        let wn_sl = self
            .builder
            .build_load(self.list_type, wn_sa, "sl")
            .map_err(llvm_err)?
            .into_struct_value();
        let wn_sp = self
            .builder
            .build_alloca(self.list_type, "wn_sp")
            .map_err(llvm_err)?;
        self.builder.build_store(wn_sp, wn_sl).map_err(llvm_err)?;
        let wn_fv = self
            .builder
            .build_insert_value(wn_ft, wn_sp, 1, "fv")
            .map_err(llvm_err)?;
        let wn_rl = self
            .builder
            .build_load(self.list_type, wn_ra, "rl")
            .map_err(llvm_err)?
            .into_struct_value();
        let wn_rps = self.call_rt(
            "action_list_push",
            &[wn_rl.into(), wn_fv.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(wn_ra, wn_rps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let wn_ivi = self
            .builder
            .build_int_add(wn_iv, i64.const_int(1, false), "ivi")
            .map_err(llvm_err)?;
        self.builder.build_store(wn_i, wn_ivi).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(wn_loop);
        self.builder.position_at_end(wn_done);
        let wn_rt = self
            .builder
            .build_load(self.list_type, wn_ra, "wn_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&wn_rt));

        // ---- action_list_index_of({ptr, i64, i64}, {i64, ptr}) -> i64 ----
        let lio_fn = self.module.add_function(
            "action_list_index_of",
            i64.fn_type(&[list_ty.into(), str_ty.into()], false),
            None,
        );
        let lio_entry = self.context.append_basic_block(lio_fn, "entry");
        self.builder.position_at_end(lio_entry);
        let lio_lst = lio_fn.get_first_param().unwrap().into_struct_value();
        let lio_tgt = lio_fn.get_nth_param(1).unwrap().into_struct_value();
        let lio_data = self
            .builder
            .build_extract_value(lio_lst, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lio_len = self
            .builder
            .build_extract_value(lio_lst, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let lio_i = self.builder.build_alloca(i64, "i").map_err(llvm_err)?;
        self.builder
            .build_store(lio_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let lio_loop = self.context.append_basic_block(lio_fn, "loop");
        let lio_body = self.context.append_basic_block(lio_fn, "body");
        let lio_nf = self.context.append_basic_block(lio_fn, "notfound");
        let _ = self.builder.build_unconditional_branch(lio_loop);
        self.builder.position_at_end(lio_loop);
        let lio_iv = self
            .builder
            .build_load(i64, lio_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let lio_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, lio_iv, lio_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(lio_cond, lio_body, lio_nf);
        self.builder.position_at_end(lio_body);
        let lio_ep = unsafe {
            self.builder
                .build_gep(self.string_type, lio_data, &[lio_iv], "ep")
                .map_err(llvm_err)
        }?;
        let lio_ev = self
            .builder
            .build_load(self.string_type, lio_ep, "ev")
            .map_err(llvm_err)?
            .into_struct_value();
        let lio_etag = self
            .builder
            .build_extract_value(lio_ev, 0, "etag")
            .map_err(llvm_err)?
            .into_int_value();
        let lio_ttag = self
            .builder
            .build_extract_value(lio_tgt, 0, "ttag")
            .map_err(llvm_err)?
            .into_int_value();
        let lio_teq = self
            .builder
            .build_int_compare(IntPredicate::EQ, lio_etag, lio_ttag, "teq")
            .map_err(llvm_err)?;
        let lio_eptr = self
            .builder
            .build_extract_value(lio_ev, 1, "eptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lio_tptr = self
            .builder
            .build_extract_value(lio_tgt, 1, "tptr")
            .map_err(llvm_err)?
            .into_pointer_value();
        let lio_ptr_match = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_ptr_to_int(lio_eptr, i64, "")
                    .map_err(llvm_err)?,
                self.builder
                    .build_ptr_to_int(lio_tptr, i64, "")
                    .map_err(llvm_err)?,
                "scm",
            )
            .map_err(llvm_err)?;
        let lio_match = self
            .builder
            .build_and(lio_teq, lio_ptr_match, "match")
            .map_err(llvm_err)?;
        let lio_ret_match = self.context.append_basic_block(lio_fn, "ret_match");
        let lio_next = self.context.append_basic_block(lio_fn, "next");
        let _ = self
            .builder
            .build_conditional_branch(lio_match, lio_ret_match, lio_next);
        self.builder.position_at_end(lio_ret_match);
        let _ = self.builder.build_return(Some(&lio_iv));
        self.builder.position_at_end(lio_next);
        let lio_inc = self
            .builder
            .build_int_add(lio_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(lio_i, lio_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(lio_loop);
        self.builder.position_at_end(lio_nf);
        let _ = self
            .builder
            .build_return(Some(&i64.const_int(-1i64 as u64, true)));

        // ---- action_abs_f(f64) -> f64 ----
        let af_fn =
            self.module
                .add_function("action_abs_f", f64.fn_type(&[f64.into()], false), None);
        let af_entry = self.context.append_basic_block(af_fn, "entry");
        self.builder.position_at_end(af_entry);
        let af_val = af_fn.get_first_param().unwrap().into_float_value();
        let af_zero = f64.const_zero();
        let af_neg = self
            .builder
            .build_float_neg(af_val, "neg")
            .map_err(llvm_err)?;
        let af_cmp = self
            .builder
            .build_float_compare(FloatPredicate::OLT, af_val, af_zero, "cmp")
            .map_err(llvm_err)?;
        let af_r = self
            .builder
            .build_select(af_cmp, af_neg, af_val, "r")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&af_r));

        // ---- action_map_keys({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let mk_fn = self.module.add_function(
            "action_map_keys",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let mk_entry = self.context.append_basic_block(mk_fn, "entry");
        self.builder.position_at_end(mk_entry);
        let mk_in = mk_fn.get_first_param().unwrap().into_struct_value();
        let mk_data = self
            .builder
            .build_extract_value(mk_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mk_len = self
            .builder
            .build_extract_value(mk_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let mk_data_i64 = self
            .builder
            .build_pointer_cast(mk_data, ptr, "data_i64")
            .map_err(llvm_err)?;
        let mk_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let mk_resv = mk_res.try_as_basic_value().unwrap_basic();
        let mk_ra = self
            .builder
            .build_alloca(self.list_type, "mk_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(mk_ra, mk_resv).map_err(llvm_err)?;
        let mk_i = self.builder.build_alloca(i64, "mk_i").map_err(llvm_err)?;
        self.builder
            .build_store(mk_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let mk_loop = self.context.append_basic_block(mk_fn, "loop");
        let mk_body = self.context.append_basic_block(mk_fn, "body");
        let mk_done = self.context.append_basic_block(mk_fn, "done");
        let _ = self.builder.build_unconditional_branch(mk_loop);
        self.builder.position_at_end(mk_loop);
        let mk_iv = self
            .builder
            .build_load(i64, mk_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mk_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mk_iv, mk_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mk_cond, mk_body, mk_done);
        self.builder.position_at_end(mk_body);
        // Map entry layout: [key_tag, key_ptr_i64, val_tag, val_ptr_i64] = 4 i64s per entry
        let mk_off = self
            .builder
            .build_int_mul(mk_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let mk_ktp = unsafe {
            self.builder
                .build_gep(i64, mk_data_i64, &[mk_off], "ktp")
                .map_err(llvm_err)
        }?;
        let mk_kt = self
            .builder
            .build_load(i64, mk_ktp, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let mk_off1 = self
            .builder
            .build_int_add(mk_off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let mk_kpp = unsafe {
            self.builder
                .build_gep(i64, mk_data_i64, &[mk_off1], "kpp")
                .map_err(llvm_err)
        }?;
        let mk_kp_i64 = self
            .builder
            .build_load(i64, mk_kpp, "kp_i64")
            .map_err(llvm_err)?
            .into_int_value();
        let mk_kp = self
            .builder
            .build_int_to_ptr(mk_kp_i64, ptr, "kp")
            .map_err(llvm_err)?;
        // Build key fat struct
        let mk_key_undef = self.string_type.get_undef();
        let mk_key1 = self
            .builder
            .build_insert_value(mk_key_undef, mk_kt, 0, "ktag")
            .map_err(llvm_err)?;
        let mk_key2 = self
            .builder
            .build_insert_value(mk_key1, mk_kp, 1, "kdata")
            .map_err(llvm_err)?;
        // Push key to result
        let mk_cl = self
            .builder
            .build_load(self.list_type, mk_ra, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let mk_ps = self.call_rt(
            "action_list_push",
            &[mk_cl.into(), mk_key2.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(mk_ra, mk_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let mk_inc = self
            .builder
            .build_int_add(mk_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(mk_i, mk_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mk_loop);
        self.builder.position_at_end(mk_done);
        let mk_rt = self
            .builder
            .build_load(self.list_type, mk_ra, "mk_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mk_rt));

        // ---- action_map_values({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let mv_fn = self.module.add_function(
            "action_map_values",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let mv_entry = self.context.append_basic_block(mv_fn, "entry");
        self.builder.position_at_end(mv_entry);
        let mv_in = mv_fn.get_first_param().unwrap().into_struct_value();
        let mv_data = self
            .builder
            .build_extract_value(mv_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mv_len = self
            .builder
            .build_extract_value(mv_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let mv_data_i64 = self
            .builder
            .build_pointer_cast(mv_data, ptr, "data_i64")
            .map_err(llvm_err)?;
        let mv_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let mv_resv = mv_res.try_as_basic_value().unwrap_basic();
        let mv_ra = self
            .builder
            .build_alloca(self.list_type, "mv_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(mv_ra, mv_resv).map_err(llvm_err)?;
        let mv_i = self.builder.build_alloca(i64, "mv_i").map_err(llvm_err)?;
        self.builder
            .build_store(mv_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let mv_loop = self.context.append_basic_block(mv_fn, "loop");
        let mv_body = self.context.append_basic_block(mv_fn, "body");
        let mv_done = self.context.append_basic_block(mv_fn, "done");
        let _ = self.builder.build_unconditional_branch(mv_loop);
        self.builder.position_at_end(mv_loop);
        let mv_iv = self
            .builder
            .build_load(i64, mv_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mv_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, mv_iv, mv_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mv_cond, mv_body, mv_done);
        self.builder.position_at_end(mv_body);
        // Map entry layout: [key_tag, key_ptr_i64, val_tag, val_ptr_i64]
        let mv_off = self
            .builder
            .build_int_mul(mv_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let mv_off2 = self
            .builder
            .build_int_add(mv_off, i64.const_int(2, false), "off2")
            .map_err(llvm_err)?;
        let mv_vtp = unsafe {
            self.builder
                .build_gep(i64, mv_data_i64, &[mv_off2], "vtp")
                .map_err(llvm_err)
        }?;
        let mv_vt = self
            .builder
            .build_load(i64, mv_vtp, "vt")
            .map_err(llvm_err)?
            .into_int_value();
        let mv_off3 = self
            .builder
            .build_int_add(mv_off, i64.const_int(3, false), "off3")
            .map_err(llvm_err)?;
        let mv_vpp = unsafe {
            self.builder
                .build_gep(i64, mv_data_i64, &[mv_off3], "vpp")
                .map_err(llvm_err)
        }?;
        let mv_vp_i64 = self
            .builder
            .build_load(i64, mv_vpp, "vp_i64")
            .map_err(llvm_err)?
            .into_int_value();
        let mv_vp = self
            .builder
            .build_int_to_ptr(mv_vp_i64, ptr, "vp")
            .map_err(llvm_err)?;
        // Build value fat struct
        let mv_val_undef = self.string_type.get_undef();
        let mv_val1 = self
            .builder
            .build_insert_value(mv_val_undef, mv_vt, 0, "vtag")
            .map_err(llvm_err)?;
        let mv_val2 = self
            .builder
            .build_insert_value(mv_val1, mv_vp, 1, "vdata")
            .map_err(llvm_err)?;
        let mv_cl = self
            .builder
            .build_load(self.list_type, mv_ra, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let mv_ps = self.call_rt(
            "action_list_push",
            &[mv_cl.into(), mv_val2.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(mv_ra, mv_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let mv_inc = self
            .builder
            .build_int_add(mv_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(mv_i, mv_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mv_loop);
        self.builder.position_at_end(mv_done);
        let mv_rt = self
            .builder
            .build_load(self.list_type, mv_ra, "mv_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mv_rt));

        // ---- action_map_entries({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let me_fn = self.module.add_function(
            "action_map_entries",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let me_entry = self.context.append_basic_block(me_fn, "entry");
        self.builder.position_at_end(me_entry);
        let me_in = me_fn.get_first_param().unwrap().into_struct_value();
        let me_data = self
            .builder
            .build_extract_value(me_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let me_len = self
            .builder
            .build_extract_value(me_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        let me_data_i64 = self
            .builder
            .build_pointer_cast(me_data, ptr, "data_i64")
            .map_err(llvm_err)?;
        let me_res = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let me_resv = me_res.try_as_basic_value().unwrap_basic();
        let me_ra = self
            .builder
            .build_alloca(self.list_type, "me_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(me_ra, me_resv).map_err(llvm_err)?;
        let me_i = self.builder.build_alloca(i64, "me_i").map_err(llvm_err)?;
        self.builder
            .build_store(me_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let me_loop = self.context.append_basic_block(me_fn, "loop");
        let me_body = self.context.append_basic_block(me_fn, "body");
        let me_done = self.context.append_basic_block(me_fn, "done");
        let _ = self.builder.build_unconditional_branch(me_loop);
        self.builder.position_at_end(me_loop);
        let me_iv = self
            .builder
            .build_load(i64, me_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let me_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, me_iv, me_len, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(me_cond, me_body, me_done);
        self.builder.position_at_end(me_body);
        // Build a tuple fat struct: (key, value)
        // Map entry layout: [key_tag, key_ptr_i64, val_tag, val_ptr_i64]
        let me_off = self
            .builder
            .build_int_mul(me_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        // Key fat struct
        let me_ktp = unsafe {
            self.builder
                .build_gep(i64, me_data_i64, &[me_off], "ktp")
                .map_err(llvm_err)
        }?;
        let me_kt = self
            .builder
            .build_load(i64, me_ktp, "kt")
            .map_err(llvm_err)?
            .into_int_value();
        let me_off1 = self
            .builder
            .build_int_add(me_off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let me_kpp = unsafe {
            self.builder
                .build_gep(i64, me_data_i64, &[me_off1], "kpp")
                .map_err(llvm_err)
        }?;
        let me_kp_i64 = self
            .builder
            .build_load(i64, me_kpp, "kp_i64")
            .map_err(llvm_err)?
            .into_int_value();
        let me_kp = self
            .builder
            .build_int_to_ptr(me_kp_i64, ptr, "kp")
            .map_err(llvm_err)?;
        // Value fat struct
        let me_off2 = self
            .builder
            .build_int_add(me_off, i64.const_int(2, false), "off2")
            .map_err(llvm_err)?;
        let me_vtp = unsafe {
            self.builder
                .build_gep(i64, me_data_i64, &[me_off2], "vtp")
                .map_err(llvm_err)
        }?;
        let me_vt = self
            .builder
            .build_load(i64, me_vtp, "vt")
            .map_err(llvm_err)?
            .into_int_value();
        let me_off3 = self
            .builder
            .build_int_add(me_off, i64.const_int(3, false), "off3")
            .map_err(llvm_err)?;
        let me_vpp = unsafe {
            self.builder
                .build_gep(i64, me_data_i64, &[me_off3], "vpp")
                .map_err(llvm_err)
        }?;
        let me_vp_i64 = self
            .builder
            .build_load(i64, me_vpp, "vp_i64")
            .map_err(llvm_err)?
            .into_int_value();
        let me_vp = self
            .builder
            .build_int_to_ptr(me_vp_i64, ptr, "vp")
            .map_err(llvm_err)?;
        // Build tuple: allocate 2 fat structs and point to them
        let me_k_undef = self.string_type.get_undef();
        let me_k1 = self
            .builder
            .build_insert_value(me_k_undef, me_kt, 0, "k1")
            .map_err(llvm_err)?;
        let me_k2 = self
            .builder
            .build_insert_value(me_k1, me_kp, 1, "k2")
            .map_err(llvm_err)?;
        let me_v_undef = self.string_type.get_undef();
        let me_v1 = self
            .builder
            .build_insert_value(me_v_undef, me_vt, 0, "v1")
            .map_err(llvm_err)?;
        let me_v2 = self
            .builder
            .build_insert_value(me_v1, me_vp, 1, "v2")
            .map_err(llvm_err)?;
        // Store key+value in a malloc'd block of 2 fat structs
        let me_tuple_ptr = self
            .builder
            .build_call(malloc_fn, &[i64.const_int(32, false).into()], "tup")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        self.builder
            .build_store(me_tuple_ptr, me_k2)
            .map_err(llvm_err)?;
        let me_vslot = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    me_tuple_ptr,
                    &[i64.const_int(1, false)],
                    "vslot",
                )
                .map_err(llvm_err)
        }?;
        self.builder
            .build_store(me_vslot, me_v2)
            .map_err(llvm_err)?;
        // Wrap in a fat struct: tag=5 (Struct), data=tuple_ptr
        let me_fat_undef = self.string_type.get_undef();
        let me_fat1 = self
            .builder
            .build_insert_value(me_fat_undef, i64.const_int(5, false), 0, "ftag")
            .map_err(llvm_err)?;
        let me_fat2 = self
            .builder
            .build_insert_value(me_fat1, me_tuple_ptr, 1, "fdata")
            .map_err(llvm_err)?;
        let me_cl = self
            .builder
            .build_load(self.list_type, me_ra, "cl")
            .map_err(llvm_err)?
            .into_struct_value();
        let me_ps = self.call_rt(
            "action_list_push",
            &[me_cl.into(), me_fat2.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(me_ra, me_ps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let me_inc = self
            .builder
            .build_int_add(me_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(me_i, me_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(me_loop);
        self.builder.position_at_end(me_done);
        let me_rt = self
            .builder
            .build_load(self.list_type, me_ra, "me_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&me_rt));

        // ---- action_set_union({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Sets use map layout (4×i64 per entry). Result must be in map format.
        let su_fn = self.module.add_function(
            "action_set_union",
            list_ty.fn_type(&[list_ty.into(), list_ty.into()], false),
            None,
        );
        let su_entry = self.context.append_basic_block(su_fn, "entry");
        self.builder.position_at_end(su_entry);
        let su_a = su_fn.get_first_param().unwrap().into_struct_value();
        let su_b = su_fn.get_nth_param(1).unwrap().into_struct_value();
        let su_alen = self
            .builder
            .build_extract_value(su_a, 1, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let su_blen = self
            .builder
            .build_extract_value(su_b, 1, "blen")
            .map_err(llvm_err)?
            .into_int_value();
        let su_cap = self
            .builder
            .build_int_add(su_alen, su_blen, "cap")
            .map_err(llvm_err)?;
        let su_cap4 = self
            .builder
            .build_int_add(su_cap, i64.const_int(4, false), "cap4")
            .map_err(llvm_err)?;
        let map_create_fn = self.module.get_function("action_map_create").unwrap();
        let mi_fn = self.module.get_function("action_map_insert").unwrap();
        let mc_fn = self.module.get_function("action_map_contains").unwrap();
        let su_res = self
            .builder
            .build_call(map_create_fn, &[su_cap4.into()], "res")
            .map_err(llvm_err)?;
        let su_resv = su_res.try_as_basic_value().unwrap_basic();
        let su_ra = self
            .builder
            .build_alloca(self.list_type, "su_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(su_ra, su_resv).map_err(llvm_err)?;
        let su_null = {
            let u = str_ty.get_undef();
            let u1 = self
                .builder
                .build_insert_value(u, i64.const_int(0, false), 0, "n0")
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(u1, self.ptr_ty().const_zero(), 1, "n1")
                .map_err(llvm_err)?
        };
        // Helper: build key fat struct from map entry at i64 offset `off`
        let build_key = |builder: &inkwell::builder::Builder<'ctx>,
                         data_i64p: PointerValue<'ctx>,
                         off: IntValue<'ctx>|
         -> Result<_, String> {
            let tp =
                unsafe { builder.build_gep(i64, data_i64p, &[off], "tp") }.map_err(llvm_err)?;
            let tag = builder
                .build_load(i64, tp, "tag")
                .map_err(llvm_err)?
                .into_int_value();
            let off1 = builder
                .build_int_add(off, i64.const_int(1, false), "off1")
                .map_err(llvm_err)?;
            let pp =
                unsafe { builder.build_gep(i64, data_i64p, &[off1], "pp") }.map_err(llvm_err)?;
            let pi = builder
                .build_load(i64, pp, "pi")
                .map_err(llvm_err)?
                .into_int_value();
            let pv = builder.build_int_to_ptr(pi, ptr, "pv").map_err(llvm_err)?;
            let u = str_ty.get_undef();
            let u1 = builder
                .build_insert_value(u, tag, 0, "k1")
                .map_err(llvm_err)?;
            Ok(builder
                .build_insert_value(u1, pv, 1, "k2")
                .map_err(llvm_err)?)
        };
        // Add all from A
        let su_adata = self
            .builder
            .build_extract_value(su_a, 0, "adata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let su_a_i64p = self
            .builder
            .build_pointer_cast(su_adata, ptr, "a_i64p")
            .map_err(llvm_err)?;
        let su_i = self.builder.build_alloca(i64, "su_i").map_err(llvm_err)?;
        self.builder
            .build_store(su_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let su_loop1 = self.context.append_basic_block(su_fn, "loop1");
        let su_body1 = self.context.append_basic_block(su_fn, "body1");
        let su_done1 = self.context.append_basic_block(su_fn, "done1");
        let _ = self.builder.build_unconditional_branch(su_loop1);
        self.builder.position_at_end(su_loop1);
        let su_iv = self
            .builder
            .build_load(i64, su_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let su_c1 = self
            .builder
            .build_int_compare(IntPredicate::SLT, su_iv, su_alen, "c1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(su_c1, su_body1, su_done1);
        self.builder.position_at_end(su_body1);
        let su_off = self
            .builder
            .build_int_mul(su_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let su_key = build_key(&self.builder, su_a_i64p, su_off)?;
        let su_cl1 = self
            .builder
            .build_load(self.list_type, su_ra, "cl1")
            .map_err(llvm_err)?
            .into_struct_value();
        let su_ins = self
            .builder
            .build_call(
                mi_fn,
                &[
                    su_cl1.into(),
                    su_key.as_basic_value_enum().into(),
                    su_null.as_basic_value_enum().into(),
                ],
                "ins",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(su_ra, su_ins.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let su_inc = self
            .builder
            .build_int_add(su_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(su_i, su_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(su_loop1);
        // Add from B only if not already in result
        self.builder.position_at_end(su_done1);
        let su_bdata = self
            .builder
            .build_extract_value(su_b, 0, "bdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let su_b_i64p = self
            .builder
            .build_pointer_cast(su_bdata, ptr, "b_i64p")
            .map_err(llvm_err)?;
        let su_j = self.builder.build_alloca(i64, "su_j").map_err(llvm_err)?;
        self.builder
            .build_store(su_j, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let su_loop2 = self.context.append_basic_block(su_fn, "loop2");
        let su_body2 = self.context.append_basic_block(su_fn, "body2");
        let su_done2 = self.context.append_basic_block(su_fn, "done2");
        let _ = self.builder.build_unconditional_branch(su_loop2);
        self.builder.position_at_end(su_loop2);
        let su_jv = self
            .builder
            .build_load(i64, su_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let su_c2 = self
            .builder
            .build_int_compare(IntPredicate::SLT, su_jv, su_blen, "c2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(su_c2, su_body2, su_done2);
        self.builder.position_at_end(su_body2);
        let su_boff = self
            .builder
            .build_int_mul(su_jv, i64.const_int(4, false), "boff")
            .map_err(llvm_err)?;
        let su_key2 = build_key(&self.builder, su_b_i64p, su_boff)?;
        let su_cl2 = self
            .builder
            .build_load(self.list_type, su_ra, "cl2")
            .map_err(llvm_err)?
            .into_struct_value();
        let su_contains = self
            .builder
            .build_call(
                mc_fn,
                &[su_cl2.into(), su_key2.as_basic_value_enum().into()],
                "cont",
            )
            .map_err(llvm_err)?;
        let su_not_cont = self
            .builder
            .build_not(
                su_contains
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value(),
                "nc",
            )
            .map_err(llvm_err)?;
        let su_add = self.context.append_basic_block(su_fn, "add");
        let su_skip = self.context.append_basic_block(su_fn, "skip");
        let _ = self
            .builder
            .build_conditional_branch(su_not_cont, su_add, su_skip);
        self.builder.position_at_end(su_add);
        let su_cl3 = self
            .builder
            .build_load(self.list_type, su_ra, "cl3")
            .map_err(llvm_err)?
            .into_struct_value();
        let su_ins2 = self
            .builder
            .build_call(
                mi_fn,
                &[
                    su_cl3.into(),
                    su_key2.as_basic_value_enum().into(),
                    su_null.as_basic_value_enum().into(),
                ],
                "ins2",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(su_ra, su_ins2.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(su_skip);
        self.builder.position_at_end(su_skip);
        let su_inc2 = self
            .builder
            .build_int_add(su_jv, i64.const_int(1, false), "inc2")
            .map_err(llvm_err)?;
        self.builder.build_store(su_j, su_inc2).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(su_loop2);
        self.builder.position_at_end(su_done2);
        let su_rt = self
            .builder
            .build_load(self.list_type, su_ra, "su_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&su_rt));

        // ---- action_set_intersection({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Sets use map layout (4×i64 per entry). Result must be in map format.
        let si_fn = self.module.add_function(
            "action_set_intersection",
            list_ty.fn_type(&[list_ty.into(), list_ty.into()], false),
            None,
        );
        let si_entry = self.context.append_basic_block(si_fn, "entry");
        self.builder.position_at_end(si_entry);
        let si_a = si_fn.get_first_param().unwrap().into_struct_value();
        let si_b = si_fn.get_nth_param(1).unwrap().into_struct_value();
        let si_alen = self
            .builder
            .build_extract_value(si_a, 1, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let si_cap4 = self
            .builder
            .build_int_add(si_alen, i64.const_int(4, false), "cap4")
            .map_err(llvm_err)?;
        let map_create_fn = self.module.get_function("action_map_create").unwrap();
        let mi_fn = self.module.get_function("action_map_insert").unwrap();
        let mc_fn = self.module.get_function("action_map_contains").unwrap();
        let si_res = self
            .builder
            .build_call(map_create_fn, &[si_cap4.into()], "res")
            .map_err(llvm_err)?;
        let si_resv = si_res.try_as_basic_value().unwrap_basic();
        let si_ra = self
            .builder
            .build_alloca(self.list_type, "si_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(si_ra, si_resv).map_err(llvm_err)?;
        let si_null = {
            let u = str_ty.get_undef();
            let u1 = self
                .builder
                .build_insert_value(u, i64.const_int(0, false), 0, "n0")
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(u1, self.ptr_ty().const_zero(), 1, "n1")
                .map_err(llvm_err)?
        };
        let si_adata = self
            .builder
            .build_extract_value(si_a, 0, "adata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let si_a_i64p = self
            .builder
            .build_pointer_cast(si_adata, ptr, "a_i64p")
            .map_err(llvm_err)?;
        let si_i = self.builder.build_alloca(i64, "si_i").map_err(llvm_err)?;
        self.builder
            .build_store(si_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let si_loop = self.context.append_basic_block(si_fn, "loop");
        let si_body = self.context.append_basic_block(si_fn, "body");
        let si_done = self.context.append_basic_block(si_fn, "done");
        let _ = self.builder.build_unconditional_branch(si_loop);
        self.builder.position_at_end(si_loop);
        let si_iv = self
            .builder
            .build_load(i64, si_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let si_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, si_iv, si_alen, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(si_cond, si_body, si_done);
        self.builder.position_at_end(si_body);
        let si_off = self
            .builder
            .build_int_mul(si_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let si_tp = unsafe {
            self.builder
                .build_gep(i64, si_a_i64p, &[si_off], "tp")
                .map_err(llvm_err)
        }?;
        let si_tag = self
            .builder
            .build_load(i64, si_tp, "tag")
            .map_err(llvm_err)?
            .into_int_value();
        let si_off1 = self
            .builder
            .build_int_add(si_off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let si_pp = unsafe {
            self.builder
                .build_gep(i64, si_a_i64p, &[si_off1], "pp")
                .map_err(llvm_err)
        }?;
        let si_pi = self
            .builder
            .build_load(i64, si_pp, "pi")
            .map_err(llvm_err)?
            .into_int_value();
        let si_pv = self
            .builder
            .build_int_to_ptr(si_pi, ptr, "pv")
            .map_err(llvm_err)?;
        let si_key_undef = str_ty.get_undef();
        let si_key1 = self
            .builder
            .build_insert_value(si_key_undef, si_tag, 0, "k1")
            .map_err(llvm_err)?;
        let si_key = self
            .builder
            .build_insert_value(si_key1, si_pv, 1, "k2")
            .map_err(llvm_err)?;
        // Check if element is in B (use map_contains for correct layout)
        let si_contains = self
            .builder
            .build_call(
                mc_fn,
                &[
                    si_b.as_basic_value_enum().into(),
                    si_key.as_basic_value_enum().into(),
                ],
                "cont",
            )
            .map_err(llvm_err)?;
        let si_found = si_contains
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let si_add = self.context.append_basic_block(si_fn, "add");
        let si_skip = self.context.append_basic_block(si_fn, "skip");
        let _ = self
            .builder
            .build_conditional_branch(si_found, si_add, si_skip);
        self.builder.position_at_end(si_add);
        let si_cl2 = self
            .builder
            .build_load(self.list_type, si_ra, "cl2")
            .map_err(llvm_err)?
            .into_struct_value();
        let si_ins = self
            .builder
            .build_call(
                mi_fn,
                &[
                    si_cl2.into(),
                    si_key.as_basic_value_enum().into(),
                    si_null.as_basic_value_enum().into(),
                ],
                "ins",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(si_ra, si_ins.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(si_skip);
        self.builder.position_at_end(si_skip);
        let si_inc = self
            .builder
            .build_int_add(si_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(si_i, si_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(si_loop);
        self.builder.position_at_end(si_done);
        let si_rt = self
            .builder
            .build_load(self.list_type, si_ra, "si_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&si_rt));

        // ---- action_set_difference({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Sets use map layout (4×i64 per entry). Result must be in map format.
        let sd_fn = self.module.add_function(
            "action_set_difference",
            list_ty.fn_type(&[list_ty.into(), list_ty.into()], false),
            None,
        );
        let sd_entry = self.context.append_basic_block(sd_fn, "entry");
        self.builder.position_at_end(sd_entry);
        let sd_a = sd_fn.get_first_param().unwrap().into_struct_value();
        let sd_b = sd_fn.get_nth_param(1).unwrap().into_struct_value();
        let sd_alen = self
            .builder
            .build_extract_value(sd_a, 1, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let sd_cap4 = self
            .builder
            .build_int_add(sd_alen, i64.const_int(4, false), "cap4")
            .map_err(llvm_err)?;
        let map_create_fn = self.module.get_function("action_map_create").unwrap();
        let mi_fn = self.module.get_function("action_map_insert").unwrap();
        let mc_fn = self.module.get_function("action_map_contains").unwrap();
        let sd_res = self
            .builder
            .build_call(map_create_fn, &[sd_cap4.into()], "res")
            .map_err(llvm_err)?;
        let sd_resv = sd_res.try_as_basic_value().unwrap_basic();
        let sd_ra = self
            .builder
            .build_alloca(self.list_type, "sd_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(sd_ra, sd_resv).map_err(llvm_err)?;
        let sd_null = {
            let u = str_ty.get_undef();
            let u1 = self
                .builder
                .build_insert_value(u, i64.const_int(0, false), 0, "n0")
                .map_err(llvm_err)?;
            self.builder
                .build_insert_value(u1, self.ptr_ty().const_zero(), 1, "n1")
                .map_err(llvm_err)?
        };
        let sd_adata = self
            .builder
            .build_extract_value(sd_a, 0, "adata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let sd_a_i64p = self
            .builder
            .build_pointer_cast(sd_adata, ptr, "a_i64p")
            .map_err(llvm_err)?;
        let sd_i = self.builder.build_alloca(i64, "sd_i").map_err(llvm_err)?;
        self.builder
            .build_store(sd_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let sd_loop = self.context.append_basic_block(sd_fn, "loop");
        let sd_body = self.context.append_basic_block(sd_fn, "body");
        let sd_done = self.context.append_basic_block(sd_fn, "done");
        let _ = self.builder.build_unconditional_branch(sd_loop);
        self.builder.position_at_end(sd_loop);
        let sd_iv = self
            .builder
            .build_load(i64, sd_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let sd_cond = self
            .builder
            .build_int_compare(IntPredicate::SLT, sd_iv, sd_alen, "cond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(sd_cond, sd_body, sd_done);
        self.builder.position_at_end(sd_body);
        let sd_off = self
            .builder
            .build_int_mul(sd_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let sd_tp = unsafe {
            self.builder
                .build_gep(i64, sd_a_i64p, &[sd_off], "tp")
                .map_err(llvm_err)
        }?;
        let sd_tag = self
            .builder
            .build_load(i64, sd_tp, "tag")
            .map_err(llvm_err)?
            .into_int_value();
        let sd_off1 = self
            .builder
            .build_int_add(sd_off, i64.const_int(1, false), "off1")
            .map_err(llvm_err)?;
        let sd_pp = unsafe {
            self.builder
                .build_gep(i64, sd_a_i64p, &[sd_off1], "pp")
                .map_err(llvm_err)
        }?;
        let sd_pi = self
            .builder
            .build_load(i64, sd_pp, "pi")
            .map_err(llvm_err)?
            .into_int_value();
        let sd_pv = self
            .builder
            .build_int_to_ptr(sd_pi, ptr, "pv")
            .map_err(llvm_err)?;
        let sd_key_undef = str_ty.get_undef();
        let sd_key1 = self
            .builder
            .build_insert_value(sd_key_undef, sd_tag, 0, "k1")
            .map_err(llvm_err)?;
        let sd_key = self
            .builder
            .build_insert_value(sd_key1, sd_pv, 1, "k2")
            .map_err(llvm_err)?;
        // Check if element is NOT in B (use map_contains for correct layout)
        let sd_contains = self
            .builder
            .build_call(
                mc_fn,
                &[
                    sd_b.as_basic_value_enum().into(),
                    sd_key.as_basic_value_enum().into(),
                ],
                "cont",
            )
            .map_err(llvm_err)?;
        let sd_not_cont = self
            .builder
            .build_not(
                sd_contains
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_int_value(),
                "nc",
            )
            .map_err(llvm_err)?;
        let sd_add = self.context.append_basic_block(sd_fn, "add");
        let sd_skip = self.context.append_basic_block(sd_fn, "skip");
        let _ = self
            .builder
            .build_conditional_branch(sd_not_cont, sd_add, sd_skip);
        self.builder.position_at_end(sd_add);
        let sd_cl2 = self
            .builder
            .build_load(self.list_type, sd_ra, "cl2")
            .map_err(llvm_err)?
            .into_struct_value();
        let sd_ins = self
            .builder
            .build_call(
                mi_fn,
                &[
                    sd_cl2.into(),
                    sd_key.as_basic_value_enum().into(),
                    sd_null.as_basic_value_enum().into(),
                ],
                "ins",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(sd_ra, sd_ins.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sd_skip);
        self.builder.position_at_end(sd_skip);
        let sd_inc = self
            .builder
            .build_int_add(sd_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(sd_i, sd_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(sd_loop);
        self.builder.position_at_end(sd_done);
        let sd_rt = self
            .builder
            .build_load(self.list_type, sd_ra, "sd_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&sd_rt));

        // ---- action_set_is_subset({ptr, i64, i64}, {ptr, i64, i64}) -> i1 ----
        // Sets use map layout: each entry = 4×i64 (key_tag, key_ptr_i64, val_tag, val_ptr_i64).
        // Compare only keys (offsets 0 and 1), skip values (offsets 2 and 3).
        let ss_fn = self.module.add_function(
            "action_set_is_subset",
            self.context
                .bool_type()
                .fn_type(&[list_ty.into(), list_ty.into()], false),
            None,
        );
        let ss_entry = self.context.append_basic_block(ss_fn, "entry");
        self.builder.position_at_end(ss_entry);
        let a = ss_fn.get_first_param().unwrap().into_struct_value();
        let b = ss_fn.get_nth_param(1).unwrap().into_struct_value();
        let a_data_ptr = self
            .builder
            .build_extract_value(a, 0, "ad")
            .map_err(llvm_err)?
            .into_pointer_value();
        let alen = self
            .builder
            .build_extract_value(a, 1, "al")
            .map_err(llvm_err)?
            .into_int_value();
        let b_data_ptr = self
            .builder
            .build_extract_value(b, 0, "bd")
            .map_err(llvm_err)?
            .into_pointer_value();
        let blen = self
            .builder
            .build_extract_value(b, 1, "bl")
            .map_err(llvm_err)?
            .into_int_value();
        // Cast data pointers to i64* for 4×i64 entry indexing
        let a_i64p = self
            .builder
            .build_pointer_cast(a_data_ptr, ptr, "a_i64p")
            .map_err(llvm_err)?;
        let b_i64p = self
            .builder
            .build_pointer_cast(b_data_ptr, ptr, "b_i64p")
            .map_err(llvm_err)?;

        // Outer loop counter
        let oi = self.builder.build_alloca(i64, "oi").map_err(llvm_err)?;
        self.builder
            .build_store(oi, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let oloop = self.context.append_basic_block(ss_fn, "oloop");
        let obody = self.context.append_basic_block(ss_fn, "obody");
        let ofound = self.context.append_basic_block(ss_fn, "ofound");
        let oinc = self.context.append_basic_block(ss_fn, "oinc");
        let rtrue = self.context.append_basic_block(ss_fn, "rtrue");
        let rfalse = self.context.append_basic_block(ss_fn, "rfalse");
        let _ = self.builder.build_unconditional_branch(oloop);

        // Outer loop
        self.builder.position_at_end(oloop);
        let oiv = self
            .builder
            .build_load(i64, oi, "oiv")
            .map_err(llvm_err)?
            .into_int_value();
        let ocond = self
            .builder
            .build_int_compare(IntPredicate::SLT, oiv, alen, "ocond")
            .map_err(llvm_err)?;
        let _ = self.builder.build_conditional_branch(ocond, obody, rtrue);

        // Outer body: load A key at offset oiv*4
        self.builder.position_at_end(obody);
        let a_off = self
            .builder
            .build_int_mul(oiv, i64.const_int(4, false), "a_off")
            .map_err(llvm_err)?;
        let a_tag_ptr = unsafe {
            self.builder
                .build_gep(i64, a_i64p, &[a_off], "a_tp")
                .map_err(llvm_err)
        }?;
        let a_tag = self
            .builder
            .build_load(i64, a_tag_ptr, "a_tag")
            .map_err(llvm_err)?
            .into_int_value();
        let a_off1 = self
            .builder
            .build_int_add(a_off, i64.const_int(1, false), "a_off1")
            .map_err(llvm_err)?;
        let a_ptr_ptr = unsafe {
            self.builder
                .build_gep(i64, a_i64p, &[a_off1], "a_pp")
                .map_err(llvm_err)
        }?;
        let a_ptr_i64 = self
            .builder
            .build_load(i64, a_ptr_ptr, "a_pi")
            .map_err(llvm_err)?
            .into_int_value();
        let a_is_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                a_ptr_i64,
                i64.const_int(0, false),
                "a_is_null",
            )
            .map_err(llvm_err)?;

        // Inner loop counter
        let ij = self.builder.build_alloca(i64, "ij").map_err(llvm_err)?;
        self.builder
            .build_store(ij, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let iloop = self.context.append_basic_block(ss_fn, "iloop");
        let ibody = self.context.append_basic_block(ss_fn, "ibody");
        let inext = self.context.append_basic_block(ss_fn, "inext");
        let inotfound = self.context.append_basic_block(ss_fn, "inotfound");
        let _ = self.builder.build_unconditional_branch(iloop);

        // Inner loop
        self.builder.position_at_end(iloop);
        let ijv = self
            .builder
            .build_load(i64, ij, "ijv")
            .map_err(llvm_err)?
            .into_int_value();
        let icond = self
            .builder
            .build_int_compare(IntPredicate::SLT, ijv, blen, "icond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(icond, ibody, inotfound);

        // Inner body: load B key at offset ijv*4, compare with A key
        self.builder.position_at_end(ibody);
        let b_off = self
            .builder
            .build_int_mul(ijv, i64.const_int(4, false), "b_off")
            .map_err(llvm_err)?;
        let b_tag_ptr = unsafe {
            self.builder
                .build_gep(i64, b_i64p, &[b_off], "b_tp")
                .map_err(llvm_err)
        }?;
        let b_tag = self
            .builder
            .build_load(i64, b_tag_ptr, "b_tag")
            .map_err(llvm_err)?
            .into_int_value();
        let b_off1 = self
            .builder
            .build_int_add(b_off, i64.const_int(1, false), "b_off1")
            .map_err(llvm_err)?;
        let b_ptr_ptr = unsafe {
            self.builder
                .build_gep(i64, b_i64p, &[b_off1], "b_pp")
                .map_err(llvm_err)
        }?;
        let b_ptr_i64 = self
            .builder
            .build_load(i64, b_ptr_ptr, "b_pi")
            .map_err(llvm_err)?
            .into_int_value();
        let tag_eq = self
            .builder
            .build_int_compare(IntPredicate::EQ, a_tag, b_tag, "tag_eq")
            .map_err(llvm_err)?;
        let icontent = self.context.append_basic_block(ss_fn, "icontent");
        let _ = self
            .builder
            .build_conditional_branch(tag_eq, icontent, inext);

        // Tags match: check pointer for null vs content
        self.builder.position_at_end(icontent);
        let b_is_null = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                b_ptr_i64,
                i64.const_int(0, false),
                "b_is_null",
            )
            .map_err(llvm_err)?;
        let both_null = self
            .builder
            .build_and(a_is_null, b_is_null, "both_null")
            .map_err(llvm_err)?;
        let ifound_bb = self.context.append_basic_block(ss_fn, "ifound_bb");
        let istr_bb = self.context.append_basic_block(ss_fn, "istr_bb");
        let _ = self
            .builder
            .build_conditional_branch(both_null, ifound_bb, istr_bb);
        // Both null: int/None match
        self.builder.position_at_end(ifound_bb);
        let _ = self.builder.build_unconditional_branch(ofound);
        // At least one pointer non-null: both must be non-null for string compare
        self.builder.position_at_end(istr_bb);
        let a_nn = self
            .builder
            .build_not(a_is_null, "a_nn")
            .map_err(llvm_err)?;
        let b_nn = self
            .builder
            .build_not(b_is_null, "b_nn")
            .map_err(llvm_err)?;
        let both_nn = self
            .builder
            .build_and(a_nn, b_nn, "both_nn")
            .map_err(llvm_err)?;
        let istr_eq = self.context.append_basic_block(ss_fn, "istr_eq");
        let _ = self
            .builder
            .build_conditional_branch(both_nn, istr_eq, inext);
        // Build fat structs for string_eq call
        self.builder.position_at_end(istr_eq);
        let a_fat_undef = str_ty.get_undef();
        let a_fat1 = self
            .builder
            .build_insert_value(a_fat_undef, a_tag, 0, "af1")
            .map_err(llvm_err)?;
        let a_ptr_val = self
            .builder
            .build_int_to_ptr(a_ptr_i64, ptr, "a_ptr")
            .map_err(llvm_err)?;
        let a_fat2 = self
            .builder
            .build_insert_value(a_fat1, a_ptr_val, 1, "af2")
            .map_err(llvm_err)?;
        let b_fat_undef = str_ty.get_undef();
        let b_fat1 = self
            .builder
            .build_insert_value(b_fat_undef, b_tag, 0, "bf1")
            .map_err(llvm_err)?;
        let b_ptr_val = self
            .builder
            .build_int_to_ptr(b_ptr_i64, ptr, "b_ptr")
            .map_err(llvm_err)?;
        let b_fat2 = self
            .builder
            .build_insert_value(b_fat1, b_ptr_val, 1, "bf2")
            .map_err(llvm_err)?;
        let sseq_fn = self.module.get_function("action_string_eq").unwrap();
        let sseq = self
            .builder
            .build_call(
                sseq_fn,
                &[
                    a_fat2.as_basic_value_enum().into(),
                    b_fat2.as_basic_value_enum().into(),
                ],
                "sseq",
            )
            .map_err(llvm_err)?;
        let seq_val = sseq.try_as_basic_value().unwrap_basic().into_int_value();
        let istr_found = self.context.append_basic_block(ss_fn, "istr_found");
        let _ = self
            .builder
            .build_conditional_branch(seq_val, istr_found, inext);
        self.builder.position_at_end(istr_found);
        let _ = self.builder.build_unconditional_branch(ofound);

        // Increment inner loop
        self.builder.position_at_end(inext);
        let nij = self
            .builder
            .build_int_add(ijv, i64.const_int(1, false), "nij")
            .map_err(llvm_err)?;
        self.builder.build_store(ij, nij).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(iloop);

        // Element NOT found in B
        self.builder.position_at_end(inotfound);
        let _ = self.builder.build_unconditional_branch(rfalse);

        // Element found in B: increment outer loop
        self.builder.position_at_end(ofound);
        let _ = self.builder.build_unconditional_branch(oinc);
        self.builder.position_at_end(oinc);
        let noi = self
            .builder
            .build_int_add(oiv, i64.const_int(1, false), "noi")
            .map_err(llvm_err)?;
        self.builder.build_store(oi, noi).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(oloop);

        // Results
        self.builder.position_at_end(rfalse);
        let _ = self
            .builder
            .build_return(Some(&self.context.bool_type().const_int(0, false)));
        self.builder.position_at_end(rtrue);
        let _ = self
            .builder
            .build_return(Some(&self.context.bool_type().const_int(1, false)));

        // ---- action_rand_shuffle({ptr, i64, i64}) -> {ptr, i64, i64} ----
        let rs_fn = self.module.add_function(
            "action_rand_shuffle",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let rs_entry = self.context.append_basic_block(rs_fn, "entry");
        self.builder.position_at_end(rs_entry);
        let rs_in = rs_fn.get_first_param().unwrap().into_struct_value();
        let rs_data = self
            .builder
            .build_extract_value(rs_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rs_len = self
            .builder
            .build_extract_value(rs_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        // Copy input list
        let rs_copy = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let rs_copyv = rs_copy.try_as_basic_value().unwrap_basic();
        let rs_ra = self
            .builder
            .build_alloca(self.list_type, "rs_ra")
            .map_err(llvm_err)?;
        self.builder
            .build_store(rs_ra, rs_copyv)
            .map_err(llvm_err)?;
        // Copy all elements
        let rs_ci = self.builder.build_alloca(i64, "rs_ci").map_err(llvm_err)?;
        self.builder
            .build_store(rs_ci, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let rs_cloop = self.context.append_basic_block(rs_fn, "cloop");
        let rs_cbody = self.context.append_basic_block(rs_fn, "cbody");
        let rs_cdone = self.context.append_basic_block(rs_fn, "cdone");
        let _ = self.builder.build_unconditional_branch(rs_cloop);
        self.builder.position_at_end(rs_cloop);
        let rs_civ = self
            .builder
            .build_load(i64, rs_ci, "civ")
            .map_err(llvm_err)?
            .into_int_value();
        let rs_ccond = self
            .builder
            .build_int_compare(IntPredicate::SLT, rs_civ, rs_len, "ccond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rs_ccond, rs_cbody, rs_cdone);
        self.builder.position_at_end(rs_cbody);
        let rs_cep = unsafe {
            self.builder
                .build_gep(self.string_type, rs_data, &[rs_civ], "cep")
                .map_err(llvm_err)
        }?;
        let rs_cev = self
            .builder
            .build_load(self.string_type, rs_cep, "cev")
            .map_err(llvm_err)?
            .into_struct_value();
        let rs_ccl = self
            .builder
            .build_load(self.list_type, rs_ra, "ccl")
            .map_err(llvm_err)?
            .into_struct_value();
        let rs_cps = self.call_rt(
            "action_list_push",
            &[rs_ccl.into(), rs_cev.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(rs_ra, rs_cps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let rs_cinc = self
            .builder
            .build_int_add(rs_civ, i64.const_int(1, false), "cinc")
            .map_err(llvm_err)?;
        self.builder.build_store(rs_ci, rs_cinc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rs_cloop);
        self.builder.position_at_end(rs_cdone);
        // Fisher-Yates shuffle: iterate from end to start
        let rs_i = self.builder.build_alloca(i64, "rs_i").map_err(llvm_err)?;
        let rs_len1 = self
            .builder
            .build_int_sub(rs_len, i64.const_int(1, false), "len1")
            .map_err(llvm_err)?;
        self.builder.build_store(rs_i, rs_len1).map_err(llvm_err)?;
        let rs_floop = self.context.append_basic_block(rs_fn, "floop");
        let rs_fbody = self.context.append_basic_block(rs_fn, "fbody");
        let rs_fdone = self.context.append_basic_block(rs_fn, "fdone");
        let _ = self.builder.build_unconditional_branch(rs_floop);
        self.builder.position_at_end(rs_floop);
        let rs_iv = self
            .builder
            .build_load(i64, rs_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let rs_fcond = self
            .builder
            .build_int_compare(IntPredicate::SGT, rs_iv, i64.const_int(0, false), "fcond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rs_fcond, rs_fbody, rs_fdone);
        self.builder.position_at_end(rs_fbody);
        // Generate random index [0, i]
        let rs_rand = self.call_rt(
            "action_rand_int",
            &[i64.const_int(0, false).into(), rs_iv.into()],
        )?;
        let rs_j = rs_rand.try_as_basic_value().unwrap_basic().into_int_value();
        // Swap elements at i and j
        let rs_cur = self
            .builder
            .build_load(self.list_type, rs_ra, "cur_list")
            .map_err(llvm_err)?
            .into_struct_value();
        let rs_cur_data = self
            .builder
            .build_extract_value(rs_cur, 0, "cur_data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let rs_epi = unsafe {
            self.builder
                .build_gep(self.string_type, rs_cur_data, &[rs_iv], "epi")
                .map_err(llvm_err)
        }?;
        let rs_epj = unsafe {
            self.builder
                .build_gep(self.string_type, rs_cur_data, &[rs_j], "epj")
                .map_err(llvm_err)
        }?;
        let rs_ei = self
            .builder
            .build_load(self.string_type, rs_epi, "ei")
            .map_err(llvm_err)?;
        let rs_ej = self
            .builder
            .build_load(self.string_type, rs_epj, "ej")
            .map_err(llvm_err)?;
        self.builder.build_store(rs_epi, rs_ej).map_err(llvm_err)?;
        self.builder.build_store(rs_epj, rs_ei).map_err(llvm_err)?;
        let rs_dec = self
            .builder
            .build_int_sub(rs_iv, i64.const_int(1, false), "dec")
            .map_err(llvm_err)?;
        self.builder.build_store(rs_i, rs_dec).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rs_floop);
        self.builder.position_at_end(rs_fdone);
        let rs_rt = self
            .builder
            .build_load(self.list_type, rs_ra, "rs_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&rs_rt));

        // ---- action_list_sorted({ptr, i64, i64}) -> {ptr, i64, i64} (Int-only for now) ----
        let srt_fn = self.module.add_function(
            "action_list_sorted",
            list_ty.fn_type(&[list_ty.into()], false),
            None,
        );
        let srt_entry = self.context.append_basic_block(srt_fn, "entry");
        self.builder.position_at_end(srt_entry);
        let srt_in = srt_fn.get_first_param().unwrap().into_struct_value();
        let srt_data = self
            .builder
            .build_extract_value(srt_in, 0, "data")
            .map_err(llvm_err)?
            .into_pointer_value();
        let srt_len = self
            .builder
            .build_extract_value(srt_in, 1, "len")
            .map_err(llvm_err)?
            .into_int_value();
        // Copy input
        let srt_copy = self.call_rt("action_list_create", &[i64.const_int(4, false).into()])?;
        let srt_copyv = srt_copy.try_as_basic_value().unwrap_basic();
        let srt_ra = self
            .builder
            .build_alloca(self.list_type, "srt_ra")
            .map_err(llvm_err)?;
        self.builder
            .build_store(srt_ra, srt_copyv)
            .map_err(llvm_err)?;
        let srt_ci = self.builder.build_alloca(i64, "srt_ci").map_err(llvm_err)?;
        self.builder
            .build_store(srt_ci, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let srt_cloop = self.context.append_basic_block(srt_fn, "cloop");
        let srt_cbody = self.context.append_basic_block(srt_fn, "cbody");
        let srt_cdone = self.context.append_basic_block(srt_fn, "cdone");
        let _ = self.builder.build_unconditional_branch(srt_cloop);
        self.builder.position_at_end(srt_cloop);
        let srt_civ = self
            .builder
            .build_load(i64, srt_ci, "civ")
            .map_err(llvm_err)?
            .into_int_value();
        let srt_ccond = self
            .builder
            .build_int_compare(IntPredicate::SLT, srt_civ, srt_len, "ccond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(srt_ccond, srt_cbody, srt_cdone);
        self.builder.position_at_end(srt_cbody);
        let srt_cep = unsafe {
            self.builder
                .build_gep(self.string_type, srt_data, &[srt_civ], "cep")
                .map_err(llvm_err)
        }?;
        let srt_cev = self
            .builder
            .build_load(self.string_type, srt_cep, "cev")
            .map_err(llvm_err)?
            .into_struct_value();
        let srt_ccl = self
            .builder
            .build_load(self.list_type, srt_ra, "ccl")
            .map_err(llvm_err)?
            .into_struct_value();
        let srt_cps = self.call_rt(
            "action_list_push",
            &[srt_ccl.into(), srt_cev.as_basic_value_enum().into()],
        )?;
        self.builder
            .build_store(srt_ra, srt_cps.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let srt_cinc = self
            .builder
            .build_int_add(srt_civ, i64.const_int(1, false), "cinc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(srt_ci, srt_cinc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(srt_cloop);
        // Simple bubble sort on the copy
        self.builder.position_at_end(srt_cdone);
        let srt_i = self.builder.build_alloca(i64, "srt_i").map_err(llvm_err)?;
        self.builder
            .build_store(srt_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let srt_oloop = self.context.append_basic_block(srt_fn, "oloop");
        let srt_obody = self.context.append_basic_block(srt_fn, "obody");
        let srt_odone = self.context.append_basic_block(srt_fn, "odone");
        let _ = self.builder.build_unconditional_branch(srt_oloop);
        self.builder.position_at_end(srt_oloop);
        let srt_iv = self
            .builder
            .build_load(i64, srt_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let srt_ocond = self
            .builder
            .build_int_compare(IntPredicate::SLT, srt_iv, srt_len, "ocond")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(srt_ocond, srt_obody, srt_odone);
        self.builder.position_at_end(srt_obody);
        let srt_j = self.builder.build_alloca(i64, "srt_j").map_err(llvm_err)?;
        self.builder
            .build_store(srt_j, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let srt_len1 = self
            .builder
            .build_int_sub(srt_len, i64.const_int(1, false), "len1")
            .map_err(llvm_err)?;
        let srt_iloop = self.context.append_basic_block(srt_fn, "iloop");
        let srt_ibody = self.context.append_basic_block(srt_fn, "ibody");
        let srt_idone = self.context.append_basic_block(srt_fn, "idone");
        let _ = self.builder.build_unconditional_branch(srt_iloop);
        self.builder.position_at_end(srt_iloop);
        let srt_jv = self
            .builder
            .build_load(i64, srt_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let srt_jc = self
            .builder
            .build_int_compare(IntPredicate::SLT, srt_jv, srt_len1, "jc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(srt_jc, srt_ibody, srt_idone);
        self.builder.position_at_end(srt_ibody);
        let srt_cur = self
            .builder
            .build_load(self.list_type, srt_ra, "cur")
            .map_err(llvm_err)?
            .into_struct_value();
        let srt_cur_data = self
            .builder
            .build_extract_value(srt_cur, 0, "curd")
            .map_err(llvm_err)?
            .into_pointer_value();
        let srt_epa = unsafe {
            self.builder
                .build_gep(self.string_type, srt_cur_data, &[srt_jv], "epa")
                .map_err(llvm_err)
        }?;
        let srt_epb = unsafe {
            self.builder
                .build_gep(
                    self.string_type,
                    srt_cur_data,
                    &[self
                        .builder
                        .build_int_add(srt_jv, i64.const_int(1, false), "jp1")
                        .map_err(llvm_err)?],
                    "epb",
                )
                .map_err(llvm_err)
        }?;
        let srt_ea = self
            .builder
            .build_load(self.string_type, srt_epa, "ea")
            .map_err(llvm_err)?
            .into_struct_value();
        let srt_eb = self
            .builder
            .build_load(self.string_type, srt_epb, "eb")
            .map_err(llvm_err)?
            .into_struct_value();
        // Compare Int values: extract data pointer as value for Tag=0
        let _srt_ea_tag = self
            .builder
            .build_extract_value(srt_ea, 0, "eat")
            .map_err(llvm_err)?
            .into_int_value();
        let _srt_eb_tag = self
            .builder
            .build_extract_value(srt_eb, 0, "ebt")
            .map_err(llvm_err)?
            .into_int_value();
        let _srt_is_int = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                _srt_ea_tag,
                i64.const_int(0, false),
                "isint",
            )
            .map_err(llvm_err)?;
        let srt_ea_ptr = self
            .builder
            .build_extract_value(srt_ea, 1, "eap")
            .map_err(llvm_err)?
            .into_pointer_value();
        let srt_eb_ptr = self
            .builder
            .build_extract_value(srt_eb, 1, "ebp")
            .map_err(llvm_err)?
            .into_pointer_value();
        let srt_ea_int = self
            .builder
            .build_ptr_to_int(srt_ea_ptr, i64, "eai")
            .map_err(llvm_err)?;
        let srt_eb_int = self
            .builder
            .build_ptr_to_int(srt_eb_ptr, i64, "ebi")
            .map_err(llvm_err)?;
        let srt_swap_needed = self
            .builder
            .build_int_compare(IntPredicate::SGT, srt_ea_int, srt_eb_int, "swap")
            .map_err(llvm_err)?;
        let srt_swap = self.context.append_basic_block(srt_fn, "swap");
        let srt_noswap = self.context.append_basic_block(srt_fn, "noswap");
        let _ = self
            .builder
            .build_conditional_branch(srt_swap_needed, srt_swap, srt_noswap);
        self.builder.position_at_end(srt_swap);
        self.builder
            .build_store(srt_epa, srt_eb)
            .map_err(llvm_err)?;
        self.builder
            .build_store(srt_epb, srt_ea)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(srt_noswap);
        self.builder.position_at_end(srt_noswap);
        let srt_jinc = self
            .builder
            .build_int_add(srt_jv, i64.const_int(1, false), "jinc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(srt_j, srt_jinc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(srt_iloop);
        self.builder.position_at_end(srt_idone);
        let srt_iinc = self
            .builder
            .build_int_add(srt_iv, i64.const_int(1, false), "iinc")
            .map_err(llvm_err)?;
        self.builder
            .build_store(srt_i, srt_iinc)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(srt_oloop);
        self.builder.position_at_end(srt_odone);
        let srt_rt = self
            .builder
            .build_load(self.list_type, srt_ra, "srt_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&srt_rt));

        // ---- action_map_union({ptr, i64, i64}, {ptr, i64, i64}) -> {ptr, i64, i64} ----
        // Merges two maps. Entries from second map overwrite first.
        let mu_fn = self.module.add_function(
            "action_map_union",
            list_ty.fn_type(&[list_ty.into(), list_ty.into()], false),
            None,
        );
        let mu_entry = self.context.append_basic_block(mu_fn, "entry");
        self.builder.position_at_end(mu_entry);
        let mu_a = mu_fn.get_first_param().unwrap().into_struct_value();
        let mu_b = mu_fn.get_nth_param(1).unwrap().into_struct_value();
        let mu_alen = self
            .builder
            .build_extract_value(mu_a, 1, "alen")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_blen = self
            .builder
            .build_extract_value(mu_b, 1, "blen")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_cap = self
            .builder
            .build_int_add(
                self.builder
                    .build_int_add(mu_alen, mu_blen, "cap")
                    .map_err(llvm_err)?,
                i64.const_int(4, false),
                "cap4",
            )
            .map_err(llvm_err)?;
        let mu_create = self.module.get_function("action_map_create").unwrap();
        let mi_fn = self.module.get_function("action_map_insert").unwrap();
        let mu_res = self
            .builder
            .build_call(mu_create, &[mu_cap.into()], "res")
            .map_err(llvm_err)?;
        let mu_resv = mu_res.try_as_basic_value().unwrap_basic();
        let mu_ra = self
            .builder
            .build_alloca(self.list_type, "mu_ra")
            .map_err(llvm_err)?;
        self.builder.build_store(mu_ra, mu_resv).map_err(llvm_err)?;
        // Helper: build key and value fat structs from map entry at i64 offset
        let mu_build_kv = |builder: &inkwell::builder::Builder<'ctx>,
                           data_i64p: PointerValue<'ctx>,
                           off: IntValue<'ctx>|
         -> Result<
            (
                inkwell::values::AggregateValueEnum<'ctx>,
                inkwell::values::AggregateValueEnum<'ctx>,
            ),
            String,
        > {
            // key_tag
            let ktp =
                unsafe { builder.build_gep(i64, data_i64p, &[off], "ktp") }.map_err(llvm_err)?;
            let kt = builder
                .build_load(i64, ktp, "kt")
                .map_err(llvm_err)?
                .into_int_value();
            let off1 = builder
                .build_int_add(off, i64.const_int(1, false), "off1")
                .map_err(llvm_err)?;
            let kpp =
                unsafe { builder.build_gep(i64, data_i64p, &[off1], "kpp") }.map_err(llvm_err)?;
            let kpi = builder
                .build_load(i64, kpp, "kpi")
                .map_err(llvm_err)?
                .into_int_value();
            let kp = builder.build_int_to_ptr(kpi, ptr, "kp").map_err(llvm_err)?;
            // val_tag
            let off2 = builder
                .build_int_add(off, i64.const_int(2, false), "off2")
                .map_err(llvm_err)?;
            let vtp =
                unsafe { builder.build_gep(i64, data_i64p, &[off2], "vtp") }.map_err(llvm_err)?;
            let vt = builder
                .build_load(i64, vtp, "vt")
                .map_err(llvm_err)?
                .into_int_value();
            let off3 = builder
                .build_int_add(off, i64.const_int(3, false), "off3")
                .map_err(llvm_err)?;
            let vpp =
                unsafe { builder.build_gep(i64, data_i64p, &[off3], "vpp") }.map_err(llvm_err)?;
            let vpi = builder
                .build_load(i64, vpp, "vpi")
                .map_err(llvm_err)?
                .into_int_value();
            let vp = builder.build_int_to_ptr(vpi, ptr, "vp").map_err(llvm_err)?;
            let ku = str_ty.get_undef();
            let ku1 = builder
                .build_insert_value(ku, kt, 0, "k1")
                .map_err(llvm_err)?;
            let kf = builder
                .build_insert_value(ku1, kp, 1, "kf")
                .map_err(llvm_err)?;
            let vu = str_ty.get_undef();
            let vu1 = builder
                .build_insert_value(vu, vt, 0, "v1")
                .map_err(llvm_err)?;
            let vf = builder
                .build_insert_value(vu1, vp, 1, "vf")
                .map_err(llvm_err)?;
            Ok((kf, vf))
        };
        // Loop 1: insert all from A
        let mu_adata = self
            .builder
            .build_extract_value(mu_a, 0, "adata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mu_a_i64p = self
            .builder
            .build_pointer_cast(mu_adata, ptr, "a_i64p")
            .map_err(llvm_err)?;
        let mu_i = self.builder.build_alloca(i64, "mu_i").map_err(llvm_err)?;
        self.builder
            .build_store(mu_i, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let mu_loop1 = self.context.append_basic_block(mu_fn, "loop1");
        let mu_body1 = self.context.append_basic_block(mu_fn, "body1");
        let mu_done1 = self.context.append_basic_block(mu_fn, "done1");
        let _ = self.builder.build_unconditional_branch(mu_loop1);
        self.builder.position_at_end(mu_loop1);
        let mu_iv = self
            .builder
            .build_load(i64, mu_i, "iv")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_c1 = self
            .builder
            .build_int_compare(IntPredicate::SLT, mu_iv, mu_alen, "c1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mu_c1, mu_body1, mu_done1);
        self.builder.position_at_end(mu_body1);
        let mu_off = self
            .builder
            .build_int_mul(mu_iv, i64.const_int(4, false), "off")
            .map_err(llvm_err)?;
        let (mu_key, mu_val) = mu_build_kv(&self.builder, mu_a_i64p, mu_off)?;
        let mu_cl1 = self
            .builder
            .build_load(self.list_type, mu_ra, "cl1")
            .map_err(llvm_err)?
            .into_struct_value();
        let mu_ins = self
            .builder
            .build_call(
                mi_fn,
                &[
                    mu_cl1.into(),
                    mu_key.as_basic_value_enum().into(),
                    mu_val.as_basic_value_enum().into(),
                ],
                "ins",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(mu_ra, mu_ins.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let mu_inc = self
            .builder
            .build_int_add(mu_iv, i64.const_int(1, false), "inc")
            .map_err(llvm_err)?;
        self.builder.build_store(mu_i, mu_inc).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mu_loop1);
        // Loop 2: insert all from B (overwrites existing keys)
        self.builder.position_at_end(mu_done1);
        let mu_bdata = self
            .builder
            .build_extract_value(mu_b, 0, "bdata")
            .map_err(llvm_err)?
            .into_pointer_value();
        let mu_b_i64p = self
            .builder
            .build_pointer_cast(mu_bdata, ptr, "b_i64p")
            .map_err(llvm_err)?;
        let mu_j = self.builder.build_alloca(i64, "mu_j").map_err(llvm_err)?;
        self.builder
            .build_store(mu_j, i64.const_int(0, false))
            .map_err(llvm_err)?;
        let mu_loop2 = self.context.append_basic_block(mu_fn, "loop2");
        let mu_body2 = self.context.append_basic_block(mu_fn, "body2");
        let mu_done2 = self.context.append_basic_block(mu_fn, "done2");
        let _ = self.builder.build_unconditional_branch(mu_loop2);
        self.builder.position_at_end(mu_loop2);
        let mu_jv = self
            .builder
            .build_load(i64, mu_j, "jv")
            .map_err(llvm_err)?
            .into_int_value();
        let mu_c2 = self
            .builder
            .build_int_compare(IntPredicate::SLT, mu_jv, mu_blen, "c2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(mu_c2, mu_body2, mu_done2);
        self.builder.position_at_end(mu_body2);
        let mu_boff = self
            .builder
            .build_int_mul(mu_jv, i64.const_int(4, false), "boff")
            .map_err(llvm_err)?;
        let (mu_key2, mu_val2) = mu_build_kv(&self.builder, mu_b_i64p, mu_boff)?;
        let mu_cl2 = self
            .builder
            .build_load(self.list_type, mu_ra, "cl2")
            .map_err(llvm_err)?
            .into_struct_value();
        let mu_ins2 = self
            .builder
            .build_call(
                mi_fn,
                &[
                    mu_cl2.into(),
                    mu_key2.as_basic_value_enum().into(),
                    mu_val2.as_basic_value_enum().into(),
                ],
                "ins2",
            )
            .map_err(llvm_err)?;
        self.builder
            .build_store(mu_ra, mu_ins2.try_as_basic_value().unwrap_basic())
            .map_err(llvm_err)?;
        let mu_inc2 = self
            .builder
            .build_int_add(mu_jv, i64.const_int(1, false), "inc2")
            .map_err(llvm_err)?;
        self.builder.build_store(mu_j, mu_inc2).map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(mu_loop2);
        self.builder.position_at_end(mu_done2);
        let mu_rt = self
            .builder
            .build_load(self.list_type, mu_ra, "mu_rt")
            .map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&mu_rt));

        // ---- action_pow(f64, f64) -> f64 ----
        let pow_fn = self.module.add_function(
            "action_pow",
            f64.fn_type(&[f64.into(), f64.into()], false),
            None,
        );
        let pow_entry = self.context.append_basic_block(pow_fn, "entry");
        self.builder.position_at_end(pow_entry);
        let pow_base = pow_fn.get_first_param().unwrap().into_float_value();
        let pow_exp = pow_fn.get_nth_param(1).unwrap().into_float_value();
        let pow_c_fn = self.module.get_function("pow").unwrap();
        let pow_r = self
            .builder
            .build_call(pow_c_fn, &[pow_base.into(), pow_exp.into()], "r")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_float_value();
        let _ = self.builder.build_return(Some(&pow_r));

        // ---- RC (Reference Counting) runtime ----
        // action_rc_inc(i8* ptr): increment refcount at ptr-8. Null-safe.
        let rc_inc_fn =
            self.module
                .add_function("action_rc_inc", void.fn_type(&[ptr.into()], false), None);
        let rc_inc_entry = self.context.append_basic_block(rc_inc_fn, "entry");
        let rc_inc_do = self.context.append_basic_block(rc_inc_fn, "do_inc");
        let rc_inc_done = self.context.append_basic_block(rc_inc_fn, "done");
        self.builder.position_at_end(rc_inc_entry);
        let rc_inc_ptr = rc_inc_fn.get_first_param().unwrap().into_pointer_value();
        let rc_is_null = self
            .builder
            .build_is_null(rc_inc_ptr, "is_null")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rc_is_null, rc_inc_done, rc_inc_do);
        self.builder.position_at_end(rc_inc_do);
        let rc_inc_i64 = self
            .builder
            .build_ptr_to_int(rc_inc_ptr, i64, "rc_i64")
            .map_err(llvm_err)?;
        let rc_inc_minus8 = self
            .builder
            .build_int_sub(rc_inc_i64, i64.const_int(8, false), "minus8")
            .map_err(llvm_err)?;
        let rc_inc_i64p = self
            .builder
            .build_int_to_ptr(rc_inc_minus8, ptr, "rc_i64p")
            .map_err(llvm_err)?;
        let rc_inc_val = self
            .builder
            .build_load(self.i64_ty(), rc_inc_i64p, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let rc_inc_new = self
            .builder
            .build_int_add(rc_inc_val, i64.const_int(1, false), "new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(rc_inc_i64p, rc_inc_new)
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rc_inc_done);
        self.builder.position_at_end(rc_inc_done);
        let _ = self.builder.build_return(None);

        // action_rc_dec(i8* ptr): decrement refcount at ptr-8, free if zero. Null-safe.
        let rc_dec_fn =
            self.module
                .add_function("action_rc_dec", void.fn_type(&[ptr.into()], false), None);
        let rc_dec_entry = self.context.append_basic_block(rc_dec_fn, "entry");
        let rc_dec_null_bb = self.context.append_basic_block(rc_dec_fn, "null_check");
        let rc_dec_free_bb = self.context.append_basic_block(rc_dec_fn, "do_free");
        let rc_dec_done_bb = self.context.append_basic_block(rc_dec_fn, "done");
        self.builder.position_at_end(rc_dec_entry);
        let rc_dec_ptr = rc_dec_fn.get_first_param().unwrap().into_pointer_value();
        // Check for null pointer — if null, skip straight to done
        let rc_is_null = self
            .builder
            .build_is_null(rc_dec_ptr, "is_null")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rc_is_null, rc_dec_done_bb, rc_dec_null_bb);
        // Non-null path
        self.builder.position_at_end(rc_dec_null_bb);
        let rc_dec_i64 = self
            .builder
            .build_ptr_to_int(rc_dec_ptr, i64, "rc_i64")
            .map_err(llvm_err)?;
        let rc_dec_minus8 = self
            .builder
            .build_int_sub(rc_dec_i64, i64.const_int(8, false), "minus8")
            .map_err(llvm_err)?;
        let rc_dec_i64p = self
            .builder
            .build_int_to_ptr(rc_dec_minus8, ptr, "rc_i64p")
            .map_err(llvm_err)?;
        let rc_dec_val = self
            .builder
            .build_load(self.i64_ty(), rc_dec_i64p, "rc")
            .map_err(llvm_err)?
            .into_int_value();
        let rc_dec_new = self
            .builder
            .build_int_sub(rc_dec_val, i64.const_int(1, false), "new_rc")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(rc_dec_i64p, rc_dec_new)
            .map_err(llvm_err)?;
        let rc_is_zero = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                rc_dec_new,
                i64.const_int(0, false),
                "is_zero",
            )
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(rc_is_zero, rc_dec_free_bb, rc_dec_done_bb);
        self.builder.position_at_end(rc_dec_free_bb);
        let free_func = self.module.get_function("free").unwrap();
        let rc_dec_free_ptr = self
            .builder
            .build_int_to_ptr(rc_dec_minus8, ptr, "free_ptr")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_call(free_func, &[rc_dec_free_ptr.into()], "")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(rc_dec_done_bb);
        self.builder.position_at_end(rc_dec_done_bb);
        let _ = self.builder.build_return(None);

        // action_malloc_rc body (declared early near malloc)
        let malloc_rc_fn = self.module.get_function("action_malloc_rc").unwrap();
        let malloc_rc_entry = self.context.append_basic_block(malloc_rc_fn, "entry");
        self.builder.position_at_end(malloc_rc_entry);
        let malloc_rc_size = malloc_rc_fn.get_first_param().unwrap().into_int_value();
        let malloc_rc_total = self
            .builder
            .build_int_add(malloc_rc_size, i64.const_int(8, false), "total")
            .map_err(llvm_err)?;
        let malloc_rc_func = self.module.get_function("malloc").unwrap();
        let malloc_rc_raw = self
            .builder
            .build_call(malloc_rc_func, &[malloc_rc_total.into()], "raw")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Write initial refcount of 0 at offset 0 (inc'd on first variable binding)
        let malloc_rc_i64p = self
            .builder
            .build_pointer_cast(malloc_rc_raw, ptr, "rc_i64p")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_store(malloc_rc_i64p, i64.const_int(0, false))
            .map_err(llvm_err)?;
        // Return ptr + 8 (the data pointer, after the refcount header)
        let malloc_rc_data = unsafe {
            self.builder
                .build_gep(i8, malloc_rc_raw, &[i64.const_int(8, false)], "data")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_return(Some(&malloc_rc_data));

        // action_utf8_encode body: encode a Unicode code point into UTF-8 bytes
        // Takes (i64 code_point, i8* buf) -> returns i64 byte_count (1-4)
        let utf8_encode_fn_body = self.module.get_function("action_utf8_encode").unwrap();
        let utf8_entry = self
            .context
            .append_basic_block(utf8_encode_fn_body, "entry");
        let utf8_1b = self
            .context
            .append_basic_block(utf8_encode_fn_body, "one_byte");
        let utf8_2b = self
            .context
            .append_basic_block(utf8_encode_fn_body, "two_byte");
        let utf8_3b = self
            .context
            .append_basic_block(utf8_encode_fn_body, "three_byte");
        let utf8_4b = self
            .context
            .append_basic_block(utf8_encode_fn_body, "four_byte");
        self.builder.position_at_end(utf8_entry);
        let ucode = utf8_encode_fn_body
            .get_first_param()
            .unwrap()
            .into_int_value();
        let ubuf = utf8_encode_fn_body
            .get_nth_param(1)
            .unwrap()
            .into_pointer_value();
        let u0x7f = i64.const_int(0x7F, false);
        let u0x7ff = i64.const_int(0x7FF, false);
        let u0xffff = i64.const_int(0xFFFF, false);
        let is_1 = self
            .builder
            .build_int_compare(IntPredicate::ULE, ucode, u0x7f, "is1")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_1, utf8_1b, utf8_2b);
        // 1-byte: buf[0] = code (0x00-0x7F)
        self.builder.position_at_end(utf8_1b);
        let u1 = self
            .builder
            .build_int_truncate(ucode, i8, "u1")
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(ubuf, u1).map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&i64.const_int(1, false)));
        // 2-byte check: code <= 0x7FF?
        self.builder.position_at_end(utf8_2b);
        let is_2 = self
            .builder
            .build_int_compare(IntPredicate::ULE, ucode, u0x7ff, "is2")
            .map_err(llvm_err)?;
        let _ = self
            .builder
            .build_conditional_branch(is_2, utf8_3b, utf8_4b);
        // Write 2-byte: buf[0] = 0xC0 | (code >> 6); buf[1] = 0x80 | (code & 0x3F)
        self.builder.position_at_end(utf8_3b);
        let u6 = i64.const_int(6, false);
        let ucp6 = self
            .builder
            .build_right_shift(ucode, u6, false, "cp6")
            .map_err(llvm_err)?;
        let ulead2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucp6, i8, "l2t")
                    .map_err(llvm_err)?,
                i8.const_int(0xC0, false),
                "lead2",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(ubuf, ulead2).map_err(llvm_err)?;
        let umask = i64.const_int(0x3F, false);
        let ucont2 = self
            .builder
            .build_and(ucode, umask, "cont2")
            .map_err(llvm_err)?;
        let ub2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont2, i8, "c2t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b2",
            )
            .map_err(llvm_err)?;
        let ugp1 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(1, false)], "gp1")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp1, ub2).map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&i64.const_int(2, false)));
        // 3-byte check: code <= 0xFFFF?
        self.builder.position_at_end(utf8_4b);
        let is_3 = self
            .builder
            .build_int_compare(IntPredicate::ULE, ucode, u0xffff, "is3")
            .map_err(llvm_err)?;
        let utf8_3b_write = self
            .context
            .append_basic_block(utf8_encode_fn_body, "three_byte_write");
        let utf8_4b_write = self
            .context
            .append_basic_block(utf8_encode_fn_body, "four_byte_write");
        let _ = self
            .builder
            .build_conditional_branch(is_3, utf8_3b_write, utf8_4b_write);
        // Write 3-byte: buf[0] = 0xE0 | (code >> 12); buf[1] = 0x80 | ((code >> 6) & 0x3F); buf[2] = 0x80 | (code & 0x3F)
        self.builder.position_at_end(utf8_3b_write);
        let u12 = i64.const_int(12, false);
        let ucp12 = self
            .builder
            .build_right_shift(ucode, u12, false, "cp12")
            .map_err(llvm_err)?;
        let ulead3 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucp12, i8, "l3t")
                    .map_err(llvm_err)?,
                i8.const_int(0xE0, false),
                "lead3",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(ubuf, ulead3).map_err(llvm_err)?;
        let ucp6b = self
            .builder
            .build_right_shift(ucode, u6, false, "cp6b")
            .map_err(llvm_err)?;
        let ucont3_1 = self
            .builder
            .build_and(ucp6b, umask, "c3_1")
            .map_err(llvm_err)?;
        let ub3_1 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont3_1, i8, "c3_1t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b3_1",
            )
            .map_err(llvm_err)?;
        let ugp3_1 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(1, false)], "gp3_1")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp3_1, ub3_1).map_err(llvm_err)?;
        let ucont3_2 = self
            .builder
            .build_and(ucode, umask, "c3_2")
            .map_err(llvm_err)?;
        let ub3_2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont3_2, i8, "c3_2t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b3_2",
            )
            .map_err(llvm_err)?;
        let ugp3_2 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(2, false)], "gp3_2")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp3_2, ub3_2).map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&i64.const_int(3, false)));
        // Write 4-byte: buf[0] = 0xF0 | (code >> 18); buf[1] = 0x80 | ((code >> 12) & 0x3F);
        //                buf[2] = 0x80 | ((code >> 6) & 0x3F); buf[3] = 0x80 | (code & 0x3F)
        self.builder.position_at_end(utf8_4b_write);
        let u18 = i64.const_int(18, false);
        let ucp18 = self
            .builder
            .build_right_shift(ucode, u18, false, "cp18")
            .map_err(llvm_err)?;
        let ulead4 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucp18, i8, "l4t")
                    .map_err(llvm_err)?,
                i8.const_int(0xF0, false),
                "lead4",
            )
            .map_err(llvm_err)?;
        let _ = self.builder.build_store(ubuf, ulead4).map_err(llvm_err)?;
        let u4_12 = i64.const_int(12, false);
        let u4_6 = i64.const_int(6, false);
        let ucp12b4 = self
            .builder
            .build_right_shift(ucode, u4_12, false, "cp12b4")
            .map_err(llvm_err)?;
        let ucont4_1 = self
            .builder
            .build_and(ucp12b4, umask, "c4_1")
            .map_err(llvm_err)?;
        let ub4_1 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont4_1, i8, "c4_1t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b4_1",
            )
            .map_err(llvm_err)?;
        let ugp4_1 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(1, false)], "gp4_1")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp4_1, ub4_1).map_err(llvm_err)?;
        let ucp6b4 = self
            .builder
            .build_right_shift(ucode, u4_6, false, "cp6b4")
            .map_err(llvm_err)?;
        let ucont4_2 = self
            .builder
            .build_and(ucp6b4, umask, "c4_2")
            .map_err(llvm_err)?;
        let ub4_2 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont4_2, i8, "c4_2t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b4_2",
            )
            .map_err(llvm_err)?;
        let ugp4_2 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(2, false)], "gp4_2")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp4_2, ub4_2).map_err(llvm_err)?;
        let ucont4_3 = self
            .builder
            .build_and(ucode, umask, "c4_3")
            .map_err(llvm_err)?;
        let ub4_3 = self
            .builder
            .build_or(
                self.builder
                    .build_int_truncate(ucont4_3, i8, "c4_3t")
                    .map_err(llvm_err)?,
                i8.const_int(0x80, false),
                "b4_3",
            )
            .map_err(llvm_err)?;
        let ugp4_3 = unsafe {
            self.builder
                .build_gep(i8, ubuf, &[i64.const_int(3, false)], "gp4_3")
                .map_err(llvm_err)
        }?;
        let _ = self.builder.build_store(ugp4_3, ub4_3).map_err(llvm_err)?;
        let _ = self.builder.build_return(Some(&i64.const_int(4, false)));

        // action_utf8_byte_len body: determine UTF-8 byte count from leading byte
        let utf8_bl_fn = self.module.get_function("action_utf8_byte_len").unwrap();
        let bl_entry = self.context.append_basic_block(utf8_bl_fn, "entry");
        self.builder.position_at_end(bl_entry);
        let bl_byte = utf8_bl_fn.get_first_param().unwrap().into_int_value();
        let bl_byte_zext = self
            .builder
            .build_int_z_extend(bl_byte, i64, "zext")
            .map_err(llvm_err)?;
        // Check if continuation byte (10xxxxxx) → treat as 1
        let bl_80 = i64.const_int(0x80, false);
        let is_ascii = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_and(bl_byte_zext, bl_80, "and80")
                    .map_err(llvm_err)?,
                i64.const_int(0, false),
                "is_ascii",
            )
            .map_err(llvm_err)?;
        // Check 2-byte: (byte & 0xE0) == 0xC0
        let bl_e0 = i64.const_int(0xE0, false);
        let bl_c0 = i64.const_int(0xC0, false);
        let is_2b = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_and(bl_byte_zext, bl_e0, "andE0")
                    .map_err(llvm_err)?,
                bl_c0,
                "is_2b",
            )
            .map_err(llvm_err)?;
        // Check 3-byte: (byte & 0xF0) == 0xE0
        let bl_f0 = i64.const_int(0xF0, false);
        let bl_e0c = i64.const_int(0xE0, false);
        let is_3b = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_and(bl_byte_zext, bl_f0, "andF0")
                    .map_err(llvm_err)?,
                bl_e0c,
                "is_3b",
            )
            .map_err(llvm_err)?;
        // Check 4-byte: (byte & 0xF8) == 0xF0
        let bl_f8 = i64.const_int(0xF8, false);
        let bl_f0c = i64.const_int(0xF0, false);
        let is_4b = self
            .builder
            .build_int_compare(
                IntPredicate::EQ,
                self.builder
                    .build_and(bl_byte_zext, bl_f8, "andF8")
                    .map_err(llvm_err)?,
                bl_f0c,
                "is_4b",
            )
            .map_err(llvm_err)?;
        // Select: 3/4, 2/selected, 1/selected
        let one = i64.const_int(1, false);
        let two = i64.const_int(2, false);
        let three = i64.const_int(3, false);
        let four = i64.const_int(4, false);
        let bl_s3 = self
            .builder
            .build_select(is_3b, three, four, "s3")
            .map_err(llvm_err)?
            .into_int_value();
        let bl_s2 = self
            .builder
            .build_select(is_2b, two, bl_s3, "s2")
            .map_err(llvm_err)?
            .into_int_value();
        let bl_result = self
            .builder
            .build_select(is_ascii, one, bl_s2, "s1")
            .map_err(llvm_err)?
            .into_int_value();
        let _ = self.builder.build_return(Some(&bl_result));

        // Restore builder position
        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }

        Ok(())
    }

    pub(super) fn emit_read_line_runtime(&self) -> Result<(), String> {
        if self.module.get_function("action_read_line").is_some() {
            return Ok(());
        }
        let saved_pos = self.builder.get_insert_block();

        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();

        let malloc_fn = self.module.get_function("malloc").unwrap();
        let strlen_fn = self.module.get_function("strlen").unwrap();

        let rl_ret_ty = self
            .context
            .struct_type(&[i64.into(), ptr.into(), self.bool_ty().into()], false);
        let rl_fn =
            self.module
                .add_function("action_read_line", rl_ret_ty.fn_type(&[], false), None);
        let fgets_fn = self.module.get_function("fgets").unwrap();
        let entry = self.context.append_basic_block(rl_fn, "entry");
        self.builder.position_at_end(entry);
        let buf_size = i64.const_int(4096, false);
        let buf = self
            .builder
            .build_call(malloc_fn, &[buf_size.into()], "buf")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        // Get stdin FILE* — platform-specific:
        //   Linux/glibc:   stdin is a global symbol exported by libc
        //   Windows/MSVC:  stdin is not a symbol; use __acrt_iob_func(0) instead
        let stdin_ptr = {
            #[cfg(target_os = "windows")]
            {
                let acrt_fn = self.module.add_function(
                    "__acrt_iob_func",
                    ptr.fn_type(&[i32.into()], false),
                    None,
                );
                self.builder
                    .build_call(acrt_fn, &[i32.const_int(0, false).into()], "stdin")
                    .map_err(llvm_err)?
                    .try_as_basic_value()
                    .unwrap_basic()
                    .into_pointer_value()
            }
            #[cfg(not(target_os = "windows"))]
            {
                let stdin_g = self.module.add_global(ptr, None, "stdin");
                self.builder
                    .build_load(ptr, stdin_g.as_pointer_value(), "stdin_ptr")
                    .map_err(llvm_err)?
                    .into_pointer_value()
            }
        };
        let fgets_ret = self
            .builder
            .build_call(
                fgets_fn,
                &[
                    buf.into(),
                    i32.const_int(4096, false).into(),
                    stdin_ptr.into(),
                ],
                "",
            )
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_pointer_value();
        let is_eof = self
            .builder
            .build_int_compare(IntPredicate::EQ, fgets_ret, ptr.const_zero(), "is_eof")
            .map_err(llvm_err)?;
        let eof_bb = self.context.append_basic_block(rl_fn, "eof");
        let ok_bb = self.context.append_basic_block(rl_fn, "ok");
        let merge_bb = self.context.append_basic_block(rl_fn, "merge");
        let _ = self.builder.build_conditional_branch(is_eof, eof_bb, ok_bb);
        self.builder.position_at_end(eof_bb);
        let eof_undef = rl_ret_ty.get_undef();
        let eof_r1 = self
            .builder
            .build_insert_value(eof_undef, i64.const_int(0, false), 0, "eof_len")
            .map_err(llvm_err)?;
        let eof_r2 = self
            .builder
            .build_insert_value(eof_r1, ptr.const_zero(), 1, "eof_ptr")
            .map_err(llvm_err)?;
        let eof_r3 = self
            .builder
            .build_insert_value(eof_r2, self.bool_ty().const_zero(), 2, "eof_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        self.builder.position_at_end(ok_bb);
        let str_len = self
            .builder
            .build_call(strlen_fn, &[buf.into()], "len")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_int_value();
        let last_idx = self
            .builder
            .build_int_sub(str_len, i64.const_int(1, false), "last_idx")
            .map_err(llvm_err)?;
        let last_ptr = unsafe {
            self.builder
                .build_gep(i8, buf, &[last_idx], "last_ptr")
                .map_err(llvm_err)
        }?;
        let last_ch = self
            .builder
            .build_load(i8, last_ptr, "last_ch")
            .map_err(llvm_err)?
            .into_int_value();
        let is_nl = self
            .builder
            .build_int_compare(IntPredicate::EQ, last_ch, i8.const_int(10, false), "is_nl")
            .map_err(llvm_err)?;
        let adj_len = self
            .builder
            .build_select(is_nl, last_idx, str_len, "adj_len")
            .map_err(llvm_err)?;
        let ok_undef = rl_ret_ty.get_undef();
        let ok_r1 = self
            .builder
            .build_insert_value(ok_undef, adj_len.into_int_value(), 0, "ok_len")
            .map_err(llvm_err)?;
        let ok_r2 = self
            .builder
            .build_insert_value(ok_r1, buf, 1, "ok_ptr")
            .map_err(llvm_err)?;
        let ok_r3 = self
            .builder
            .build_insert_value(ok_r2, self.bool_ty().const_int(1, false), 2, "ok_ok")
            .map_err(llvm_err)?;
        let _ = self.builder.build_unconditional_branch(merge_bb);
        self.builder.position_at_end(merge_bb);
        let rl_phi = self
            .builder
            .build_phi(rl_ret_ty, "rl_ret")
            .map_err(llvm_err)?;
        rl_phi.add_incoming(&[(&eof_r3, eof_bb), (&ok_r3, ok_bb)]);
        let _ = self.builder.build_return(Some(&rl_phi.as_basic_value()));

        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }
        Ok(())
    }

    pub(super) fn emit_read_dir_runtime(&self) -> Result<(), String> {
        if self.module.get_function("action_read_dir").is_some() {
            return Ok(());
        }
        let saved_pos = self.builder.get_insert_block();

        let i64 = self.i64_ty();
        let ptr = self.ptr_ty();
        let i8 = self.context.i8_type();
        let str_ty = self.string_type;
        let list_ty = self.list_type;

        let strlen_fn = self.module.get_function("strlen").unwrap();

        let rd_fn = self.module.add_function(
            "action_read_dir",
            list_ty.fn_type(&[str_ty.into()], false),
            None,
        );
        let rd_entry = self.context.append_basic_block(rd_fn, "entry");
        self.builder.position_at_end(rd_entry);
        let rd_path = rd_fn.get_first_param().unwrap().into_struct_value();
        let rd_path_data = self
            .builder
            .build_extract_value(rd_path, 1, "path_data")
            .map_err(llvm_err)?
            .into_pointer_value();

        let rd_empty = self.module.get_function("action_list_create").unwrap();
        let rd_init = self
            .builder
            .build_call(rd_empty, &[i64.const_int(0, false).into()], "rd_init")
            .map_err(llvm_err)?
            .try_as_basic_value()
            .unwrap_basic()
            .into_struct_value();

        #[cfg(not(target_os = "windows"))]
        {
            // POSIX: opendir / readdir / closedir
            let opendir_fn =
                self.module
                    .add_function("opendir", ptr.fn_type(&[ptr.into()], false), None);
            let readdir_fn =
                self.module
                    .add_function("readdir", ptr.fn_type(&[ptr.into()], false), None);
            let closedir_fn = self.module.add_function(
                "closedir",
                self.i32_ty().fn_type(&[ptr.into()], false),
                None,
            );

            let rd_dir_ptr = self
                .builder
                .build_call(opendir_fn, &[rd_path_data.into()], "dir")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let rd_dir_null = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_ptr_to_int(rd_dir_ptr, i64, "")
                        .map_err(llvm_err)?,
                    self.builder
                        .build_ptr_to_int(ptr.const_null(), i64, "")
                        .map_err(llvm_err)?,
                    "dir_null",
                )
                .map_err(llvm_err)?;
            let rd_opendir_ok_bb = self.context.append_basic_block(rd_fn, "dir_ok");
            let rd_opendir_fail_bb = self.context.append_basic_block(rd_fn, "dir_fail");
            let rd_merge_bb = self.context.append_basic_block(rd_fn, "rd_merge");
            let _ = self.builder.build_conditional_branch(
                rd_dir_null,
                rd_opendir_fail_bb,
                rd_opendir_ok_bb,
            );
            self.builder.position_at_end(rd_opendir_ok_bb);
            let rd_cur_a = self
                .builder
                .build_alloca(list_ty, "rd_cur")
                .map_err(llvm_err)?;
            self.builder
                .build_store(rd_cur_a, rd_init)
                .map_err(llvm_err)?;
            let rd_hdr = self.context.append_basic_block(rd_fn, "rd_hdr");
            let rd_bdy = self.context.append_basic_block(rd_fn, "rd_bdy");
            let rd_done = self.context.append_basic_block(rd_fn, "rd_done");
            let _ = self.builder.build_unconditional_branch(rd_hdr);
            self.builder.position_at_end(rd_hdr);
            let rd_ent = self
                .builder
                .build_call(readdir_fn, &[rd_dir_ptr.into()], "ent")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let rd_ent_null = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_ptr_to_int(rd_ent, i64, "")
                        .map_err(llvm_err)?,
                    self.builder
                        .build_ptr_to_int(ptr.const_null(), i64, "")
                        .map_err(llvm_err)?,
                    "ent_null",
                )
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(rd_ent_null, rd_done, rd_bdy);
            self.builder.position_at_end(rd_bdy);
            let rd_name = unsafe {
                self.builder
                    .build_gep(i8, rd_ent, &[i64.const_int(19, false)], "name")
                    .map_err(llvm_err)
            }?;
            let rd_nlen = self
                .builder
                .build_call(strlen_fn, &[rd_name.into()], "nlen")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let rd_asc_fn = self.module.get_function("action_string_create").unwrap();
            let rd_new_str = self
                .builder
                .build_call(rd_asc_fn, &[rd_name.into(), rd_nlen.into()], "")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            let rd_push_fn = self.module.get_function("action_list_push").unwrap();
            let rd_cur_list = self
                .builder
                .build_load(list_ty, rd_cur_a, "rd_cur_v")
                .map_err(llvm_err)?;
            let rd_pushed = self
                .builder
                .build_call(rd_push_fn, &[rd_cur_list.into(), rd_new_str.into()], "")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            self.builder
                .build_store(rd_cur_a, rd_pushed)
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rd_hdr);
            self.builder.position_at_end(rd_done);
            let _ = self
                .builder
                .build_call(closedir_fn, &[rd_dir_ptr.into()], "")
                .map_err(llvm_err)?;
            let rd_result = self
                .builder
                .build_load(list_ty, rd_cur_a, "rd_result")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rd_merge_bb);
            self.builder.position_at_end(rd_opendir_fail_bb);
            let _ = self.builder.build_unconditional_branch(rd_merge_bb);
            self.builder.position_at_end(rd_merge_bb);
            let rd_phi = self
                .builder
                .build_phi(list_ty, "rd_phi")
                .map_err(llvm_err)?;
            rd_phi.add_incoming(&[(&rd_result, rd_done), (&rd_init, rd_opendir_fail_bb)]);
            let _ = self.builder.build_return(Some(&rd_phi.as_basic_value()));
        }

        #[cfg(target_os = "windows")]
        {
            // Windows: FindFirstFileA / FindNextFileA / FindClose
            let i32 = self.context.i32_type();
            let malloc_fn = self.module.get_function("malloc").unwrap();
            let memcpy_fn = self.module.get_function("memcpy").unwrap();
            let rd_path_len = self
                .builder
                .build_extract_value(rd_path, 0, "path_len")
                .map_err(llvm_err)?
                .into_int_value();

            let ff_fn = self.module.add_function(
                "FindFirstFileA",
                ptr.fn_type(&[ptr.into(), ptr.into()], false),
                None,
            );
            let fn_fn = self.module.add_function(
                "FindNextFileA",
                i32.fn_type(&[ptr.into(), ptr.into()], false),
                None,
            );
            let fc_fn =
                self.module
                    .add_function("FindClose", i32.fn_type(&[ptr.into()], false), None);
            // WIN32_FIND_DATAA = 320 bytes; cFileName at offset 44
            let find_data_size = i64.const_int(320, false);
            let cfile_name_offset = 44u64;

            // Build search pattern: path + "\*"
            // pattern = malloc(path_len + 3)
            let pat_len = self
                .builder
                .build_int_add(rd_path_len, i64.const_int(3, false), "pat_len")
                .map_err(llvm_err)?;
            let pat_buf = self
                .builder
                .build_call(malloc_fn, &[pat_len.into()], "pat_buf")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            let _ = self
                .builder
                .build_call(
                    memcpy_fn,
                    &[pat_buf.into(), rd_path_data.into(), rd_path_len.into()],
                    "",
                )
                .map_err(llvm_err)?;
            let pat_slash = unsafe {
                self.builder
                    .build_gep(i8, pat_buf, &[rd_path_len], "pat_slash")
                    .map_err(llvm_err)
            }?;
            self.builder
                .build_store(pat_slash, i8.const_int(0x5C, false))
                .map_err(llvm_err)?;
            let pat_star = unsafe {
                self.builder
                    .build_gep(
                        i8,
                        pat_buf,
                        &[self
                            .builder
                            .build_int_add(rd_path_len, i64.const_int(1, false), "")
                            .map_err(llvm_err)?],
                        "pat_star",
                    )
                    .map_err(llvm_err)
            }?;
            self.builder
                .build_store(pat_star, i8.const_int(0x2A, false))
                .map_err(llvm_err)?;
            let pat_null = unsafe {
                self.builder
                    .build_gep(
                        i8,
                        pat_buf,
                        &[self
                            .builder
                            .build_int_add(rd_path_len, i64.const_int(2, false), "")
                            .map_err(llvm_err)?],
                        "pat_null",
                    )
                    .map_err(llvm_err)
            }?;
            self.builder
                .build_store(pat_null, i8.const_int(0, false))
                .map_err(llvm_err)?;

            // Allocate WIN32_FIND_DATAA
            let fd_ptr = self
                .builder
                .build_call(malloc_fn, &[find_data_size.into()], "fd")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();

            // FindFirstFileA(pattern, &findData)
            let h_find = self
                .builder
                .build_call(ff_fn, &[pat_buf.into(), fd_ptr.into()], "hfind")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_pointer_value();
            // INVALID_HANDLE_VALUE = -1
            let invalid_handle = self
                .builder
                .build_int_to_ptr(i64.const_int((-1i64) as u64, true), ptr, "invalid_handle")
                .map_err(llvm_err)?;
            let is_invalid = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    self.builder
                        .build_ptr_to_int(h_find, i64, "")
                        .map_err(llvm_err)?,
                    self.builder
                        .build_ptr_to_int(invalid_handle, i64, "")
                        .map_err(llvm_err)?,
                    "is_invalid",
                )
                .map_err(llvm_err)?;

            let ff_ok_bb = self.context.append_basic_block(rd_fn, "ff_ok");
            let ff_fail_bb = self.context.append_basic_block(rd_fn, "ff_fail");
            let rd_merge_bb = self.context.append_basic_block(rd_fn, "rd_merge");
            let _ = self
                .builder
                .build_conditional_branch(is_invalid, ff_fail_bb, ff_ok_bb);

            // ff_ok: iterate entries
            self.builder.position_at_end(ff_ok_bb);
            let rd_cur_a = self
                .builder
                .build_alloca(list_ty, "rd_cur")
                .map_err(llvm_err)?;
            self.builder
                .build_store(rd_cur_a, rd_init)
                .map_err(llvm_err)?;
            let rd_loop_hdr = self.context.append_basic_block(rd_fn, "rd_loop");
            let rd_loop_bdy = self.context.append_basic_block(rd_fn, "rd_body");
            let rd_loop_next = self.context.append_basic_block(rd_fn, "rd_next");
            let rd_done = self.context.append_basic_block(rd_fn, "rd_done");
            let _ = self.builder.build_unconditional_branch(rd_loop_hdr);

            // Loop header: extract filename from findData.cFileName
            self.builder.position_at_end(rd_loop_hdr);
            let rd_name = unsafe {
                self.builder
                    .build_gep(
                        i8,
                        fd_ptr,
                        &[i64.const_int(cfile_name_offset, false)],
                        "name",
                    )
                    .map_err(llvm_err)
            }?;
            // Skip "." and ".." entries
            let rd_name_first = self
                .builder
                .build_load(i8, rd_name, "first_char")
                .map_err(llvm_err)?
                .into_int_value();
            let is_dot = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    rd_name_first,
                    i8.const_int(0x2E, false),
                    "is_dot",
                )
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(is_dot, rd_loop_next, rd_loop_bdy);

            // rd_loop_bdy: add filename to list
            self.builder.position_at_end(rd_loop_bdy);
            let rd_nlen = self
                .builder
                .build_call(strlen_fn, &[rd_name.into()], "nlen")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let rd_asc_fn = self.module.get_function("action_string_create").unwrap();
            let rd_new_str = self
                .builder
                .build_call(rd_asc_fn, &[rd_name.into(), rd_nlen.into()], "")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            let rd_push_fn = self.module.get_function("action_list_push").unwrap();
            let rd_cur_list = self
                .builder
                .build_load(list_ty, rd_cur_a, "rd_cur_v")
                .map_err(llvm_err)?;
            let rd_pushed = self
                .builder
                .build_call(rd_push_fn, &[rd_cur_list.into(), rd_new_str.into()], "")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_struct_value();
            self.builder
                .build_store(rd_cur_a, rd_pushed)
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rd_loop_next);

            // rd_loop_next: FindNextFileA, branch back or done
            self.builder.position_at_end(rd_loop_next);
            let has_next = self
                .builder
                .build_call(fn_fn, &[h_find.into(), fd_ptr.into()], "has_next")
                .map_err(llvm_err)?
                .try_as_basic_value()
                .unwrap_basic()
                .into_int_value();
            let is_end = self
                .builder
                .build_int_compare(
                    IntPredicate::EQ,
                    has_next,
                    i32.const_int(0, false),
                    "is_end",
                )
                .map_err(llvm_err)?;
            let _ = self
                .builder
                .build_conditional_branch(is_end, rd_done, rd_loop_hdr);

            // rd_done: close handle and return list
            self.builder.position_at_end(rd_done);
            let _ = self
                .builder
                .build_call(fc_fn, &[h_find.into()], "")
                .map_err(llvm_err)?;
            let rd_result = self
                .builder
                .build_load(list_ty, rd_cur_a, "rd_result")
                .map_err(llvm_err)?;
            let _ = self.builder.build_unconditional_branch(rd_merge_bb);

            // ff_fail: return empty list
            self.builder.position_at_end(ff_fail_bb);
            let _ = self.builder.build_unconditional_branch(rd_merge_bb);

            // rd_merge: phi (result, init)
            self.builder.position_at_end(rd_merge_bb);
            let rd_phi = self
                .builder
                .build_phi(list_ty, "rd_phi")
                .map_err(llvm_err)?;
            rd_phi.add_incoming(&[(&rd_result, rd_done), (&rd_init, ff_fail_bb)]);
            let _ = self.builder.build_return(Some(&rd_phi.as_basic_value()));
        }

        if let Some(block) = saved_pos {
            self.builder.position_at_end(block);
        }
        Ok(())
    }
}
