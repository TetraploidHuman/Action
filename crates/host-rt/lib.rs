//! Minimal host runtime static library for AOT executable linking.
//! Provides JSON/HTTP/threading C ABI symbols without pulling in LLVM.

mod runtime_json;
mod http_runtime;
mod runtime_threading;
