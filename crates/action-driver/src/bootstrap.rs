//! Path B bootstrap frontend gate (M76).
//!
//! User source is typechecked/lowered by `bootstrap/compiler.ac` (Action-in-Action),
//! then consumed as HIR JSON by Rust `compile_hir`. The Rust parse/typecheck of the
//! *user* file is skipped.

use action_codegen::CodeGen;
use action_frontend::hir::HirModule;
use action_frontend::type_registry::TypeRegistry;
use inkwell::context::Context;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Fixture stems (and basename stems) allowed for `--frontend bootstrap`.
/// Keep in sync with `BOOTSTRAP_FIXTURE_STEMS` positives in `tests/bootstrap_subset.rs`
/// (subset gate — not the full language).
pub const BOOTSTRAP_FRONTEND_ALLOWLIST: &[&str] = &[
    "arith_add_string_ok",
    "arith_ok",
    "assign_expr",
    "assign_point_ok",
    "call_point_ok",
    "coll_homo_ok",
    "cmp_ok",
    "control_flow",
    "custom_struct",
    "many_structs",
    "custom_enum",
    "enum_simple",
    "env_scope_good",
    "field_assign_ok",
    "for_break",
    "for_cond_ok",
    "for_continue",
    "for_index",
    "for_modulo",
    "for_range",
    "for_range_exclusive",
    "for_string",
    "if_stmts_ok",
    "infinite_for",
    "infinite_for_return",
    "import_call_ok",
    "import_fixtures_ok",
    "import_graph_ok",
    "import_prelude",
    "index_assign_ok",
    "index_key_ok",
    "jit_smoke",
    "keywords_subset",
    "lambda_it_ok",
    "lambda_block_ok",
    "lambda_multi_ok",
    "lambda_stmts_ok",
    "lambda_val_ok",
    "let_point_ok",
    "list_string",
    "logical_not",
    "logical_ops",
    "map_index",
    "map_iter",
    "map_keys",
    "map_literal",
    "map_values",
    "nested_for",
    "or_block_ok",
    "plain_block_arith_add_string_ok",
    "plain_block_arith_ok",
    "plain_block_assign_expr_ok",
    "plain_block_assign_point_ok",
    "plain_block_break_ok",
    "plain_block_call_point_ok",
    "plain_block_cmp_ok",
    "plain_block_coll_homo_ok",
    "plain_block_continue_ok",
    "plain_block_custom_struct_ok",
    "plain_block_field_assign_ok",
    "plain_block_for_cond_ok",
    "plain_block_for_infinite_ok",
    "plain_block_for_modulo_ok",
    "plain_block_for_ok",
    "plain_block_for_range_exclusive_ok",
    "plain_block_for_range_ok",
    "plain_block_for_string_ok",
    "plain_block_for_with_index_ok",
    "plain_block_index_assign_ok",
    "plain_block_index_key_ok",
    "plain_block_let_point_ok",
    "plain_block_list_string_ok",
    "plain_block_logical_not_ok",
    "plain_block_logical_ops_ok",
    "plain_block_map_index_ok",
    "plain_block_map_iter_ok",
    "plain_block_map_keys_ok",
    "plain_block_map_literal_ok",
    "plain_block_map_values_ok",
    "plain_block_nested_for_ok",
    "plain_block_or_ok",
    "plain_block_print_ok",
    "plain_block_range_ok",
    "plain_block_return_ok",
    "plain_block_return_bool_cmp_ok",
    "plain_block_return_point_make_ok",
    "plain_block_return_string_concat_ok",
    "plain_block_return_token_make_ok",
    "plain_block_set_iter_ok",
    "plain_block_string_index_ok",
    "plain_block_trailing_lambda_ok",
    "plain_block_ufcs_len_ok",
    "plain_block_unary_neg_ok",
    "plain_block_unary_plus_ok",
    "plain_block_val_ok",
    "plain_block_when_and_ok",
    "plain_block_when_cond_ok",
    "plain_block_when_condition_chain_ok",
    "plain_block_when_exhaustive_ok",
    "plain_block_when_guard_ok",
    "plain_block_when_ok",
    "print_stmt",
    "range_ok",
    "return_bool_cmp",
    "return_point_make",
    "return_string_concat",
    "return_token_make",
    "set_iter",
    "string_index_ok",
    "struct_when",
    "tokenize_keywords",
    "trailing_lambda_ok",
    "ufcs_len_ok",
    "unary_neg_ok",
    "unary_plus",
    "when_condition_chain",
    "when_cond_ok",
    "when_exhaustive",
    "when_for",
    "when_guard",
    "when_guard_bool",
];

/// Result of a successful Path B bootstrap front-end pass.
pub struct BootstrapCheckResult {
    pub hir: HirModule,
    pub hir_json: String,
}

/// True when the file stem is on the M76 allowlist.
pub fn is_bootstrap_allowlisted(path: &Path) -> bool {
    path.file_stem()
        .and_then(|s| s.to_str())
        .is_some_and(|stem| BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&stem))
}

/// Walk `start` and ancestors for the Action repo root (`Cargo.toml` + `bootstrap/compiler.ac`).
pub fn find_project_root(start: &Path) -> Option<PathBuf> {
    let start = fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
    let mut cur = if start.is_file() {
        start.parent()?.to_path_buf()
    } else {
        start
    };
    loop {
        // Require Cargo.toml so `tests/fixtures/bootstrap/compiler.ac` (synced copy)
        // is not mistaken for the real bootstrap tree.
        if cur.join("Cargo.toml").is_file() && cur.join("bootstrap").join("compiler.ac").is_file() {
            return fs::canonicalize(&cur).ok().or(Some(cur));
        }
        if !cur.pop() {
            break;
        }
    }
    None
}

