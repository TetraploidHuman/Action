//! Builtin function type signatures and UFCS metadata (frontend layer).
//!
//! Codegen dispatch enums live in `backend::codegen::builtin_dispatch`.

mod registry;

pub use registry::{
    all, lookup, lookup_ufcs, receiver_kind_from_type, BuiltinDef, UfcsReceiverKind,
};
