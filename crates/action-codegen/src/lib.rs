// Atomic CodeGen — LLVM IR code generation
//
// Submodules: scope, typed_value, loop_control, nullable_state, mono_cache, type_layout,
// compile/hir_compile, expr/ stmt/ pattern/ for_loop/, mono/, builtins/* (iter/ tree),
// runtime_decl/* (list/core/*.inc.rs → body.inc.rs via build.rs + concat_list_body.py).

use action_frontend::ast::*;
use action_frontend::typecheck::TypeRegistry;
use inkwell::builder::BuilderError;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::StructType;
use inkwell::values::PointerValue;
use std::collections::{HashMap, HashSet};

mod loop_control;
mod mono_cache;
mod nullable_state;
mod type_layout;

mod scope;
mod typed_value;

pub(crate) use loop_control::LoopControl;
pub(crate) use mono_cache::MonoCache;
pub(crate) use nullable_state::NullableState;
pub(crate) use type_layout::TypeLayoutCache;

pub(crate) use scope::{Scope, ScopeVar, ValKind};
pub(crate) use typed_value::{InnerType, TypedValue};

pub(crate) fn llvm_err(e: BuilderError) -> String {
    format!("LLVM: {:?}", e)
}

// ---- CodeGen ----
pub struct CodeGen<'ctx> {
    pub(crate) context: &'ctx Context,
    pub(crate) module: Module<'ctx>,
    pub(crate) builder: inkwell::builder::Builder<'ctx>,
    pub(crate) scope: Scope<'ctx>,
    pub(crate) string_type: StructType<'ctx>,
    pub(crate) list_type: StructType<'ctx>,
    /// Block-based B-tree leaf node: {i32 count, i32 pad, [B x {i64, ptr}] elements}
    pub(crate) leaf_type: StructType<'ctx>,
    /// Block-based B-tree internal node: {i32 count, i32 pad, i64 total, [B x {ptr, i64}] children}
    pub(crate) internal_type: StructType<'ctx>,
    /// Child entry in internal node: {ptr child, i64 subtree_total}
    pub(crate) child_entry_type: StructType<'ctx>,
    pub(crate) lambda_count: usize,
    pub(crate) str_pat_counter: usize,
    pub(crate) registry: TypeRegistry,
    pub(crate) type_layout: TypeLayoutCache<'ctx>,
    /// Reused scratch slot for map/set insert results (one alloca per function).
    pub(crate) ht_result_scratch: Option<inkwell::values::PointerValue<'ctx>>,
    pub(crate) loop_control: LoopControl<'ctx>,
    /// Extension method mapping: "TypeName.method" → "TypeName_method"
    pub(crate) extension_methods: HashMap<String, String>,
    /// TCO (Tail Call Optimization) state for the current function.
    pub(crate) tco_state: Option<TcoState<'ctx>>,
    /// Coroutine: list alloca where launch results are collected inside coroutineScope.
    /// None means we are not inside a coroutineScope.
    pub(crate) coroutine_collector: Option<inkwell::values::PointerValue<'ctx>>,
    /// Task type: {pthread: i64, done: i64, cancelled: i64, result_list: {ptr, i64, i64}}
    pub(crate) task_type: StructType<'ctx>,
    /// LazyList type: {head_val: i64, step_fn: i8*, state: i64, take_count: i64, map_fn: i8*, filter_fn: i8*}
    /// take_count = -1 means infinite (or no step fn), >=0 means take that many
    /// map_fn is an optional transformer applied during to_list evaluation
    /// filter_fn is an optional predicate; elements failing the predicate are skipped
    pub(crate) lazylist_type: StructType<'ctx>,
    /// Range type: {start: i64, end: i64, inclusive: i64}
    pub(crate) range_type: StructType<'ctx>,
    /// Stream type: {mutex: [40 x i8], list: {ptr, i64, i64}} (mutex-protected buffer)
    pub(crate) stream_type: StructType<'ctx>,
    /// Fat return type: named {i64, ptr} struct distinct from enum types.
    /// Used for untyped function/lambda returns. When packed with a scalar,
    /// field 0 holds the value and field 1 is null. When wrapping an enum,
    /// field 0 is the tag and field 1 is the data pointer.
    pub(crate) fat_return_type: StructType<'ctx>,
    /// Last fat_ret alloca from unpack_fat_return/bv_to_typed, for potential
    /// bitcast when the result is returned from a typed function (e.g., enum).
    pub(crate) last_fat_ret: Option<(PointerValue<'ctx>, StructType<'ctx>)>,
    /// Last-known enum inner type for bv_to_typed to preserve through struct→Enum conversion.
    pub(crate) last_enum_inner: Option<(InnerType, bool)>,
    /// Overloaded function mapping: base name → [(param_types, mangled_name)]
    /// e.g., "add" → [([Int, Int], "add_Int_Int"), ([Float, Float], "add_Float_Float")]
    pub(crate) overloaded_functions: HashMap<String, Vec<(Vec<Type>, String)>>,
    /// Whether we are currently compiling inside an `unsafe { }` block
    pub(crate) in_unsafe: bool,
    /// Fallibility context copied from CheckedProgram (R7).
    pub(crate) fallibility: action_frontend::typecheck::FallibilityContext,
    /// Depth of nested `or { }` blocks during codegen.
    pub(crate) or_block_depth: usize,
    /// Stack of fail basic blocks for fallible regions (or-block / fn or).
    pub(crate) fallible_fail_stack: Vec<inkwell::basic_block::BasicBlock<'ctx>>,
    /// When set, `compile_return_value` wraps returns as `{payload, i1 ok}`.
    pub(crate) propagating_fallible_ret: Option<action_frontend::ast::Type>,
    /// Builtin wrappers needed for :: function references (e.g., List::head)
    pub(crate) builtin_wrappers_needed: HashSet<String>,
    /// LLVM optimization level (0-3)
    pub(crate) opt_level: u8,
    /// Target triple for cross-compilation (None = native)
    pub(crate) target_triple: Option<String>,
    /// Counter for unique wrapper function names (lazy_map, lazy_filter, etc.)
    pub(crate) wrapper_counter: u64,
    pub(crate) nullable_state: NullableState<'ctx>,
    pub(crate) mono_cache: MonoCache,
    /// Tracks whether compile_block did an rc_inc on the last expression.
    /// val stmt uses this to apply a balancing rc_dec.
    pub(crate) block_did_rc_inc: bool,
}

