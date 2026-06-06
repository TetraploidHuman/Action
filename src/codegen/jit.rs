// Submodule: jit
//
// JIT execution via inkwell's MCJIT engine. All platforms use the same
// code path now that the binary is statically linked and all CRT symbols
// are resolvable at JIT time.

use std::io::Write;

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    pub fn run_jit(&self) -> Result<(), String> {
        #[cfg(not(target_os = "windows"))]
        if let Err(e) = self.module.verify() {
            return Err(format!("LLVM module verification failed: {}", e));
        }

        run_via_jit(self)
    }
}

fn run_via_jit(cg: &CodeGen) -> Result<(), String> {
    let opt = match cg.opt_level {
        0 => inkwell::OptimizationLevel::None,
        1 => inkwell::OptimizationLevel::Less,
        2 => inkwell::OptimizationLevel::Default,
        _ => inkwell::OptimizationLevel::Aggressive,
    };
    let engine = cg
        .module
        .create_jit_execution_engine(opt)
        .map_err(|e| e.to_string())?;

    // Map host-provided runtime functions so the JIT can find them via
    // the symbol address rather than relying on dlsym(RTLD_DEFAULT).
    // Needed on NixOS where symbols in the main binary may not be
    // exported to the dynamic symbol table.
    map_host_symbols(cg, &engine);

    unsafe {
        let main: inkwell::execution_engine::JitFunction<unsafe extern "C" fn() -> u64> =
            engine.get_function("main").map_err(|e| e.to_string())?;
        let _exit_code = main.call();
        extern "C" {
            fn fflush(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
        }
        fflush(std::ptr::null_mut());
    }
    std::io::stdout().flush().ok();
    Ok(())
}

fn map_host_symbols(cg: &CodeGen, engine: &inkwell::execution_engine::ExecutionEngine) {
    // Map @stdin global to real libc stdin address.
    if let Some(stdin_g) = cg.module.get_global("stdin") {
        unsafe {
            extern "C" {
                static stdin: *mut std::ffi::c_void;
            }
            engine.add_global_mapping(&stdin_g, &stdin as *const _ as usize);
        }
    }

    // Map host-provided runtime functions that the module declares as
    // external. These are defined with #[no_mangle] in Rust and need to
    // be made visible to the JIT.
    //
    // HTTP / networking (src/http_runtime.rs)
    extern "C" {
        fn action_http_request(
            _: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
            _: *const std::ffi::c_char,
            _: i64,
        ) -> *mut std::ffi::c_char;
        fn action_http_free(_: *mut std::ffi::c_char);
        fn action_test_ping() -> i64;
    }
    // Concurrency / threading (src/runtime_threading.rs)
    extern "C" {
        fn action_mutex_init(_: *mut u8, _: *const u8) -> std::ffi::c_int;
        fn action_mutex_lock(_: *mut u8) -> std::ffi::c_int;
        fn action_mutex_unlock(_: *mut u8) -> std::ffi::c_int;
        fn action_mutex_destroy(_: *mut u8) -> std::ffi::c_int;
        fn action_cond_init(_: *mut u8, _: *const u8) -> std::ffi::c_int;
        fn action_cond_wait(_: *mut u8, _: *mut u8) -> std::ffi::c_int;
        fn action_cond_signal(_: *mut u8) -> std::ffi::c_int;
        fn action_cond_broadcast(_: *mut u8) -> std::ffi::c_int;
        fn action_cond_destroy(_: *mut u8) -> std::ffi::c_int;
        fn action_thread_create(
            _: *mut u64,
            _: *const u8,
            _: extern "C" fn(*mut u8) -> *mut u8,
            _: *mut u8,
        ) -> std::ffi::c_int;
        fn action_thread_join(_: u64, _: *mut *mut u8) -> std::ffi::c_int;
        fn action_thread_detach(_: u64) -> std::ffi::c_int;
        fn action_thread_cancel(_: u64) -> std::ffi::c_int;
        fn action_sleep_us(_: std::ffi::c_int) -> std::ffi::c_int;
        fn action_clock_gettime(_: std::ffi::c_int, _: *mut u8) -> std::ffi::c_int;
    }
    for name in [
        "action_http_request",
        "action_http_free",
        "action_test_ping",
        "action_mutex_init",
        "action_mutex_lock",
        "action_mutex_unlock",
        "action_mutex_destroy",
        "action_cond_init",
        "action_cond_wait",
        "action_cond_signal",
        "action_cond_broadcast",
        "action_cond_destroy",
        "action_thread_create",
        "action_thread_join",
        "action_thread_detach",
        "action_thread_cancel",
        "action_sleep_us",
        "action_clock_gettime",
    ] {
        if let Some(func) = cg.module.get_function(name) {
            let addr = match name {
                "action_http_request" => action_http_request as *const () as usize,
                "action_http_free" => action_http_free as *const () as usize,
                "action_test_ping" => action_test_ping as *const () as usize,
                "action_mutex_init" => action_mutex_init as *const () as usize,
                "action_mutex_lock" => action_mutex_lock as *const () as usize,
                "action_mutex_unlock" => action_mutex_unlock as *const () as usize,
                "action_mutex_destroy" => action_mutex_destroy as *const () as usize,
                "action_cond_init" => action_cond_init as *const () as usize,
                "action_cond_wait" => action_cond_wait as *const () as usize,
                "action_cond_signal" => action_cond_signal as *const () as usize,
                "action_cond_broadcast" => action_cond_broadcast as *const () as usize,
                "action_cond_destroy" => action_cond_destroy as *const () as usize,
                "action_thread_create" => action_thread_create as *const () as usize,
                "action_thread_join" => action_thread_join as *const () as usize,
                "action_thread_detach" => action_thread_detach as *const () as usize,
                "action_thread_cancel" => action_thread_cancel as *const () as usize,
                "action_sleep_us" => action_sleep_us as *const () as usize,
                "action_clock_gettime" => action_clock_gettime as *const () as usize,
                _ => continue,
            };
            engine.add_global_mapping(&func, addr);
        }
    }
}
