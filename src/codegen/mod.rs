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
//   Lines 4119-4300 Type inference: infer_return_type, infer_expr_type, build_fn_type, etc.
//   Lines 4302-4418 compile(), print_ir(), verify()
//   Lines 4423-5081 compile_stmt(), compile_fun_def(), compile_let, etc.
//   Lines 5081-5975 compile_expr(), compile_lambda(), compile_binary(), compile_unary(), compile_call()
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

use crate::ast::*;
use crate::typecheck::TypeRegistry;
use inkwell::builder::BuilderError;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{BasicTypeEnum, FunctionType, StructType};
use inkwell::values::{BasicValue, BasicValueEnum, IntValue, PointerValue};
use std::collections::{HashMap, HashSet};

// ---- Scope ----
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum ValKind {
    Int,
    Float,
    Bool,
    Str,
    Fn,
    List,
    Map,
    Set,
    Task,
    Stream,
    LazyList,
    CString,
    Ptr,
    FileHandle,
    Struct,
    Enum,
    Nullable,
    Unit,
}

#[derive(Clone)]
struct ScopeVar<'ctx> {
    ptr: PointerValue<'ctx>,
    ty: inkwell::types::BasicTypeEnum<'ctx>,
    kind: ValKind,
    fn_type: Option<FunctionType<'ctx>>,
    mutable: bool,
    /// For lazy val: pointer to i1 initialized flag
    lazy_flag: Option<PointerValue<'ctx>>,
    /// For lazy val: the initializer expression (cloned)
    lazy_init_expr: Option<Expr>,
    /// AST-level type for enum resolution (e.g., Option<Date>)
    ast_type: Option<Type>,
    /// For Enum values: the inner type (Int, Float, Str) to preserve through loads
    enum_inner_type: Option<InnerType>,
    /// For Enum values with heap-allocated data: whether the data pointer needs RC cleanup
    enum_data_rc_managed: bool,
    /// Whether this variable holds a closure (vs a bare function pointer)
    is_closure: bool,
    /// For closures: the LLVM captures struct type
    closure_ty: Option<StructType<'ctx>>,
    /// For closures: the LLVM function pointer (for reconstruction after load)
    closure_fn_ptr: Option<PointerValue<'ctx>>,
    /// For closures: the actual LLVM fn type (with captures ptr param)
    actual_fn_type: Option<FunctionType<'ctx>>,
}

#[derive(Clone)]
pub(super) struct Scope<'ctx> {
    variables: HashMap<String, ScopeVar<'ctx>>,
    parent: Option<Box<Scope<'ctx>>>,
}

impl<'ctx> Scope<'ctx> {
    fn new() -> Self {
        Scope {
            variables: HashMap::new(),
            parent: None,
        }
    }
    fn with_parent(parent: Scope<'ctx>) -> Self {
        Scope {
            variables: HashMap::new(),
            parent: Some(Box::new(parent)),
        }
    }
    fn get(&self, name: &str) -> Option<&ScopeVar<'ctx>> {
        self.variables
            .get(name)
            .or_else(|| self.parent.as_ref().and_then(|p| p.get(name)))
    }
    fn set(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: inkwell::types::BasicTypeEnum<'ctx>,
        kind: ValKind,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type: None,
                mutable: false,
                lazy_flag: None,
                lazy_init_expr: None,
                ast_type: None,
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
            },
        );
    }
    fn set_with_fn_type(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: inkwell::types::BasicTypeEnum<'ctx>,
        kind: ValKind,
        fn_type: Option<FunctionType<'ctx>>,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type,
                mutable: false,
                lazy_flag: None,
                lazy_init_expr: None,
                ast_type: None,
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
            },
        );
    }
    fn set_mutable(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: inkwell::types::BasicTypeEnum<'ctx>,
        kind: ValKind,
        fn_type: Option<FunctionType<'ctx>>,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type,
                mutable: true,
                lazy_flag: None,
                lazy_init_expr: None,
                ast_type: None,
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
            },
        );
    }
    fn set_lazy(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: inkwell::types::BasicTypeEnum<'ctx>,
        kind: ValKind,
        flag: PointerValue<'ctx>,
        init_expr: Expr,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type: None,
                mutable: false,
                lazy_flag: Some(flag),
                lazy_init_expr: Some(init_expr),
                ast_type: None,
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
            },
        );
    }
    fn set_with_ast_type(
        &mut self,
        name: String,
        ptr: PointerValue<'ctx>,
        ty: inkwell::types::BasicTypeEnum<'ctx>,
        kind: ValKind,
        fn_type: Option<FunctionType<'ctx>>,
        ast_type: Type,
    ) {
        self.variables.insert(
            name,
            ScopeVar {
                ptr,
                ty,
                kind,
                fn_type,
                mutable: false,
                lazy_flag: None,
                lazy_init_expr: None,
                ast_type: Some(ast_type),
                enum_inner_type: None,
                enum_data_rc_managed: false,
                is_closure: false,
                closure_ty: None,
                closure_fn_ptr: None,
                actual_fn_type: None,
            },
        );
    }
    fn set_closure_info(
        &mut self,
        name: &str,
        closure_ty: StructType<'ctx>,
        closure_fn_ptr: PointerValue<'ctx>,
        actual_fn_type: FunctionType<'ctx>,
    ) {
        if let Some(var) = self.variables.get_mut(name) {
            var.is_closure = true;
            var.closure_ty = Some(closure_ty);
            var.closure_fn_ptr = Some(closure_fn_ptr);
            var.actual_fn_type = Some(actual_fn_type);
        }
    }
    fn set_enum_inner_type(&mut self, name: &str, inner_type: InnerType) {
        if let Some(var) = self.variables.get_mut(name) {
            var.enum_inner_type = Some(inner_type);
        }
    }
    fn set_enum_data_rc_managed(&mut self, name: &str, managed: bool) {
        if let Some(var) = self.variables.get_mut(name) {
            var.enum_data_rc_managed = managed;
        }
    }
    pub(super) fn remove_var(&mut self, name: &str) {
        self.variables.remove(name);
    }
    fn local_variables(&self) -> &HashMap<String, ScopeVar<'ctx>> {
        &self.variables
    }
}

