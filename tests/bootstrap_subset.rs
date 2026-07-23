//! Bootstrap subset boundary tests: allowed fixtures typecheck; forbidden ones fail.
//! M4 lexer goldens, M5 compiler HIR goldens, M6 self-host lexer+compiler alpha, HIR codegen round-trip.

use action::loader;
use action_codegen::CodeGen;
use action_frontend::hir::HirModule;
use action_frontend::registry::TypeRegistry;
use inkwell::context::Context;
use std::fs;
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn action_binary() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_action") {
        return PathBuf::from(path);
    }
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
    let suffix = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    base.join(format!("debug/action{}", suffix))
}

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn bootstrap_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bootstrap")
}

fn collect_ac_files(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|_| panic!("read dir {}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ac"))
        .collect();
    paths.sort();
    paths
}

fn run_action(args: &[&str]) -> std::process::Output {
    Command::new(action_binary())
        .args(args)
        .output()
        .expect("spawn action")
}

fn filter_action_stdout(stdout: &str) -> Vec<String> {
    stdout
        .replace("\r\n", "\n")
        .lines()
        .filter(|line| {
            !line.is_empty()
                && !line.starts_with("Atomic Language")
                && !line.starts_with("LLVM version:")
                && !line.starts_with("Rust version:")
        })
        .map(str::to_owned)
        .collect()
}

fn write_bootstrap_run_source(source: &Path) {
    let dest = bootstrap_dir().join("_run_source.txt");
    fs::write(
        &dest,
        fs::read_to_string(source).expect("read lexer fixture source"),
    )
    .expect("write bootstrap/_run_source.txt");
}

fn write_bootstrap_compile_input(source: &Path) {
    let dest = bootstrap_dir().join("_compile_input.txt");
    fs::write(
        &dest,
        fs::read_to_string(source).expect("read compile fixture"),
    )
    .expect("write bootstrap/_compile_input.txt");
}

fn strip_spans(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if k == "span" {
                    continue;
                }
                out.insert(k.clone(), strip_spans(v));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(strip_spans).collect())
        }
        other => other.clone(),
    }
}

fn strip_types(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut out = serde_json::Map::new();
            for (k, v) in map {
                if k == "ty" || k == "type_ann" {
                    continue;
                }
                out.insert(k.clone(), strip_types(v));
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(strip_types).collect())
        }
        other => other.clone(),
    }
}

fn normalized_oracle_hir(value: &serde_json::Value) -> serde_json::Value {
    strip_types(&strip_spans(value))
}

fn hir_oracle_json(value: &serde_json::Value) -> String {
    serde_json::to_string(&normalized_oracle_hir(value)).expect("oracle hir json")
}

fn filter_main_stmts(value: &serde_json::Value) -> serde_json::Value {
    filter_user_stmts(value, &["main"])
}

fn main_return_ident_ty(hir: &serde_json::Value, ident: &str) -> Option<serde_json::Value> {
    let stmts = hir.get("stmts")?.as_array()?;
    let main_fun = stmts.iter().find(|stmt| {
        stmt.get("Fun")
            .and_then(|f| f.get("name"))
            .and_then(|n| n.as_str())
            == Some("main")
    })?;
    let block = main_fun
        .get("Fun")?
        .get("body")?
        .get("kind")?
        .get("Block")?
        .as_array()?;
    let ret = block.iter().find_map(|stmt| stmt.get("Return"))?;
    let value = ret.get("value")?;
    let name = value.get("kind")?.get("Ident")?.as_str()?;
    if name != ident {
        return None;
    }
    value.get("ty").cloned()
}

fn assert_bootstrap_main_oracle(fixture: &str) {
    let path = fixtures_root().join(format!("bootstrap/{fixture}.ac"));
    let rust = loader::check_file(&path, false)
        .unwrap_or_else(|e| panic!("Rust frontend should typecheck {fixture}: {e:?}"));
    let rust_value: serde_json::Value =
        serde_json::from_str(&rust.hir_json_pretty().expect("rust hir")).expect("parse");
    let bootstrap = load_bootstrap_hir_from_source(&path, fixture);
    let bootstrap_value = serde_json::to_value(&bootstrap).expect("serialize");

    assert_eq!(
        hir_oracle_json(&filter_main_stmts(&bootstrap_value)),
        hir_oracle_json(&filter_main_stmts(&rust_value)),
        "bootstrap {fixture} main HIR should match Rust frontend (ty/span stripped)"
    );
}

fn normalized_hir_json(path: &Path) -> String {
    let raw = fs::read_to_string(path).expect("read hir json");
    let value: serde_json::Value = serde_json::from_str(&raw).expect("parse hir json");
    serde_json::to_string(&strip_spans(&value)).expect("serialize normalized hir")
}

#[test]
fn test_bootstrap_subset_allowed_typechecks() {
    let dir = fixtures_root().join("bootstrap");
    for path in collect_ac_files(&dir) {
        // TC3-negative: intentionally fails typecheck (no golden).
        if path.file_stem().and_then(|s| s.to_str()) == Some("env_scope_leak") {
            continue;
        }
        let result = loader::check_file(&path, false);
        assert!(
            result.is_ok(),
            "bootstrap fixture should typecheck: {} — {:?}",
            path.display(),
            result.err()
        );
    }
}

#[test]
fn test_bootstrap_subset_forbidden_fails_typecheck() {
    let dir = fixtures_root().join("bootstrap_forbidden");
    for path in collect_ac_files(&dir) {
        let result = loader::check_file(&path, false);
        assert!(
            result.is_err(),
            "forbidden bootstrap fixture should fail typecheck: {}",
            path.display()
        );
    }
}

