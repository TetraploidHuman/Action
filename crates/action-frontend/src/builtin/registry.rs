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

/// Return type for a global builtin call (`name(...)`).
pub fn lookup_return_type(name: &str) -> Option<Type> {
    lookup(name).map(|d| d.return_type.clone())
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
        assert!(lookup_ufcs(UfcsReceiverKind::String, "toFloat").is_some_and(|d| d.fallible));
        assert!(lookup_ufcs(UfcsReceiverKind::String, "indexOf").is_some_and(|d| d.fallible));
        assert!(lookup_ufcs(UfcsReceiverKind::Map, "remove").is_some_and(|d| !d.fallible));
    }
}
