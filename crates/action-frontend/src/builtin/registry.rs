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
}

fn int() -> Type {
    Type::Named("Int".into())
}
fn bool() -> Type {
    Type::Named("Bool".into())
}
fn string() -> Type {
    Type::Named("String".into())
}
fn list() -> Type {
    Type::Named("list".into())
}
fn unit() -> Type {
    Type::Unit
}
fn nullable_int() -> Type {
    Type::Nullable(Box::new(int()))
}
fn fn_int_to_int() -> Type {
    Type::Function(vec![int()], Box::new(int()))
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
        },
        BuiltinDef {
            name: "isEmpty",
            param_types: vec![list()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::Collection,
            readonly: true,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "map",
            param_types: vec![fn_int_to_int(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
        },
        BuiltinDef {
            name: "filter",
            param_types: vec![fn_int_to_int(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
        },
        BuiltinDef {
            name: "fold",
            param_types: vec![fn_int_to_int(), int(), list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: true,
        },
        BuiltinDef {
            name: "append",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "prepend",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "get",
            param_types: vec![list(), int()],
            return_type: nullable_int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "head",
            param_types: vec![list()],
            return_type: nullable_int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "last",
            param_types: vec![list()],
            return_type: nullable_int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "contains",
            param_types: vec![list(), int()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "take",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "drop",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "reverse",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "concat",
            param_types: vec![list(), list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "flatten",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "sum",
            param_types: vec![list()],
            return_type: int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "print",
            param_types: vec![int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "println",
            param_types: vec![int()],
            return_type: unit(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "toString",
            param_types: vec![int()],
            return_type: string(),
            ufcs_receiver: UfcsReceiverKind::Global,
            readonly: true,
            supports_trailing_lambda: false,
        },
        // Callback-based list builtins (hot path; dispatch via builtin_callback_list)
        BuiltinDef {
            name: "any",
            param_types: vec![fn_int_to_int(), list()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: true,
        },
        BuiltinDef {
            name: "all",
            param_types: vec![fn_int_to_int(), list()],
            return_type: bool(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: true,
        },
        BuiltinDef {
            name: "indexOf",
            param_types: vec![list(), int()],
            return_type: nullable_int(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "tail",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "remove",
            param_types: vec![list(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "insert",
            param_types: vec![list(), int(), int()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: false,
            supports_trailing_lambda: false,
        },
        BuiltinDef {
            name: "withIndex",
            param_types: vec![list()],
            return_type: list(),
            ufcs_receiver: UfcsReceiverKind::List,
            readonly: true,
            supports_trailing_lambda: false,
        },
    ]
}

static REGISTRY: OnceLock<Vec<BuiltinDef>> = OnceLock::new();

pub fn all() -> &'static [BuiltinDef] {
    REGISTRY.get_or_init(build_registry)
}

pub fn lookup(name: &str) -> Option<&'static BuiltinDef> {
    all().iter().find(|d| d.name == name)
}

/// Map a typechecker receiver type to a UFCS kind.
pub fn receiver_kind_from_type(ty: &Type) -> Option<UfcsReceiverKind> {
    match ty {
        Type::Named(n) if n == "list" || n == "List" => Some(UfcsReceiverKind::List),
        Type::Named(n) if n == "String" => Some(UfcsReceiverKind::String),
        Type::Map(_, _) => Some(UfcsReceiverKind::Map),
        Type::Set(_) => Some(UfcsReceiverKind::Set),
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
    lookup(method).filter(|d| ufcs_matches(d, kind))
}

/// All UFCS methods applicable to a receiver kind (for LSP completion).
pub fn ufcs_methods_for_kind(kind: UfcsReceiverKind) -> Vec<&'static BuiltinDef> {
    all()
        .iter()
        .filter(|d| ufcs_matches(d, kind))
        .collect()
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
    }
}
