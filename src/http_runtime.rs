// HTTP runtime support for Atomic language
// Uses system curl via std::process::Command for HTTP/HTTPS requests.
// From the Atomic language perspective, httpRequest() is a built-in primitive —
// the implementation details are the compiler's concern.
use std::ffi::CString;
use std::os::raw::c_char;
use std::process::Command;

// Preserve symbols from being optimized out by the linker.
// These are called by JIT-compiled Atomic code via dlsym.
#[used]
static ATOMIC_HTTP_REQUEST_PTR: unsafe extern "C" fn(
    *const c_char,
    *const c_char,
    *const c_char,
    *const c_char,
    i64,
) -> *mut c_char = action_http_request;
#[used]
static ATOMIC_HTTP_FREE_PTR: unsafe extern "C" fn(*mut c_char) = action_http_free;

// Simple test function to verify JIT FFI works
#[no_mangle]
pub extern "C" fn action_test_ping() -> i64 {
    42
}

#[used]
static ATOMIC_TEST_PING_PTR: unsafe extern "C" fn() -> i64 = action_test_ping;

/// Perform an HTTP request using system curl.
///
/// Parameters:
///   method    - HTTP method ("GET", "POST", "PUT", "DELETE", "PATCH")
///   url       - Full URL including https://
///   headers   - Headers as "Name: Value\n" separated lines
///   body      - Request body (null if no body)
///   body_len  - Length of body in bytes (0 if no body)
///
/// Returns a C string in format "STATUS_CODE\nRESPONSE_BODY"
/// On error, returns "0\nError message"
/// Caller must free with action_http_free()
const ALLOWED_METHODS: &[&str] = &["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"];

fn validate_url(url: &str) -> Result<(), String> {
    if url.is_empty() {
        return Err("URL is empty".to_string());
    }
    // Reject URLs with embedded newlines/carriage returns (header injection via URL)
    if url.contains('\n') || url.contains('\r') {
        return Err("URL contains invalid characters".to_string());
    }
    // Basic scheme check: must start with http:// or https://
    let lower = url.to_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        return Err(format!(
            "URL scheme not allowed: only http and https are supported"
        ));
    }
    // Reject URLs targeting loopback / internal addresses via raw IPv4/IPv6
    // (defense-in-depth SSRF prevention; this is best-effort)
    if let Some(authority) = url.split("://").nth(1).and_then(|s| s.split('/').next()) {
        let host = authority.split(':').next().unwrap_or("");
        let host_lower = host.to_lowercase();
        if host_lower == "localhost"
            || host_lower == "127.0.0.1"
            || host_lower == "[::1]"
            || host_lower == "::1"
            || host_lower.starts_with("127.")
            || host_lower.starts_with("10.")
            || host_lower.starts_with("192.168.")
            || host_lower.starts_with("172.") && {
                let parts: Vec<&str> = host_lower.split('.').collect();
                parts.len() == 4
                    && parts[1]
                        .parse::<u32>()
                        .map_or(false, |n| n >= 16 && n <= 31)
            }
        {
            return Err(format!("URL targets a private/internal address: {}", host));
        }
    }
    Ok(())
}

fn validate_method(method: &str) -> Result<(), String> {
    if !ALLOWED_METHODS.contains(&method.to_uppercase().as_str()) {
        return Err(format!(
            "Unknown HTTP method '{}'. Allowed: {}",
            method,
            ALLOWED_METHODS.join(", ")
        ));
    }
    Ok(())
}

fn validate_header_line(header: &str) -> Result<(), String> {
    if header.contains('\r') || header.contains('\n') {
        return Err("Header line contains invalid characters (CR/LF)".to_string());
    }
    Ok(())
}

#[no_mangle]
pub extern "C" fn action_http_request(
    method: *const c_char,
    url: *const c_char,
    headers: *const c_char,
    body: *const c_char,
    body_len: i64,
) -> *mut c_char {
    let method = unsafe { std::ffi::CStr::from_ptr(method) }
        .to_str()
        .unwrap_or("GET");
    let url = unsafe { std::ffi::CStr::from_ptr(url) }
        .to_str()
        .unwrap_or("");

    // Validate inputs before executing the request
    if let Err(e) = validate_url(url) {
        let err = format!("0\nInvalid URL: {}", e);
        return CString::new(err)
            .unwrap_or_else(|_| CString::new("0\nError").unwrap())
            .into_raw();
    }
    if let Err(e) = validate_method(method) {
        let err = format!("0\nInvalid method: {}", e);
        return CString::new(err)
            .unwrap_or_else(|_| CString::new("0\nError").unwrap())
            .into_raw();
    }

    let headers_str = unsafe { std::ffi::CStr::from_ptr(headers) }
        .to_str()
        .unwrap_or("");

    let mut cmd = Command::new("curl");
    cmd.arg("-s") // silent mode
        .arg("-i") // include response headers
        .arg("--max-time")
        .arg("120") // timeout
        .arg("-X")
        .arg(method)
        .arg(url);

    // Parse and add headers
    for h in headers_str.lines() {
        let h = h.trim();
        if !h.is_empty() {
            if let Err(e) = validate_header_line(h) {
                let err = format!("0\nInvalid header: {}", e);
                return CString::new(err)
                    .unwrap_or_else(|_| CString::new("0\nError").unwrap())
                    .into_raw();
            }
            cmd.arg("-H").arg(h);
        }
    }

    // Add body if present
    if !body.is_null() && body_len > 0 {
        let body_bytes =
            unsafe { std::slice::from_raw_parts(body as *const u8, body_len as usize) };
        let body_str = std::str::from_utf8(body_bytes).unwrap_or("");
        cmd.arg("-d").arg(body_str);
    }

    match cmd.output() {
        Ok(output) => {
            let raw = String::from_utf8_lossy(&output.stdout);
            // Parse HTTP response: may contain proxy CONNECT tunnel headers
            // before the actual response (e.g. "HTTP/1.1 200 Connection established\r\n\r\n").
            // Skip past any leading header blocks whose status line is a CONNECT response.
            let mut search_from = 0usize;
            let body_start = loop {
                let next = raw[search_from..]
                    .find("\r\n\r\n")
                    .map(|i| search_from + i + 4)
                    .or_else(|| raw[search_from..].find("\n\n").map(|i| search_from + i + 2))
                    .unwrap_or(raw.len());
                let after_headers = &raw[next..];
                if after_headers.starts_with("HTTP/") {
                    search_from = next;
                } else {
                    break next;
                }
            };

            let headers_part = &raw[..body_start.saturating_sub(2)];
            let response_body = &raw[body_start..];

            // Extract status code from the LAST HTTP status line (skip proxy CONNECT responses).
            let status_code = headers_part
                .lines()
                .filter(|line| {
                    line.starts_with("HTTP/") && !line.contains("Connection established")
                })
                .last()
                .and_then(|line| line.split_whitespace().nth(1))
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or(0);

            let result = format!("{}\n{}", status_code, response_body.trim_end());
            CString::new(result)
                .unwrap_or_else(|_| CString::new("0\nEncoding error").unwrap())
                .into_raw()
        }
        Err(e) => {
            let err = format!("0\nHTTP request failed: {}", e);
            CString::new(err)
                .unwrap_or_else(|_| CString::new("0\nError").unwrap())
                .into_raw()
        }
    }
}

/// Free a string returned by action_http_request
#[no_mangle]
pub extern "C" fn action_http_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            drop(CString::from_raw(ptr));
        }
    }
}
