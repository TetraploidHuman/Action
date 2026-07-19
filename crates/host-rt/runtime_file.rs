//! Cached append FILE stream for bootstrap/hot-path `appendFile`.
//!
//! Each Action `appendFile` used to `fopen`/`fclose` per call. For bootstrap HIR
//! emit that means hundreds of thousands of open/close cycles. This host helper
//! keeps one append handle open for the current path and closes it on a
//! **path-aware** IO barrier (before `readFile`/`writeFile` of that same path).
//! Writes to other session files must not flush the HIR append handle.

use std::ffi::CString;
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::raw::c_void;
use std::path::Path;
use std::sync::Mutex;

/// Action string layout `{len, data}` with `data` pointing past an 8-byte RC header.
#[repr(C)]
pub struct HostStr {
    pub len: i64,
    pub data: *mut u8,
}

impl HostStr {
    pub(crate) const fn empty() -> Self {
        Self {
            len: 0,
            data: std::ptr::null_mut(),
        }
    }
}

fn path_bytes<'a>(path_data: *const u8, path_len: i64) -> Option<&'a [u8]> {
    if path_data.is_null() || path_len < 0 {
        return None;
    }
    Some(unsafe { std::slice::from_raw_parts(path_data, path_len as usize) })
}

/// Allocate Action-owned string buffer: `[i64 rc=1][bytes…][\0]`, return data ptr.
pub(crate) fn alloc_action_str(bytes: &[u8]) -> HostStr {
    let len = bytes.len();
    let total = 8 + len + 1;
    let raw = unsafe { libc::malloc(total) as *mut u8 };
    if raw.is_null() {
        return HostStr::empty();
    }
    unsafe {
        (raw as *mut i64).write(1);
        let data = raw.add(8);
        if len > 0 {
            std::ptr::copy_nonoverlapping(bytes.as_ptr(), data, len);
        }
        *data.add(len) = 0;
        HostStr {
            len: len as i64,
            data,
        }
    }
}

struct AppendCache {
    path: Vec<u8>,
    file: Option<File>,
}

impl AppendCache {
    const fn new() -> Self {
        Self {
            path: Vec::new(),
            file: None,
        }
    }

    fn close(&mut self) {
        if let Some(mut f) = self.file.take() {
            let _ = f.flush();
        }
        self.path.clear();
    }

    /// Close only when `path_bytes` refers to the cached append target.
    fn close_if_path(&mut self, path_bytes: &[u8]) {
        if self.file.is_some() && self.path == path_bytes {
            self.close();
        }
    }
}

static APPEND_CACHE: Mutex<AppendCache> = Mutex::new(AppendCache::new());

