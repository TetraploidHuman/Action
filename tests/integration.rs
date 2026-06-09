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
    assert_eq!(run_example("safe_access.at"), "104210");
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
fn test_when_no_else_match() {
    // value-match when without else branch — only matching arm executes
    assert_eq!(run_example("test_when_no_else_match.at"), "23");
}

#[test]
fn test_when_no_else_chain() {
    // condition-chain when without else branch — only matching arm executes
    assert_eq!(run_example("test_when_no_else_chain.at"), "23");
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
    assert_eq!(
        run_example("test_compare_chain.at"),
        "truetruetruetruetruetruetruefalse00420770"
    );
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
    assert_eq!(
        run_example("test_arithmetic_complex.at"),
        "14203264645-71615251410"
    );
}

#[test]
fn test_struct_nested() {
    // Nested struct: struct fields inside struct, two-level field access
    assert_eq!(run_example("test_struct_nested.at"), "1020100200");
}

#[test]
fn test_lambda_capture() {
    assert_eq!(run_example("test_lambda_capture.at"), "42");
}

#[test]
fn test_map_option() {
    assert_eq!(run_example("test_map_option.at"), "42");
}

#[test]
fn test_read_line() {
    // No stdin input: readLine returns None, prints "EOF"
    assert_eq!(run_example("test_read_line.at"), "EOF");
}

#[test]
fn test_read_line_with_input() {
    // Pipe input to stdin: readLine should return the input string
    let example = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("test_read_line_with_input.at");
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
        .join("test_read_line_multi.at");
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
    // readLine with EOF -> unwrapOr default "World"
    assert_eq!(run_example("io.at"), "Hello, World\n");
}

// ---- Edge-case tests: pattern matching ----

#[test]
fn test_and_guards() {
    assert_eq!(run_example("test_and_guards.at"), "positive\ndone\n");
}

#[test]
fn test_or_patterns() {
    assert_eq!(run_example("test_or_patterns.at"), "small\ndone\n");
}

// ---- Edge-case tests: data structures ----

#[test]
fn test_named_tuple() {
    assert_eq!(
        run_example("test_named_tuple.at"),
        "name: Alice\nage: 30\npos0: Alice\ndone\n"
    );
}

#[test]
fn test_struct_destructure() {
    assert_eq!(
        run_example("test_struct_destructure.at"),
        "x: 10\ny: 20\ndone\n"
    );
}

#[test]
fn test_map_set_ops() {
    assert_eq!(run_example("test_map_set.at"), "true100999true");
}

#[test]
fn test_empty_collections() {
    assert_eq!(run_example("test_empty_collections.at"), "0true0true00true");
}

// ---- Edge-case tests: functions ----

#[test]
fn test_tco_deep() {
    // Deep recursion that would overflow without TCO (n=5000)
    assert_eq!(run_example("test_tco.at"), "12036288005005000");
}

#[test]
fn test_overload_str() {
    assert_eq!(run_example("test_overload_str.at"), "Hello, World\n42\n");
}

// ---- Edge-case tests: callbacks and closures ----

#[test]
fn test_pat_cb() {
    // Pattern binding + callback in same function
    assert_eq!(run_example("test_pat_cb.at"), "42");
}

#[test]
fn test_simple_cb() {
    // Untyped callback parameter
    assert_eq!(run_example("test_simple_cb.at"), "15");
}

#[test]
fn test_cb4() {
    // Callback returning Int
    assert_eq!(run_example("test_cb4.at"), "42");
}

#[test]
fn test_cb2() {
    // Callback returning Option, called via typed function
    assert_eq!(run_example("test_cb2.at"), "4210");
}

#[test]
fn test_cb5() {
    // Callback returning Option (simpler variant)
    assert_eq!(run_example("test_cb5.at"), "42");
}

#[test]
fn test_multi_capture() {
    // Multiple closures capturing the same variable
    assert_eq!(run_example("test_nested_closure.at"), "4284");
}

#[test]
fn test_closure_loop() {
    // Closures in for loops capturing loop variable
    assert_eq!(run_example("test_closure_loop.at"), "15");
}

// ---- Edge-case tests: float and string ----

#[test]
fn test_float_edge() {
    // Float arithmetic edge cases: decimals, negatives, fractions
    assert_eq!(run_example("test_float_edge.at"), "truetruetruetruetrue");
}

