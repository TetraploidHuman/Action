use std::path::PathBuf;
use std::process::Command;

fn action_binary() -> PathBuf {
    // CARGO_BIN_EXE_action is set by cargo test itself — trust it unconditionally.
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_action") {
        return PathBuf::from(&path);
    }

    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target");
    let exe_suffix = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };

    let candidates: &[&str] = if cfg!(target_os = "windows") {
        &["x86_64-pc-windows-msvc/debug/action"]
    } else {
        &[
            "x86_64-unknown-linux-gnu/debug/action",
            "aarch64-unknown-linux-gnu/debug/action",
        ]
    };

    for c in candidates {
        let p = base.join(format!("{}{}", c, exe_suffix));
        if p.exists() {
            return p;
        }
    }

    // Fallback: default target dir (no --target)
    let p = base.join(format!("debug/action{}", exe_suffix));
    if p.exists() {
        return p;
    }

    panic!("action binary not found — build with `cargo build` first");
}

fn run_example(name: &str) -> String {
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    let output = Command::new(action_binary())
        .args(["run", example.to_str().unwrap()])
        .output()
        .expect(&format!("Failed to run example: {}", name));
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    // Normalize CRLF -> LF so tests pass on Windows where CRT emits \r\n.
    // Strip all \r to handle cases where git CRLF conversion adds an extra
    // carriage return (e.g. multiline string literals in .at source files).
    stdout.replace("\r\n", "\n").replace('\r', "")
}

#[test]
fn test_hello() {
    assert_eq!(run_example("hello.at"), "Hello, World!\n");
}

#[test]
fn test_fn_ref() {
    assert_eq!(run_example("fn_ref.at"), "42");
}

#[test]
fn test_lambda() {
    assert_eq!(run_example("lambda.at"), "42423042");
}

#[test]
fn test_struct() {
    assert_eq!(run_example("struct.at"), "1020");
}

#[test]
fn test_shorthand_struct() {
    assert_eq!(run_example("shorthand_struct.at"), "1020");
}

#[test]
fn test_enum() {
    assert_eq!(run_example("enum.at"), "Red42");
}

#[test]
fn test_tuple() {
    assert_eq!(run_example("tuple.at"), "12342");
}

#[test]
fn test_destructure() {
    assert_eq!(run_example("destructure.at"), "4210");
}

#[test]
fn test_char_literal() {
    assert_eq!(run_example("char_literal.at"), "65");
}

#[test]
fn test_number_literals() {
    assert_eq!(run_example("number_literals.at"), "105112552408");
}

#[test]
fn test_power() {
    assert_eq!(run_example("power.at"), "8181102449");
}

#[test]
fn test_bitwise() {
    assert_eq!(run_example("bitwise.at"), "176-184");
}

#[test]
fn test_short_circuit() {
    assert_eq!(run_example("short_circuit.at"), "04200770");
}

#[test]
fn test_compound() {
    assert_eq!(run_example("compound.at"), "151312332");
}

#[test]
fn test_range_exclusive() {
    assert_eq!(run_example("range_exclusive.at"), "01234");
}

#[test]
fn test_for_loop() {
    assert_eq!(run_example("for_loop.at"), "012341011");
}

#[test]
fn test_yield() {
    assert_eq!(run_example("yield.at"), "125210127");
}

#[test]
fn test_nested_for() {
    assert_eq!(run_example("nested_for.at"), "110111210211111221223132");
}

#[test]
fn test_math() {
    assert_eq!(run_example("math_builtins.at"), "4209910-10720-57");
}

#[test]
fn test_const() {
    assert_eq!(run_example("const.at"), "1024390");
}

#[test]
fn test_fn_type() {
    assert_eq!(run_example("fn_type.at"), "20");
}

#[test]
fn test_fn_type2() {
    assert_eq!(run_example("fn_type2.at"), "2021");
}

#[test]
fn test_type_ann() {
    assert_eq!(run_example("type_ann.at"), "4212");
}

#[test]
fn test_list() {
    assert_eq!(run_example("list.at"), "103050");
}

#[test]
fn test_map_filter() {
    assert_eq!(run_example("map_filter.at"), "210215");
}

#[test]
fn test_str_match() {
    assert_eq!(run_example("str_match.at"), "1234");
}

#[test]
fn test_is_match() {
    assert_eq!(run_example("is_match.at"), "123");
}

#[test]
fn test_when_match() {
    assert_eq!(run_example("when_match.at"), "the answer42");
}

#[test]
fn test_when_chain() {
    assert_eq!(run_example("when_chain.at"), "positivemedium");
}

#[test]
fn test_stdlib() {
    assert_eq!(run_example("stdlib.at"), "42993150200");
}

#[test]
fn test_propagate() {
    assert_eq!(run_example("propagate.at"), "449");
}

#[test]
fn test_safe_access() {
    assert_eq!(run_example("safe_access.at"), "10429");
}

#[test]
fn test_multiline() {
    assert_eq!(run_example("multiline.at"), "Hello\nWorld");
}

#[test]
fn test_interp() {
    assert_eq!(
        run_example("interp.at"),
        "Hello, World!Age: 42World is 42 years olddone"
    );
}

// ---- Additional complex tests for CI coverage ----

#[test]
fn test_nested_for3() {
    // 3-level nested for (tests generalized N-binding fix, was hardcoded to 2)
    // Cartesian product: x in [1,2], y in [1,2], z in [1,2] -> 8 elements
    // r[0]=111, r[3]=122, r[5]=212, r[7]=222
    assert_eq!(run_example("test_nested_for3.at"), "111122212222");
}

#[test]
fn test_when_complex() {
    // Guards with compound conditions, or-patterns, is-patterns,
    // expression-based condition-chain when, destructuring arms
    assert_eq!(run_example("test_when_complex.at"), "2010012-199");
}

#[test]
fn test_for_collect() {
    // for-expressions with continue filtering, break early exit
    assert_eq!(run_example("test_for_collect.at"), "246192515");
}

#[test]
fn test_enum_option() {
    // Option enum: safe division, chained operations, nested pattern matching
    assert_eq!(run_example("test_enum_option.at"), "2014325");
}

#[test]
fn test_block_scope() {
    // Block scoping, shadowing, mutable variable mutation across blocks
    assert_eq!(run_example("test_block_scope.at"), "1015123");
}

#[test]
fn test_compare_chain() {
    // All comparison operators, chained comparisons with and/or,
    // short-circuit evaluation of and/or (Bools print as true/false)
    assert_eq!(run_example("test_compare_chain.at"), "truetruetruetruetruetruetruefalse00420770");
}

#[test]
fn test_safe_ops() {
    // Safe field access (?.) on Some and None propagation
    assert_eq!(run_example("test_safe_ops.at"), "101");
}

#[test]
fn test_arithmetic_complex() {
    // Operator precedence, mixed arithmetic, unary ops, compound assignment
    // Note: ** is left-associative
    assert_eq!(run_example("test_arithmetic_complex.at"), "14203264645-71615251410");
}

#[test]
fn test_lambda_capture() {
    // Lambda capturing outer variables: base=100, n=5, offset=1000
    assert_eq!(run_example("test_lambda_capture.at"), "120150151050");
}

#[test]
fn test_struct_nested() {
    // Nested struct: struct fields inside struct, two-level field access
    assert_eq!(run_example("test_struct_nested.at"), "1020100200");
}
