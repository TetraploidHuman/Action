//! Builtin function type signatures and UFCS metadata (frontend layer).
//!
//! Codegen dispatch enums live in `backend::codegen::builtin_dispatch`.

mod registry;

pub use registry::{
    all, format_builtin_detail, format_ufcs_method_detail, lookup, lookup_return_type, lookup_ufcs,
    lookup_ufcs_return_type, receiver_kind_from_type, ufcs_methods_for_kind, BuiltinDef,
    UfcsReceiverKind,
};
