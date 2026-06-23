//! Nullable smart-cast and synthetic receiver state.

use inkwell::types::StructType;
use std::collections::{HashMap, HashSet};

pub(crate) struct NullableState<'ctx> {
    pub nullable_types: HashMap<String, StructType<'ctx>>,
    pub not_null_set: HashSet<String>,
    pub synthetic_counter: u64,
}

impl<'ctx> Default for NullableState<'ctx> {
    fn default() -> Self {
        Self {
            nullable_types: HashMap::new(),
            not_null_set: HashSet::new(),
            synthetic_counter: 0,
        }
    }
}
