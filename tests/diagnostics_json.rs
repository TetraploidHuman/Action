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

#[test]
fn test_check_format_json_reports_e001_code() {
    let file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/test_error_e001_parseInt.ac");
    let output = Command::new(action_binary())
        .args(["check", "--format", "json", file.to_str().unwrap()])
        .output()
        .expect("failed to run action check");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("check --format json should emit valid JSON");
    let diags = json["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty());
    assert_eq!(diags[0]["code"], "E001");
}

#[test]
fn test_check_format_json_reports_e006_code() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/test_error_e006_list_var_index.ac");
    let output = Command::new(action_binary())
        .args(["check", "--format", "json", file.to_str().unwrap()])
        .output()
        .expect("failed to run action check");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("check --format json should emit valid JSON");
    let diags = json["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty());
    assert_eq!(diags[0]["code"], "E006");
}

#[test]
fn test_check_format_json_reports_e008_code() {
    let file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/test_error_e008_map_var_key.ac");
    let output = Command::new(action_binary())
        .args(["check", "--format", "json", file.to_str().unwrap()])
        .output()
        .expect("failed to run action check");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("check --format json should emit valid JSON");
    let diags = json["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty());
    assert_eq!(diags[0]["code"], "E008");
}

#[test]
fn test_check_format_json_explain_e002_includes_help() {
    let file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/test_error_e002_or_type.ac");
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
    let diags = json["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty());
    assert_eq!(diags[0]["code"], "E002");
    let help = diags[0]["help"].as_str().expect("E002 should include help");
    assert!(
        help.contains("or {"),
        "expected or-block help, got: {}",
        help
    );
}

#[test]
fn test_check_format_json_reports_e003_code() {
    let file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/test_error_e003_fn_or.ac");
    let output = Command::new(action_binary())
        .args(["check", "--format", "json", file.to_str().unwrap()])
        .output()
        .expect("failed to run action check");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("check --format json should emit valid JSON");
    let diags = json["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty());
    assert_eq!(diags[0]["code"], "E003");
}

#[test]
fn test_check_format_json_reports_e010_code() {
    let file =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/test_error_e010_null.ac");
    let output = Command::new(action_binary())
        .args(["check", "--format", "json", file.to_str().unwrap()])
        .output()
        .expect("failed to run action check");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("check --format json should emit valid JSON");
    let diags = json["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty());
    assert_eq!(diags[0]["code"], "E010");
}

#[test]
fn test_check_format_json_reports_e011_code() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/test_error_e011_nullable_type.ac");
    let output = Command::new(action_binary())
        .args(["check", "--format", "json", file.to_str().unwrap()])
        .output()
        .expect("failed to run action check");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("check --format json should emit valid JSON");
    let diags = json["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty());
    assert_eq!(diags[0]["code"], "E011");
}

#[test]
fn test_check_format_json_reports_e012_code() {
    let file = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/test_error_standalone_question.ac");
    let output = Command::new(action_binary())
        .args(["check", "--format", "json", file.to_str().unwrap()])
        .output()
        .expect("failed to run action check");
    assert!(!output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("check --format json should emit valid JSON");
    let diags = json["diagnostics"].as_array().expect("diagnostics array");
    assert!(!diags.is_empty());
    assert_eq!(diags[0]["code"], "E012");
}
