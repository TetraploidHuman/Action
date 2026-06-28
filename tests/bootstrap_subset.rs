//! Bootstrap subset boundary tests: allowed fixtures typecheck; forbidden ones fail.

use action::loader;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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

fn collect_at_files(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|_| panic!("read dir {}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("ac"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn test_bootstrap_subset_allowed_typechecks() {
    let dir = fixtures_root().join("bootstrap");
    for path in collect_at_files(&dir) {
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
    for path in collect_at_files(&dir) {
        let result = loader::check_file(&path, false);
        assert!(
            result.is_err(),
            "forbidden bootstrap fixture should fail typecheck: {}",
            path.display()
        );
    }
}

/// M4: bootstrap `lexer.ac` token text output must match Rust lexer golden kinds for `keywords.ac`.
#[test]
fn test_bootstrap_m4_lexer_matches_keywords_golden() {
    let json_path = fixtures_root().join("lexer/keywords.tokens.json");
    let expected_json = fs::read_to_string(&json_path).expect("read keywords.tokens.json");
    let tokens: Vec<serde_json::Value> =
        serde_json::from_str(&expected_json).expect("parse keywords golden");
    let expected_kinds: Vec<&str> = tokens
        .iter()
        .map(|t| t["kind"].as_str().expect("kind"))
        .collect();

    let lexer_ac = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bootstrap/lexer.ac");
    let output = Command::new(action_binary())
        .args(["run", lexer_ac.to_str().unwrap()])
        .output()
        .expect("run bootstrap/lexer.ac");
    assert!(
        output.status.success(),
        "bootstrap lexer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let normalized = stdout.replace("\r\n", "\n");
    let actual: Vec<&str> = normalized.trim_end().split('\n').collect();
    assert_eq!(
        actual, expected_kinds,
        "bootstrap lexer token text should match Rust lexer golden kinds"
    );
}