#[test]
fn test_string_edge() {
    // String manipulation edge cases
    assert_eq!(run_example("test_string_edge.at"), "Hello Worldbcd0312");
}

// ---- Edge-case tests: stream, coroutine, task ----

#[test]
fn test_stream_ops() {
    assert_eq!(run_example("test_stream.at"), "4299done");
}

#[test]
fn test_coroutine() {
    assert_eq!(run_example("test_coroutine.at"), "322");
}

#[test]
fn test_task_stream() {
    assert_eq!(
        run_example("test_task_stream.at"),
        "4299falsefalse1237falsefalse456"
    );
}

// ---- Edge-case tests: imports ----

#[test]
fn test_import_selective() {
    assert_eq!(
        run_example("test_import_selective.at"),
        "15\n5\n3.14159\ndone\n"
    );
}

#[test]
fn test_import_wildcard() {
    assert_eq!(
        run_example("test_import_wildcard.at"),
        "15\n5\n3.14159\ndone\n"
    );
}

// ---- Comprehensive builtin tests ----

#[test]
fn test_option_returns() {
    // Nullable types: tail, init, indexOf, toInt, toFloat, parseInt return T?
    assert_eq!(
        run_example("test_option_returns.at"),
        "tail([1,2,3]) != null: true\n\
         tail([]) == null: true\n\
         init([1,2,3]) != null: true\n\
         init([]) == null: true\n\
         indexOf(2, [1,2,3]) != null: true\n\
         indexOf(2, [1,2,3]) value: 1\n\
         indexOf(99, [1,2,3]) == null: true\n\
         indexOf('bc', 'abcde') != null: true\n\
         indexOf('bc', 'abcde') value: 1\n\
         indexOf('xyz', 'abcde') == null: true\n\
         toInt('42') != null: true\n\
         toInt('42') value: 42\n\
         toInt('abc') == null: true\n\
         toInt(3.14) != null: true\n\
         toFloat('3.14') != null: true\n\
         toFloat('abc') == null: true\n\
         toFloat(42) != null: true\n\
         parseInt('123') != null: true\n\
         parseInt('123') value: 123\n\
         parseInt('not_a_number') == null: true\n\
         slice('hello world', 0, 5): hello\n\
         fromList([1,2,3]) contains 2: true\n\
         containsKey(m, 'a'): true\n\
         containsKey(m, 'c'): false\n\
         done\n"
    );
}

#[test]
fn test_new_features() {
    // Nullable type features: Elvis, null checks, when, LazyList, curry
    assert_eq!(
        run_example("test_new_features.at"),
        "s != null: true\n\
         s == null: false\n\
         n == null: true\n\
         s ?: 0: 42\n\
         n ?: 0: 0\n\
         s ?: -1 (unwrap): 42\n\
         s != null check: true\n\
         n == null check: true\n\
         s ?: 99: 42\n\
         n ?: 99: 99\n\
         ok != null: true\n\
         err == null: true\n\
         ok ?: 0: 10\n\
         err ?: 0: 0\n\
         ok ?: -1 (unwrap): 10\n\
         n == null via when: true\n\
         s != null via when: true\n\
         toLazyList + len: 3\n\
         toList back + len: 3\n\
         lazyHead of non-empty != null: true\n\
         lazyHead of empty == null: true\n\
         lazyTake(2) len: 2\n\
         lazyDrop(1) len: 2\n\
         curry(add,5)(10): 15\n\
         done\n"
    );
}

#[test]
fn test_network_ping() {
    // Verify action_test_ping() FFI returns 42
    assert_eq!(run_example("test_network_ping.at"), "42\n");
}

#[test]
fn test_http_error() {
    // Request to a port where nothing is listening — should return error status "0"
    run_example_starts_with("test_http_error.at", "0\n");
}

// ---- JSON tests ----

#[test]
fn test_json() {
    // Minimal test: just verify action_json_parse runs without crashing
    assert_eq!(run_example("test_json.at"), "42\n");
}

#[test]
fn test_json_error() {
    // action_json_parse on invalid JSON returns null, action_json_type(null) returns -1
    assert_eq!(run_example("test_json_error.at"), "-1\n");
}

#[test]
fn test_smart_cast() {
    assert_eq!(run_example("test_smart_cast.at"), "43920");
}

#[test]
fn test_smart_cast_if() {
    assert_eq!(run_example("test_smart_cast_if.at"), "43100");
}

