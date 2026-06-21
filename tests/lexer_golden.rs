//! Lexer golden tests: token JSON must match fixtures under tests/fixtures/lexer/.

use action_frontend::lexer::{tokens_to_json, Lexer};
use std::fs;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lexer")
}

fn tokenize_json(source: &str) -> String {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize();
    tokens_to_json(&tokens)
}

fn assert_lexer_golden(fixture_stem: &str) {
    let dir = fixtures_dir();
    let at_path = dir.join(format!("{fixture_stem}.at"));
    let json_path = dir.join(format!("{fixture_stem}.tokens.json"));

    let source = fs::read_to_string(&at_path).expect("read fixture source");
    let expected = fs::read_to_string(&json_path).expect("read golden tokens json");
    let actual = tokenize_json(&source);

    assert_eq!(
        normalize_json(&actual),
        normalize_json(&expected),
        "lexer golden mismatch for {}",
        at_path.display()
    );
}

fn normalize_json(s: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(s).expect("valid json");
    serde_json::to_string(&v).expect("compact json")
}

#[test]
fn test_lexer_golden_keywords() {
    assert_lexer_golden("keywords");
}

#[test]
fn test_lexer_golden_literals() {
    assert_lexer_golden("literals");
}

#[test]
fn test_lexer_golden_operators() {
    assert_lexer_golden("operators");
}

#[test]
fn test_lexer_golden_all_fixtures() {
    let dir = fixtures_dir();
    for entry in fs::read_dir(&dir).expect("read lexer fixtures dir") {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("at") {
            continue;
        }
        let stem = path.file_stem().and_then(|s| s.to_str()).expect("stem");
        let json_path = dir.join(format!("{stem}.tokens.json"));
        assert!(
            json_path.exists(),
            "missing golden json for {}",
            path.display()
        );
        assert_lexer_golden(stem);
    }
}
