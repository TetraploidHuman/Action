// Platform-abstracted concurrency primitives exported as `action_*` symbols.
//
// Each function is #[no_mangle] pub extern "C" so the JIT can resolve it
// via ExecutionEngine::add_global_mapping() (see codegen/jit.rs).
//
// On Linux:   thin wrappers around pthread/libc functions.
// On Windows: thin wrappers around kernel32.dll primitives.

use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Linux — delegate to pthread / libc
// ---------------------------------------------------------------------------

#[cfg(not(target_os = "windows"))]
mod platform {
    use std::ffi::c_int;

    extern "C" {
        fn pthread_mutex_init(mutex: *mut u8, attr: *const u8) -> c_int;
        fn pthread_mutex_lock(mutex: *mut u8) -> c_int;
        fn pthread_mutex_unlock(mutex: *mut u8) -> c_int;
        fn pthread_mutex_destroy(mutex: *mut u8) -> c_int;
        fn pthread_cond_init(cond: *mut u8, attr: *const u8) -> c_int;
        fn pthread_cond_wait(cond: *mut u8, mutex: *mut u8) -> c_int;
        fn pthread_cond_signal(cond: *mut u8) -> c_int;
        fn pthread_cond_broadcast(cond: *mut u8) -> c_int;
        fn pthread_cond_destroy(cond: *mut u8) -> c_int;
        fn pthread_create(
            tid: *mut u64,
            attr: *const u8,
            func: extern "C" fn(*mut u8) -> *mut u8,
            arg: *mut u8,
        ) -> c_int;
        fn pthread_join(thread: u64, retval: *mut *mut u8) -> c_int;
        fn pthread_detach(thread: u64) -> c_int;
        fn pthread_cancel(thread: u64) -> c_int;
        fn usleep(usec: c_int) -> c_int;
        fn clock_gettime(clockid: c_int, ts: *mut u8) -> c_int;
    }

    #[no_mangle]
    pub extern "C" fn action_mutex_init(mutex: *mut u8, _attr: *const u8) -> c_int {
        unsafe { pthread_mutex_init(mutex, std::ptr::null()) }
    }

    #[no_mangle]
    pub extern "C" fn action_mutex_lock(mutex: *mut u8) -> c_int {
        unsafe { pthread_mutex_lock(mutex) }
    }

    #[no_mangle]
    pub extern "C" fn action_mutex_unlock(mutex: *mut u8) -> c_int {
        unsafe { pthread_mutex_unlock(mutex) }
    }

    #[no_mangle]
    pub extern "C" fn action_mutex_destroy(mutex: *mut u8) -> c_int {
        unsafe { pthread_mutex_destroy(mutex) }
    }

    #[no_mangle]
    pub extern "C" fn action_cond_init(cond: *mut u8, _attr: *const u8) -> c_int {
        unsafe { pthread_cond_init(cond, std::ptr::null()) }
    }

    #[no_mangle]
    pub extern "C" fn action_cond_wait(cond: *mut u8, mutex: *mut u8) -> c_int {
        unsafe { pthread_cond_wait(cond, mutex) }
    }

    #[no_mangle]
    pub extern "C" fn action_cond_signal(cond: *mut u8) -> c_int {
        unsafe { pthread_cond_signal(cond) }
    }

    #[no_mangle]
    pub extern "C" fn action_cond_broadcast(cond: *mut u8) -> c_int {
        unsafe { pthread_cond_broadcast(cond) }
    }

    #[no_mangle]
    pub extern "C" fn action_cond_destroy(cond: *mut u8) -> c_int {
        unsafe { pthread_cond_destroy(cond) }
    }

    #[no_mangle]
    pub extern "C" fn action_thread_create(
        tid: *mut u64,
        _attr: *const u8,
        func: extern "C" fn(*mut u8) -> *mut u8,
        arg: *mut u8,
    ) -> c_int {
        unsafe { pthread_create(tid, std::ptr::null(), func, arg) }
    }

    #[no_mangle]
    pub extern "C" fn action_thread_join(thread: u64, retval: *mut *mut u8) -> c_int {
        unsafe { pthread_join(thread, retval) }
    }

    #[no_mangle]
    pub extern "C" fn action_thread_detach(thread: u64) -> c_int {
        unsafe { pthread_detach(thread) }
    }

    #[no_mangle]
    pub extern "C" fn action_thread_cancel(thread: u64) -> c_int {
        unsafe { pthread_cancel(thread) }
    }

    #[no_mangle]
    pub extern "C" fn action_sleep_us(usec: c_int) -> c_int {
        unsafe { usleep(usec) }
    }

