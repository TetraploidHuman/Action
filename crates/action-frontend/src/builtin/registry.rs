use crate::ast::Type;
use std::sync::OnceLock;

/// UFCS receiver kind: which type `recv.method(...)` may bind to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UfcsReceiverKind {
    /// Global call only (`len(x)`), not `x.len()`.
    Global,
    List,
    Map,
    Set,
    String,
    Stream,
    Task,
    /// `len` / `isEmpty` on list, map, set, or string.
    Collection,
}

/// Metadata for one builtin function.
#[derive(Clone, Debug)]
pub struct BuiltinDef {
    pub name: &'static str,
    pub param_types: Vec<Type>,
    pub return_type: Type,
    pub ufcs_receiver: UfcsReceiverKind,
    pub readonly: bool,
    pub supports_trailing_lambda: bool,
    pub fallible: bool,
}

fn int() -> Type {
    Type::Named("Int".into())
}
fn lazy_list() -> Type {
    Type::LazyList(Box::new(int()))
}
fn float() -> Type {
    Type::Named("Float".into())
}
fn char_ty() -> Type {
    Type::Named("Char".into())
}
fn bool() -> Type {
    Type::Named("Bool".into())
}
fn string() -> Type {
    Type::Named("String".into())
}
fn list() -> Type {
    Type::Named("List".into())
}
fn partition_pair() -> Type {
    Type::Struct(vec![("_0".into(), list()), ("_1".into(), list())])
}
fn next_int_pair() -> Type {
    Type::Struct(vec![("_0".into(), random_ty()), ("_1".into(), int())])
}
fn list_of(elem: Type) -> Type {
    Type::Generic(Box::new(Type::Named("List".into())), vec![elem])
}
fn list_string() -> Type {
    list_of(string())
}
fn unit() -> Type {
    Type::Unit
}
fn fn_int_to_int() -> Type {
    Type::Function(vec![int()], Box::new(int()))
}
fn map_ty() -> Type {
    Type::Map(Box::new(int()), Box::new(int()))
}
fn set_ty() -> Type {
    Type::Set(Box::new(int()))
}
fn date_ty() -> Type {
    Type::Named("Date".into())
}
fn datetime_ty() -> Type {
    Type::Named("DateTime".into())
}
fn random_ty() -> Type {
    Type::Named("Random".into())
}
fn cstring() -> Type {
    Type::Named("CString".into())
}
fn task_int() -> Type {
    Type::Task(Box::new(int()))
}
fn stream_int() -> Type {
    Type::Stream(Box::new(int()))
}
fn ptr_json_node() -> Type {
    Type::Ptr(Box::new(Type::Named("JsonNode".into())))
}
fn http_response_ty() -> Type {
    Type::Named("HttpResponse".into())
}
fn file_handle() -> Type {
    Type::FileHandle
}