pub(crate) struct TcoState<'ctx> {
    /// Target block to jump to for tail-recursive calls
    tail_entry: inkwell::basic_block::BasicBlock<'ctx>,
    /// Parameter allocas: (alloca, type, valkind)
    param_slots: Vec<(
        inkwell::values::PointerValue<'ctx>,
        inkwell::types::BasicTypeEnum<'ctx>,
        ValKind,
    )>,
    /// Original AST function name (unmangled) for self-recognition in TCO
    fn_name: String,
}

impl<'ctx> CodeGen<'ctx> {
    pub fn new(
        context: &'ctx Context,
        name: &str,
        registry: TypeRegistry,
        target_triple: Option<String>,
    ) -> Self {
        let module = context.create_module(name);
        let builder = context.create_builder();
        // Named type to distinguish from anonymous {i64, i8*} enum types
        let string_type = context.opaque_struct_type("__action_str");
        string_type.set_body(
            &[
                context.i64_type().into(),
                context.ptr_type(inkwell::AddressSpace::default()).into(),
            ],
            false,
        );
        let list_type = context.struct_type(
            &[
                context.ptr_type(inkwell::AddressSpace::default()).into(), // data ptr
                context.i64_type().into(),                                 // length
                context.i64_type().into(),                                 // capacity
            ],
            false,
        );
        // Block-based B-tree node types for persistent List
        // B = 64: leaf holds up to 64 elements, internal holds up to 64 children
        const B: usize = 64;
        // Child entry: {ptr child, i64 subtree_total}
        let child_entry_type = context.struct_type(
            &[
                context.ptr_type(inkwell::AddressSpace::default()).into(),
                context.i64_type().into(),
            ],
            false,
        );
        // Leaf: {i32 count, i32 pad, [B x {i64, ptr}] elements}
        let leaf_type = context.struct_type(
            &[
                context.i32_type().into(),               // count (0..B)
                context.i32_type().into(),               // padding (align to 8)
                string_type.array_type(B as u32).into(), // elements array
            ],
            false,
        );
        // Internal: {i32 count, i32 pad, i64 total, [B x {ptr, i64}] children}
        let internal_type = context.struct_type(
            &[
                context.i32_type().into(),                    // count (0..B)
                context.i32_type().into(),                    // padding (align to 8)
                context.i64_type().into(),                    // total elements in subtree
                child_entry_type.array_type(B as u32).into(), // children array
            ],
            false,
        );
        // Task type: {pthread: i64, done: i64, cancelled: i64, scheduler: i64, result_list: {ptr, i64, i64}}
        let task_type = context.struct_type(
            &[
                context.i64_type().into(), // pthread_t (opaque thread handle)
                context.i64_type().into(), // done flag (0=not done, 1=done)
                context.i64_type().into(), // cancelled flag (0=not cancelled, 1=cancelled)
                context.i64_type().into(), // scheduler (0=default, 1=io, 2=cpu)
                list_type.into(),          // result list
            ],
            false,
        );
        // LazyList type: {head_val: i64, step_fn: i8*, state: i64, take_count: i64, map_fn: i8*, filter_fn: i8*}
        let lazylist_type = context.struct_type(
            &[
                context.i64_type().into(), // head value (i64 for Int lazy lists)
                context.ptr_type(inkwell::AddressSpace::default()).into(), // step_fn ptr
                context.i64_type().into(), // state
                context.i64_type().into(), // take_count (-1 = infinite, >=0 = count)
                context.ptr_type(inkwell::AddressSpace::default()).into(), // map_fn ptr (null = no mapping)
                context.ptr_type(inkwell::AddressSpace::default()).into(), // filter_fn ptr (null = no filter)
            ],
            false,
        );
        // Stream type: {mutex: [40 x i8], cond: [48 x i8], closed: i64, list: {ptr, i64, i64}}
        let stream_type = context.struct_type(
            &[
                context.i8_type().array_type(40).into(), // pthread_mutex_t = 40 bytes
                context.i8_type().array_type(48).into(), // pthread_cond_t = 48 bytes
                context.i64_type().into(),               // closed flag
                list_type.into(),                        // data buffer list
            ],
            false,
        );
        // Range type: {start: i64, end: i64, inclusive: i64}
        let range_type = context.struct_type(
            &[
                context.i64_type().into(),
                context.i64_type().into(),
                context.i64_type().into(),
            ],
            false,
        );
        let fat_return_type = context.opaque_struct_type("__fat_ret");
        fat_return_type.set_body(
            &[
                context.i64_type().into(),
                context.ptr_type(inkwell::AddressSpace::default()).into(),
            ],
            false,
        );
        CodeGen {
            context,
            module,
            builder,
            scope: Scope::new(),
            string_type,
            list_type,
            leaf_type,
            internal_type,
            child_entry_type,
            lambda_count: 0,
            str_pat_counter: 0,
            registry,
            type_layout: TypeLayoutCache::default(),
            ht_result_scratch: None,
            loop_control: LoopControl::default(),
            extension_methods: HashMap::new(),
            tco_state: None,
            coroutine_collector: None,
            task_type,
            lazylist_type,
            range_type,
            stream_type,
            fat_return_type,
            last_fat_ret: None,
            last_enum_inner: None,
            overloaded_functions: HashMap::new(),
            in_unsafe: false,
            fallibility: action_frontend::typecheck::FallibilityContext::new(),
            or_block_depth: 0,
            fallible_fail_stack: Vec::new(),
            propagating_fallible_ret: None,
            builtin_wrappers_needed: HashSet::new(),
            opt_level: 0,
            target_triple,
            wrapper_counter: 0,
            nullable_state: NullableState::default(),
            mono_cache: MonoCache::default(),
            block_did_rc_inc: false,
        }
    }

