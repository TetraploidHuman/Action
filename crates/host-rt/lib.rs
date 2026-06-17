//! Minimal host runtime static library for AOT executable linking.
//! Provides JSON/HTTP/threading C ABI symbols without pulling in LLVM.

#[path = "../../src/runtime_json.rs"]
mod runtime_json;

#[path = "../../src/http_runtime.rs"]
mod http_runtime;

#[path = "../../src/runtime_threading.rs"]
mod runtime_threading;
