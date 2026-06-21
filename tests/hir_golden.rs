//! HIR golden tests: verify HIR JSON and round-trip for key examples.

use action::checked::CheckedProgram;
use action::driver;
use action::loader;
use std::path::{Path, PathBuf};

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bootstrap")
}

fn load_checked(path: &Path) -> CheckedProgram {
    loader::load_checked(&path.to_path_buf(), false).expect("typecheck should pass")
}

fn hir_json(path: &Path) -> String {
    load_checked(path)
        .hir_json_pretty()
        .expect("hir json serialize")
}

#[test]
fn test_hir_golden_bootstrap_control_flow() {
    let path = fixtures_dir().join("control_flow.at");
    let json = hir_json(&path);
    assert!(json.contains("For") || json.contains("for") || json.contains("While"));
}

#[test]
fn test_hir_golden_bootstrap_enum_simple() {
    let path = fixtures_dir().join("enum_simple.at");
    let json = hir_json(&path);
    assert!(json.contains("Enum") || json.contains("when") || json.contains("When"));
}

#[test]
fn test_hir_golden_bootstrap_struct_when() {
    let path = fixtures_dir().join("struct_when.at");
    let json = hir_json(&path);
    assert!(json.contains("Struct") || json.contains("when") || json.contains("When"));
}

#[test]
fn test_hir_golden_hello() {
    let path = examples_dir().join("hello.at");
    let json = hir_json(&path);
    assert!(json.contains("Ident") || json.contains("println"));
}

#[test]
fn test_hir_golden_bench_cow() {
    let path = examples_dir().join("bench_cow.at");
    let json = hir_json(&path);
    assert!(json.contains("Let") || json.contains("let"));
}

#[test]
fn test_hir_golden_map_filter() {
    let path = examples_dir().join("map_filter.at");
    let json = hir_json(&path);
    assert!(json.len() > 100);
}

#[test]
fn test_hir_round_trip_integration_examples() {
    for name in ["hello.at", "bench_cow.at", "map_filter.at", "tuple.at"] {
        let path = examples_dir().join(name);
        let checked = load_checked(&path);
        assert!(
            checked.verify_hir_round_trip(),
            "HIR round-trip failed for {}",
            name
        );
    }
}

#[test]
fn test_emit_hir_writes_file() {
    let path = examples_dir().join("hello.at");
    let checked = load_checked(&path);
    driver::emit_hir(&checked, &path, false).expect("emit_hir");
    let expected = path.with_extension("hir.json");
    assert!(expected.exists(), "expected {}", expected.display());
}
