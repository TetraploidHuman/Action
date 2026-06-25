//! Function symbol metadata for fallibility tracking (R7).

use crate::ast::Type;

/// Resolved function/builtin symbol with fallibility metadata.
#[derive(Clone, Debug, PartialEq)]
pub struct FunctionSymbol {
    pub is_fallible: bool,
    pub return_type: Type,
}

impl FunctionSymbol {
    pub fn new(is_fallible: bool, return_type: Type) -> Self {
        Self {
            is_fallible,
            return_type,
        }
    }
}