fn build_registry() -> Vec<BuiltinDef> {
    vec![
        BuiltinDef {
            name: "len",
            param_types: vec![list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Collection,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "isEmpty",
            param_types: vec![list()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Collection,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "map",
            param_types: vec![fn_int_to_int(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "filter",
            param_types: vec![fn_int_to_int(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "fold",
            param_types: vec![fn_int_to_int(), int(), list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "append",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "prepend",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "get",
            param_types: vec![list(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "head",
            param_types: vec![list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "last",
            param_types: vec![list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "contains",
            param_types: vec![list(), int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "take",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "drop",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "reverse",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "concat",
            param_types: vec![list(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "concat",
            param_types: vec![string(), string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "flatten",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "sum",
            param_types: vec![list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "print",
            param_types: vec![int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "println",
            param_types: vec![int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "toString",
            param_types: vec![int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        // Callback-based list builtins (hot path; dispatch via builtin_callback_list)
        BuiltinDef {
            name: "any",
            param_types: vec![fn_int_to_int(), list()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "all",
            param_types: vec![fn_int_to_int(), list()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "indexOf",
            param_types: vec![list(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "indexOf",
            param_types: vec![string(), string()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "tail",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "init",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "remove",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "insert",
            param_types: vec![list(), int(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "withIndex",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        // --- stdlib (global) ---
        BuiltinDef {
            name: "send",
            param_types: vec![stream_int(), int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "close",
            param_types: vec![stream_int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "cancel",
            param_types: vec![task_int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "toCString",
            param_types: vec![string()],
            return_type: cstring(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "fromCString",
            param_types: vec![cstring()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "readLine",
            param_types: vec![],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "readFile",
            param_types: vec![string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "writeFile",
            param_types: vec![string(), string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "appendFile",
            param_types: vec![string(), string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "exists",
            param_types: vec![string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "deleteFile",
            param_types: vec![string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "openFile",
            param_types: vec![string(), string()],
            return_type: file_handle(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "closeFile",
            param_types: vec![file_handle()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "isEof",
            param_types: vec![file_handle()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "fileReadLine",
            param_types: vec![file_handle()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "httpRequest",
            param_types: vec![string(), string(), string(), string()],
            return_type: http_response_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "jsonEscape",
            param_types: vec![string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "substring",
            param_types: vec![string(), int(), int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        // Also register Global for Ident-form `substring(s, i, j)` (UFCS-only above is skipped by lookup).
        BuiltinDef {
            name: "substring",
            param_types: vec![string(), int(), int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "str",
            param_types: vec![int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "toUpper",
            param_types: vec![string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "toLower",
            param_types: vec![string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "receive",
            param_types: vec![stream_int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "wait",
            param_types: vec![task_int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "is_done",
            param_types: vec![task_int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "is_cancelled",
            param_types: vec![task_int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "withTimeout",
            param_types: vec![int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: true,
        },
        BuiltinDef {
            name: "__list",
            param_types: vec![],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "coroutineScope",
            param_types: vec![fn_int_to_int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "find",
            param_types: vec![fn_int_to_int(), list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: true,
            fallible: true,
        },
        BuiltinDef {
            name: "findIndex",
            param_types: vec![fn_int_to_int(), list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: true,
            fallible: true,
        },
        BuiltinDef {
            name: "reduce",
            param_types: vec![fn_int_to_int(), list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: true,
            fallible: true,
        },
        BuiltinDef {
            name: "foldRight",
            param_types: vec![fn_int_to_int(), int(), list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "takeWhile",
            param_types: vec![fn_int_to_int(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "dropWhile",
            param_types: vec![fn_int_to_int(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "sortedBy",
            param_types: vec![fn_int_to_int(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        // --- Map UFCS (ufcs-only) ---
        BuiltinDef {
            name: "contains",
            param_types: vec![map_ty(), int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "insert",
            param_types: vec![map_ty(), int(), int()],
            return_type: map_ty(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "remove",
            param_types: vec![map_ty(), int()],
            return_type: map_ty(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "get",
            param_types: vec![map_ty(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        // Map HOF UFCS (`m.filter` / `m.mapValues` / `m.fold`) — distinct from List methods.
        BuiltinDef {
            name: "filter",
            param_types: vec![map_ty(), fn_int_to_int()],
            return_type: map_ty(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "mapValues",
            param_types: vec![map_ty(), fn_int_to_int()],
            return_type: map_ty(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "fold",
            param_types: vec![map_ty(), int(), fn_int_to_int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: true,
            supports_trailing_lambda: true,
            fallible: false,
        },
        // Map projection UFCS aliases (codegen remaps to mapKeys / mapValues / …).
        BuiltinDef {
            name: "keys",
            param_types: vec![map_ty()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "values",
            param_types: vec![map_ty()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "entries",
            param_types: vec![map_ty()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "union",
            param_types: vec![map_ty(), map_ty()],
            return_type: map_ty(),
            ufcs_receiver: UfcsReceiverKind::Map,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        // --- Set UFCS (ufcs-only) ---
        BuiltinDef {
            name: "contains",
            param_types: vec![set_ty(), int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Set,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "insert",
            param_types: vec![set_ty(), int()],
            return_type: set_ty(),
            ufcs_receiver: UfcsReceiverKind::Set,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "remove",
            param_types: vec![set_ty(), int()],
            return_type: set_ty(),
            ufcs_receiver: UfcsReceiverKind::Set,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        // --- Stream UFCS (ufcs-only) ---
        BuiltinDef {
            name: "send",
            param_types: vec![stream_int(), int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Stream,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "receive",
            param_types: vec![stream_int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Stream,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "close",
            param_types: vec![stream_int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Stream,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        // --- Task UFCS (ufcs-only) ---
        BuiltinDef {
            name: "cancel",
            param_types: vec![task_int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Task,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "is_done",
            param_types: vec![task_int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Task,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "is_cancelled",
            param_types: vec![task_int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Task,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "wait",
            param_types: vec![task_int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Task,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        // --- M53: math / assert / string / list / map-set / host session / RNG ---
        BuiltinDef {
            name: "abs",
            param_types: vec![int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "min",
            param_types: vec![int(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "max",
            param_types: vec![int(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "clamp",
            param_types: vec![int(), int(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "sqrt",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "cbrt",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "sin",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "cos",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "tan",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "asin",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "acos",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "atan",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "atan2",
            param_types: vec![float(), float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "log",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "log2",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "log10",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "exp",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "floor",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "ceil",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "round",
            param_types: vec![float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "pow",
            param_types: vec![float(), float()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "pi",
            param_types: vec![],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "e",
            param_types: vec![],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "isNaN",
            param_types: vec![float()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "isInfinite",
            param_types: vec![float()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "assert",
            param_types: vec![bool(), string()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "panic",
            param_types: vec![string()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "trim",
            param_types: vec![string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "trimStart",
            param_types: vec![string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "trimEnd",
            param_types: vec![string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "startsWith",
            param_types: vec![string(), string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "endsWith",
            param_types: vec![string(), string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "split",
            param_types: vec![string(), string()],
            return_type: list_string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "splitLines",
            param_types: vec![string()],
            return_type: list_string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "join",
            param_types: vec![list_string(), string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "replace",
            param_types: vec![string(), string(), string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "stringContains",
            param_types: vec![string(), string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "stringRepeat",
            param_types: vec![string(), int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "charAt",
            param_types: vec![string(), int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "chars",
            param_types: vec![string()],
            return_type: list_string(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "isAlpha",
            param_types: vec![string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "charCode",
            param_types: vec![string()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "codeToChar",
            param_types: vec![int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "range",
            param_types: vec![int(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "repeat",
            param_types: vec![int(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "unique",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "slice",
            param_types: vec![list(), int(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "sorted",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "chunks",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "windows",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "splitAt",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "zip",
            param_types: vec![list(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "product",
            param_types: vec![list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "digits",
            param_types: vec![int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "identity",
            param_types: vec![int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "mapKeys",
            param_types: vec![map_ty()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "mapValues",
            param_types: vec![map_ty()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "mapEntries",
            param_types: vec![map_ty()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "mapUnion",
            param_types: vec![map_ty(), map_ty()],
            return_type: map_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "containsKey",
            param_types: vec![map_ty(), int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "setToList",
            param_types: vec![set_ty()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "setFromList",
            param_types: vec![list()],
            return_type: set_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "fromList",
            param_types: vec![list()],
            return_type: set_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "setUnion",
            param_types: vec![set_ty(), set_ty()],
            return_type: set_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "setIntersection",
            param_types: vec![set_ty(), set_ty()],
            return_type: set_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "setDifference",
            param_types: vec![set_ty(), set_ty()],
            return_type: set_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "setIsSubset",
            param_types: vec![set_ty(), set_ty()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "bsBufClear",
            param_types: vec![int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "bsBufAppend",
            param_types: vec![int(), string()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "bsBufSet",
            param_types: vec![int(), string()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "bsBufGet",
            param_types: vec![int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "bsIntSet",
            param_types: vec![int(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "bsIntGet",
            param_types: vec![int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "randInt",
            param_types: vec![int(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "randFloat",
            param_types: vec![],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        // --- M57: datetime call APIs (codegen: builtins/stdlib/datetime/*) ---
        BuiltinDef {
            name: "today",
            param_types: vec![],
            return_type: date_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "now",
            param_types: vec![],
            return_type: datetime_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "nowUtc",
            param_types: vec![],
            return_type: datetime_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "date",
            param_types: vec![int(), int(), int()],
            return_type: date_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "datetime",
            param_types: vec![int(), int(), int(), int(), int(), int()],
            return_type: datetime_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "format",
            param_types: vec![datetime_ty(), string()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "parseDate",
            // Codegen: (format_str, date_str); format is passed to sscanf (three %d fields).
            param_types: vec![string(), string()],
            return_type: date_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "addDays",
            param_types: vec![date_ty(), int()],
            return_type: date_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "addHours",
            param_types: vec![datetime_ty(), int()],
            return_type: datetime_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "diffDays",
            param_types: vec![date_ty(), date_ty()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "diffSeconds",
            param_types: vec![datetime_ty(), datetime_ty()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "weekday",
            param_types: vec![date_ty()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "year",
            param_types: vec![date_ty()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "month",
            param_types: vec![date_ty()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "day",
            param_types: vec![date_ty()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "hour",
            param_types: vec![datetime_ty()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "minute",
            param_types: vec![datetime_ty()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "second",
            param_types: vec![datetime_ty()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "Random_new",
            param_types: vec![int()],
            return_type: random_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        // Persistent PRNG step: returns {Random, Int} pair (updated state + sample).
        BuiltinDef {
            name: "nextInt",
            param_types: vec![random_ty(), int(), int()],
            return_type: next_int_pair(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "randShuffle",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "randChoice",
            param_types: vec![list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        // --- M54: lazy / higher-order / datetime / IO extras ---
        BuiltinDef {
            name: "lazy_list",
            param_types: vec![int()],
            return_type: lazy_list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "toList",
            param_types: vec![lazy_list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "toLazyList",
            param_types: vec![list()],
            return_type: lazy_list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "lazyTake",
            param_types: vec![int(), lazy_list()],
            return_type: lazy_list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "lazyDrop",
            param_types: vec![int(), lazy_list()],
            return_type: lazy_list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "lazyMap",
            param_types: vec![fn_int_to_int(), lazy_list()],
            return_type: lazy_list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "lazyFilter",
            param_types: vec![fn_int_to_int(), lazy_list()],
            return_type: lazy_list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "lazyTakeWhile",
            param_types: vec![fn_int_to_int(), lazy_list()],
            return_type: lazy_list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "lazyZip",
            param_types: vec![lazy_list(), lazy_list()],
            return_type: lazy_list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "flatMap",
            param_types: vec![list(), fn_int_to_int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        // partition: codegen returns anonymous {List, List}; type as Struct so `parts[0]` is
        // a literal tuple slot (no E006) while `parts[0][i]` remains a fallible list index.
        BuiltinDef {
            name: "partition",
            param_types: vec![fn_int_to_int(), list()],
            return_type: partition_pair(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "count",
            param_types: vec![fn_int_to_int(), list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: true,
            fallible: false,
        },
        // M56: formerly check_call allowlist-only (codegen existed; soft fresh_var).
        BuiltinDef {
            name: "delay",
            param_types: vec![int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "mapFilter",
            param_types: vec![map_ty(), fn_int_to_int()],
            return_type: map_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "mapMapValues",
            param_types: vec![map_ty(), fn_int_to_int()],
            return_type: map_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "mapFold",
            param_types: vec![int(), map_ty(), fn_int_to_int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: true,
            fallible: false,
        },
        BuiltinDef {
            name: "compose",
            param_types: vec![fn_int_to_int(), fn_int_to_int(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "flip",
            param_types: vec![fn_int_to_int(), int(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "constant",
            param_types: vec![int(), int()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "fileReadBytes",
            param_types: vec![file_handle(), int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "fileWrite",
            param_types: vec![file_handle(), string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "fileWriteLine",
            param_types: vec![file_handle(), string()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "fileFlush",
            param_types: vec![file_handle()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "fileSeek",
            param_types: vec![file_handle(), int(), int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "fileTell",
            param_types: vec![file_handle()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: false,
        },
        BuiltinDef {
            name: "readDir",
            param_types: vec![string()],
            return_type: list_string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        // --- Fallible builtins (R7) ---
        BuiltinDef {
            name: "toInt",
            param_types: vec![string()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "toFloat",
            param_types: vec![string()],
            return_type: float(),
            ufcs_receiver: UfcsReceiverKind::String,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "toChar",
            param_types: vec![int()],
            return_type: char_ty(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "lazyHead",
            param_types: vec![lazy_list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "parseInt",
            param_types: vec![string()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "__jsonParse",
            param_types: vec![cstring()],
            return_type: ptr_json_node(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "__jsonGet",
            param_types: vec![ptr_json_node(), cstring()],
            return_type: ptr_json_node(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
        BuiltinDef {
            name: "__jsonGetIdx",
            param_types: vec![ptr_json_node(), int()],
            return_type: ptr_json_node(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
            fallible: true,
        },
    ]
}

static REGISTRY: OnceLock<Vec<BuiltinDef>> = OnceLock::new();

pub fn all() -> &'static [BuiltinDef] {
    REGISTRY.get_or_init(build_registry)
}

pub fn lookup(name: &str) -> Option<&'static BuiltinDef> {
    all().iter().find(|d| {
        d.name == name
            && !matches!(
                d.ufcs_receiver,
                UfcsReceiverKind::Map
                    | UfcsReceiverKind::Set
                    | UfcsReceiverKind::Stream
                    | UfcsReceiverKind::Task
            )
    })
}

fn is_global_callable(d: &BuiltinDef) -> bool {
    !matches!(
        d.ufcs_receiver,
        UfcsReceiverKind::Map
            | UfcsReceiverKind::Set
            | UfcsReceiverKind::Stream
            | UfcsReceiverKind::Task
    )
}

/// All global/Ident-form overloads of `name` (excludes Map/Set/Stream/Task-only UFCS entries).
pub fn lookup_overloads(name: &str) -> Vec<&'static BuiltinDef> {
    all()
        .iter()
        .filter(|d| d.name == name && is_global_callable(d))
        .collect()
}

/// Resolve a global builtin overload by argument types (e.g. `concat` List vs String).
pub fn lookup_matching(name: &str, arg_tys: &[Type]) -> Option<&'static BuiltinDef> {
    use crate::types::types_compatible;
    lookup_overloads(name).into_iter().find(|d| {
        d.param_types.len() == arg_tys.len()
            && d.param_types
                .iter()
                .zip(arg_tys.iter())
                .all(|(p, a)| types_compatible(p, a))
    })
}

/// Return type for a global builtin call (`name(...)`).
pub fn lookup_return_type(name: &str) -> Option<Type> {
    lookup(name).map(|d| d.return_type.clone())
}

/// Return type for a global builtin call with known argument types.
pub fn lookup_return_type_for_args(name: &str, arg_tys: &[Type]) -> Option<Type> {
    lookup_matching(name, arg_tys)
        .or_else(|| lookup(name))
        .map(|d| d.return_type.clone())
}

/// Return type for a UFCS builtin call (`recv.name(...)`).
pub fn lookup_ufcs_return_type(kind: UfcsReceiverKind, method: &str) -> Option<Type> {
    lookup_ufcs(kind, method).map(|d| d.return_type.clone())
}

/// Map a typechecker receiver type to a UFCS kind.
pub fn receiver_kind_from_type(ty: &Type) -> Option<UfcsReceiverKind> {
    use crate::types::collection_kind_from_type;
    match ty {
        Type::Named(n) if n == "String" => Some(UfcsReceiverKind::String),
        Type::LazyList(_) => Some(UfcsReceiverKind::List),
        Type::Map(_, _) => Some(UfcsReceiverKind::Map),
        Type::Set(_) => Some(UfcsReceiverKind::Set),
        Type::Stream(_) => Some(UfcsReceiverKind::Stream),
        Type::Task(_) => Some(UfcsReceiverKind::Task),
        ty if collection_kind_from_type(ty) == Some(crate::types::CollectionKind::List) => {
            Some(UfcsReceiverKind::List)
        }
        Type::Named(_) => Some(UfcsReceiverKind::Collection),
        _ => None,
    }
}

fn ufcs_matches(def: &BuiltinDef, kind: UfcsReceiverKind) -> bool {
    match def.ufcs_receiver {
        UfcsReceiverKind::Global => false,
        UfcsReceiverKind::Collection => matches!(
            kind,
            UfcsReceiverKind::List
                | UfcsReceiverKind::Map
                | UfcsReceiverKind::Set
                | UfcsReceiverKind::String
                | UfcsReceiverKind::Collection
        ),
        expected => {
            kind == expected
                || (expected == UfcsReceiverKind::List && kind == UfcsReceiverKind::Collection)
        }
    }
}

/// Lookup a UFCS method on a receiver kind.
pub fn lookup_ufcs(kind: UfcsReceiverKind, method: &str) -> Option<&'static BuiltinDef> {
    all()
        .iter()
        .find(|d| d.name == method && ufcs_matches(d, kind))
}

/// All UFCS methods applicable to a receiver kind (for LSP completion).
pub fn ufcs_methods_for_kind(kind: UfcsReceiverKind) -> Vec<&'static BuiltinDef> {
    all().iter().filter(|d| ufcs_matches(d, kind)).collect()
}

/// Format a UFCS method signature for LSP detail text.
pub fn format_ufcs_method_detail(def: &BuiltinDef) -> String {
    let params: Vec<String> = def
        .param_types
        .iter()
        .skip(1) // UFCS: first param is receiver
        .enumerate()
        .map(|(i, ty)| format!("arg{}: {}", i, ty))
        .collect();
    if params.is_empty() {
        format!("{}() -> {}", def.name, def.return_type)
    } else {
        format!("{}({}) -> {}", def.name, params.join(", "), def.return_type)
    }
}

/// Format a global or UFCS builtin for LSP completion detail.
pub fn format_builtin_detail(def: &BuiltinDef) -> String {
    let params: Vec<String> = def
        .param_types
        .iter()
        .enumerate()
        .map(|(i, ty)| format!("p{}: {}", i, ty))
        .collect();
    if params.is_empty() {
        format!("{}() -> {}", def.name, def.return_type)
    } else {
        format!("{}({}) -> {}", def.name, params.join(", "), def.return_type)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_hot_builtins() {
        assert!(lookup("len").is_some());
        assert!(lookup("map").is_some());
        assert!(lookup("indexOf").is_some());
        assert!(lookup("remove").is_some());
        assert!(lookup("insert").is_some());
        // Collection builtins (len/isEmpty) also match List UFCS receivers.
        assert!(lookup_ufcs(UfcsReceiverKind::List, "len").is_some());
        assert!(lookup_ufcs(UfcsReceiverKind::Collection, "len").is_some());
        assert!(lookup_ufcs(UfcsReceiverKind::List, "map").is_some());
        assert!(lookup_ufcs(UfcsReceiverKind::List, "indexOf").is_some());
        assert!(lookup("toFloat").is_some_and(|d| d.fallible));
        assert!(lookup("toChar").is_some_and(|d| d.fallible));
        assert!(lookup("lazyHead").is_some_and(|d| d.fallible));
        assert!(lookup("readFile").is_some());
        assert!(lookup("openFile").is_some());
        assert!(lookup("fileReadLine").is_some_and(|d| d.fallible));
        assert!(lookup("exists").is_some());
        assert!(lookup("abs").is_some());
        assert!(lookup("sqrt").is_some());
        assert!(lookup("assert").is_some());
        assert!(lookup("trim").is_some());
        assert!(lookup("range").is_some());
        assert!(lookup("mapKeys").is_some());
        assert!(lookup("bsBufGet").is_some());
        assert!(lookup("bsIntSet").is_some());
        assert!(lookup("randInt").is_some());
        assert!(matches!(
            lookup("nextInt").map(|d| &d.return_type),
            Some(Type::Struct(fields))
                if fields.len() == 2
                    && matches!(fields[0].1, Type::Named(ref n) if n == "Random")
                    && matches!(fields[1].1, Type::Named(ref n) if n == "Int")
        ));
        assert!(lookup("delay").is_some_and(|d| matches!(d.return_type, Type::Unit)));
        assert!(lookup("mapFilter").is_some());
        assert!(lookup("mapMapValues").is_some());
        assert!(lookup("mapFold").is_some());
        assert!(matches!(
            lookup("partition").map(|d| &d.return_type),
            Some(Type::Struct(fields)) if fields.len() == 2
        ));
        assert!(lookup("withTimeout").is_some_and(|d| d.fallible));
        assert!(lookup("coroutineScope").is_some());
        assert!(lookup("today").is_some());
        assert!(lookup("now").is_some());
        assert!(lookup("date").is_some_and(|d| d.fallible));
        assert!(lookup("datetime").is_some_and(|d| d.fallible));
        assert!(lookup("parseDate").is_some_and(|d| d.fallible));
        assert!(lookup("addDays").is_some());
        assert!(lookup("format").is_some());
        assert!(lookup("weekday").is_some());
        assert!(lookup("year").is_some());
        assert!(lookup_ufcs(UfcsReceiverKind::Map, "filter").is_some());
        assert!(lookup_ufcs(UfcsReceiverKind::Map, "mapValues").is_some());
        assert!(lookup_ufcs(UfcsReceiverKind::Map, "fold").is_some());
        assert!(lookup_ufcs(UfcsReceiverKind::Map, "keys").is_some());
        assert!(lookup_ufcs(UfcsReceiverKind::Map, "union").is_some());
        assert!(lookup_ufcs(UfcsReceiverKind::String, "toFloat").is_some_and(|d| d.fallible));
        assert!(lookup_ufcs(UfcsReceiverKind::String, "indexOf").is_some_and(|d| d.fallible));
        assert!(lookup_ufcs(UfcsReceiverKind::Map, "remove").is_some_and(|d| !d.fallible));
    }
}
