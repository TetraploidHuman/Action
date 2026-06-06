// JSON runtime support for Action language
// Uses serde_json for parsing and serialization.
// Exposes opaque JsonNode handles via #[no_mangle] extern "C" functions.

use std::ffi::{c_char, c_void, CString};

use serde_json::Value;

// Prevent linker from optimizing out these symbols (called by JIT via dlsym).
#[used]
static ACTION_JSON_PARSE_PTR: unsafe extern "C" fn(*const c_char) -> *mut c_void =
    action_json_parse;
#[used]
static ACTION_JSON_STRINGIFY_PTR: unsafe extern "C" fn(*mut c_void) -> *mut c_char =
    action_json_stringify;
#[used]
static ACTION_JSON_FREE_PTR: unsafe extern "C" fn(*mut c_void) = action_json_free;
#[used]
static ACTION_JSON_TYPE_PTR: unsafe extern "C" fn(*mut c_void) -> i64 = action_json_type;
#[used]
static ACTION_JSON_GET_PTR: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void =
    action_json_get;
#[used]
static ACTION_JSON_GET_IDX_PTR: unsafe extern "C" fn(*mut c_void, i64) -> *mut c_void =
    action_json_get_idx;
#[used]
static ACTION_JSON_AS_STR_PTR: unsafe extern "C" fn(*mut c_void) -> *mut c_char =
    action_json_as_str;
#[used]
static ACTION_JSON_AS_FLOAT_PTR: unsafe extern "C" fn(*mut c_void) -> f64 = action_json_as_float;
#[used]
static ACTION_JSON_AS_BOOL_PTR: unsafe extern "C" fn(*mut c_void) -> i64 = action_json_as_bool;
#[used]
static ACTION_JSON_LEN_PTR: unsafe extern "C" fn(*mut c_void) -> i64 = action_json_len;

fn to_cstring(s: &str) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("").unwrap())
        .into_raw()
}

fn from_cstr<'a>(ptr: *const c_char) -> &'a str {
    if ptr.is_null() {
        return "";
    }
    unsafe { std::ffi::CStr::from_ptr(ptr) }
        .to_str()
        .unwrap_or("")
}

/// Parse a JSON string. Returns null on parse error.
/// The returned pointer must be freed with action_json_free().
#[no_mangle]
pub extern "C" fn action_json_parse(json_str: *const c_char) -> *mut c_void {
    let s = from_cstr(json_str);
    match serde_json::from_str::<Value>(s) {
        Ok(value) => Box::into_raw(Box::new(value)) as *mut c_void,
        Err(_) => std::ptr::null_mut(),
    }
}

/// Serialize a JsonNode to a JSON string.
/// The returned C string must be freed with free() (or action_json_free_cstr).
#[no_mangle]
pub extern "C" fn action_json_stringify(node: *mut c_void) -> *mut c_char {
    if node.is_null() {
        return to_cstring("null");
    }
    let value = unsafe { &*(node as *const Value) };
    to_cstring(&value.to_string())
}

/// Free a JsonNode tree created by action_json_parse.
/// Also frees the nodes obtained via action_json_get / action_json_get_idx
/// (which are internal pointers into the same tree).
#[no_mangle]
pub extern "C" fn action_json_free(node: *mut c_void) {
    if node.is_null() {
        return;
    }
    unsafe {
        drop(Box::from_raw(node as *mut Value));
    }
}

/// Get the type of a JsonNode.
/// Returns: 0=null, 1=bool, 2=number, 3=string, 4=array, 5=object, -1=error
#[no_mangle]
pub extern "C" fn action_json_type(node: *mut c_void) -> i64 {
    if node.is_null() {
        return -1;
    }
    let value = unsafe { &*(node as *const Value) };
    match value {
        Value::Null => 0,
        Value::Bool(_) => 1,
        Value::Number(_) => 2,
        Value::String(_) => 3,
        Value::Array(_) => 4,
        Value::Object(_) => 5,
    }
}

/// Get an object field by key. Returns null if not an object or key not found.
/// The returned pointer is an internal reference — it lives as long as the root node.
#[no_mangle]
pub extern "C" fn action_json_get(node: *mut c_void, key: *const c_char) -> *mut c_void {
    if node.is_null() {
        return std::ptr::null_mut();
    }
    let value = unsafe { &*(node as *const Value) };
    let key_str = from_cstr(key);
    match value {
        Value::Object(map) => map
            .get(key_str)
            .map(|v| v as *const Value as *mut c_void)
            .unwrap_or(std::ptr::null_mut()),
        _ => std::ptr::null_mut(),
    }
}

/// Get an array element by index. Returns null if not an array or index out of bounds.
/// The returned pointer is an internal reference — it lives as long as the root node.
#[no_mangle]
pub extern "C" fn action_json_get_idx(node: *mut c_void, idx: i64) -> *mut c_void {
    if node.is_null() || idx < 0 {
        return std::ptr::null_mut();
    }
    let value = unsafe { &*(node as *const Value) };
    match value {
        Value::Array(arr) => arr
            .get(idx as usize)
            .map(|v| v as *const Value as *mut c_void)
            .unwrap_or(std::ptr::null_mut()),
        _ => std::ptr::null_mut(),
    }
}

/// Extract string value. Returns null if not a string.
/// The returned C string must be freed with free().
#[no_mangle]
pub extern "C" fn action_json_as_str(node: *mut c_void) -> *mut c_char {
    if node.is_null() {
        return std::ptr::null_mut();
    }
    let value = unsafe { &*(node as *const Value) };
    match value {
        Value::String(s) => to_cstring(s),
        _ => std::ptr::null_mut(),
    }
}

/// Extract numeric value as f64. Returns 0.0 if not a number.
#[no_mangle]
pub extern "C" fn action_json_as_float(node: *mut c_void) -> f64 {
    if node.is_null() {
        return 0.0;
    }
    let value = unsafe { &*(node as *const Value) };
    match value {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    }
}

/// Extract boolean value. Returns -1 if not a bool.
#[no_mangle]
pub extern "C" fn action_json_as_bool(node: *mut c_void) -> i64 {
    if node.is_null() {
        return -1;
    }
    let value = unsafe { &*(node as *const Value) };
    match value {
        Value::Bool(b) => {
            if *b {
                1
            } else {
                0
            }
        }
        _ => -1,
    }
}

/// Get length of array or object. Returns -1 for other types.
#[no_mangle]
pub extern "C" fn action_json_len(node: *mut c_void) -> i64 {
    if node.is_null() {
        return -1;
    }
    let value = unsafe { &*(node as *const Value) };
    match value {
        Value::Array(arr) => arr.len() as i64,
        Value::Object(map) => map.len() as i64,
        _ => -1,
    }
}
