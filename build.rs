use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    configure_llvm_linking();
    regenerate_list_body_includes();
    build_host_runtime_staticlib();
}

/// Regenerate list core/tree `body.inc.rs` when fragment `.inc.rs` files change (R4-7).
fn regenerate_list_body_includes() {
    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => return,
    };
    let list_base = manifest_dir.join("crates/action-codegen/src/runtime_decl/list");
    for sub in ["core", "tree"] {
        let dir = list_base.join(sub);
        if !dir.exists() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "rs")
                    && path
                        .file_name()
                        .is_some_and(|n| n != "mod.rs" && n != "body.inc.rs")
                {
                    println!("cargo:rerun-if-changed={}", path.display());
                }
            }
        }
    }
    println!("cargo:rerun-if-changed=scripts/concat_list_body.py");

    let script = manifest_dir.join("scripts/concat_list_body.py");
    if !script.exists() {
        return;
    }
    let status = Command::new("python3")
        .arg(&script)
        .current_dir(&manifest_dir)
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            panic!(
                "concat_list_body.py failed (exit {}); list body.inc.rs not regenerated",
                s.code().unwrap_or(-1)
            );
        }
        Err(e) => panic!("failed to run concat_list_body.py: {e}"),
    }
}

fn configure_llvm_linking() {
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

/// Build `libaction_host_rt.a` for AOT executable linking (JSON/HTTP/threading C ABI).
fn build_host_runtime_staticlib() {
    if std::env::var("ACTION_BUILDING_RUNTIME_BC").is_ok()
        || std::env::var("ACTION_BUILDING_HOST_RT").is_ok()
    {
        return;
    }

    let manifest_dir = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => return,
    };
    let host_rt_manifest = manifest_dir.join("crates/host-rt/Cargo.toml");
    if !host_rt_manifest.exists() {
        return;
    }

    println!("cargo:rerun-if-changed=crates/host-rt/");
    println!("cargo:rerun-if-changed=crates/host-rt/runtime_json.rs");
    println!("cargo:rerun-if-changed=crates/host-rt/http_runtime.rs");
    println!("cargo:rerun-if-changed=crates/host-rt/runtime_threading.rs");

    let profile = if std::env::var("PROFILE").unwrap_or_default() == "release" {
        "release"
    } else {
        "debug"
    };
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| manifest_dir.join("target"));

    let host_rt_target = target_dir.join("host_rt_build");
    let lib_name = if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        "action_host_rt.lib"
    } else {
        "libaction_host_rt.a"
    };
    let lib_path = host_rt_target.join(profile).join(lib_name);
    emit_host_rt_link(&host_rt_target, profile);
    let sources = [
        host_rt_manifest.clone(),
        manifest_dir.join("crates/host-rt/runtime_json.rs"),
        manifest_dir.join("crates/host-rt/http_runtime.rs"),
        manifest_dir.join("crates/host-rt/runtime_threading.rs"),
    ];
    if lib_path.exists() && !host_rt_sources_changed(&lib_path, &sources) {
        return;
    }

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&manifest_dir);
    cmd.env("ACTION_BUILDING_HOST_RT", "1");
    // Separate target dir: nested build must not share the outer artifact lock (CI deadlock).
    let host_rt_target = target_dir.join("host_rt_build");
    cmd.env("CARGO_TARGET_DIR", &host_rt_target);
    cmd.args(["build", "--manifest-path"])
        .arg(&host_rt_manifest);
    if profile == "release" {
        cmd.arg("--release");
    }

    let status = cmd.status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => {
            eprintln!(
                "cargo:warning=action-host-rt build failed (exit {}); AOT --emit exe may fail to link",
                s.code().unwrap_or(-1)
            );
        }
        Err(e) => {
            eprintln!("cargo:warning=failed to spawn action-host-rt build: {e}");
        }
    }
}

fn emit_host_rt_link(host_rt_target: &Path, profile: &str) {
    let lib_dir = host_rt_target.join(profile);
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    // AOT `--emit exe` links libaction_host_rt.a explicitly in main.rs; search path only here.
    let _ = profile;
}

fn host_rt_sources_changed(lib: &Path, sources: &[PathBuf]) -> bool {
    let Ok(lib_mtime) = std::fs::metadata(lib).and_then(|m| m.modified()) else {
        return true;
    };
    sources.iter().any(|src| {
        std::fs::metadata(src)
            .and_then(|m| m.modified())
            .map(|t| t > lib_mtime)
            .unwrap_or(true)
    })
}