fn bootstrap_dir(project_root: &Path) -> PathBuf {
    project_root.join("bootstrap")
}

/// Prefer a runnable `action` binary for re-entrancy (Path B spawns the compiler).
fn resolve_action_bin(hint: &Path, project_root: &Path) -> PathBuf {
    let candidates = [
        hint.to_path_buf(),
        project_root.join("target/release/action"),
        project_root.join("target/debug/action"),
        project_root.join("target/release/action.exe"),
        project_root.join("target/debug/action.exe"),
    ];
    for c in candidates {
        if let Ok(canon) = fs::canonicalize(&c) {
            if canon.is_file() {
                return canon;
            }
        } else if c.is_file() {
            return c;
        }
    }
    hint.to_path_buf()
}

fn run_bootstrap_compiler(
    action_bin: &Path,
    project_root: &Path,
    compiler_ac: &Path,
) -> Result<std::process::Output, String> {
    let project_root =
        fs::canonicalize(project_root).unwrap_or_else(|_| project_root.to_path_buf());
    let compiler_ac = fs::canonicalize(compiler_ac).unwrap_or_else(|_| compiler_ac.to_path_buf());
    // Prefer on-disk target artifacts: `current_exe` can point at a replaced/`(deleted)` inode.
    let candidates = [
        project_root.join("target/release/action"),
        project_root.join("target/debug/action"),
        resolve_action_bin(action_bin, &project_root),
    ];
    let mut last_err = "no runnable action binary".to_string();
    for cand in candidates {
        let bin = match fs::canonicalize(&cand) {
            Ok(p) => p,
            Err(_) if cand.is_file() => cand.clone(),
            Err(_) => continue,
        };
        if !bin.is_file() {
            continue;
        }
        match Command::new(&bin)
            .arg("run")
            .arg(&compiler_ac)
            .current_dir(&project_root)
            .output()
        {
            Ok(output) => return Ok(output),
            Err(e) => {
                last_err = format!("spawn {} run {}: {e}", bin.display(), compiler_ac.display());
            }
        }
    }
    Err(last_err)
}

/// Run Action-in-Action frontend on `source`, returning deserialized HIR.
///
/// `action_bin` should be the `action` executable (typically `std::env::current_exe()`).
pub fn check_file_bootstrap(
    source: &Path,
    action_bin: &Path,
) -> Result<BootstrapCheckResult, String> {
    if !source.is_file() {
        return Err(format!("source file not found: {}", source.display()));
    }
    if !is_bootstrap_allowlisted(source) {
        return Err(format!(
            "file '{}' is not on the bootstrap frontend allowlist (M76). \
             Use --frontend rust, or pick a stem from doc/bootstrap-m72-plan.md / \
             action_driver::bootstrap::BOOTSTRAP_FRONTEND_ALLOWLIST",
            source.file_name().and_then(|n| n.to_str()).unwrap_or("?")
        ));
    }

    let project_root = find_project_root(source)
        .or_else(|| find_project_root(&std::env::current_dir().unwrap_or_default()))
        .ok_or_else(|| {
            "cannot locate project root (expected Cargo.toml + bootstrap/compiler.ac)".to_string()
        })?;

    let bs = bootstrap_dir(&project_root);
    let compiler_ac = bs.join("compiler.ac");
    if !compiler_ac.is_file() {
        return Err(format!(
            "bootstrap compiler missing: {}",
            compiler_ac.display()
        ));
    }

    let src_text =
        fs::read_to_string(source).map_err(|e| format!("read {}: {e}", source.display()))?;
    let compile_input = bs.join("_compile_input.txt");
    fs::write(&compile_input, src_text)
        .map_err(|e| format!("write {}: {e}", compile_input.display()))?;

    let output = run_bootstrap_compiler(action_bin, &project_root, &compiler_ac)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(format!(
            "bootstrap frontend rejected '{}' (exit {:?})\nstderr: {}\nstdout: {}",
            source.display(),
            output.status.code(),
            stderr.trim(),
            stdout.chars().take(500).collect::<String>()
        ));
    }

    let hir_path = bs.join("_hir_out.json");
    let hir_json =
        fs::read_to_string(&hir_path).map_err(|e| format!("read {}: {e}", hir_path.display()))?;
    let hir: HirModule = serde_json::from_str(&hir_json)
        .map_err(|e| format!("bootstrap HIR deserialize failed: {e}"))?;

    Ok(BootstrapCheckResult { hir, hir_json })
}

/// Path B: `compile_hir` + LLVM verify (no Rust frontend on the user program).
pub fn verify_bootstrap_hir(hir: &HirModule, label: &str) -> Result<(), String> {
    let context = Context::create();
    let mut cg = CodeGen::new(
        &context,
        &format!("bootstrap_m76_{label}"),
        TypeRegistry::new(),
        None,
    );
    cg.compile_hir(hir)?;
    cg.verify()?;
    Ok(())
}

/// Write HIR JSON next to the source (or stdout), same naming as Rust `--emit hir`.
pub fn emit_bootstrap_hir(hir_json: &str, src_path: &Path, to_stdout: bool) -> Result<(), String> {
    let pretty = match serde_json::from_str::<serde_json::Value>(hir_json) {
        Ok(v) => {
            serde_json::to_string_pretty(&v).map_err(|e| format!("HIR pretty-print failed: {e}"))?
        }
        Err(_) => hir_json.to_string(),
    };
    if to_stdout {
        println!("=== HIR JSON (bootstrap) ===");
        println!("{pretty}");
    } else {
        let out = src_path.with_extension("hir.json");
        fs::write(&out, pretty)
            .map_err(|e| format!("Cannot write to '{}': {}", out.display(), e))?;
        println!("HIR written to: {} (bootstrap frontend)", out.display());
    }
    Ok(())
}
