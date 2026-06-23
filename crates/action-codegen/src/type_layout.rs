//! LLVM struct layout and compile-time constant cache.

use crate::scope::ValKind;
use inkwell::types::{BasicTypeEnum, StructType};
use inkwell::values::PointerValue;
use std::collections::HashMap;

pub(crate) struct TypeLayoutCache<'ctx> {
    pub named_structs: HashMap<String, StructType<'ctx>>,
    pub enum_types: HashMap<String, StructType<'ctx>>,
    pub anon_structs: HashMap<Vec<String>, StructType<'ctx>>,
    pub consts: HashMap<String, (PointerValue<'ctx>, BasicTypeEnum<'ctx>, ValKind)>,
}

impl<'ctx> Default for TypeLayoutCache<'ctx> {
    fn default() -> Self {
        Self {
            named_structs: HashMap::new(),
            enum_types: HashMap::new(),
            anon_structs: HashMap::new(),
            consts: HashMap::new(),
        }
    }
}
