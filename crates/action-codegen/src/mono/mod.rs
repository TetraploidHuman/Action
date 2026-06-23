//! Lambda monomorphization submodule tree.

// P2: monomorphic lambda direct-call specialization for map/filter/fold/any/all.
//
// Capture-free (or simple scalar-capture) lambdas compile to internal LLVM
// functions; higher-order builtins call them directly via B-tree walks instead
// of passing fn ptrs into action_list_*_walk runtime helpers.

use inkwell::values::{FunctionValue, PointerValue};

use super::{llvm_err, CodeGen, TypedValue};

const LEAF_BATCH: u64 = 64;

/// A lambda that can be invoked with a direct LLVM call inside list walks.
pub(super) enum DirectLambdaTarget<'ctx> {
    /// No captures: `lambda(arg)` or `lambda(acc, arg)`.
    Plain(FunctionValue<'ctx>),
    /// Scalar captures only: `lambda(captures_ptr, arg)` or `lambda(captures_ptr, acc, arg)`.
    WithCaptures {
        lambda_fn: FunctionValue<'ctx>,
        captures_ptr: PointerValue<'ctx>,
    },
}

mod any_all_walk;
mod cache;
mod filter_walk;
mod fold_walk;
mod map_walk;
