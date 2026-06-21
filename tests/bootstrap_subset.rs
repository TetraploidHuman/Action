//! Bootstrap subset boundary tests: allowed fixtures typecheck; forbidden ones fail.

use action::loader;
use std::fs;
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn collect_at_files(dir: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|_| panic!("read dir {}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("at"))
        .collect();
    paths.sort();
    paths
}

#[test]
fn test_bootstrap_subset_allowed_typechecks() {
    let dir = fixtures_root().join("bootstrap");
    for path in collect_at_files(&dir) {
        let result = loader::load_checked(&path, false);
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
        let result = loader::load_checked(&path, false);
        assert!(
            result.is_err(),
            "forbidden bootstrap fixture should fail typecheck: {}",
            path.display()
        );
    }
}