#[test]
fn test_nullable_comprehensive() {
    assert_eq!(
        run_example("test_nullable_comprehensive.at"),
        "100425130-199773355"
    );
}

// ---- Comprehensive nullable type system tests ----

#[test]
fn test_nullable_complex_smart_cast() {
    // Nested when smart cast, function param smart cast, multi-variable smart cast,
    // null comparison (null == null), nested null checks
    assert_eq!(
        run_example("test_nullable_complex_smart_cast.at"),
        "112699201015427"
    );
}

#[test]
fn test_nullable_pattern_edges() {
    // Null pattern with var binding, null with else, != null OneLine,
    // nested null checks, Elvis + value match, Bool flag smart cast
    assert_eq!(
        run_example("test_nullable_pattern_edges.at"),
        "429901515true\n20"
    );
}

#[test]
fn test_nullable_elvis_chain() {
    // Elvis with expressions, arithmetic with Elvis, nested Elvis via intermediates,
    // Elvis on non-null, Elvis in comparisons, multiple defaults
    assert_eq!(
        run_example("test_nullable_elvis_chain.at"),
        "5210101577false\n6504230"
    );
}

#[test]
fn test_nullable_nested() {
    // Nullable from map operations, nullable from conditional assignment,
    // Elvis with arithmetic expressions
    assert_eq!(
        run_example("test_nullable_nested.at"),
        "10\n0\n3\n42\n-1\n100"
    );
}

#[test]
fn test_nullable_data_structures() {
    // Nullable count, map lookups with Elvis, multiple nullable values, sum with Elvis
    assert_eq!(
        run_example("test_nullable_data_structures.at"),
        "531991000101"
    );
}

#[test]
fn test_nullable_propagation() {
    // Smart cast non-null/null, Elvis defaults, arithmetic with Elvis, combined Elvis sum
    assert_eq!(
        run_example("test_nullable_propagation.at"),
        "15411001540309942117"
    );
}

#[test]
fn test_nullable_bugfixes() {
    // Bug #4: function returning nullable; Bug #5: when with null in else branch
    assert_eq!(run_example("test_bugfixes.at"), "100\n-1\n42\n-1\n30");
}

#[test]
fn test_nullable_chained_elvis() {
    // Chained Elvis operator: (a ?: b) ?: c
    assert_eq!(run_example("test_bug2_chained_elvis.at"), "5\n10\n");
}

#[test]
fn test_lazyhead_empty() {
    // lazyHead on empty LazyList returns null
    assert_eq!(run_example("test_lazyhead_empty.at"), "true\nfalse\n");
}

#[test]
fn test_struct_nullable() {
    // Struct with nullable field, Elvis extraction
    assert_eq!(run_example("test_struct_nullable.at"), "10-12025");
}

#[test]
fn test_nullable_method_call() {
    // Auto short-circuit for method calls on nullable receivers:
    // Map keys/contains, List head/len, String len, LazyList head,
    // function-returned nullable receivers, chained calls
    assert_eq!(
        run_example("test_nullable_method_call.at"),
        "5HELLO-1NULLSTR103-1-110truefalsefalsetrue"
    );
}

// ============================================================
// Compile-error tests — nullable type system rejects bad code
// ============================================================

#[test]
fn test_error_nested_nullable() {
    assert_compile_error(
        "test_error_nested_nullable.at",
        "Nested nullable types (T??) are not allowed",
    );
}

#[test]
fn test_error_arithmetic_nullable() {
    assert_compile_error("test_error_arithmetic_nullable.at", "Cannot add these types");
}

#[test]
fn test_error_nullable_to_nonnullable() {
    assert_compile_error(
        "test_error_nullable_to_nonnullable.at",
        "cannot use nullable",
    );
}

#[test]
fn test_error_standalone_question() {
    assert_compile_error("test_error_standalone_question.at", "Unexpected '?'");
}

#[test]
fn test_error_null_arg_nonnull_param() {
    assert_compile_error(
        "test_error_null_arg_nonnull_param.at",
        "Call parameter type does not match",
    );
}

#[test]
fn test_error_return_null_nonnull() {
    assert_compile_error(
        "test_error_return_null_nonnull.at",
        "Function return type does not match",
    );
}

#[test]
fn test_error_safe_call_no_field() {
    assert_compile_error(
        "test_error_safe_call_no_field.at",
        "Expected field name after '?.'",
    );
}