    pub fn print_ir(&self) -> String {
        // On Windows with /FORCE:UNRESOLVED, print_to_string can trigger LLVM
        // analysis passes that call into NULL unresolved symbols. Write to a
        // temp file and read it back instead.
        #[cfg(target_os = "windows")]
        {
            let dir = std::env::temp_dir();
            let path = dir.join(format!("action_ir_{}.ll", std::process::id()));
            self.module
                .print_to_file(&path)
                .expect("Failed to write IR to temp file");
            let ir = std::fs::read_to_string(&path).expect("Failed to read IR temp file");
            let _ = std::fs::remove_file(&path);
            ir
        }
        #[cfg(not(target_os = "windows"))]
        {
            self.module.print_to_string().to_string()
        }
    }

    pub fn verify(&self) -> Result<(), String> {
        // Module verification can trigger analysis passes that call into
        // unresolved symbols on Windows (/FORCE:UNRESOLVED makes them NULL).
        // The IR will be verified again by clang during compilation anyway.
        #[cfg(not(target_os = "windows"))]
        {
            self.module.verify().map_err(|e| e.to_string())
        }
        #[cfg(target_os = "windows")]
        {
            let _ = self;
            Ok(())
        }
    }

    // Write LLVM bitcode to a file
}

mod call_arg;
mod call_hir;
mod compile;
mod hir_compile;
mod ufcs;
// ---- Submodules ----
mod builtin_dispatch;
mod builtins;
mod expr;
mod fallible;
mod for_loop;
mod generics;
mod gep_cursor;
mod jit;
mod map_set;
mod misc;
mod mono;
mod opt_pass;
mod pattern;
mod rc_ops;
mod runtime_decl;
mod runtime_io;
mod stmt;
mod struct_ops;
mod type_helpers;

