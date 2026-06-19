//! Codegen dispatch targets for registry-backed builtins.
//!
//! Frontend [`BuiltinDef`] holds type/UFCS metadata; this module maps names to
//! LLVM/codegen handlers and provides dispatch-time helpers.

use action_frontend::builtin::{BuiltinDef, UfcsReceiverKind};

/// Codegen dispatch target for a builtin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BuiltinDispatch {
    Stdlib,
    Print,
    Map,
    Filter,
    Fold,
    CallbackList,
}

impl BuiltinDispatch {
    /// Resolve dispatch from a builtin name.
    pub fn for_name(name: &str) -> Self {
        match name {
            "print" | "println" => BuiltinDispatch::Print,
            "map" => BuiltinDispatch::Map,
            "filter" => BuiltinDispatch::Filter,
            "fold" => BuiltinDispatch::Fold,
            "any" | "all" => BuiltinDispatch::CallbackList,
            _ => BuiltinDispatch::Stdlib,
        }
    }

    /// Resolve dispatch from frontend registry metadata.
    pub fn for_builtin(def: &BuiltinDef) -> Self {
        Self::for_name(def.name)
    }

    pub fn is_readonly_ufcs_on_collection(def: &BuiltinDef) -> bool {
        def.readonly && def.ufcs_receiver == UfcsReceiverKind::Collection
    }

    /// Index of the list argument for higher-order builtins (map/filter/fold/any/all).
    pub fn list_operand_index(
        def: &BuiltinDef,
        has_trailing: bool,
        arg_count: usize,
    ) -> Option<usize> {
        match Self::for_builtin(def) {
            BuiltinDispatch::Map | BuiltinDispatch::Filter | BuiltinDispatch::CallbackList => {
                if has_trailing {
                    Some(0)
                } else if arg_count >= 2 {
                    Some(1)
                } else {
                    None
                }
            }
            BuiltinDispatch::Fold => {
                if has_trailing && arg_count >= 2 {
                    Some(1)
                } else if arg_count >= 3 {
                    Some(1)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
