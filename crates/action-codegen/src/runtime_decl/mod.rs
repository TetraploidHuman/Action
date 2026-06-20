// Submodule: runtime_decl

use super::{llvm_err, CodeGen};
use action_frontend::typecheck::TypeRegistry;
use inkwell::context::Context;
use inkwell::memory_buffer::MemoryBuffer;
use inkwell::module::Module;
use std::sync::OnceLock;

// Validated at build time by build.rs (not linked at runtime — see define_runtime).
include!(concat!(env!("OUT_DIR"), "/runtime_bc_embed.rs"));

/// Process-wide cache of LLVM bitcode for the runtime module (List/Map/String/RC etc.).
/// Populated on the first `define_runtime` call; subsequent compilations link this in
/// instead of regenerating thousands of lines of IR.
static RUNTIME_BITCODE: OnceLock<Vec<u8>> = OnceLock::new();

fn link_runtime_bitcode_into<'ctx>(
    module: &Module<'ctx>,
    context: &'ctx Context,
    bitcode: &[u8],
) -> Result<(), String> {
    let buffer = MemoryBuffer::create_from_memory_range_copy(bitcode, "action_runtime.bc");
    let runtime_mod =
        Module::parse_bitcode_from_buffer(&buffer, context).map_err(|e| e.to_string())?;
    module
        .link_in_module(runtime_mod)
        .map_err(|e| e.to_string())
}

impl<'ctx> CodeGen<'ctx> {
    /// Create a global string constant in the LLVM module.
    pub(super) fn make_global_str(
        &self,
        name: &str,
        content: &[u8],
    ) -> Result<inkwell::values::PointerValue<'ctx>, String> {
        let i8 = self.context.i8_type();
        let arr_ty = i8.array_type(content.len() as u32);
        let global = self.add_module_global(arr_ty, name)?;
        let arr = self.context.const_string(content, false);
        global.set_initializer(&arr);
        Ok(global.as_pointer_value())
    }

    pub(super) fn define_runtime(&self) -> Result<(), String> {
        // Build-time embed is validated only; linking duplicates LLVM types from CodeGen::new().
        let _ = EMBEDDED_RUNTIME_BC;
        if let Some(bitcode) = RUNTIME_BITCODE.get() {
            return link_runtime_bitcode_into(&self.module, self.context, bitcode);
        }

        self.define_runtime_generate()?;

        if RUNTIME_BITCODE.get().is_none() {
            let mem = self.module.write_bitcode_to_memory();
            let _ = RUNTIME_BITCODE.set(mem.as_slice().to_vec());
        }
        Ok(())
    }

    /// Emit LLVM bitcode for the Action runtime (build.rs / runtime-bc-emit).
    pub fn generate_runtime_bitcode() -> Result<Vec<u8>, String> {
        let context = Context::create();
        let registry = TypeRegistry::default();
        let cg = CodeGen::new(&context, "action_runtime", registry, None);
        cg.define_runtime_generate()?;
        cg.module
            .verify()
            .map_err(|e| format!("runtime bitcode verify failed: {e}"))?;
        Ok(cg.module.write_bitcode_to_memory().as_slice().to_vec())
    }

    fn define_runtime_generate(&self) -> Result<(), String> {
        #![allow(unused_macros)]
        let i64 = self.i64_ty();
        let f64 = self.f64_ty();
        let void = self.void_ty();
        let ptr = self.ptr_ty();
        let _str_ty = self.string_type;
        let _b1 = self.bool_ty();
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

        // === Define group closures ===

        // === Execute runtime function groups ===
        self.define_str_core()?;
        self.define_print()?;
        self.define_str_basic()?;
        self.define_list_core()?;
        self.define_lazy_list()?;
        self.define_list_insert_rec()?;
        self.define_list_iter()?;
        self.define_list_xform()?;
        self.define_str_util()?;
        self.define_hash_table()?;
        self.define_map()?;
        self.define_str_extra()?;
        self.define_file_parse()?;
        self.define_rand()?;
        self.define_str_adv()?;
        self.define_list_extra()?;
        self.define_list_tree()?;
        self.define_math_ms()?;
        self.define_misc()?;
        self.apply_runtime_fn_attrs();
        Ok(())
    }
}

// ---- Submodules ----
mod define_file_parse;
mod define_hash_table;
mod define_list_core;
mod define_lazy_list;
mod define_list_extra;
mod define_list_insert_rec;
mod define_list_iter;
mod define_list_tree;
mod define_list_xform;
mod define_map;
mod define_math_ms;
mod define_misc;
mod define_print;
mod define_rand;
mod define_str_adv;
mod define_str_basic;
mod define_str_core;
mod define_str_extra;
mod define_str_util;