pub(crate) use self::gep_cursor::GepCursor;

#[cfg(test)]
mod tests {
    use super::*;
    use action_frontend::checked::CheckedProgram;
    use action_frontend::lexer::Lexer;
    use action_frontend::parser::Parser;
    use action_frontend::typecheck::TypeChecker;
    use action_frontend::typecheck::TypeRegistry;
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    /// Shared LLVM context reused across all codegen tests.
    /// Creating/destroying multiple contexts crashes on MSVC (STATUS_ACCESS_VIOLATION),
    /// so we create one context and never drop it.
    static TEST_CONTEXT: OnceLock<Mutex<Context>> = OnceLock::new();

    fn compile_program(source: &str) -> String {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parsing should succeed");

        // Register types from program statements
        let mut registry = TypeRegistry::new();
        for stmt in &program.stmts {
            let _ = registry.register(stmt);
        }

        // Type check
        let mut checker = TypeChecker::new(registry.clone());
        let mut type_env = HashMap::new();
        type_env.insert("Int".to_string(), Type::Named("Int".into()));
        type_env.insert("String".to_string(), Type::Named("String".into()));
        type_env.insert("Bool".to_string(), Type::Named("Bool".into()));
        type_env.insert("Float".to_string(), Type::Named("Float".into()));
        type_env.insert("Char".to_string(), Type::Named("Char".into()));
        checker.seed_type_env(&type_env);
        let errors = checker.check(&program);
        if !errors.is_empty() {
            panic!("Type check failed: {:?}", errors);
        }

        let checked = CheckedProgram::new(program, registry.clone(), &checker);

        // Compile to LLVM IR via HIR path
        // Use a shared context to avoid STATUS_ACCESS_VIOLATION on Windows
        // when creating multiple LLVM contexts in the same process.
        let guard = TEST_CONTEXT
            .get_or_init(|| Mutex::new(Context::create()))
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let mut cg = CodeGen::new(&guard, "test", registry, None);
        cg.compile_checked(&checked)
            .expect("Compilation should succeed");
        cg.print_ir()
    }

    #[test]
    fn test_codegen_empty_program() {
        let ir = compile_program("");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(ir.contains("@main"), "IR should contain main function");
        assert!(
            ir.contains("define"),
            "IR should contain function definitions"
        );
    }

