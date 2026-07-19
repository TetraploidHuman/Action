//! Minimal host runtime static library for AOT executable linking.
//! Provides JSON/HTTP/threading/file C ABI symbols without pulling in LLVM.

mod http_runtime;
mod runtime_bs_buf;
mod runtime_bs_int;
mod runtime_file;
mod runtime_json;
mod runtime_threading;
