use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

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

fn run_example_starts_with(name: &str, prefix: &str) {
    let output = run_example(name);
    assert!(
        output.starts_with(prefix),
        "Expected output to start with {:?}, but got {:?}",
        prefix,
        output
    );
}

fn run_example(name: &str) -> String {
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    run_action_file(&example)
}

fn run_action_file(path: &std::path::Path) -> String {
    let output = Command::new(action_binary())
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect(&format!("Failed to run: {}", path.display()));
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    // Normalize CRLF -> LF so tests pass on Windows where CRT emits \r\n.
    // Strip all \r to handle cases where git CRLF conversion adds an extra
    // carriage return (e.g. multiline string literals in .ac source files).
    stdout.replace("\r\n", "\n").replace('\r', "")
}

/// Run an example that is expected to fail. Returns stderr.
/// Asserts the process exits with non-zero status.
fn run_example_fails(name: &str) -> String {
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    let output = Command::new(action_binary())
        .args(["run", example.to_str().unwrap()])
        .output()
        .expect(&format!("Failed to run example: {}", name));
    assert!(
        !output.status.success(),
        "Expected {} to fail, but it succeeded.\nstdout: {}",
        name,
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    stderr.replace("\r\n", "\n").replace('\r', "")
}

/// Assert that running `name` fails with `expected_msg` contained in stderr.
fn assert_compile_error(name: &str, expected_msg: &str) {
    let stderr = run_example_fails(name);
    assert!(
        stderr.contains(expected_msg),
        "Expected error containing {:?} when compiling {}, but got:\n{}",
        expected_msg,
        name,
        stderr
    );
}

/// Assert `action check --format json` reports structured diagnostic `expected_code`.
fn assert_compile_error_code(name: &str, expected_code: &str) {
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    let output = Command::new(action_binary())
        .args(["check", "--format", "json", example.to_str().unwrap()])
        .output()
        .expect(&format!("Failed to check example: {}", name));
    assert!(
        !output.status.success(),
        "Expected {} to fail type-check, but it succeeded.\nstdout: {}",
        name,
        String::from_utf8_lossy(&output.stdout)
    );
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let needle = format!("\"code\": \"{}\"", expected_code);
    assert!(
        stdout.contains(&needle),
        "Expected diagnostic code {} when checking {}, but got:\n{}",
        expected_code,
        name,
        stdout
    );
}

fn assert_compile_error_at(path: &std::path::Path, expected_msg: &str) {
    let output = Command::new(action_binary())
        .args(["run", path.to_str().unwrap()])
        .output()
        .expect(&format!("Failed to run: {}", path.display()));
    assert!(
        !output.status.success(),
        "Expected {} to fail, but it succeeded.\nstdout: {}",
        path.display(),
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr)
        .replace("\r\n", "\n")
        .replace('\r', "");
    assert!(
        stderr.contains(expected_msg),
        "Expected error containing {:?} when compiling {}, but got:\n{}",
        expected_msg,
        path.display(),
        stderr
    );
}

#[test]
fn test_hello() {
    assert_eq!(run_example("hello.ac"), "Hello, World!\n");
}

#[test]
fn test_fn_ref() {
    assert_eq!(run_example("fn_ref.ac"), "42");
}

#[test]
fn test_lambda() {
    assert_eq!(run_example("lambda.ac"), "42423042");
}

#[test]
fn test_struct() {
    assert_eq!(run_example("struct.ac"), "1020");
}

#[test]
fn test_shorthand_struct() {
    assert_eq!(run_example("shorthand_struct.ac"), "1020");
}

#[test]
fn test_enum() {
    assert_eq!(run_example("enum.ac"), "Red42");
}

#[test]
fn test_tuple() {
    assert_eq!(run_example("tuple.ac"), "12342");
}

#[test]
fn test_destructure() {
    assert_eq!(run_example("destructure.ac"), "4210");
}

#[test]
fn test_char_literal() {
    assert_eq!(run_example("char_literal.ac"), "65");
}

#[test]
fn test_number_literals() {
    assert_eq!(run_example("number_literals.ac"), "105112552408");
}

#[test]
fn test_power() {
    assert_eq!(run_example("power.ac"), "8181102449");
}

#[test]
fn test_bitwise() {
    assert_eq!(run_example("bitwise.ac"), "176-184");
}

#[test]
fn test_short_circuit() {
    assert_eq!(run_example("short_circuit.ac"), "04200770");
}

#[test]
fn test_compound() {
    assert_eq!(run_example("compound.ac"), "151312332");
}

#[test]
fn test_range_exclusive() {
    assert_eq!(run_example("range_exclusive.ac"), "01234");
}

#[test]
fn test_for_loop() {
    assert_eq!(run_example("for_loop.ac"), "012341011");
}

#[test]
fn test_for_with_index() {
    assert_eq!(run_example("for_with_index.ac"), "63\n9\n");
}

#[test]
fn test_lazy_val() {
    assert_eq!(run_example("lazy_val_test.ac"), "142\n42\n");
}

#[test]
fn test_yield() {
    assert_eq!(run_example("yield.ac"), "125210127");
}

#[test]
fn test_nested_for() {
    assert_eq!(run_example("nested_for.ac"), "110111210211111221223132");
}

#[test]
fn test_math() {
    assert_eq!(run_example("math_builtins.ac"), "4209910-10720-57");
}

#[test]
fn test_const() {
    assert_eq!(run_example("const.ac"), "1024390");
}

#[test]
fn test_fn_type() {
    assert_eq!(run_example("fn_type.ac"), "20");
}

#[test]
fn test_fn_type2() {
    assert_eq!(run_example("fn_type2.ac"), "2021");
}

#[test]
fn test_type_ann() {
    assert_eq!(run_example("type_ann.ac"), "4212");
}

#[test]
fn test_list() {
    assert_eq!(run_example("list.ac"), "103050");
}

#[test]
fn test_map_filter() {
    assert_eq!(run_example("map_filter.ac"), "210215");
}

#[test]
fn test_str_match() {
    assert_eq!(run_example("str_match.ac"), "1234");
}

#[test]
fn test_is_match() {
    assert_eq!(run_example("is_match.ac"), "123");
}

#[test]
fn test_when_match() {
    assert_eq!(run_example("when_match.ac"), "the answer42");
}

#[test]
fn test_when_chain() {
    assert_eq!(run_example("when_chain.ac"), "positivemedium");
}

#[test]
fn test_stdlib() {
    assert_eq!(run_example("stdlib.ac"), "42993150200");
}

#[test]
fn test_import_math() {
    assert_eq!(run_example("import.ac"), "15712");
}

#[test]
fn test_atom_path_dependency() {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/path_dep");
    let main = fixture.join("main.ac");
    assert_eq!(run_action_file(&main), "42\n");
}

#[test]
fn test_propagate() {
    assert_eq!(run_example("propagate.ac"), "449");
}

#[test]
fn test_safe_access() {
    assert_eq!(run_example("safe_access.ac"), "104210");
}

#[test]
fn test_multiline() {
    assert_eq!(run_example("multiline.ac"), "Hello\nWorld");
}

#[test]
fn test_interp() {
    assert_eq!(
        run_example("interp.ac"),
        "Hello, World!Age: 42World is 42 years olddone"
    );
}

// ---- Additional complex tests for CI coverage ----

#[test]
fn test_nested_for3() {
    // 3-level nested for (tests generalized N-binding fix, was hardcoded to 2)
    // Cartesian product: x in [1,2], y in [1,2], z in [1,2] -> 8 elements
    // r[0]=111, r[3]=122, r[5]=212, r[7]=222
    assert_eq!(run_example("test_nested_for3.ac"), "111122212222");
}

#[test]
fn test_when_complex() {
    // Guards with compound conditions, or-patterns, is-patterns,
    // expression-based condition-chain when, destructuring arms
    assert_eq!(run_example("test_when_complex.ac"), "2010012-199");
}

#[test]
fn test_when_no_else_match() {
    // value-match when without else branch — only matching arm executes
    assert_eq!(run_example("test_when_no_else_match.ac"), "23");
}

#[test]
fn test_when_no_else_chain() {
    // condition-chain when without else branch — only matching arm executes
    assert_eq!(run_example("test_when_no_else_chain.ac"), "23");
}

#[test]
fn test_for_collect() {
    // for-expressions with continue filtering, break early exit
    assert_eq!(run_example("test_for_collect.ac"), "246192515");
}

#[test]
fn test_enum_option() {
    // Option enum: safe division, chained operations, nested pattern matching
    assert_eq!(run_example("test_enum_option.ac"), "2014325");
}

#[test]
fn test_block_scope() {
    // Block scoping, shadowing, mutable variable mutation across blocks
    assert_eq!(run_example("test_block_scope.ac"), "1015123");
}

#[test]
fn test_compare_chain() {
    // All comparison operators, chained comparisons with and/or,
    // short-circuit evaluation of and/or (Bools print as true/false)
    assert_eq!(
        run_example("test_compare_chain.ac"),
        "truetruetruetruetruetruetruefalse00420770"
    );
}

#[test]
fn test_safe_ops() {
    // Safe field access (?.) on Some and None propagation
    assert_eq!(run_example("test_safe_ops.ac"), "101");
}

#[test]
fn test_arithmetic_complex() {
    // Operator precedence, mixed arithmetic, unary ops, compound assignment
    // Note: ** is left-associative
    assert_eq!(
        run_example("test_arithmetic_complex.ac"),
        "1420325364645-71615251410"
    );
}

#[test]
fn test_struct_nested() {
    // Nested struct: struct fields inside struct, two-level field access
    assert_eq!(run_example("test_struct_nested.ac"), "1020100200");
}

#[test]
fn test_lambda_capture() {
    assert_eq!(run_example("test_lambda_capture.ac"), "42");
}

#[test]
fn test_map_option() {
    assert_eq!(run_example("test_map_option.ac"), "100\n-1\n");
}

#[test]
fn test_read_line() {
    // No stdin input: readLine returns None, prints "EOF"
    assert_eq!(run_example("test_read_line.ac"), "EOF");
}

#[test]
fn test_read_line_with_input() {
    // Pipe input to stdin: readLine should return the input string
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("test_read_line_with_input.ac");
    let mut child = Command::new(action_binary())
        .args(["run", example.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn action");

    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"hello\n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout)
        .replace("\r\n", "\n")
        .replace('\r', "");
    assert_eq!(stdout, "hello\n", "readLine should return piped input");
}

#[test]
fn test_read_line_multiple() {
    // Pipe multiple lines: each readLine call should return one line
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("test_read_line_multi.ac");
    let mut child = Command::new(action_binary())
        .args(["run", example.to_str().unwrap()])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn action");

    {
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(b"Alice\nBob\nquit\n").unwrap();
    }

    let output = child.wait_with_output().expect("Failed to read output");
    let stdout = String::from_utf8_lossy(&output.stdout)
        .replace("\r\n", "\n")
        .replace('\r', "");
    assert_eq!(stdout, "Hello, Alice\nHello, Bob\nGoodbye!");
}

#[test]
fn test_io() {
    // readLine with EOF -> or { } default "World"
    assert_eq!(run_example("io.ac"), "Hello, World\n");
}

// ---- Edge-case tests: pattern matching ----

#[test]
fn test_and_guards() {
    assert_eq!(run_example("test_and_guards.ac"), "positive\ndone\n");
}

#[test]
fn test_or_patterns() {
    assert_eq!(run_example("test_or_patterns.ac"), "small\ndone\n");
}

// ---- Edge-case tests: data structures ----

#[test]
fn test_named_tuple() {
    assert_eq!(
        run_example("test_named_tuple.ac"),
        "name: Alice\nage: 30\npos0: Alice\ndone\n"
    );
}

#[test]
fn test_struct_destructure() {
    assert_eq!(
        run_example("test_struct_destructure.ac"),
        "x: 10\ny: 20\ndone\n"
    );
}

#[test]
fn test_map_set_ops() {
    assert_eq!(run_example("test_map_set.ac"), "true100999true");
}

#[test]
fn test_empty_collections() {
    assert_eq!(run_example("test_empty_collections.ac"), "0true0true00true");
}

// ---- Edge-case tests: functions ----

#[test]
fn test_tco_deep() {
    // Deep recursion that would overflow without TCO (n=5000)
    assert_eq!(run_example("test_tco.ac"), "12036288005005000");
}

#[test]
fn test_overload_str() {
    assert_eq!(run_example("test_overload_str.ac"), "Hello, World\n42\n");
}

// ---- Edge-case tests: callbacks and closures ----

#[test]
fn test_pat_cb() {
    // Pattern binding + callback in same function
    assert_eq!(run_example("test_pat_cb.ac"), "42");
}

#[test]
fn test_simple_cb() {
    // Untyped callback parameter
    assert_eq!(run_example("test_simple_cb.ac"), "15");
}

#[test]
fn test_cb4() {
    // Callback returning Int
    assert_eq!(run_example("test_cb4.ac"), "42");
}

#[test]
fn test_cb2() {
    // Callback returning Option, called via typed function
    assert_eq!(run_example("test_cb2.ac"), "4210");
}

#[test]
fn test_cb5() {
    // Callback returning Option (simpler variant)
    assert_eq!(run_example("test_cb5.ac"), "42");
}

#[test]
fn test_multi_capture() {
    // Multiple closures capturing the same variable
    assert_eq!(run_example("test_nested_closure.ac"), "4284");
}

#[test]
fn test_closure_loop() {
    // Closures in for loops capturing loop variable
    assert_eq!(run_example("test_closure_loop.ac"), "15");
}

// ---- Edge-case tests: float and string ----

#[test]
fn test_float_edge() {
    // Float arithmetic edge cases: decimals, negatives, fractions
    assert_eq!(run_example("test_float_edge.ac"), "truetruetruetruetrue");
}

#[test]
fn test_string_edge() {
    // String manipulation edge cases
    assert_eq!(run_example("test_string_edge.ac"), "Hello Worldbcd0312");
}

// ---- Edge-case tests: stream, coroutine, task ----

#[test]
fn test_stream_ops() {
    assert_eq!(run_example("test_stream.ac"), "4299done");
}

#[test]
fn test_stream_buffer() {
    // Stress test: multiple send/receive cycles verify memmove preserves leaf header.
    assert_eq!(
        run_example("test_stream_buffer.ac"),
        "10203040100200300DONE"
    );
}

#[test]
fn test_coroutine() {
    assert_eq!(run_example("test_coroutine.ac"), "322");
}

#[test]
fn test_task_stream() {
    assert_eq!(
        run_example("test_task_stream.ac"),
        "4299falsefalse1237falsefalse456"
    );
}

// ---- Edge-case tests: imports ----

#[test]
fn test_import_selective() {
    assert_eq!(
        run_example("test_import_selective.ac"),
        "15\n5\n3.14159\ndone\n"
    );
}

#[test]
fn test_import_wildcard() {
    assert_eq!(
        run_example("test_import_wildcard.ac"),
        "15\n5\n3.14159\ndone\n"
    );
}

// ---- Comprehensive builtin tests ----

#[test]
fn test_network_ping() {
    // Verify action_test_ping() FFI returns 42
    assert_eq!(run_example("test_network_ping.ac"), "42\n");
}

#[test]
fn test_http_error() {
    // Request to a port where nothing is listening — should return error status "0"
    run_example_starts_with("test_http_error.ac", "0\n");
}

// ---- JSON tests ----

#[test]
fn test_json() {
    // Minimal test: just verify action_json_parse runs without crashing
    assert_eq!(run_example("test_json.ac"), "42\n");
}

#[test]
fn test_json_error() {
    // action_json_parse on invalid JSON returns null, action_json_type(null) returns -1
    assert_eq!(run_example("test_json_error.ac"), "-1\n");
}

#[test]
fn test_json_lib() {
    // JSON library round-trip via import json.{jsonParse, jsonGet, jsonStringify}
    assert_eq!(run_example("test_json_lib.ac"), "\"hi\"\n\"hi\"\n7\n7\n");
}

#[test]
fn test_higher_order() {
    // find, flatMap, sortedBy, partition
    assert_eq!(run_example("test_higher_order.ac"), "15512413");
}

// ---- Fallible (or {}) tests ----

#[test]
fn test_fallible_or_default() {
    // R7 fallible or-block: toInt succeeds or falls back
    assert_eq!(run_example("test_fallible_or_default.ac"), "42\n-1\n");
}

#[test]
fn test_fallible_builtin_returns() {
    assert_eq!(
        run_example("test_fallible_builtin_returns.ac"),
        "tail([1,2,3]) len: 2\n\
         init([1,2,3]) len: 2\n\
         tail([]) fallback len: 0\n\
         toInt('42'): 42\n\
         toInt('abc') fallback: -999\n\
         parseInt('123'): 123\n\
         parseInt('not_a_number') fallback: -999\n\
         done\n"
    );
}

#[test]
fn test_tutorial() {
    assert_eq!(
        run_example("tutorial.ac"),
        "=== Action Language Tutorial ===\n\
Hello, World!\n\
Count: 3\n\
Typed: 42, typed string\n\
Int: 42, Float: 3.14, Bool: true, String: hello\n\
parsed: 42, bad parse fallback: -1\n\
add(3, 4) = 7\n\
factorial(5) = 120\n\
identity(42) = 42\n\
identity(\"hi\") = hi\n\
len: 14\n\
upper: HELLO, ACTION!\n\
lower: hello, action!\n\
substring(7, 13): Action!\n\
split count: 2\n\
joined: Hello | Action!\n\
nums len: 5\n\
len: 5\n\
head: 1\n\
doubled len: 5\n\
evens len: 2\n\
sum: 15\n\
squares len: 5\n\
empty map: true\n\
map len: 3\n\
contains 'a': true\n\
get 'a': 1\n\
set contains 3: true\n\
set size: 5\n\
when value: three\n\
grade: B\n\
max: 5\n\
Color: red\n\
unwrap: 42\n\
addFn(10, 20) = 30\n\
mapped len: 5\n\
addBase(5): 15\n\
first: 1, empty list head fallback: -1\n\
1 2 3 4 5 \n\
cubes len: 5\n\
word count: 13\n\
=== Tutorial Complete ===\n"
    );
}

#[test]
fn test_find_index_pred() {
    assert_eq!(run_example("find_index_pred.ac"), "1\n1\n");
}

#[test]
fn test_complex_fallible_when() {
    assert_eq!(run_example("complex_fallible_when.ac"), "427high1\n");
}

#[test]
fn test_new_builtins_test() {
    assert_eq!(
        run_example("new_builtins_test.ac"),
        "tail len: 3\nzip len: 3\nlines count: 3\nindexOf(3, xs): 2\ndone\n"
    );
}

#[test]
fn test_fallible_list_var_index_or() {
    assert_eq!(
        run_example("test_fallible_list_var_index_or.ac"),
        "20\n-1\n"
    );
}

#[test]
fn test_fallible_map_var_key_or() {
    assert_eq!(run_example("test_fallible_map_var_key_or.ac"), "20\n-1\n");
}

#[test]
fn test_fallible_set_var_elem_or() {
    assert_eq!(run_example("test_fallible_set_var_elem_or.ac"), "2\n-1\n");
}

#[test]
fn test_fallible_head_parseInt() {
    assert_eq!(
        run_example("test_fallible_head_parseInt.ac"),
        "1\n-1\n99\n0\n"
    );
}

#[test]
fn test_fallible_fn_or() {
    assert_eq!(run_example("test_fallible_fn_or.ac"), "42\n-1\n99\n0\n");
}

#[test]
fn test_fallible_block_or_and_narrowing() {
    assert_eq!(run_example("test_fallible_block_or.ac"), "10\n99\n42\n-1\n");
    assert_eq!(run_example("test_fallible_narrowing_for.ac"), "6\n");
}

#[test]
fn test_fallible_readLine() {
    assert_eq!(run_example("test_fallible_readLine.ac"), "EOF\n");
}

#[test]
fn test_fallible_json_parse() {
    assert_eq!(run_example("test_fallible_json_parse.ac"), "5\n-1\n");
}

#[test]
fn test_fallible_user_fn_propagate() {
    assert_eq!(
        run_example("test_fallible_user_fn_propagate.ac"),
        "42\n-1\n"
    );
}

#[test]
fn test_fallible_generic() {
    assert_eq!(run_example("test_fallible_generic.ac"), "42\n-1\n");
}

#[test]
fn test_fallible_module() {
    assert_eq!(run_example("test_fallible_module.ac"), "42\n-1\n");
}

#[test]
fn test_error_e003_fn_or_return() {
    assert_compile_error_code("test_error_e003_fn_or.ac", "E003");
}

#[test]
fn test_error_e001_module_fallible() {
    assert_compile_error_code("test_error_e001_module.ac", "E001");
}

#[test]
fn test_fallible_user_fn_chain() {
    assert_eq!(run_example("test_fallible_user_fn_chain.ac"), "42\n-1\n");
}

#[test]
fn test_error_e001_user_fn_fallible() {
    assert_compile_error_code("test_error_e001_user_fn.ac", "E001");
}

#[test]
fn test_bootstrap_lexer_keywords() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("bootstrap/lexer.ac");
    assert_eq!(
        run_action_file(&path),
        "fun\nmain\n(\n)\n{\nval\nx\n=\n1\nvar\ny\n=\n2\n}\n"
    );
}

// ============================================================

#[test]
fn test_error_e010_null() {
    assert_compile_error_code("test_error_e010_null.ac", "E010");
}

#[test]
fn test_error_e011_nullable_type() {
    assert_compile_error_code("test_error_e011_nullable_type.ac", "E011");
}

// Compile-error tests — imports, generics, arity
// ============================================================

#[test]
fn test_error_e006_list_index() {
    assert_compile_error_code("test_error_e006_list_index.ac", "E006");
}

#[test]
fn test_error_e006_list_var_index() {
    assert_compile_error_code("test_error_e006_list_var_index.ac", "E006");
}

#[test]
fn test_error_e007_or_unnecessary() {
    assert_compile_error_code("test_error_e007_or_unnecessary.ac", "E007");
}

#[test]
fn test_error_e008_map_index() {
    assert_compile_error_code("test_error_e008_map_index.ac", "E008");
}

#[test]
fn test_error_e008_map_var_key() {
    assert_compile_error_code("test_error_e008_map_var_key.ac", "E008");
}

#[test]
fn test_error_e009_set_index() {
    assert_compile_error_code("test_error_e009_set_index.ac", "E009");
}

#[test]
fn test_error_e001_fallible_needs_or() {
    assert_compile_error_code("test_error_e001_parseInt.ac", "E001");
}

#[test]
fn test_error_e001_toFloat() {
    assert_compile_error_code("test_error_e001_toFloat.ac", "E001");
}

#[test]
fn test_error_e001_toChar() {
    assert_compile_error_code("test_error_e001_toChar.ac", "E001");
}

#[test]
fn test_error_e001_lazyHead() {
    assert_compile_error_code("test_error_e001_lazyHead.ac", "E001");
}

#[test]
fn test_error_e001_withTimeout() {
    assert_compile_error_code("test_error_e001_withTimeout.ac", "E001");
}

#[test]
fn test_fallible_reduce_or() {
    assert_eq!(run_example("test_fallible_reduce_or.ac"), "6\n-1\n");
}

#[test]
fn test_error_e001_reduce() {
    assert_compile_error_code("test_error_e001_reduce.ac", "E001");
}

#[test]
fn test_error_e002_or_type_mismatch() {
    assert_compile_error_code("test_error_e002_or_type.ac", "E002");
}

#[test]
fn test_error_import_not_found() {
    assert_compile_error(
        "test_error_import_not_found.ac",
        "Module 'no_such_module_xyz' not found",
    );
}

#[test]
fn test_error_generic_mismatch() {
    assert_compile_error(
        "test_error_generic_mismatch.ac",
        "Cannot infer type arguments for 'first'",
    );
}

#[test]
fn test_error_generic_type_mismatch() {
    assert_compile_error(
        "test_error_generic_type_mismatch.ac",
        "Cannot infer type arguments for 'pair'",
    );
}

#[test]
fn test_error_overload_no_match() {
    assert_compile_error("test_error_overload_no_match.ac", "No matching overload");
}

#[test]
fn test_error_import_cycle() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/import_cycle/main.ac");
    assert_compile_error_at(&p, "Circular import detected");
}

#[test]
fn test_error_import_invalid_module_name() {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/import_invalid_name/main.ac");
    assert_compile_error_at(&p, "Unexpected token");
}

#[test]
fn test_error_arg_count() {
    assert_compile_error("test_error_arg_count.ac", "expects 2 arguments, but got 1");
}

// ============================================================
// Compile-error tests — fallible / removed nullable syntax
// ============================================================

#[test]
fn test_error_standalone_question() {
    assert_compile_error_code("test_error_standalone_question.ac", "E012");
}

#[test]
fn test_error_safe_call_no_field() {
    assert_compile_error_code("test_error_safe_call_no_field.ac", "E012");
}

// --- Generics tests ---

#[test]
fn test_generic_fun() {
    let out = run_example("generic_fun.ac");
    assert!(out.contains("identity(42): 42"));
    assert!(out.contains("pickFirst(10, 20): 10"));
    assert!(out.contains("pickSecond(99, 77): 77"));
    assert!(out.contains("generic functions work!"));
}

#[test]
fn test_generic_identity() {
    let out = run_example("generic_identity.ac");
    assert!(out.contains("identity(Int): 42"));
    assert!(out.contains("identity(Bool): true"));
    assert!(out.contains("identity(String): hello"));
    assert!(out.contains("identity(Float): 3.14"));
    assert!(out.contains("all types work!"));
}

#[test]
fn test_generic_pair() {
    let out = run_example("generic_pair.ac");
    assert!(out.contains("pickFirst(10, \"world\"): 10"));
    assert!(out.contains("pickSecond(10, \"world\"): world"));
    assert!(out.contains("pickFirst(false, 99): false"));
    assert!(out.contains("pickSecond(false, 99): 99"));
    assert!(out.contains("pickFirst(\"a\", \"b\"): a"));
    assert!(out.contains("mixed types work!"));
}

#[test]
fn test_generic_enum() {
    let out = run_example("generic_enum.ac");
    assert!(out.contains("unwrapOr(Some(42), 0): 42"));
    assert!(out.contains("unwrapOr(None, 99): 99"));
    assert!(out.contains("isSome(Some(42)): true"));
    assert!(out.contains("isSome(None): false"));
    assert!(out.contains("generic enum functions work!"));
}

#[test]
fn test_rc_cycle() {
    let out = run_example("rc_cycle_test.ac");
    assert!(out.contains("RC cycle test completed"));
}

#[test]
fn test_rc_pressure() {
    let out = run_example("rc_pressure_test.ac");
    assert!(out.contains("All RC pressure tests passed"));
}

#[test]
fn test_string_ops() {
    let out = run_example("string_ops.ac");
    assert!(out.contains("split count: 3"));
    assert!(out.contains("joined: a-b-c"));
    assert!(out.contains("replaced: hi world hi"));
    assert!(out.contains("copied: abc"));
    assert!(out.contains("words count: 2 first: hello"));
    assert!(out.contains("done"));
}

#[test]
fn test_list_index_assign() {
    assert_eq!(run_example("test_list_index_assign.ac"), "10\n42\n30\n");
}

#[test]
fn test_list_insert_remove() {
    let out = run_example("list_insert_remove.ac");
    assert!(out.contains("All List insert/remove tests passed"));
}

#[test]
fn test_list_stress() {
    let out = run_example("stress_list.ac");
    assert!(out.contains("All stress tests passed"));
}

#[test]
fn test_list_concat() {
    let out = run_example("list_concat_test.ac");
    assert!(out.contains("All concat tests passed"));
}

#[test]
fn test_ufcs() {
    let out = run_example("ufcs_test.ac");
    assert!(out.contains("r1: 10"));
    assert!(out.contains("r2: 8"));
    assert!(out.contains("r3: 11"));
    assert!(out.contains("r4: 39"));
}

#[test]
fn test_tuple_pattern() {
    let out = run_example("tuple_pattern_test.ac");
    assert!(out.contains("1"));
    assert!(out.contains("12"));
    assert!(out.contains("ab"));
    assert!(out.contains("first"));
}

#[test]
fn test_method_chain_remove_len() {
    assert_eq!(run_example("test_method_chain_remove_len.ac"), "2\n2\n2\n");
}

#[test]
fn test_bench_cow() {
    assert_eq!(run_example("bench_cow.ac"), "11\n");
}

#[test]
fn test_bench_all() {
    assert_eq!(
        run_example("bench_all.ac"),
        "2000\n2000\n1000\n2100\n2000\n1000\ntrue\nfalse\n"
    );
}

#[test]
fn test_ffi_cstring_roundtrip() {
    assert_eq!(run_example("test_ffi.ac"), "Hello from Atomic FFI!\ndone\n");
}

#[test]
fn test_bench_set() {
    assert_eq!(run_example("bench_set.ac"), "500\n500\n600\n0\n");
}

#[test]
fn test_bench_map() {
    assert_eq!(run_example("bench_map.ac"), "500\n500\n500\n500\n");
}

#[test]
fn test_bench_math() {
    assert_eq!(
        run_example("bench_math.ac"),
        "31259995\n295725\n7442.17\n200\n0.363123\n"
    );
}

#[test]
fn test_bench_string() {
    assert_eq!(
        run_example("bench_string.ac"),
        "2500\n2500\n2500\n2500\n2500\n100\n0\n"
    );
}

#[test]
fn test_cow_semantics() {
    let out = run_example("cow_test.ac");
    assert!(out.contains("List CoW a len (expect 3): 3"));
    assert!(out.contains("Map CoW m1 len (expect 2): 2"));
    assert!(out.contains("All CoW tests passed"));
}

// ---- Deep concat tree benchmark ----

#[test]
fn test_bench_concat_depth() {
    let out = run_example("bench_concat_depth.ac");
    assert!(
        out.contains("1001"),
        "Expected 1001 (lst len) in output, got: {}",
        out
    );
    assert!(
        out.contains("500500"),
        "Expected 500500 (sum) in output, got: {}",
        out
    );
    assert!(
        out.contains("30300"),
        "Expected 30300 (nested chain fold) in output, got: {}",
        out
    );
}

// ---- CoW property tests ----

#[test]
fn test_cow_properties() {
    // CoW property tests should end with 999 sentinel
    let out = run_example("test_cow_properties.ac");
    assert!(
        out.contains("999"),
        "Expected sentinel 999 in output, got: {}",
        out
    );
    assert!(out.contains("5"), "Expected len=5 in output");
}

#[test]
fn test_map_cow_properties() {
    let out = run_example("test_map_cow_properties.ac");
    assert_eq!(out.trim(), "2\n2\n3\n1\n2\n999");
}

#[test]
fn test_collection_stmt_mut() {
    // Stmt-form mutating UFCS (no assignment) must update var binding, preserve CoW.
    let out = run_example("test_collection_stmt_mut.ac");
    assert_eq!(out.trim(), "3\n4\n4\n2\n3\n3\n2\n777");
}

#[test]
fn test_insert_exit() {
    // Large append + alias + repeated insert; must exit cleanly (no heap corruption).
    let out = run_example("test_insert_exit.ac");
    assert_eq!(out.trim(), "2010\n2000");
}

#[test]
fn test_list_alias_append() {
    // lst + ins same scope; append on ins must not corrupt lst.
    let out = run_example("test_list_alias_append.ac");
    assert_eq!(out.trim(), "2010\n2000");
}

#[test]
fn test_list_alias_remove() {
    // lst + ins same scope; remove on ins must not corrupt lst.
    let out = run_example("test_list_alias_remove.ac");
    assert_eq!(out.trim(), "190\n200");
}

#[test]
fn test_list_alias_block() {
    // Inner for-loop scope releases ins; outer lst must stay valid.
    let out = run_example("test_list_alias_block.ac");
    assert_eq!(out.trim(), "2010\n2000");
}

#[test]
fn test_list_alias_insert() {
    // lst + ins same scope; insert on ins must not corrupt lst.
    let out = run_example("test_list_alias_insert.ac");
    assert_eq!(out.trim(), "2010\n2000");
}

#[test]
fn test_cow_insert_isolation() {
    let out = run_example("test_cow_insert_isolation.ac");
    assert_eq!(out.trim(), "10\n9999");
}

#[test]
fn test_list_cow_property() {
    let out = run_example("test_list_cow_property.ac");
    assert_eq!(
        out.trim(),
        "4\n3\n3\n10\n30\n2\n4\n3\n20\n4\n3\n20\n3\n11\n22\n33\nCoW property ok"
    );
}

// ---- UFCS chain regression test ----

#[test]
fn test_ufcs_chain() {
    // UFCS chain: lst.remove(0).len() should return 2 without SIGSEGV
    let out = run_example("test_ufcs_chain.ac");
    assert!(
        out.contains("999"),
        "Expected sentinel 999 in output, got: {}",
        out
    );
    assert!(
        out.contains("2"),
        "Expected lst.remove(0).len() == 2 in output"
    );
}

// ---- Example coverage (Phase 2) ----

#[test]
fn test_extension() {
    assert_eq!(run_example("extension.ac"), "108");
}

#[test]
fn test_type_methods() {
    assert_eq!(run_example("type_methods.ac"), "30112200");
}

#[test]
fn test_error_self_field_assign() {
    assert_compile_error(
        "test_error_self_field_assign.ac",
        "Cannot assign to fields of 'self'",
    );
}

#[test]
fn test_copy() {
    assert_eq!(run_example("copy.ac"), "421020");
}

#[test]
fn test_var_mut() {
    assert_eq!(run_example("var_mut.ac"), "10205042");
}

#[test]
fn test_non_exhaustive() {
    assert_compile_error(
        "non_exhaustive.ac",
        "Non-exhaustive when: enum 'Color' is missing variant(s): 'Blue'",
    );
}

#[test]
fn test_lazy_filter() {
    assert_eq!(
        run_example("lazy_filter_test.ac"),
        "squares: [0, 1, 4, 9, 16]\n\
         evens: [0, 2, 4, 6, 8]\n\
         mapFilter: [16, 25, 36]\n\
         filter_map: [0, 4, 16, 36]\n\
         done\n"
    );
}

#[test]
fn test_lazy_drop() {
    assert_eq!(
        run_example("lazy_drop_test.ac"),
        "drop3_take4: [3, 4, 5, 6]\n\
         drop_map_take: [10, 12, 14]\n\
         list_drop2: [0, 0, 0]\n\
         drop_all len: 1\n\
         drop0: [42, 43, 44]\n\
         done\n"
    );
}

#[test]
fn test_bench_map_10k() {
    run_example_starts_with("bench_map_10k.ac", "10000\n10000");
}

#[test]
fn test_bench_insert2() {
    assert_eq!(run_example("bench_insert2.ac"), "2002\n9999\n8888\n");
}

#[test]
fn test_bench_insert10() {
    assert_eq!(run_example("bench_insert10.ac"), "2010\n");
}

#[test]
fn test_bench_insert50() {
    assert_eq!(run_example("bench_insert50.ac"), "2050\n");
}

#[test]
fn test_bench_insert100() {
    assert_eq!(run_example("bench_insert100.ac"), "2100\n");
}

#[test]
fn test_drop_h0_mid() {
    assert_eq!(run_example("drop_h0_mid.ac"), "32\n65\n");
}

#[test]
fn test_insert_h0_mid() {
    assert_eq!(run_example("test_insert_h0_mid.ac"), "65\n999\n0\n62\n");
}

#[test]
fn test_find_named_pred() {
    assert_eq!(run_example("find_pred.ac"), "true\ntrue\n");
}

#[test]
fn test_bench_for_chain() {
    assert_eq!(run_example("bench_for_chain.ac"), "1998000\n");
}

#[test]
fn test_bench_funcall() {
    assert_eq!(
        run_example("bench_funcall.ac"),
        "832040\n2432902008176640000\n999000\n5\n"
    );
}

#[test]
fn test_lazyhead_empty() {
    assert_eq!(run_example("test_lazyhead_empty.ac"), "true\nfalse\n");
}

#[test]
fn test_bench_for_nested() {
    assert_eq!(run_example("bench_for_nested.ac"), "1500625\n");
}

#[test]
fn test_list_builtins() {
    assert_eq!(
        run_example("list_test.ac"),
        "range(1,5) len: 4\n\
head: 1\n\
last: 4\n\
get(nums,2): 3\n\
reverse len: 4\n\
take(2) len: 2\n\
drop(2) len: 2\n\
contains 3: true\n\
repeat(42,3) len: 3\n\
prepend len: 5\n\
done\n"
    );
}

#[test]
fn test_exhaustive_runtime() {
    assert_eq!(run_example("exhaustive.ac"), "1420-1");
}

#[test]
fn test_overloading2() {
    assert_eq!(
        run_example("overloading2.ac"),
        "Hello, World\n10\n30\n25\n30\n"
    );
}

#[test]
fn test_overloading() {
    assert_eq!(run_example("overloading.ac"), "3\n4\n");
}

#[test]
fn test_print_types() {
    assert_eq!(
        run_example("test_print_types.ac"),
        "Task print: Task(done=0, cancelled=0)\n\
Stream print: [1, 2]\n\
LazyList print: []\n\
Struct print: <struct>\n\
done\n"
    );
}

#[test]
fn test_lazylist_test() {
    assert_eq!(
        run_example("lazylist_test.ac"),
        "lazy_list created, len: 1\nsecond lazy_list len: 1\ndone\n"
    );
}

#[test]
fn test_math_test() {
    assert_eq!(
        run_example("math_test.ac"),
        "pi: 3.14159\n\
e: 2.71828\n\
sqrt(16): 4\n\
sin(0): 0\n\
cos(0): 1\n\
floor(3.7): 3\n\
ceil(3.2): 4\n\
round(3.5): 4\n\
abs(-5): 5\n\
min(3, 7): 3\n\
max(3.0, 7.0): 7\n\
exp(1): 2.71828\n\
log(e): 1\n\
clamp(5, 0, 10): 5\n\
clamp(-3, 0, 10): 0\n\
done\n"
    );
}

#[test]
fn test_stream_test() {
    assert_eq!(run_example("stream_test.ac"), "42\n100\n999\n");
}

#[test]
fn test_contains_set_test() {
    assert_eq!(
        run_example("contains_set_test.ac"),
        "contains a: true\ncontains d: false\ndone\n"
    );
}

#[test]
fn test_bench_for_method() {
    assert_eq!(run_example("bench_for_method.ac"), "1666\n1002\n");
}

#[test]
fn test_bench_set_10k() {
    assert_eq!(run_example("bench_set_10k.ac"), "10000\n10000\n5000\n");
}

// ---- Example coverage (stdlib / IO / builtins) ----

#[test]
fn test_integration_stdlib() {
    assert_eq!(
        run_example("integration_test.ac"),
        "845299411003false456332true"
    );
}

#[test]
fn test_io_builtins() {
    assert_eq!(
        run_example("io_test.ac"),
        "exists self: true\n\
         exists nofile: false\n\
         delete nofile: false\n\
         append: true\n\
         appended exists: true\n\
         write slice: true\n\
         read slice: slice\n\
         exists path slice: true\n\
         read path slice: slice\n\
         parseInt slice: 42\n\
         stream line1: line-a\n\
         stream line2: line-b\n\
         stream eof: EOF\n\
         close stream: true\n\
         delete test: true\n\
         delete write: true\n\
         delete stream: true\n\
         exists after delete: false\n\
         done\n"
    );
}

#[test]
fn test_new_list_builtins() {
    assert_eq!(
        run_example("new_list_builtins_test.ac"),
        "flatten len: 3\n\
         splitAt parts len: 2\n\
         chunks len: 2\n\
         windows len: 2\n\
         sorted len: 5\n\
         unique len: 3\n\
         abs(-3.5): 3.5\n\
         pow(2.0, 3.0): 8\n\
         identity: 42\n\
         shuffled len: 3\n\
         withIndex len: 3\n\
         slice len: 3\n\
         done\n"
    );
}

#[test]
fn test_map_set_builtins() {
    assert_eq!(
        run_example("map_set_builtins_test.ac"),
        "keys len: 3\n\
         values len: 3\n\
         entries len: 3\n\
         union len: 4\n\
         intersection len: 2\n\
         difference len: 1\n\
         subset true: true\n\
         subset false: false\n\
         done\n"
    );
}

#[test]
fn test_range_api() {
    assert_eq!(
        run_example("range_api.ac"),
        "0..10.contains(5): true\n\
         0..10.contains(15): false\n\
         0..10.toList().len: 11\n\
         Range API works\n"
    );
}

#[test]
fn test_datetime() {
    assert_eq!(
        run_example("datetime_test.ac"),
        "date year: 2026 month: 6\n\
         datetime hour: 12\n\
         random seed: 42\n\
         rand: 51\n\
         done\n"
    );
}

#[test]
fn test_assert() {
    assert_eq!(
        run_example("assert_test.ac"),
        "Int: 42\n\
         Float: 3.14\n\
         true: true\n\
         false: false\n\
         str: hello\n\
         after_assert\n\
         done\n"
    );
}

// ---- Complex regression (CoW, Map cascade, UFCS chains, fused iter, concat) ----

#[test]
fn test_complex_cow_persist() {
    assert_eq!(run_example("complex_cow_persist.ac"), "551299\n");
}

#[test]
fn test_complex_map_cascade() {
    assert_eq!(run_example("complex_map_cascade.ac"), "134144\n");
}

#[test]
fn test_complex_list_ufcs_chain() {
    assert_eq!(run_example("complex_list_ufcs_chain.ac"), "43302\n");
}

#[test]
fn test_complex_filter_map_fold() {
    assert_eq!(run_example("complex_filter_map_fold.ac"), "270270\n");
}

#[test]
fn test_complex_concat_mutate() {
    assert_eq!(run_example("complex_concat_mutate.ac"), "6465101999\n");
}
