use std::path::Path;

fn main() {
    if let Ok(prefix) = std::env::var("LLVM_SYS_211_PREFIX") {
        let lib_dir = Path::new(&prefix).join("lib");
        if lib_dir.exists() {
            if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
                // Only enable /FORCE:UNRESOLVED when explicitly opted in via
                // ACTION_FORCE_LINK env var. Unconditional force-unresolved
                // masks real linker errors.
                if std::env::var("ACTION_FORCE_LINK").is_ok() {
                    println!("cargo:rustc-link-arg=/FORCE:UNRESOLVED");
                }
                println!("cargo:rustc-link-arg=/STACK:8388608");
            } else {
                println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
            }
        }
    }

    println!("cargo:rerun-if-env-changed=LLVM_SYS_211_PREFIX");
    println!("cargo:rerun-if-env-changed=ACTION_FORCE_LINK");
}
