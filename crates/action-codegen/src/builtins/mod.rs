//! Builtin function codegen: dispatch by domain (call, iter, list, stdlib, …).

mod call;
mod conversion;
mod ffi;
mod iter;
mod lazy;
mod list;
mod map;
mod nullable;
mod print;
mod range;
mod stdlib;
mod stream;
mod thread;