/// Truncating write of `content` to `path` (creates the file if needed).
///
/// Args are UTF-8 byte ranges already resolved by IR (`action_string_data`), so
/// String **slices** from `substring` work — callers must not pass slice headers.
#[no_mangle]
pub extern "C" fn action_host_file_write(
    path_data: *const u8,
    path_len: i64,
    content_data: *const u8,
    content_len: i64,
) -> i8 {
    if path_data.is_null() || path_len < 0 {
        return 0;
    }
    if content_len < 0 || (content_len > 0 && content_data.is_null()) {
        return 0;
    }

    let path_bytes = unsafe { std::slice::from_raw_parts(path_data, path_len as usize) };
    let Ok(path_str) = std::str::from_utf8(path_bytes) else {
        return 0;
    };
    let path = Path::new(path_str);
    let content = if content_len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(content_data, content_len as usize) }
    };

    let Ok(mut cache) = APPEND_CACHE.lock() else {
        return 0;
    };
    cache.close_if_path(path_bytes);

    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
    {
        Ok(mut f) => {
            if f.write_all(content).is_ok() {
                let _ = f.flush();
                1
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

/// Append `content` to `path`, reusing an open append handle when the path matches.
///
/// Args are UTF-8 byte ranges already resolved by IR (`action_string_data`).
#[no_mangle]
pub extern "C" fn action_host_file_append(
    path_data: *const u8,
    path_len: i64,
    content_data: *const u8,
    content_len: i64,
) -> i8 {
    if path_data.is_null() || path_len < 0 {
        return 0;
    }
    if content_len < 0 || (content_len > 0 && content_data.is_null()) {
        return 0;
    }

    let path_bytes = unsafe { std::slice::from_raw_parts(path_data, path_len as usize) };
    let Ok(path_str) = std::str::from_utf8(path_bytes) else {
        return 0;
    };
    let path = Path::new(path_str);
    let content = if content_len == 0 {
        &[][..]
    } else {
        unsafe { std::slice::from_raw_parts(content_data, content_len as usize) }
    };

    let Ok(mut cache) = APPEND_CACHE.lock() else {
        return 0;
    };

    let need_reopen = cache.file.is_none() || cache.path != path_bytes;
    if need_reopen {
        cache.close();
        match OpenOptions::new().create(true).append(true).open(path) {
            Ok(f) => {
                cache.file = Some(f);
                cache.path = path_bytes.to_vec();
            }
            Err(_) => return 0,
        }
    }

    match cache.file.as_mut() {
        Some(f) => {
            if f.write_all(content).is_ok() {
                1
            } else {
                cache.close();
                0
            }
        }
        None => 0,
    }
}

/// Flush/close the cached append handle **only if** it targets `path`.
///
/// Call before `readFile`/`writeFile`/`exists`/`delete`/`open` of that path so
/// interleaved session-file writes do not thrash the HIR append cache.
#[no_mangle]
pub extern "C" fn action_host_file_io_barrier(path_data: *const u8, path_len: i64) {
    let Ok(mut cache) = APPEND_CACHE.lock() else {
        return;
    };
    if path_data.is_null() || path_len < 0 {
        cache.close();
        return;
    }
    let path_bytes = unsafe { std::slice::from_raw_parts(path_data, path_len as usize) };
    cache.close_if_path(path_bytes);
}

/// Read entire file at `path` into an Action-owned String buffer.
///
/// Path bytes must already be resolved via `action_string_data` (slice-safe).
/// On failure returns `{0, null}` (same shape as the prior fopen fail path).
#[no_mangle]
pub extern "C" fn action_host_file_read(path_data: *const u8, path_len: i64) -> HostStr {
    let Some(path_bytes) = path_bytes(path_data, path_len) else {
        return HostStr::empty();
    };
    let Ok(path_str) = std::str::from_utf8(path_bytes) else {
        return HostStr::empty();
    };
    let path = Path::new(path_str);

    let Ok(mut cache) = APPEND_CACHE.lock() else {
        return HostStr::empty();
    };
    cache.close_if_path(path_bytes);
    drop(cache);

    match fs::read(path) {
        Ok(bytes) => alloc_action_str(&bytes),
        Err(_) => HostStr::empty(),
    }
}

/// Path exists as a regular file (len-aware; slice-safe).
#[no_mangle]
pub extern "C" fn action_host_file_exists(path_data: *const u8, path_len: i64) -> i8 {
    let Some(path_bytes) = path_bytes(path_data, path_len) else {
        return 0;
    };
    let Ok(path_str) = std::str::from_utf8(path_bytes) else {
        return 0;
    };
    let path = Path::new(path_str);
    let Ok(mut cache) = APPEND_CACHE.lock() else {
        return 0;
    };
    cache.close_if_path(path_bytes);
    drop(cache);
    if path.is_file() {
        1
    } else {
        0
    }
}

/// Open file with C `fopen` semantics; path/mode are len-aware (slice-safe).
///
/// Returns `FILE*` as `*mut c_void`, or null on failure / interior NUL in path or mode.
#[no_mangle]
pub extern "C" fn action_host_file_open(
    path_data: *const u8,
    path_len: i64,
    mode_data: *const u8,
    mode_len: i64,
) -> *mut c_void {
    let Some(path_bytes) = path_bytes(path_data, path_len) else {
        return std::ptr::null_mut();
    };
    if mode_data.is_null() || mode_len < 0 {
        return std::ptr::null_mut();
    }
    let mode_bytes = unsafe { std::slice::from_raw_parts(mode_data, mode_len as usize) };
    let Ok(path_c) = CString::new(path_bytes) else {
        return std::ptr::null_mut();
    };
    let Ok(mode_c) = CString::new(mode_bytes) else {
        return std::ptr::null_mut();
    };

    let Ok(mut cache) = APPEND_CACHE.lock() else {
        return std::ptr::null_mut();
    };
    cache.close_if_path(path_bytes);
    drop(cache);

    unsafe { libc::fopen(path_c.as_ptr(), mode_c.as_ptr()) as *mut c_void }
}

/// Delete file at `path` (len-aware; slice-safe). Returns 1 on success.
#[no_mangle]
pub extern "C" fn action_host_file_delete(path_data: *const u8, path_len: i64) -> i8 {
    let Some(path_bytes) = path_bytes(path_data, path_len) else {
        return 0;
    };
    let Ok(path_str) = std::str::from_utf8(path_bytes) else {
        return 0;
    };
    let path = Path::new(path_str);
    let Ok(mut cache) = APPEND_CACHE.lock() else {
        return 0;
    };
    cache.close_if_path(path_bytes);
    drop(cache);
    if fs::remove_file(path).is_ok() {
        1
    } else {
        0
    }
}