    #[no_mangle]
    pub extern "C" fn action_clock_gettime(clockid: c_int, ts: *mut u8) -> c_int {
        unsafe { clock_gettime(clockid, ts) }
    }
}

// ---------------------------------------------------------------------------
// Windows — delegate to kernel32.dll
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
mod platform {
    use std::ffi::c_int;

    extern "system" {
        fn CreateThread(
            lpThreadAttributes: *mut u8,
            dwStackSize: usize,
            lpStartAddress: extern "system" fn(*mut u8) -> u32,
            lpParameter: *mut u8,
            dwCreationFlags: u32,
            lpThreadId: *mut u32,
        ) -> *mut u8; // HANDLE
        fn WaitForSingleObject(hHandle: *mut u8, dwMilliseconds: u32) -> u32;
        fn CloseHandle(hObject: *mut u8) -> i32;
        fn TerminateThread(hThread: *mut u8, dwExitCode: u32) -> i32;
        fn Sleep(dwMilliseconds: u32);
        fn InitializeCriticalSection(lpCriticalSection: *mut u8);
        fn EnterCriticalSection(lpCriticalSection: *mut u8);
        fn LeaveCriticalSection(lpCriticalSection: *mut u8);
        fn DeleteCriticalSection(lpCriticalSection: *mut u8);
        fn InitializeConditionVariable(ConditionVariable: *mut u8);
        fn SleepConditionVariableCS(
            ConditionVariable: *mut u8,
            CriticalSection: *mut u8,
            dwMilliseconds: u32,
        ) -> i32;
        fn WakeConditionVariable(ConditionVariable: *mut u8);
        fn WakeAllConditionVariable(ConditionVariable: *mut u8);
    }

    const INFINITE: u32 = 0xFFFF_FFFF;

    #[no_mangle]
    pub extern "C" fn action_mutex_init(mutex: *mut u8, _attr: *const u8) -> c_int {
        unsafe {
            InitializeCriticalSection(mutex);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_mutex_lock(mutex: *mut u8) -> c_int {
        unsafe {
            EnterCriticalSection(mutex);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_mutex_unlock(mutex: *mut u8) -> c_int {
        unsafe {
            LeaveCriticalSection(mutex);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_mutex_destroy(mutex: *mut u8) -> c_int {
        unsafe {
            DeleteCriticalSection(mutex);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_cond_init(cond: *mut u8, _attr: *const u8) -> c_int {
        unsafe {
            InitializeConditionVariable(cond);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_cond_wait(cond: *mut u8, mutex: *mut u8) -> c_int {
        unsafe {
            SleepConditionVariableCS(cond, mutex, INFINITE);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_cond_signal(cond: *mut u8) -> c_int {
        unsafe {
            WakeConditionVariable(cond);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_cond_broadcast(cond: *mut u8) -> c_int {
        unsafe {
            WakeAllConditionVariable(cond);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_cond_destroy(_cond: *mut u8) -> c_int {
        // CONDITION_VARIABLE on Windows does not require explicit destruction.
        0
    }

    #[no_mangle]
    pub extern "C" fn action_thread_create(
        tid: *mut u64,
        _attr: *const u8,
        func: extern "C" fn(*mut u8) -> *mut u8,
        arg: *mut u8,
    ) -> c_int {
        unsafe {
            let handle = CreateThread(
                std::ptr::null_mut(), // lpThreadAttributes
                0,                    // dwStackSize (default)
                std::mem::transmute::<
                    extern "C" fn(*mut u8) -> *mut u8,
                    extern "system" fn(*mut u8) -> u32,
                >(func),
                arg,
                0,                    // dwCreationFlags
                std::ptr::null_mut(), // lpThreadId (not needed)
            );
            if handle.is_null() {
                return -1;
            }
            *tid = handle as u64;
            0
        }
    }

    #[no_mangle]
    pub extern "C" fn action_thread_join(thread: u64, _retval: *mut *mut u8) -> c_int {
        unsafe {
            let handle = thread as *mut u8;
            WaitForSingleObject(handle, INFINITE);
            CloseHandle(handle);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_thread_detach(thread: u64) -> c_int {
        unsafe {
            CloseHandle(thread as *mut u8);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_thread_cancel(thread: u64) -> c_int {
        unsafe {
            TerminateThread(thread as *mut u8, 0);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_sleep_us(usec: c_int) -> c_int {
        let ms = if usec <= 0 {
            0u32
        } else {
            (usec as u32 + 999) / 1000
        };
        unsafe {
            Sleep(ms);
        }
        0
    }

    #[no_mangle]
    pub extern "C" fn action_clock_gettime(_clockid: c_int, _ts: *mut u8) -> c_int {
        // clock_gettime is not available on Windows.
        // Not currently called by any codegen path.
        0
    }
}
