//! LSP handler helpers (R4-5).

mod completion;
mod scope;
mod signature;

pub(crate) use completion::*;
pub(crate) use scope::*;
pub(crate) use signature::*;

#[cfg(test)]
mod tests;
