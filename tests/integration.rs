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
    // No stdin input: read_line returns None, prints "EOF"
    assert_eq!(run_example("test_read_line.at"), "EOF");
}

#[test]
fn test_io() {
    // read_line with EOF -> unwrap_or default "World"
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
    assert_eq!(
        run_example("test_string_edge.at"),
        "Hello Worldbcd03EnumVariant<0>(1)EnumVariant<0>(2)"
    );
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
    // Comprehensive test of Option-returning builtins: tail, init, index_of,
    // to_int, to_float, parse_int, slice, from_list, contains_key
    assert_eq!(
        run_example("test_option_returns.at"),
        "tail([1,2,3]) is_some: true\n\
         tail([]) is_none: true\n\
         init([1,2,3]) is_some: true\n\
         init([]) is_none: true\n\
         index_of(2, [1,2,3]) is_some: true\n\
         index_of(2, [1,2,3]) value: 1\n\
         index_of(99, [1,2,3]) is_none: true\n\
         index_of('bc', 'abcde') is_some: true\n\
         index_of('bc', 'abcde') value: 1\n\
         index_of('xyz', 'abcde') is_none: true\n\
         to_int('42') is_some: true\n\
         to_int('42') value: 42\n\
         to_int('abc') is_none: true\n\
         to_int(3.14) is_some: true\n\
         to_float('3.14') is_some: true\n\
         to_float('abc') is_none: true\n\
         to_float(42) is_some: true\n\
         parse_int('123') is_some: true\n\
         parse_int('123') value: 123\n\
         parse_int('not_a_number') is_none: true\n\
         slice('hello world', 0, 5): hello\n\
         from_list([1,2,3]) contains 2: true\n\
         contains_key(m, 'a'): true\n\
         contains_key(m, 'c'): false\n\
         done\n"
    );
}

#[test]
fn test_new_features() {
    // Comprehensive test of v6 features: Option/Result methods, dot notation,
    // LazyList, curry, ok()
    assert_eq!(
        run_example("test_new_features.at"),
        "is_some(Some(42)): true\n\
         is_none(Some(42)): false\n\
         is_none(None): true\n\
         unwrap_or(Some(42), 0): 42\n\
         unwrap_or(None, 0): 0\n\
         unwrap(Some(42)): 42\n\
         s.is_some(): true\n\
         n.is_none(): true\n\
         s.unwrap_or(99): 42\n\
         n.unwrap_or(99): 99\n\
         is_ok(Ok(10)): true\n\
         is_err(Err(...)): true\n\
         unwrap_or(Ok(10), 0): 10\n\
         unwrap_or(Err(...), 0): 0\n\
         unwrap(Ok(10)): 10\n\
         ok(Some(42), 99) -> is_ok: true\n\
         ok(None, 99) -> is_err: true\n\
         to_lazy_list + len: 3\n\
         to_list back + len: 3\n\
         lazy_head of non-empty: true\n\
         lazy_head of empty: false\n\
         lazy_take(2) len: 2\n\
         lazy_drop(1) len: 2\n\
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
    assert_eq!(
        run_example("test_json.at"),
        "Alice\n\
         30\n\
         true\n\
         95.5\n\
         5\n\
         10\n\
         30\n\
         Bob\n\
         3\n\
         92\n\
         {\"active\":true,\"age\":30,\"name\":\"Alice\",\"score\":95.5}\n\
         5\n\
         4\n\
         3\n\
         2\n\
         true\n"
    );
}

#[test]
fn test_json_error() {
    assert_eq!(run_example("test_json_error.at"), "-1\ntrue\ntrue\n-1\n");
}
