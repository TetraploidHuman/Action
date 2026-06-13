// Platform-abstracted concurrency primitives exported as `action_*` symbols.
//
// Each function is #[no_mangle] pub extern "C" so the JIT can resolve it
// via ExecutionEngine::add_global_mapping() (see codegen/jit.rs).
//
// On Linux:   thin wrappers around pthread/libc functions.
// On Windows: thin wrappers around kernel32.dll primitives.

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
        fn Sleep(dwMilliseconds: u32);
        fn GetSystemTimePreciseAsFileTime(lpSystemTimeAsFileTime: *mut u64);
        fn QueryPerformanceCounter(lpPerformanceCount: *mut i64) -> i32;
        fn QueryPerformanceFrequency(lpFrequency: *mut i64) -> i32;
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
    pub extern "C" fn action_thread_cancel(_thread: u64) -> c_int {
        // TerminateThread is unsafe: it kills the thread without running
        // destructors, releasing critical sections, or freeing memory.
        // Return an error — cooperative cancellation should be used instead.
        -1
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
    pub extern "C" fn action_clock_gettime(clockid: c_int, ts: *mut u8) -> c_int {
        if ts.is_null() {
            return -1;
        }
        const CLOCK_REALTIME: c_int = 0;
        const CLOCK_MONOTONIC: c_int = 1; // also CLOCK_MONOTONIC_RAW on Linux

        // timespec layout: two i64 fields (tv_sec, tv_nsec), 16 bytes total.
        // Even though Windows c_long is 4 bytes, we match the Linux layout
        // because the JIT-generated caller expects two 8-byte fields.
        unsafe {
            match clockid {
                CLOCK_REALTIME => {
                    let mut ft: u64 = 0;
                    GetSystemTimePreciseAsFileTime(&mut ft);
                    // FILETIME: 100-nanosecond intervals since 1601-01-01.
                    // Unix epoch offset: 11644473600 seconds from 1601 to 1970.
                    const EPOCH_100NS: u64 = 116444736000000000;
                    if ft < EPOCH_100NS {
                        return -1;
                    }
                    let since_epoch = ft - EPOCH_100NS;
                    let sec = (since_epoch / 10_000_000) as i64;
                    let nsec = ((since_epoch % 10_000_000) * 100) as i64;
                    std::ptr::write(ts as *mut i64, sec);
                    std::ptr::write(ts.add(8) as *mut i64, nsec);
                }
                CLOCK_MONOTONIC => {
                    let mut count: i64 = 0;
                    let mut freq: i64 = 0;
                    if QueryPerformanceCounter(&mut count) == 0 {
                        return -1;
                    }
                    if QueryPerformanceFrequency(&mut freq) == 0 || freq == 0 {
                        return -1;
                    }
                    let sec = count / freq;
                    let nsec = ((count % freq).abs() * 1_000_000_000) / freq.abs();
                    std::ptr::write(ts as *mut i64, sec);
                    std::ptr::write(ts.add(8) as *mut i64, nsec);
                }
                _ => return -1,
            }
        }
        0
    }
}
