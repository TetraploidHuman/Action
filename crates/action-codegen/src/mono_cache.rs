//! Generic monomorphization cache.

use action_frontend::ast::Type;
use action_frontend::hir::HirStmt;
use std::collections::{HashMap, HashSet};

pub(crate) struct MonoCache {
    pub generic_fun_defs: HashMap<String, HirStmt>,
    pub monomorphized_fns: HashSet<String>,
    pub fun_return_types: HashMap<String, Type>,
    /// LLVM-mangled names of user functions using the `{T, i1}` fallible ABI.
    pub fallible_user_fns: HashSet<String>,
}

impl Default for MonoCache {
    fn default() -> Self {
        Self {
            generic_fun_defs: HashMap::new(),
            monomorphized_fns: HashSet::new(),
            fun_return_types: HashMap::new(),
            fallible_user_fns: HashSet::new(),
        }
    }
}
