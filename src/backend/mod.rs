//! LLVM codegen, JIT/AOT, and embedded runtime IR.
//!
//! Depends on `frontend` only through public crate re-exports (`ast`, `typecheck`, …).

pub mod codegen;

pub use codegen::CodeGen;
