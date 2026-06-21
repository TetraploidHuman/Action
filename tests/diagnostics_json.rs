use std::path::PathBuf;
use std::process::Command;

fn action_binary() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_action") {
        return PathBuf::from(path);
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/debug/action")
}

#[test]
fn test_check_format_json_reports_type_error() {
    let file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/test_error_generic_mismatch.ac");
    let output = Command::new(action_binary())
        .args([
            "check",
            "--explain",
            "--format",
            "json",
            file.to_str().unwrap(),
        ])
        .output()
        .expect("failed to run action check");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("check --format json should emit valid JSON");
    assert_eq!(json["version"], 1);
    let diags = json["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty());
    assert!(diags[0]["message"]
        .as_str()
        .unwrap()
        .contains("Cannot infer type arguments"));
    // help is optional depending on error class
}

#[test]
fn test_check_format_json_success_empty() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/hello.ac");
    let output = Command::new(action_binary())
        .args(["check", "--format", "json", file.to_str().unwrap()])
        .output()
        .expect("failed to run action check");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid JSON on success");
    assert_eq!(json["diagnostics"].as_array().unwrap().len(), 0);
}
