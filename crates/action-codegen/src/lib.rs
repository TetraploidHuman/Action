// Atomic CodeGen — LLVM IR code generation
// Core types and compilation entry point. See submodules for other methods.

// Atomic CodeGen — LLVM IR code generation
//
// File structure (line ranges approximate):
//   Lines    1-11   Imports
//   Lines   12-75   Scope / ScopeVar / ValKind types
//   Lines   77-127  TypedValue type
//   Lines  129-163  CodeGen struct, TcoState
//   Lines  165-203  CodeGen::new() + type helpers (i64_ty, f64_ty, ptr_ty, etc.)
//   Lines  204-4074 define_runtime() — LLVM runtime function declarations (~3900 lines)
//   Lines 4076-4116 Runtime helpers: call_rt, load_string, load_list, etc.
//   Lines 4119-4300 Type inference: infer_hir_expr_type, build_fn_type, etc.
//   Lines 4302-4418 compile(), print_ir(), verify()
//   HIR-only: compile_hir_*, compile_checked → compile_hir (no AST compile_expr/compile_stmt)
//   Lines 5975-6395 compile_call() (continued)
//   Lines 6396-8237 Builtin functions: print, list, map, filter, fold, flat_map, etc.
//   Lines 8238-10902 builtin_stdlib() — stdlib function dispatcher (~2600 lines)
//   Lines 10903-11544 Pattern matching: compile_when, compile_pattern_match, bind_pattern_vars
//   Lines 11545-12267 For loops: compile_for, compile_for_iterate, compile_for_yield, etc.
//   Lines 12260-13308 Expressions: compile_range, compile_if, compile_block, compile_index,
//          compile_field_access, compile_struct_lit, compile_tuple, compile_map_lit, compile_set_lit,
//          compile_string_interp, compile_enum_construct
//   Lines 13058-13220 Map/Set operations: builtin_map_insert, builtin_set_contains, etc.
//   Lines 13290-13343 run_jit(), TypedValue helpers
//
// To split further: break the `impl<'ctx> CodeGen<'ctx>` block into submodules
// by closing/reopening at the boundaries marked above.

use action_frontend::ast::*;
use action_frontend::typecheck::TypeRegistry;
use inkwell::builder::BuilderError;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::PointerValue;
use std::collections::{HashMap, HashSet};

mod scope;
mod typed_value;

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
    pub(crate) named_structs: HashMap<String, StructType<'ctx>>,
    pub(crate) enum_types: HashMap<String, StructType<'ctx>>,
    pub(crate) anon_structs: HashMap<Vec<String>, StructType<'ctx>>,
    /// Compile-time constants: name → (global pointer, element type, ValKind)
    pub(crate) consts: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>, ValKind)>,
    /// Reused scratch slot for map/set insert results (one alloca per function).
    pub(crate) ht_result_scratch: Option<inkwell::values::PointerValue<'ctx>>,
    /// Target block for `continue` — set inside for loops, cleared on exit
    pub(crate) continue_target: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    /// Target block for `break` — set inside for loops, cleared on exit
    pub(crate) break_target: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    /// When compiling a sequential `for i < n` loop that indexes a List, reuse one get cache.
    pub(crate) list_loop_get_cache: Option<inkwell::values::PointerValue<'ctx>>,
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
    /// Builtin wrappers needed for :: function references (e.g., List::head)
    pub(crate) builtin_wrappers_needed: HashSet<String>,
    /// LLVM optimization level (0-3)
    pub(crate) opt_level: u8,
    /// Target triple for cross-compilation (None = native)
    pub(crate) target_triple: Option<String>,
    /// Counter for unique wrapper function names (lazy_map, lazy_filter, etc.)
    pub(crate) wrapper_counter: u64,
    /// Counter for synthetic receiver names in nullable method call short-circuit
    pub(crate) synthetic_counter: u64,
    /// Nullable type cache: type name string → {i1, T} LLVM struct type
    pub(crate) nullable_types: HashMap<String, StructType<'ctx>>,
    /// Smart cast: variables known to be non-null in current scope (from when matching)
    pub(crate) not_null_set: HashSet<String>,
    /// Generic function definitions with type_params, indexed by name.
    /// Used for monomorphization at call sites.
    pub(crate) generic_fun_defs: HashMap<String, action_frontend::hir::HirStmt>,
    /// Monomorphized LLVM function names already compiled (or in progress).
    pub(crate) monomorphized_fns: HashSet<String>,
    /// LLVM function name → AST return type (Pass 1), for call-site List/Map/Set tagging.
    pub(crate) fun_return_types: HashMap<String, Type>,
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
            named_structs: HashMap::new(),
            enum_types: HashMap::new(),
            anon_structs: HashMap::new(),
            consts: HashMap::new(),
            continue_target: None,
            break_target: None,
            ht_result_scratch: None,
            list_loop_get_cache: None,
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
            builtin_wrappers_needed: HashSet::new(),
            opt_level: 0,
            target_triple,
            wrapper_counter: 0,
            synthetic_counter: 0,
            nullable_types: HashMap::new(),
            not_null_set: HashSet::new(),
            generic_fun_defs: HashMap::new(),
            monomorphized_fns: HashSet::new(),
            fun_return_types: HashMap::new(),
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
mod for_loop;
mod generics;
mod gep_cursor;
mod jit;
mod lambda_mono;
mod map_set;
mod misc;
mod nullable;
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
}