    #[test]
    fn test_codegen_val_int() {
        let ir = compile_program("val x = 42");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(ir.contains("i64 42"), "IR should contain i64 constant 42");
    }

    #[test]
    fn test_codegen_binary_add() {
        let ir = compile_program("val x = 1 + 2");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(ir.contains("add"), "IR should contain add instruction");
    }

    #[test]
    fn test_codegen_simple_fun() {
        let ir = compile_program("fun add(x: Int, y: Int) -> Int { x + y }");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(ir.contains("@add"), "IR should contain 'add' function");
    }

    #[test]
    fn test_codegen_val_bool() {
        let ir = compile_program("val x = true");
        assert!(!ir.is_empty(), "IR should not be empty");
        // Bool is represented as i64 in this compiler
        assert!(
            ir.contains("i64 1") || ir.contains("i64 true"),
            "IR should contain bool constant"
        );
    }

    #[test]
    fn test_codegen_string_constant() {
        let ir = compile_program("val x = \"hello\"");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(
            ir.contains("hello") || ir.contains("string"),
            "IR should contain string reference"
        );
    }

    #[test]
    fn test_codegen_when_match_int() {
        let ir = compile_program("val x = when 42 { 1 -> 10; 2 -> 20; else -> 0 }");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(ir.contains("@main"), "IR should contain main function");
    }

    #[test]
    fn test_codegen_lambda() {
        let ir = compile_program("val f = { x -> x * 2 }");
        assert!(!ir.is_empty(), "IR should not be empty");
        // Lambda generates a function — at minimum main should exist
        assert!(ir.contains("@main"), "IR should contain main");
    }

    #[test]
    fn test_codegen_type_annotated_variable() {
        let ir = compile_program("val x: Int = 42");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(ir.contains("i64 42"), "IR should contain i64 42");
    }

    #[test]
    fn test_codegen_multiple_statements() {
        let ir = compile_program("val a = 1\nval b = 2\nval c = a + b");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(ir.contains("@main"), "IR should contain main");
        assert!(ir.contains("add"), "IR should contain add instruction");
    }

    #[test]
    fn test_codegen_negation() {
        let ir = compile_program("val x = -42");
        assert!(!ir.is_empty(), "IR should not be empty");
        // Check for either sub instruction (0 - 42) or the i64 42 constant
        assert!(
            ir.contains("i64 42") || ir.contains("sub"),
            "IR should negate 42"
        );
    }

    #[test]
    fn test_codegen_comparison() {
        let ir = compile_program("val x = 1 < 2");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(
            ir.contains("icmp"),
            "IR should contain icmp instruction for comparison"
        );
    }

    #[test]
    fn test_codegen_string_interpolation() {
        let ir = compile_program("val name = \"world\"\nval msg = \"hello, ${name}\"");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(ir.contains("@main"), "IR should contain main function");
    }

    #[test]
    fn test_codegen_function_call() {
        let ir = compile_program("fun double(x: Int) -> Int { x * 2 }\nval y = double(21)");
        assert!(!ir.is_empty(), "IR should not be empty");
        assert!(ir.contains("@double"), "IR should contain double function");
        assert!(ir.contains("@main"), "IR should contain main function");
    }

    fn count_llvm_defines(ir: &str, fn_name: &str) -> usize {
        let needle = format!("@{fn_name}");
        ir.lines()
            .filter(|l| l.trim_start().starts_with("define") && l.contains(&needle))
            .count()
    }

    #[test]
    fn test_generic_monomorphization_instance_cache() {
        let ir = compile_program(
            "fun <T, U> pickFirst(a: T, b: U) -> T { a }\n\
             fun main() {\n\
                 val a = pickFirst(1, 2)\n\
                 val b = pickFirst(3, 4)\n\
                 val c = pickFirst(true, 5)\n\
             }",
        );
        assert_eq!(
            count_llvm_defines(&ir, "pickFirst_Int_Int"),
            1,
            "pickFirst_Int_Int should be defined once despite multiple Int/Int call sites"
        );
        assert_eq!(
            count_llvm_defines(&ir, "pickFirst_Bool_Int"),
            1,
            "pickFirst_Bool_Int should be defined once"
        );
    }

