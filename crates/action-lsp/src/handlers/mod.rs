//! LSP handlers (R3-6).

mod document;
mod editing;
pub(crate) mod helpers;
mod navigation;
mod rename;
mod symbols;

use crate::project::Project;

pub struct ServerState {
    pub project: Project,
}

impl ServerState {
    pub fn new(project: Project) -> Self {
        Self { project }
    }
}

pub use document::{handle_did_change, handle_did_close, handle_did_open, handle_formatting};
pub use editing::{handle_code_actions, handle_completion, handle_signature_help};
pub use navigation::{
    handle_document_highlight, handle_goto_definition, handle_hover, handle_references,
};
pub use rename::{handle_inlay_hints, handle_prepare_rename, handle_rename};
pub use symbols::{
    handle_document_symbols, handle_folding_range, handle_semantic_tokens, handle_workspace_symbol,
};
