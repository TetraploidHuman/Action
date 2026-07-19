// Submodule: runtime_decl/extern_decls (R3-3)
//
// C/pthread/JSON/HTTP extern function declarations for runtime IR.

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    pub(super) fn declare_c_runtime_externs(&self) -> Result<(), String> {
        let i64 = self.i64_ty();
        let f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let i32 = self.context.i32_type();
        let i8 = self.context.i8_type();
        // Declare external C functions
        let _printf_fn = self
            .module
            .add_function("printf", i32.fn_type(&[ptr.into()], true), None);
        let _malloc_fn =
            self.module
                .add_function("malloc", ptr.fn_type(&[i64.into()], false), None);
        let _realloc_fn = self.module.add_function(
            "realloc",
            ptr.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let _free_fn = self
            .module
            .add_function("free", void.fn_type(&[ptr.into()], false), None);
        // Declare RC functions early (defined at end of define_runtime)
        let _malloc_rc_fn: inkwell::values::FunctionValue<'ctx> =
            self.module
                .add_function("action_malloc_rc", ptr.fn_type(&[i64.into()], false), None);
        // Forward-declare list functions used before their definitions
        self.module.add_function(
            "action_list_get",
            self.string_type
                .fn_type(&[self.list_type.into(), i64.into()], false),
            None,
        );
        self.module
            .add_function("action_rc_inc", void.fn_type(&[ptr.into()], false), None);
        self.module
            .add_function("action_rc_dec", void.fn_type(&[ptr.into()], false), None);
        self.module.add_function(
            "action_list_flatten",
            self.list_type.fn_type(&[self.list_type.into()], false),
            None,
        );
        self.module.add_function(
            "action_max_tree_height",
            i64.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        self.module.add_function(
            "action_list_concat",
            self.list_type
                .fn_type(&[self.list_type.into(), self.list_type.into()], false),
            None,
        );
        self.module.add_function(
            "action_list_push_subtree",
            void.fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
            None,
        );
        self.module.add_function(
            "action_list_push_leaf",
            void.fn_type(&[ptr.into(), ptr.into()], false),
            None,
        );
        self.module.add_function(
            "action_list_reverse_walk_rec",
            void.fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
            None,
        );
        self.module.add_function(
            "action_list_range_walk_rec",
            void.fn_type(
                &[ptr.into(), ptr.into(), i64.into(), ptr.into(), ptr.into()],
                false,
            ),
            None,
        );
        let _memcmp_fn = self.module.add_function(
            "memcmp",
            i32.fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
            None,
        );
        let _utf8_encode_fn = self.module.add_function(
            "action_utf8_encode",
            i64.fn_type(&[i64.into(), ptr.into()], false),
            None,
        );
        let _utf8_byte_len_fn = self.module.add_function(
            "action_utf8_byte_len",
            i64.fn_type(&[i8.into()], false),
            None,
        );
        let _sprintf_fn = self.module.add_function(
            "sprintf",
            i32.fn_type(&[ptr.into(), ptr.into()], true),
            None,
        );
        let _strlen_fn =
            self.module
                .add_function("strlen", i64.fn_type(&[ptr.into()], false), None);
        let _strchr_fn =
            self.module
                .add_function("strchr", ptr.fn_type(&[ptr.into(), i8.into()], false), None);
        let _memcpy_fn = self.module.add_function(
            "memcpy",
            ptr.fn_type(&[ptr.into(), ptr.into(), i64.into()], false),
            None,
        );
        let _qsort_fn = self.module.add_function(
            "qsort",
            void.fn_type(&[ptr.into(), i64.into(), i64.into(), ptr.into()], false),
            None,
        );
        let _memset_fn = self.module.add_function(
            "memset",
            ptr.fn_type(&[ptr.into(), i32.into(), i64.into()], false),
            None,
        );
        let _pow_fn =
            self.module
                .add_function("pow", f64.fn_type(&[f64.into(), f64.into()], false), None);
        let _fopen_fn =
            self.module
                .add_function("fopen", ptr.fn_type(&[ptr.into(), ptr.into()], false), None);
        let _fclose_fn =
            self.module
                .add_function("fclose", i32.fn_type(&[ptr.into()], false), None);
        let _fgets_fn = self.module.add_function(
            "fgets",
            ptr.fn_type(&[ptr.into(), i32.into(), ptr.into()], false),
            None,
        );
        let _fread_fn = self.module.add_function(
            "fread",
            i64.fn_type(&[ptr.into(), i64.into(), i64.into(), ptr.into()], false),
            None,
        );
        let _fwrite_fn = self.module.add_function(
            "fwrite",
            i64.fn_type(&[ptr.into(), i64.into(), i64.into(), ptr.into()], false),
            None,
        );
        let _fseek_fn = self.module.add_function(
            "fseek",
            i32.fn_type(&[ptr.into(), i64.into(), i32.into()], false),
            None,
        );
        let _ftell_fn = self
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

        // Host-side file IO (crates/host-rt/runtime_file.rs) — (ptr,len) after string_data
        let _host_file_write_fn = self.module.add_function(
            "action_host_file_write",
            i8.fn_type(
                &[ptr.into(), i64.into(), ptr.into(), i64.into()],
                false,
            ),
            None,
        );
        let _host_file_append_fn = self.module.add_function(
            "action_host_file_append",
            i8.fn_type(
                &[ptr.into(), i64.into(), ptr.into(), i64.into()],
                false,
            ),
            None,
        );
        let _host_file_read_fn = self.module.add_function(
            "action_host_file_read",
            self.string_type.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let _host_file_exists_fn = self.module.add_function(
            "action_host_file_exists",
            i8.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let _host_file_delete_fn = self.module.add_function(
            "action_host_file_delete",
            i8.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let _host_file_io_barrier_fn = self.module.add_function(
            "action_host_file_io_barrier",
            void.fn_type(&[ptr.into(), i64.into()], false),
            None,
        );
        let _host_file_open_fn = self.module.add_function(
            "action_host_file_open",
            ptr.fn_type(
                &[ptr.into(), i64.into(), ptr.into(), i64.into()],
                false,
            ),
            None,
        );
        // Bootstrap in-memory session buffers (M42) — slot + (ptr,len) append / String get
        let _host_bs_buf_clear_fn = self.module.add_function(
            "action_host_bs_buf_clear",
            i64.fn_type(&[i64.into()], false),
            None,
        );
        let _host_bs_buf_append_fn = self.module.add_function(
            "action_host_bs_buf_append",
            i64.fn_type(&[i64.into(), ptr.into(), i64.into()], false),
            None,
        );
        let _host_bs_buf_set_fn = self.module.add_function(
            "action_host_bs_buf_set",
            i64.fn_type(&[i64.into(), ptr.into(), i64.into()], false),
            None,
        );
        let _host_bs_buf_get_fn = self.module.add_function(
            "action_host_bs_buf_get",
            self.string_type.fn_type(&[i64.into()], false),
            None,
        );
        // Bootstrap Int session slots (M45) — span / line-col scalars
        let _host_bs_int_set_fn = self.module.add_function(
            "action_host_bs_int_set",
            i64.fn_type(&[i64.into(), i64.into()], false),
            None,
        );
        let _host_bs_int_get_fn = self.module.add_function(
            "action_host_bs_int_get",
            i64.fn_type(&[i64.into()], false),
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

        let _one = i64.const_int(1, false);
        Ok(())
    }
}