    #[test]
    fn runtime_defines_list_get() {
        let ir = compile_program("fun main() { println(1) }");
        assert!(ir.contains("declare") && ir.contains("action_list_get"));
    }

    #[test]
    fn runtime_defines_rc_ops() {
        let ir = compile_program("fun main() { println(1) }");
        assert!(ir.contains("action_rc_inc") && ir.contains("action_rc_dec"));
    }

    #[test]
    fn runtime_defines_map_insert() {
        let ir = compile_program("fun main() { println(1) }");
        assert!(ir.contains("action_map_insert"));
    }

    #[test]
    fn runtime_defines_list_concat_and_push_subtree() {
        let ir = compile_program("fun main() { println(1) }");
        assert!(ir.contains("action_list_concat") && ir.contains("action_list_push_subtree"));
    }

    #[test]
    fn runtime_defines_list_push() {
        let ir = compile_program("fun main() { println(1) }");
        assert!(
            ir.contains("define") && ir.contains("action_list_push"),
            "list push runtime must remain linked after push.inc split"
        );
    }

    #[test]
    fn codegen_map_emits_walk_or_mono() {
        let ir = compile_program(
            "fun main() {\n\
                 val xs = List[1, 2, 3]\n\
                 val ys = map(xs) { x -> x + 1 }\n\
                 println(ys.len())\n\
             }",
        );
        assert!(
            ir.contains("action_list_map_walk") || ir.contains(".mono_map"),
            "map codegen path must survive iter/mono submodule split"
        );
    }

    #[test]
    fn codegen_for_loop_emits_body() {
        let ir = compile_program(
            "fun main() {\n\
                 var sum = 0\n\
                 for x in 1..4 { sum = sum + x }\n\
                 println(sum)\n\
             }",
        );
        assert!(
            ir.contains("@main"),
            "for-loop submodule split must still compile main"
        );
        assert!(
            ir.contains("icmp") || ir.contains("br"),
            "for-loop should emit control flow"
        );
    }

    #[test]
    fn runtime_defines_ht_insert() {
        let ir = compile_program("fun main() { println(1) }");
        assert!(
            ir.contains("define") && ir.contains("action_ht_insert"),
            "hash_table submodule split must keep ht insert runtime"
        );
    }

    #[test]
    fn codegen_hir_lazy_map() {
        let ir = compile_program(
            "fun main() {\n\
                 val ll = lazy_list(0) { it + 1 }\n\
                 val mapped = lazyMap({ it * 2 }, ll)\n\
                 println(lazyHead(mapped))\n\
             }",
        );
        assert!(
            ir.contains("@main"),
            "lazy submodule split must compile lazyMap program"
        );
    }

    #[test]
    fn codegen_hir_destructure() {
        let ir = compile_program(
            "fun main() {\n\
                 val (a, b) = (1, 2)\n\
                 println(a + b)\n\
             }",
        );
        assert!(
            ir.contains("@main") && (ir.contains("extractvalue") || ir.contains("extract_value")),
            "hir_compile stmt split must compile tuple destructure"
        );
    }

    #[test]
    fn runtime_defines_str_split() {
        let ir = compile_program("fun main() { println(1) }");
        assert!(
            ir.contains("define") && ir.contains("action_string_split"),
            "str_adv submodule split must keep string split runtime"
        );
    }

    #[test]
    fn codegen_stdlib_collection_sum() {
        let ir = compile_program(
            "fun main() {\n\
                 val xs = List[1, 2, 3]\n\
                 println(sum(xs))\n\
             }",
        );
        assert!(
            ir.contains("@main"),
            "collection submodule split must compile sum()"
        );
    }

    #[test]
    fn codegen_stdlib_datetime_rand() {
        let ir = compile_program("fun main() { println(randInt(1, 10)) }");
        assert!(
            ir.contains("@main"),
            "datetime submodule split must compile randInt"
        );
    }
}