/// The type of value stored inside an enum variant (Some/Ok).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum InnerType {
    Int,
    Float,
    Str,
}

// ---- TypedValue ----
#[derive(Clone, Copy)]
pub(super) enum TypedValue<'ctx> {
    Int(IntValue<'ctx>),
    Float(inkwell::values::FloatValue<'ctx>),
    Bool(IntValue<'ctx>),
    Str(PointerValue<'ctx>),
    /// Function pointer (lambda) with its function type for correct indirect calls
    Fn(PointerValue<'ctx>, FunctionType<'ctx>),
    /// Closure: heap-allocated captures struct + function pointer for lambdas with free vars
    Closure {
        fn_ptr: PointerValue<'ctx>,
        actual_fn_type: FunctionType<'ctx>,
        closure_ptr: PointerValue<'ctx>,
        closure_ty: StructType<'ctx>,
        /// Scope-variable alloca when this closure was loaded from a variable.
        /// None for lambda-created closures (which are not scope variables).
        alloca: Option<PointerValue<'ctx>>,
    },
    /// List value (pointer to {ptr, i64, i64} alloca)
    List(PointerValue<'ctx>),
    /// Struct value (alloca pointer, LLVM struct type)
    Struct(PointerValue<'ctx>, StructType<'ctx>),
    /// Enum value (alloca pointer to {i64, i8*}, LLVM enum type, inner type, rc_managed)
    Enum(PointerValue<'ctx>, StructType<'ctx>, InnerType, bool),
    /// Map value (alloca pointer to {ptr, i64, i64}, same layout as list)
    Map(PointerValue<'ctx>),
    /// Set value (alloca pointer to {ptr, i64, i64}, same layout as list)
    Set(PointerValue<'ctx>),
    /// Task<T> value (alloca pointer to {ptr, i64, i64}, same layout as list, stores single fat struct)
    Task(PointerValue<'ctx>),
    /// Stream<T> value (alloca pointer to {ptr, i64, i64}, same layout as list)
    Stream(PointerValue<'ctx>),
    /// LazyList<T> value (alloca pointer to {i64, ptr, i64, i64} struct)
    LazyList(PointerValue<'ctx>),
    /// CString value (pointer to null-terminated C string)
    CString(PointerValue<'ctx>),
    /// Ptr<T> value (opaque pointer for FFI)
    Ptr(PointerValue<'ctx>),
    /// FileHandle value (wraps FILE* pointer)
    FileHandle(PointerValue<'ctx>),
    /// Nullable value: alloca pointer to {i1 null_flag, T value}, inner LLVM type
    Nullable(PointerValue<'ctx>, BasicTypeEnum<'ctx>),
    Unit,
}

impl<'ctx> TypedValue<'ctx> {
    fn to_bv(&self) -> Option<BasicValueEnum<'ctx>> {
        match self {
            TypedValue::Int(v) => Some(v.as_basic_value_enum()),
            TypedValue::Float(v) => Some(v.as_basic_value_enum()),
            TypedValue::Bool(v) => Some(v.as_basic_value_enum()),
            TypedValue::Str(_v) => None,
            TypedValue::Fn(ptr, _) => Some(ptr.as_basic_value_enum()),
            TypedValue::Closure { closure_ptr, .. } => Some(closure_ptr.as_basic_value_enum()),
            TypedValue::List(_) => None,
            TypedValue::Map(_) => None,
            TypedValue::Set(_) => None,
            TypedValue::Task(_) => None,
            TypedValue::Stream(_)
            | TypedValue::LazyList(_)
            | TypedValue::CString(_)
            | TypedValue::FileHandle(_) => None,
            TypedValue::Ptr(v) => Some(v.as_basic_value_enum()),
            TypedValue::Struct(_, _) => None,
            TypedValue::Enum(..) => None,
            TypedValue::Nullable(_, _) => None,
            TypedValue::Unit => None,
        }
    }
}

pub(super) fn llvm_err(e: BuilderError) -> String {
    format!("LLVM: {:?}", e)
}

// ---- CodeGen ----
pub struct CodeGen<'ctx> {
    pub(super) context: &'ctx Context,
    pub(super) module: Module<'ctx>,
    pub(super) builder: inkwell::builder::Builder<'ctx>,
    pub(super) scope: Scope<'ctx>,
    pub(super) string_type: StructType<'ctx>,
    pub(super) list_type: StructType<'ctx>,
    /// Block-based B-tree leaf node: {i32 count, i32 pad, [B x {i64, ptr}] elements}
    pub(super) leaf_type: StructType<'ctx>,
    /// Block-based B-tree internal node: {i32 count, i32 pad, i64 total, [B x {ptr, i64}] children}
    pub(super) internal_type: StructType<'ctx>,
    /// Child entry in internal node: {ptr child, i64 subtree_total}
    pub(super) child_entry_type: StructType<'ctx>,
    /// List iterator cursor: {ptr leaf, i64 pos, ptr internal, i64 leaf_count, i64 child_idx}
    pub(super) cursor_type: StructType<'ctx>,
    /// ConcatNode: {i64 depth, i64 total_len, list_type left, list_type right}
    /// height = -1 is the sentinel; node_ptr points to this heap-allocated struct
    /// TODO: reserved for lazy list concatenation — remove once full lazy-list codegen is wired
    #[allow(dead_code)]
    pub(super) concat_node_type: StructType<'ctx>,
    pub(super) lambda_count: usize,
    pub(super) str_pat_counter: usize,
    pub(super) registry: TypeRegistry,
    pub(super) named_structs: HashMap<String, StructType<'ctx>>,
    pub(super) enum_types: HashMap<String, StructType<'ctx>>,
    pub(super) anon_structs: HashMap<Vec<String>, StructType<'ctx>>,
    /// Compile-time constants: name → (global pointer, element type, ValKind)
    pub(super) consts: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>, ValKind)>,
    /// Target block for `continue` — set inside for loops, cleared on exit
    pub(super) continue_target: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    /// Target block for `break` — set inside for loops, cleared on exit
    pub(super) break_target: Option<inkwell::basic_block::BasicBlock<'ctx>>,
    /// Extension method mapping: "TypeName.method" → "TypeName_method"
    pub(super) extension_methods: HashMap<String, String>,
    /// TCO (Tail Call Optimization) state for the current function
    pub(super) tco_state: Option<TcoState<'ctx>>,
    /// Coroutine: list alloca where launch results are collected inside coroutineScope.
    /// None means we are not inside a coroutineScope.
    pub(super) coroutine_collector: Option<inkwell::values::PointerValue<'ctx>>,
    /// Task type: {pthread: i64, done: i64, cancelled: i64, result_list: {ptr, i64, i64}}
    pub(super) task_type: StructType<'ctx>,
    /// LazyList type: {head_val: i64, step_fn: i8*, state: i64, take_count: i64, map_fn: i8*, filter_fn: i8*}
    /// take_count = -1 means infinite (or no step fn), >=0 means take that many
    /// map_fn is an optional transformer applied during to_list evaluation
    /// filter_fn is an optional predicate; elements failing the predicate are skipped
    pub(super) lazylist_type: StructType<'ctx>,
    /// Range type: {start: i64, end: i64, inclusive: i64}
    pub(super) range_type: StructType<'ctx>,
    /// Stream type: {mutex: [40 x i8], list: {ptr, i64, i64}} (mutex-protected buffer)
    pub(super) stream_type: StructType<'ctx>,
    /// Fat return type: named {i64, ptr} struct distinct from enum types.
    /// Used for untyped function/lambda returns. When packed with a scalar,
    /// field 0 holds the value and field 1 is null. When wrapping an enum,
    /// field 0 is the tag and field 1 is the data pointer.
    pub(super) fat_return_type: StructType<'ctx>,
    /// Last fat_ret alloca from unpack_fat_return/bv_to_typed, for potential
    /// bitcast when the result is returned from a typed function (e.g., enum).
    pub(super) last_fat_ret: Option<(PointerValue<'ctx>, StructType<'ctx>)>,
    /// Last-known enum inner type for bv_to_typed to preserve through struct→Enum conversion.
    pub(super) last_enum_inner: Option<(InnerType, bool)>,
    /// Overloaded function mapping: base name → [(param_types, mangled_name)]
    /// e.g., "add" → [([Int, Int], "add_Int_Int"), ([Float, Float], "add_Float_Float")]
    pub(super) overloaded_functions: HashMap<String, Vec<(Vec<Type>, String)>>,
    /// Whether we are currently compiling inside an `unsafe { }` block
    pub(super) in_unsafe: bool,
    /// External C functions declared via `external fun`: name → LLVM function value
    /// TODO: reserved for cross-module FFI — remove once external fun codegen is fully wired
    #[allow(dead_code)]
    pub(super) external_fns: HashMap<String, inkwell::values::FunctionValue<'ctx>>,
    /// Builtin wrappers needed for :: function references (e.g., List::head)
    pub(super) builtin_wrappers_needed: HashSet<String>,
    /// LLVM optimization level (0-3)
    pub(super) opt_level: u8,
    /// Target triple for cross-compilation (None = native)
    pub(super) target_triple: Option<String>,
    /// Counter for unique wrapper function names (lazy_map, lazy_filter, etc.)
    pub(super) wrapper_counter: u64,
    /// Counter for synthetic receiver names in nullable method call short-circuit
    pub(super) synthetic_counter: u64,
    /// Nullable type cache: type name string → {i1, T} LLVM struct type
    pub(super) nullable_types: HashMap<String, StructType<'ctx>>,
    /// Smart cast: variables known to be non-null in current scope (from when matching)
    pub(super) not_null_set: HashSet<String>,
    /// Generic function definitions with type_params, indexed by name.
    /// Used for monomorphization at call sites.
    pub(super) generic_fun_defs: HashMap<String, Stmt>,
    /// Tracks whether compile_block did an rc_inc on the last expression.
    /// val stmt uses this to apply a balancing rc_dec.
    pub(super) block_did_rc_inc: bool,
}

pub(super) struct TcoState<'ctx> {
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
        // ConcatNode for lazy concatenation: {i64 depth, i64 total_len, list_type left, list_type right}
        let concat_node_type = context.struct_type(
            &[
                context.i64_type().into(), // depth
                context.i64_type().into(), // total_len
                list_type.into(),          // left: {ptr, i64, i64}
                list_type.into(),          // right: {ptr, i64, i64}
            ],
            false,
        );
        // List iterator cursor: {ptr leaf, i64 pos, ptr internal, i64 leaf_count, i64 child_idx}
        let cursor_type = context.struct_type(
            &[
                context.ptr_type(inkwell::AddressSpace::default()).into(), // leaf_ptr
                context.i64_type().into(),                                 // pos
                context.ptr_type(inkwell::AddressSpace::default()).into(), // internal_ptr
                context.i64_type().into(),                                 // leaf_count
                context.i64_type().into(),                                 // child_idx
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
            concat_node_type,
            cursor_type,
            lambda_count: 0,
            str_pat_counter: 0,
            registry,
            named_structs: HashMap::new(),
            enum_types: HashMap::new(),
            anon_structs: HashMap::new(),
            consts: HashMap::new(),
            continue_target: None,
            break_target: None,
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
            external_fns: HashMap::new(),
            builtin_wrappers_needed: HashSet::new(),
            opt_level: 0,
            target_triple,
            wrapper_counter: 0,
            synthetic_counter: 0,
            nullable_types: HashMap::new(),
            not_null_set: HashSet::new(),
            generic_fun_defs: HashMap::new(),
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

mod compile;
// ---- Submodules ----
mod builtins_call;
mod builtins_conversion;
mod builtins_ffi;
mod builtins_iter;
mod builtins_lazy;
mod builtins_list;
mod builtins_map;
mod builtins_nullable;
mod builtins_print;
mod builtins_range;
mod builtins_stdlib;
mod builtins_stdlib_datetime;
mod builtins_stream;
mod builtins_thread;
mod expr;
mod for_loop;
mod generics;
mod gep_cursor;
mod jit;
mod map_set;
mod misc;
mod nullable;
mod pattern;
mod rc_ops;
mod runtime_decl;
mod runtime_io;
mod stmt;
mod struct_ops;
mod type_helpers;

pub(super) use self::gep_cursor::GepCursor;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::typecheck::TypeChecker;
    use crate::typecheck::TypeRegistry;
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

        // Compile to LLVM IR
        // Use a shared context to avoid STATUS_ACCESS_VIOLATION on Windows
        // when creating multiple LLVM contexts in the same process.
        let guard = TEST_CONTEXT
            .get_or_init(|| Mutex::new(Context::create()))
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        let mut cg = CodeGen::new(&guard, "test", registry, None);
        cg.compile(&program).expect("Compilation should succeed");
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
        let ir = compile_program("val x Int = 42");
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
}
