//! LLVM target initialization gated by inkwell feature flags (Windows: x86 only).

use inkwell::targets::{InitializationConfig, Target};

pub fn init_x86() {
    Target::initialize_x86(&InitializationConfig::default());
}

#[cfg(not(target_os = "windows"))]
pub fn init_aarch64() {
    Target::initialize_aarch64(&InitializationConfig::default());
}

#[cfg(not(target_os = "windows"))]
pub fn init_webassembly() {
    Target::initialize_webassembly(&InitializationConfig::default());
}

/// X86 plus cross targets when inkwell features allow (aarch64, wasm).
pub fn init_for_cross_triple() {
    init_x86();
    #[cfg(not(target_os = "windows"))]
    {
        init_aarch64();
        init_webassembly();
    }
}

/// Host JIT: x86 everywhere; aarch64 on non-Windows for cross-target tests.
pub fn init_for_jit() {
    init_x86();
    #[cfg(not(target_os = "windows"))]
    init_aarch64();
}