fn assert_lexer_matches_golden(fixture_stem: &str) {
    let json_path = fixtures_root().join(format!("lexer/{fixture_stem}.tokens.json"));
    let expected_json = fs::read_to_string(&json_path).expect("read lexer golden");
    let tokens: Vec<serde_json::Value> =
        serde_json::from_str(&expected_json).expect("parse lexer golden");
    let expected_kinds: Vec<String> = tokens
        .iter()
        .map(|t| t["kind"].as_str().expect("kind").to_owned())
        .collect();

    let source = fixtures_root().join(format!("lexer/{fixture_stem}.ac"));
    write_bootstrap_run_source(&source);

    let lexer_ac = bootstrap_dir().join("lexer.ac");
    let output = run_action(&["run", lexer_ac.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "bootstrap lexer failed on {fixture_stem}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = filter_action_stdout(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(
        actual, expected_kinds,
        "bootstrap lexer token text should match Rust lexer golden kinds for {fixture_stem}"
    );
}

/// M4: bootstrap `lexer.ac` token output must match Rust lexer golden kinds.
#[test]
fn test_bootstrap_m4_lexer_matches_keywords_golden() {
    assert_lexer_matches_golden("keywords");
}

#[test]
fn test_bootstrap_m4_lexer_matches_literals_golden() {
    assert_lexer_matches_golden("literals");
}

#[test]
fn test_bootstrap_m4_lexer_matches_operators_golden() {
    assert_lexer_matches_golden("operators");
}

#[test]
fn test_bootstrap_m4_lexer_matches_ranges_golden() {
    assert_lexer_matches_golden("ranges");
}

#[test]
fn test_bootstrap_m4_lexer_matches_bootstrap_keywords_golden() {
    assert_lexer_matches_golden("bootstrap_keywords");
}

/// Lexer/compiler keyword parity: `external` (used in compiler.ac) must tokenize as keyword.
#[test]
fn test_bootstrap_lexer_recognizes_external_keyword() {
    fs::write(bootstrap_dir().join("_run_source.txt"), "external").expect("write run source");
    let output = run_action(&["run", bootstrap_dir().join("lexer.ac").to_str().unwrap()]);
    assert!(
        output.status.success(),
        "lexer should tokenize external: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let tokens = filter_action_stdout(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(
        tokens.first().map(String::as_str),
        Some("external"),
        "lexer.ac keywordKind should classify external as keyword"
    );
}

/// M23/M24: `bootstrap/prelude.ac` standalone; lexer.ac and compiler.ac `import prelude`.
#[test]
fn test_bootstrap_m23_prelude_embed_in_sync() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_prelude.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_prelude.py");
    assert!(
        output.status.success(),
        "prelude import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M24: bootstrap compiler loads `import prelude` and resolves shared helpers.
#[test]
fn test_bootstrap_m24_import_prelude_smoke() {
    let path = bootstrap_fixture_ac("import_prelude");
    let code = run_bootstrap_hir_jit(&path, "import_prelude");
    assert_eq!(
        code, 42,
        "import_prelude should return 42 via bootstrap HIR JIT"
    );
}
/// M25: `bootstrap/parser.ac` scannerless lexer imported by `compiler.ac`.
#[test]
fn test_bootstrap_m25_parser_module_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_parser.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_parser.py");
    assert!(
        output.status.success(),
        "parser import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M26: `bootstrap/emit.ac` HIR JSON helpers imported by `compiler.ac`.
#[test]
fn test_bootstrap_m26_emit_module_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_emit.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_emit.py");
    assert!(
        output.status.success(),
        "emit import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M27: `bootstrap/typeenv.ac` type environment imported by `compiler.ac`.
#[test]
fn test_bootstrap_m27_typeenv_module_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_typeenv.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_typeenv.py");
    assert!(
        output.status.success(),
        "typeenv import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M27: `bootstrap/whenty.ac` when unify + pattern JSON imported by `compiler.ac`.
#[test]
fn test_bootstrap_m27_whenty_module_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_whenty.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_whenty.py");
    assert!(
        output.status.success(),
        "whenty import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M29: `bootstrap/modload.ac` import registry imported by `compiler.ac`.
#[test]
fn test_bootstrap_m29_modload_module_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_modload.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_modload.py");
    assert!(
        output.status.success(),
        "modload import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M30: `bootstrap/pexpr.ac` expression parser imported by `compiler.ac`.
#[test]
fn test_bootstrap_m30_pexpr_module_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_pexpr.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_pexpr.py");
    assert!(
        output.status.success(),
        "pexpr import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M31: `bootstrap/pstmt.ac` statement/block parser imported by `compiler.ac`.
#[test]
fn test_bootstrap_m31_pstmt_module_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_pstmt.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_pstmt.py");
    assert!(
        output.status.success(),
        "pstmt import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M32: `bootstrap/pdecl.ac` top-level decl parsers imported by `compiler.ac`.
#[test]
fn test_bootstrap_m32_pdecl_module_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_pdecl.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_pdecl.py");
    assert!(
        output.status.success(),
        "pdecl import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M33: `bootstrap/pscan.ac` pre-scan imported by `compiler.ac`.
#[test]
fn test_bootstrap_m33_pscan_module_smoke() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let script = root.join("scripts/check_bootstrap_pscan.py");
    let output = Command::new("python3")
        .arg(&script)
        .current_dir(&root)
        .output()
        .expect("run check_bootstrap_pscan.py");
    assert!(
        output.status.success(),
        "pscan import check failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M5: bootstrap `compiler.ac` emits HIR JSON matching bootstrap goldens (spans stripped).
#[test]
fn test_bootstrap_m5_compiler_matches_hir_goldens() {
    let compiler_ac = bootstrap_dir().join("compiler.ac");

    for fixture in BOOTSTRAP_FIXTURE_STEMS {
        let source = bootstrap_fixture_ac(fixture);
        let golden = bootstrap_fixture_golden(fixture);
        write_bootstrap_compile_input(&source);

        let output = run_action(&["run", compiler_ac.to_str().unwrap()]);
        assert!(
            output.status.success(),
            "bootstrap compiler failed on {fixture}: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let emitted = bootstrap_dir().join("_hir_out.json");
        let actual = normalized_hir_json(&emitted);
        let expected = normalized_hir_json(&golden);
        assert_eq!(
            actual, expected,
            "bootstrap compiler HIR should match golden for {fixture}"
        );
    }
}

/// Every golden fixture has `.ac` source and `.bootstrap_hir.json` on disk.
#[test]
fn test_bootstrap_fixture_goldens_exist() {
    for stem in BOOTSTRAP_FIXTURE_STEMS {
        assert!(
            bootstrap_fixture_ac(stem).is_file(),
            "missing bootstrap fixture: {stem}.ac"
        );
        assert!(
            bootstrap_fixture_golden(stem).is_file(),
            "missing bootstrap golden: {stem}.bootstrap_hir.json"
        );
    }
}

/// M28: multiline source must emit real span line/col (not stubbed (1,1) everywhere).
/// Reads raw `_hir_out.json` so bootstrap-emitted line/col are not lost in HIR round-trips.
#[test]
fn test_bootstrap_m28_span_line_col_oracle() {
    let source = fixtures_root().join("bootstrap/span_multiline.ac");
    write_bootstrap_compile_input(&source);
    let compiler_ac = bootstrap_dir().join("compiler.ac");
    let output = run_action(&["run", compiler_ac.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "bootstrap compiler failed on span_multiline: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let raw = fs::read_to_string(bootstrap_dir().join("_hir_out.json"))
        .expect("read bootstrap _hir_out.json");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("span_multiline HIR json");
    let mut spans = Vec::new();
    collect_spans(&json, &mut spans);
    assert!(
        !spans.is_empty(),
        "expected at least one span in span_multiline HIR"
    );
    let has_non_stub = spans.iter().any(|(line, col)| *line != 1 || *col != 1);
    assert!(
        has_non_stub,
        "expected non-(1,1) span line/col after lineColEnsure, got {spans:?}"
    );
    let max_line = spans.iter().map(|(line, _)| *line).max().unwrap_or(0);
    assert!(
        max_line >= 5,
        "second fun on line 6 should produce span.line >= 5, max_line={max_line}, spans={spans:?}"
    );
}

fn collect_spans(value: &serde_json::Value, out: &mut Vec<(u64, u64)>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(span) = map.get("span") {
                if let (Some(line), Some(col)) = (
                    span.get("line").and_then(|v| v.as_u64()),
                    span.get("col").and_then(|v| v.as_u64()),
                ) {
                    out.push((line, col));
                }
            }
            for v in map.values() {
                collect_spans(v, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for v in arr {
                collect_spans(v, out);
            }
        }
        _ => {}
    }
}

/// M5 smoke: `compiler.ac` parses `bootstrap/token.ac` into five top-level items.
#[test]
fn test_bootstrap_m5_compiler_parses_token_ac() {
    let compiler_ac = bootstrap_dir().join("compiler.ac");
    let source = bootstrap_dir().join("token.ac");
    write_bootstrap_compile_input(&source);

    let output = run_action(&["run", compiler_ac.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "bootstrap compiler failed on token.ac: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let emitted = bootstrap_dir().join("_hir_out.json");
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&emitted).expect("read hir out"))
            .expect("hir json");
    let names: Vec<&str> = value["stmts"]
        .as_array()
        .expect("stmts array")
        .iter()
        .map(|stmt| {
            stmt.as_object()
                .and_then(|o| o.keys().next())
                .map(|s| s.as_str())
                .expect("stmt kind")
        })
        .collect();
    assert_eq!(
        names,
        vec!["TypeAlias", "Fun", "Fun", "Fun", "Fun"],
        "token.ac should yield type alias + four functions"
    );
}

/// M5: bootstrap compiler parses keywords_subset fixture (M4 lexer source shape).
#[test]
fn test_bootstrap_m5_compiler_parses_keywords_subset() {
    let compiler_ac = bootstrap_dir().join("compiler.ac");
    let source = fixtures_root().join("bootstrap/keywords_subset.ac");
    write_bootstrap_compile_input(&source);

    let output = run_action(&["run", compiler_ac.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "bootstrap compiler failed on keywords_subset: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let emitted = bootstrap_dir().join("_hir_out.json");
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&emitted).expect("read hir out"))
            .expect("hir json");
    let funs = value["stmts"]
        .as_array()
        .expect("stmts array")
        .iter()
        .filter(|stmt| stmt.get("Fun").is_some())
        .count();
    assert_eq!(funs, 1, "keywords_subset should parse to one main function");
}

/// M5: bootstrap compiler parses inline tokenize demo (many helper functions).
#[test]
fn test_bootstrap_m5_compiler_parses_tokenize_keywords() {
    let compiler_ac = bootstrap_dir().join("compiler.ac");
    let source = fixtures_root().join("bootstrap/tokenize_keywords.ac");
    write_bootstrap_compile_input(&source);

    let output = run_action(&["run", compiler_ac.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "bootstrap compiler failed on tokenize_keywords: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let emitted = bootstrap_dir().join("_hir_out.json");
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&emitted).expect("read hir out"))
            .expect("hir json");
    let funs = value["stmts"]
        .as_array()
        .expect("stmts array")
        .iter()
        .filter(|stmt| stmt.get("Fun").is_some())
        .count();
    assert!(
        funs >= 12,
        "tokenize_keywords should parse to many functions (got {funs})"
    );

    let rust = loader::check_file(&source, false).expect("tokenize_keywords typechecks");
    let rust_value: serde_json::Value =
        serde_json::from_str(&rust.hir_json_pretty().expect("rust hir")).expect("parse");
    let rust_funs = rust_value["stmts"]
        .as_array()
        .expect("stmts")
        .iter()
        .filter(|stmt| stmt.get("Fun").is_some())
        .count();
    assert_eq!(
        funs, rust_funs,
        "bootstrap compiler should emit same function count as Rust frontend for tokenize_keywords"
    );
}

/// tokenize_keywords fixture stdout matches M4 keywords golden kinds (Rust frontend sanity).
#[test]
fn test_tokenize_keywords_matches_m4_keywords() {
    let json_path = fixtures_root().join("lexer/keywords.tokens.json");
    let expected_json = fs::read_to_string(&json_path).expect("read keywords golden");
    let tokens: Vec<serde_json::Value> =
        serde_json::from_str(&expected_json).expect("parse keywords golden");
    let expected: Vec<String> = tokens
        .iter()
        .map(|t| t["kind"].as_str().expect("kind").to_owned())
        .collect();

    let path = fixtures_root().join("bootstrap/tokenize_keywords.ac");
    let output = run_action(&["run", path.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(14),
        "tokenize_keywords should return token count 14; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual = filter_action_stdout(&String::from_utf8_lossy(&output.stdout));
    assert_eq!(
        actual, expected,
        "inline tokenize demo should emit same kinds as M4 keywords golden"
    );
}

/// M5: infinite for + return runs via Rust frontend (codegen regression).
#[test]
fn test_infinite_for_return_runs_via_action() {
    let path = fixtures_root().join("bootstrap/infinite_for_return.ac");
    let output = run_action(&["run", path.to_str().unwrap()]);
    assert_eq!(
        output.status.code(),
        Some(1),
        "infinite_for_return main should return 1; stderr={}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M5: bootstrap `compiler.ac` parses full `lexer.ac` into valid HIR JSON.
#[test]
fn test_bootstrap_m5_compiler_parses_lexer_ac() {
    let compiler_ac = bootstrap_dir().join("compiler.ac");
    let source = bootstrap_dir().join("lexer.ac");
    write_bootstrap_compile_input(&source);

    let output = run_action(&["run", compiler_ac.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "bootstrap compiler failed on lexer.ac: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let emitted = bootstrap_dir().join("_hir_out.json");
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&emitted).expect("read hir out"))
            .expect("hir json");
    let funs = value["stmts"]
        .as_array()
        .expect("stmts array")
        .iter()
        .filter(|stmt| stmt.get("Fun").is_some())
        .count();
    assert!(
        funs >= 40,
        "lexer.ac should compile to many functions (got {funs})"
    );
}

/// M5: bootstrap `compiler.ac` parses itself (external decls skipped) into valid HIR JSON.
#[test]
fn test_bootstrap_m5_compiler_parses_compiler_ac() {
    let compiler_ac = bootstrap_dir().join("compiler.ac");
    write_bootstrap_compile_input(&compiler_ac);

    let output = run_action(&["run", compiler_ac.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "bootstrap compiler failed on compiler.ac: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let emitted = bootstrap_dir().join("_hir_out.json");
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&emitted).expect("read hir out"))
            .expect("hir json");
    let funs = value["stmts"]
        .as_array()
        .expect("stmts array")
        .iter()
        .filter(|stmt| stmt.get("Fun").is_some())
        .count();
    assert!(
        funs >= 196,
        "compiler.ac should self-parse to many functions (got {funs})"
    );
}

fn top_level_fun_names(value: &serde_json::Value) -> Vec<String> {
    value["stmts"]
        .as_array()
        .expect("stmts array")
        .iter()
        .filter_map(|stmt| {
            stmt.get("Fun")
                .and_then(|f| f.get("name"))
                .and_then(|n| n.as_str())
                .map(str::to_owned)
        })
        .collect()
}

fn rust_hir_value(source: &Path) -> serde_json::Value {
    let path = source.to_path_buf();
    let checked = loader::check_file(&path, false)
        .unwrap_or_else(|e| panic!("Rust frontend should typecheck {}: {e:?}", source.display()));
    serde_json::from_str(&checked.hir_json_pretty().expect("rust hir json"))
        .expect("parse rust hir")
}

fn read_emitted_bootstrap_hir_json() -> serde_json::Value {
    let emitted = bootstrap_dir().join("_hir_out.json");
    let raw = fs::read_to_string(&emitted).expect("read bootstrap _hir_out.json");
    serde_json::from_str(&raw).expect("parse emitted bootstrap hir json")
}

/// After runtime self-host (M12/M13/M17/M18), emitted HIR must name the same top-level `fun`s as Rust.
fn assert_emitted_hir_fun_names_match_rust(source: &Path, label: &str) {
    let bootstrap_value = read_emitted_bootstrap_hir_json();
    let rust_value = rust_hir_value(source);
    assert_eq!(
        top_level_fun_names(&bootstrap_value),
        top_level_fun_names(&rust_value),
        "bootstrap emitted HIR fun names should match Rust frontend for {label}"
    );
}

/// M8 alpha: bootstrap compiler emits the same top-level function names as Rust frontend HIR.
#[test]
fn test_bootstrap_alpha_rust_oracle_fun_names() {
    for fixture in BOOTSTRAP_FIXTURE_STEMS {
        if BOOTSTRAP_SKIP_RUST_FUN_NAME_ORACLE.contains(&fixture) {
            continue;
        }
        let path = bootstrap_fixture_ac(fixture);
        let rust = loader::check_file(&path, false)
            .unwrap_or_else(|e| panic!("Rust frontend should typecheck {fixture}: {e:?}"));
        let rust_value: serde_json::Value =
            serde_json::from_str(&rust.hir_json_pretty().expect("rust hir json"))
                .expect("parse rust hir json");

        let bootstrap = load_bootstrap_hir_from_source(&path, fixture);
        let bootstrap_value = serde_json::to_value(&bootstrap).expect("serialize bootstrap hir");

        assert_eq!(
            top_level_fun_names(&bootstrap_value),
            top_level_fun_names(&rust_value),
            "bootstrap compiler should name top-level functions like Rust frontend for {fixture}"
        );
    }
}

fn hir_json_contains(value: &serde_json::Value, needle: &str) -> bool {
    serde_json::to_string(value)
        .expect("hir json string")
        .contains(needle)
}

/// M8: bootstrap HIR contains the same major constructs as Rust frontend HIR.
#[test]
fn test_bootstrap_alpha_rust_oracle_hir_shape() {
    for (fixture, needles) in [
        ("when_condition_chain", &["ConditionChain"][..]),
        ("when_guard", &["ConditionChain", "And"][..]),
        ("for_range", &["Range", "Iterate"][..]),
        ("for_range_exclusive", &["RangeExclusive", "Iterate"][..]),
        ("for_index", &["IterateWithIndex"][..]),
        ("for_string", &["Iterate"][..]),
        ("list_string", &["__list"][..]),
        ("infinite_for", &["Infinite"][..]),
        ("infinite_for_return", &["Infinite"][..]),
        ("nested_for", &["Range", "Iterate"][..]),
        ("when_for", &["When", "Iterate"][..]),
        ("map_literal", &["MapLiteral"][..]),
        ("for_break", &["Break", "Range", "Iterate"][..]),
        ("for_continue", &["Continue", "Iterate"][..]),
        ("for_modulo", &["Mod", "Continue", "Range", "Iterate"][..]),
        ("map_index", &["MapLiteral", "Index"][..]),
        ("map_iter", &["MapLiteral", "IterateWithIndex"][..]),
        ("map_values", &["MapLiteral", "Iterate"][..]),
        ("map_keys", &["MapLiteral", "Iterate"][..]),
        ("logical_ops", &["And", "Or", "When"][..]),
        ("logical_not", &["Not"][..]),
        ("custom_enum", &["Enum", "ValueMatch"][..]),
        ("when_exhaustive", &["ValueMatch"][..]),
        ("when_guard_bool", &["ValueMatch"][..]),
        ("set_iter", &["SetLiteral", "Iterate"][..]),
        ("unary_plus", &["Literal"][..]),
        ("assign_expr", &["Assign"][..]),
    ] {
        let path = fixtures_root().join(format!("bootstrap/{fixture}.ac"));
        let rust = loader::check_file(&path, false)
            .unwrap_or_else(|e| panic!("Rust frontend should typecheck {fixture}: {e:?}"));
        let rust_value: serde_json::Value =
            serde_json::from_str(&rust.hir_json_pretty().expect("rust hir json"))
                .expect("parse rust hir json");

        let bootstrap = load_bootstrap_hir_from_source(&path, fixture);
        let bootstrap_value = serde_json::to_value(&bootstrap).expect("serialize bootstrap hir");

        for needle in needles {
            assert!(
                hir_json_contains(&bootstrap_value, needle),
                "bootstrap HIR for {fixture} should contain {needle}"
            );
            assert!(
                hir_json_contains(&rust_value, needle),
                "Rust HIR for {fixture} should contain {needle}"
            );
        }
    }
}

/// M10: enum_simple bootstrap HIR matches Rust frontend for user-defined items (ty/span stripped).
#[test]
fn test_bootstrap_alpha_rust_oracle_enum_simple_shape() {
    let fixture = "enum_simple";
    let path = fixtures_root().join(format!("bootstrap/{fixture}.ac"));
    let rust = loader::check_file(&path, false).expect("enum_simple typechecks");
    let rust_value: serde_json::Value =
        serde_json::from_str(&rust.hir_json_pretty().expect("rust hir")).expect("parse");
    let bootstrap = load_bootstrap_hir_from_source(&path, fixture);
    let bootstrap_value = serde_json::to_value(&bootstrap).expect("serialize");

    let rust_user = filter_user_stmts(&rust_value, &["Color", "main"]);
    let boot_user = filter_user_stmts(&bootstrap_value, &["Color", "main"]);

    assert_eq!(
        hir_oracle_json(&boot_user),
        hir_oracle_json(&rust_user),
        "bootstrap enum_simple user HIR should match Rust frontend (ty/span stripped)"
    );
}

/// M10: main-only HIR shape oracle for all bootstrap fixtures with `main`.
#[test]
fn test_bootstrap_alpha_rust_oracle_main_shape() {
    for fixture in BOOTSTRAP_FIXTURE_STEMS {
        assert_bootstrap_main_oracle(fixture);
    }
}

fn stmt_top_level_name(stmt: &serde_json::Value) -> Option<String> {
    if let Some(e) = stmt.get("Enum") {
        return e.get("name").and_then(|n| n.as_str()).map(str::to_owned);
    }
    if let Some(f) = stmt.get("Fun") {
        return f.get("name").and_then(|n| n.as_str()).map(str::to_owned);
    }
    None
}

fn filter_user_stmts(value: &serde_json::Value, names: &[&str]) -> serde_json::Value {
    let stmts = value
        .get("stmts")
        .and_then(|s| s.as_array())
        .expect("hir stmts array");
    let filtered: Vec<serde_json::Value> = stmts
        .iter()
        .filter(|stmt| {
            stmt_top_level_name(stmt)
                .map(|n| names.contains(&n.as_str()))
                .unwrap_or(false)
        })
        .cloned()
        .collect();
    serde_json::json!({ "stmts": filtered })
}

fn run_bootstrap_hir_jit(source: &Path, label: &str) -> i64 {
    let hir = load_bootstrap_hir_from_source(source, label);
    let context = Context::create();
    let mut cg = CodeGen::new(
        &context,
        &format!("bootstrap_jit_{label}"),
        TypeRegistry::new(),
        None,
    );
    cg.set_opt_level(0);
    cg.compile_hir(&hir)
        .unwrap_or_else(|e| panic!("compile_hir failed for bootstrap JIT {label}: {e}"));
    cg.run_jit()
        .unwrap_or_else(|e| panic!("run_jit failed for bootstrap {label}: {e}"))
}

/// CI may keep a stale `host_rt_build/release/libaction_host_rt.a` that predates
/// `action_host_file_*` / `action_host_bs_*`. Reject archives missing those symbols.
fn host_rt_staticlib_has_required_symbols(path: &Path) -> bool {
    let output = Command::new("nm")
        .args(["-g", "--defined-only"])
        .arg(path)
        .output();
    let Ok(output) = output else {
        // Windows / missing nm: accept by path existence only.
        return true;
    };
    if !output.status.success() {
        return false;
    }
    let syms = String::from_utf8_lossy(&output.stdout);
    syms.contains("action_host_file_read") && syms.contains("action_host_bs_buf_get")
}

fn find_aot_host_staticlib() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // Prefer the profile cargo test is using (debug) before a cached release archive.
    let profiles = if cfg!(debug_assertions) {
        ["debug", "release"]
    } else {
        ["release", "debug"]
    };
    let mut candidates = Vec::new();
    // Nested `cargo build` of host-rt may land under host_rt_build/{triple}/{profile}/
    // when CI sets TARGET / --target.
    let triples: Vec<String> = [
        std::env::var("TARGET").ok(),
        std::env::var("CARGO_BUILD_TARGET").ok(),
        Some("x86_64-unknown-linux-gnu".into()),
        None,
    ]
    .into_iter()
    .flatten()
    .collect();

    let mut push_under = |base: &Path| {
        for profile in profiles {
            candidates.push(base.join(format!("host_rt_build/{profile}/libaction_host_rt.a")));
            for triple in &triples {
                candidates.push(base.join(format!(
                    "host_rt_build/{triple}/{profile}/libaction_host_rt.a"
                )));
            }
            candidates.push(base.join(format!("{profile}/libaction_host_rt.a")));
            for triple in &triples {
                candidates.push(base.join(format!("{triple}/{profile}/libaction_host_rt.a")));
            }
        }
    };

    // Prefer build.rs output (`…/host_rt_build/...`) over a stale profile copy.
    if let Ok(target_dir) = std::env::var("CARGO_TARGET_DIR") {
        push_under(&PathBuf::from(target_dir));
    }
    push_under(&root.join("target"));
    push_under(&root);

    // Walk up from the test binary (deps/ → debug/ → target/) like action-cli.
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent().map(|p| p.to_path_buf());
        for _ in 0..6 {
            if let Some(ref d) = dir {
                push_under(d);
                dir = d.parent().map(|p| p.to_path_buf());
            }
        }
    }

    candidates
        .into_iter()
        .find(|p| p.is_file() && host_rt_staticlib_has_required_symbols(p))
}

fn link_bootstrap_aot_executable(
    obj_path: &Path,
    exe_path: &Path,
    needs_host_rt: bool,
) -> Result<(), String> {
    let mut cmd = Command::new("cc");
    cmd.arg("-o").arg(exe_path).arg(obj_path);
    if needs_host_rt {
        let host_lib = find_aot_host_staticlib().ok_or_else(|| {
            format!(
                "libaction_host_rt.a not found for bootstrap AOT (CARGO_TARGET_DIR={:?}, TARGET={:?})",
                std::env::var("CARGO_TARGET_DIR").ok(),
                std::env::var("TARGET").ok()
            )
        })?;
        cmd.arg(host_lib);
    }
    cmd.args(["-lm", "-lpthread", "-ldl"]);
    let output = cmd
        .output()
        .map_err(|e| format!("failed to invoke cc for bootstrap AOT: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cc link failed for bootstrap AOT (status {:?})\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    if !exe_path.is_file() {
        return Err(format!(
            "cc reported success but AOT executable is missing: {}",
            exe_path.display()
        ));
    }
    Ok(())
}

/// Bootstrap HIR → object → `cc` link → native exe path (AOT pure execution path).
fn build_bootstrap_hir_aot_exe(hir: &HirModule, label: &str, opt_level: u8) -> PathBuf {
    let context = Context::create();
    let mut cg = CodeGen::new(
        &context,
        &format!("bootstrap_aot_{label}"),
        TypeRegistry::new(),
        None,
    );
    cg.set_opt_level(opt_level);
    cg.compile_hir(hir)
        .unwrap_or_else(|e| panic!("compile_hir failed for bootstrap AOT {label}: {e}"));
    #[cfg(not(target_os = "windows"))]
    cg.verify()
        .unwrap_or_else(|e| panic!("LLVM verify failed for bootstrap AOT {label}: {e}"));

    let obj_path = bootstrap_dir().join(format!("_aot_{label}.o"));
    let exe_path = bootstrap_dir().join(format!("_aot_{label}"));
    let _ = fs::remove_file(&obj_path);
    let _ = fs::remove_file(&exe_path);

    cg.emit_object(&obj_path)
        .unwrap_or_else(|e| panic!("emit_object failed for bootstrap AOT {label}: {e}"));
    link_bootstrap_aot_executable(&obj_path, &exe_path, cg.needs_host_rt_link())
        .unwrap_or_else(|e| panic!("AOT link failed for bootstrap {label}: {e}"));
    exe_path
}

fn run_bootstrap_aot_exe(exe_path: &Path) -> (i64, String) {
    let output = Command::new(exe_path).output().unwrap_or_else(|e| {
        panic!(
            "failed to run bootstrap AOT exe {}: {e}",
            exe_path.display()
        )
    });
    let code = match output.status.code() {
        Some(c) => c as i64,
        None => panic!(
            "bootstrap AOT exe {} terminated by signal\nstdout:\n{}\nstderr:\n{}",
            exe_path.display(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ),
    };
    if !output.stderr.is_empty() {
        eprintln!(
            "bootstrap AOT exe {} stderr:\n{}",
            exe_path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    (code, String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Bootstrap HIR → AOT exe → process exit code (maps `main()` return to Unix exit status).
fn run_bootstrap_hir_aot(hir: &HirModule, label: &str, opt_level: u8) -> i64 {
    let exe = build_bootstrap_hir_aot_exe(hir, label, opt_level);
    run_bootstrap_aot_exe(&exe).0
}

fn run_bootstrap_hir_aot_from_source(source: &Path, label: &str, opt_level: u8) -> i64 {
    let hir = load_bootstrap_hir_from_source(source, label);
    run_bootstrap_hir_aot(&hir, label, opt_level)
}

/// Bootstrap fixture `main()` return values — shared by M9 JIT and M20 AOT oracles.
const BOOTSTRAP_FIXTURE_RETURN_ORACLES: &[(&str, i64, &str)] = &[
    (
        "assign_expr",
        0,
        "assign_expr when-assign smoke should return 0",
    ),
    (
        "assign_point_ok",
        1,
        "assign_point_ok Point reassign should return 1",
    ),
    (
        "call_point_ok",
        0,
        "call_point_ok Point→Point call should return 0",
    ),
    (
        "coll_homo_ok",
        0,
        "coll_homo_ok homogeneous literals should return 0",
    ),
    (
        "control_flow",
        0,
        "control_flow when/return smoke should return 0",
    ),
    (
        "custom_struct",
        12,
        "custom_struct area(3x4) should return 12",
    ),
    (
        "many_structs",
        9,
        "many_structs T2(tag>=10) field access + String len should return 9",
    ),
    ("enum_simple", 0, "enum_simple when match should return 0"),
    ("custom_enum", 1, "custom_enum Circle arm should return 1"),
    (
        "when_exhaustive",
        3,
        "when_exhaustive Blue arm should return 3",
    ),
    (
        "when_guard_bool",
        1,
        "when_guard_bool Red and true should return 1",
    ),
    (
        "logical_not",
        0,
        "logical_not not false → if true {0} else {1} should return 0",
    ),
    (
        "keywords_subset",
        0,
        "keywords_subset parse smoke should return 0",
    ),
    (
        "struct_when",
        0,
        "struct_when pattern match should return 0",
    ),
    (
        "when_condition_chain",
        0,
        "when_condition_chain nested when should return 0",
    ),
    ("when_guard", 0, "when_guard guarded arm should return 0"),
    (
        "field_assign_ok",
        7,
        "field_assign_ok p.x = 7 should return 7",
    ),
    (
        "index_assign_ok",
        0,
        "index_assign_ok xs[0] = 9 should return 0",
    ),
    (
        "index_key_ok",
        0,
        "index_key_ok list index + map key assign should return 0",
    ),
    (
        "string_index_ok",
        0,
        "string_index_ok s[0]-s[0] should return 0",
    ),
    (
        "import_call_ok",
        7,
        "import_call_ok prelude.keywordKind(\"fun\") should return 7",
    ),
    ("arith_ok", 4, "arith_ok 10 - 3 * 2 should return 4"),
    (
        "arith_add_string_ok",
        2,
        "arith_add_string_ok len(\"a\"+\"b\") should return 2",
    ),
    ("cmp_ok", 0, "cmp_ok when 1 < 2 should return 0"),
    ("unary_neg_ok", 2, "unary_neg_ok -3 + 5 should return 2"),
    ("range_ok", 3, "range_ok sum 1..<3 should return 3"),
    ("when_cond_ok", 0, "when_cond_ok when true should return 0"),
    ("for_cond_ok", 3, "for_cond_ok for x < 3 should return 3"),
    ("jit_smoke", 42, "jit_smoke should return 42"),
    ("for_range", 10, "for_range sum 1..5 should be 10"),
    (
        "for_range_exclusive",
        10,
        "for_range_exclusive sum 1..<5 should be 10",
    ),
    ("for_index", 6, "for_index sum List[1,2,3] should be 6"),
    (
        "for_string",
        6,
        "for_string sum len(s) over List[String] should be 6",
    ),
    (
        "list_string",
        3,
        "list_string len(List[String]) should be 3",
    ),
    ("let_point_ok", 0, "let_point_ok Point init should return 0"),
    (
        "infinite_for_return",
        1,
        "infinite for with return should exit on first iteration",
    ),
    ("nested_for", 3, "nested_for nested ranges should return 3"),
    ("when_for", 37, "when_for sum should be 37"),
    (
        "tokenize_keywords",
        14,
        "bootstrap HIR tokenize demo should return 14 tokens",
    ),
    ("for_break", 15, "for_break should sum 1..5 before break"),
    ("for_continue", 12, "for_continue should skip i==3"),
    ("for_modulo", 20, "for_modulo sum evens 1..10 should be 20"),
    ("map_index", 10, "map_index m[\"a\"] should be 10"),
    ("map_iter", 15, "map_iter sum of values should be 15"),
    (
        "map_values",
        15,
        "map_values sum of map values should be 15",
    ),
    ("set_iter", 6, "set_iter sum 1+2+3 should be 6"),
    ("map_keys", 3, "map_keys sum of len(k) should be 3"),
    (
        "logical_ops",
        0,
        "both(true, false) should be false, main returns 0",
    ),
    (
        "env_scope_good",
        154,
        "env_scope_good: inner(55) + shadowLen(99) should be 154",
    ),
    ("unary_plus", 15, "unary_plus +10 + +5 should be 15"),
    ("map_literal", 0, "map_literal smoke should return 0"),
    (
        "print_stmt",
        0,
        "print_stmt should return 0 after print call",
    ),
    (
        "return_string_concat",
        11,
        "return_string_concat len(\"hello world\") should be 11",
    ),
    (
        "return_bool_cmp",
        1,
        "return_bool_cmp isOne(1) should make main return 1",
    ),
    (
        "return_token_make",
        3,
        "return_token_make len(\"fun\") should be 3",
    ),
    (
        "return_point_make",
        0,
        "return_point_make origin().x + origin().y should be 0",
    ),
    (
        "ufcs_len_ok",
        3,
        "ufcs_len_ok List[1,2,3].len() should return 3",
    ),
    (
        "or_block_ok",
        0,
        "or_block_ok parseInt(\"x\") or { 0 } should return 0",
    ),
    (
        "lambda_it_ok",
        42,
        "lambda_it_ok { it * 2 }(21) should return 42",
    ),
    (
        "lambda_block_ok",
        42,
        "lambda_block_ok { 21 * 2 }() should return 42",
    ),
    (
        "lambda_stmts_ok",
        42,
        "lambda_stmts_ok { 21; 21 * 2 }() should return 42",
    ),
    (
        "if_stmts_ok",
        42,
        "if_stmts_ok if true { 21; 21 * 2 } else { 0 } should return 42",
    ),
    (
        "plain_block_val_ok",
        42,
        "plain_block_val_ok { val a: Int = 21; a * 2 } should return 42",
    ),
    (
        "lambda_val_ok",
        42,
        "lambda_val_ok { 21; val a: Int = 21; a * 2 }() should return 42",
    ),
    (
        "plain_block_return_ok",
        42,
        "plain_block_return_ok if true { return 42; 0 } else { 0 } should return 42",
    ),
    (
        "plain_block_for_ok",
        42,
        "plain_block_for_ok for i in 0..42 { s = s + 1 } should return 42",
    ),
    (
        "plain_block_for_cond_ok",
        42,
        "plain_block_for_cond_ok for s < 42 { s = s + 1 } should return 42",
    ),
    (
        "plain_block_for_with_index_ok",
        6,
        "plain_block_for_with_index_ok for idx, n in List[1,2,3] should return 6",
    ),
    (
        "plain_block_for_infinite_ok",
        42,
        "plain_block_for_infinite_ok for { return 42 } should return 42",
    ),
    (
        "plain_block_map_values_ok",
        15,
        "plain_block_map_values_ok for v in Map values should return 15",
    ),
    (
        "plain_block_map_keys_ok",
        3,
        "plain_block_map_keys_ok for k in Map len(k) should return 3",
    ),
    (
        "plain_block_map_iter_ok",
        15,
        "plain_block_map_iter_ok for k, v in Map should return 15",
    ),
    (
        "plain_block_set_iter_ok",
        6,
        "plain_block_set_iter_ok for x in Set should return 6",
    ),
    (
        "plain_block_when_ok",
        37,
        "plain_block_when_ok when-in-for should return 37",
    ),
    (
        "plain_block_when_condition_chain_ok",
        0,
        "plain_block_when_condition_chain_ok else arm should return 0",
    ),
    (
        "plain_block_when_and_ok",
        0,
        "plain_block_when_and_ok ConditionChain and should return 0",
    ),
    (
        "plain_block_map_index_ok",
        10,
        "plain_block_map_index_ok m[\"a\"] should return 10",
    ),
    (
        "plain_block_string_index_ok",
        0,
        "plain_block_string_index_ok s[0]-s[0] should return 0",
    ),
    (
        "plain_block_logical_not_ok",
        0,
        "plain_block_logical_not_ok not false should return 0",
    ),
    (
        "plain_block_for_range_exclusive_ok",
        10,
        "plain_block_for_range_exclusive_ok sum 1..<5 should return 10",
    ),
    (
        "plain_block_for_string_ok",
        6,
        "plain_block_for_string_ok sum len(s) should return 6",
    ),
    (
        "plain_block_unary_plus_ok",
        15,
        "plain_block_unary_plus_ok +10 + +5 should return 15",
    ),
    (
        "plain_block_unary_neg_ok",
        2,
        "plain_block_unary_neg_ok -3 + 5 should return 2",
    ),
    (
        "plain_block_logical_ops_ok",
        0,
        "plain_block_logical_ops_ok true and false should return 0",
    ),
    (
        "plain_block_assign_expr_ok",
        0,
        "plain_block_assign_expr_ok assign smoke should return 0",
    ),
    (
        "plain_block_assign_point_ok",
        1,
        "plain_block_assign_point_ok Point reassign should return 1",
    ),
    (
        "plain_block_let_point_ok",
        0,
        "plain_block_let_point_ok Point let should return 0",
    ),
    (
        "plain_block_call_point_ok",
        0,
        "plain_block_call_point_ok Point→Point call should return 0",
    ),
    (
        "plain_block_return_point_make_ok",
        0,
        "plain_block_return_point_make_ok Point fields sum should return 0",
    ),
    (
        "plain_block_return_bool_cmp_ok",
        1,
        "plain_block_return_bool_cmp_ok isOne(1) should return 1",
    ),
    (
        "plain_block_return_token_make_ok",
        3,
        "plain_block_return_token_make_ok len(\"fun\") should return 3",
    ),
    (
        "plain_block_range_ok",
        3,
        "plain_block_range_ok sum 1..<3 should return 3",
    ),
    (
        "plain_block_when_cond_ok",
        0,
        "plain_block_when_cond_ok if true should return 0",
    ),
    (
        "plain_block_custom_struct_ok",
        12,
        "plain_block_custom_struct_ok area(3x4) should return 12",
    ),
    (
        "plain_block_many_structs_ok",
        9,
        "plain_block_many_structs_ok T2 a+len(c) should return 9",
    ),
    (
        "plain_block_arith_ok",
        4,
        "plain_block_arith_ok 10 - 3 * 2 should return 4",
    ),
    (
        "plain_block_arith_add_string_ok",
        2,
        "plain_block_arith_add_string_ok \"a\"+\"b\" len should return 2",
    ),
    (
        "plain_block_cmp_ok",
        0,
        "plain_block_cmp_ok 1 < 2 should return 0",
    ),
    (
        "plain_block_for_range_ok",
        10,
        "plain_block_for_range_ok sum 1..5 should return 10",
    ),
    (
        "plain_block_coll_homo_ok",
        0,
        "plain_block_coll_homo_ok List/Set/Map lit smoke should return 0",
    ),
    (
        "plain_block_list_string_ok",
        3,
        "plain_block_list_string_ok List[String] len should return 3",
    ),
    (
        "plain_block_index_key_ok",
        0,
        "plain_block_index_key_ok List/Map index assign smoke should return 0",
    ),
    (
        "plain_block_map_literal_ok",
        0,
        "plain_block_map_literal_ok Map lit smoke should return 0",
    ),
    (
        "plain_block_for_modulo_ok",
        20,
        "plain_block_for_modulo_ok sum evens 1..10 should return 20",
    ),
    (
        "plain_block_return_string_concat_ok",
        11,
        "plain_block_return_string_concat_ok len(\"hello world\") should return 11",
    ),
    (
        "plain_block_when_guard_ok",
        1,
        "plain_block_when_guard_ok Red and true should return 1",
    ),
    (
        "plain_block_print_ok",
        0,
        "plain_block_print_ok print then return 0",
    ),
    (
        "plain_block_when_exhaustive_ok",
        3,
        "plain_block_when_exhaustive_ok Blue arm should return 3",
    ),
    (
        "plain_block_ufcs_len_ok",
        3,
        "plain_block_ufcs_len_ok List.len() should return 3",
    ),
    (
        "plain_block_or_ok",
        0,
        "plain_block_or_ok parseInt or { 0 } should return 0",
    ),
    (
        "plain_block_trailing_lambda_ok",
        42,
        "plain_block_trailing_lambda_ok map trailing { it * 2 } should return 42",
    ),
    (
        "plain_block_field_assign_ok",
        7,
        "plain_block_field_assign_ok p.x = 7 should return 7",
    ),
    (
        "plain_block_index_assign_ok",
        0,
        "plain_block_index_assign_ok xs[0] = 9 should return 0",
    ),
    (
        "plain_block_nested_for_ok",
        3,
        "plain_block_nested_for_ok nested ranges should return 3",
    ),
    (
        "plain_block_break_ok",
        15,
        "plain_block_break_ok for-break should sum 1..5 → 15",
    ),
    (
        "plain_block_continue_ok",
        12,
        "plain_block_continue_ok for-continue should skip 3 → 12",
    ),
    (
        "lambda_multi_ok",
        42,
        "lambda_multi_ok { x, y -> x + y }(20, 22) should return 42",
    ),
    (
        "trailing_lambda_ok",
        42,
        "trailing_lambda_ok map(List[21]) { it * 2 }[0] or { 0 } should return 42",
    ),
    (
        "import_graph_ok",
        42,
        "import_graph_ok m120_lib.add1(41) should return 42",
    ),
    (
        "import_fixtures_ok",
        42,
        "import_fixtures_ok m124_lib.add1(41) via fixtures root should return 42",
    ),
];

/// Bootstrap allowed fixtures with golden HIR + main oracle (65 stems).
/// `env_scope_leak.ac` is TC3-negative only (no golden).
/// Fixtures where bootstrap import loader emits the full module but Rust uses selective import.
const BOOTSTRAP_SKIP_RUST_FUN_NAME_ORACLE: &[&str] = &[
    "import_call_ok",
    "import_fixtures_ok",
    "import_graph_ok",
    "import_prelude",
];

const BOOTSTRAP_FIXTURE_STEMS: &[&str] = &[
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
    "plain_block_many_structs_ok",
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

fn bootstrap_fixture_ac(stem: &str) -> PathBuf {
    fixtures_root().join(format!("bootstrap/{stem}.ac"))
}

fn bootstrap_fixture_golden(stem: &str) -> PathBuf {
    fixtures_root().join(format!("bootstrap/{stem}.bootstrap_hir.json"))
}

/// M9: bootstrap fixture HIR return-value oracles under MCJIT (isolated subprocess).
#[test]
#[ignore = "JIT must run in an isolated process"]
fn test_bootstrap_m9_jit_fixture_return_oracles() {
    for (stem, expected, msg) in BOOTSTRAP_FIXTURE_RETURN_ORACLES {
        let path = fixtures_root().join(format!("bootstrap/{stem}.ac"));
        let code = run_bootstrap_hir_jit(&path, stem);
        assert_eq!(code, *expected, "JIT {msg}");
    }
}

/// Bootstrap HIR JIT stdout matches M4 keywords golden (Unix dup2 capture).
#[cfg(unix)]
#[test]
#[ignore = "JIT stdout must run in an isolated process"]
fn test_bootstrap_alpha_jit_tokenize_keywords_stdout() {
    let json_path = fixtures_root().join("lexer/keywords.tokens.json");
    let expected_json = fs::read_to_string(&json_path).expect("read keywords golden");
    let tokens: Vec<serde_json::Value> =
        serde_json::from_str(&expected_json).expect("parse keywords golden");
    let expected: Vec<String> = tokens
        .iter()
        .map(|t| t["kind"].as_str().expect("kind").to_owned())
        .collect();

    let out_path = bootstrap_dir().join("_jit_stdout.txt");
    let _ = fs::remove_file(&out_path);
    let file = fs::File::create(&out_path).expect("create stdout capture");
    let fd = file.as_raw_fd();
    unsafe {
        libc::dup2(fd, 1);
    }

    let code = run_bootstrap_hir_jit(
        &fixtures_root().join("bootstrap/tokenize_keywords.ac"),
        "tokenize_keywords_stdout",
    );
    assert_eq!(code, 14, "tokenize_keywords JIT should return 14");

    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    drop(file);

    let stdout = fs::read_to_string(&out_path).expect("read captured jit stdout");
    let actual = filter_action_stdout(&stdout);
    assert_eq!(
        actual, expected,
        "bootstrap HIR JIT tokenize stdout should match M4 keywords golden"
    );
}

/// Run bootstrap HIR JIT smoke tests in fresh processes.
#[test]
fn test_bootstrap_alpha_jit_run_subprocess() {
    for test_name in [
        "test_bootstrap_m9_jit_fixture_return_oracles",
        "test_bootstrap_m11_compiler_self_jit",
        "test_bootstrap_m12_compiler_jit_parse_lexer",
        "test_bootstrap_m13_compiler_jit_parse_self",
    ] {
        run_isolated_test(test_name);
    }
}

/// Run bootstrap HIR JIT stdout oracle in a fresh process (Unix only).
#[cfg(unix)]
#[test]
fn test_bootstrap_alpha_jit_tokenize_keywords_stdout_subprocess() {
    run_isolated_test("test_bootstrap_alpha_jit_tokenize_keywords_stdout");
}

/// M6 alpha: Action lexer tokenizes its own source (majority of tokens before host stack limit).
#[test]
fn test_bootstrap_m6_lexer_self_tokenize_alpha() {
    let lexer_src = bootstrap_dir().join("lexer.ac");
    write_bootstrap_run_source(&lexer_src);

    let lexer_ac = bootstrap_dir().join("lexer.ac");
    let out_path = bootstrap_dir().join("_m6_stdout.txt");
    let out_file = fs::File::create(&out_path).expect("create m6 stdout capture");
    let status = Command::new(action_binary())
        .args(["run", lexer_ac.to_str().unwrap()])
        .stdout(out_file)
        .stderr(Stdio::null())
        .status()
        .expect("run bootstrap lexer on itself");

    let stdout = fs::read_to_string(&out_path).expect("read m6 stdout capture");
    let tokens = filter_action_stdout(&stdout);

    assert!(
        status.success(),
        "lexer should exit cleanly when scanning bootstrap/lexer.ac; status={status:?}"
    );
    assert!(
        tokens.len() > 800,
        "lexer should emit many real tokens for bootstrap/lexer.ac (got {})",
        tokens.len()
    );
    assert!(
        !tokens.iter().any(|t| t.is_empty()),
        "token kinds must not be empty"
    );
    assert_eq!(tokens.first().map(String::as_str), Some("import"));
    assert!(tokens.iter().any(|t| t == "tokenize"));
}

/// M6 alpha: bootstrap compiler parses its own source into large HIR with core symbols.
#[test]
fn test_bootstrap_m6_compiler_self_hir_alpha() {
    let compiler_src = bootstrap_dir().join("compiler.ac");
    let hir = load_bootstrap_hir_from_source(&compiler_src, "compiler.ac");
    let value = serde_json::to_value(&hir).expect("serialize bootstrap self-hir");
    let funs = top_level_fun_names(&value);
    assert!(
        funs.len() >= 196,
        "compiler.ac self-HIR should contain many functions (got {})",
        funs.len()
    );
    for needle in [
        "main",
        "parseExpr",
        "lexKindAt",
        "parseProgram",
        "keywordKind",
    ] {
        assert!(
            funs.iter().any(|n| n == needle),
            "compiler.ac self-HIR missing top-level function {needle}"
        );
    }
}

/// M6 alpha: LLVM verify bootstrap compiler.ac HIR (isolated subprocess).
#[test]
#[ignore = "LLVM verify must run in an isolated process"]
fn test_bootstrap_m6_compiler_self_verify() {
    assert_bootstrap_hir_compiles_from(&bootstrap_dir().join("compiler.ac"), "compiler.ac");
}

#[test]
fn test_bootstrap_m6_compiler_self_verify_subprocess() {
    run_isolated_test("test_bootstrap_m6_compiler_self_verify");
}

/// M11: bootstrap compiler.ac HIR executes under MCJIT on real input (isolated subprocess).
#[test]
#[ignore = "JIT must run in an isolated process"]
fn test_bootstrap_m11_compiler_self_jit() {
    let compiler_src = bootstrap_dir().join("compiler.ac");
    let hir = load_bootstrap_hir_from_source(&compiler_src, "compiler.ac");
    write_bootstrap_compile_input(&fixtures_root().join("bootstrap/enum_simple.ac"));
    let context = Context::create();
    let mut cg = CodeGen::new(
        &context,
        "bootstrap_compiler_self_m11",
        TypeRegistry::new(),
        None,
    );
    cg.set_opt_level(0);
    cg.compile_hir(&hir)
        .unwrap_or_else(|e| panic!("compile_hir failed for bootstrap compiler.ac M11: {e}"));
    let code = cg
        .run_jit()
        .unwrap_or_else(|e| panic!("run_jit failed for bootstrap compiler.ac M11: {e}"));
    assert_eq!(
        code, 0,
        "bootstrap compiler.ac JIT main should return 0 on enum_simple input"
    );
    assert_emitted_hir_fun_names_match_rust(
        &fixtures_root().join("bootstrap/enum_simple.ac"),
        "M11 enum_simple",
    );
}

#[test]
fn test_bootstrap_m11_compiler_self_jit_subprocess() {
    run_isolated_test("test_bootstrap_m11_compiler_self_jit");
}

/// M12: bootstrap compiler JIT parses `lexer.ac` into large HIR (isolated subprocess).
#[test]
#[ignore = "JIT must run in an isolated process"]
fn test_bootstrap_m12_compiler_jit_parse_lexer() {
    let compiler_src = bootstrap_dir().join("compiler.ac");
    let hir = load_bootstrap_hir_from_source(&compiler_src, "compiler.ac");
    write_bootstrap_compile_input(&bootstrap_dir().join("lexer.ac"));
    let context = Context::create();
    let mut cg = CodeGen::new(
        &context,
        "bootstrap_compiler_self_m12",
        TypeRegistry::new(),
        None,
    );
    cg.set_opt_level(0);
    cg.compile_hir(&hir)
        .unwrap_or_else(|e| panic!("compile_hir failed for bootstrap compiler.ac M12: {e}"));
    let code = cg
        .run_jit()
        .unwrap_or_else(|e| panic!("run_jit failed for bootstrap compiler.ac M12: {e}"));
    assert_eq!(
        code, 0,
        "bootstrap compiler.ac JIT main should return 0 on lexer.ac input"
    );
    assert_emitted_hir_fun_names_match_rust(&bootstrap_dir().join("lexer.ac"), "M12 lexer.ac");
}

#[test]
fn test_bootstrap_m12_compiler_jit_parse_lexer_subprocess() {
    run_isolated_test("test_bootstrap_m12_compiler_jit_parse_lexer");
}

/// M13: bootstrap compiler JIT parses `compiler.ac` (runtime self-bootstrap, isolated subprocess).
#[test]
#[ignore = "JIT must run in an isolated process"]
fn test_bootstrap_m13_compiler_jit_parse_self() {
    let compiler_src = bootstrap_dir().join("compiler.ac");
    let hir = load_bootstrap_hir_from_source(&compiler_src, "compiler.ac");
    write_bootstrap_compile_input(&compiler_src);
    let context = Context::create();
    let mut cg = CodeGen::new(
        &context,
        "bootstrap_compiler_self_m13",
        TypeRegistry::new(),
        None,
    );
    cg.set_opt_level(0);
    cg.compile_hir(&hir)
        .unwrap_or_else(|e| panic!("compile_hir failed for bootstrap compiler.ac M13: {e}"));
    let code = cg
        .run_jit()
        .unwrap_or_else(|e| panic!("run_jit failed for bootstrap compiler.ac M13: {e}"));
    assert_eq!(
        code, 0,
        "bootstrap compiler.ac JIT main should return 0 when parsing compiler.ac"
    );
    assert_emitted_hir_fun_names_match_rust(&compiler_src, "M13 compiler.ac");
}

#[test]
fn test_bootstrap_m13_compiler_jit_parse_self_subprocess() {
    run_isolated_test("test_bootstrap_m13_compiler_jit_parse_self");
}

fn lexer_golden_kinds(fixture_stem: &str) -> Vec<String> {
    let json_path = fixtures_root().join(format!("lexer/{fixture_stem}.tokens.json"));
    let expected_json = fs::read_to_string(&json_path).expect("read lexer golden");
    let tokens: Vec<serde_json::Value> =
        serde_json::from_str(&expected_json).expect("parse lexer golden");
    tokens
        .iter()
        .map(|t| t["kind"].as_str().expect("kind").to_owned())
        .collect()
}

/// M14: `lexer.ac` bootstrap HIR → MCJIT tokenize → M4 Rust lexer golden kinds (Unix stdout capture).
#[cfg(unix)]
fn run_lexer_hir_jit_tokens(fixture_stem: &str) -> (i64, Vec<String>) {
    let lexer_src = bootstrap_dir().join("lexer.ac");
    let hir = load_bootstrap_hir_from_source(&lexer_src, "lexer.ac");
    let source = fixtures_root().join(format!("lexer/{fixture_stem}.ac"));
    write_bootstrap_run_source(&source);

    let out_path = bootstrap_dir().join("_m14_stdout.txt");
    let _ = fs::remove_file(&out_path);
    let file = fs::File::create(&out_path).expect("create m14 stdout capture");
    let fd = file.as_raw_fd();
    unsafe {
        libc::dup2(fd, 1);
    }

    let context = Context::create();
    let mut cg = CodeGen::new(
        &context,
        &format!("bootstrap_lexer_m14_{fixture_stem}"),
        TypeRegistry::new(),
        None,
    );
    cg.set_opt_level(0);
    cg.compile_hir(&hir).unwrap_or_else(|e| {
        panic!("compile_hir failed for bootstrap lexer.ac M14 {fixture_stem}: {e}")
    });
    let code = cg.run_jit().unwrap_or_else(|e| {
        panic!("run_jit failed for bootstrap lexer.ac M14 {fixture_stem}: {e}")
    });

    unsafe {
        libc::fflush(std::ptr::null_mut());
    }
    drop(file);

    let stdout = fs::read_to_string(&out_path).expect("read m14 jit stdout");
    (code, filter_action_stdout(&stdout))
}

#[cfg(unix)]
#[test]
#[ignore = "JIT stdout must run in an isolated process"]
fn test_bootstrap_m14_lexer_jit_tokenize_goldens() {
    for fixture in [
        "keywords",
        "literals",
        "operators",
        "ranges",
        "bootstrap_keywords",
    ] {
        let expected = lexer_golden_kinds(fixture);
        let (code, actual) = run_lexer_hir_jit_tokens(fixture);
        assert_eq!(
            code, 0,
            "lexer.ac JIT main should return 0 on lexer/{fixture}.ac"
        );
        assert_eq!(
            actual, expected,
            "lexer.ac bootstrap HIR JIT tokens should match M4 golden for {fixture}"
        );
    }
}

#[cfg(unix)]
#[test]
fn test_bootstrap_m14_lexer_jit_tokenize_goldens_subprocess() {
    run_isolated_test("test_bootstrap_m14_lexer_jit_tokenize_goldens");
}

/// M15: large bootstrap sources (`lexer.ac` / `compiler.ac`) pass LLVM verify after
/// runtime bitcode type sync (isolated — avoids in-process LLVMContext clashes).
#[test]
#[ignore = "LLVM verify must run in an isolated process"]
fn test_bootstrap_m15_hir_verify_lexer_ac() {
    assert_bootstrap_hir_compiles_from(&bootstrap_dir().join("lexer.ac"), "lexer.ac");
}

#[test]
#[ignore = "LLVM verify must run in an isolated process"]
fn test_bootstrap_m15_hir_verify_compiler_ac() {
    assert_bootstrap_hir_compiles_from(&bootstrap_dir().join("compiler.ac"), "compiler.ac");
}

#[test]
fn test_bootstrap_m15_hir_verify_large_subprocess() {
    run_isolated_test("test_bootstrap_m15_hir_verify_lexer_ac");
    run_isolated_test("test_bootstrap_m15_hir_verify_compiler_ac");
}

/// M15: path B stdout closure — bootstrap compiler → `tokenize_keywords.ac` HIR → JIT
/// token kinds match M4 `keywords.tokens.json` (Unix dup2; isolated subprocess).
#[cfg(unix)]
#[test]
fn test_bootstrap_m15_tokenize_keywords_stdout_subprocess() {
    run_isolated_test("test_bootstrap_alpha_jit_tokenize_keywords_stdout");
}

/// M16: bootstrap HIR → AOT (`emit_object` + `cc`) smoke (isolated — large IR + link).
#[test]
#[ignore = "AOT compile/link must run in an isolated process"]
fn test_bootstrap_m16_aot_jit_smoke() {
    let code = run_bootstrap_hir_aot_from_source(
        &fixtures_root().join("bootstrap/jit_smoke.ac"),
        "m16_jit_smoke",
        0,
    );
    assert_eq!(code, 42, "jit_smoke AOT should return 42");
}

/// M16: `compiler.ac` bootstrap HIR → AOT parses `enum_simple.ac` (path B, mirrors M11 JIT).
#[test]
#[ignore = "AOT compile/link must run in an isolated process"]
fn test_bootstrap_m16_compiler_aot_enum_simple() {
    let compiler_src = bootstrap_dir().join("compiler.ac");
    let hir = load_bootstrap_hir_from_source(&compiler_src, "compiler.ac");
    write_bootstrap_compile_input(&fixtures_root().join("bootstrap/enum_simple.ac"));
    let code = run_bootstrap_hir_aot(&hir, "m16_compiler_enum_simple", 0);
    assert_eq!(
        code, 0,
        "bootstrap compiler.ac AOT main should return 0 on enum_simple input"
    );
    assert_emitted_hir_fun_names_match_rust(
        &fixtures_root().join("bootstrap/enum_simple.ac"),
        "M16 enum_simple",
    );
}

#[test]
fn test_bootstrap_m16_aot_subprocess() {
    run_isolated_test("test_bootstrap_m16_aot_jit_smoke");
    run_isolated_test("test_bootstrap_m16_compiler_aot_enum_simple");
    run_isolated_test("test_bootstrap_m20_aot_fixture_return_oracles");
}

/// M20: bootstrap fixture HIR return-value oracles under AOT (mirrors all M9 JIT tests).
#[test]
#[ignore = "AOT compile/link must run in an isolated process"]
fn test_bootstrap_m20_aot_fixture_return_oracles() {
    for (stem, expected, msg) in BOOTSTRAP_FIXTURE_RETURN_ORACLES {
        let path = fixtures_root().join(format!("bootstrap/{stem}.ac"));
        let code = run_bootstrap_hir_aot_from_source(&path, &format!("m20_aot_{stem}"), 0);
        assert_eq!(code, *expected, "AOT {msg}");
    }
}

/// M17: `compiler.ac` bootstrap HIR → AOT parses `compiler.ac` (runtime self-bootstrap via AOT).
#[test]
#[ignore = "AOT compile/link must run in an isolated process"]
fn test_bootstrap_m17_compiler_aot_parse_self() {
    let compiler_src = bootstrap_dir().join("compiler.ac");
    let hir = load_bootstrap_hir_from_source(&compiler_src, "compiler.ac");
    write_bootstrap_compile_input(&compiler_src);
    let code = run_bootstrap_hir_aot(&hir, "m17_compiler_self", 0);
    assert_eq!(
        code, 0,
        "bootstrap compiler.ac AOT main should return 0 when parsing compiler.ac"
    );
    assert_emitted_hir_fun_names_match_rust(&compiler_src, "M17 compiler.ac");
}

#[test]
fn test_bootstrap_m17_compiler_aot_parse_self_subprocess() {
    run_isolated_test("test_bootstrap_m17_compiler_aot_parse_self");
}

/// M18: `compiler.ac` bootstrap HIR → AOT parses `lexer.ac` (mirrors M12 JIT).
#[test]
#[ignore = "AOT compile/link must run in an isolated process"]
fn test_bootstrap_m18_compiler_aot_parse_lexer() {
    let compiler_src = bootstrap_dir().join("compiler.ac");
    let hir = load_bootstrap_hir_from_source(&compiler_src, "compiler.ac");
    write_bootstrap_compile_input(&bootstrap_dir().join("lexer.ac"));
    let code = run_bootstrap_hir_aot(&hir, "m18_compiler_lexer", 0);
    assert_eq!(
        code, 0,
        "bootstrap compiler.ac AOT main should return 0 on lexer.ac input"
    );
    assert_emitted_hir_fun_names_match_rust(&bootstrap_dir().join("lexer.ac"), "M18 lexer.ac");
}

#[test]
fn test_bootstrap_m18_compiler_aot_parse_lexer_subprocess() {
    run_isolated_test("test_bootstrap_m18_compiler_aot_parse_lexer");
}

/// M19: `lexer.ac` bootstrap HIR → AOT tokenize → M4 Rust lexer golden kinds (mirrors M14 JIT).
#[cfg(unix)]
fn run_lexer_hir_aot_tokens(fixture_stem: &str) -> (i64, Vec<String>) {
    let lexer_src = bootstrap_dir().join("lexer.ac");
    let hir = load_bootstrap_hir_from_source(&lexer_src, "lexer.ac");
    let source = fixtures_root().join(format!("lexer/{fixture_stem}.ac"));
    write_bootstrap_run_source(&source);

    let exe = build_bootstrap_hir_aot_exe(&hir, &format!("m19_lexer_{fixture_stem}"), 0);
    let (code, stdout) = run_bootstrap_aot_exe(&exe);
    (code, filter_action_stdout(&stdout))
}

#[cfg(unix)]
#[test]
#[ignore = "AOT stdout oracle must run in an isolated process"]
fn test_bootstrap_m19_lexer_aot_tokenize_goldens() {
    for fixture in [
        "keywords",
        "literals",
        "operators",
        "ranges",
        "bootstrap_keywords",
    ] {
        let expected = lexer_golden_kinds(fixture);
        let (code, actual) = run_lexer_hir_aot_tokens(fixture);
        assert_eq!(
            code, 0,
            "lexer.ac AOT main should return 0 on lexer/{fixture}.ac"
        );
        assert_eq!(
            actual, expected,
            "lexer.ac bootstrap HIR AOT tokens should match M4 golden for {fixture}"
        );
    }
}

#[cfg(unix)]
#[test]
fn test_bootstrap_m19_lexer_aot_tokenize_goldens_subprocess() {
    run_isolated_test("test_bootstrap_m19_lexer_aot_tokenize_goldens");
}

/// `bad_return.ac` is rejected by the Rust frontend (bootstrap_forbidden).
#[test]
fn test_bootstrap_compiler_bad_return_rust_typecheck() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return.ac");
    let result = loader::check_file(&path, false);
    assert!(
        result.is_err(),
        "bad_return.ac should fail Rust frontend typecheck"
    );
}

/// TC2: bootstrap compiler exits 1 on Int/Bool return mismatch (still emits HIR).
#[test]
fn test_bootstrap_compiler_detects_return_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC8: bootstrap compiler exits 1 on parse errors (expect / invalid toplevel).
#[test]
fn test_bootstrap_compiler_detects_parse_errors() {
    let root = fixtures_root().join("bootstrap_bootstrap_only");
    for name in ["bad_parse_missing_paren", "bad_toplevel_return"] {
        let path = root.join(format!("{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// TC8: bootstrap compiler exits 1 on when branch type mismatch.
#[test]
fn test_bootstrap_compiler_detects_when_branch_mismatch() {
    let root = fixtures_root().join("bootstrap_bootstrap_only");
    for name in ["bad_when_branch_type", "bad_when_chain_type"] {
        let path = root.join(format!("{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// TC4: bootstrap compiler exits 1 on Int/String return mismatch.
#[test]
fn test_bootstrap_compiler_detects_return_string_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_string.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_string.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_bootstrap_compiler_detects_return_int_string_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_int_string.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_int_string.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC4 positive: String `return` with `+` on string literals must not false-positive.
#[test]
fn test_bootstrap_compiler_accepts_string_concat_return() {
    let path = fixtures_root().join("bootstrap/return_string_concat.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept return_string_concat.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC5: bootstrap compiler exits 1 on Bool/String return mismatch.
#[test]
fn test_bootstrap_compiler_detects_return_bool_string_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_bool_string.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_bool_string.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_bootstrap_compiler_detects_return_string_bool_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_string_bool.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_string_bool.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC5 positive: Bool `return` with comparison must not false-positive.
#[test]
fn test_bootstrap_compiler_accepts_bool_cmp_return() {
    let path = fixtures_root().join("bootstrap/return_bool_cmp.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept return_bool_cmp.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC6: bootstrap compiler exits 1 on Named/primitive return mismatch.
#[test]
fn test_bootstrap_compiler_detects_return_token_int_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_token_int.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_token_int.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_bootstrap_compiler_detects_return_int_point_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_int_point.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_int_point.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC6: struct literal field inference catches Point literal in Int function.
#[test]
fn test_bootstrap_compiler_detects_return_int_point_lit_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_int_point_lit.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_int_point_lit.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M21/TC6: custom struct literal in Int `main` is rejected (dynamic type table).
#[test]
fn test_bootstrap_m21_custom_struct_return_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_int_rect.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_int_rect.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M21: custom `type` alias gets a dynamic tag (not Int) and compiles.
#[test]
fn test_bootstrap_m21_custom_struct_accepts() {
    let path = fixtures_root().join("bootstrap/custom_struct.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept custom_struct.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M72: multi-digit tags (≥10) + tyTagName reverse lookup for String fields.
#[test]
fn test_bootstrap_m72_many_structs_accepts() {
    let path = fixtures_root().join("bootstrap/many_structs.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept many_structs.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M72: Path B compile_hir for many_structs (tag 10 round-trip).
#[test]
fn test_bootstrap_m72_many_structs_hir_compiles() {
    let path = fixtures_root().join("bootstrap/many_structs.ac");
    assert_bootstrap_hir_compiles_from(&path, "many_structs");
}

/// M73/TC9: wrong call arity rejected by bootstrap.
#[test]
fn test_bootstrap_m73_rejects_bad_call_arity() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_call_arity.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_call_arity.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M73/TC9: wrong call argument tag rejected by bootstrap.
#[test]
fn test_bootstrap_m73_rejects_bad_call_arg_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_call_arg_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_call_arg_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M74/TC10: struct hygiene — unknown field / missing / extra / field type.
#[test]
fn test_bootstrap_m74_rejects_struct_hygiene() {
    for name in [
        "bad_struct_unknown_field",
        "bad_struct_lit_missing",
        "bad_struct_field_ty",
        "bad_struct_lit_extra",
    ] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M75: List[String] index + for-in bind String element tags.
#[test]
fn test_bootstrap_m75_list_string_accepts() {
    let path = fixtures_root().join("bootstrap/list_string.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept list_string.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_bootstrap_m75_for_string_accepts() {
    let path = fixtures_root().join("bootstrap/for_string.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept for_string.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_bootstrap_m75_list_string_hir_compiles() {
    assert_bootstrap_hir_compiles_from(
        &fixtures_root().join("bootstrap/list_string.ac"),
        "list_string",
    );
}

#[test]
fn test_bootstrap_m75_for_string_hir_compiles() {
    assert_bootstrap_hir_compiles_from(
        &fixtures_root().join("bootstrap/for_string.ac"),
        "for_string",
    );
}

/// M77: non-exhaustive when / unknown variant rejected by bootstrap.
#[test]
fn test_bootstrap_m77_rejects_when_exhaustiveness() {
    for name in ["bad_when_non_exhaustive", "bad_when_unknown_variant"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M77: exhaustive Color when without else is accepted.
#[test]
fn test_bootstrap_m77_accepts_when_exhaustive() {
    let path = fixtures_root().join("bootstrap/when_exhaustive.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept when_exhaustive.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M78: custom enum variants bind to parent tag; exhaustive when accepts.
#[test]
fn test_bootstrap_m78_accepts_custom_enum() {
    let path = fixtures_root().join("bootstrap/custom_enum.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept custom_enum.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M78: custom enum non-exhaustive / unknown variant rejected.
#[test]
fn test_bootstrap_m78_rejects_custom_enum_exhaustiveness() {
    for name in [
        "bad_custom_enum_non_exhaustive",
        "bad_custom_enum_unknown_variant",
    ] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M78: Path B compile_hir for custom_enum.
#[test]
fn test_bootstrap_m78_custom_enum_hir_compiles() {
    assert_bootstrap_hir_compiles_from(
        &fixtures_root().join("bootstrap/custom_enum.ac"),
        "custom_enum",
    );
}

/// M79: value-match `and <guard>` with Bool guard is accepted.
#[test]
fn test_bootstrap_m79_accepts_when_guard_bool() {
    let path = fixtures_root().join("bootstrap/when_guard_bool.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept when_guard_bool.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M79/M83: non-Bool when guard rejected (Rust + bootstrap).
#[test]
fn test_bootstrap_m79_rejects_when_guard_not_bool() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_when_guard_not_bool.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_when_guard_not_bool.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M83: Rust frontend rejects non-Bool when guard (parity with bootstrap M79).
#[test]
fn test_bootstrap_m83_rust_rejects_when_guard_not_bool() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_when_guard_not_bool.ac");
    let result = loader::check_file(&path, false);
    assert!(
        result.is_err(),
        "Rust frontend should reject bad_when_guard_not_bool.ac"
    );
}

/// M80: `and` / `or` with non-Bool operands rejected.
#[test]
fn test_bootstrap_m80_rejects_logical_non_bool() {
    for name in ["bad_logical_and_int", "bad_logical_or_int"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M80: existing Bool `and`/`or` fixture still accepted.
#[test]
fn test_bootstrap_m80_accepts_logical_ops() {
    let path = fixtures_root().join("bootstrap/logical_ops.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept logical_ops.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M84: Named call-arg mismatches rejected (single-file; extends M73 primitives).
#[test]
fn test_bootstrap_m84_rejects_bad_call_named_arg_ty() {
    for name in [
        "bad_call_arg_int_point",
        "bad_call_arg_token_point",
        "bad_call_arg_point_int",
    ] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M84: Point→Point call arg accepted.
#[test]
fn test_bootstrap_m84_accepts_call_point_ok() {
    let path = fixtures_root().join("bootstrap/call_point_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept call_point_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M85: Named let/assign mismatches rejected (parity with Rust types_compatible).
#[test]
fn test_bootstrap_m85_rejects_bad_let_assign_named_ty() {
    for name in [
        "bad_let_int_point",
        "bad_let_point_int",
        "bad_assign_int_point",
        "bad_assign_point_int",
    ] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M85: Point let/assign accepted.
#[test]
fn test_bootstrap_m85_accepts_let_assign_point_ok() {
    for name in ["let_point_ok", "assign_point_ok"] {
        let path = fixtures_root().join(format!("bootstrap/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            output.status.success(),
            "bootstrap compiler should accept {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M86: field assign unknown / type mismatch rejected (parity with Rust E013 + types_compatible).
#[test]
fn test_bootstrap_m86_rejects_bad_field_assign() {
    for name in ["bad_field_assign_ty", "bad_field_assign_unknown"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M86: `p.x = …` accepted when types match.
#[test]
fn test_bootstrap_m86_accepts_field_assign_ok() {
    let path = fixtures_root().join("bootstrap/field_assign_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept field_assign_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M87: mixed List/Set/Map entry tags rejected.
#[test]
fn test_bootstrap_m87_rejects_mixed_collection_elems() {
    for name in ["bad_list_mixed", "bad_set_mixed", "bad_map_mixed"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M87: homogeneous List/Set/Map literals accepted.
#[test]
fn test_bootstrap_m87_accepts_coll_homo_ok() {
    let path = fixtures_root().join("bootstrap/coll_homo_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept coll_homo_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M88: M84–M87 positive fixtures on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m88_allowlisted_new_stems_accept() {
    for name in [
        "assign_point_ok",
        "call_point_ok",
        "coll_homo_ok",
        "field_assign_ok",
        "let_point_ok",
    ] {
        let path = fixtures_root().join(format!("bootstrap/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            output.status.success(),
            "bootstrap compiler should accept {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M89: index assign type mismatches rejected (List/Map; bootstrap via M86 + Rust refine).
#[test]
fn test_bootstrap_m89_rejects_bad_index_assign_ty() {
    for name in ["bad_index_assign_ty", "bad_map_index_assign_ty"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M89: `xs[i] = …` accepted when element types match.
#[test]
fn test_bootstrap_m89_accepts_index_assign_ok() {
    let path = fixtures_root().join("bootstrap/index_assign_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept index_assign_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M90: index_assign_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m90_allowlisted_index_assign_ok() {
    let path = fixtures_root().join("bootstrap/index_assign_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept index_assign_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"index_assign_ok"),
        "index_assign_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M91: List index / Map key type mismatches rejected.
#[test]
fn test_bootstrap_m91_rejects_bad_index_key_ty() {
    for name in ["bad_list_index_key", "bad_map_index_key"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M91: Int list index + String map key accepted.
#[test]
fn test_bootstrap_m91_accepts_index_key_ok() {
    let path = fixtures_root().join("bootstrap/index_key_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept index_key_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M92: index_key_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m92_allowlisted_index_key_ok() {
    let path = fixtures_root().join("bootstrap/index_key_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept index_key_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"index_key_ok"),
        "index_key_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M93: Sub/Mul/Div with Bool/String operands rejected.
#[test]
fn test_bootstrap_m93_rejects_bad_arith_ty() {
    for name in [
        "bad_arith_sub_bool",
        "bad_arith_mul_string",
        "bad_arith_div_bool",
    ] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M93: Int arithmetic accepted.
#[test]
fn test_bootstrap_m93_accepts_arith_ok() {
    let path = fixtures_root().join("bootstrap/arith_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept arith_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M94: arith_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m94_allowlisted_arith_ok() {
    let path = fixtures_root().join("bootstrap/arith_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept arith_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"arith_ok"),
        "arith_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M95: ordered compare mixed Bool/non-Bool rejected.
#[test]
fn test_bootstrap_m95_rejects_bad_cmp_ty() {
    for name in ["bad_cmp_lt_int_bool", "bad_cmp_gt_bool_int"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M95: Int ordered compare accepted.
#[test]
fn test_bootstrap_m95_accepts_cmp_ok() {
    let path = fixtures_root().join("bootstrap/cmp_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept cmp_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M96: cmp_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m96_allowlisted_cmp_ok() {
    let path = fixtures_root().join("bootstrap/cmp_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept cmp_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"cmp_ok"),
        "cmp_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M97: unary Neg with Bool/String rejected.
#[test]
fn test_bootstrap_m97_rejects_bad_unary_neg() {
    for name in ["bad_unary_neg_bool", "bad_unary_neg_string"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M97: unary Neg on Int accepted.
#[test]
fn test_bootstrap_m97_accepts_unary_neg_ok() {
    let path = fixtures_root().join("bootstrap/unary_neg_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept unary_neg_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M98: unary_neg_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m98_allowlisted_unary_neg_ok() {
    let path = fixtures_root().join("bootstrap/unary_neg_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept unary_neg_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"unary_neg_ok"),
        "unary_neg_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M99: range endpoints must be Int.
#[test]
fn test_bootstrap_m99_rejects_bad_range_ty() {
    for name in ["bad_range_bool_end", "bad_range_string_start"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M99: Int range accepted.
#[test]
fn test_bootstrap_m99_accepts_range_ok() {
    let path = fixtures_root().join("bootstrap/range_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept range_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M100: range_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m100_allowlisted_range_ok() {
    let path = fixtures_root().join("bootstrap/range_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept range_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"range_ok"),
        "range_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M101: when OneLine / ConditionChain non-Bool condition rejected.
#[test]
fn test_bootstrap_m101_rejects_bad_when_cond() {
    for name in ["bad_when_cond_int", "bad_when_chain_int"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M101: Bool when condition accepted.
#[test]
fn test_bootstrap_m101_accepts_when_cond_ok() {
    let path = fixtures_root().join("bootstrap/when_cond_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept when_cond_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M102: when_cond_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m102_allowlisted_when_cond_ok() {
    let path = fixtures_root().join("bootstrap/when_cond_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept when_cond_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"when_cond_ok"),
        "when_cond_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M103: for-condition non-Bool rejected.
#[test]
fn test_bootstrap_m103_rejects_bad_for_cond() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_for_cond_int.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_for_cond_int.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M103: Bool for-condition accepted.
#[test]
fn test_bootstrap_m103_accepts_for_cond_ok() {
    let path = fixtures_root().join("bootstrap/for_cond_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept for_cond_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M104: for_cond_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m104_allowlisted_for_cond_ok() {
    let path = fixtures_root().join("bootstrap/for_cond_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept for_cond_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"for_cond_ok"),
        "for_cond_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M105: tyCheckReturn delegates to tyCheckBind — return mismatch fixtures still exit 1.
#[test]
fn test_bootstrap_m105_return_via_ty_check_bind() {
    for name in [
        "bad_return",
        "bad_return_int_string",
        "bad_return_token_point",
        "bad_return_point_token",
    ] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac after M105 (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let path = fixtures_root().join("bootstrap/return_point_make.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should still accept return_point_make.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M106: Add with Bool operands rejected; string concat still ok.
#[test]
fn test_bootstrap_m106_rejects_bad_arith_add_bool() {
    for name in ["bad_arith_add_bool", "bad_arith_add_bool_left"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M106: String concat via Add accepted.
#[test]
fn test_bootstrap_m106_accepts_arith_add_string_ok() {
    let path = fixtures_root().join("bootstrap/arith_add_string_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept arith_add_string_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M107: arith_add_string_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m107_allowlisted_arith_add_string_ok() {
    let path = fixtures_root().join("bootstrap/arith_add_string_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept arith_add_string_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"arith_add_string_ok"),
        "arith_add_string_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M108: unary + with Bool/String rejected.
#[test]
fn test_bootstrap_m108_rejects_bad_unary_pos() {
    for name in ["bad_unary_pos_bool", "bad_unary_pos_string"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M108: unary + on Int still accepted (existing unary_plus fixture).
#[test]
fn test_bootstrap_m108_accepts_unary_plus() {
    let path = fixtures_root().join("bootstrap/unary_plus.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept unary_plus.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M109: assign to `val` rejected (stmt + expr forms).
#[test]
fn test_bootstrap_m109_rejects_bad_val_assign() {
    for name in ["bad_val_assign", "bad_val_assign_expr"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M109: assign to `var` still accepted (existing assign_expr fixture).
#[test]
fn test_bootstrap_m109_accepts_assign_expr() {
    let path = fixtures_root().join("bootstrap/assign_expr.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept assign_expr.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M110: field/index assign through `val` root rejected.
#[test]
fn test_bootstrap_m110_rejects_bad_val_field_index_assign() {
    for name in ["bad_field_assign_val", "bad_index_assign_val"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M110: field/index assign through `var` root still accepted.
#[test]
fn test_bootstrap_m110_accepts_field_index_assign_ok() {
    for name in ["field_assign_ok", "index_assign_ok"] {
        let path = fixtures_root().join(format!("bootstrap/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            output.status.success(),
            "bootstrap compiler should accept {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M111: undefined call rejected (single-file).
#[test]
fn test_bootstrap_m111_rejects_bad_undef_call() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_undef_call.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_undef_call.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M111: known builtins / declared calls still accepted.
#[test]
fn test_bootstrap_m111_accepts_known_calls() {
    for name in ["call_point_ok", "arith_add_string_ok"] {
        let path = fixtures_root().join(format!("bootstrap/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            output.status.success(),
            "bootstrap compiler should accept {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M112: undefined Ident outside bare return rejected (bind/arg/arith).
#[test]
fn test_bootstrap_m112_rejects_bad_undef_ident_use() {
    for name in [
        "bad_undef_bind",
        "bad_undef_arg",
        "bad_undef_arith",
        "bad_undef_var",
    ] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M112: defined idents still accepted.
#[test]
fn test_bootstrap_m112_accepts_defined_idents() {
    for name in ["arith_ok", "assign_expr", "env_scope_good"] {
        let path = fixtures_root().join(format!("bootstrap/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            output.status.success(),
            "bootstrap compiler should accept {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M113: String index key must be Int.
#[test]
fn test_bootstrap_m113_rejects_bad_string_index_key() {
    for name in ["bad_string_index_bool", "bad_string_index_string"] {
        let path = fixtures_root().join(format!("bootstrap_forbidden/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            !output.status.success(),
            "bootstrap compiler should exit 1 on {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M113: String[Int] accepted.
#[test]
fn test_bootstrap_m113_accepts_string_index_ok() {
    let path = fixtures_root().join("bootstrap/string_index_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept string_index_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M114: string_index_ok on Path B allowlist / golden stems.
#[test]
fn test_bootstrap_m114_allowlisted_string_index_ok() {
    let path = fixtures_root().join("bootstrap/string_index_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept string_index_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"string_index_ok"),
        "string_index_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M115: import funSig committed; wrong arity on imported fun rejected.
#[test]
fn test_bootstrap_m115_rejects_bad_import_call_arity() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_import_call_arity.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_import_call_arity.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M115: import funSig committed; wrong arg tag on imported fun rejected.
#[test]
fn test_bootstrap_m115_rejects_bad_import_call_arg_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_import_call_arg_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_import_call_arg_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M115: import_call_ok accepted + Path B allowlist.
#[test]
fn test_bootstrap_m115_allowlisted_import_call_ok() {
    let path = fixtures_root().join("bootstrap/import_call_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept import_call_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"import_call_ok"),
        "import_call_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M116: external funSig committed; wrong arity rejected.
#[test]
fn test_bootstrap_m116_rejects_bad_external_call_arity() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_external_call_arity.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_external_call_arity.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M116: external funSig committed; wrong arg tag rejected.
#[test]
fn test_bootstrap_m116_rejects_bad_external_call_arg_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_external_call_arg_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_external_call_arg_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M116: correct external call accepted (compiler-only; not Path B allowlisted).
#[test]
fn test_bootstrap_m116_accepts_external_call_ok() {
    let path = fixtures_root().join("bootstrap/external_call_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept external_call_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"external_call_ok"),
        "external_call_ok must stay off Path B allowlist (no host echo symbol)"
    );
}

/// M117: unknown nullary UFCS method rejected.
#[test]
fn test_bootstrap_m117_rejects_bad_ufcs_unknown() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_ufcs_unknown.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_ufcs_unknown.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M117: nullary UFCS `List[1,2,3].len()` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m117_allowlisted_ufcs_len_ok() {
    let path = fixtures_root().join("bootstrap/ufcs_len_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept ufcs_len_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"ufcs_len_ok"),
        "ufcs_len_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M118: fallible `or {}` type mismatch rejected.
#[test]
fn test_bootstrap_m118_rejects_bad_or_block_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_or_block_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_or_block_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M118: `parseInt(\"x\") or { 0 }` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m118_allowlisted_or_block_ok() {
    let path = fixtures_root().join("bootstrap/or_block_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept or_block_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"or_block_ok"),
        "or_block_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M119: implicit-it lambda body type error rejected.
#[test]
fn test_bootstrap_m119_rejects_bad_lambda_it_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_lambda_it_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_lambda_it_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M119: `{ it * 2 }(21)` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m119_allowlisted_lambda_it_ok() {
    let path = fixtures_root().join("bootstrap/lambda_it_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept lambda_it_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"lambda_it_ok"),
        "lambda_it_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M120: circular import rejected.
#[test]
fn test_bootstrap_m120_rejects_bad_import_cycle() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_import_cycle.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_import_cycle.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M120: missing module rejected.
#[test]
fn test_bootstrap_m120_rejects_bad_import_unknown() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_import_unknown.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_import_unknown.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M120: open-graph import of non-allowlist module accepted + Path B allowlist.
#[test]
fn test_bootstrap_m120_allowlisted_import_graph_ok() {
    let path = fixtures_root().join("bootstrap/import_graph_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept import_graph_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"import_graph_ok"),
        "import_graph_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M121: multi-param lambda body type error rejected.
#[test]
fn test_bootstrap_m121_rejects_bad_lambda_multi_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_lambda_multi_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_lambda_multi_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M121: `{ x, y -> x + y }(20, 22)` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m121_allowlisted_lambda_multi_ok() {
    let path = fixtures_root().join("bootstrap/lambda_multi_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept lambda_multi_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"lambda_multi_ok"),
        "lambda_multi_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M122: trailing lambda body type error rejected.
#[test]
fn test_bootstrap_m122_rejects_bad_trailing_lambda_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_trailing_lambda_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_trailing_lambda_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M122: `map(List[21]) { it * 2 }` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m122_allowlisted_trailing_lambda_ok() {
    let path = fixtures_root().join("bootstrap/trailing_lambda_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept trailing_lambda_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"trailing_lambda_ok"),
        "trailing_lambda_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M123: no-param lambda body type error rejected.
#[test]
fn test_bootstrap_m123_rejects_bad_lambda_block_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_lambda_block_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_lambda_block_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M123: `{ 21 * 2 }()` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m123_allowlisted_lambda_block_ok() {
    let path = fixtures_root().join("bootstrap/lambda_block_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept lambda_block_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"lambda_block_ok"),
        "lambda_block_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M124: fixtures-root import accepted + Path B allowlist.
#[test]
fn test_bootstrap_m124_allowlisted_import_fixtures_ok() {
    let path = fixtures_root().join("bootstrap/import_fixtures_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept import_fixtures_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"import_fixtures_ok"),
        "import_fixtures_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
    assert!(
        !bootstrap_dir().join("m124_lib.ac").is_file(),
        "m124_lib.ac must not live under bootstrap/ (fixtures-root proof)"
    );
    assert!(
        fixtures_root().join("bootstrap/m124_lib.ac").is_file(),
        "m124_lib.ac must exist under tests/fixtures/bootstrap/"
    );
}

/// M125: multi-stmt lambda body type error rejected.
#[test]
fn test_bootstrap_m125_rejects_bad_lambda_stmts_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_lambda_stmts_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_lambda_stmts_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M125: `{ 21; 21 * 2 }()` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m125_allowlisted_lambda_stmts_ok() {
    let path = fixtures_root().join("bootstrap/lambda_stmts_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept lambda_stmts_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"lambda_stmts_ok"),
        "lambda_stmts_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M126: multi-stmt if arm type error rejected.
#[test]
fn test_bootstrap_m126_rejects_bad_if_stmts_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_if_stmts_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_if_stmts_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M126: multi-stmt if then/else PlainBlock accepted + Path B allowlist.
#[test]
fn test_bootstrap_m126_allowlisted_if_stmts_ok() {
    let path = fixtures_root().join("bootstrap/if_stmts_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept if_stmts_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"if_stmts_ok"),
        "if_stmts_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M127: PlainBlock val init type error rejected.
#[test]
fn test_bootstrap_m127_rejects_bad_plain_block_val_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_val_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_val_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M127: `{ val a: Int = 21; a * 2 }` PlainBlock accepted + Path B allowlist.
#[test]
fn test_bootstrap_m127_allowlisted_plain_block_val_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_val_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_val_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_val_ok"),
        "plain_block_val_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M128: lambda body val init type error rejected.
#[test]
fn test_bootstrap_m128_rejects_bad_lambda_val_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_lambda_val_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_lambda_val_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M128: `{ 21; val a: Int = 21; a * 2 }()` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m128_allowlisted_lambda_val_ok() {
    let path = fixtures_root().join("bootstrap/lambda_val_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept lambda_val_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"lambda_val_ok"),
        "lambda_val_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M129: PlainBlock return-value type error rejected.
#[test]
fn test_bootstrap_m129_rejects_bad_plain_block_return_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_return_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_return_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M129: `if true { return 42; 0 }` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m129_allowlisted_plain_block_return_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_return_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_return_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_return_ok"),
        "plain_block_return_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M130: PlainBlock for-in range type error rejected.
#[test]
fn test_bootstrap_m130_rejects_bad_plain_block_for_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_for_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_for_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M130: PlainBlock `for i in 0..42` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m130_allowlisted_plain_block_for_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_for_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_for_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_for_ok"),
        "plain_block_for_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M131: PlainBlock for-condition type error rejected.
#[test]
fn test_bootstrap_m131_rejects_bad_plain_block_for_cond_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_for_cond_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_for_cond_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M131: PlainBlock `for s < 42` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m131_allowlisted_plain_block_for_cond_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_for_cond_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_for_cond_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_for_cond_ok"),
        "plain_block_for_cond_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M132: PlainBlock for-with-index heterogeneous List rejected.
#[test]
fn test_bootstrap_m132_rejects_bad_plain_block_for_with_index_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_for_with_index_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_for_with_index_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M132: PlainBlock `for idx, n in List[1,2,3]` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m132_allowlisted_plain_block_for_with_index_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_for_with_index_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_for_with_index_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_for_with_index_ok"),
        "plain_block_for_with_index_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M133: PlainBlock infinite for type error rejected.
#[test]
fn test_bootstrap_m133_rejects_bad_plain_block_for_infinite_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_for_infinite_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_for_infinite_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M133: PlainBlock `for { return 42 }` accepted + Path B allowlist.
#[test]
fn test_bootstrap_m133_allowlisted_plain_block_for_infinite_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_for_infinite_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_for_infinite_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_for_infinite_ok"),
        "plain_block_for_infinite_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M134: PlainBlock Map value for-in type error rejected.
#[test]
fn test_bootstrap_m134_rejects_bad_plain_block_map_values_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_map_values_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_map_values_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M134: PlainBlock `for v in Map` value bind accepted + Path B allowlist.
#[test]
fn test_bootstrap_m134_allowlisted_plain_block_map_values_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_map_values_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_map_values_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_map_values_ok"),
        "plain_block_map_values_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M135: non-Bool if around break in PlainBlock for rejected.
#[test]
fn test_bootstrap_m135_rejects_bad_plain_block_break_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_break_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_break_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M135: PlainBlock for-break / for-continue accepted + Path B allowlist.
#[test]
fn test_bootstrap_m135_allowlisted_plain_block_break_continue_ok() {
    for stem in ["plain_block_break_ok", "plain_block_continue_ok"] {
        let path = fixtures_root().join(format!("bootstrap/{stem}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            output.status.success(),
            "bootstrap compiler should accept {stem}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&stem),
            "{stem} must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
        );
    }
}

/// M136: key-bound String misused as Int after len(k) rejected.
#[test]
fn test_bootstrap_m136_rejects_bad_plain_block_map_keys_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_map_keys_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_map_keys_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M136: PlainBlock Map key for-in via len(k) accepted + Path B allowlist.
#[test]
fn test_bootstrap_m136_allowlisted_plain_block_map_keys_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_map_keys_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_map_keys_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_map_keys_ok"),
        "plain_block_map_keys_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M137: nested for range type error inside PlainBlock rejected.
#[test]
fn test_bootstrap_m137_rejects_bad_plain_block_nested_for_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_nested_for_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_nested_for_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M137: PlainBlock nested for-in accepted + Path B allowlist.
#[test]
fn test_bootstrap_m137_allowlisted_plain_block_nested_for_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_nested_for_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_nested_for_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_nested_for_ok"),
        "plain_block_nested_for_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M138: heterogeneous Map values in PlainBlock for k, v rejected.
#[test]
fn test_bootstrap_m138_rejects_bad_plain_block_map_iter_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_map_iter_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_map_iter_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M138: PlainBlock Map for k, v accepted + Path B allowlist.
#[test]
fn test_bootstrap_m138_allowlisted_plain_block_map_iter_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_map_iter_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_map_iter_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_map_iter_ok"),
        "plain_block_map_iter_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M139: heterogeneous Set elements in PlainBlock for-in rejected.
#[test]
fn test_bootstrap_m139_rejects_bad_plain_block_set_iter_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_set_iter_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_set_iter_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M139: PlainBlock Set for-in accepted + Path B allowlist.
#[test]
fn test_bootstrap_m139_allowlisted_plain_block_set_iter_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_set_iter_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_set_iter_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_set_iter_ok"),
        "plain_block_set_iter_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M140: when guard not Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m140_rejects_bad_plain_block_when_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_when_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_when_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M140: PlainBlock when ValueMatch accepted + Path B allowlist.
#[test]
fn test_bootstrap_m140_allowlisted_plain_block_when_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_when_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_when_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_when_ok"),
        "plain_block_when_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M142: field assign type error inside PlainBlock rejected.
#[test]
fn test_bootstrap_m142_rejects_bad_plain_block_field_assign_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_field_assign_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_field_assign_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M142: PlainBlock field assign accepted + Path B allowlist.
#[test]
fn test_bootstrap_m142_allowlisted_plain_block_field_assign_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_field_assign_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_field_assign_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_field_assign_ok"),
        "plain_block_field_assign_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M143: index assign type error inside PlainBlock rejected.
#[test]
fn test_bootstrap_m143_rejects_bad_plain_block_index_assign_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_index_assign_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_index_assign_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M143: PlainBlock index assign accepted + Path B allowlist.
#[test]
fn test_bootstrap_m143_allowlisted_plain_block_index_assign_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_index_assign_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_index_assign_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_index_assign_ok"),
        "plain_block_index_assign_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M144: when guard not Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m144_rejects_bad_plain_block_when_guard_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_when_guard_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_when_guard_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M144: PlainBlock when guard Bool accepted + Path B allowlist.
#[test]
fn test_bootstrap_m144_allowlisted_plain_block_when_guard_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_when_guard_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_when_guard_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_when_guard_ok"),
        "plain_block_when_guard_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M145: undefined print arg inside PlainBlock rejected.
#[test]
fn test_bootstrap_m145_rejects_bad_plain_block_print_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_print_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_print_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M145: PlainBlock print accepted + Path B allowlist.
#[test]
fn test_bootstrap_m145_allowlisted_plain_block_print_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_print_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_print_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_print_ok"),
        "plain_block_print_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M146: non-exhaustive when inside PlainBlock rejected.
#[test]
fn test_bootstrap_m146_rejects_bad_plain_block_when_exhaustive_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_when_exhaustive_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_when_exhaustive_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M146: PlainBlock when exhaustive accepted + Path B allowlist.
#[test]
fn test_bootstrap_m146_allowlisted_plain_block_when_exhaustive_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_when_exhaustive_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_when_exhaustive_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_when_exhaustive_ok"),
        "plain_block_when_exhaustive_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M147: unknown UFCS method inside PlainBlock rejected.
#[test]
fn test_bootstrap_m147_rejects_bad_plain_block_ufcs_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_ufcs_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_ufcs_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M147: PlainBlock nullary UFCS accepted + Path B allowlist.
#[test]
fn test_bootstrap_m147_allowlisted_plain_block_ufcs_len_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_ufcs_len_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_ufcs_len_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_ufcs_len_ok"),
        "plain_block_ufcs_len_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M148: or-block fallback type mismatch inside PlainBlock rejected.
#[test]
fn test_bootstrap_m148_rejects_bad_plain_block_or_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_or_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_or_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M148: PlainBlock fallible or {} accepted + Path B allowlist.
#[test]
fn test_bootstrap_m148_allowlisted_plain_block_or_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_or_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_or_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_or_ok"),
        "plain_block_or_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M149: ConditionChain arm not Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m149_rejects_bad_plain_block_when_chain_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_when_chain_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_when_chain_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M149: PlainBlock when ConditionChain accepted + Path B allowlist.
#[test]
fn test_bootstrap_m149_allowlisted_plain_block_when_condition_chain_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_when_condition_chain_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_when_condition_chain_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST
            .contains(&"plain_block_when_condition_chain_ok"),
        "plain_block_when_condition_chain_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M150: trailing lambda body type error inside PlainBlock rejected.
#[test]
fn test_bootstrap_m150_rejects_bad_plain_block_trailing_lambda_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_trailing_lambda_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_trailing_lambda_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M150: PlainBlock trailing lambda accepted + Path B allowlist.
#[test]
fn test_bootstrap_m150_allowlisted_plain_block_trailing_lambda_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_trailing_lambda_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_trailing_lambda_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_trailing_lambda_ok"),
        "plain_block_trailing_lambda_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M151: ConditionChain and-operand not Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m151_rejects_bad_plain_block_when_and_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_when_and_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_when_and_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M151: PlainBlock when ConditionChain + and accepted + Path B allowlist.
#[test]
fn test_bootstrap_m151_allowlisted_plain_block_when_and_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_when_and_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_when_and_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_when_and_ok"),
        "plain_block_when_and_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M152: Map index key not String inside PlainBlock rejected.
#[test]
fn test_bootstrap_m152_rejects_bad_plain_block_map_index_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_map_index_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_map_index_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M152: PlainBlock Map index read accepted + Path B allowlist.
#[test]
fn test_bootstrap_m152_allowlisted_plain_block_map_index_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_map_index_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_map_index_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_map_index_ok"),
        "plain_block_map_index_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M153: String index key not Int inside PlainBlock rejected.
#[test]
fn test_bootstrap_m153_rejects_bad_plain_block_string_index_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_string_index_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_string_index_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M153: PlainBlock String index accepted + Path B allowlist.
#[test]
fn test_bootstrap_m153_allowlisted_plain_block_string_index_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_string_index_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_string_index_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_string_index_ok"),
        "plain_block_string_index_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M154: not operand not Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m154_rejects_bad_plain_block_logical_not_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_logical_not_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_logical_not_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M154: PlainBlock logical not accepted + Path B allowlist.
#[test]
fn test_bootstrap_m154_allowlisted_plain_block_logical_not_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_logical_not_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_logical_not_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_logical_not_ok"),
        "plain_block_logical_not_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M155: exclusive range end not Int inside PlainBlock rejected.
#[test]
fn test_bootstrap_m155_rejects_bad_plain_block_for_range_exclusive_ty() {
    let path =
        fixtures_root().join("bootstrap_forbidden/bad_plain_block_for_range_exclusive_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_for_range_exclusive_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M155: PlainBlock exclusive range for accepted + Path B allowlist.
#[test]
fn test_bootstrap_m155_allowlisted_plain_block_for_range_exclusive_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_for_range_exclusive_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_for_range_exclusive_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST
            .contains(&"plain_block_for_range_exclusive_ok"),
        "plain_block_for_range_exclusive_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M156: for-in List[String] body type error inside PlainBlock rejected.
#[test]
fn test_bootstrap_m156_rejects_bad_plain_block_for_string_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_for_string_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_for_string_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M156: PlainBlock for-in List[String] accepted + Path B allowlist.
#[test]
fn test_bootstrap_m156_allowlisted_plain_block_for_string_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_for_string_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_for_string_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_for_string_ok"),
        "plain_block_for_string_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M157: unary + on Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m157_rejects_bad_plain_block_unary_plus_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_unary_plus_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_unary_plus_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M157: PlainBlock unary plus accepted + Path B allowlist.
#[test]
fn test_bootstrap_m157_allowlisted_plain_block_unary_plus_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_unary_plus_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_unary_plus_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_unary_plus_ok"),
        "plain_block_unary_plus_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M158: unary - on Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m158_rejects_bad_plain_block_unary_neg_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_unary_neg_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_unary_neg_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M158: PlainBlock unary neg accepted + Path B allowlist.
#[test]
fn test_bootstrap_m158_allowlisted_plain_block_unary_neg_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_unary_neg_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_unary_neg_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_unary_neg_ok"),
        "plain_block_unary_neg_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M159: and operand not Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m159_rejects_bad_plain_block_logical_ops_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_logical_ops_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_logical_ops_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M159: PlainBlock and/or accepted + Path B allowlist.
#[test]
fn test_bootstrap_m159_allowlisted_plain_block_logical_ops_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_logical_ops_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_logical_ops_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_logical_ops_ok"),
        "plain_block_logical_ops_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M160: assign Point to Int inside PlainBlock rejected.
#[test]
fn test_bootstrap_m160_rejects_bad_plain_block_assign_expr_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_assign_expr_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_assign_expr_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M160: PlainBlock assign accepted + Path B allowlist.
#[test]
fn test_bootstrap_m160_allowlisted_plain_block_assign_expr_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_assign_expr_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_assign_expr_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_assign_expr_ok"),
        "plain_block_assign_expr_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M161: Int - Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m161_rejects_bad_plain_block_arith_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_arith_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_arith_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M161: PlainBlock arith accepted + Path B allowlist.
#[test]
fn test_bootstrap_m161_allowlisted_plain_block_arith_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_arith_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_arith_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_arith_ok"),
        "plain_block_arith_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M162: Int < Bool inside PlainBlock rejected.
#[test]
fn test_bootstrap_m162_rejects_bad_plain_block_cmp_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_cmp_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_cmp_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M162: PlainBlock cmp accepted + Path B allowlist.
#[test]
fn test_bootstrap_m162_allowlisted_plain_block_cmp_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_cmp_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_cmp_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_cmp_ok"),
        "plain_block_cmp_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M163: inclusive range end not Int inside PlainBlock rejected.
#[test]
fn test_bootstrap_m163_rejects_bad_plain_block_for_range_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_for_range_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_for_range_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M163: PlainBlock inclusive for-range accepted + Path B allowlist.
#[test]
fn test_bootstrap_m163_allowlisted_plain_block_for_range_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_for_range_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_for_range_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_for_range_ok"),
        "plain_block_for_range_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M164: heterogeneous List inside PlainBlock rejected.
#[test]
fn test_bootstrap_m164_rejects_bad_plain_block_coll_homo_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_coll_homo_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_coll_homo_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M164: PlainBlock coll homo accepted + Path B allowlist.
#[test]
fn test_bootstrap_m164_allowlisted_plain_block_coll_homo_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_coll_homo_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_coll_homo_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_coll_homo_ok"),
        "plain_block_coll_homo_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M165: String + Int inside PlainBlock rejected.
#[test]
fn test_bootstrap_m165_rejects_bad_plain_block_arith_add_string_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_arith_add_string_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_arith_add_string_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M165: PlainBlock string + accepted + Path B allowlist.
#[test]
fn test_bootstrap_m165_allowlisted_plain_block_arith_add_string_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_arith_add_string_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_arith_add_string_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_arith_add_string_ok"),
        "plain_block_arith_add_string_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M166: heterogeneous List[String]/Int inside PlainBlock rejected.
#[test]
fn test_bootstrap_m166_rejects_bad_plain_block_list_string_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_list_string_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_list_string_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M166: PlainBlock List[String] lit accepted + Path B allowlist.
#[test]
fn test_bootstrap_m166_allowlisted_plain_block_list_string_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_list_string_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_list_string_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_list_string_ok"),
        "plain_block_list_string_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M167: Map index key not String inside PlainBlock rejected.
#[test]
fn test_bootstrap_m167_rejects_bad_plain_block_index_key_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_index_key_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_index_key_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M167: PlainBlock index key accepted + Path B allowlist.
#[test]
fn test_bootstrap_m167_allowlisted_plain_block_index_key_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_index_key_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_index_key_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_index_key_ok"),
        "plain_block_index_key_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M168: heterogeneous Map values inside PlainBlock rejected.
#[test]
fn test_bootstrap_m168_rejects_bad_plain_block_map_literal_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_map_literal_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_map_literal_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M168: PlainBlock Map lit accepted + Path B allowlist.
#[test]
fn test_bootstrap_m168_allowlisted_plain_block_map_literal_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_map_literal_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_map_literal_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_map_literal_ok"),
        "plain_block_map_literal_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M169: Int % Bool inside PlainBlock for rejected.
#[test]
fn test_bootstrap_m169_rejects_bad_plain_block_for_modulo_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_for_modulo_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_for_modulo_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M169: PlainBlock for+modulo accepted + Path B allowlist.
#[test]
fn test_bootstrap_m169_allowlisted_plain_block_for_modulo_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_for_modulo_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_for_modulo_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_for_modulo_ok"),
        "plain_block_for_modulo_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M170: String fun returns Int rejected (call site in PlainBlock).
#[test]
fn test_bootstrap_m170_rejects_bad_plain_block_return_string_concat_ty() {
    let path =
        fixtures_root().join("bootstrap_forbidden/bad_plain_block_return_string_concat_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_return_string_concat_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M170: PlainBlock string return/concat accepted + Path B allowlist.
#[test]
fn test_bootstrap_m170_allowlisted_plain_block_return_string_concat_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_return_string_concat_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_return_string_concat_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST
            .contains(&"plain_block_return_string_concat_ok"),
        "plain_block_return_string_concat_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M171: assign Int to Point inside PlainBlock rejected.
#[test]
fn test_bootstrap_m171_rejects_bad_plain_block_assign_point_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_assign_point_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_assign_point_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M171: PlainBlock Point assign accepted + Path B allowlist.
#[test]
fn test_bootstrap_m171_allowlisted_plain_block_assign_point_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_assign_point_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_assign_point_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_assign_point_ok"),
        "plain_block_assign_point_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M172: val Int = Point inside PlainBlock rejected.
#[test]
fn test_bootstrap_m172_rejects_bad_plain_block_let_point_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_let_point_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_let_point_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M172: PlainBlock Point let accepted + Path B allowlist.
#[test]
fn test_bootstrap_m172_allowlisted_plain_block_let_point_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_let_point_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_let_point_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_let_point_ok"),
        "plain_block_let_point_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M173: call Point-param with Int inside PlainBlock rejected.
#[test]
fn test_bootstrap_m173_rejects_bad_plain_block_call_point_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_call_point_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_call_point_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M173: PlainBlock Point call accepted + Path B allowlist.
#[test]
fn test_bootstrap_m173_allowlisted_plain_block_call_point_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_call_point_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_call_point_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_call_point_ok"),
        "plain_block_call_point_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M174: return Point where Int expected inside PlainBlock rejected.
#[test]
fn test_bootstrap_m174_rejects_bad_plain_block_return_point_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_return_point_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_return_point_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M174: PlainBlock return Point make accepted + Path B allowlist.
#[test]
fn test_bootstrap_m174_allowlisted_plain_block_return_point_make_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_return_point_make_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_return_point_make_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_return_point_make_ok"),
        "plain_block_return_point_make_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M175: Bool fun returns String rejected (call site in PlainBlock).
#[test]
fn test_bootstrap_m175_rejects_bad_plain_block_return_bool_cmp_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_return_bool_cmp_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_return_bool_cmp_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M175: PlainBlock Bool return/cmp accepted + Path B allowlist.
#[test]
fn test_bootstrap_m175_allowlisted_plain_block_return_bool_cmp_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_return_bool_cmp_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_return_bool_cmp_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_return_bool_cmp_ok"),
        "plain_block_return_bool_cmp_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M176: Token fun returns Int rejected (call site in PlainBlock).
#[test]
fn test_bootstrap_m176_rejects_bad_plain_block_return_token_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_return_token_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_return_token_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M176: PlainBlock Token return accepted + Path B allowlist.
#[test]
fn test_bootstrap_m176_allowlisted_plain_block_return_token_make_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_return_token_make_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_return_token_make_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_return_token_make_ok"),
        "plain_block_return_token_make_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M177: exclusive range end not Int inside PlainBlock rejected.
#[test]
fn test_bootstrap_m177_rejects_bad_plain_block_range_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_range_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_range_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M177: PlainBlock exclusive range accepted + Path B allowlist.
#[test]
fn test_bootstrap_m177_allowlisted_plain_block_range_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_range_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_range_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_range_ok"),
        "plain_block_range_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M178: non-Bool if cond inside PlainBlock rejected.
#[test]
fn test_bootstrap_m178_rejects_bad_plain_block_when_cond_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_when_cond_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_when_cond_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M178: PlainBlock when/if cond accepted + Path B allowlist.
#[test]
fn test_bootstrap_m178_allowlisted_plain_block_when_cond_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_when_cond_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_when_cond_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_when_cond_ok"),
        "plain_block_when_cond_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M179: Rect field Bool where Int expected inside PlainBlock rejected.
#[test]
fn test_bootstrap_m179_rejects_bad_plain_block_custom_struct_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_custom_struct_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_custom_struct_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M179: PlainBlock custom struct accepted + Path B allowlist.
#[test]
fn test_bootstrap_m179_allowlisted_plain_block_custom_struct_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_custom_struct_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_custom_struct_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_custom_struct_ok"),
        "plain_block_custom_struct_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M180: T2 field String where Int expected inside PlainBlock rejected.
#[test]
fn test_bootstrap_m180_rejects_bad_plain_block_many_structs_ty() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_plain_block_many_structs_ty.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_plain_block_many_structs_ty.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M180: PlainBlock many structs accepted + Path B allowlist.
#[test]
fn test_bootstrap_m180_allowlisted_plain_block_many_structs_ok() {
    let path = fixtures_root().join("bootstrap/plain_block_many_structs_ok.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept plain_block_many_structs_ok.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.contains(&"plain_block_many_structs_ok"),
        "plain_block_many_structs_ok must be on BOOTSTRAP_FRONTEND_ALLOWLIST"
    );
}

/// M81: `not` with Bool operand accepted.
#[test]
fn test_bootstrap_m81_accepts_logical_not() {
    let path = fixtures_root().join("bootstrap/logical_not.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept logical_not.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M81: `not` with non-Bool operand rejected (Rust + bootstrap after M82 parity).
#[test]
fn test_bootstrap_m81_rejects_logical_not_int() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_logical_not_int.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_logical_not_int.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// M82: new allowlisted stems accept via dedicated compiler checks (covered by stems loop too).
#[test]
fn test_bootstrap_m82_allowlisted_new_stems_accept() {
    for name in [
        "custom_enum",
        "when_exhaustive",
        "when_guard_bool",
        "logical_not",
    ] {
        let path = fixtures_root().join(format!("bootstrap/{name}.ac"));
        let output = run_bootstrap_compiler_on(&path);
        assert!(
            output.status.success(),
            "bootstrap compiler should accept {name}.ac (stderr: {})",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// M76: driver allowlist stays aligned with fixture stems used by Path B harness.
#[test]
fn test_bootstrap_m76_allowlist_matches_fixture_stems() {
    let mut allow = action::driver::BOOTSTRAP_FRONTEND_ALLOWLIST.to_vec();
    let mut stems = BOOTSTRAP_FIXTURE_STEMS.to_vec();
    allow.sort();
    stems.sort();
    assert_eq!(
        allow, stems,
        "BOOTSTRAP_FRONTEND_ALLOWLIST must match BOOTSTRAP_FIXTURE_STEMS"
    );
}

/// M76: `action check --frontend bootstrap` accepts allowlisted fixture (no Rust typecheck of input).
#[test]
fn test_bootstrap_m76_cli_check_frontend_bootstrap() {
    let path = fixtures_root().join("bootstrap/jit_smoke.ac");
    let output = run_action(&[
        "check",
        "--frontend",
        "bootstrap",
        path.to_str().expect("utf8 path"),
    ]);
    assert!(
        output.status.success(),
        "action check --frontend bootstrap jit_smoke failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Bootstrap frontend check passed"),
        "unexpected stdout: {stdout}"
    );
}

/// M76: non-allowlisted path is rejected by the CLI gate.
#[test]
fn test_bootstrap_m76_cli_rejects_non_allowlisted() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_undef_var.ac");
    let output = run_action(&[
        "check",
        "--frontend",
        "bootstrap",
        path.to_str().expect("utf8 path"),
    ]);
    assert!(
        !output.status.success(),
        "non-allowlisted file should fail --frontend bootstrap"
    );
    let err = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        err.contains("allowlist") || err.contains("Allowlist"),
        "expected allowlist error, got: {err}"
    );
}

/// M76 dual oracle: driver Path B HIR compiles; Rust frontend also accepts the same stem.
#[test]
fn test_bootstrap_m76_dual_oracle_sample() {
    for stem in ["jit_smoke", "for_string", "map_keys", "list_string"] {
        let path = fixtures_root().join(format!("bootstrap/{stem}.ac"));
        let action_bin = action_binary();
        let result = action::driver::check_file_bootstrap(&path, &action_bin)
            .unwrap_or_else(|e| panic!("bootstrap frontend {stem}: {e}"));
        action::driver::verify_bootstrap_hir(&result.hir, stem)
            .unwrap_or_else(|e| panic!("compile_hir {stem}: {e}"));
        loader::check_file(&path, false)
            .unwrap_or_else(|e| panic!("Rust frontend should still accept {stem}: {e:?}"));
    }
}

/// TC6 positive: Token `return` with struct literal must not false-positive.
#[test]
fn test_bootstrap_compiler_accepts_token_make_return() {
    let path = fixtures_root().join("bootstrap/return_token_make.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept return_token_make.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC7: bootstrap compiler exits 1 on distinct Named type return mismatch.
#[test]
fn test_bootstrap_compiler_detects_return_point_token_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_point_token.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_point_token.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_bootstrap_compiler_detects_return_token_point_mismatch() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_return_token_point.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_return_token_point.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC7 positive: Point `return` with struct literal must not false-positive.
#[test]
fn test_bootstrap_compiler_accepts_point_make_return() {
    let path = fixtures_root().join("bootstrap/return_point_make.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept return_point_make.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC3: global fun names resolve across functions; local `val`/`param` cleared per `fun`.
#[test]
fn test_bootstrap_tc3_env_scope_good_accepts() {
    let path = fixtures_root().join("bootstrap/env_scope_good.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        output.status.success(),
        "bootstrap compiler should accept env_scope_good.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_bootstrap_tc3_env_scope_main_oracle() {
    assert_bootstrap_main_oracle("env_scope_good");
}

/// TC3: bootstrap compiler exits 1 when `return` uses an undefined identifier.
#[test]
fn test_bootstrap_compiler_detects_undefined_return_ident() {
    let path = fixtures_root().join("bootstrap_forbidden/bad_undef_var.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on bad_undef_var.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC3 negative: `inner`'s local `leaked` must not be visible in `main` (exit 1).
#[test]
fn test_bootstrap_tc3_env_scope_local_not_leaked() {
    let path = fixtures_root().join("bootstrap/env_scope_leak.ac");
    let output = run_bootstrap_compiler_on(&path);
    assert!(
        !output.status.success(),
        "bootstrap compiler should exit 1 on env_scope_leak.ac (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// TC3 negative oracle: undefined `leaked` in `main` still emits HIR with ty Unit.
#[test]
fn test_bootstrap_tc3_env_scope_leak_hir_oracle() {
    let path = fixtures_root().join("bootstrap/env_scope_leak.ac");
    write_bootstrap_compile_input(&path);
    let compiler_ac = bootstrap_dir().join("compiler.ac");
    let output = run_action(&["run", compiler_ac.to_str().unwrap()]);
    assert!(
        !output.status.success(),
        "env_scope_leak.ac should fail bootstrap compile (stderr: {})",
        String::from_utf8_lossy(&output.stderr)
    );
    let emitted = bootstrap_dir().join("_hir_out.json");
    let raw = fs::read_to_string(&emitted).expect("read bootstrap hir json");
    let json: serde_json::Value = serde_json::from_str(&raw).expect("hir json");
    let ty = main_return_ident_ty(&json, "leaked").expect("main should return ident `leaked`");
    assert_eq!(
        ty,
        serde_json::Value::String("Unit".to_owned()),
        "undefined ident should keep Unit ty in emitted HIR"
    );
}

fn run_bootstrap_compiler_on(source: &Path) -> std::process::Output {
    write_bootstrap_compile_input(source);
    let compiler_ac = bootstrap_dir().join("compiler.ac");
    run_action(&["run", compiler_ac.to_str().unwrap()])
}

fn load_bootstrap_hir_from_source(source: &Path, label: &str) -> HirModule {
    write_bootstrap_compile_input(source);

    let compiler_ac = bootstrap_dir().join("compiler.ac");
    let output = run_action(&["run", compiler_ac.to_str().unwrap()]);
    assert!(
        output.status.success(),
        "bootstrap compiler failed on {label}: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let emitted = bootstrap_dir().join("_hir_out.json");
    let raw = fs::read_to_string(&emitted).expect("read bootstrap hir json");
    serde_json::from_str(&raw).unwrap_or_else(|e| {
        panic!("bootstrap HIR for {label} should deserialize as HirModule: {e}\n{raw}")
    })
}

fn run_isolated_test(test_name: &str) {
    let exe = std::env::current_exe().expect("current test exe");
    let output = Command::new(exe)
        .args([test_name, "--exact", "--test-threads=1", "--ignored"])
        .output()
        .unwrap_or_else(|e| panic!("spawn isolated test {test_name}: {e}"));
    assert!(
        output.status.success(),
        "isolated test {test_name} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_bootstrap_hir_compiles_from(source: &Path, label: &str) {
    let hir = load_bootstrap_hir_from_source(source, label);
    let context = Context::create();
    let mut cg = CodeGen::new(
        &context,
        &format!("bootstrap_{label}"),
        TypeRegistry::new(),
        None,
    );
    cg.set_opt_level(0);
    cg.compile_hir(&hir)
        .unwrap_or_else(|e| panic!("compile_hir failed for bootstrap {label}: {e}"));
    cg.verify()
        .unwrap_or_else(|e| panic!("LLVM verify failed for bootstrap {label}: {e}"));
}

fn assert_bootstrap_hir_compiles_from_no_verify(source: &Path, label: &str) {
    let hir = load_bootstrap_hir_from_source(source, label);
    let context = Context::create();
    let mut cg = CodeGen::new(
        &context,
        &format!("bootstrap_{label}"),
        TypeRegistry::new(),
        None,
    );
    cg.set_opt_level(0);
    cg.compile_hir(&hir)
        .unwrap_or_else(|e| panic!("compile_hir failed for bootstrap {label}: {e}"));
}

/// Golden fixtures: compile_hir smoke (LLVM verify runs in isolated subprocess tests below).
#[test]
fn test_bootstrap_hir_codegen_goldens_compile() {
    for stem in BOOTSTRAP_FIXTURE_STEMS {
        assert_bootstrap_hir_compiles_from_no_verify(&bootstrap_fixture_ac(stem), stem);
    }
}

/// LLVM verify for golden fixtures (isolated subprocess). Skips `infinite_for` (non-terminating loop).
#[test]
#[ignore = "LLVM verify must run in an isolated process"]
fn test_bootstrap_hir_verify_fixture_goldens() {
    for stem in BOOTSTRAP_FIXTURE_STEMS {
        if *stem == "infinite_for" {
            continue;
        }
        assert_bootstrap_hir_compiles_from(&bootstrap_fixture_ac(stem), stem);
    }
}

#[test]
#[ignore = "LLVM verify must run in an isolated process"]
fn test_bootstrap_hir_verify_token_ac() {
    assert_bootstrap_hir_compiles_from(&bootstrap_dir().join("token.ac"), "token.ac");
}

/// Run LLVM verify for each fixture in a fresh process (avoids in-process LLVM type clashes).
#[test]
fn test_bootstrap_hir_codegen_goldens_verify_subprocess() {
    run_isolated_test("test_bootstrap_hir_verify_fixture_goldens");
    run_isolated_test("test_bootstrap_hir_verify_token_ac");
}

#[test]
fn test_bootstrap_hir_codegen_token_ac() {
    // String-in-struct IR can fail LLVM verify after other codegen tests in-process; compile_hir suffices.
    assert_bootstrap_hir_compiles_from_no_verify(&bootstrap_dir().join("token.ac"), "token.ac");
}

#[test]
fn test_bootstrap_hir_codegen_lexer_ac() {
    // LLVM verify for lexer.ac runs in test_bootstrap_m15_hir_verify_lexer_ac (subprocess).
    assert_bootstrap_hir_compiles_from_no_verify(&bootstrap_dir().join("lexer.ac"), "lexer.ac");
}

#[test]
fn test_bootstrap_hir_codegen_compiler_ac() {
    // LLVM verify for compiler.ac runs in test_bootstrap_m15_hir_verify_compiler_ac (subprocess).
    assert_bootstrap_hir_compiles_from_no_verify(
        &bootstrap_dir().join("compiler.ac"),
        "compiler.ac",
    );
}
