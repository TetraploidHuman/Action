// Submodule: jit
//
// JIT execution via inkwell's MCJIT engine. All platforms use the same
// code path now that the binary is statically linked and all CRT symbols
// are resolvable at JIT time.

use std::io::Write;

use super::CodeGen;

impl<'ctx> CodeGen<'ctx> {
    pub fn run_jit(&self) -> Result<i64, String> {
        #[cfg(not(target_os = "windows"))]
        if let Err(e) = self.module.verify() {
            return Err(format!("LLVM module verification failed: {}", e));
        }

        run_via_jit(self)
    }
}

fn run_via_jit(cg: &CodeGen) -> Result<i64, String> {
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

    let exit_code = unsafe {
        let main: inkwell::execution_engine::JitFunction<unsafe extern "C" fn() -> u64> =
            engine.get_function("main").map_err(|e| e.to_string())?;
        let code = main.call();
        extern "C" {
            fn fflush(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
        }
        fflush(std::ptr::null_mut());
        code
    };
    std::io::stdout().flush().ok();
    Ok(exit_code as i64)
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

    // On Windows, map CRT functions that may not be visible to the JIT
    // via default symbol resolution (e.g. __acrt_iob_func used by readLine).
    #[cfg(target_os = "windows")]
    {
        // __acrt_iob_func(0) returns stdin FILE*
        if let Some(func) = cg.module.get_function("__acrt_iob_func") {
            type AcrtIob = unsafe extern "C" fn(std::ffi::c_int) -> *mut std::ffi::c_void;
            extern "C" {
                fn __acrt_iob_func(index: std::ffi::c_int) -> *mut std::ffi::c_void;
            }
            engine.add_global_mapping(&func, __acrt_iob_func as AcrtIob as usize);
        }

        // Standard C library functions — on Windows these are imported
        // from ucrtbase.dll and may not be resolvable by the JIT's default
        // symbol lookup (GetProcAddress on the main module). Map them
        // explicitly via their Rust extern "C" bindings.
        extern "C" {
            fn fgets(
                buf: *mut std::ffi::c_void,
                n: std::ffi::c_int,
                stream: *mut std::ffi::c_void,
            ) -> *mut std::ffi::c_void;
            fn malloc(size: u64) -> *mut std::ffi::c_void;
            fn free(ptr: *mut std::ffi::c_void);
            fn strlen(s: *const std::ffi::c_char) -> u64;
            fn memcpy(
                dst: *mut std::ffi::c_void,
                src: *const std::ffi::c_void,
                n: u64,
            ) -> *mut std::ffi::c_void;
            fn strcmp(s1: *const std::ffi::c_char, s2: *const std::ffi::c_char) -> std::ffi::c_int;
            fn printf(fmt: *const std::ffi::c_char, ...) -> std::ffi::c_int;
            fn fprintf(
                stream: *mut std::ffi::c_void,
                fmt: *const std::ffi::c_char,
                ...
            ) -> std::ffi::c_int;
            fn fflush(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
            fn fclose(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
            fn fopen(
                path: *const std::ffi::c_char,
                mode: *const std::ffi::c_char,
            ) -> *mut std::ffi::c_void;
            fn fread(
                ptr: *mut std::ffi::c_void,
                size: u64,
                nmemb: u64,
                stream: *mut std::ffi::c_void,
            ) -> u64;
            fn fwrite(
                ptr: *const std::ffi::c_void,
                size: u64,
                nmemb: u64,
                stream: *mut std::ffi::c_void,
            ) -> u64;
            fn fseek(
                stream: *mut std::ffi::c_void,
                offset: i64,
                whence: std::ffi::c_int,
            ) -> std::ffi::c_int;
            fn ftell(stream: *mut std::ffi::c_void) -> i64;
            fn feof(stream: *mut std::ffi::c_void) -> std::ffi::c_int;
            fn remove(path: *const std::ffi::c_char) -> std::ffi::c_int;
            fn sprintf(
                buf: *mut std::ffi::c_char,
                fmt: *const std::ffi::c_char,
                ...
            ) -> std::ffi::c_int;
            fn strtod(nptr: *const std::ffi::c_char, endptr: *mut *mut std::ffi::c_char) -> f64;
            fn strftime(
                s: *mut std::ffi::c_char,
                max: u64,
                fmt: *const std::ffi::c_char,
                tm: *const std::ffi::c_void,
            ) -> u64;
            fn strptime(
                buf: *const std::ffi::c_char,
                fmt: *const std::ffi::c_char,
                tm: *mut std::ffi::c_void,
            ) -> *mut std::ffi::c_char;
            // Math functions from ucrtbase.dll
            fn sqrt(x: f64) -> f64;
            fn sin(x: f64) -> f64;
            fn cos(x: f64) -> f64;
            fn tan(x: f64) -> f64;
            fn asin(x: f64) -> f64;
            fn acos(x: f64) -> f64;
            fn atan(x: f64) -> f64;
            fn atan2(y: f64, x: f64) -> f64;
            fn exp(x: f64) -> f64;
            fn log(x: f64) -> f64;
            fn log10(x: f64) -> f64;
            fn log2(x: f64) -> f64;
            fn pow(base: f64, exp: f64) -> f64;
            fn abs(x: std::ffi::c_int) -> std::ffi::c_int;
            fn floor(x: f64) -> f64;
            fn ceil(x: f64) -> f64;
            fn round(x: f64) -> f64;
            fn cbrt(x: f64) -> f64;
        }

        for name in [
            "fgets", "malloc", "free", "strlen", "memcpy", "strcmp", "printf", "fprintf", "fflush",
            "fclose", "fopen", "fread", "fwrite", "fseek", "ftell", "feof", "remove", "sprintf",
            "strtod", "strftime", "strptime", "sqrt", "sin", "cos", "tan", "asin", "acos", "atan",
            "atan2", "exp", "log", "log10", "log2", "pow", "abs", "floor", "ceil", "round", "cbrt",
        ] {
            if let Some(func) = cg.module.get_function(name) {
                let addr = match name {
                    "fgets" => fgets as *const () as usize,
                    "malloc" => malloc as *const () as usize,
                    "free" => free as *const () as usize,
                    "strlen" => strlen as *const () as usize,
                    "memcpy" => memcpy as *const () as usize,
                    "strcmp" => strcmp as *const () as usize,
                    "printf" => printf as *const () as usize,
                    "fprintf" => fprintf as *const () as usize,
                    "fflush" => fflush as *const () as usize,
                    "fclose" => fclose as *const () as usize,
                    "fopen" => fopen as *const () as usize,
                    "fread" => fread as *const () as usize,
                    "fwrite" => fwrite as *const () as usize,
                    "fseek" => fseek as *const () as usize,
                    "ftell" => ftell as *const () as usize,
                    "feof" => feof as *const () as usize,
                    "remove" => remove as *const () as usize,
                    "sprintf" => sprintf as *const () as usize,
                    "strtod" => strtod as *const () as usize,
                    "strftime" => strftime as *const () as usize,
                    "strptime" => strptime as *const () as usize,
                    "sqrt" => sqrt as *const () as usize,
                    "sin" => sin as *const () as usize,
                    "cos" => cos as *const () as usize,
                    "tan" => tan as *const () as usize,
                    "asin" => asin as *const () as usize,
                    "acos" => acos as *const () as usize,
                    "atan" => atan as *const () as usize,
                    "atan2" => atan2 as *const () as usize,
                    "exp" => exp as *const () as usize,
                    "log" => log as *const () as usize,
                    "log10" => log10 as *const () as usize,
                    "log2" => log2 as *const () as usize,
                    "pow" => pow as *const () as usize,
                    "abs" => abs as *const () as usize,
                    "floor" => floor as *const () as usize,
                    "ceil" => ceil as *const () as usize,
                    "round" => round as *const () as usize,
                    "cbrt" => cbrt as *const () as usize,
                    _ => continue,
                };
                engine.add_global_mapping(&func, addr);
            }
        }

        // Kernel32 functions — always available on Windows
        if let Some(func) = cg.module.get_function("GetStdHandle") {
            type Gsh = unsafe extern "C" fn(std::ffi::c_int) -> *mut std::ffi::c_void;
            extern "C" {
                fn GetStdHandle(nStdHandle: std::ffi::c_int) -> *mut std::ffi::c_void;
            }
            engine.add_global_mapping(&func, GetStdHandle as Gsh as usize);
        }
        if let Some(func) = cg.module.get_function("ReadFile") {
            type Rf = unsafe extern "C" fn(
                *mut std::ffi::c_void,
                *mut std::ffi::c_void,
                std::ffi::c_int,
                *mut std::ffi::c_int,
                *mut std::ffi::c_void,
            ) -> std::ffi::c_int;
            extern "C" {
                fn ReadFile(
                    hFile: *mut std::ffi::c_void,
                    lpBuffer: *mut std::ffi::c_void,
                    nNumberOfBytesToRead: std::ffi::c_int,
                    lpNumberOfBytesRead: *mut std::ffi::c_int,
                    lpOverlapped: *mut std::ffi::c_void,
                ) -> std::ffi::c_int;
            }
            engine.add_global_mapping(&func, ReadFile as Rf as usize);
        }
        if let Some(func) = cg.module.get_function("FindFirstFileA") {
            type Fff = unsafe extern "C" fn(
                *const std::ffi::c_char,
                *mut std::ffi::c_void,
            ) -> *mut std::ffi::c_void;
            extern "C" {
                fn FindFirstFileA(
                    lpFileName: *const std::ffi::c_char,
                    lpFindFileData: *mut std::ffi::c_void,
                ) -> *mut std::ffi::c_void;
            }
            engine.add_global_mapping(&func, FindFirstFileA as Fff as usize);
        }
        if let Some(func) = cg.module.get_function("FindNextFileA") {
            type Fnf = unsafe extern "C" fn(
                *mut std::ffi::c_void,
                *mut std::ffi::c_void,
            ) -> std::ffi::c_int;
            extern "C" {
                fn FindNextFileA(
                    hFindFile: *mut std::ffi::c_void,
                    lpFindFileData: *mut std::ffi::c_void,
                ) -> std::ffi::c_int;
            }
            engine.add_global_mapping(&func, FindNextFileA as Fnf as usize);
        }
        if let Some(func) = cg.module.get_function("FindClose") {
            type Fc = unsafe extern "C" fn(*mut std::ffi::c_void) -> std::ffi::c_int;
            extern "C" {
                fn FindClose(hFindFile: *mut std::ffi::c_void) -> std::ffi::c_int;
            }
            engine.add_global_mapping(&func, FindClose as Fc as usize);
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
    // JSON (src/runtime_json.rs)
    extern "C" {
        fn action_json_parse(_: *const std::ffi::c_char) -> *mut std::ffi::c_void;
        fn action_json_stringify(_: *mut std::ffi::c_void) -> *mut std::ffi::c_char;
        fn action_json_free(_: *mut std::ffi::c_void);
        fn action_json_type(_: *mut std::ffi::c_void) -> i64;
        fn action_json_get(
            _: *mut std::ffi::c_void,
            _: *const std::ffi::c_char,
        ) -> *mut std::ffi::c_void;
        fn action_json_get_idx(_: *mut std::ffi::c_void, _: i64) -> *mut std::ffi::c_void;
        fn action_json_as_str(_: *mut std::ffi::c_void) -> *mut std::ffi::c_char;
        fn action_json_as_float(_: *mut std::ffi::c_void) -> f64;
        fn action_json_as_bool(_: *mut std::ffi::c_void) -> i64;
        fn action_json_len(_: *mut std::ffi::c_void) -> i64;
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
        "action_json_parse",
        "action_json_stringify",
        "action_json_free",
        "action_json_type",
        "action_json_get",
        "action_json_get_idx",
        "action_json_as_str",
        "action_json_as_float",
        "action_json_as_bool",
        "action_json_len",
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
                "action_json_parse" => action_json_parse as *const () as usize,
                "action_json_stringify" => action_json_stringify as *const () as usize,
                "action_json_free" => action_json_free as *const () as usize,
                "action_json_type" => action_json_type as *const () as usize,
                "action_json_get" => action_json_get as *const () as usize,
                "action_json_get_idx" => action_json_get_idx as *const () as usize,
                "action_json_as_str" => action_json_as_str as *const () as usize,
                "action_json_as_float" => action_json_as_float as *const () as usize,
                "action_json_as_bool" => action_json_as_bool as *const () as usize,
                "action_json_len" => action_json_len as *const () as usize,
                _ => continue,
            };
            engine.add_global_mapping(&func, addr);
        }
    }
}
